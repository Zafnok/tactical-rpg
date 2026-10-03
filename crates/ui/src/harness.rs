//! Headless test driver for screens (ADR-0007 layer 4): scripted key and
//! controller-button presses in, screen names and snapshots out. No window,
//! no clock.
//!
//! Available to unit tests and, with the `harness` feature, to integration
//! tests in `crates/ui/tests/`. See `crates/ui/README.md`.
//!
//! ```
//! use trpg_ui::harness::Harness;
//! use trpg_ui::input::Layout;
//!
//! let mut h = Harness::with_layout(Layout::RightHanded);
//! h.keys("f");
//! assert_eq!(h.top_screen(), "mode_select");
//! h.keys("d");
//! assert_eq!(h.top_screen(), "title");
//! ```

use crate::audio::{AudioRequest, MusicClock, MusicCommand};
use crate::flow::FlowScreen;
use crate::game::{Game, RawInputEvent};
use crate::input::{Button, Chord, Device, Layout, PadKind};
use crate::map_view::{MapScene, skin_named};
use crate::screen::{Ctx, KeyPrompt, LAYOUT_KEY, Screen};
use crate::screens::BattleScreen;
use crate::storage::{MemoryStorage, Storage};

/// Simulated length of one frame, in seconds (60 fps).
pub const FRAME_DT: f32 = 1.0 / 60.0;

/// Most frames one [`Harness::hold`] or [`Harness::wait`] may run (about
/// half a simulated minute), so a runaway script fails instead of hanging.
pub const MAX_FRAMES: u32 = 2_000;

/// Drives a [`Game`] with simulated key presses and time.
pub struct Harness {
    game: Game,
    /// Every frame's audio requests, oldest first.
    audio: Vec<Vec<AudioRequest>>,
    /// Every music command of the run, in order.
    music: Vec<MusicCommand>,
    /// The simulated music player, for [`Ctx::music_clock`].
    player: Player,
    /// The kind of controller [`pad`](Self::pad) presses buttons on.
    pad_kind: PadKind,
}

/// The Harness's stand-in for `app`'s music player (ADR-0037): which track
/// "sounds" and for how long, worked out from the music commands and the
/// frame times, so a screen that keeps time with the music is tested
/// without a sound card or a clock.
#[derive(Debug, Default)]
struct Player {
    /// Seconds a track takes to load: it sounds that long after its
    /// `Start`.
    load_delay: f64,
    /// Whether no track ever sounds (the music files are missing).
    silent: bool,
    /// The track started last, and the seconds since its `Start`.
    started: Option<(String, f64)>,
}

impl Player {
    /// Lets `dt` seconds pass (nonsense frame times don't count, as in
    /// [`Game`]).
    fn advance(&mut self, dt: f32) {
        if let Some((_, since)) = &mut self.started
            && dt.is_finite()
        {
            *since += f64::from(dt.max(0.0));
        }
    }

    /// What `app` would report: the track sounding and for how long.
    fn playing(&self) -> Option<(&str, f64)> {
        let (cue, since) = self.started.as_ref().filter(|_| !self.silent)?;
        let elapsed = since - self.load_delay;
        (elapsed >= 0.0).then_some((cue.as_str(), elapsed))
    }

    /// Follows one music command: a `Start` starts the count from 0; a
    /// `Stop` (or a fresh `Load`) of that track ends it.
    fn apply(&mut self, command: &MusicCommand) {
        match command {
            MusicCommand::Start { cue } => self.started = Some((cue.clone(), 0.0)),
            MusicCommand::Stop { cue } | MusicCommand::Load { cue } => {
                if self.started.as_ref().is_some_and(|(c, _)| c == cue) {
                    self.started = None;
                }
            }
            MusicCommand::Gain { .. } => {}
        }
    }
}

impl Harness {
    /// A first launch: the embedded content and empty storage, so the
    /// layout picker is open over the title screen.
    ///
    /// # Panics
    ///
    /// If the embedded content fails to load (the content tests catch that
    /// first).
    pub fn new() -> Self {
        Self::with_storage(Box::new(MemoryStorage::new()))
    }

    /// A later launch where `layout` was picked before: at the title screen
    /// with that layout's keys.
    pub fn with_layout(layout: Layout) -> Self {
        let mut storage = MemoryStorage::new();
        if let Err(e) = storage.write(LAYOUT_KEY, layout.name()) {
            panic!("saving the layout: {e}");
        }
        Self::with_storage(Box::new(storage))
    }

