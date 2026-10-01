//! Keyboard and controller input as [`Action`]s (ADR-0006, ADR-0034): a
//! data-driven [`Keymap`] from chords and controller buttons to actions,
//! and [`InputState`], which turns presses and releases plus frame time
//! into the actions screens see, including repeat for held cursor keys and
//! buttons.
//!
//! [`Key`], [`Chord`], [`Button`], [`Action`] and [`Layout`] are defined in
//! `trpg-content` (its keymap loader validates them) and re-exported here.
//! The player's own bindings (3 slots per action, per layout) are in
//! [`bindings`]; [`pad`] turns the controllers' raw state into button
//! presses.
//!
//! **Fixed keys** (`docs/design/controls.md`, *Rebinding keys*; ADR-0031):
//! plain `Escape` is Cancel in every keymap, and `Delete` does nothing in
//! play (the Key bindings screen reads it to empty a slot). Neither can be
//! bound; see [`Chord::is_reserved`]. `Shift+Escape` and `Shift+Delete`
//! are ordinary chords.

pub mod bindings;
pub mod pad;

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

pub use bindings::{BindError, LayoutBindings, PlayerKeys, Slots};
pub use pad::{ButtonSet, PadId, PadKind, PadState, Pads};
pub use trpg_content::keymap::{
    Action, Button, Chord, Key, KeymapDef, Layout, LayoutKeys, PadKeys, RepeatDef, SLOTS, StickDef,
};

/// The keys every keymap has on top of its own bindings, whatever the
/// player binds: plain `Escape` cancels.
const FIXED: [(Chord, Action); 1] = [(Chord::plain(Key::Escape), Action::Cancel)];

/// The chord that backs out of the Key bindings screen's "Press a key…"
/// without changing anything.
const CAPTURE_ABORT: Chord = Chord::plain(Key::Escape);

/// The chord that empties the highlighted slot on the Key bindings screen.
const CLEAR_SLOT: Chord = Chord::plain(Key::Delete);

/// What a slot on the Key bindings screen shows while it waits for a key
/// (`docs/design/controls.md`, *Rebinding keys*).
pub const CAPTURE_PROMPT: &str = "Press a key…";

/// Whether `chord` backs out of the Key bindings screen's "Press a key…"
/// (`docs/design/controls.md`, *Rebinding keys*). The screen asks this
/// rather than naming the key (the `keyboard-input` skill).
pub fn is_capture_abort(chord: Chord) -> bool {
    chord == CAPTURE_ABORT
}

/// Whether `chord` empties the highlighted slot on the Key bindings screen.
pub fn is_clear_slot(chord: Chord) -> bool {
    chord == CLEAR_SLOT
}

/// The name of the [`is_capture_abort`] key, for help text.
pub fn capture_abort_key_name() -> String {
    CAPTURE_ABORT.to_string()
}

/// The name of the [`is_clear_slot`] key, for help text.
pub fn clear_slot_key_name() -> String {
    CLEAR_SLOT.to_string()
}

/// A text box's own keys (the lead's name, 0801): fixed like `Escape` for
/// Cancel, since while a player types, every letter key is a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    /// `Enter`: keep the text.
    Done,
    /// `Backspace`: delete the last character.
    Delete,
    /// `Escape`: close the box, text unchanged.
    Cancel,
}

/// The text-box key `chord` is, if any (Shift or not).
pub fn text_key(chord: Chord) -> Option<TextKey> {
    match chord.key {
        Key::Enter => Some(TextKey::Done),
        Key::Backspace => Some(TextKey::Delete),
        Key::Escape => Some(TextKey::Cancel),
        _ => None,
    }
}

/// The help line of a text box: `Enter done · Backspace delete · Escape
/// cancel`. Lives here because it names keys (only this module may).
pub fn text_keys_help() -> String {
    let name = |key| Chord::plain(key).to_string();
    crate::widgets::help::help_line(&[
        (Some(name(Key::Enter)), "done"),
        (Some(name(Key::Backspace)), "delete"),
        (Some(name(Key::Escape)), "cancel"),
    ])
}

/// At most this many repeats are emitted by one [`InputState::update`], so a
/// lag spike can't teleport the cursor across the map.
pub const MAX_REPEATS_PER_UPDATE: u64 = 5;

