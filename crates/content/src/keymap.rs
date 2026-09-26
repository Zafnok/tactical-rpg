//! The keymap (`assets/data/keymap.ron`, ADR-0015): for each [`Layout`],
//! which key chords trigger which [`Action`], plus key-repeat timings.
//!
//! [`Key`], [`Chord`] and [`Action`] live here (not in `trpg-ui`) because the
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
            /// (`0`), `;`, and capitalised names for the rest (`Left`, `F12`).
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
}

impl Key {
    /// Looks a key up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.name() == name)
    }
}

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
}

impl Action {
    /// Every action, in declaration order.
    pub const ALL: [Action; 14] = [
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
    ];

    /// The name used in keymap files (the variant name).
    pub fn name(self) -> &'static str {
        const NAMES: [&str; 14] = [
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
            delay_ms: 170,
            interval_ms: 55,
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

/// One layout's validated bindings: each chord maps to exactly one action.
/// Unbound chords are absent.
pub type Bindings = BTreeMap<Chord, Action>;

/// The validated keymap: every [`Layout`]'s bindings, plus the key-repeat
/// timings they share.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapDef {
    /// Layout → its bindings. After [`load`](Self::load) every layout is
    /// present.
    pub layouts: BTreeMap<Layout, Bindings>,
    /// Key-repeat timings.
    pub repeat: RepeatDef,
}

/// The file as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKeymap {
    layouts: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    repeat: RepeatDef,
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

    /// `layout`'s bindings (`None` only for a definition built by hand
    /// without it; a loaded keymap has every layout).
    pub fn bindings(&self, layout: Layout) -> Option<&Bindings> {
        self.layouts.get(&layout)
    }

    /// Parses and validates keymap `source`, attributing errors to `file`.
    /// Reports every unknown or missing layout, and within each layout every
    /// unknown action, unparsable chord, chord bound more than once and
    /// missing action (prefixed with the layout name), plus bad repeat
    /// timing.
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
            match validate_bindings(file, source, from, layout, actions) {
                Ok(bindings) => {
                    layouts.insert(layout, bindings);
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
        if raw.repeat.interval_ms == 0 {
            errors.push(positioned(
                ContentError::new(file, "repeat interval_ms must be at least 1"),
                line_of_quoted(source, 0, "repeat"),
            ));
        }
        if errors.is_empty() {
            Ok(Self {
                layouts,
                repeat: raw.repeat,
            })
        } else {
            Err(errors)
        }
    }
}

/// Validates one layout's `actions` (action name → chord names). Messages
/// start with the layout name; lines are searched from line index `from`
/// (just after the layout's own line) on.
fn validate_bindings(
    file: &str,
    source: &str,
    from: usize,
    layout: Layout,
    actions: &BTreeMap<String, Vec<String>>,
) -> Result<Bindings, Vec<ContentError>> {
    let mut errors = Vec::new();
    let mut bindings = Bindings::new();
    let err_at = |key: &str, message: String| {
        positioned(
            ContentError::new(file, format!("layout \"{layout}\": {message}")),
            line_of_quoted(source, from, key),
        )
    };
    for (action_name, chords) in actions {
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
        for text in chords {
            match Chord::parse(text) {
                Ok(chord) => {
                    if let Some(&other) = bindings.get(&chord) {
                        let message = if other == action {
                            format!("chord \"{chord}\" is listed twice for \"{action}\"")
                        } else {
                            format!(
                                "chord \"{chord}\" is bound to both \"{other}\" and \"{action}\""
                            )
                        };
                        errors.push(err_at(action_name, message));
                    } else {
                        bindings.insert(chord, action);
                    }
                }
                Err(why) => errors.push(err_at(
                    action_name,
                    format!("action \"{action_name}\": {why}"),
                )),
            }
        }
    }
    for action in Action::ALL {
        if !actions.contains_key(action.name()) {
            errors.push(ContentError::new(
                file,
                format!(
                    "layout \"{layout}\": missing action \"{action}\" (list it with [] to leave \
                     it unbound)"
                ),
            ));
        }
    }
    if errors.is_empty() {
        Ok(bindings)
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
        assert_eq!(Key::ALL.len(), 26 + 10 + 1 + 4 + 5 + 12);
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

    /// A keymap source holding `blocks` as its layouts.
    fn source_of(blocks: &[String]) -> String {
        format!(
            "(\n    layouts: {{\n{}    }},\n    repeat: (delay_ms: 170, interval_ms: 55),\n)",
            blocks.concat()
        )
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
        k.bindings(Layout::LeftHanded).cloned().unwrap_or_default()
    }

    #[test]
    fn valid_keymap_loads() {
        let src = source_with("Confirm", "        \"Confirm\": [\"f\", \"Space\"],\n");
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        let b = left(&k);
        assert_eq!(b.get(&Chord::plain(Key::F)), Some(&Action::Confirm));
        assert_eq!(b.get(&Chord::plain(Key::Space)), Some(&Action::Confirm));
        assert_eq!(b.len(), 2);
        assert_eq!(k.bindings(Layout::RightHanded).map(BTreeMap::len), Some(0));
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
            .and_then(|k| k.bindings(layout).cloned())
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
        assert_eq!(get("Escape"), Some(Action::Cancel));
        assert_eq!(get("a"), Some(Action::PrevUnit));
        assert_eq!(get("s"), Some(Action::NextUnit));
        assert_eq!(get("e"), Some(Action::Info));
        assert_eq!(get("w"), Some(Action::DangerZone));
        assert_eq!(get("Space"), Some(Action::EndTurn));
        assert_eq!(get("Shift+Space"), Some(Action::ToggleAutoEnd));
        assert_eq!(get("F12"), Some(Action::Debug));
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
        assert_eq!(get("Escape"), Some(Action::Cancel));
        assert_eq!(get(";"), Some(Action::PrevUnit));
        assert_eq!(get("l"), Some(Action::NextUnit));
        assert_eq!(get("i"), Some(Action::Info));
        assert_eq!(get("o"), Some(Action::DangerZone));
        assert_eq!(get("Space"), Some(Action::EndTurn));
        assert_eq!(get("Shift+Space"), Some(Action::ToggleAutoEnd));
        assert_eq!(get("F12"), Some(Action::Debug));
    }

    #[test]
    fn embedded_keymap_has_exactly_the_design_bindings() {
        // 14 chords per layout: nothing bound beyond the design table.
        let k = KeymapDef::load().unwrap_or_default();
        for layout in Layout::ALL {
            assert_eq!(k.bindings(layout).map(BTreeMap::len), Some(14), "{layout}");
        }
        assert_eq!(k.layouts.len(), 2);
        assert_eq!(
            k.repeat,
            RepeatDef {
                delay_ms: 170,
                interval_ms: 55
            }
        );
    }
}
