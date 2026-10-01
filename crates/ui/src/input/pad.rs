//! Controllers (ticket 0219, ADR-0034): from each connected pad's raw state
//! to the [`Button`] presses and releases [`InputState`] is fed. Pure:
//! `app` fills a [`PadState`] per pad every frame (gilrs on native, the
//! browser's Gamepad API on web) and [`Pads::update`] does the rest, so both
//! platforms share these rules (`docs/design/controls.md`, *Controller*):
//!
//! - **Sticks are 4-way buttons.** A stick pushed far enough counts as one
//!   held direction button, the direction it is pushed furthest, so it
//!   repeats like a held key and never moves diagonally.
//! - **Switch-style pads swap Confirm and Cancel.** Bindings name button
//!   *positions*; on a [`PadKind::Nintendo`] pad the bottom and right face
//!   buttons trade places before the keymap sees them.
//! - **Several pads drive the game as one.** A button held on two pads is
//!   one press, released when the last pad lets go; unplugging a pad
//!   releases what it held.
//!
//! [`InputState`]: super::InputState

use std::collections::BTreeMap;

use super::{Button, StickDef};

/// USB vendor ids of the pad makers the game tells apart.
const MICROSOFT: u16 = 0x045e;
const SONY: u16 = 0x054c;
const NINTENDO: u16 = 0x057e;

/// Who made a pad, which decides the Confirm / Cancel swap here and the
/// button names shown on screen (ticket 0220).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum PadKind {
    /// A Microsoft pad.
    Xbox,
    /// A Sony pad.
    PlayStation,
    /// A Nintendo pad (Switch Pro controller, Joy-Cons).
    Nintendo,
    /// Anything else, or a pad that doesn't say. Treated like an Xbox pad.
    #[default]
    Generic,
}

impl PadKind {
    /// The kind of a pad from its USB vendor id; unknown vendors (and 0,
    /// "not reported") are [`Generic`](Self::Generic).
    pub fn from_vendor(vendor: u16) -> Self {
        match vendor {
            MICROSOFT => Self::Xbox,
            SONY => Self::PlayStation,
            NINTENDO => Self::Nintendo,
            _ => Self::Generic,
        }
    }

    /// The position bindings use for the button physically at `button` on
    /// this kind of pad: on a Nintendo pad the bottom and right face buttons
    /// swap (so its right button, labelled `A`, is `South` and confirms by
    /// default); every other button, and every other pad, is unchanged.
    pub fn position(self, button: Button) -> Button {
        match (self, button) {
            (Self::Nintendo, Button::South) => Button::East,
            (Self::Nintendo, Button::East) => Button::South,
            _ => button,
        }
    }
}

/// A set of [`Button`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ButtonSet(u32);

impl ButtonSet {
    /// No buttons.
    pub const EMPTY: Self = Self(0);

    /// Adds `button`.
    pub fn insert(&mut self, button: Button) {
        self.0 |= bit(button);
    }

    /// Whether `button` is in the set.
    pub fn contains(self, button: Button) -> bool {
        self.0 & bit(button) != 0
    }

    /// The buttons in either set.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// The buttons in the set, in [`Button::ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Button> {
        Button::ALL
            .iter()
            .copied()
            .filter(move |&b| self.contains(b))
    }
}

/// `button`'s bit: its index in [`Button::ALL`].
fn bit(button: Button) -> u32 {
    1 << (button as u32)
}

impl FromIterator<Button> for ButtonSet {
    fn from_iter<I: IntoIterator<Item = Button>>(buttons: I) -> Self {
        let mut set = Self::EMPTY;
        for button in buttons {
            set.insert(button);
        }
        set
    }
}

/// How many buttons the browser Gamepad API's "standard" mapping has before
/// its optional extras: the first 16 of [`Button::ALL`], in the same order.
const STANDARD_BUTTONS: u32 = 16;

/// One pad's raw state in one frame, as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PadState {
    /// The real buttons held, by where they physically are on the pad (the
    /// stick directions come from the sticks below).
    pub buttons: ButtonSet,
    /// The left stick as `(x, y)`, each from -1 to 1: `+x` is right, `+y`
    /// is **down**.
    pub left_stick: (f32, f32),
    /// The right stick, like [`left_stick`](Self::left_stick).
    pub right_stick: (f32, f32),
}

impl PadState {
    /// A pad the browser reports in the Gamepad API's "standard" mapping:
    /// bit `i` of `buttons` is set while its `buttons[i]` is pressed (only
    /// the first 16 count), and `axes` are its first four axes (left stick
    /// x, y, right stick x, y; down is positive).
    pub fn from_standard(buttons: u32, axes: [f32; 4]) -> Self {
        let [left_x, left_y, right_x, right_y] = axes;
        Self {
            buttons: ButtonSet(buttons & ((1 << STANDARD_BUTTONS) - 1)),
            left_stick: (left_x, left_y),
            right_stick: (right_x, right_y),
        }
    }
}

