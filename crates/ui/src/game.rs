//! [`Game`]: the whole UI behind one call per frame. `app` feeds it raw key
//! events and the frame time and blits the buffer it returns; the test
//! `Harness` drives it the same way without a window.

use crate::audio::{AudioRequest, MusicCommand, MusicState};
use crate::color::UiColor;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::debug::{self, DebugMenuScreen};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::{Action, Chord, InputState, Key};
use crate::screen::{Ctx, FrameInput, KeyPrompt, Screen, ScreenStack};
use crate::screens::{LayoutPickerScreen, TitleScreen};

/// A keyboard event as `app` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawKeyEvent {
    /// A key went down, with the Shift state at the time.
    Down(Chord),
    /// A key went up.
    Up(Key),
}

/// The result of one frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameOutput<'a> {
    /// What to show.
    pub buffer: &'a GlyphBuffer,
    /// Whether the game has asked to quit (it then ignores further input).
    pub quit: bool,
    /// Every audio request screens made this frame, in order. `app` plays
    /// the sounds; the music requests are already turned into [`music`].
    ///
    /// [`music`]: Self::music
    pub audio: &'a [AudioRequest],
    /// What to do to the music this frame (ADR-0026).
    pub music: &'a [MusicCommand],
}

/// Owns the screens, input state, shared context and the console buffer.
pub struct Game {
    stack: ScreenStack,
    input: InputState,
    ctx: Ctx,
    buffer: GlyphBuffer,
    quit: bool,
    music: MusicState,
    /// This frame's audio requests and music commands.
    audio_out: Vec<AudioRequest>,
    music_out: Vec<MusicCommand>,
}

impl Game {
    /// A game showing `root`. Debug screens (F2) are on in debug builds.
    pub fn new(ctx: Ctx, root: Box<dyn Screen>) -> Self {
        Self::with_stack(ctx, ScreenStack::new(root))
    }

    /// A game starting at the title screen. If no layout is in use yet, the
    /// saved one is loaded from `ctx.storage`; if none is saved (first
    /// launch), the layout picker opens on top of the title.
    pub fn start(mut ctx: Ctx) -> Self {
        if ctx.layout().is_none()
            && let Some(layout) = ctx.saved_layout()
        {
            ctx.use_layout(layout);
        }
        let title = if ctx.debug_tools {
            TitleScreen::with_quick_battle()
        } else {
            TitleScreen::new()
        };
        let mut stack = ScreenStack::new(Box::new(title));
        if ctx.layout().is_none() {
            stack.push(Box::new(LayoutPickerScreen::new()));
        }
        Self::with_stack(ctx, stack)
    }

