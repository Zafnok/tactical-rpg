//! The keymap (`assets/data/keymap.ron`, ADR-0015): for each [`Layout`],
//! which key chords trigger which [`Action`], the keys that work before a
//! layout is chosen, the controller [`Button`]s of each action (ADR-0034),
//! plus key-repeat timings and the sticks' thresholds.
//!
//! [`Key`], [`Chord`], [`Button`] and [`Action`] live here (not in `trpg-ui`) because the
//! loader must parse chords and action names to validate the file, and
//! `trpg-ui` depends on this crate, not the other way round. `trpg-ui::input`
//! re-exports them.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;

/// Path of the keymap inside the asset bundle.
pub const KEYMAP_PATH: &str = "data/keymap.ron";

/// Declares [`Key`] with its keymap-file names, keeping the variant list,
/// [`Key::ALL`] and [`Key::name`] in one table.
macro_rules! keys {
    ($($variant:ident => $name:literal,)+) => {
        /// A hardware-agnostic keyboard key. `app` translates platform key
        /// codes to these; anything without a variant is ignored.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Key {
            $(
                #[doc = concat!("The `", $name, "` key.")]
                $variant,
            )+
        }

        impl Key {
            /// Every key, in declaration order.
            pub const ALL: &'static [Key] = &[$(Key::$variant),+];

            /// The name used in keymap files: lowercase letters (`h`), digits
            /// (`0`), punctuation as itself (`;`, `,`, `\`), numpad keys
            /// prefixed `Kp` (`Kp0`, `Kp+`), and capitalised names for the
            /// rest (`Left`, `F12`).
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$variant => $name,)+
                }
            }
        }
    };
}

keys! {
    A => "a",
    B => "b",
    C => "c",
    D => "d",
    E => "e",
    F => "f",
    G => "g",
    H => "h",
    I => "i",
    J => "j",
    K => "k",
    L => "l",
    M => "m",
    N => "n",
    O => "o",
    P => "p",
    Q => "q",
    R => "r",
    S => "s",
    T => "t",
    U => "u",
    V => "v",
    W => "w",
    X => "x",
    Y => "y",
    Z => "z",
    Digit0 => "0",
    Digit1 => "1",
    Digit2 => "2",
    Digit3 => "3",
    Digit4 => "4",
    Digit5 => "5",
    Digit6 => "6",
    Digit7 => "7",
    Digit8 => "8",
    Digit9 => "9",
    Semicolon => ";",
    Up => "Up",
    Down => "Down",
    Left => "Left",
    Right => "Right",
    Enter => "Enter",
    Escape => "Escape",
    Space => "Space",
    Tab => "Tab",
    Backspace => "Backspace",
    F1 => "F1",
    F2 => "F2",
    F3 => "F3",
    F4 => "F4",
    F5 => "F5",
    F6 => "F6",
    F7 => "F7",
    F8 => "F8",
    F9 => "F9",
    F10 => "F10",
    F11 => "F11",
    F12 => "F12",
    Comma => ",",
    Period => ".",
    Slash => "/",
    Apostrophe => "'",
    LeftBracket => "[",
    RightBracket => "]",
    Backslash => "\\",
    Minus => "-",
    Equal => "=",
    Backquote => "`",
    Insert => "Insert",
    Delete => "Delete",
    Home => "Home",
    End => "End",
    PageUp => "PageUp",
    PageDown => "PageDown",
    Kp0 => "Kp0",
    Kp1 => "Kp1",
    Kp2 => "Kp2",
    Kp3 => "Kp3",
    Kp4 => "Kp4",
    Kp5 => "Kp5",
    Kp6 => "Kp6",
    Kp7 => "Kp7",
    Kp8 => "Kp8",
    Kp9 => "Kp9",
    KpAdd => "Kp+",
    KpSubtract => "Kp-",
    KpMultiply => "Kp*",
    KpDivide => "Kp/",
    KpDecimal => "Kp.",
}

impl Key {
    /// Looks a key up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.name() == name)
    }
}

/// Key slots per action: a layout lists at most this many chords for an
/// action, and the player can give each action up to this many keys
/// (`docs/design/controls.md`, *Rebinding keys*).
pub const SLOTS: usize = 3;

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A key plus modifier state, e.g. `h` or `Shift+h`. `Shift+h` and `h` are
/// different chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Chord {
    /// The key pressed.
    pub key: Key,
    /// Whether Shift was held.
    pub shift: bool,
}

/// Prefix for shifted chords in keymap files.
const SHIFT_PREFIX: &str = "Shift+";

impl Chord {
    /// A chord without modifiers.
    pub const fn plain(key: Key) -> Self {
        Self { key, shift: false }
    }

    /// A chord with Shift held.
    pub const fn shifted(key: Key) -> Self {
        Self { key, shift: true }
    }

    /// Whether this chord is fixed by the game and can never be bound
    /// (`docs/design/controls.md`, *Rebinding keys*): plain `Escape` always
    /// cancels and plain `Delete` empties a key slot. `Shift+Escape` and
    /// `Shift+Delete` are ordinary chords.
    pub fn is_reserved(self) -> bool {
        !self.shift && matches!(self.key, Key::Escape | Key::Delete)
    }

    /// Parses `"h"`, `"Shift+h"`, `"Left"`, `"Shift+Tab"`, `"F12"`, …
    pub fn parse(s: &str) -> Result<Self, String> {
        let (shift, name) = match s.strip_prefix(SHIFT_PREFIX) {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        match Key::from_name(name) {
            Some(key) => Ok(Self { key, shift }),
            None => Err(unknown_key_message(s, name)),
        }
    }
}

fn unknown_key_message(chord: &str, name: &str) -> String {
    let hint = if name.len() == 1 && name.bytes().all(|b| b.is_ascii_uppercase()) {
        format!(
            " (letters are lowercase; write \"{SHIFT_PREFIX}{}\" for a shifted letter)",
            name.to_ascii_lowercase()
        )
    } else {
        String::new()
    };
    format!(
        "unknown key chord \"{chord}\"{hint}; expected a key such as \"h\", \"0\", \";\", \"Left\", \
         \"Enter\", \"Escape\", \"Space\", \"Tab\", \"Backspace\" or \"F1\", optionally \
         prefixed by \"{SHIFT_PREFIX}\""
    )
}

impl FromStr for Chord {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.shift {
            f.write_str(SHIFT_PREFIX)?;
        }
        f.write_str(self.key.name())
    }
}

/// Declares [`Button`] with its docs, keeping the variant list,
/// [`Button::ALL`] and [`Button::name`] in one table.
macro_rules! buttons {
    ($($variant:ident => $doc:literal,)+) => {
        /// A controller button, named by its **position** on an Xbox-shaped
        /// pad, whatever is printed on it (ADR-0034). Each stick's four
        /// directions count as buttons too. `app` translates what the
        /// platform reports to these; anything else on a pad is ignored.
        ///
        /// The first 16 are in the order of the browser Gamepad API's
        /// "standard" mapping.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Button {
            $(
                #[doc = $doc]
                $variant,
            )+
        }

        impl Button {
            /// Every button, in declaration order.
            pub const ALL: &'static [Button] = &[$(Button::$variant),+];

            /// The name used in keymap files (the variant name).
            pub fn name(self) -> &'static str {
                match self {
                    $(Button::$variant => stringify!($variant),)+
                }
            }
        }
    };
}