    /// Like [`new`](Self::new), as the web build launches: the title
    /// waits for a key press ([`Ctx::key_prompt`]).
    pub fn on_web() -> Self {
        Self::new().web()
    }

    /// Like [`with_layout`](Self::with_layout), as the web build launches
    /// ([`on_web`](Self::on_web)).
    pub fn on_web_with_layout(layout: Layout) -> Self {
        Self::with_layout(layout).web()
    }

    /// Sets [`KeyPrompt::Waiting`] and redraws.
    fn web(mut self) -> Self {
        self.game.ctx_mut().key_prompt = KeyPrompt::Waiting;
        self.game.redraw();
        self
    }

    /// A launch with `storage` (e.g. from [`into_storage`](Self::into_storage)
    /// of an earlier run), starting as the real game does.
    pub fn with_storage(storage: Box<dyn Storage>) -> Self {
        Self::from_game(Game::start(embedded_ctx().with_storage(storage)))
    }

    /// The game with the embedded content and the right-handed layout,
    /// showing only `root`, for testing a screen on its own.
    pub fn with_screen(root: Box<dyn Screen>) -> Self {
        let ctx = embedded_ctx().with_layout(Layout::RightHanded);
        Self::from_game(Game::new(ctx, root))
    }

    /// Wraps `game`. Debug screens are always on, so F2 behaves the same
    /// in debug and release test runs.
    pub fn from_game(game: Game) -> Self {
        Self {
            game: game.with_debug_screens(true),
            audio: Vec::new(),
            music: Vec::new(),
            player: Player::default(),
            pad_kind: PadKind::default(),
        }
    }

    /// Makes music take `seconds` to load: a track starts sounding, and
    /// [`Ctx::music_clock`] starts counting, that long after the game
    /// starts it (at once by default). Negative or NaN counts as 0.
    pub fn music_load_delay(&mut self, seconds: f32) -> &mut Self {
        self.player.load_delay = f64::from(seconds.max(0.0));
        self
    }

    /// Makes no music ever sound, as when the music files are missing:
    /// [`Ctx::music_clock`] stays `None`.
    pub fn without_music(&mut self) -> &mut Self {
        self.player.silent = true;
        self
    }

    /// Where the music is in its track, as the screens saw it in the last
    /// frame ([`Ctx::music_clock`]).
    pub fn music_clock(&self) -> Option<&MusicClock> {
        self.game.ctx().music_clock.as_ref()
    }

    /// From now on [`pad`](Self::pad) and [`hold_pad`](Self::hold_pad)
    /// press buttons on a controller of `kind` (a generic one until then).
    /// The button names in scripts stay binding positions: on a Nintendo
    /// pad `South` is its right face button.
    pub fn use_pad(&mut self, kind: PadKind) -> &mut Self {
        self.pad_kind = kind;
        self
    }

    /// Presses and releases each whitespace-separated chord in turn, e.g.
    /// `"Down Down f"` or `"Shift+Space Enter"`: a frame with the press,
    /// then a frame with the release, each [`FRAME_DT`] long.
    ///
    /// # Panics
    ///
    /// On a chord [`Chord::parse`] rejects.
    pub fn keys(&mut self, script: &str) -> &mut Self {
        for token in script.split_whitespace() {
            let chord = parse(token);
            self.frame(&[RawInputEvent::Down(chord)], FRAME_DT);
            self.frame(&[RawInputEvent::Up(chord.key)], FRAME_DT);
        }
        self
    }