    fn with_stack(ctx: Ctx, stack: ScreenStack) -> Self {
        let input = InputState::new(ctx.keymap.clone());
        let blank = Cell::new(
            ' ',
            ctx.palette.get(UiColor::Text),
            ctx.palette.get(UiColor::Black),
        );
        #[allow(clippy::cast_precision_loss)] // A fade is well under 2^24 ms.
        let fade_secs = ctx.content.audio.music_fade_ms as f32 / 1000.0;
        let mut game = Self {
            stack,
            input,
            ctx,
            buffer: GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank),
            quit: false,
            music: MusicState::new(fade_secs),
            audio_out: Vec::new(),
            music_out: Vec::new(),
        };
        game.redraw();
        game
    }

    /// Turns the debug screens (the [`Action::Debug`] key) on or off
    /// ([`Ctx::debug_tools`]).
    #[must_use]
    pub fn with_debug_screens(mut self, on: bool) -> Self {
        self.ctx.debug_tools = on;
        self
    }

    /// Runs one frame: applies `events` (in order), advances input by `dt`
    /// seconds, updates the top screen with the resulting actions, collects
    /// its audio requests and redraws. After a quit, frames do nothing.
    pub fn frame(&mut self, events: &[RawKeyEvent], dt: f32) -> FrameOutput<'_> {
        self.audio_out.clear();
        self.music_out.clear();
        if !self.quit {
            self.step(events, dt);
            self.collect_audio(dt);
        }
        FrameOutput {
            buffer: &self.buffer,
            quit: self.quit,
            audio: &self.audio_out,
            music: &self.music_out,
        }
    }

    /// Moves the screens' audio requests into this frame's output and runs
    /// the music state machine. In debug builds, a cue the manifest lacks
    /// is a bug in the screen and panics.
    fn collect_audio(&mut self, dt: f32) {
        self.audio_out = self.ctx.audio.take();
        for request in &self.audio_out {
            debug_assert!(
                is_known(&self.ctx.content.audio, request),
                "audio request for a cue not in assets/audio/audio.ron: {request:?}"
            );
            self.music.request(request, &mut self.music_out);
        }
        self.music.update(dt, &mut self.music_out);
    }

    fn step(&mut self, events: &[RawKeyEvent], dt: f32) {
        // Bindings changed between frames (a test rebinding keys).
        self.sync_keymap();
        for &event in events {
            if matches!(event, RawKeyEvent::Down(_)) && self.ctx.key_prompt == KeyPrompt::Waiting {
                self.ctx.key_prompt = KeyPrompt::Pressed;
            }
            match event {
                RawKeyEvent::Down(chord) => self.input.key_down(chord),
                RawKeyEvent::Up(key) => self.input.key_up(key),
            }
        }
        let pressed = events
            .iter()
            .filter_map(|event| match event {
                RawKeyEvent::Down(chord) => Some(*chord),
                RawKeyEvent::Up(_) => None,
            })
            .collect();
        let actions = self.input.update(dt);
        let held = Action::ALL
            .into_iter()
            .filter(|&a| self.input.is_held(a))
            .collect();
        let opens_debug_menu = self.ctx.debug_tools
            && actions.contains(&Action::Debug)
            && !self
                .stack
                .top_name()
                .is_some_and(|n| debug::SCREENS.contains(&n));
        if opens_debug_menu {
            self.stack.push(Box::new(DebugMenuScreen::new()));
        } else {
            let input = FrameInput::new(actions, dt, held).with_pressed_chords(pressed);
            self.quit = self.stack.update(&mut self.ctx, &input);
            self.sync_keymap();
        }
        self.redraw();
    }

    /// Hands `ctx`'s keymap to the input if it changed (a layout was
    /// picked, keys were rebound), so the new keys work from the next press.
    fn sync_keymap(&mut self) {
        if *self.input.keymap() != self.ctx.keymap {
            self.input.set_keymap(self.ctx.keymap.clone());
        }
    }

    /// Clears the buffer and draws the stack into it.
    pub(crate) fn redraw(&mut self) {
        let blank = Cell::new(
            ' ',
            self.ctx.palette.get(UiColor::Text),
            self.ctx.palette.get(UiColor::Black),
        );
        self.buffer.fill_rect(self.buffer.bounds(), blank);
        self.stack.draw(&self.ctx, &mut self.buffer);
    }

    /// The music state machine (which track plays).
    pub fn music(&self) -> &MusicState {
        &self.music
    }

    /// The last frame drawn.
    pub fn buffer(&self) -> &GlyphBuffer {
        &self.buffer
    }

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// The shared context.
    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    /// The shared context, to change settings mid-run (tests).
    #[cfg(any(test, feature = "harness"))]
    pub(crate) fn ctx_mut(&mut self) -> &mut Ctx {
        &mut self.ctx
    }

    /// Ends the game, handing back the shared context (and so its storage).
    pub fn into_ctx(self) -> Ctx {
        self.ctx
    }

    /// Name of the top screen, or `None` once the last one has closed.
    pub fn top_screen(&self) -> Option<&'static str> {
        self.stack.top_name()
    }

    /// Names of the open screens, bottom first.
    pub fn screens(&self) -> Vec<&'static str> {
        self.stack.names()
    }
}