buttons! {
    South => "The bottom face button (Xbox `A`).",
    East => "The right face button (Xbox `B`).",
    West => "The left face button (Xbox `X`).",
    North => "The top face button (Xbox `Y`).",
    LeftShoulder => "The left shoulder button (Xbox `LB`).",
    RightShoulder => "The right shoulder button (Xbox `RB`).",
    LeftTrigger => "The left trigger (Xbox `LT`), pulled far enough to count as pressed.",
    RightTrigger => "The right trigger (Xbox `RT`), pulled far enough to count as pressed.",
    Select => "The left centre button (Xbox `Back`).",
    Start => "The right centre button (Xbox `Start`).",
    LeftStickPress => "Pressing the left stick in.",
    RightStickPress => "Pressing the right stick in.",
    DpadUp => "D-pad up.",
    DpadDown => "D-pad down.",
    DpadLeft => "D-pad left.",
    DpadRight => "D-pad right.",
    LeftStickUp => "The left stick pushed up.",
    LeftStickDown => "The left stick pushed down.",
    LeftStickLeft => "The left stick pushed left.",
    LeftStickRight => "The left stick pushed right.",
    RightStickUp => "The right stick pushed up.",
    RightStickDown => "The right stick pushed down.",
    RightStickLeft => "The right stick pushed left.",
    RightStickRight => "The right stick pushed right.",
}

impl Button {
    /// Looks a button up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|b| b.name() == name)
    }

    /// Parses a keymap-file button name: `"South"`, `"DpadUp"`,
    /// `"LeftStickLeft"`, …
    pub fn parse(s: &str) -> Result<Self, String> {
        Self::from_name(s).ok_or_else(|| {
            let known: Vec<&str> = Self::ALL.iter().map(|b| b.name()).collect();
            format!(
                "unknown button \"{s}\"; known buttons: {}",
                known.join(", ")
            )
        })
    }
}

impl FromStr for Button {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Everything a player can ask for (ADR-0006). Screens only ever see these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Move the cursor one tile left.
    CursorLeft,
    /// Move the cursor one tile down.
    CursorDown,
    /// Move the cursor one tile up.
    CursorUp,
    /// Move the cursor one tile right.
    CursorRight,
    /// Confirm / select.
    Confirm,
    /// Cancel / back. Screens with nothing to cancel open the menu instead.
    Cancel,
    /// Unit info / details.
    Info,
    /// Toggle the enemy danger zone.
    DangerZone,
    /// Jump to the next ready unit.
    NextUnit,
    /// Jump to the previous ready unit.
    PrevUnit,
    /// End the player phase (asks for confirmation).
    EndTurn,
    /// Open the map menu.
    Menu,
    /// Debug overlay (developer).
    Debug,
    /// Toggle auto-end of the player phase (`docs/design/turn-structure.md`).
    ToggleAutoEnd,
    /// Open the turn rewind (`docs/design/death-and-difficulty.md`).
    Rewind,
    /// Pick a unit, tile or target on the map with the cursor. Optional:
    /// with no key, Confirm does it (`docs/design/controls.md`, *Optional
    /// split keys*).
    Select,
    /// Accept the end-turn prompt. Optional: with no key, End turn pressed
    /// again does it (`docs/design/controls.md`, *Optional split keys*).
    ConfirmEndTurn,
}

impl Action {
    /// Every action, in declaration order.
    pub const ALL: [Action; 17] = [
        Self::CursorLeft,
        Self::CursorDown,
        Self::CursorUp,
        Self::CursorRight,
        Self::Confirm,
        Self::Cancel,
        Self::Info,
        Self::DangerZone,
        Self::NextUnit,
        Self::PrevUnit,
        Self::EndTurn,
        Self::Menu,
        Self::Debug,
        Self::ToggleAutoEnd,
        Self::Rewind,
        Self::Select,
        Self::ConfirmEndTurn,
    ];

    /// The name used in keymap files (the variant name).
    pub fn name(self) -> &'static str {
        const NAMES: [&str; 17] = [
            "CursorLeft",
            "CursorDown",
            "CursorUp",
            "CursorRight",
            "Confirm",
            "Cancel",
            "Info",
            "DangerZone",
            "NextUnit",
            "PrevUnit",
            "EndTurn",
            "Menu",
            "Debug",
            "ToggleAutoEnd",
            "Rewind",
            "Select",
            "ConfirmEndTurn",
        ];
        NAMES[self as usize]
    }

    /// Looks an action up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Whether holding the key re-emits the action (the four cursor moves).
    pub fn is_repeatable(self) -> bool {
        matches!(
            self,
            Self::CursorLeft | Self::CursorDown | Self::CursorUp | Self::CursorRight
        )
    }

    /// Whether the player must always keep at least one key on this action:
    /// the *Required* column of `docs/design/controls.md`'s *Rebinding keys*
    /// table (the four cursor moves, Confirm, Cancel and End turn). The rest
    /// may be left with no key.
    pub fn is_required(self) -> bool {
        matches!(
            self,
            Self::CursorLeft
                | Self::CursorDown
                | Self::CursorUp
                | Self::CursorRight
                | Self::Confirm
                | Self::Cancel
                | Self::EndTurn
        )
    }

    /// Whether the player can rebind this action. Only the developer Debug
    /// key can't (`docs/design/controls.md`: it is not on the Key bindings
    /// screen).
    pub fn is_rebindable(self) -> bool {
        self != Self::Debug
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Key-repeat timings for held repeatable actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatDef {
    /// Time from the press to the first repeat, in milliseconds.
    pub delay_ms: u32,
    /// Time between later repeats, in milliseconds (never 0).
    pub interval_ms: u32,
}

impl Default for RepeatDef {
    fn default() -> Self {
        Self {
            delay_ms: 300,
            interval_ms: 55,
        }
    }
}

/// How far a stick must be pushed to count as a held direction
/// ([`Button::LeftStickUp`] and the rest), in percent of its full travel.
/// `release_percent` is below `press_percent`, so a stick resting near the
/// edge of the dead zone doesn't flicker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StickDef {
    /// A direction starts once the stick is pushed at least this far.
    pub press_percent: u8,
    /// A held direction ends once the stick falls below this.
    pub release_percent: u8,
}

impl Default for StickDef {
    fn default() -> Self {
        Self {
            press_percent: 50,
            release_percent: 35,
        }
    }
}

/// A complete set of key bindings the player can pick
/// (`docs/design/controls.md`). Each is one entry of `layouts` in the keymap
/// file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layout {
    /// Arrow keys steer; the left hand acts from the `A S D F` row.
    RightHanded,
    /// `W A S D` steer; the right hand acts from the `J K L ;` row.
    LeftHanded,
}

impl Layout {
    /// Every layout, in the order the layout picker lists them.
    pub const ALL: [Layout; 2] = [Self::RightHanded, Self::LeftHanded];

    /// The name used in keymap files and saved settings (the variant name).
    pub fn name(self) -> &'static str {
        match self {
            Self::RightHanded => "RightHanded",
            Self::LeftHanded => "LeftHanded",
        }
    }

    /// Looks a layout up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.name() == name)
    }
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Validated bindings as a lookup: each chord maps to exactly one action.
/// Unbound chords are absent.
pub type Bindings = BTreeMap<Chord, Action>;

/// One layout's default keys: every [`Action`] with its chords in the order
/// the keymap file lists them (at most [`SLOTS`], none [reserved]), no chord
/// on two actions. The order matters: the first chord is the one help text
/// names, and the chords fill the player's key slots in this order.
///
/// [reserved]: Chord::is_reserved
pub type LayoutKeys = BTreeMap<Action, Vec<Chord>>;

/// The default controller buttons: every [`Action`] with its buttons in the
/// order the keymap file lists them (at most [`SLOTS`]), no button on two
/// actions. One table for both layouts (`docs/design/controls.md`,
/// *Controller*).
pub type PadKeys = BTreeMap<Action, Vec<Button>>;