    /// Types `text` one character per frame as the app reports typing: the
    /// key's press (Shift for capitals) with the character, then its
    /// release. A character without a key of its own is typed alone.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for c in text.chars() {
            let name = match c {
                ' ' => "Space".to_owned(),
                c if c.is_ascii_uppercase() => format!("Shift+{}", c.to_ascii_lowercase()),
                c => c.to_string(),
            };
            let Ok(chord) = Chord::parse(&name) else {
                self.frame(&[RawInputEvent::Text(c)], FRAME_DT);
                continue;
            };
            self.frame(
                &[RawInputEvent::Down(chord), RawInputEvent::Text(c)],
                FRAME_DT,
            );
            self.frame(&[RawInputEvent::Up(chord.key)], FRAME_DT);
        }
        self
    }

    /// Holds `chord` for `seconds` of frames (so held keys repeat), then
    /// releases it in one more frame.
    ///
    /// # Panics
    ///
    /// On a chord [`Chord::parse`] rejects.
    pub fn hold(&mut self, chord: &str, seconds: f32) -> &mut Self {
        let chord = parse(chord);
        self.advance(&[RawInputEvent::Down(chord)], seconds);
        self.frame(&[RawInputEvent::Up(chord.key)], FRAME_DT);
        self
    }

    /// Presses and releases each whitespace-separated controller button in
    /// turn, e.g. `"DpadDown South"`, like [`keys`](Self::keys). The names
    /// are button positions, as in `keymap.ron`.
    ///
    /// # Panics
    ///
    /// On a button [`Button::parse`] rejects.
    pub fn pad(&mut self, script: &str) -> &mut Self {
        for token in script.split_whitespace() {
            let button = parse_button(token);
            self.frame(&[RawInputEvent::PadDown(button, self.pad_kind)], FRAME_DT);
            self.frame(&[RawInputEvent::PadUp(button)], FRAME_DT);
        }
        self
    }

    /// Holds the controller button `button` for `seconds` of frames (so a
    /// held direction repeats), then releases it in one more frame.
    ///
    /// # Panics
    ///
    /// On a button [`Button::parse`] rejects.
    pub fn hold_pad(&mut self, button: &str, seconds: f32) -> &mut Self {
        let button = parse_button(button);
        self.advance(&[RawInputEvent::PadDown(button, self.pad_kind)], seconds);
        self.frame(&[RawInputEvent::PadUp(button)], FRAME_DT);
        self
    }

    /// Lets `seconds` pass with no input, in frames of at most [`FRAME_DT`].
    pub fn wait(&mut self, seconds: f32) -> &mut Self {
        self.advance(&[], seconds);
        self
    }

    /// Runs frames totalling `seconds` (at least one frame); `first` goes
    /// into the first frame.
    ///
    /// # Panics
    ///
    /// If that takes more than [`MAX_FRAMES`] frames.
    fn advance(&mut self, first: &[RawInputEvent], seconds: f32) {
        let mut left = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        let mut events = first;
        for _ in 0..MAX_FRAMES {
            let dt = left.min(FRAME_DT);
            self.frame(events, dt);
            events = &[];
            left -= dt;
            if left <= 0.0 {
                return;
            }
        }
        panic!("Harness: more than {MAX_FRAMES} frames in one hold or wait");
    }

    /// Runs one game frame as `app` does: reports the music sounding,
    /// runs the frame, then records and "plays" its audio.
    fn frame(&mut self, events: &[RawInputEvent], dt: f32) {
        self.player.advance(dt);
        self.game.set_music_playing(self.player.playing());
        let out = self.game.frame(events, dt);
        self.audio.push(out.audio.to_vec());
        self.music.extend_from_slice(out.music);
        for command in out.music {
            self.player.apply(command);
        }
    }

    /// Every audio request of the run so far, in order.
    pub fn audio_requests(&self) -> Vec<AudioRequest> {
        self.audio.concat()
    }

    /// The sound cues (not music) played so far, in order.
    pub fn sounds(&self) -> Vec<String> {
        self.audio
            .iter()
            .flatten()
            .filter_map(|r| match r {
                AudioRequest::PlaySound { cue, .. } => Some(cue.clone()),
                _ => None,
            })
            .collect()
    }

    /// The audio requests of the last frame run (a key press runs two
    /// frames, press and release; this is the release).
    pub fn last_frame_audio(&self) -> &[AudioRequest] {
        self.audio.last().map_or(&[], Vec::as_slice)
    }

    /// Every music command of the run so far, in order.
    pub fn music_commands(&self) -> &[MusicCommand] {
        &self.music
    }

    /// Forgets the audio recorded so far, so the next checks see only
    /// what comes after.
    pub fn clear_audio(&mut self) -> &mut Self {
        self.audio.clear();
        self.music.clear();
        self
    }

    /// Name of the top screen; empty once every screen has closed.
    pub fn top_screen(&self) -> &'static str {
        self.game.top_screen().unwrap_or("")
    }

    /// Names of the open screens, bottom first.
    pub fn screens(&self) -> Vec<&'static str> {
        self.game.screens()
    }

    /// The current frame in the snapshot format of [`crate::snapshot`].
    pub fn snapshot(&self) -> String {
        self.game.buffer().to_snapshot(&self.game.ctx().palette)
    }

    /// The current frame as it would look had the player pressed `device`
    /// last: the same screen with that device's key or button names. For
    /// comparing play on a controller with the same play on the keyboard.
    pub fn snapshot_as(&mut self, device: Device) -> String {
        let used = std::mem::replace(&mut self.game.ctx_mut().device, device);
        self.game.redraw();
        let snapshot = self.snapshot();
        self.game.ctx_mut().device = used;
        self.game.redraw();
        snapshot
    }

    /// The game flow on the stack (New Game or Quick Battle), if any.
    pub fn flow(&self) -> Option<&FlowScreen> {
        self.game.screen()
    }

    /// The game flow on the stack, to play its battle with scripted
    /// commands ([`FlowScreen::battle_mut`]).
    pub fn flow_mut(&mut self) -> Option<&mut FlowScreen> {
        self.game.screen_mut()
    }

    /// The battle on the stack: the game flow's, else a battle screen put
    /// on the stack itself. `None` with no battle.
    pub fn battle(&self) -> Option<&BattleScreen> {
        let in_flow = self.flow().and_then(FlowScreen::battle);
        in_flow.or_else(|| self.game.screen::<BattleScreen>())
    }

    /// What the battle on the stack shows on its map (ADR-0038), as
    /// [`battle`](Self::battle) finds it. Assert on this (or on the
    /// battle's state), not on cells and colours, to check what happened.
    pub fn map_scene(&self) -> Option<MapScene> {
        Some(self.battle()?.scene(self.game.ctx()))
    }

    /// Paints battle maps with the skin called `name` from the next frame
    /// on (`"glyph"`, or `"sprite"` for the test tileset; ADR-0038), as
    /// the debug menu's "Map skin" does.
    ///
    /// # Panics
    ///
    /// If there is no such skin.
    pub fn with_map_skin(&mut self, name: &str) -> &mut Self {
        let ctx = self.game.ctx_mut();
        let skin = skin_named(&ctx.content, name);
        ctx.map_skin = skin.unwrap_or_else(|| panic!("no map skin {name:?}"));
        self
    }

    /// [`map_scene`](Self::map_scene) as text ([`MapScene::to_text`]);
    /// empty with no battle.
    pub fn map_text(&self) -> String {
        let content = &self.game.ctx().content;
        self.map_scene()
            .map_or_else(String::new, |scene| scene.to_text(content))
    }

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.game.quit_requested()
    }

    /// The game being driven.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// The shared context, to change it between key presses, e.g.
    /// `h.ctx_mut().set_layout_bindings(…)` to rebind keys as the Key
    /// bindings screen would. The next frame uses the new keys.
    pub fn ctx_mut(&mut self) -> &mut Ctx {
        self.game.ctx_mut()
    }

    /// Turns the one-time tips on (they are off in tests by default).
    pub fn with_tips(&mut self) -> &mut Self {
        self.game.ctx_mut().tips_enabled = true;
        self
    }

    /// Ends the run, handing back its storage, to start the next launch
    /// with [`with_storage`](Self::with_storage).
    pub fn into_storage(self) -> Box<dyn Storage> {
        self.game.into_ctx().storage
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

fn embedded_ctx() -> Ctx {
    match Ctx::embedded() {
        Ok(mut ctx) => {
            ctx.debug_tools = true;
            ctx
        }
        Err(e) => panic!("embedded content failed to load: {e}"),
    }
}

fn parse(chord: &str) -> Chord {
    match Chord::parse(chord) {
        Ok(chord) => chord,
        Err(e) => panic!("bad chord in test script: {e}"),
    }
}

fn parse_button(button: &str) -> Button {
    match Button::parse(button) {
        Ok(button) => button,
        Err(e) => panic!("bad button in test script: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_buffer::GlyphBuffer;
    use crate::input::Action;
    use crate::screen::{FrameInput, Transition};
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Duration;

    type Seen = Rc<RefCell<Vec<(Vec<Action>, f32)>>>;

    /// Records every frame's actions and dt.
    struct Recorder(Seen);

    impl Screen for Recorder {
        fn name(&self) -> &'static str {
            "recorder"
        }
        fn update(&mut self, _: &mut Ctx, input: &FrameInput) -> Transition {
            self.0.borrow_mut().push((input.actions.clone(), input.dt));
            Transition::None
        }
        fn draw(&self, _: &Ctx, _: &mut GlyphBuffer) {}
    }

    fn recorder() -> (Harness, Seen) {
        let seen = Seen::default();
        (
            Harness::with_screen(Box::new(Recorder(Rc::clone(&seen)))),
            seen,
        )
    }

    fn total_time(seen: &Seen) -> f32 {
        seen.borrow().iter().map(|(_, dt)| dt).sum()
    }

    #[test]
    fn keys_press_and_release_each_chord_over_two_frames() {
        let (mut h, seen) = recorder();
        h.keys("  f\tShift+Space  Down ");
        let seen = seen.borrow();
        let actions: Vec<_> = seen.iter().map(|(a, _)| a.clone()).collect();
        assert_eq!(
            actions,
            [
                vec![Action::Confirm],
                vec![],
                vec![Action::ToggleAutoEnd],
                vec![],
                vec![Action::CursorDown],
                vec![],
            ]
        );
        assert!(
            seen.iter()
                .all(|&(_, dt)| (dt - FRAME_DT).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn hold_repeats_then_releases() {
        let (mut h, seen) = recorder();
        h.hold("Down", 0.5);
        // Press, then repeats at 300 ms and every 55 ms up to 500 ms.
        let downs = seen
            .borrow()
            .iter()
            .flat_map(|(a, _)| a.clone())
            .filter(|&a| a == Action::CursorDown)
            .count();
        assert_eq!(downs, 1 + 1 + (500 - 300) / 55);
        assert!((total_time(&seen) - (0.5 + FRAME_DT)).abs() < 1e-4);
        // Released: waiting emits nothing more.
        seen.borrow_mut().clear();
        h.wait(1.0);
        assert!(seen.borrow().iter().all(|(a, _)| a.is_empty()));
    }

    #[test]
    fn use_pad_sets_the_kind_of_pad_pressed_and_snapshot_as_only_looks() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        let keys = h.snapshot();
        assert_eq!(h.game().ctx().device, Device::Keyboard);
        h.pad("RightTrigger");
        // Unbound: still the keyboard.
        assert_eq!(h.game().ctx().device, Device::Keyboard);
        h.pad("DpadDown DpadUp");
        assert_eq!(h.game().ctx().device, Device::Pad(PadKind::Generic));
        let pad = h.snapshot();
        assert_ne!(pad, keys);
        assert_eq!(h.snapshot_as(Device::Keyboard), keys);
        // Looking doesn't change the device or the frame.
        assert_eq!(h.game().ctx().device, Device::Pad(PadKind::Generic));
        assert_eq!(h.snapshot(), pad);
        h.use_pad(PadKind::PlayStation).pad("DpadDown DpadUp");
        assert_eq!(h.game().ctx().device, Device::Pad(PadKind::PlayStation));
        assert_ne!(h.snapshot(), pad);
        h.use_pad(PadKind::Nintendo).hold_pad("DpadDown", 0.0);
        assert_eq!(h.game().ctx().device, Device::Pad(PadKind::Nintendo));
        h.keys("Up");
        assert_eq!(h.game().ctx().device, Device::Keyboard);
        assert_eq!(h.snapshot(), keys);
    }

    #[test]
    fn pad_presses_and_releases_each_button_over_two_frames() {
        let (mut h, seen) = recorder();
        h.pad(" South\tDpadDown  Start ");
        let seen = seen.borrow();
        let actions: Vec<_> = seen.iter().map(|(a, _)| a.clone()).collect();
        assert_eq!(
            actions,
            [
                vec![Action::Confirm],
                vec![],
                vec![Action::CursorDown],
                vec![],
                vec![Action::EndTurn],
                vec![],
            ]
        );
        assert!(
            seen.iter()
                .all(|&(_, dt)| (dt - FRAME_DT).abs() < f32::EPSILON)
        );
    }

    #[test]
    fn hold_pad_repeats_with_the_keyboard_timings_then_releases() {
        let (mut h, seen) = recorder();
        h.hold_pad("DpadDown", 0.5);
        let downs = |seen: &Seen| {
            let seen = seen.borrow();
            let all = seen.iter().flat_map(|(a, _)| a.clone());
            all.filter(|&a| a == Action::CursorDown).count()
        };
        // Exactly what a held Down key gives (`hold_repeats_then_releases`).
        assert_eq!(downs(&seen), 1 + 1 + (500 - 300) / 55);
        assert!((total_time(&seen) - (0.5 + FRAME_DT)).abs() < 1e-4);
        let (mut keys, seen_keys) = recorder();
        keys.hold("Down", 0.5);
        assert_eq!(*seen.borrow(), *seen_keys.borrow());
        // Released: waiting emits nothing more.
        seen.borrow_mut().clear();
        h.wait(1.0);
        assert!(seen.borrow().iter().all(|(a, _)| a.is_empty()));
    }

    #[test]
    #[should_panic(expected = "bad button in test script")]
    fn bad_button_panics() {
        Harness::with_layout(Layout::RightHanded).pad("f");
    }

    #[test]
    fn wait_runs_frames_of_at_most_frame_dt() {
        let (mut h, seen) = recorder();
        h.wait(0.1);
        assert!((total_time(&seen) - 0.1).abs() < 1e-4);
        assert!(seen.borrow().iter().all(|&(_, dt)| dt <= FRAME_DT));
        seen.borrow_mut().clear();
        h.wait(0.0).wait(-1.0).wait(f32::NAN);
        assert_eq!(
            *seen.borrow(),
            [(vec![], 0.0), (vec![], 0.0), (vec![], 0.0)]
        );
    }

    #[test]
    fn new_is_a_first_launch_and_with_layout_a_later_one() {
        let h = Harness::default();
        assert_eq!(h.screens(), ["title", "layout_picker"]);
        let h = Harness::with_layout(Layout::LeftHanded);
        assert_eq!(h.screens(), ["title"]);
        assert_eq!(h.game().ctx().layout(), Some(Layout::LeftHanded));
    }

    #[test]
    fn storage_carries_over_between_runs() {
        let mut h = Harness::new();
        h.keys("Enter");
        let h = Harness::with_storage(h.into_storage());
        assert_eq!(h.screens(), ["title"]);
        assert_eq!(h.game().ctx().layout(), Some(Layout::RightHanded));
    }

    #[test]
    fn reports_game_state() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        assert_eq!(h.top_screen(), "title");
        assert_eq!(
            h.snapshot(),
            h.game().buffer().to_snapshot(&h.game().ctx().palette)
        );
        h.keys("Up f");
        assert!(h.quit_requested());
        let mut h = Harness::with_screen(Box::new(crate::screens::ModeSelectScreen::new()));

        h.keys("d");
        assert_eq!(h.top_screen(), "");
        assert!(h.screens().is_empty());
        assert!(h.quit_requested());
    }

    #[test]
    fn map_scene_is_the_battle_on_the_stack() {
        use crate::screens::BattleScreen;
        use crate::screens::battle::quick_battle;
        // No battle: no scene, no text.
        let mut h = Harness::with_layout(Layout::RightHanded);
        assert_eq!(h.map_scene(), None);
        assert_eq!(h.map_text(), "");
        // The Quick Battle, inside the game flow.
        h.keys("Down f");
        assert_eq!(h.screens(), ["title", "battle"]);
        let scene = h.map_scene().unwrap();
        let battle = h.flow().and_then(FlowScreen::battle).unwrap();
        assert_eq!(scene, battle.scene(h.game().ctx()));
        assert_eq!(scene.units.len(), 8);
        assert_eq!(h.map_text(), scene.to_text(&h.game().ctx().content));
        assert!(h.map_text().starts_with("origin (-10,-11) size 35x30\n"));
        // A battle screen on the stack itself.
        let state = quick_battle(&embedded_ctx().content).unwrap();
        let h = Harness::with_screen(Box::new(BattleScreen::new(state)));
        let bare = h.map_scene().unwrap();
        // The same map; only the cursor's pulse is a frame behind.
        assert_eq!((bare.tiles, bare.units), (scene.tiles, scene.units));
        assert_eq!(bare.cursor.map(|c| c.pos), scene.cursor.map(|c| c.pos));
        assert!(h.battle().is_some());
    }

    #[test]
    fn the_map_skin_can_be_switched_by_name() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        assert!(h.battle().is_none());
        h.keys("Down f");
        assert_eq!(h.battle().map(|b| b.state().units().len()), Some(8));
        h.with_map_skin("sprite").wait(FRAME_DT);
        assert_eq!(h.game().ctx().map_skin.name(), "sprite");
        // The test tileset's 24 px tiles: 23 × 20 of them.
        assert!(h.map_text().contains(" size 23x20\n"), "{}", h.map_text());
        h.with_map_skin("glyph");
        assert_eq!(h.game().ctx().map_skin.name(), "glyph");
    }

    #[test]
    #[should_panic(expected = "no map skin \"ascii\"")]
    fn an_unknown_map_skin_panics() {
        Harness::with_layout(Layout::RightHanded).with_map_skin("ascii");
    }

    #[test]
    fn debug_screens_are_always_on() {
        let ctx = embedded_ctx().with_layout(Layout::RightHanded);
        let game = Game::start(ctx).with_debug_screens(false);
        let mut h = Harness::from_game(game);
        h.keys("F2");
        assert_eq!(h.top_screen(), "debug_menu");
    }

    #[test]
    fn long_waits_are_refused() {
        let (mut h, seen) = recorder();
        h.wait(32.0);
        let frames = seen.borrow().len();
        assert!(frames > 1_900 && frames <= 2_000, "{frames} frames");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            h.wait(34.0);
        }));
        assert!(result.is_err(), "a wait over MAX_FRAMES must panic");
    }

    #[test]
    #[should_panic(expected = "bad chord in test script")]
    fn bad_chord_panics() {
        Harness::with_layout(Layout::RightHanded).keys("Ctrl+x");
    }

    /// Plays `beep` on Confirm and music `theme` on Cancel.
    struct Beeper;

    impl Screen for Beeper {
        fn name(&self) -> &'static str {
            "beeper"
        }
        fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
            for action in &input.actions {
                match action {
                    Action::Confirm => ctx.audio.play_sound("beep"),
                    Action::Cancel => ctx.audio.play_music("theme"),
                    _ => {}
                }
            }
            Transition::None
        }
        fn draw(&self, _: &Ctx, _: &mut GlyphBuffer) {}
    }

    fn beeper() -> Harness {
        let ctx = crate::screen::tests::ctx_with_cues(&["beep"], &["theme"]);
        Harness::from_game(Game::new(ctx, Box::new(Beeper)))
    }

    fn beep() -> AudioRequest {
        AudioRequest::PlaySound {
            cue: "beep".into(),
            volume: 1.0,
        }
    }

    #[test]
    fn records_the_audio_screens_ask_for() {
        let theme = AudioRequest::PlayMusic {
            cue: "theme".into(),
        };
        let mut h = beeper();
        assert!(h.audio_requests().is_empty());
        assert!(h.last_frame_audio().is_empty());
        h.keys("f");
        assert_eq!(h.audio_requests(), [beep()]);
        h.keys("d f");
        assert_eq!(h.audio_requests(), [beep(), theme, beep()]);
        let load = MusicCommand::Load {
            cue: "theme".into(),
        };
        let start = MusicCommand::Start {
            cue: "theme".into(),
        };
        assert_eq!(h.music_commands(), [load, start]);
        h.clear_audio();
        assert!(h.audio_requests().is_empty());
        assert!(h.music_commands().is_empty());
    }

    fn command(cue: &str, command: fn(String) -> MusicCommand) -> MusicCommand {
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
    fn the_player_counts_from_a_start_to_its_stop() {
        let mut p = Player::default();
        assert_eq!(p.playing(), None);
        p.advance(1.0);
        p.apply(&command("a", load));
        assert_eq!(p.playing(), None, "loaded, not started");
        p.apply(&command("a", start));
        assert_eq!(p.playing(), Some(("a", 0.0)));
        p.advance(0.5);
        p.advance(0.25);
        assert_eq!(p.playing(), Some(("a", 0.75)));
        // Nonsense frame times don't count.
        for dt in [-1.0, f32::NAN, f32::INFINITY] {
            p.advance(dt);
        }
        assert_eq!(p.playing(), Some(("a", 0.75)));
        // A fade and other tracks' commands leave it playing.
        let fade = MusicCommand::Gain {
            cue: "a".into(),
            gain: 0.5,
        };
        for other in [fade, command("b", load), command("b", stop)] {
            p.apply(&other);
        }
        assert_eq!(p.playing(), Some(("a", 0.75)));
        p.apply(&command("a", stop));
        assert_eq!(p.playing(), None);
        p.advance(1.0);
        assert_eq!(p.playing(), None);
        // The next track counts from its own start; loading a track again
        // replaces it, as in `app`.
        p.apply(&command("b", start));
        p.advance(0.5);
        assert_eq!(p.playing(), Some(("b", 0.5)));
        p.apply(&command("b", load));
        assert_eq!(p.playing(), None);
    }

    #[test]
    fn the_player_sounds_after_its_load_delay_or_never() {
        let mut p = Player {
            load_delay: 0.5,
            ..Player::default()
        };
        p.apply(&command("a", start));
        assert_eq!(p.playing(), None);
        p.advance(0.25);
        assert_eq!(p.playing(), None);
        p.advance(0.25);
        assert_eq!(p.playing(), Some(("a", 0.0)));
        p.advance(0.25);
        assert_eq!(p.playing(), Some(("a", 0.25)));
        p.silent = true;
        assert_eq!(p.playing(), None);
    }

    /// The clock's cue and position, if the music sounds.
    fn clock(h: &Harness) -> Option<(&str, f32)> {
        h.music_clock().map(|c| (c.cue.as_str(), c.position))
    }

    /// Whether the title track sounds, `expected` seconds in (to 10 ms:
    /// frame times are `f32`s).
    fn title_at(h: &Harness, expected: f32) -> bool {
        clock(h).is_some_and(|(cue, at)| cue == "title" && (at - expected).abs() < 0.01)
    }

    /// Ticket 0227: the title's music clock.
    #[test]
    fn the_title_music_clock_counts_up_and_wraps_at_the_track_length() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        assert_eq!(h.music_clock(), None);
        // The title asks for its music in its first frame; the track
        // sounds from then, so the next frame is the first to see a clock.
        h.wait(0.0);
        assert_eq!(h.music_clock(), None);
        h.wait(1.0);
        assert!(title_at(&h, 1.0), "{:?}", h.music_clock());
        let length_ms = h.game().ctx().content.audio.music["title"].length_ms;
        let length = Duration::from_millis(length_ms.into()).as_secs_f32();
        assert!(length > 60.0, "{length}");
        let clock_length = h.music_clock().map(|c| c.length).unwrap();
        assert!((clock_length - length).abs() < 1e-4, "{clock_length}");
        // Key presses are frames too.
        h.wait(2.5).keys("Down Up");
        let so_far = 3.5 + 4.0 * FRAME_DT;
        assert!(title_at(&h, so_far), "{:?}", h.music_clock());
        // To a quarter of a second before the end of the track ...
        let mut left = length - 0.25 - so_far;
        while left > 0.0 {
            let step = left.min(30.0);
            h.wait(step);
            left -= step;
        }
        assert!(title_at(&h, length - 0.25), "{:?}", h.music_clock());
        // ... and round to its start again.
        h.wait(1.0);
        assert!(title_at(&h, 0.75), "{:?}", h.music_clock());
    }

    #[test]
    fn a_music_load_delay_starts_the_clock_late() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        h.music_load_delay(2.0).wait(0.0);
        h.wait(1.5);
        assert_eq!(h.music_clock(), None);
        h.wait(1.0);
        assert!(title_at(&h, 0.5), "{:?}", h.music_clock());
        // A negative or NaN delay is none.
        for delay in [-1.0, f32::NAN] {
            let mut h = Harness::with_layout(Layout::RightHanded);
            h.music_load_delay(2.0).music_load_delay(delay).wait(0.0);
            h.wait(0.5);
            assert!(title_at(&h, 0.5), "{delay}: {:?}", h.music_clock());
        }
    }

    #[test]
    fn without_music_the_clock_never_starts() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        h.without_music().wait(0.0);
        h.wait(3.0);
        assert_eq!(h.music_clock(), None);
        // The game asked for its music all the same.
        assert!(h.music_commands().contains(&command("title", start)));
    }

    /// The web title asks for its music at the first key press
    /// (`docs/design/title-screen.md`), so its clock starts there.
    #[test]
    fn on_the_web_the_clock_starts_at_the_first_key_press() {
        let mut h = Harness::on_web_with_layout(Layout::RightHanded);
        h.wait(1.0);
        assert_eq!(h.music_clock(), None);
        // The press starts the music; the release is one frame later.
        h.keys("f");
        assert!(title_at(&h, FRAME_DT), "{:?}", h.music_clock());
        h.wait(0.5);
        assert!(title_at(&h, 0.5 + FRAME_DT), "{:?}", h.music_clock());
    }

    /// During a fade the clock is still the old track's; the new track's
    /// starts when the fade ends (0.5 s, `audio.ron`).
    #[test]
    fn the_clock_follows_a_switch_of_track_once_the_fade_ends() {
        let mut h = Harness::with_layout(Layout::RightHanded);
        h.wait(0.0).wait(1.0);
        // Quick Battle plays a track from the skirmish pool.
        h.keys("Down f");
        assert_eq!(h.top_screen(), "battle");
        assert!(title_at(&h, 1.0 + 4.0 * FRAME_DT), "{:?}", h.music_clock());
        h.wait(1.0);
        let (cue, position) = clock(&h).unwrap();
        let pool = &h.game().ctx().content.audio.pools["skirmish"];
        assert!(pool.iter().any(|c| c == cue), "{cue}");
        // The fade began with the `f` press, two frames before the wait.
        let expected = 1.0 + 2.0 * FRAME_DT - 0.5;
        assert!((position - expected).abs() < 0.05, "{position}");
    }

    #[test]
    fn last_frame_audio_is_the_last_frame_only() {
        let mut h = beeper();
        // `keys` presses in one frame and releases in the next.
        h.keys("f");
        assert!(h.last_frame_audio().is_empty());
        h.frame(&[RawInputEvent::Down(parse("f"))], FRAME_DT);
        assert_eq!(h.last_frame_audio(), [beep()]);
        h.wait(0.1);
        assert!(h.last_frame_audio().is_empty());
        assert_eq!(h.audio_requests(), [beep(), beep()]);
    }
}