/// Whether `request` names a cue of the right kind in `manifest`.
fn is_known(manifest: &trpg_content::AudioManifest, request: &AudioRequest) -> bool {
    match request {
        AudioRequest::PlaySound { cue, .. } => manifest.sounds.contains_key(cue),
        AudioRequest::PlayMusic { cue } => manifest.music.contains_key(cue),
        AudioRequest::StopMusic => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Layout;
    use crate::screen::tests::{ctx, ctx_with_cues};

    fn down(key: Key) -> RawKeyEvent {
        RawKeyEvent::Down(Chord::plain(key))
    }

    fn tap(game: &mut Game, key: Key) -> bool {
        let quit = game.frame(&[down(key)], 0.0).quit;
        game.frame(&[RawKeyEvent::Up(key)], 0.0);
        quit
    }

    #[test]
    fn starts_at_the_title_already_drawn() {
        let game = Game::start(ctx());
        assert_eq!(game.top_screen(), Some("title"));
        assert!(!game.quit_requested());
        let expected = {
            let mut g = Game::start(ctx());
            g.frame(&[], 0.0);
            g.buffer().clone()
        };
        assert_eq!(game.buffer(), &expected);
        assert_eq!(
            (game.buffer().width(), game.buffer().height()),
            (CONSOLE_W, CONSOLE_H)
        );
    }

    fn first_launch() -> Ctx {
        Ctx::embedded().unwrap()
    }

    #[test]
    fn first_launch_opens_the_picker_over_the_title() {
        let game = Game::start(first_launch());
        assert_eq!(game.screens(), ["title", "layout_picker"]);
        assert_eq!(game.ctx().layout(), None);
    }

    #[test]
    fn a_saved_layout_skips_the_picker() {
        let mut ctx = first_launch();
        ctx.storage.write("layout", "LeftHanded").unwrap();
        let game = Game::start(ctx);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.ctx().layout(), Some(Layout::LeftHanded));
        assert_eq!(game.input.keymap(), &game.ctx().keymap);
    }

    #[test]
    fn a_layout_in_use_beats_the_saved_one() {
        let mut ctx = ctx();
        ctx.storage.write("layout", "LeftHanded").unwrap();
        let game = Game::start(ctx);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.ctx().layout(), Some(Layout::RightHanded));
    }

    #[test]
    fn picking_a_layout_switches_the_input_keys() {
        let mut game = Game::start(first_launch());
        // Picker keys: `s` moves down, `j` picks.
        tap(&mut game, Key::S);
        tap(&mut game, Key::J);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.ctx().layout(), Some(Layout::LeftHanded));
        assert_eq!(game.input.keymap(), &game.ctx().keymap);
        // Left-handed: `j` confirms, `f` does nothing, `k` backs out.
        tap(&mut game, Key::F);
        assert_eq!(game.screens(), ["title"]);
        tap(&mut game, Key::J);
        assert_eq!(game.screens(), ["title", "placeholder"]);
        tap(&mut game, Key::K);
        assert_eq!(game.screens(), ["title"]);
        let ctx = game.into_ctx();
        assert_eq!(ctx.saved_layout(), Some(Layout::LeftHanded));
    }

    /// Ticket 0224: the first key press ends the web title's wait; releases
    /// don't, and native builds never wait.
    #[test]
    fn a_key_press_moves_the_key_prompt_on() {
        let mut game = Game::start(ctx());
        game.frame(&[down(Key::Q)], 0.0);
        assert_eq!(game.ctx().key_prompt, KeyPrompt::Off);
        let mut web = ctx();
        web.key_prompt = KeyPrompt::Waiting;
        let mut game = Game::start(web);
        game.frame(&[RawKeyEvent::Up(Key::Q)], 0.0);
        assert_eq!(game.ctx().key_prompt, KeyPrompt::Waiting);
        game.frame(&[down(Key::Q)], 0.0);
        assert_eq!(game.ctx().key_prompt, KeyPrompt::Pressed);
    }

    #[test]
    fn rebinding_keys_reaches_the_input_before_the_next_press() {
        let mut game = Game::start(ctx());
        let mut b = game.ctx().layout_bindings(Layout::RightHanded);
        assert_eq!(b.bind(Action::Confirm, 0, Chord::plain(Key::G)), Ok(None));
        game.ctx_mut()
            .set_layout_bindings(Layout::RightHanded, b)
            .unwrap();
        // The old key does nothing; the new one confirms at once.
        tap(&mut game, Key::F);
        assert_eq!(game.screens(), ["title"]);
        tap(&mut game, Key::G);
        assert_eq!(game.screens(), ["title", "placeholder"]);
        assert_eq!(game.input.keymap(), &game.ctx().keymap);
    }

    #[test]
    fn escape_is_ignored_by_the_first_launch_picker() {
        let mut game = Game::start(first_launch());
        assert_eq!(
            game.input.keymap().action(Chord::plain(Key::Escape)),
            Some(Action::Cancel)
        );
        tap(&mut game, Key::Escape);
        assert_eq!(game.screens(), ["title", "layout_picker"]);
    }

    #[test]
    fn events_reach_the_top_screen() {
        let mut game = Game::start(ctx());
        assert!(!tap(&mut game, Key::F));
        assert_eq!(game.screens(), ["title", "placeholder"]);
        tap(&mut game, Key::D);
        assert_eq!(game.screens(), ["title"]);
    }

    #[test]
    fn quit_stops_further_frames() {
        let mut game = Game::start(ctx());
        game.frame(&[down(Key::Up)], 0.0);
        let out = game.frame(&[RawKeyEvent::Up(Key::Up), down(Key::F)], 0.0);
        assert!(out.quit);
        assert!(game.quit_requested());
        let before = game.buffer().clone();
        // Would open the placeholder if the game were still running.
        game.frame(&[RawKeyEvent::Up(Key::F), down(Key::Up)], 0.0);
        game.frame(&[RawKeyEvent::Up(Key::Up), down(Key::F)], 0.0);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.buffer(), &before);
        assert!(game.frame(&[], 0.0).quit);
    }

    #[test]
    fn popping_the_last_screen_quits() {
        let mut game = Game::new(ctx(), Box::new(crate::screens::PlaceholderScreen));
        assert!(tap(&mut game, Key::D));
        assert_eq!(game.top_screen(), None);
    }

    #[test]
    fn debug_key_opens_the_debug_menu_once() {
        let mut game = Game::start(ctx()).with_debug_screens(true);
        tap(&mut game, Key::F2);
        assert_eq!(game.screens(), ["title", "debug_menu"]);
        tap(&mut game, Key::F2);
        assert_eq!(game.screens(), ["title", "debug_menu"]);
        // Not over a debug tool either.
        tap(&mut game, Key::F);
        assert_eq!(game.screens(), ["title", "debug_menu", "glyph_sampler"]);
        tap(&mut game, Key::F2);
        assert_eq!(game.screens(), ["title", "debug_menu", "glyph_sampler"]);
        tap(&mut game, Key::D);
        tap(&mut game, Key::Down);
        tap(&mut game, Key::F);
        assert_eq!(game.screens(), ["title", "debug_menu", "portrait_viewer"]);
        tap(&mut game, Key::F2);
        assert_eq!(game.screens(), ["title", "debug_menu", "portrait_viewer"]);
        tap(&mut game, Key::D);
        tap(&mut game, Key::D);
        assert_eq!(game.screens(), ["title"]);
    }

    #[test]
    fn debug_key_does_nothing_without_debug_screens() {
        let mut game = Game::start(ctx()).with_debug_screens(false);
        tap(&mut game, Key::F2);
        assert_eq!(game.screens(), ["title"]);
    }

    #[test]
    fn debug_screens_follow_the_build() {
        let game = Game::start(Ctx::embedded().unwrap());
        assert_eq!(game.ctx.debug_tools, crate::screen::DEBUG_TOOLS);
        let names = |ctx: Ctx| {
            let mut game = Game::start(ctx);
            tap(&mut game, Key::Down);
            tap(&mut game, Key::F);
            game.screens()
        };
        let mut release = ctx();
        release.debug_tools = false;
        assert_eq!(names(release), ["title"]); // Down + f chose Quit.
        let mut debug = ctx();
        debug.debug_tools = true;
        assert_eq!(names(debug), ["title", "battle"]);
    }

    /// Records what the screen saw.
    struct Spy(std::rc::Rc<std::cell::RefCell<Vec<FrameInput>>>);

    impl Screen for Spy {
        fn name(&self) -> &'static str {
            "spy"
        }
        fn update(&mut self, _: &mut Ctx, input: &FrameInput) -> crate::screen::Transition {
            self.0.borrow_mut().push(input.clone());
            crate::screen::Transition::None
        }
        fn draw(&self, _: &Ctx, _: &mut GlyphBuffer) {}
    }

    #[test]
    fn screens_see_actions_dt_and_held_keys() {
        let seen = std::rc::Rc::default();
        let mut game = Game::new(ctx(), Box::new(Spy(std::rc::Rc::clone(&seen))));
        assert_eq!(game.ctx().palette, ctx().palette);
        game.frame(&[down(Key::Right), down(Key::F)], 0.1);
        game.frame(&[RawKeyEvent::Up(Key::F)], 0.0);
        let seen = seen.borrow();
        assert_eq!(seen[0].actions, [Action::CursorRight, Action::Confirm]);
        assert!((seen[0].dt - 0.1).abs() < f32::EPSILON);
        assert!(seen[0].is_held(Action::CursorRight));
        assert!(seen[0].is_held(Action::Confirm));
        assert!(seen[1].actions.is_empty());
        assert!(seen[1].is_held(Action::CursorRight));
        assert!(!seen[1].is_held(Action::Confirm));
        // Every press of the frame, in order; releases aren't presses.
        assert_eq!(
            seen[0].pressed_chords,
            [Chord::plain(Key::Right), Chord::plain(Key::F)]
        );
        assert!(seen[1].pressed_chords.is_empty());
    }

    #[test]
    fn screens_see_unbound_and_shifted_presses_as_chords() {
        let seen = std::rc::Rc::default();
        let mut game = Game::new(ctx(), Box::new(Spy(std::rc::Rc::clone(&seen))));
        let shifted = Chord::shifted(Key::Q);
        game.frame(&[down(Key::Delete), RawKeyEvent::Down(shifted)], 0.0);
        let seen = seen.borrow();
        assert!(seen[0].actions.is_empty());
        assert_eq!(seen[0].pressed_chords, [Chord::plain(Key::Delete), shifted]);
    }

    /// Plays `beep` on Confirm, switches to music `battle` on Cancel, quits
    /// (with a quieter beep) on Up and asks for an unknown cue otherwise.
    struct Noisy;

    impl Screen for Noisy {
        fn name(&self) -> &'static str {
            "noisy"
        }
        fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> crate::screen::Transition {
            for action in &input.actions {
                match action {
                    Action::Confirm => ctx.audio.play_sound("beep"),
                    Action::Cancel => ctx.audio.play_music("battle"),
                    Action::CursorUp => {
                        ctx.audio.play_sound_at("beep", 0.5);
                        return crate::screen::Transition::Quit;
                    }
                    _ => ctx.audio.play_sound("nope"),
                }
            }
            crate::screen::Transition::None
        }
        fn draw(&self, _: &Ctx, _: &mut GlyphBuffer) {}
    }

    /// A game on [`Noisy`] with a 0.5 s fade, `title` asked for already.
    fn noisy() -> Game {
        let mut ctx = ctx_with_cues(&["beep"], &["title", "battle"]);
        ctx.content.audio.music_fade_ms = 500;
        ctx.audio.play_music("title");
        Game::new(ctx, Box::new(Noisy))
    }

    fn beep(volume: f32) -> AudioRequest {
        AudioRequest::PlaySound {
            cue: "beep".into(),
            volume,
        }
    }

    fn music(cue: &str, command: fn(String) -> MusicCommand) -> MusicCommand {
        command(cue.into())
    }

    fn load(cue: String) -> MusicCommand {
        MusicCommand::Load { cue }
    }

    fn start(cue: String) -> MusicCommand {
        MusicCommand::Start { cue }
    }

    fn stop(cue: String) -> MusicCommand {
        MusicCommand::Stop { cue }
    }

    #[test]
    fn frames_carry_the_audio_screens_asked_for() {
        let mut game = noisy();
        // A request made before the first frame goes out with it.
        let out = game.frame(&[], 0.0);
        let title = AudioRequest::PlayMusic {
            cue: "title".into(),
        };
        assert_eq!(out.audio, [title]);
        assert_eq!(out.music, [music("title", load), music("title", start)]);
        let out = game.frame(&[down(Key::F)], 0.0);
        assert_eq!(out.audio, [beep(1.0)]);
        assert!(out.music.is_empty());
        // Each frame holds only its own requests.
        let out = game.frame(&[RawKeyEvent::Up(Key::F)], 0.0);
        assert!(out.audio.is_empty());
        assert!(game.ctx().audio.pending().is_empty());
    }

    #[test]
    fn frames_carry_the_music_fade() {
        let mut game = noisy();
        game.frame(&[], 0.0);
        let out = game.frame(&[down(Key::D)], 0.25);
        let half = MusicCommand::Gain {
            cue: "title".into(),
            gain: 0.5,
        };
        assert_eq!(out.music, [music("battle", load), half]);
        assert_eq!(game.music().target(), Some("battle"));
        let out = game.frame(&[RawKeyEvent::Up(Key::D)], 0.25);
        assert_eq!(out.music, [music("title", stop), music("battle", start)]);
        assert_eq!(game.music().current(), Some("battle"));
    }

    #[test]
    fn the_quitting_frame_still_sounds_and_later_ones_are_silent() {
        let mut game = noisy();
        let out = game.frame(&[down(Key::Up)], 0.0);
        assert!(out.quit);
        assert_eq!(out.audio[1..], [beep(0.5)]);
        let out = game.frame(&[RawKeyEvent::Up(Key::Up), down(Key::F)], 0.0);
        assert!(out.audio.is_empty());
        assert!(out.music.is_empty());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "audio request for a cue not in assets/audio/audio.ron")]
    fn an_unknown_cue_panics_in_debug_builds() {
        let mut game = noisy();
        game.frame(&[down(Key::Right)], 0.0);
    }

    #[test]
    fn the_fade_length_comes_from_the_manifest() {
        let mut ctx = ctx_with_cues(&[], &["title", "battle"]);
        ctx.content.audio.music_fade_ms = 1000;
        ctx.audio.play_music("title");
        let mut game = Game::new(ctx, Box::new(Noisy));
        game.frame(&[], 0.0);
        let out = game.frame(&[down(Key::D)], 0.25);
        let gain = MusicCommand::Gain {
            cue: "title".into(),
            gain: 0.75,
        };
        assert_eq!(out.music[1..], [gain]);
    }

    #[test]
    fn cues_must_be_of_the_right_kind() {
        let audio = &ctx_with_cues(&["beep"], &["title"]).content.audio;
        let sound = |cue: &str| AudioRequest::PlaySound {
            cue: cue.into(),
            volume: 1.0,
        };
        let play = |cue: &str| AudioRequest::PlayMusic { cue: cue.into() };
        assert!(is_known(audio, &sound("beep")));
        assert!(!is_known(audio, &sound("title")));
        assert!(is_known(audio, &play("title")));
        assert!(!is_known(audio, &play("beep")));
        assert!(is_known(audio, &AudioRequest::StopMusic));
    }

    /// A won battle pops back to the title, which asks for its music again.
    #[test]
    fn the_title_music_returns_after_a_battle() {
        use crate::screens::battle::testing::battle_with;
        use crate::screens::battle::{BattleScreen, quick_battle};
        use trpg_core::{Objective, Pos, UnitId};

        let c = ctx();
        let quick = quick_battle(&c.content).unwrap();
        // The lord next to a brigand on 1 HP, which one hit routs.
        let mut units = quick.units().to_vec();
        units.retain(|u| u.id == UnitId(1) || u.id == UnitId(4));
        units[0].pos = Pos::new(7, 2);
        units[1].hp = 1;
        let rout = Objective::Rout { turn_limit: None };
        let battle = battle_with(&c, quick.map().clone(), units, rout);
        let mut stack = ScreenStack::new(Box::new(TitleScreen::with_quick_battle()));
        stack.push(Box::new(BattleScreen::new(battle)));
        let mut game = Game::with_stack(c, stack);
        let mut music = Vec::new();
        // The battle's sounds (0424) aside.
        let music_of = |audio: &[AudioRequest]| {
            let music = |r: &&AudioRequest| !matches!(r, AudioRequest::PlaySound { .. });
            audio.iter().filter(music).cloned().collect::<Vec<_>>()
        };
        // Select the lord, stay, Attack, the brigand, confirm the forecast.
        for key in [Key::F; 5] {
            let quit = game.frame(&[down(key)], 0.0).quit;
            assert!(!quit);
            music.extend(music_of(game.frame(&[RawKeyEvent::Up(key)], 0.5).audio));
        }
        for _ in 0..60 {
            music.extend(music_of(game.frame(&[], 0.5).audio));
        }
        assert!(music.is_empty(), "{music:?}");
        assert_eq!(game.screens(), ["title", "battle"]);
        // Past the VICTORY banner, back at the title.
        assert!(game.frame(&[down(Key::F)], 0.0).audio.is_empty());
        assert_eq!(game.screens(), ["title"]);
        let title = AudioRequest::PlayMusic {
            cue: "title".into(),
        };
        assert_eq!(game.frame(&[RawKeyEvent::Up(Key::F)], 0.0).audio, [title]);
        assert!(game.frame(&[], 0.1).audio.is_empty());
    }
}