/// The validated keymap: every [`Layout`]'s default keys, the layout
/// picker's bindings, the default controller buttons, plus the key-repeat
/// timings and stick thresholds they share.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapDef {
    /// Layout → its default keys. After [`load`](Self::load) every layout
    /// is present, with every action.
    pub layouts: BTreeMap<Layout, LayoutKeys>,
    /// The keys that work before any layout is chosen (the layout picker).
    /// Actions not listed are unbound there.
    pub layout_picker: Bindings,
    /// The default controller buttons, the same in every layout and the
    /// layout picker. After [`load`](Self::load) every action is present.
    pub pad: PadKeys,
    /// Key-repeat timings, for held keys and held buttons alike.
    pub repeat: RepeatDef,
    /// How far a stick is pushed to count as a direction.
    pub stick: StickDef,
}

/// The file as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKeymap {
    layouts: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    layout_picker: BTreeMap<String, Vec<String>>,
    pad: BTreeMap<String, Vec<String>>,
    repeat: RepeatDef,
    stick: StickDef,
}

impl KeymapDef {
    /// Loads and validates the embedded keymap.
    pub fn load() -> Result<Self, Vec<ContentError>> {
        let display = bundle::display_path(KEYMAP_PATH);
        let source = bundle::file(KEYMAP_PATH).ok_or_else(|| {
            vec![ContentError::new(
                &display,
                "file not found in asset bundle",
            )]
        })?;
        Self::from_source(&display, source)
    }

    /// `layout`'s default bindings as a chord lookup (`None` only for a
    /// definition built by hand without it; a loaded keymap has every
    /// layout).
    pub fn bindings(&self, layout: Layout) -> Option<Bindings> {
        let keys = self.layouts.get(&layout)?;
        Some(
            keys.iter()
                .flat_map(|(&a, chords)| chords.iter().map(move |&c| (c, a)))
                .collect(),
        )
    }

    /// `action`'s default chords in `layout`, in file order; empty if the
    /// layout or action is missing (only in a definition built by hand).
    pub fn chords(&self, layout: Layout, action: Action) -> &[Chord] {
        self.layouts
            .get(&layout)
            .and_then(|keys| keys.get(&action))
            .map_or(&[], Vec::as_slice)
    }

    /// `action`'s default controller buttons, in file order; empty if the
    /// action is missing (only in a definition built by hand).
    pub fn buttons(&self, action: Action) -> &[Button] {
        self.pad.get(&action).map_or(&[], Vec::as_slice)
    }

    /// Parses and validates keymap `source`, attributing errors to `file`.
    /// Reports every unknown or missing layout, and within each layout every
    /// unknown action, unparsable chord, chord bound more than once,
    /// [reserved](Chord::is_reserved) chord, action with more than [`SLOTS`]
    /// chords and missing action (prefixed with the layout name), the same
    /// for the `layout_picker` section (except that unlisted actions are
    /// unbound there, and an action may have more than [`SLOTS`] chords:
    /// the picker isn't rebindable), and for the `pad` section (buttons
    /// instead of chords, every action listed, none on Debug), plus bad
    /// repeat timing and stick thresholds.
    pub fn from_source(file: &str, source: &str) -> Result<Self, Vec<ContentError>> {
        let raw: RawKeymap = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut errors = Vec::new();
        let mut layouts = BTreeMap::new();
        for (name, actions) in &raw.layouts {
            let start = line_of_quoted(source, 0, name);
            let Some(layout) = Layout::from_name(name) else {
                let known: Vec<&str> = Layout::ALL.iter().map(|l| l.name()).collect();
                let message = format!(
                    "unknown layout \"{name}\"; known layouts: {}",
                    known.join(", ")
                );
                errors.push(positioned(ContentError::new(file, message), start));
                continue;
            };
            // Search the layout's own lines first, so errors in the second
            // layout don't point into the first.
            let from = start.map_or(0, |line| usize::try_from(line).unwrap_or(0));
            let label = format!("layout \"{layout}\"");
            match validate_bindings::<Chord>(file, source, from, &label, true, actions) {
                Ok(keys) => {
                    layouts.insert(layout, keys);
                }
                Err(e) => errors.extend(e),
            }
        }
        for layout in Layout::ALL {
            if !raw.layouts.contains_key(layout.name()) {
                errors.push(ContentError::new(
                    file,
                    format!("missing layout \"{layout}\""),
                ));
            }
        }
        let picker_line = line_of_quoted(source, 0, "layout_picker");
        let from = picker_line.map_or(0, |line| usize::try_from(line).unwrap_or(0));
        let layout_picker = match validate_bindings::<Chord>(
            file,
            source,
            from,
            "layout_picker",
            false,
            &raw.layout_picker,
        ) {
            Ok(keys) => keys
                .iter()
                .flat_map(|(&a, chords)| chords.iter().map(move |&c| (c, a)))
                .collect(),
            Err(e) => {
                errors.extend(e);
                Bindings::new()
            }
        };
        let pad_line = line_of_quoted(source, 0, "pad");
        let from = pad_line.map_or(0, |line| usize::try_from(line).unwrap_or(0));
        let pad = match validate_bindings::<Button>(file, source, from, "pad", true, &raw.pad) {
            Ok(buttons) => buttons,
            Err(e) => {
                errors.extend(e);
                PadKeys::new()
            }
        };
        if !pad.get(&Action::Debug).is_none_or(Vec::is_empty) {
            let message = format!(
                "pad: action \"{}\" can't have a button: it is keyboard only, see controls.md",
                Action::Debug
            );
            errors.push(positioned(
                ContentError::new(file, message),
                line_of_quoted(source, from, Action::Debug.name()),
            ));
        }
        if raw.repeat.interval_ms == 0 {
            errors.push(positioned(
                ContentError::new(file, "repeat interval_ms must be at least 1"),
                line_of_quoted(source, 0, "repeat"),
            ));
        }
        let StickDef {
            press_percent,
            release_percent,
        } = raw.stick;
        if release_percent == 0 || release_percent >= press_percent || press_percent > 100 {
            errors.push(positioned(
                ContentError::new(
                    file,
                    "stick release_percent must be at least 1 and below press_percent, which \
                     must be at most 100",
                ),
                line_of_quoted(source, 0, "stick"),
            ));
        }
        if errors.is_empty() {
            Ok(Self {
                layouts,
                layout_picker,
                pad,
                repeat: raw.repeat,
                stick: raw.stick,
            })
        } else {
            Err(errors)
        }
    }
}

/// Something `keymap.ron` binds to actions: a key [`Chord`] or a controller
/// [`Button`].
trait Bound: Copy + Ord + fmt::Display {
    /// What one is called in error messages.
    const NOUN: &'static str;
    /// What several are called in error messages.
    const PLURAL: &'static str;

    /// Parses its keymap-file name.
    fn parse_name(text: &str) -> Result<Self, String>;

    /// Why it may never appear in the file, if so.
    fn refused(self) -> Option<&'static str>;
}

impl Bound for Chord {
    const NOUN: &'static str = "chord";
    const PLURAL: &'static str = "keys";

    fn parse_name(text: &str) -> Result<Self, String> {
        Self::parse(text)
    }

    fn refused(self) -> Option<&'static str> {
        self.is_reserved()
            .then_some("Esc and Delete are fixed, see controls.md")
    }
}

impl Bound for Button {
    const NOUN: &'static str = "button";
    const PLURAL: &'static str = "buttons";

    fn parse_name(text: &str) -> Result<Self, String> {
        Self::parse(text)
    }

    fn refused(self) -> Option<&'static str> {
        None
    }
}

