//! Keyboard input as [`Action`]s (ADR-0006): a data-driven [`Keymap`] from
//! chords to actions, and [`InputState`], which turns key presses and
//! releases plus frame time into the actions screens see, including key
//! repeat for held cursor keys.
//!
//! [`Key`], [`Chord`] and [`Action`] are defined in `trpg-content` (its
//! keymap loader validates them) and re-exported here.

use std::collections::HashMap;
use std::time::Duration;

pub use trpg_content::keymap::{Action, Chord, Key, KeymapDef, RepeatDef};

/// At most this many repeats are emitted by one [`InputState::update`], so a
/// lag spike can't teleport the cursor across the map.
pub const MAX_REPEATS_PER_UPDATE: u64 = 5;

/// Lookup from chord to action, plus repeat timings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    bindings: HashMap<Chord, Action>,
    repeat: RepeatDef,
}

impl Keymap {
    /// Builds the lookup from a validated keymap definition.
    pub fn from_def(def: &KeymapDef) -> Self {
        Self {
            bindings: def.bindings.iter().map(|(&c, &a)| (c, a)).collect(),
            repeat: def.repeat,
        }
    }

    /// The action bound to `chord`, if any. `Shift+h` and `h` are distinct:
    /// there is no fallback from a shifted chord to the plain one.
    pub fn action(&self, chord: Chord) -> Option<Action> {
        self.bindings.get(&chord).copied()
    }

    /// Key-repeat timings.
    pub fn repeat(&self) -> RepeatDef {
        self.repeat
    }

    /// Every chord bound to `action`, in [`Chord`] order (letters, digits,
    /// then named keys; plain before shifted), so the result is stable.
    pub fn chords_for(&self, action: Action) -> Vec<Chord> {
        let mut chords: Vec<Chord> = self
            .bindings
            .iter()
            .filter(|&(_, &a)| a == action)
            .map(|(&c, _)| c)
            .collect();
        chords.sort_unstable();
        chords
    }

    /// The chord help text names for `action`: the first of
    /// [`chords_for`](Self::chords_for), or `None` if it is unbound.
    pub fn primary(&self, action: Action) -> Option<Chord> {
        self.chords_for(action).into_iter().next()
    }
}

/// The repeat currently running for the most recently pressed repeatable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Repeat {
    key: Key,
    action: Action,
    /// Time held so far, in microseconds.
    held_us: u64,
    /// Repeats accounted for so far (emitted or dropped by the cap).
    counted: u64,
}

impl Repeat {
    fn start(key: Key, action: Action) -> Self {
        Self {
            key,
            action,
            held_us: 0,
            counted: 0,
        }
    }
}

/// Turns key events and elapsed time into [`Action`]s.
///
/// Feed it every key press ([`key_down`](Self::key_down)) and release
/// ([`key_up`](Self::key_up)), then call [`update`](Self::update) once per
/// frame with the frame time. A press emits its action once. While a
/// repeatable action's key is held, it re-emits after the repeat delay and
/// then every interval. The most recently pressed repeatable key is the one
/// that repeats; releasing it hands repeating back to the most recent
/// repeatable key still held, which waits a full delay before repeating.
#[derive(Debug, Clone)]
pub struct InputState {
    keymap: Keymap,
    /// Actions from presses since the last update.
    pending: Vec<Action>,
    /// Bound keys currently down with the action they triggered, oldest first.
    held: Vec<(Key, Action)>,
    repeat: Option<Repeat>,
}

impl InputState {
    /// Input handling with the given bindings and repeat timings.
    pub fn new(keymap: Keymap) -> Self {
        Self {
            keymap,
            pending: Vec::new(),
            held: Vec::new(),
            repeat: None,
        }
    }

    /// The bindings in use.
    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    /// A key went down (with its modifier state). Unbound chords and keys
    /// already held are ignored.
    pub fn key_down(&mut self, chord: Chord) {
        if self.held.iter().any(|&(k, _)| k == chord.key) {
            return;
        }
        let Some(action) = self.keymap.action(chord) else {
            return;
        };
        self.held.push((chord.key, action));
        self.pending.push(action);
        if action.is_repeatable() {
            self.repeat = Some(Repeat::start(chord.key, action));
        }
    }

