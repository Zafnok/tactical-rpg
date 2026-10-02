//! The title screen: `New Game` starts the game flow ([`FlowScreen`],
//! ticket 0801); `Continue` (shown only while there is a suspend save)
//! carries on the suspended battle and `Load Game` opens the save slots
//! (0802); debug builds add `Quick Battle`, a test battle through the same
//! flow.

use super::{centre_x, draw_debug_hint, print_centred};
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::flow::FlowScreen;
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::save;
use crate::screen::{Ctx, FrameInput, KeyPrompt, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// Title text.
pub const TITLE: &str = "Visions of Shuyi";
/// Line under the title.
pub const SUBTITLE: &str = "an ASCII tactics game";

/// Row of the title text.
const TITLE_ROW: i32 = 9;
/// Row of the subtitle.
const SUBTITLE_ROW: i32 = 11;
/// Top row of the menu box.
const MENU_ROW: i32 = 14;

/// Shown where the menu goes until a key is pressed, on the web build
/// (`docs/design/title-screen.md`).
pub const PRESS_ANY_KEY: &str = "Press any key";

/// Menu item that carries on the suspended battle.
const CONTINUE: &str = "Continue";
/// Menu item that starts a new game.
const NEW_GAME: &str = "New Game";
/// Menu item that opens the save slots.
const LOAD_GAME: &str = "Load Game";
/// Debug menu item: straight into a test battle.
const QUICK_BATTLE: &str = "Quick Battle";
/// Menu item that quits.
const QUIT: &str = "Quit";

/// The title screen's music cue (`audio.md`).
pub const TITLE_MUSIC: &str = "title";

/// Fills `buf` with blank `text`-on-`black` cells.
fn clear(ctx: &Ctx, buf: &mut GlyphBuffer) {
    let p = &ctx.palette;
    buf.fill_rect(
        buf.bounds(),
        Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black)),
    );
}

/// Title, subtitle and a menu (`Continue` while a battle is suspended,
/// `New Game`, `Load Game`, `Quit`), with a help line naming the keys of
/// the active layout.
#[derive(Debug, Clone)]
pub struct TitleScreen {
    menu: Menu,
    /// Menu item labels, in menu order.
    items: Vec<&'static str>,
    /// Whether the saves may have changed since the menu was built (the
    /// screen has no "shown" hook): set when another screen is opened, and
    /// the next update builds the menu again ([`refresh`](Self::refresh)).
    stale: bool,
    /// Why `Continue` couldn't, shown under the menu until the next key.
    notice: Option<String>,
    /// Whether the title music was asked for since this screen was last
    /// shown. The screen has no "shown" hook, so starting the game flow
    /// (whose battles play their own music) clears it and the next update
    /// asks again.
    music_on: bool,
    /// Whether the "press any key" prompt ([`Ctx::key_prompt`]) is over.
    /// It shows once per launch.
    prompt_done: bool,
}

impl TitleScreen {
    /// The title screen with `New Game` focused, as it is with no saves
    /// (the first update looks for them; [`refreshed`](Self::refreshed)
    /// does at once).
    pub fn new() -> Self {
        Self::with_debug(false)
    }

    /// The title screen with a debug `Quick Battle` item after `Load
    /// Game`, which plays a test battle through the game flow.
    pub fn with_quick_battle() -> Self {
        Self::with_debug(true)
    }

    fn with_debug(quick_battle: bool) -> Self {
        let (menu, items) = Self::build(quick_battle, false, false);
        Self {
            menu,
            items,
            stale: true,
            notice: None,
            music_on: false,
            prompt_done: false,
        }
    }

    /// The menu and its labels: `Continue` first if a battle is
    /// `suspended`; `Load Game` disabled unless a slot is `saved`. Nothing
    /// to back out of on the title screen.
    fn build(quick_battle: bool, suspended: bool, saved: bool) -> (Menu, Vec<&'static str>) {
        let mut items = vec![NEW_GAME, LOAD_GAME];
        if suspended {
            items.insert(0, CONTINUE);
        }
        if quick_battle {
            items.push(QUICK_BATTLE);
        }
        items.push(QUIT);
        let entries = items.iter().map(|&label| {
            if label == LOAD_GAME && !saved {
                MenuItem::disabled(label)
            } else {
                MenuItem::new(label)
            }
        });
        (Menu::new(entries.collect()).without_cancel(), items)
    }

