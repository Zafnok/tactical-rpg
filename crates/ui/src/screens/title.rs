//! The placeholder title screen (the real game flow comes with ticket 0801).

use super::battle::quick_battle_screen;
use super::{centre_x, draw_debug_hint, print_centred};
use crate::audio::{MenuSound, pick_from_pool};
use crate::color::UiColor;
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
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

/// Menu item that starts a new game.
const NEW_GAME: &str = "New Game";
/// Debug menu item: straight into a test battle.
const QUICK_BATTLE: &str = "Quick Battle";
/// Menu item that quits.
const QUIT: &str = "Quit";

/// The title screen's music cue (`audio.md`).
pub const TITLE_MUSIC: &str = "title";
/// The music pool Quick Battle picks its track from: it is a test
/// skirmish, and skirmishes pick at random from this pool (`audio.md`).
pub const QUICK_BATTLE_MUSIC_POOL: &str = "skirmish";

/// Fills `buf` with blank `text`-on-`black` cells.
fn clear(ctx: &Ctx, buf: &mut GlyphBuffer) {
    let p = &ctx.palette;
    buf.fill_rect(
        buf.bounds(),
        Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black)),
    );
}

/// Title, subtitle and a `New Game` / `Quit` menu, with a help line naming
/// the keys of the active layout.
#[derive(Debug, Clone)]
pub struct TitleScreen {
    menu: Menu,
    /// Menu item labels, in menu order.
    items: Vec<&'static str>,
    /// Whether the title music was asked for since this screen was last
    /// shown. The screen has no "shown" hook, so a screen that changes the
    /// music clears it and the next update asks again.
    music_on: bool,
    /// Quick Battles started, mixed into the music seed so each one can
    /// pick a different track.
    battles_started: u64,
    /// Whether the "press any key" prompt ([`Ctx::key_prompt`]) is over.
    /// It shows once per launch.
    prompt_done: bool,
}

impl TitleScreen {
    /// The title screen with `New Game` focused.
    pub fn new() -> Self {
        Self::with_items(vec![NEW_GAME, QUIT])
    }

    /// The title screen with a debug `Quick Battle` item after `New Game`,
    /// which opens a battle on the test map with placeholder units.
    pub fn with_quick_battle() -> Self {
        Self::with_items(vec![NEW_GAME, QUICK_BATTLE, QUIT])
    }

    fn with_items(items: Vec<&'static str>) -> Self {
        Self {
            // Nothing to back out of on the title screen.
            menu: Menu::new(items.iter().map(|&i| MenuItem::new(i)).collect()).without_cancel(),
            items,
            music_on: false,
            battles_started: 0,
            prompt_done: false,
        }
    }

    /// Whether the title is still showing [`PRESS_ANY_KEY`] instead of
    /// its menu.
    fn waiting(&self, ctx: &Ctx) -> bool {
        ctx.key_prompt != KeyPrompt::Off && !self.prompt_done
    }

    /// Plays a Quick Battle track: one from [`QUICK_BATTLE_MUSIC_POOL`],
    /// kept for the whole battle (the battle screen asks for no music).
    /// Ticket 0807 moves this to where battles start once battle files name
    /// their music.
    fn start_quick_battle_music(&mut self, ctx: &mut Ctx) {
        let seed = ctx.music_seed ^ self.battles_started;
        self.battles_started = self.battles_started.wrapping_add(1);
        if let Some(cue) = pick_from_pool(&ctx.content.audio, QUICK_BATTLE_MUSIC_POOL, seed) {
            ctx.audio.play_music(cue);
            self.music_on = false;
        }
    }