/// Lookup from chord, and from controller button, to action, plus repeat
/// timings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    /// Every chord that does something, fixed keys included.
    bindings: HashMap<Chord, Action>,
    /// Each action's own chords (not the fixed keys), in the order given.
    chords: BTreeMap<Action, Vec<Chord>>,
    /// Every controller button that does something.
    pad: HashMap<Button, Action>,
    /// Each action's buttons, in the order given.
    buttons: BTreeMap<Action, Vec<Button>>,
    repeat: RepeatDef,
}

impl Keymap {
    /// A keymap with `bindings` plus the fixed keys (plain `Escape` is
    /// Cancel). A chord listed twice keeps its last action;
    /// [reserved](Chord::is_reserved) chords (plain `Escape` and `Delete`)
    /// are left out.
    pub fn new(bindings: impl IntoIterator<Item = (Chord, Action)>, repeat: RepeatDef) -> Self {
        let mut lookup = HashMap::new();
        let mut chords: BTreeMap<Action, Vec<Chord>> = BTreeMap::new();
        for (chord, action) in bindings {
            if chord.is_reserved() {
                continue;
            }
            if let Some(old) = lookup.insert(chord, action)
                && let Some(list) = chords.get_mut(&old)
            {
                list.retain(|&c| c != chord);
            }
            chords.entry(action).or_default().push(chord);
        }
        lookup.extend(FIXED);
        Self {
            bindings: lookup,
            chords,
            pad: HashMap::new(),
            buttons: BTreeMap::new(),
            repeat,
        }
    }

    /// This keymap with `pad` as its controller buttons, replacing any it
    /// had. A button listed twice keeps its last action.
    #[must_use]
    pub fn with_pad(mut self, pad: impl IntoIterator<Item = (Button, Action)>) -> Self {
        self.pad.clear();
        self.buttons.clear();
        for (button, action) in pad {
            if let Some(old) = self.pad.insert(button, action)
                && let Some(list) = self.buttons.get_mut(&old)
            {
                list.retain(|&b| b != button);
            }
            self.buttons.entry(action).or_default().push(button);
        }
        self
    }

    /// This keymap with the default controller buttons from a validated
    /// keymap definition: the same in every layout and the layout picker
    /// (`docs/design/controls.md`, *Controller*).
    #[must_use]
    pub fn with_default_pad(self, def: &KeymapDef) -> Self {
        let pad = def.pad.iter();
        self.with_pad(pad.flat_map(|(&a, buttons)| buttons.iter().map(move |&b| (b, a))))
    }

    /// `layout`'s default bindings from a validated keymap definition (no
    /// player changes; for those see [`PlayerKeys`]), with its repeat
    /// timings and the default controller buttons. A definition without
    /// that layout (only possible when built by hand) gives a keymap with
    /// only the fixed keys and the buttons.
    pub fn for_layout(def: &KeymapDef, layout: Layout) -> Self {
        let keys = def.layouts.get(&layout).into_iter().flatten();
        let bindings = keys.flat_map(|(&a, chords)| chords.iter().map(move |&c| (c, a)));
        Self::new(bindings, def.repeat).with_default_pad(def)
    }

    /// The keys that work before any layout is chosen, from the keymap
    /// definition's `layout_picker` section, so the layout picker can be
    /// used whichever hand the player types with. Actions it doesn't list
    /// (e.g. Cancel: a layout must be picked) have no key. The controller
    /// buttons are the default ones, as in every layout.
    pub fn layout_picker(def: &KeymapDef) -> Self {
        let bindings = def.layout_picker.iter().map(|(&c, &a)| (c, a));
        Self::new(bindings, def.repeat).with_default_pad(def)
    }

    /// The action bound to `chord`, if any. `Shift+h` and `h` are distinct:
    /// there is no fallback from a shifted chord to the plain one.
    pub fn action(&self, chord: Chord) -> Option<Action> {
        self.bindings.get(&chord).copied()
    }

    /// The action bound to the controller button `button`, if any.
    pub fn pad_action(&self, button: Button) -> Option<Action> {
        self.pad.get(&button).copied()
    }

    /// Every controller button bound to `action`, in the order they were
    /// given (file order for `keymap.ron`).
    pub fn buttons_for(&self, action: Action) -> Vec<Button> {
        self.buttons.get(&action).cloned().unwrap_or_default()
    }

    /// Key-repeat timings.
    pub fn repeat(&self) -> RepeatDef {
        self.repeat
    }