/// Validates one section's `actions` (action name → chord or button
/// names): a layout, the layout picker or the `pad` table. Messages start
/// with `label`; lines are searched from line index `from` (just after the
/// section's own line) on. For a `complete` section (a layout, `pad`), every
/// [`Action`] must be listed, with at most [`SLOTS`] entries.
fn validate_bindings<T: Bound>(
    file: &str,
    source: &str,
    from: usize,
    label: &str,
    complete: bool,
    actions: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<Action, Vec<T>>, Vec<ContentError>> {
    let noun = T::NOUN;
    let mut errors = Vec::new();
    let mut bindings = BTreeMap::new();
    let mut keys: BTreeMap<Action, Vec<T>> = BTreeMap::new();
    let err_at = |key: &str, message: String| {
        positioned(
            ContentError::new(file, format!("{label}: {message}")),
            line_of_quoted(source, from, key),
        )
    };
    for (action_name, names) in actions {
        let Some(action) = Action::from_name(action_name) else {
            let known: Vec<&str> = Action::ALL.iter().map(|a| a.name()).collect();
            errors.push(err_at(
                action_name,
                format!(
                    "unknown action \"{action_name}\"; known actions: {}",
                    known.join(", ")
                ),
            ));
            continue;
        };
        if complete && names.len() > SLOTS {
            errors.push(err_at(
                action_name,
                format!(
                    "action \"{action}\" has {} {}; at most {SLOTS} are allowed",
                    names.len(),
                    T::PLURAL
                ),
            ));
        }
        let listed = keys.entry(action).or_default();
        for text in names {
            let bound = match T::parse_name(text) {
                Ok(bound) => bound,
                Err(why) => {
                    errors.push(err_at(
                        action_name,
                        format!("action \"{action_name}\": {why}"),
                    ));
                    continue;
                }
            };
            if let Some(why) = bound.refused() {
                errors.push(err_at(
                    action_name,
                    format!("action \"{action}\": \"{bound}\" can't be bound here: {why}"),
                ));
            } else if let Some(&other) = bindings.get(&bound) {
                let message = if other == action {
                    format!("{noun} \"{bound}\" is listed twice for \"{action}\"")
                } else {
                    format!("{noun} \"{bound}\" is bound to both \"{other}\" and \"{action}\"")
                };
                errors.push(err_at(action_name, message));
            } else {
                bindings.insert(bound, action);
                listed.push(bound);
            }
        }
    }
    for action in Action::ALL {
        if complete && !actions.contains_key(action.name()) {
            errors.push(ContentError::new(
                file,
                format!(
                    "{label}: missing action \"{action}\" (list it with [] to leave it \
                     unbound)"
                ),
            ));
        }
    }
    if errors.is_empty() {
        Ok(keys)
    } else {
        Err(errors)
    }
}

/// `error` at `line`, if known.
fn positioned(error: ContentError, line: Option<u32>) -> ContentError {
    match line {
        Some(line) => error.at(line, None),
        None => error,
    }
}

/// 1-based number of the first line, at or after line index `from`
/// (0-based), that contains `key` quoted or starts with the field `key:`,
/// for error positions.
fn line_of_quoted(source: &str, from: usize, key: &str) -> Option<u32> {
    let quoted = format!("\"{key}\"");
    let field = format!("{key}:");
    let (index, _) = source
        .lines()
        .enumerate()
        .skip(from)
        .find(|(_, l)| l.contains(&quoted) || l.trim_start().starts_with(&field))?;
    u32::try_from(index + 1).ok()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn chord_parse_plain_and_shifted() {
        assert_eq!(Chord::parse("h"), Ok(Chord::plain(Key::H)));
        assert_eq!(Chord::parse("Shift+h"), Ok(Chord::shifted(Key::H)));
        assert_eq!(Chord::parse("Shift+Tab"), Ok(Chord::shifted(Key::Tab)));
        assert_eq!(Chord::parse("F12"), Ok(Chord::plain(Key::F12)));
        assert_eq!(Chord::parse("7"), Ok(Chord::plain(Key::Digit7)));
        assert_eq!(Chord::parse(";"), Ok(Chord::plain(Key::Semicolon)));
        assert_eq!(Chord::parse("Shift+;"), Ok(Chord::shifted(Key::Semicolon)));
        assert_eq!("Left".parse::<Chord>(), Ok(Chord::plain(Key::Left)));
    }

    #[test]
    fn chord_parse_rejects_garbage() {
        for bad in [
            "",
            "Shift+",
            "shift+h",
            "Ctrl+h",
            "hh",
            "F13",
            "left",
            "Shift+Shift+h",
        ] {
            assert!(Chord::parse(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn chord_parse_hints_at_uppercase_letters() {
        let err = Chord::parse("H").err().unwrap_or_default();
        assert!(err.contains("\"Shift+h\""), "{err}");
        for no_hint in ["Hh", "HH", "?"] {
            let err = Chord::parse(no_hint).err().unwrap_or_default();
            assert!(!err.contains("shifted letter"), "{err}");
        }
    }

    #[test]
    fn chord_display() {
        assert_eq!(Chord::plain(Key::Digit0).to_string(), "0");
        assert_eq!(Chord::shifted(Key::L).to_string(), "Shift+l");
        assert_eq!(Chord::plain(Key::Backspace).to_string(), "Backspace");
        assert_eq!(Key::Escape.to_string(), "Escape");
        assert_eq!(Chord::plain(Key::Semicolon).to_string(), ";");
        assert_eq!(Key::from_name(";"), Some(Key::Semicolon));
    }

    #[test]
    fn key_and_action_tables_line_up() {
        assert_eq!(Key::ALL.len(), 26 + 10 + 1 + 4 + 5 + 12 + 10 + 6 + 10 + 5);
        for (i, k) in Key::ALL.iter().enumerate() {
            assert_eq!(*k as usize, i);
            assert_eq!(Key::from_name(k.name()), Some(*k));
        }
        for (i, a) in Action::ALL.iter().enumerate() {
            assert_eq!(*a as usize, i);
            assert_eq!(Action::from_name(&a.to_string()), Some(*a));
        }
        assert_eq!(Action::from_name("cursorleft"), None);
    }

    #[test]
    fn only_cursor_actions_repeat() {
        let repeatable: Vec<Action> = Action::ALL
            .into_iter()
            .filter(|a| a.is_repeatable())
            .collect();
        assert_eq!(repeatable, Action::ALL[..4].to_vec());
        assert!(
            Action::ALL[..4]
                .iter()
                .all(|a| a.name().starts_with("Cursor"))
        );
    }

    fn arb_chord() -> impl Strategy<Value = Chord> {
        (prop::sample::select(Key::ALL), any::<bool>())
            .prop_map(|(key, shift)| Chord { key, shift })
    }

    proptest! {
        #[test]
        fn chord_round_trips(c in arb_chord()) {
            prop_assert_eq!(Chord::parse(&c.to_string()), Ok(c));
        }
    }

    /// One layout block with every action but `skip` listed as `[]`, then
    /// `extra`.
    fn block(name: &str, skip: &str, extra: &str) -> String {
        let lines: Vec<String> = Action::ALL
            .iter()
            .filter(|a| a.name() != skip)
            .map(|a| format!("            \"{a}\": [],\n"))
            .collect();
        format!(
            "        \"{name}\": {{\n{}{extra}        }},\n",
            lines.concat()
        )
    }

    /// A keymap source holding `blocks` as its layouts and an empty
    /// `layout_picker`.
    fn source_of(blocks: &[String]) -> String {
        source_with_picker(blocks, "")
    }

    /// A keymap source holding `blocks` as its layouts, then `repeat`, then
    /// `picker` as the `layout_picker` section's entries (on the line after
    /// `layout_picker: {`), then a `pad` section with every action `[]` and
    /// the default `stick`.
    fn source_with_picker(blocks: &[String], picker: &str) -> String {
        source_with_pad(blocks, picker, "", "")
    }

    /// [`source_with_picker`], except that the `pad` section leaves out
    /// `skip` and has `extra` spliced in after the other actions.
    fn source_with_pad(blocks: &[String], picker: &str, skip: &str, extra: &str) -> String {
        let pad: Vec<String> = Action::ALL
            .iter()
            .filter(|a| a.name() != skip)
            .map(|a| format!("        \"{a}\": [],\n"))
            .collect();
        format!(
            "(\n    layouts: {{\n{}    }},\n    repeat: (delay_ms: 300, interval_ms: 55),\n    \
             layout_picker: {{\n{picker}    }},\n    pad: {{\n{}{extra}    }},\n    stick: \
             (press_percent: 50, release_percent: 35),\n)",
            blocks.concat(),
            pad.concat()
        )
    }

    /// A keymap source with empty layouts and picker, whose `pad` section
    /// leaves out `skip` and has `extra` spliced in.
    fn pad_source(skip: &str, extra: &str) -> String {
        source_with_pad(&empty_layouts(), "", skip, extra)
    }

    /// Line of the `pad` section's entry number `n` (0-based) in
    /// [`pad_source`].
    fn pad_line(n: usize) -> Option<u32> {
        u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 4 + 1 + n).ok()
    }

    /// A complete valid keymap source, except that the `LeftHanded` layout
    /// (listed first, starting on line 3) leaves out `skip` and has `extra`
    /// spliced in. `RightHanded` follows with every action `[]`.
    fn source_with(skip: &str, extra: &str) -> String {
        source_of(&[
            block("LeftHanded", skip, extra),
            block("RightHanded", "", ""),
        ])
    }

    fn errors(src: &str) -> Vec<String> {
        KeymapDef::from_source("k.ron", src)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn left(k: &KeymapDef) -> Bindings {
        k.bindings(Layout::LeftHanded).unwrap_or_default()
    }

    #[test]
    fn button_names_round_trip() {
        assert_eq!(Button::ALL.len(), 16 + 4 + 4);
        for (i, &b) in Button::ALL.iter().enumerate() {
            assert_eq!(b as usize, i);
            assert_eq!(Button::parse(&b.to_string()), Ok(b));
            assert_eq!(b.name().parse::<Button>(), Ok(b));
            assert_eq!(Button::from_name(b.name()), Some(b));
        }
        assert_eq!(Button::South.name(), "South");
        assert_eq!(Button::RightStickRight.to_string(), "RightStickRight");
        // The browser Gamepad API's standard mapping order.
        let standard = [
            "South",
            "East",
            "West",
            "North",
            "LeftShoulder",
            "RightShoulder",
            "LeftTrigger",
            "RightTrigger",
            "Select",
            "Start",
            "LeftStickPress",
            "RightStickPress",
            "DpadUp",
            "DpadDown",
            "DpadLeft",
            "DpadRight",
        ];
        let names: Vec<&str> = Button::ALL[..16].iter().map(|b| b.name()).collect();
        assert_eq!(names, standard);
    }

    #[test]
    fn button_parse_rejects_garbage() {
        for bad in ["", "south", "A", "Shift+South", "South ", "f"] {
            assert_eq!(Button::from_name(bad), None, "{bad:?}");
            let err = Button::parse(bad).err().unwrap_or_default();
            assert!(
                err.starts_with(&format!(
                    "unknown button \"{bad}\"; known buttons: South, East,"
                )),
                "{err}"
            );
        }
    }

    #[test]
    fn pad_section_loads_in_file_order() {
        let extra = "        \"CursorUp\": [\"LeftStickUp\", \"DpadUp\", \"RightStickUp\"],\n";
        let k = KeymapDef::from_source("k.ron", &pad_source("CursorUp", extra)).unwrap_or_default();
        assert_eq!(
            k.buttons(Action::CursorUp),
            [Button::LeftStickUp, Button::DpadUp, Button::RightStickUp]
        );
        assert_eq!(k.buttons(Action::Confirm), []);
        assert_eq!(k.pad.len(), Action::ALL.len());
        assert_eq!(k.stick, StickDef::default());
        assert_eq!(KeymapDef::default().buttons(Action::CursorUp), []);
    }

    #[test]
    fn pad_button_on_two_actions_is_error_with_line() {
        let src = pad_source("Info", "        \"Info\": [\"South\"],\n");
        // Confirm in the pad section: the last `"Confirm": []` of the file.
        let empty = "\"Confirm\": []";
        let at = src.rfind(empty).unwrap_or(0);
        let rest = &src[at + empty.len()..];
        let src = format!("{}\"Confirm\": [\"South\"]{rest}", &src[..at]);
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(
            errs[0].message,
            "pad: button \"South\" is bound to both \"Confirm\" and \"Info\""
        );
        assert_eq!(errs[0].line, pad_line(Action::ALL.len() - 1));
    }

    #[test]
    fn pad_button_listed_twice_is_error() {
        let errs = errors(&pad_source(
            "Info",
            "        \"Info\": [\"West\", \"West\"],\n",
        ));
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].ends_with("pad: button \"West\" is listed twice for \"Info\""),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn pad_button_on_debug_is_error_with_line() {
        let src = pad_source("Debug", "        \"Debug\": [\"RightTrigger\"],\n");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(
            errs[0].message,
            "pad: action \"Debug\" can't have a button: it is keyboard only, see controls.md"
        );
        assert_eq!(errs[0].line, pad_line(Action::ALL.len() - 1));
        // Listed empty, it is fine.
        assert!(errors(&pad_source("", "")).is_empty());
    }

    #[test]
    fn pad_unknown_button_and_action_are_errors() {
        let errs = errors(&pad_source(
            "Info",
            "        \"Info\": [\"f\"],\n        \"Attack\": [],\n",
        ));
        assert_eq!(errs.len(), 2, "{errs:?}");
        assert!(
            errs[0].contains("pad: unknown action \"Attack\""),
            "{}",
            errs[0]
        );
        assert!(
            errs[1].contains("pad: action \"Info\": unknown button \"f\"; known buttons: South"),
            "{}",
            errs[1]
        );
    }

    #[test]
    fn pad_missing_action_is_error() {
        let errs = errors(&pad_source("Rewind", ""));
        assert_eq!(
            errs,
            vec!["k.ron: pad: missing action \"Rewind\" (list it with [] to leave it unbound)"]
        );
    }

    #[test]
    fn pad_more_than_three_buttons_is_error() {
        let four = "        \"Info\": [\"South\", \"East\", \"West\", \"North\"],\n";
        let errs = KeymapDef::from_source("k.ron", &pad_source("Info", four))
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(
            errs[0].message,
            "pad: action \"Info\" has 4 buttons; at most 3 are allowed"
        );
        assert_eq!(errs[0].line, pad_line(Action::ALL.len() - 1));
        let three = "        \"Info\": [\"South\", \"East\", \"West\"],\n";
        let k = KeymapDef::from_source("k.ron", &pad_source("Info", three)).unwrap_or_default();
        assert_eq!(k.buttons(Action::Info).len(), 3);
    }

    #[test]
    fn missing_pad_or_stick_section_is_error() {
        let src = pad_source("", "");
        let stick = "    stick: (press_percent: 50, release_percent: 35),\n";
        assert!(src.contains(stick));
        assert_eq!(errors(&src.replace(stick, "")).len(), 1);
        let at = src.find("    pad: {").unwrap_or(0);
        let end = src.find("    stick:").unwrap_or(0);
        assert_eq!(errors(&format!("{}{}", &src[..at], &src[end..])).len(), 1);
    }

    #[test]
    fn stick_thresholds_must_be_in_order_and_in_range() {
        let stick_line =
            u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 6 + Action::ALL.len()).ok();
        for (press, release, ok) in [
            (50, 35, true),
            (100, 99, true),
            (2, 1, true),
            (101, 35, false),
            (50, 50, false),
            (50, 51, false),
            (50, 0, false),
        ] {
            let src = pad_source("", "").replace(
                "press_percent: 50, release_percent: 35",
                &format!("press_percent: {press}, release_percent: {release}"),
            );
            match KeymapDef::from_source("k.ron", &src) {
                Ok(k) => {
                    assert!(ok, "{press}/{release} should be refused");
                    assert_eq!(
                        (k.stick.press_percent, k.stick.release_percent),
                        (press, release)
                    );
                }
                Err(errs) => {
                    assert!(!ok, "{press}/{release}: {errs:?}");
                    assert_eq!(errs.len(), 1);
                    assert!(errs[0].message.starts_with("stick release_percent"));
                    assert_eq!(errs[0].line, stick_line);
                }
            }
        }
    }

    #[test]
    fn embedded_pad_buttons_match_the_design() {
        // docs/design/controls.md, Controller → Default buttons.
        use Button::{
            DpadDown, DpadLeft, DpadRight, DpadUp, East, LeftShoulder, LeftStickDown,
            LeftStickLeft, LeftStickRight, LeftStickUp, LeftTrigger, North, RightShoulder, Select,
            South, Start, West,
        };
        let k = KeymapDef::load().unwrap_or_default();
        let design: PadKeys = [
            // D-pad and left stick both move the cursor.
            (Action::CursorLeft, vec![DpadLeft, LeftStickLeft]),
            (Action::CursorDown, vec![DpadDown, LeftStickDown]),
            (Action::CursorUp, vec![DpadUp, LeftStickUp]),
            (Action::CursorRight, vec![DpadRight, LeftStickRight]),
            (Action::Confirm, vec![South]),
            (Action::Cancel, vec![East]),
            (Action::PrevUnit, vec![LeftShoulder]),
            (Action::NextUnit, vec![RightShoulder]),
            (Action::Info, vec![North]),
            (Action::DangerZone, vec![West]),
            (Action::EndTurn, vec![Start]),
            (Action::ToggleAutoEnd, vec![Select]),
            (Action::Rewind, vec![LeftTrigger]),
            // No button: the map menu, the optional split keys, Debug.
            (Action::Menu, vec![]),
            (Action::Select, vec![]),
            (Action::ConfirmEndTurn, vec![]),
            (Action::Debug, vec![]),
        ]
        .into();
        assert_eq!(design.len(), Action::ALL.len());
        assert_eq!(k.pad, design);
        // Stick thresholds: tunable, press above release.
        assert_eq!(
            k.stick,
            StickDef {
                press_percent: 50,
                release_percent: 35
            }
        );
    }

    #[test]
    fn valid_keymap_loads() {
        let src = source_with("Confirm", "        \"Confirm\": [\"f\", \"Space\"],\n");
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        let b = left(&k);
        assert_eq!(b.get(&Chord::plain(Key::F)), Some(&Action::Confirm));
        assert_eq!(b.get(&Chord::plain(Key::Space)), Some(&Action::Confirm));
        assert_eq!(b.len(), 2);
        assert_eq!(k.bindings(Layout::RightHanded).map(|b| b.len()), Some(0));
        assert_eq!(
            k.chords(Layout::LeftHanded, Action::Confirm),
            [Chord::plain(Key::F), Chord::plain(Key::Space)]
        );
        assert_eq!(k.chords(Layout::RightHanded, Action::Confirm), []);
        assert_eq!(k.layouts.len(), 2);
        assert_eq!(k.repeat, RepeatDef::default());
    }

    #[test]
    fn layouts_are_independent() {
        // The same chord may mean different actions in different layouts.
        let src = source_of(&[
            block("LeftHanded", "Confirm", "\"Confirm\": [\"f\"],\n"),
            block("RightHanded", "Cancel", "\"Cancel\": [\"f\"],\n"),
        ]);
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        let get = |l| {
            k.bindings(l)
                .and_then(|b| b.get(&Chord::plain(Key::F)).copied())
        };
        assert_eq!(get(Layout::LeftHanded), Some(Action::Confirm));
        assert_eq!(get(Layout::RightHanded), Some(Action::Cancel));
    }

    #[test]
    fn unknown_action_is_error_with_line() {
        let src = source_with("", "        \"Attack\": [\"f\"],\n");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0]
                .message
                .starts_with("layout \"LeftHanded\": unknown action \"Attack\""),
            "{}",
            errs[0].message
        );
        assert!(errs[0].message.contains("Confirm"));
        let line = u32::try_from(Action::ALL.len() + 4).unwrap_or(0);
        assert_eq!(errs[0].line, Some(line));
    }

    #[test]
    fn bad_chord_is_error() {
        let src = source_with("Info", "        \"Info\": [\"s\", \"Ctrl+i\"],\n");
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("layout \"LeftHanded\""), "{}", errs[0]);
        assert!(errs[0].contains("\"Info\""), "{}", errs[0]);
        assert!(errs[0].contains("\"Ctrl+i\""), "{}", errs[0]);
    }

    #[test]
    fn conflicting_chord_is_error() {
        let src = source_with("Info", "        \"Info\": [\"f\"],\n").replacen(
            "\"Confirm\": []",
            "\"Confirm\": [\"f\"]",
            1,
        );
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0].contains(
                "layout \"LeftHanded\": chord \"f\" is bound to both \"Confirm\" and \"Info\""
            ),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn conflict_in_the_second_layout_points_at_its_own_line() {
        let src = source_of(&[
            block("LeftHanded", "", ""),
            block("RightHanded", "Info", "\"Info\": [\"d\", \"d\"],\n"),
        ]);
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0]
                .message
                .starts_with("layout \"RightHanded\": chord \"d\" is listed twice"),
            "{}",
            errs[0].message
        );
        let n = Action::ALL.len();
        // Line 3 opens LeftHanded; its n actions and closing brace follow,
        // then RightHanded's line and its n - 1 other actions.
        let line = u32::try_from(3 + n + 1 + 1 + (n - 1) + 1).unwrap_or(0);
        assert_eq!(errs[0].line, Some(line));
    }

    #[test]
    fn duplicate_chord_in_one_action_is_error() {
        let src = source_with("Info", "        \"Info\": [\"s\", \"s\"],\n");
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("listed twice"), "{}", errs[0]);
    }

    #[test]
    fn shift_makes_a_distinct_chord() {
        let src = source_with("", "")
            .replacen("\"CursorLeft\": []", "\"CursorLeft\": [\"h\"]", 1)
            .replacen("\"Info\": []", "\"Info\": [\"Shift+h\"]", 1);
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        assert_eq!(left(&k).len(), 2);
    }

    #[test]
    fn missing_action_is_error() {
        let errs = errors(&source_with("Debug", ""));
        assert_eq!(
            errs,
            vec![
                "k.ron: layout \"LeftHanded\": missing action \"Debug\" (list it with [] to \
                 leave it unbound)"
            ]
        );
    }

    #[test]
    fn missing_layout_is_error() {
        let errs = errors(&source_of(&[block("RightHanded", "", "")]));
        assert_eq!(errs, vec!["k.ron: missing layout \"LeftHanded\""]);
        let errs = errors(&source_of(&[]));
        assert_eq!(
            errs,
            vec![
                "k.ron: missing layout \"RightHanded\"",
                "k.ron: missing layout \"LeftHanded\""
            ]
        );
    }

    #[test]
    fn unknown_layout_is_error_with_line() {
        let src = source_of(&[
            block("LeftHanded", "", ""),
            block("RightHanded", "", ""),
            block("Vim", "Bogus", ""),
        ]);
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        // Only the unknown layout itself: its contents aren't checked.
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].message,
            "unknown layout \"Vim\"; known layouts: RightHanded, LeftHanded"
        );
        let line = u32::try_from(3 + 2 * (Action::ALL.len() + 2)).unwrap_or(0);
        assert_eq!(errs[0].line, Some(line));
    }

    #[test]
    fn zero_interval_is_error() {
        let src = source_with("", "").replace("interval_ms: 55", "interval_ms: 0");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("interval_ms"), "{}", errs[0]);
        let line = u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 1).unwrap_or(0);
        assert_eq!(errs[0].line, Some(line));
    }

    #[test]
    fn all_errors_reported_together() {
        let src = source_of(&[
            block(
                "LeftHanded",
                "Confirm",
                "        \"Confirm\": [\"Nope\", \"f\"],\n        \"Bogus\": [],\n",
            )
            .replacen("\"Cancel\": []", "\"Cancel\": [\"f\"]", 1),
            block("RightHanded", "Debug", ""),
            block("Vim", "", ""),
        ])
        .replace("interval_ms: 55", "interval_ms: 0");
        assert_eq!(errors(&src).len(), 6);
    }

    #[test]
    fn old_single_layout_format_is_rejected() {
        let src = "(\n    bindings: {},\n    repeat: (delay_ms: 1, interval_ms: 1),\n)";
        assert_eq!(errors(src).len(), 1);
    }

    /// Two complete layouts, every action `[]`.
    fn empty_layouts() -> [String; 2] {
        [block("LeftHanded", "", ""), block("RightHanded", "", "")]
    }

    /// Line of the `layout_picker` section's first entry in
    /// [`source_with_picker`] with [`empty_layouts`].
    fn first_picker_line() -> Option<u32> {
        u32::try_from(3 + 2 * (Action::ALL.len() + 2) + 3).ok()
    }

    #[test]
    fn layout_picker_section_loads_and_leaves_unlisted_actions_unbound() {
        let picker = "        \"CursorUp\": [\"Up\", \"w\"],\n        \"Confirm\": [\"Enter\"],\n";
        let src = source_with_picker(&empty_layouts(), picker);
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        let expected: Bindings = [
            (Chord::plain(Key::Up), Action::CursorUp),
            (Chord::plain(Key::W), Action::CursorUp),
            (Chord::plain(Key::Enter), Action::Confirm),
        ]
        .into();
        assert_eq!(k.layout_picker, expected);
    }

    #[test]
    fn layout_picker_unknown_action_is_error_with_line() {
        let src = source_with_picker(&empty_layouts(), "        \"Attack\": [\"f\"],\n");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0]
                .message
                .starts_with("layout_picker: unknown action \"Attack\""),
            "{}",
            errs[0].message
        );
        assert_eq!(errs[0].line, first_picker_line());
    }

    #[test]
    fn layout_picker_bad_chord_is_error_with_line() {
        let src = source_with_picker(&empty_layouts(), "        \"Confirm\": [\"Nope\"],\n");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0]
                .message
                .starts_with("layout_picker: action \"Confirm\": "),
            "{}",
            errs[0].message
        );
        assert_eq!(errs[0].line, first_picker_line());
    }

    #[test]
    fn layout_picker_conflicting_chord_is_error() {
        let picker = "        \"Confirm\": [\"f\"],\n        \"Cancel\": [\"f\"],\n";
        let errs = errors(&source_with_picker(&empty_layouts(), picker));
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("layout_picker: chord \"f\" is bound to both"),
            "{}",
            errs[0]
        );
    }

    #[test]
    fn missing_layout_picker_section_is_error() {
        let src = source_of(&empty_layouts()).replace("    layout_picker: {\n    },\n", "");
        assert!(!src.contains("layout_picker"));
        assert_eq!(errors(&src).len(), 1);
    }

    #[test]
    fn syntax_error_is_positioned() {
        let errs = KeymapDef::from_source("k.ron", "(\n  layouts: {\n  \"a\" {} },\n)")
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(3));
    }

    #[test]
    fn line_of_quoted_finds_keys_and_fields() {
        let src = "(\n \"Info\": [],\n    interval_ms: 3,\n \"Info\": [],\n)";
        assert_eq!(line_of_quoted(src, 0, "Info"), Some(2));
        assert_eq!(line_of_quoted(src, 1, "Info"), Some(2));
        assert_eq!(line_of_quoted(src, 2, "Info"), Some(4));
        assert_eq!(line_of_quoted(src, 0, "interval_ms"), Some(3));
        assert_eq!(line_of_quoted(src, 0, "nope"), None);
        assert_eq!(line_of_quoted(src, 9, "Info"), None);
    }

    #[test]
    fn layout_names_round_trip() {
        for l in Layout::ALL {
            assert_eq!(Layout::from_name(l.name()), Some(l));
            assert_eq!(Layout::from_name(&l.to_string()), Some(l));
        }
        assert_eq!(Layout::RightHanded.name(), "RightHanded");
        assert_eq!(Layout::LeftHanded.name(), "LeftHanded");
        assert_eq!(Layout::from_name("lefthanded"), None);
    }

    /// Looks `chord` up in the embedded keymap's `layout`.
    fn embedded(layout: Layout) -> impl Fn(&str) -> Option<Action> {
        let bindings = KeymapDef::load()
            .ok()
            .and_then(|k| k.bindings(layout))
            .unwrap_or_default();
        move |s| Chord::parse(s).ok().and_then(|c| bindings.get(&c).copied())
    }

    #[test]
    fn embedded_right_handed_layout_matches_the_design() {
        // docs/design/controls.md, right-handed column.
        let get = embedded(Layout::RightHanded);
        assert_eq!(get("Left"), Some(Action::CursorLeft));
        assert_eq!(get("Down"), Some(Action::CursorDown));
        assert_eq!(get("Up"), Some(Action::CursorUp));
        assert_eq!(get("Right"), Some(Action::CursorRight));
        assert_eq!(get("f"), Some(Action::Confirm));
        assert_eq!(get("d"), Some(Action::Cancel));
        // Escape is a fixed key, not in the file (see trpg-ui's Keymap).
        assert_eq!(get("Escape"), None);
        assert_eq!(get("a"), Some(Action::PrevUnit));
        assert_eq!(get("s"), Some(Action::NextUnit));
        assert_eq!(get("e"), Some(Action::Info));
        assert_eq!(get("w"), Some(Action::DangerZone));
        assert_eq!(get("Space"), Some(Action::EndTurn));
        assert_eq!(get("Shift+Space"), Some(Action::ToggleAutoEnd));
        assert_eq!(get("F2"), Some(Action::Debug));
        assert_eq!(get("r"), Some(Action::Rewind));
    }

    #[test]
    fn embedded_left_handed_layout_matches_the_design() {
        // docs/design/controls.md, left-handed column.
        let get = embedded(Layout::LeftHanded);
        assert_eq!(get("a"), Some(Action::CursorLeft));
        assert_eq!(get("s"), Some(Action::CursorDown));
        assert_eq!(get("w"), Some(Action::CursorUp));
        assert_eq!(get("d"), Some(Action::CursorRight));
        assert_eq!(get("j"), Some(Action::Confirm));
        assert_eq!(get("k"), Some(Action::Cancel));
        assert_eq!(get("Escape"), None);
        assert_eq!(get(";"), Some(Action::PrevUnit));
        assert_eq!(get("l"), Some(Action::NextUnit));
        assert_eq!(get("i"), Some(Action::Info));
        assert_eq!(get("o"), Some(Action::DangerZone));
        assert_eq!(get("Space"), Some(Action::EndTurn));
        assert_eq!(get("Shift+Space"), Some(Action::ToggleAutoEnd));
        assert_eq!(get("F2"), Some(Action::Debug));
        assert_eq!(get("u"), Some(Action::Rewind));
    }

    #[test]
    fn embedded_keymap_has_exactly_the_design_bindings() {
        // 14 chords per layout: nothing bound beyond the design table.
        let k = KeymapDef::load().unwrap_or_default();
        for layout in Layout::ALL {
            assert_eq!(k.bindings(layout).map(|b| b.len()), Some(14), "{layout}");
            assert_eq!(k.layouts.get(&layout).map(BTreeMap::len), Some(17));
            // The optional split keys start with no key (controls.md).
            for split in [Action::Select, Action::ConfirmEndTurn] {
                assert_eq!(k.chords(layout, split), [], "{layout} {split}");
                assert!(!split.is_required());
            }
        }
        assert_eq!(k.layouts.len(), 2);
        // Before a layout is chosen: either hand's up/down and confirm keys.
        let picker: Bindings = [
            ("Up", Action::CursorUp),
            ("w", Action::CursorUp),
            ("Down", Action::CursorDown),
            ("s", Action::CursorDown),
            ("f", Action::Confirm),
            ("j", Action::Confirm),
            ("Enter", Action::Confirm),
            ("Space", Action::Confirm),
        ]
        .into_iter()
        .filter_map(|(c, a)| Chord::parse(c).ok().map(|c| (c, a)))
        .collect();
        assert_eq!(picker.len(), 8);
        assert_eq!(k.layout_picker, picker);
        assert_eq!(
            k.repeat,
            RepeatDef {
                delay_ms: 300,
                interval_ms: 55
            }
        );
    }

    #[test]
    fn more_than_three_chords_on_one_action_is_error() {
        let src = source_with("Info", "        \"Info\": [\"a\", \"b\", \"c\", \"e\"],\n");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].message,
            "layout \"LeftHanded\": action \"Info\" has 4 keys; at most 3 are allowed"
        );
        assert_eq!(errs[0].line, u32::try_from(Action::ALL.len() + 3).ok());
        // Three is fine.
        let src = source_with("Info", "        \"Info\": [\"a\", \"b\", \"c\"],\n");
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        assert_eq!(k.chords(Layout::LeftHanded, Action::Info).len(), 3);
    }

    #[test]
    fn the_layout_picker_may_list_more_than_three_chords() {
        let picker = "        \"Confirm\": [\"f\", \"j\", \"Enter\", \"Space\"],\n";
        let src = source_with_picker(&empty_layouts(), picker);
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        assert_eq!(k.layout_picker.len(), 4);
    }

    #[test]
    fn escape_and_delete_are_errors_anywhere() {
        for chord in ["Escape", "Delete"] {
            let extra = format!("        \"Cancel\": [\"d\", \"{chord}\"],\n");
            let errs = errors(&source_with("Cancel", &extra));
            assert_eq!(
                errs,
                vec![format!(
                    "k.ron:{}: layout \"LeftHanded\": action \"Cancel\": \"{chord}\" can't be \
                     bound here: Esc and Delete are fixed, see controls.md",
                    Action::ALL.len() + 3
                )],
                "{chord}"
            );
            let picker = format!("        \"Cancel\": [\"{chord}\"],\n");
            let errs = errors(&source_with_picker(&empty_layouts(), &picker));
            assert_eq!(errs.len(), 1, "{chord}");
            assert!(
                errs[0].contains("layout_picker: action \"Cancel\""),
                "{}",
                errs[0]
            );
            assert!(errs[0].contains("Esc and Delete are fixed"), "{}", errs[0]);
        }
    }

    #[test]
    fn shifted_escape_and_delete_are_ordinary_chords() {
        let extra = "        \"Info\": [\"Shift+Escape\", \"Shift+Delete\"],
";
        let k = KeymapDef::from_source("k.ron", &source_with("Info", extra)).unwrap_or_default();
        assert_eq!(
            k.chords(Layout::LeftHanded, Action::Info),
            [Chord::shifted(Key::Escape), Chord::shifted(Key::Delete)]
        );
    }

    #[test]
    fn chords_keep_the_file_order() {
        let src = source_with(
            "Info",
            "        \"Info\": [\"Space\", \"e\", \"Shift+a\"],\n",
        );
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        let chords: Vec<String> = k
            .chords(Layout::LeftHanded, Action::Info)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(chords, ["Space", "e", "Shift+a"]);
        assert_eq!(k.chords(Layout::LeftHanded, Action::Debug), []);
        assert_eq!(
            KeymapDef::default().chords(Layout::LeftHanded, Action::Info),
            []
        );
    }

    #[test]
    fn required_and_rebindable_actions_follow_the_design() {
        // docs/design/controls.md, Rebinding keys.
        use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, EndTurn};
        let required: Vec<Action> = Action::ALL
            .into_iter()
            .filter(|a| a.is_required())
            .collect();
        assert_eq!(
            required,
            [
                CursorLeft,
                CursorDown,
                CursorUp,
                CursorRight,
                Confirm,
                Cancel,
                EndTurn
            ]
        );
        let fixed: Vec<Action> = Action::ALL
            .into_iter()
            .filter(|a| !a.is_rebindable())
            .collect();
        assert_eq!(fixed, [Action::Debug]);
    }

    #[test]
    fn reserved_chords() {
        let reserved: Vec<Chord> = Key::ALL
            .iter()
            .flat_map(|&k| [Chord::plain(k), Chord::shifted(k)])
            .filter(|c| c.is_reserved())
            .collect();
        assert_eq!(
            reserved,
            [Chord::plain(Key::Escape), Chord::plain(Key::Delete)]
        );
    }

    #[test]
    fn new_keys_have_readable_names() {
        for (name, key) in [
            (",", Key::Comma),
            (".", Key::Period),
            ("/", Key::Slash),
            ("'", Key::Apostrophe),
            ("[", Key::LeftBracket),
            ("]", Key::RightBracket),
            ("\\", Key::Backslash),
            ("-", Key::Minus),
            ("=", Key::Equal),
            ("`", Key::Backquote),
            ("Insert", Key::Insert),
            ("Delete", Key::Delete),
            ("Home", Key::Home),
            ("End", Key::End),
            ("PageUp", Key::PageUp),
            ("PageDown", Key::PageDown),
            ("Kp0", Key::Kp0),
            ("Kp9", Key::Kp9),
            ("Kp+", Key::KpAdd),
            ("Kp-", Key::KpSubtract),
            ("Kp*", Key::KpMultiply),
            ("Kp/", Key::KpDivide),
            ("Kp.", Key::KpDecimal),
        ] {
            assert_eq!(Chord::parse(name), Ok(Chord::plain(key)), "{name}");
            let shifted = format!("Shift+{name}");
            assert_eq!(Chord::parse(&shifted), Ok(Chord::shifted(key)), "{shifted}");
        }
    }

    #[test]
    fn every_key_round_trips_through_its_chord_name() {
        for &key in Key::ALL {
            for chord in [Chord::plain(key), Chord::shifted(key)] {
                assert_eq!(Chord::parse(&chord.to_string()), Ok(chord));
            }
        }
    }

    #[test]
    fn punctuation_chords_load_from_a_keymap_file() {
        // RON strings need `\\` for a backslash; everything else is literal.
        let extra = "        \"Info\": [\",\", \"\\\\\", \"Kp+\"],\n";
        let k = KeymapDef::from_source("k.ron", &source_with("Info", extra)).unwrap_or_default();
        assert_eq!(
            k.chords(Layout::LeftHanded, Action::Info),
            [
                Chord::plain(Key::Comma),
                Chord::plain(Key::Backslash),
                Chord::plain(Key::KpAdd)
            ]
        );
    }
}