    /// A key went up. Stops its repeat, if it was the repeating one.
    pub fn key_up(&mut self, key: Key) {
        self.held.retain(|&(k, _)| k != key);
        if self.repeat.is_some_and(|r| r.key == key) {
            self.repeat = self
                .held
                .iter()
                .rev()
                .find(|(_, a)| a.is_repeatable())
                .map(|&(k, a)| Repeat::start(k, a));
        }
    }

    /// Whether any held key is bound to `action` (e.g. hold to fast-forward).
    pub fn is_held(&self, action: Action) -> bool {
        self.held.iter().any(|&(_, a)| a == action)
    }

    /// Advances time by `dt` seconds and returns the actions to handle this
    /// frame, in order: presses since the last update, then repeats (at most
    /// [`MAX_REPEATS_PER_UPDATE`]; repeats beyond the cap are dropped, not
    /// deferred). Negative or non-finite `dt` counts as zero.
    pub fn update(&mut self, dt: f32) -> Vec<Action> {
        let mut actions = std::mem::take(&mut self.pending);
        let RepeatDef {
            delay_ms,
            interval_ms,
        } = self.keymap.repeat;
        if let Some(r) = &mut self.repeat {
            r.held_us = r.held_us.saturating_add(seconds_to_micros(dt));
            let delay_us = u64::from(delay_ms) * 1000;
            // A zero interval is rejected by the loader; guard anyway.
            let interval_us = u64::from(interval_ms).max(1) * 1000;
            let due = match r.held_us.checked_sub(delay_us) {
                Some(past_delay) => 1 + past_delay / interval_us,
                None => 0,
            };
            let new = due.saturating_sub(r.counted).min(MAX_REPEATS_PER_UPDATE);
            r.counted = due;
            for _ in 0..new {
                actions.push(r.action);
            }
        }
        actions
    }
}