    /// Every chord bound to `action` (its key slots), in the order they
    /// were given: slot order for the player's bindings, file order for
    /// `keymap.ron`. Leaves out the fixed keys (see
    /// [`fixed_chords_for`](Self::fixed_chords_for)).
    pub fn chords_for(&self, action: Action) -> Vec<Chord> {
        self.chords.get(&action).cloned().unwrap_or_default()
    }

    /// The fixed chords that also trigger `action` in every keymap and
    /// can't be rebound: plain `Escape` for Cancel, nothing for the rest.
    pub fn fixed_chords_for(action: Action) -> Vec<Chord> {
        FIXED
            .iter()
            .filter(|&&(_, a)| a == action)
            .map(|&(c, _)| c)
            .collect()
    }

    /// What moves the cursor, for help text: `arrows` when the four cursor
    /// actions' [`primary`](Self::primary) chords are the plain arrow keys,
    /// otherwise those chords in up-left-down-right order (`wasd`). `None`
    /// if any cursor action is unbound. Lives here, not in `widgets::help`,
    /// because it names keys (only this module may).
    pub fn cursor_keys_name(&self) -> Option<String> {
        let order = [
            (Action::CursorUp, Key::Up),
            (Action::CursorLeft, Key::Left),
            (Action::CursorDown, Key::Down),
            (Action::CursorRight, Key::Right),
        ];
        let chords = order
            .iter()
            .map(|&(action, _)| self.primary(action))
            .collect::<Option<Vec<_>>>()?;
        let arrows = chords
            .iter()
            .zip(order)
            .all(|(chord, (_, arrow))| *chord == Chord::plain(arrow));
        Some(if arrows {
            "arrows".to_owned()
        } else {
            chords.iter().map(ToString::to_string).collect()
        })
    }

    /// The chord help text names for `action`: the first of
    /// [`chords_for`](Self::chords_for), or `None` if it has none.
    pub fn primary(&self, action: Action) -> Option<Chord> {
        self.chords_for(action).into_iter().next()
    }

    /// The action that picks on the map with the cursor (a unit, its tile,
    /// a target): [`Action::Select`] once it has a key, else
    /// [`Action::Confirm`] (`docs/design/controls.md`, *Optional split
    /// keys*).
    pub fn select_action(&self) -> Action {
        if self.primary(Action::Select).is_some() {
            Action::Select
        } else {
            Action::Confirm
        }
    }

    /// The actions that accept the end-turn prompt, the one help text names
    /// first: [`Action::ConfirmEndTurn`] and Confirm once it has a key,
    /// else End turn (pressed again) and Confirm.
    pub fn end_turn_accept_actions(&self) -> [Action; 2] {
        if self.primary(Action::ConfirmEndTurn).is_some() {
            [Action::ConfirmEndTurn, Action::Confirm]
        } else {
            [Action::EndTurn, Action::Confirm]
        }
    }
}

/// Something the player can hold down: a keyboard key or a controller
/// button (a stick direction counts as a button).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Input {
    /// A keyboard key.
    Key(Key),
    /// A controller button.
    Pad(Button),
}

/// The repeat currently running for the most recently pressed repeatable
/// key or button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Repeat {
    input: Input,
    action: Action,
    /// Time held so far, in microseconds.
    held_us: u64,
    /// Repeats accounted for so far (emitted or dropped by the cap).
    counted: u64,
}

impl Repeat {
    fn start(input: Input, action: Action) -> Self {
        Self {
            input,
            action,
            held_us: 0,
            counted: 0,
        }
    }
}

/// Turns key and controller-button events and elapsed time into
/// [`Action`]s.
///
/// Feed it every key press ([`key_down`](Self::key_down)) and release
/// ([`key_up`](Self::key_up)), and every controller button press
/// ([`pad_down`](Self::pad_down)) and release ([`pad_up`](Self::pad_up)),
/// then call [`update`](Self::update) once per frame with the frame time.
/// Keys and buttons follow the same rules. A press emits its action once.
/// While a repeatable action's key or button is held, it re-emits after the
/// repeat delay and then every interval. The most recently pressed
/// repeatable one is the one that repeats; releasing it hands repeating
/// back to the most recent repeatable one still held, which waits a full
/// delay before repeating.
#[derive(Debug, Clone)]
pub struct InputState {
    keymap: Keymap,
    /// Actions from presses since the last update.
    pending: Vec<Action>,
    /// Bound keys and buttons currently down with the action they
    /// triggered, oldest first.
    held: Vec<(Input, Action)>,
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