    /// Builds the menu for the saves in `ctx`'s storage. If it comes out
    /// different (a battle was suspended or continued, a first save made),
    /// the focus goes to its first item; otherwise it stays.
    pub fn refresh(&mut self, ctx: &Ctx) {
        self.stale = false;
        let storage = ctx.storage.as_ref();
        let suspended = save::has_suspend(storage);
        let saved = save::any_slot(storage);
        let quick_battle = self.items.contains(&QUICK_BATTLE);
        let (menu, items) = Self::build(quick_battle, suspended, saved);
        if menu.items() != self.menu.items() {
            self.menu = menu;
            self.items = items;
        }
    }

    /// The same screen with its menu built for `ctx`'s saves
    /// ([`refresh`](Self::refresh)).
    #[must_use]
    pub fn refreshed(mut self, ctx: &Ctx) -> Self {
        self.refresh(ctx);
        self
    }

    /// The notice under the menu, if any.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Opens `screen` over the title; the saves and the music may change
    /// under it, so the title looks for both again when it is back.
    fn open(&mut self, screen: Box<dyn Screen>) -> Transition {
        self.stale = true;
        self.music_on = false;
        Transition::Push(screen)
    }

    /// Whether the title is still showing [`PRESS_ANY_KEY`] instead of
    /// its menu.
    fn waiting(&self, ctx: &Ctx) -> bool {
        ctx.key_prompt != KeyPrompt::Off && !self.prompt_done
    }

    /// The bottom help line: the cursor keys `move`, the Confirm key
    /// `select`, the Cancel key `back`, named from the active keymap.
    pub fn help(ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        help_line(&[
            (Some(cursor_keys_name(km)), "move"),
            (Some(key_name(km, Action::Confirm)), "select"),
            (Some(key_name(km, Action::Cancel)), "back"),
        ])
    }
}