/// A stick's direction buttons: up, down, left, right.
type StickButtons = [Button; 4];

const LEFT_STICK: StickButtons = [
    Button::LeftStickUp,
    Button::LeftStickDown,
    Button::LeftStickLeft,
    Button::LeftStickRight,
];
const RIGHT_STICK: StickButtons = [
    Button::RightStickUp,
    Button::RightStickDown,
    Button::RightStickLeft,
    Button::RightStickRight,
];

/// The one direction button a stick at `(x, y)` holds, if any. `held` is
/// the one it held last frame.
///
/// A direction starts when the stick is pushed at least `press_percent` its
/// way, and it is the direction pushed furthest (left or right on a perfect
/// diagonal). It then stays held until the stick falls below
/// `release_percent` that way, or another direction is pushed further by
/// the gap between the two thresholds, so neither the edge of the dead zone
/// nor a diagonal flickers.
fn pushed(
    held: Option<Button>,
    (x, y): (f32, f32),
    [up, down, left, right]: StickButtons,
    def: StickDef,
) -> Option<Button> {
    let press = f32::from(def.press_percent) / 100.0;
    let release = f32::from(def.release_percent) / 100.0;
    // How far the stick is pushed each way. On a tie the last wins.
    let amounts = [(up, -y), (down, y), (left, -x), (right, x)];
    let along = |button| {
        let found = amounts.iter().find(|&&(b, _)| b == button);
        found.map_or(0.0, |&(_, amount)| amount)
    };
    let (furthest, amount) = amounts.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let fresh = (amount >= press).then_some(furthest);
    let Some(held) = held.filter(|&h| along(h) >= release) else {
        return fresh;
    };
    match fresh {
        Some(new) if amount >= along(held) + (press - release) => Some(new),
        _ => Some(held),
    }
}

/// The buttons a pad of `kind` holds, by binding position, stick directions
/// included. `was` is what it held last frame (for the sticks).
fn resolve(was: ButtonSet, kind: PadKind, state: &PadState, stick: StickDef) -> ButtonSet {
    let mut down: ButtonSet = state.buttons.iter().map(|b| kind.position(b)).collect();
    for (buttons, at) in [
        (LEFT_STICK, state.left_stick),
        (RIGHT_STICK, state.right_stick),
    ] {
        let held = buttons.into_iter().find(|&b| was.contains(b));
        if let Some(button) = pushed(held, at, buttons, stick) {
            down.insert(button);
        }
    }
    down
}

/// What tells one connected pad from another: the platform's own number for
/// it, stable while the pad stays plugged in.
pub type PadId = usize;

/// Every connected pad, merged into one stream of button presses and
/// releases (see the [module docs](self)).
#[derive(Debug, Clone)]
pub struct Pads {
    stick: StickDef,
    /// Each connected pad's buttons held, by binding position.
    held: BTreeMap<PadId, ButtonSet>,
}

impl Pads {
    /// No pads yet, with the sticks' thresholds from the keymap.
    pub fn new(stick: StickDef) -> Self {
        Self {
            stick,
            held: BTreeMap::new(),
        }
    }

    /// Takes this frame's pads (`connected`: every pad plugged in, with its
    /// kind and state; a pad left out was removed) and returns what changed
    /// since the last call as `(button, pressed)`, releases first, each in
    /// [`Button::ALL`] order. Buttons are binding positions: feed them to
    /// [`InputState::pad_down`] and [`pad_up`].
    ///
    /// [`InputState::pad_down`]: super::InputState::pad_down
    /// [`pad_up`]: super::InputState::pad_up
    pub fn update(&mut self, connected: &[(PadId, PadKind, PadState)]) -> Vec<(Button, bool)> {
        let before = self.down();
        let mut held = BTreeMap::new();
        for (id, kind, state) in connected {
            let was = self.held.get(id).copied().unwrap_or_default();
            held.insert(*id, resolve(was, *kind, state, self.stick));
        }
        self.held = held;
        let after = self.down();
        let released = before.iter().filter(|&b| !after.contains(b));
        let pressed = after.iter().filter(|&b| !before.contains(b));
        released
            .map(|b| (b, false))
            .chain(pressed.map(|b| (b, true)))
            .collect()
    }

    /// The buttons held on any pad, by binding position.
    pub fn down(&self) -> ButtonSet {
        self.held
            .values()
            .fold(ButtonSet::EMPTY, |all, &pad| all.union(pad))
    }
}

#[cfg(test)]
mod tests;