    /// Switches to other bindings (the player picked a layout). Forgets held
    /// keys, pending presses and any running repeat, so nothing pressed
    /// under the old bindings leaks into the new ones; a key still down is
    /// ignored until it is pressed again.
    pub fn set_keymap(&mut self, keymap: Keymap) {
        *self = Self::new(keymap);
    }

    /// A key went down (with its modifier state). Unbound chords and keys
    /// already held are ignored.
    pub fn key_down(&mut self, chord: Chord) {
        self.press(Input::Key(chord.key), self.keymap.action(chord));
    }

    /// A key went up. Stops its repeat, if it was the repeating one.
    pub fn key_up(&mut self, key: Key) {
        self.release(Input::Key(key));
    }

    /// A controller button went down, on any pad ([`Pads`] merges them and
    /// gives the button's binding position). Unbound buttons and buttons
    /// already held are ignored.
    pub fn pad_down(&mut self, button: Button) {
        self.press(Input::Pad(button), self.keymap.pad_action(button));
    }

    /// A controller button went up. Stops its repeat, if it was the
    /// repeating one.
    pub fn pad_up(&mut self, button: Button) {
        self.release(Input::Pad(button));
    }

    /// `input` went down, bound to `action` (if any).
    fn press(&mut self, input: Input, action: Option<Action>) {
        if self.held.iter().any(|&(i, _)| i == input) {
            return;
        }
        let Some(action) = action else {
            return;
        };
        self.held.push((input, action));
        self.pending.push(action);
        if action.is_repeatable() {
            self.repeat = Some(Repeat::start(input, action));
        }
    }

    /// `input` went up.
    fn release(&mut self, input: Input) {
        self.held.retain(|&(i, _)| i != input);
        if self.repeat.is_some_and(|r| r.input == input) {
            self.repeat = self
                .held
                .iter()
                .rev()
                .find(|(_, a)| a.is_repeatable())
                .map(|&(i, a)| Repeat::start(i, a));
        }
    }

    /// Whether any held key or button is bound to `action` (e.g. hold to
    /// fast-forward).
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
    fn test_keymap(delay_ms: u32, interval_ms: u32) -> Keymap {
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
        .map(|(c, a)| (chord(c), a));
        Keymap::new(
            bindings,
            RepeatDef {
                delay_ms,
                interval_ms,
            },
        )
    }

    fn default_state() -> InputState {
        state_with(170, 55)
    }

    fn state_with(delay_ms: u32, interval_ms: u32) -> InputState {
        InputState::new(test_keymap(delay_ms, interval_ms))
    }

    /// The buttons [`pad_state`] adds to the test keymap.
    const TEST_PAD: [(Button, Action); 5] = [
        (Button::DpadLeft, CursorLeft),
        (Button::DpadDown, CursorDown),
        (Button::LeftStickDown, CursorDown),
        (Button::DpadRight, CursorRight),
        (Button::South, Confirm),
    ];

    /// [`default_state`] with a few controller buttons bound too.
    fn pad_state() -> InputState {
        InputState::new(test_keymap(170, 55).with_pad(TEST_PAD))
    }

    /// Seconds for `ms` milliseconds.
    fn ms(ms: u32) -> f32 {
        Duration::from_millis(u64::from(ms)).as_secs_f32()
    }

    #[test]
    fn only_the_plain_fixed_chords_abort_capture_and_clear_a_slot() {
        assert!(is_capture_abort(chord("Escape")));
        assert!(!is_capture_abort(chord("Shift+Escape")));
        assert!(!is_capture_abort(chord("Delete")));
        assert!(is_clear_slot(chord("Delete")));
        assert!(!is_clear_slot(chord("Shift+Delete")));
        assert!(!is_clear_slot(chord("Escape")));
        assert_eq!(capture_abort_key_name(), "Escape");
        assert_eq!(clear_slot_key_name(), "Delete");
        // Both are reserved, so neither can end up in a slot.
        assert!(chord("Escape").is_reserved() && chord("Delete").is_reserved());
    }

    #[test]
    fn select_falls_back_to_confirm_until_it_has_a_key() {
        let km = test_keymap(170, 55);
        assert_eq!(km.select_action(), Confirm);
        let bound = Keymap::new([(chord("g"), Action::Select)], km.repeat());
        assert_eq!(bound.select_action(), Action::Select);
    }

