//! Headless test driver for screens (ADR-0007 layer 4): scripted key
//! presses in, screen names and snapshots out. No window, no clock.
//!
//! Available to unit tests and, with the `harness` feature, to integration
//! tests in `crates/ui/tests/`. See `crates/ui/README.md`.
//!
//! ```
//! use trpg_ui::harness::Harness;
//!
//! let mut h = Harness::new();
//! h.keys("f");
//! assert_eq!(h.top_screen(), "placeholder");
//! h.keys("d");
//! assert_eq!(h.top_screen(), "title");
//! ```

use crate::game::{Game, RawKeyEvent};
use crate::input::Chord;
use crate::screen::{Ctx, Screen};

/// Simulated length of one frame, in seconds (60 fps).
pub const FRAME_DT: f32 = 1.0 / 60.0;

/// Most frames one [`Harness::hold`] or [`Harness::wait`] may run (about
/// half a simulated minute), so a runaway script fails instead of hanging.
pub const MAX_FRAMES: u32 = 2_000;

/// Drives a [`Game`] with simulated key presses and time.
pub struct Harness {
    game: Game,
}

impl Harness {
    /// The game with the embedded content, at the title screen.
    ///
    /// # Panics
    ///
    /// If the embedded content fails to load (the content tests catch that
    /// first).
    pub fn new() -> Self {
        Self::from_game(Game::start(embedded_ctx()))
    }

    /// The game with the embedded content, showing only `root`, for testing
    /// a screen on its own.
    pub fn with_screen(root: Box<dyn Screen>) -> Self {
        Self::from_game(Game::new(embedded_ctx(), root))
    }

    /// Wraps `game`. Debug screens are always on, so F12 behaves the same
    /// in debug and release test runs.
    pub fn from_game(game: Game) -> Self {
        Self {
            game: game.with_debug_screens(true),
        }
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
            self.game.frame(&[RawKeyEvent::Down(chord)], FRAME_DT);
            self.game.frame(&[RawKeyEvent::Up(chord.key)], FRAME_DT);
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
        self.advance(&[RawKeyEvent::Down(chord)], seconds);
        self.game.frame(&[RawKeyEvent::Up(chord.key)], FRAME_DT);
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
    fn advance(&mut self, first: &[RawKeyEvent], seconds: f32) {
        let mut left = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        let mut events = first;
        for _ in 0..MAX_FRAMES {
            let dt = left.min(FRAME_DT);
            self.game.frame(events, dt);
            events = &[];
            left -= dt;
            if left <= 0.0 {
                return;
            }
        }
        panic!("Harness: more than {MAX_FRAMES} frames in one hold or wait");
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

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.game.quit_requested()
    }

    /// The game being driven.
    pub fn game(&self) -> &Game {
        &self.game
    }
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

fn embedded_ctx() -> Ctx {
    match Ctx::embedded() {
        Ok(ctx) => ctx,
        Err(e) => panic!("embedded content failed to load: {e}"),
    }
}

fn parse(chord: &str) -> Chord {
    match Chord::parse(chord) {
        Ok(chord) => chord,
        Err(e) => panic!("bad chord in test script: {e}"),
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
        // Press, then repeats at 170 ms and every 55 ms up to 500 ms.
        let downs = seen
            .borrow()
            .iter()
            .flat_map(|(a, _)| a.clone())
            .filter(|&a| a == Action::CursorDown)
            .count();
        assert_eq!(downs, 1 + 1 + (500 - 170) / 55);
        assert!((total_time(&seen) - (0.5 + FRAME_DT)).abs() < 1e-4);
        // Released: waiting emits nothing more.
        seen.borrow_mut().clear();
        h.wait(1.0);
        assert!(seen.borrow().iter().all(|(a, _)| a.is_empty()));
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
    fn reports_game_state() {
        let mut h = Harness::default();
        assert_eq!(h.top_screen(), "title");
        assert_eq!(
            h.snapshot(),
            h.game().buffer().to_snapshot(&h.game().ctx().palette)
        );
        h.keys("Down f");
        assert!(h.quit_requested());
        let mut h = Harness::with_screen(Box::new(crate::screens::PlaceholderScreen));
        h.keys("d");
        assert_eq!(h.top_screen(), "");
        assert!(h.screens().is_empty());
        assert!(h.quit_requested());
    }

    #[test]
    fn debug_screens_are_always_on() {
        let game = Game::start(embedded_ctx()).with_debug_screens(false);
        let mut h = Harness::from_game(game);
        h.keys("F12");
        assert_eq!(h.top_screen(), "glyph_sampler");
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
        Harness::new().keys("Ctrl+x");
    }
}