/// Converts seconds to whole microseconds, rounding to nearest. Rounding
/// per frame keeps millisecond-exact timings exact despite `f32` error.
fn seconds_to_micros(dt: f32) -> u64 {
    let Ok(d) = Duration::try_from_secs_f32(dt) else {
        return 0;
    };
    u64::try_from(d.as_nanos().saturating_add(500) / 1000).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use Action::{Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Info};

    fn chord(s: &str) -> Chord {
        Chord::parse(s).unwrap_or(Chord::plain(Key::F1))
    }

    /// A fixed test keymap, independent of the shipped defaults.
    fn test_def(delay_ms: u32, interval_ms: u32) -> KeymapDef {
        let bindings = [
            ("h", CursorLeft),
            ("Left", CursorLeft),
            ("j", CursorDown),
            ("k", CursorUp),
            ("l", CursorRight),
            ("f", Confirm),
            ("Shift+h", Info),
        ]
        .into_iter()
        .map(|(c, a)| (chord(c), a))
        .collect();
        KeymapDef {
            bindings,
            repeat: RepeatDef {
                delay_ms,
                interval_ms,
            },
        }
    }

    fn default_state() -> InputState {
        state_with(170, 55)
    }

    fn state_with(delay_ms: u32, interval_ms: u32) -> InputState {
        InputState::new(Keymap::from_def(&test_def(delay_ms, interval_ms)))
    }

    /// Seconds for `ms` milliseconds.
    fn ms(ms: u32) -> f32 {
        Duration::from_millis(u64::from(ms)).as_secs_f32()
    }

    #[test]
    fn keymap_lookup_with_and_without_shift() {
        let km = default_state().keymap().clone();
        assert_eq!(km.action(chord("h")), Some(CursorLeft));
        assert_eq!(km.action(chord("Shift+h")), Some(Info));
        assert_eq!(km.action(chord("Left")), Some(CursorLeft));
        assert_eq!(km.action(chord("Shift+Left")), None);
        assert_eq!(km.action(chord("Shift+f")), None);
        assert_eq!(km.action(chord("z")), None);
        assert_eq!(km.repeat(), RepeatDef::default());
        let custom = state_with(300, 40);
        assert_eq!(
            custom.keymap().repeat(),
            RepeatDef {
                delay_ms: 300,
                interval_ms: 40
            }
        );
    }

    #[test]
    fn chords_for_lists_every_binding_in_order() {
        let km = default_state().keymap().clone();
        assert_eq!(km.chords_for(CursorLeft), vec![chord("h"), chord("Left")]);
        assert_eq!(km.chords_for(Info), vec![chord("Shift+h")]);
        assert_eq!(km.chords_for(Action::Cancel), vec![]);
        assert_eq!(km.primary(CursorLeft), Some(chord("h")));
        assert_eq!(km.primary(Confirm), Some(chord("f")));
        assert_eq!(km.primary(Action::Cancel), None);
    }

    #[test]
    fn press_emits_once_immediately() {
        let mut s = default_state();
        s.key_down(chord("f"));
        assert_eq!(s.update(0.0), vec![Confirm]);
        assert_eq!(s.update(ms(1000)), vec![]);
        assert!(s.is_held(Confirm));
        s.key_up(Key::F);
        assert!(!s.is_held(Confirm));
        assert_eq!(s.update(ms(1000)), vec![]);
    }

    #[test]
    fn unbound_keys_are_ignored() {
        let mut s = default_state();
        s.key_down(chord("z"));
        s.key_up(Key::Z);
        assert_eq!(s.update(ms(500)), vec![]);
    }

    #[test]
    fn repeat_exact_timing() {
        let mut s = default_state();
        s.key_down(chord("l"));
        assert_eq!(s.update(ms(169)), vec![CursorRight]);
        assert_eq!(s.update(ms(1)), vec![CursorRight]); // 170: first repeat
        assert_eq!(s.update(ms(54)), vec![]); // 224
        assert_eq!(s.update(ms(1)), vec![CursorRight]); // 225
        assert_eq!(s.update(ms(55)), vec![CursorRight]); // 280
        assert_eq!(s.update(ms(110)), vec![CursorRight, CursorRight]); // 390
    }

    #[test]
    fn release_stops_repeat() {
        let mut s = default_state();
        s.key_down(chord("j"));
        assert_eq!(s.update(ms(200)), vec![CursorDown, CursorDown]);
        s.key_up(Key::J);
        assert_eq!(s.update(ms(1000)), vec![]);
        assert!(!s.is_held(CursorDown));
    }

    #[test]
    fn repeat_of_one_frame_press_and_release() {
        // Press and release within one frame: the press still counts.
        let mut s = default_state();
        s.key_down(chord("k"));
        s.key_up(Key::K);
        assert_eq!(s.update(ms(500)), vec![CursorUp]);
    }

    #[test]
    fn new_direction_takes_over_repeat() {
        let mut s = default_state();
        s.key_down(chord("l"));
        assert_eq!(s.update(ms(100)), vec![CursorRight]);
        s.key_down(chord("j"));
        // The new key starts its own delay; the old one no longer repeats.
        assert_eq!(s.update(ms(169)), vec![CursorDown]);
        assert_eq!(s.update(ms(1)), vec![CursorDown]);
        assert!(s.is_held(CursorRight));
        assert!(s.is_held(CursorDown));
    }

    #[test]
    fn releasing_the_old_direction_keeps_the_new_one_repeating() {
        let mut s = default_state();
        s.key_down(chord("l"));
        s.key_down(chord("j"));
        assert_eq!(s.update(ms(100)), vec![CursorRight, CursorDown]);
        s.key_up(Key::L);
        assert_eq!(s.update(ms(70)), vec![CursorDown]);
    }

    #[test]
    fn releasing_the_new_direction_resumes_the_old_after_a_delay() {
        let mut s = default_state();
        s.key_down(chord("l"));
        s.key_down(chord("j"));
        assert_eq!(
            s.update(ms(300)),
            vec![CursorRight, CursorDown, CursorDown, CursorDown, CursorDown]
        );
        s.key_up(Key::J);
        assert_eq!(s.update(ms(169)), vec![]);
        assert_eq!(s.update(ms(1)), vec![CursorRight]);
    }

    #[test]
    fn non_repeatable_press_does_not_stop_cursor_repeat() {
        let mut s = default_state();
        s.key_down(chord("l"));
        assert_eq!(s.update(ms(100)), vec![CursorRight]);
        s.key_down(chord("f"));
        s.key_up(Key::F);
        assert_eq!(s.update(ms(70)), vec![Confirm, CursorRight]);
    }

    #[test]
    fn repeated_key_down_while_held_is_ignored() {
        let mut s = default_state();
        s.key_down(chord("l"));
        assert_eq!(s.update(ms(100)), vec![CursorRight]);
        s.key_down(chord("l"));
        s.key_down(chord("Shift+l"));
        assert_eq!(s.update(ms(70)), vec![CursorRight]);
    }

    #[test]
    fn huge_dt_is_capped() {
        let mut s = default_state();
        s.key_down(chord("h"));
        let out = s.update(ms(10_000));
        assert_eq!(out.len(), 1 + 5);
        assert!(out.iter().all(|&a| a == CursorLeft));
        // Dropped repeats are not carried over into the next frame.
        assert_eq!(s.update(ms(55)), vec![CursorLeft]);
    }

    #[test]
    fn cap_counts_only_repeats() {
        // Exactly 5 repeats due: all emitted.
        let mut s = default_state();
        s.key_down(chord("h"));
        assert_eq!(s.update(ms(170 + 4 * 55)).len(), 1 + 5);
        // 6 due in one frame after the press frame: capped to 5.
        let mut s = default_state();
        s.key_down(chord("h"));
        assert_eq!(s.update(0.0).len(), 1);
        assert_eq!(s.update(ms(170 + 5 * 55)).len(), 5);
    }

    #[test]
    fn bad_dt_counts_as_zero() {
        let mut s = default_state();
        s.key_down(chord("h"));
        assert_eq!(s.update(-1.0), vec![CursorLeft]);
        assert_eq!(s.update(f32::NAN), vec![]);
        assert_eq!(s.update(f32::INFINITY), vec![]);
        assert_eq!(s.update(ms(170)), vec![CursorLeft]);
    }

    #[test]
    fn seconds_to_micros_rounds_to_nearest() {
        assert_eq!(seconds_to_micros(0.017), 17_000);
        assert_eq!(seconds_to_micros(0.000_000_4), 0);
        assert_eq!(seconds_to_micros(0.000_000_6), 1);
        assert_eq!(seconds_to_micros(1.0), 1_000_000);
        assert_eq!(seconds_to_micros(-0.5), 0);
    }

    #[test]
    fn zero_interval_does_not_hang() {
        let mut s = state_with(0, 0);
        s.key_down(chord("h"));
        assert_eq!(s.update(ms(10)).len(), 1 + 5);
    }

    proptest! {
        #[test]
        fn held_key_emits_expected_count(
            slices in prop::collection::vec(0u32..=100, 0..60),
            delay in 1u32..400,
            interval in 20u32..200,
        ) {
            let mut s = state_with(delay, interval);
            s.key_down(chord("l"));
            let mut total = 0;
            for &slice in &slices {
                let out = s.update(ms(slice));
                prop_assert!(out.iter().all(|&a| a == CursorRight));
                total += out.len();
            }
            if slices.is_empty() {
                total += s.update(0.0).len();
            }
            let t: u32 = slices.iter().sum();
            let expected = if t < delay { 1 } else { 2 + (t - delay) / interval };
            prop_assert_eq!(Ok(total), usize::try_from(expected));
        }
    }
}