    #[test]
    fn end_turn_is_accepted_by_end_turn_until_confirm_end_turn_has_a_key() {
        let km = test_keymap(170, 55);
        assert_eq!(km.end_turn_accept_actions(), [Action::EndTurn, Confirm]);
        let bound = Keymap::new([(chord("Enter"), Action::ConfirmEndTurn)], km.repeat());
        assert_eq!(
            bound.end_turn_accept_actions(),
            [Action::ConfirmEndTurn, Confirm]
        );
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
        assert_eq!(
            km.repeat(),
            RepeatDef {
                delay_ms: 170,
                interval_ms: 55
            }
        );
        let custom = state_with(400, 40);
        assert_eq!(
            custom.keymap().repeat(),
            RepeatDef {
                delay_ms: 400,
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
    fn with_pad_looks_buttons_up_and_lists_them_in_order() {
        let km = pad_state().keymap().clone();
        assert_eq!(km.pad_action(Button::South), Some(Confirm));
        assert_eq!(km.pad_action(Button::LeftStickDown), Some(CursorDown));
        assert_eq!(km.pad_action(Button::East), None);
        assert_eq!(
            km.buttons_for(CursorDown),
            vec![Button::DpadDown, Button::LeftStickDown]
        );
        assert_eq!(km.buttons_for(Info), vec![]);
        // The keys are untouched, and a keymap has no buttons by itself.
        assert_eq!(km.action(chord("f")), Some(Confirm));
        assert_eq!(km.chords_for(CursorLeft), vec![chord("h"), chord("Left")]);
        let keys_only = test_keymap(170, 55);
        assert_eq!(keys_only.pad_action(Button::South), None);
        assert_eq!(keys_only.buttons_for(Confirm), vec![]);
        assert_ne!(km, keys_only);
    }

    #[test]
    fn a_repeated_button_keeps_its_last_action_and_with_pad_replaces_the_table() {
        let km = test_keymap(170, 55).with_pad([
            (Button::South, Confirm),
            (Button::East, Confirm),
            (Button::North, Info),
            (Button::South, Info),
        ]);
        assert_eq!(km.buttons_for(Confirm), vec![Button::East]);
        assert_eq!(km.buttons_for(Info), vec![Button::North, Button::South]);
        assert_eq!(km.pad_action(Button::South), Some(Info));
        let km = km.with_pad([(Button::West, Confirm)]);
        assert_eq!(km.buttons_for(Confirm), vec![Button::West]);
        assert_eq!(km.buttons_for(Info), vec![]);
        assert_eq!(km.pad_action(Button::South), None);
        assert_eq!(km.pad_action(Button::East), None);
    }

    #[test]
    fn every_keymap_from_the_definition_has_the_default_buttons() {
        let def = KeymapDef::load().unwrap_or_default();
        let keymaps = [
            Keymap::for_layout(&def, Layout::RightHanded),
            Keymap::for_layout(&def, Layout::LeftHanded),
            Keymap::layout_picker(&def),
            PlayerKeys::default().keymap(&def, Layout::LeftHanded),
            Keymap::new([], def.repeat).with_default_pad(&def),
        ];
        for km in keymaps {
            assert_eq!(km.pad_action(Button::South), Some(Confirm));
            assert_eq!(km.pad_action(Button::East), Some(Action::Cancel));
            assert_eq!(km.pad_action(Button::Start), Some(Action::EndTurn));
            assert_eq!(
                km.buttons_for(CursorUp),
                vec![Button::DpadUp, Button::LeftStickUp]
            );
            // Unused by default.
            assert_eq!(km.pad_action(Button::RightTrigger), None);
            assert_eq!(km.pad_action(Button::RightStickUp), None);
            assert_eq!(km.buttons_for(Action::Debug), vec![]);
            for action in Action::ALL {
                assert_eq!(km.buttons_for(action), def.buttons(action), "{action}");
            }
        }
        // A layout's keys alone carry no buttons.
        let keys = LayoutBindings::defaults(&def, Layout::RightHanded).keymap(def.repeat);
        assert_eq!(keys.pad_action(Button::South), None);
        assert_eq!(keys.clone().with_default_pad(&def), {
            PlayerKeys::default().keymap(&def, Layout::RightHanded)
        });
        // A definition built by hand without buttons gives none.
        let empty = Keymap::for_layout(&KeymapDef::default(), Layout::LeftHanded);
        assert_eq!(empty.pad_action(Button::South), None);
    }

    #[test]
    fn for_layout_picks_that_layouts_bindings() {
        let def = trpg_content::load_embedded()
            .map(|c| c.keymap)
            .unwrap_or_default();
        let right = Keymap::for_layout(&def, Layout::RightHanded);
        let left = Keymap::for_layout(&def, Layout::LeftHanded);
        assert_eq!(right.action(chord("f")), Some(Confirm));
        assert_eq!(right.action(chord("j")), None);
        assert_eq!(left.action(chord("j")), Some(Confirm));
        assert_eq!(left.action(chord("f")), None);
        assert_eq!(left.action(chord(";")), Some(Action::PrevUnit));
        assert_eq!(left.repeat(), def.repeat);
        assert_eq!(right.chords_for(Confirm).len(), 1);
        let empty = Keymap::for_layout(&KeymapDef::default(), Layout::LeftHanded);
        assert_eq!(empty.primary(Confirm), None);
    }

    #[test]
    fn escape_always_cancels_and_delete_does_nothing() {
        let def = KeymapDef::load().unwrap_or_default();
        let keymaps = [
            Keymap::for_layout(&def, Layout::RightHanded),
            Keymap::for_layout(&def, Layout::LeftHanded),
            Keymap::layout_picker(&def),
            Keymap::new([], RepeatDef::default()),
            // Even a keymap that tries to bind them elsewhere.
            Keymap::new(
                [(chord("Escape"), Confirm), (chord("Delete"), Confirm)],
                RepeatDef::default(),
            ),
        ];
        for km in keymaps {
            assert_eq!(km.action(chord("Escape")), Some(Action::Cancel));
            assert_eq!(km.action(chord("Shift+Escape")), None);
            assert_eq!(km.action(chord("Delete")), None);
            assert_eq!(km.action(chord("Shift+Delete")), None);
            assert!(!km.chords_for(Action::Cancel).contains(&chord("Escape")));
            assert!(!km.chords_for(Confirm).contains(&chord("Escape")));
        }
        // Shifted, they are ordinary chords.
        let km = Keymap::new(
            [
                (chord("Shift+Escape"), Info),
                (chord("Shift+Delete"), Confirm),
            ],
            RepeatDef::default(),
        );
        assert_eq!(km.action(chord("Shift+Escape")), Some(Info));
        assert_eq!(km.action(chord("Shift+Delete")), Some(Confirm));
        assert_eq!(km.action(chord("Escape")), Some(Action::Cancel));
        assert_eq!(km.chords_for(Info), vec![chord("Shift+Escape")]);
        assert_eq!(
            Keymap::fixed_chords_for(Action::Cancel),
            vec![chord("Escape")]
        );
        assert!(
            Action::ALL
                .into_iter()
                .filter(|&a| a != Action::Cancel)
                .all(|a| Keymap::fixed_chords_for(a).is_empty())
        );
    }

    #[test]
    fn chords_keep_the_order_given_and_a_repeated_chord_its_last_action() {
        let km = Keymap::new(
            [
                (chord("Space"), Confirm),
                (chord("f"), Confirm),
                (chord("e"), Info),
                (chord("Space"), Info),
            ],
            RepeatDef::default(),
        );
        assert_eq!(km.chords_for(Confirm), vec![chord("f")]);
        assert_eq!(km.chords_for(Info), vec![chord("e"), chord("Space")]);
        assert_eq!(km.action(chord("Space")), Some(Info));
        assert_eq!(km.primary(Info), Some(chord("e")));
    }

    #[test]
    fn layout_picker_keys_work_for_either_hand() {
        let def = KeymapDef::load().unwrap_or_default();
        let km = Keymap::layout_picker(&def);
        for (c, a) in [
            ("Up", CursorUp),
            ("w", CursorUp),
            ("Down", CursorDown),
            ("s", CursorDown),
            ("f", Confirm),
            ("j", Confirm),
            ("Enter", Confirm),
            ("Space", Confirm),
        ] {
            assert_eq!(km.action(chord(c)), Some(a), "{c}");
        }
        assert_eq!(km.chords_for(Confirm).len(), 4);
        assert_eq!(km.primary(Action::Cancel), None);
        assert_eq!(km.action(chord("d")), None);
        assert_eq!(km.repeat(), def.repeat);
    }

    #[test]
    fn set_keymap_switches_bindings_and_forgets_held_keys() {
        let mut s = default_state();
        s.key_down(chord("l"));
        s.key_down(chord("f"));
        s.set_keymap(Keymap::layout_picker(
            &KeymapDef::load().unwrap_or_default(),
        ));
        assert!(!s.is_held(Confirm));
        assert_eq!(s.update(ms(1000)), vec![]);
        // `f` is still physically down: ignored until pressed again.
        s.key_up(Key::F);
        s.key_down(chord("j"));
        assert_eq!(s.update(0.0), vec![Confirm]);
        assert_eq!(s.keymap().action(chord("l")), None);
    }

    #[test]
    fn set_keymap_forgets_held_buttons_too() {
        let mut s = pad_state();
        s.pad_down(Button::DpadRight);
        s.pad_down(Button::South);
        s.set_keymap(test_keymap(170, 55).with_pad([(Button::East, Confirm)]));
        assert!(!s.is_held(Confirm) && !s.is_held(CursorRight));
        assert_eq!(s.update(ms(1000)), vec![]);
        // South is unbound now; East confirms.
        s.pad_up(Button::South);
        s.pad_down(Button::South);
        s.pad_down(Button::East);
        assert_eq!(s.update(0.0), vec![Confirm]);
    }

    #[test]
    fn a_button_press_emits_once_and_is_held_until_released() {
        let mut s = pad_state();
        s.pad_down(Button::South);
        assert_eq!(s.update(0.0), vec![Confirm]);
        assert_eq!(s.update(ms(1000)), vec![]);
        assert!(s.is_held(Confirm));
        // Reported down again while held (a second pad): ignored.
        s.pad_down(Button::South);
        assert_eq!(s.update(0.0), vec![]);
        s.pad_up(Button::South);
        assert!(!s.is_held(Confirm));
        assert_eq!(s.update(ms(1000)), vec![]);
    }

    #[test]
    fn unbound_buttons_are_ignored() {
        let mut s = pad_state();
        s.pad_down(Button::RightTrigger);
        assert!(s.held.is_empty());
        s.pad_up(Button::RightTrigger);
        assert_eq!(s.update(ms(500)), vec![]);
        // Without buttons in the keymap, every button is unbound.
        let mut s = default_state();
        s.pad_down(Button::South);
        assert_eq!(s.update(ms(500)), vec![]);
    }

    #[test]
    fn a_held_button_repeats_with_the_key_timings() {
        let mut s = pad_state();
        s.pad_down(Button::DpadRight);
        assert_eq!(s.update(ms(169)), vec![CursorRight]);
        assert_eq!(s.update(ms(1)), vec![CursorRight]); // 170: first repeat
        assert_eq!(s.update(ms(54)), vec![]); // 224
        assert_eq!(s.update(ms(1)), vec![CursorRight]); // 225
        assert_eq!(s.update(ms(110)), vec![CursorRight, CursorRight]); // 335
        s.pad_up(Button::DpadRight);
        assert_eq!(s.update(ms(1000)), vec![]);
        assert!(!s.is_held(CursorRight));
    }

    #[test]
    fn a_key_and_a_button_on_one_action_are_separate_inputs() {
        let mut s = pad_state();
        s.key_down(chord("f"));
        s.pad_down(Button::South);
        assert_eq!(s.update(0.0), vec![Confirm, Confirm]);
        // Letting go of one leaves the action held by the other.
        s.key_up(Key::F);
        assert!(s.is_held(Confirm));
        assert_eq!(s.held, vec![(Input::Pad(Button::South), Confirm)]);
        s.pad_up(Button::South);
        assert!(!s.is_held(Confirm));
        // Likewise for two directions' repeat: the stick takes over from
        // the D-pad, and the D-pad resumes after a full delay.
        s.pad_down(Button::DpadDown);
        s.pad_down(Button::LeftStickDown);
        assert_eq!(s.update(ms(170)), vec![CursorDown, CursorDown, CursorDown]);
        s.pad_up(Button::LeftStickDown);
        assert_eq!(s.update(ms(169)), vec![]);
        assert_eq!(s.update(ms(1)), vec![CursorDown]);
    }

    #[test]
    fn the_newest_of_keys_and_buttons_repeats() {
        let mut s = pad_state();
        s.key_down(chord("l"));
        assert_eq!(s.update(ms(100)), vec![CursorRight]);
        // A button pressed later takes over the repeat from the key...
        s.pad_down(Button::DpadDown);
        assert_eq!(s.update(ms(169)), vec![CursorDown]);
        assert_eq!(s.update(ms(1)), vec![CursorDown]);
        assert!(s.is_held(CursorRight) && s.is_held(CursorDown));
        // ...and hands it back when released, after a full delay.
        s.pad_up(Button::DpadDown);
        assert_eq!(s.update(ms(169)), vec![]);
        assert_eq!(s.update(ms(1)), vec![CursorRight]);
        // And the other way round: a key takes over from a button.
        let mut s = pad_state();
        s.pad_down(Button::DpadLeft);
        s.key_down(chord("j"));
        assert_eq!(s.update(ms(170)), vec![CursorLeft, CursorDown, CursorDown]);
        // Releasing the older, non-repeating one changes nothing.
        s.pad_up(Button::DpadLeft);
        assert_eq!(s.update(ms(55)), vec![CursorDown]);
        // A non-repeatable button doesn't stop the key's repeat.
        s.pad_down(Button::South);
        s.pad_up(Button::South);
        assert_eq!(s.update(ms(55)), vec![Confirm, CursorDown]);
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

    /// One step of a random input script.
    #[derive(Debug, Clone, Copy)]
    enum Step {
        KeyDown(Chord),
        KeyUp(Key),
        PadDown(Button),
        PadUp(Button),
        Wait(u32),
    }

    fn arb_step() -> impl Strategy<Value = Step> {
        // Bound and unbound keys and buttons, repeatable and not.
        let key = || prop::sample::select(vec![Key::H, Key::J, Key::L, Key::F, Key::Z]);
        let button = || {
            prop::sample::select(vec![
                Button::DpadLeft,
                Button::DpadDown,
                Button::LeftStickDown,
                Button::South,
                Button::North,
            ])
        };
        prop_oneof![
            (key(), any::<bool>()).prop_map(|(key, shift)| Step::KeyDown(Chord { key, shift })),
            key().prop_map(Step::KeyUp),
            button().prop_map(Step::PadDown),
            button().prop_map(Step::PadUp),
            (0u32..400).prop_map(Step::Wait),
        ]
    }

    proptest! {
        /// Any mix of key and button presses and releases: nothing stays
        /// held after its release, only bound inputs are held (each once,
        /// oldest first), and the one repeating is always the newest held
        /// cursor input.
        #[test]
        fn keys_and_buttons_never_stick_and_only_the_newest_cursor_input_repeats(
            steps in prop::collection::vec(arb_step(), 0..60),
        ) {
            let mut s = pad_state();
            let km = s.keymap().clone();
            // What should be held, oldest first, and the presses not yet
            // handed out.
            let mut held: Vec<(Input, Action)> = Vec::new();
            let mut pending = Vec::new();
            for &step in &steps {
                let press = match step {
                    Step::KeyDown(chord) => {
                        s.key_down(chord);
                        Some((Input::Key(chord.key), km.action(chord)))
                    }
                    Step::PadDown(button) => {
                        s.pad_down(button);
                        Some((Input::Pad(button), km.pad_action(button)))
                    }
                    _ => None,
                };
                if let Some((input, Some(action))) = press
                    && !held.iter().any(|&(i, _)| i == input)
                {
                    held.push((input, action));
                    pending.push(action);
                }
                let release = match step {
                    Step::KeyUp(key) => {
                        s.key_up(key);
                        Some(Input::Key(key))
                    }
                    Step::PadUp(button) => {
                        s.pad_up(button);
                        Some(Input::Pad(button))
                    }
                    _ => None,
                };
                if let Some(input) = release {
                    held.retain(|&(i, _)| i != input);
                    prop_assert!(!s.held.iter().any(|&(i, _)| i == input));
                }
                prop_assert_eq!(&s.held, &held);
                let newest = held.iter().rev().find(|(_, a)| a.is_repeatable());
                prop_assert_eq!(s.repeat.map(|r| (r.input, r.action)), newest.copied());
                for action in Action::ALL {
                    prop_assert_eq!(s.is_held(action), held.iter().any(|&(_, a)| a == action));
                }
                if let Step::Wait(millis) = step {
                    let out = s.update(ms(millis));
                    // The presses, in order, then only the newest cursor
                    // input's repeats.
                    prop_assert!(out.len() >= pending.len());
                    prop_assert_eq!(&out[..pending.len()], &pending[..]);
                    let repeats = &out[pending.len()..];
                    match newest {
                        Some(&(_, action)) => prop_assert!(repeats.iter().all(|&a| a == action)),
                        None => prop_assert!(repeats.is_empty()),
                    }
                    pending.clear();
                }
            }
        }

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