impl Default for TitleScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for TitleScreen {
    fn name(&self) -> &'static str {
        "title"
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if self.waiting(ctx) {
            // A key pressed on another screen first (the layout picker)
            // counts too. The key that ends the wait does nothing else.
            if ctx.key_prompt == KeyPrompt::Pressed {
                self.prompt_done = true;
                ctx.audio.play_music(TITLE_MUSIC);
                self.music_on = true;
            }
            return Transition::None;
        }
        if !self.music_on {
            // The music state ignores a request for the track already on.
            ctx.audio.play_music(TITLE_MUSIC);
            self.music_on = true;
        }
        if self.stale {
            self.refresh(ctx);
        }
        for &action in &input.actions {
            self.notice = None;
            let chosen = match self.menu.handle_with_sound(action, &mut ctx.audio) {
                Some(MenuEvent::Chosen(i)) => self.items.get(i).copied(),
                Some(MenuEvent::Cancelled) | None => None,
            };
            match chosen {
                Some(CONTINUE) => match FlowScreen::resume(ctx) {
                    Ok(flow) => return self.open(Box::new(flow)),
                    // The save stays; the title says why it can't go on.
                    Err(e) => {
                        self.notice = Some(e.to_string());
                        ctx.audio.menu(MenuSound::Denied);
                    }
                },
                Some(NEW_GAME) => return self.open(Box::new(FlowScreen::new_game())),
                Some(LOAD_GAME) => return self.open(Box::new(FlowScreen::load_game(ctx))),
                // The test data always builds (tested); should it ever
                // not, the item does nothing.
                Some(QUICK_BATTLE) => {
                    if let Some(flow) = FlowScreen::quick_battle(ctx) {
                        return self.open(Box::new(flow));
                    }
                }

                Some(QUIT) => return Transition::Quit,
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        clear(ctx, buf);
        print_centred(buf, TITLE_ROW, TITLE, c(UiColor::TextHighlight), black);
        print_centred(buf, SUBTITLE_ROW, SUBTITLE, c(UiColor::TextDim), black);
        let bottom = i32::from(buf.height()) - 1;
        if self.waiting(ctx) {
            print_centred(buf, MENU_ROW, PRESS_ANY_KEY, c(UiColor::TextDim), black);
            draw_debug_hint(ctx, buf, bottom);
            return;
        }
        let (w, h) = self.menu.size();
        let x = centre_x(buf, usize::try_from(w).unwrap_or(0));
        self.menu.draw(&ctx.palette, buf, x, MENU_ROW);
        if let Some(notice) = &self.notice {
            print_centred(buf, MENU_ROW + h + 1, notice, c(UiColor::HpLow), black);
        }
        print_centred(buf, bottom, &Self::help(ctx), c(UiColor::TextDim), black);
        draw_debug_hint(ctx, buf, bottom);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{AudioRequest, pick_from_pool};
    use crate::screen::tests::ctx;

    /// The pool the Quick Battle's file names (`assets/battles/quick.ron`).
    const QUICK_BATTLE_MUSIC_POOL: &str = "skirmish";

    fn input(actions: &[Action]) -> FrameInput {
        FrameInput::new(actions.to_vec(), 0.0, vec![])
    }

    fn outcome(screen: &mut dyn Screen, actions: &[Action]) -> String {
        format!("{:?}", screen.update(&mut ctx(), &input(actions)))
    }

    #[test]
    fn title_menu_transitions() {
        use Action::{Cancel, Confirm, CursorDown, CursorUp};
        let mut t = TitleScreen::new();
        assert_eq!(t.name(), "title");
        assert!(!t.is_overlay());
        assert_eq!(outcome(&mut t, &[]), "None");
        assert_eq!(outcome(&mut t, &[Cancel]), "None");
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(mode_select)");
        assert_eq!(outcome(&mut t, &[CursorDown, Confirm]), "Quit");
        assert_eq!(outcome(&mut t, &[CursorUp]), "None");
        // Actions after the one that transitions are dropped.
        assert_eq!(outcome(&mut t, &[Confirm, CursorDown]), "Push(mode_select)");
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(mode_select)");
    }

    #[test]
    fn debug_title_offers_quick_battle() {
        use Action::{Confirm, CursorDown, CursorUp};
        let mut t = TitleScreen::with_quick_battle();
        assert_eq!(t.items, [NEW_GAME, LOAD_GAME, QUICK_BATTLE, QUIT]);
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(mode_select)");
        assert_eq!(
            outcome(&mut t, &[CursorDown, Confirm]),
            "Push(preparations)"
        );
        assert_eq!(outcome(&mut t, &[CursorDown, Confirm]), "Quit");
        assert_eq!(
            outcome(&mut t, &[CursorUp, CursorUp, Confirm]),
            "Push(mode_select)"
        );
        // Without its chapter, Quick Battle does nothing.
        let mut c = ctx();
        c.content.chapters.clear();
        t.menu = TitleScreen::with_quick_battle().menu;
        let input = FrameInput::new(vec![CursorDown, Confirm], 0.0, vec![]);
        assert_eq!(format!("{:?}", t.update(&mut c, &input)), "None");
        assert_eq!(TitleScreen::new().items, [NEW_GAME, LOAD_GAME, QUIT]);
    }

    /// The music each update asks for, as cue names (`-` for a stop).
    fn music_of(t: &mut TitleScreen, c: &mut Ctx, actions: &[Action]) -> Vec<String> {
        t.update(c, &input(actions));
        c.audio
            .take()
            .iter()
            .filter(|r| !matches!(r, AudioRequest::PlaySound { .. }))
            .map(|r| r.cue().unwrap_or("-").to_owned())
            .collect()
    }

    /// The sounds (not music) each update plays, as cue names.
    fn sounds_of(s: &mut dyn Screen, c: &mut Ctx, actions: &[Action]) -> Vec<String> {
        s.update(c, &input(actions));
        c.audio
            .take()
            .iter()
            .filter(|r| matches!(r, AudioRequest::PlaySound { .. }))
            .filter_map(|r| r.cue().map(str::to_owned))
            .collect()
    }

    #[test]
    fn menu_sounds() {
        use Action::{Cancel, Confirm, CursorDown, CursorUp};
        let mut c = ctx();
        let mut t = TitleScreen::new();
        assert_eq!(
            sounds_of(&mut t, &mut c, &[CursorDown, CursorUp]),
            ["menu_move"; 2]
        );
        // Nothing to back out of: Cancel is silent.
        assert!(sounds_of(&mut t, &mut c, &[Cancel]).is_empty());
        assert_eq!(sounds_of(&mut t, &mut c, &[Confirm]), ["menu_select"]);
    }

    #[test]
    fn title_music_plays_on_show_and_again_after_a_quick_battle() {
        use Action::{Confirm, CursorDown};
        let mut c = ctx();
        let mut t = TitleScreen::with_quick_battle();
        assert_eq!(music_of(&mut t, &mut c, &[]), [TITLE_MUSIC]);
        assert!(music_of(&mut t, &mut c, &[CursorDown]).is_empty());
        let pool = c.content.audio.pools[QUICK_BATTLE_MUSIC_POOL].clone();
        let first = music_of(&mut t, &mut c, &[Confirm]);
        assert_eq!(first.len(), 1);
        assert!(pool.contains(&first[0]), "{first:?}");
        // Back on top after the battle: the title music again, once.
        assert_eq!(music_of(&mut t, &mut c, &[]), [TITLE_MUSIC]);
        assert!(music_of(&mut t, &mut c, &[]).is_empty());
    }

    /// New Game's battles change the music too, so the title asks for its
    /// own again once it is back on top.
    #[test]
    fn title_music_plays_again_after_new_game() {
        let mut c = ctx();
        let mut t = TitleScreen::new();
        assert_eq!(music_of(&mut t, &mut c, &[]), [TITLE_MUSIC]);
        // The mode screen keeps the title music: nothing asked.
        assert!(music_of(&mut t, &mut c, &[Action::Confirm]).is_empty());
        assert_eq!(music_of(&mut t, &mut c, &[]), [TITLE_MUSIC]);
        assert!(music_of(&mut t, &mut c, &[]).is_empty());
    }

    #[test]
    fn each_quick_battle_rolls_its_track_afresh() {
        let mut c = ctx();
        let mut t = TitleScreen::with_quick_battle();
        let pool = c.content.audio.pools[QUICK_BATTLE_MUSIC_POOL].clone();
        let mut picked = std::collections::BTreeSet::new();
        for _ in 0..100 {
            // Down from New Game, then Confirm: Quick Battle.
            let cues = music_of(&mut t, &mut c, &[Action::CursorDown, Action::Confirm]);
            picked.extend(cues.into_iter().filter(|cue| cue != TITLE_MUSIC));
            t.menu = TitleScreen::with_quick_battle().menu;
        }
        // Same seed every time, yet the counter varies the pick.
        assert_eq!(picked.len(), pool.len(), "{picked:?}");
    }

    /// The n-th Quick Battle's track comes from the seed mixed with n.
    #[test]
    fn quick_battle_tracks_follow_the_seed_and_the_count() {
        let mut c = ctx();
        c.music_seed = 0xA5A5;
        let mut t = TitleScreen::with_quick_battle();
        for n in 0..16 {
            let cues = music_of(&mut t, &mut c, &[Action::CursorDown, Action::Confirm]);
            let want = pick_from_pool(&c.content.audio, QUICK_BATTLE_MUSIC_POOL, 0xA5A5 ^ n);
            assert_eq!(cues.last().map(String::as_str), want, "battle {n}");
            t.menu = TitleScreen::with_quick_battle().menu;
        }
    }

    /// A context whose storage holds `suspend` as the suspend save and
    /// `slot` in slot 3.
    fn ctx_with_saves(suspend: Option<&str>, slot: Option<&str>) -> Ctx {
        let mut c = ctx();
        if let Some(text) = suspend {
            c.storage.write(save::SUSPEND_KEY, text).unwrap();
        }
        if let Some(text) = slot {
            c.storage.write(&save::slot_key(3), text).unwrap();
        }
        c
    }

    fn labels(t: &TitleScreen) -> Vec<(&str, bool)> {
        let items = t.menu.items().iter();
        items.map(|i| (i.label.as_str(), i.enabled)).collect()
    }

    #[test]
    fn the_menu_follows_the_saves() {
        // No saves: no Continue, Load Game disabled.
        let none = [(NEW_GAME, true), (LOAD_GAME, false), (QUIT, true)];
        assert_eq!(labels(&TitleScreen::new()), none);
        assert_eq!(labels(&TitleScreen::new().refreshed(&ctx())), none);
        // A slot with anything in it: Load Game.
        let c = ctx_with_saves(None, Some("junk"));
        let t = TitleScreen::new().refreshed(&c);
        let saved = [(NEW_GAME, true), (LOAD_GAME, true), (QUIT, true)];
        assert_eq!(labels(&t), saved);
        assert_eq!(t.items, [NEW_GAME, LOAD_GAME, QUIT]);
        assert_eq!(t.menu.focus(), 0);
        // A suspend save: Continue, first and focused.
        let c = ctx_with_saves(Some("junk"), None);
        let t = TitleScreen::with_quick_battle().refreshed(&c);
        assert_eq!(
            labels(&t),
            [
                (CONTINUE, true),
                (NEW_GAME, true),
                (LOAD_GAME, false),
                (QUICK_BATTLE, true),
                (QUIT, true)
            ]
        );
        assert_eq!(t.items, [CONTINUE, NEW_GAME, LOAD_GAME, QUICK_BATTLE, QUIT]);
        assert_eq!(t.menu.focus(), 0);
    }

    #[test]
    fn the_first_update_and_every_return_look_for_saves() {
        use Action::{Confirm, CursorDown};
        let mut c = ctx_with_saves(None, Some("junk"));
        let mut t = TitleScreen::new();
        assert!(!t.menu.items()[1].enabled);
        // The first update finds the save; the focus moves onto Load Game.
        assert_eq!(
            format!("{:?}", t.update(&mut c, &input(&[CursorDown, Confirm]))),
            "Push(save_slots)"
        );
        // Nothing changed while it was open: the focus stays.
        t.update(&mut c, &input(&[]));
        assert_eq!(t.menu.focus(), 1);
        // The saves don't change under the title itself: no look.
        c.storage.write(save::SUSPEND_KEY, "junk").unwrap();
        t.update(&mut c, &input(&[]));
        assert_eq!(t.items, [NEW_GAME, LOAD_GAME, QUIT]);
        // Back from another screen with a battle suspended: Continue,
        // focused.
        t.update(&mut c, &input(&[Confirm]));
        t.update(&mut c, &input(&[]));
        assert_eq!(t.items, [CONTINUE, NEW_GAME, LOAD_GAME, QUIT]);
        assert_eq!(t.menu.focus(), 0);
    }

    #[test]
    fn continue_with_a_save_that_cant_be_read_says_why() {
        use Action::{Confirm, CursorDown};
        let mut c = ctx_with_saves(Some("junk"), None);
        let mut t = TitleScreen::new().refreshed(&c);
        assert_eq!(t.notice(), None);
        assert_eq!(outcome_in(&mut t, &mut c, &[Confirm]), "None");
        assert_eq!(t.notice(), Some("This save can't be read"));
        c.audio.take();
        // The save is still there, and so is Continue.
        assert_eq!(
            c.storage.read(save::SUSPEND_KEY),
            Ok(Some("junk".to_owned()))
        );
        assert_eq!(t.items[0], CONTINUE);
        // No key, no change; the next key clears the notice.
        t.update(&mut c, &input(&[]));
        assert!(t.notice().is_some());
        t.update(&mut c, &input(&[CursorDown]));
        assert_eq!(t.notice(), None);
        c.audio.take();
        // Chosen: the select sound, then the refusal.
        t.menu = TitleScreen::new().refreshed(&c).menu;
        assert_eq!(
            sounds_of(&mut t, &mut c, &[Confirm]),
            ["menu_select", MenuSound::Denied.cue()]
        );
    }

    fn outcome_in(screen: &mut dyn Screen, c: &mut Ctx, actions: &[Action]) -> String {
        format!("{:?}", screen.update(c, &input(actions)))
    }

    #[test]
    fn texts_name_the_layout_keys() {
        let mut c = ctx();
        assert_eq!(TitleScreen::help(&c), "arrows move · f select · d back");
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(TitleScreen::help(&c), "wasd move · j select · k back");
        c.use_layout(crate::input::Layout::RightHanded);
        c.keymap = crate::input::Keymap::new(
            crate::input::Action::ALL
                .iter()
                .flat_map(|&a| c.keymap.chords_for(a).into_iter().map(move |ch| (ch, a)))
                .filter(|&(_, a)| a != Action::Cancel),
            c.keymap.repeat(),
        );
        // Cancel with no key of its own (the fixed Esc isn't named).
        assert_eq!(
            TitleScreen::help(&c),
            "arrows move · f select · ! not mapped back"
        );
    }

    /// Opaque screens must paint every cell, not rely on `Game` clearing.
    #[test]
    fn screens_cover_the_whole_buffer() {
        use crate::console::{CONSOLE_H, CONSOLE_W};
        let c = ctx();
        let mut naming = crate::screens::LeadSelectScreen::new();
        let typing = FrameInput::new(vec![Action::CursorDown], 0.0, vec![]);
        naming.update(&mut ctx(), &typing);
        naming.update(&mut ctx(), &input(&[Action::Confirm]));
        let screens: [&dyn Screen; 7] = [
            &TitleScreen::new(),
            &TitleScreen::with_quick_battle(),
            &crate::screens::ModeSelectScreen::new(),
            &crate::screens::LeadSelectScreen::new(),
            &naming,
            &crate::screens::GameOverScreen::new(),
            &crate::screens::ToBeContinuedScreen,
        ];
        for screen in screens {
            let stale = Cell::new(
                'x',
                c.palette.get(UiColor::Enemy),
                c.palette.get(UiColor::Enemy),
            );
            let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
            screen.draw(&c, &mut buf);
            let left = (0..i32::from(CONSOLE_H))
                .flat_map(|y| (0..i32::from(CONSOLE_W)).map(move |x| (x, y)))
                .filter(|&(x, y)| buf.get(x, y) == Some(&stale))
                .count();
            assert_eq!(left, 0, "{} left stale cells", screen.name());
        }
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(TitleScreen::default().menu, TitleScreen::new().menu);
        assert_eq!(TitleScreen::default().items, TitleScreen::new().items);
    }
}