    /// The bottom help line: the cursor keys `move`, the Confirm key
    /// `select`, the Cancel key `back`, named from the active keymap.
    pub fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        help_line(&[
            (cursor_keys_name(km), "move"),
            (key_name(km, Action::Confirm), "select"),
            (key_name(km, Action::Cancel), "back"),
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
        for &action in &input.actions {
            let chosen = match self.menu.handle_with_sound(action, &mut ctx.audio) {
                Some(MenuEvent::Chosen(i)) => self.items.get(i).copied(),
                Some(MenuEvent::Cancelled) | None => None,
            };
            match chosen {
                Some(NEW_GAME) => return Transition::Push(Box::new(PlaceholderScreen)),
                // The placeholder data always builds (tested); should it
                // ever not, the item does nothing.
                Some(QUICK_BATTLE) => {
                    if let Ok(screen) = quick_battle_screen(&ctx.content) {
                        self.start_quick_battle_music(ctx);
                        return Transition::Push(Box::new(screen));
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
        let (w, _) = self.menu.size();
        let x = centre_x(buf, usize::try_from(w).unwrap_or(0));
        self.menu.draw(&ctx.palette, buf, x, MENU_ROW);
        print_centred(buf, bottom, &Self::help(ctx), c(UiColor::TextDim), black);
        draw_debug_hint(ctx, buf, bottom);
    }
}

/// Stand-in for screens that don't exist yet; Cancel goes back.
#[derive(Debug, Clone, Copy)]
pub struct PlaceholderScreen;

impl PlaceholderScreen {
    /// The message shown: `Coming soon`, then `press <Cancel key> to go
    /// back` with the key named from the active keymap.
    pub fn message(ctx: &Ctx) -> String {
        match key_name(&ctx.keymap, Action::Cancel) {
            Some(key) => format!("Coming soon — press {key} to go back"),
            None => "Coming soon".to_owned(),
        }
    }
}

impl Screen for PlaceholderScreen {
    fn name(&self) -> &'static str {
        "placeholder"
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if input.actions.contains(&Action::Cancel) {
            ctx.audio.menu(MenuSound::Cancel);
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        clear(ctx, buf);
        let y = i32::from(buf.height()) / 2;
        let p = &ctx.palette;
        print_centred(
            buf,
            y,
            &Self::message(ctx),
            p.get(UiColor::Text),
            p.get(UiColor::Black),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioRequest;
    use crate::screen::tests::ctx;

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
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(placeholder)");
        assert_eq!(outcome(&mut t, &[CursorDown, Confirm]), "Quit");
        assert_eq!(outcome(&mut t, &[CursorUp]), "None");
        // Actions after the one that transitions are dropped.
        assert_eq!(outcome(&mut t, &[Confirm, CursorDown]), "Push(placeholder)");
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(placeholder)");
    }

    #[test]
    fn debug_title_offers_quick_battle() {
        use Action::{Confirm, CursorDown, CursorUp};
        let mut t = TitleScreen::with_quick_battle();
        assert_eq!(t.items, [NEW_GAME, QUICK_BATTLE, QUIT]);
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(placeholder)");
        assert_eq!(outcome(&mut t, &[CursorDown, Confirm]), "Push(battle)");
        assert_eq!(outcome(&mut t, &[CursorDown, Confirm]), "Quit");
        assert_eq!(
            outcome(&mut t, &[CursorUp, CursorUp, Confirm]),
            "Push(placeholder)"
        );
        // Without the test characters, Quick Battle does nothing.
        let mut c = ctx();
        c.content.characters.characters.clear();
        t.menu = TitleScreen::with_quick_battle().menu;
        let input = FrameInput::new(vec![CursorDown, Confirm], 0.0, vec![]);
        assert_eq!(format!("{:?}", t.update(&mut c, &input)), "None");
        assert_eq!(TitleScreen::new().items, [NEW_GAME, QUIT]);
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
        let mut p = PlaceholderScreen;
        assert!(sounds_of(&mut p, &mut c, &[Confirm, CursorDown]).is_empty());
        assert_eq!(sounds_of(&mut p, &mut c, &[Cancel]), ["menu_cancel"]);
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

    #[test]
    fn placeholder_pops_on_cancel_only() {
        let mut p = PlaceholderScreen;
        assert_eq!(p.name(), "placeholder");
        assert_eq!(outcome(&mut p, &[Action::Confirm]), "None");
        assert_eq!(outcome(&mut p, &[Action::Confirm, Action::Cancel]), "Pop");
    }

    #[test]
    fn texts_name_the_layout_keys() {
        let mut c = ctx();
        assert_eq!(TitleScreen::help(&c), "arrows move · f select · d back");
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(TitleScreen::help(&c), "wasd move · j select · k back");
        assert_eq!(
            PlaceholderScreen::message(&c),
            "Coming soon — press k to go back"
        );
        c.use_layout(crate::input::Layout::RightHanded);
        assert_eq!(
            PlaceholderScreen::message(&c),
            "Coming soon — press d to go back"
        );
        c.keymap = crate::input::Keymap::new(
            crate::input::Action::ALL
                .iter()
                .flat_map(|&a| c.keymap.chords_for(a).into_iter().map(move |ch| (ch, a)))
                .filter(|&(_, a)| a != Action::Cancel),
            c.keymap.repeat(),
        );
        assert_eq!(TitleScreen::help(&c), "arrows move · f select");
        assert_eq!(PlaceholderScreen::message(&c), "Coming soon");
    }

    /// Opaque screens must paint every cell, not rely on `Game` clearing.
    #[test]
    fn screens_cover_the_whole_buffer() {
        use crate::console::{CONSOLE_H, CONSOLE_W};
        let c = ctx();
        let screens: [&dyn Screen; 3] = [
            &TitleScreen::new(),
            &TitleScreen::with_quick_battle(),
            &PlaceholderScreen,
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
