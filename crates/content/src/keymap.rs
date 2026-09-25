//! The default keymap (`assets/data/keymap.ron`, ADR-0006): which key chords
//! trigger which [`Action`], plus key-repeat timings.
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
            /// (`0`), and capitalised names for the rest (`Left`, `F12`).
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
        "unknown key chord \"{chord}\"{hint}; expected a key such as \"h\", \"0\", \"Left\", \
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
    /// Move the cursor several tiles left.
    CursorJumpLeft,
    /// Move the cursor several tiles down.
    CursorJumpDown,
    /// Move the cursor several tiles up.
    CursorJumpUp,
    /// Move the cursor several tiles right.
    CursorJumpRight,
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
    pub const ALL: [Action; 18] = [
        Self::CursorLeft,
        Self::CursorDown,
        Self::CursorUp,
        Self::CursorRight,
        Self::CursorJumpLeft,
        Self::CursorJumpDown,
        Self::CursorJumpUp,
        Self::CursorJumpRight,
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
        const NAMES: [&str; 18] = [
            "CursorLeft",
            "CursorDown",
            "CursorUp",
            "CursorRight",
            "CursorJumpLeft",
            "CursorJumpDown",
            "CursorJumpUp",
            "CursorJumpRight",
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

    /// Whether holding the key re-emits the action (the eight cursor moves).
    pub fn is_repeatable(self) -> bool {
        matches!(
            self,
            Self::CursorLeft
                | Self::CursorDown
                | Self::CursorUp
                | Self::CursorRight
                | Self::CursorJumpLeft
                | Self::CursorJumpDown
                | Self::CursorJumpUp
                | Self::CursorJumpRight
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

/// The validated keymap: each chord maps to exactly one action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeymapDef {
    /// Chord → action. Unbound chords are absent.
    pub bindings: BTreeMap<Chord, Action>,
    /// Key-repeat timings.
    pub repeat: RepeatDef,
}

/// The file as written, before validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKeymap {
    bindings: BTreeMap<String, Vec<String>>,
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

    /// Parses and validates keymap `source`, attributing errors to `file`.
    /// Reports every unknown action, unparsable chord, chord bound more than
    /// once, action missing from the file, and bad repeat timing.
    pub fn from_source(file: &str, source: &str) -> Result<Self, Vec<ContentError>> {
        let raw: RawKeymap = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut errors = Vec::new();
        let mut bindings: BTreeMap<Chord, Action> = BTreeMap::new();
        let err_at = |key: &str, message: String| {
            let e = ContentError::new(file, message);
            match line_of_quoted(source, key) {
                Some(line) => e.at(line, None),
                None => e,
            }
        };
        for (action_name, chords) in &raw.bindings {
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
                                    "chord \"{chord}\" is bound to both \"{other}\" and \
                                     \"{action}\""
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
            if !raw.bindings.contains_key(action.name()) {
                errors.push(ContentError::new(
                    file,
                    format!("missing action \"{action}\" (list it with [] to leave it unbound)"),
                ));
            }
        }
        if raw.repeat.interval_ms == 0 {
            errors.push(err_at(
                "interval_ms",
                "repeat interval_ms must be at least 1".into(),
            ));
        }
        if errors.is_empty() {
            Ok(Self {
                bindings,
                repeat: raw.repeat,
            })
        } else {
            Err(errors)
        }
    }
}

/// 1-based line of the first line containing `key` as a whole word (quoted
/// map keys or bare field names), for error positions.
fn line_of_quoted(source: &str, key: &str) -> Option<u32> {
    let quoted = format!("\"{key}\"");
    let field = format!("{key}:");
    let index = source
        .lines()
        .position(|l| l.contains(&quoted) || l.trim_start().starts_with(&field))?;
    u32::try_from(index + 1).ok()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A complete valid keymap source with `extra` spliced into `bindings`.
    fn source_with(skip: &str, extra: &str) -> String {
        let lines: Vec<String> = Action::ALL
            .iter()
            .filter(|a| a.name() != skip)
            .map(|a| format!("        \"{a}\": [],\n"))
            .collect();
        format!(
            "(\n    bindings: {{\n{}{extra}    }},\n    repeat: (delay_ms: 170, interval_ms: 55),\n)",
            lines.concat()
        )
    }

    fn errors(src: &str) -> Vec<String> {
        KeymapDef::from_source("k.ron", src)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn chord_parse_plain_and_shifted() {
        assert_eq!(Chord::parse("h"), Ok(Chord::plain(Key::H)));
        assert_eq!(Chord::parse("Shift+h"), Ok(Chord::shifted(Key::H)));
        assert_eq!(Chord::parse("Shift+Tab"), Ok(Chord::shifted(Key::Tab)));
        assert_eq!(Chord::parse("F12"), Ok(Chord::plain(Key::F12)));
        assert_eq!(Chord::parse("7"), Ok(Chord::plain(Key::Digit7)));
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
    }

    #[test]
    fn key_and_action_tables_line_up() {
        assert_eq!(Key::ALL.len(), 26 + 10 + 4 + 5 + 12);
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
        assert_eq!(repeatable, Action::ALL[..8].to_vec());
        assert!(
            Action::ALL[..8]
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

    #[test]
    fn valid_keymap_loads() {
        let src = source_with("Confirm", "        \"Confirm\": [\"f\", \"Space\"],\n");
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        assert_eq!(
            k.bindings.get(&Chord::plain(Key::F)),
            Some(&Action::Confirm)
        );
        assert_eq!(
            k.bindings.get(&Chord::plain(Key::Space)),
            Some(&Action::Confirm)
        );
        assert_eq!(k.bindings.len(), 2);
        assert_eq!(k.repeat, RepeatDef::default());
    }

    #[test]
    fn unknown_action_is_error_with_line() {
        let src = source_with("", "        \"Attack\": [\"f\"],\n");
        let errs = KeymapDef::from_source("k.ron", &src)
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("unknown action \"Attack\""));
        assert!(errs[0].message.contains("Confirm"));
        let line = u32::try_from(Action::ALL.len() + 3).unwrap_or(0);
        assert_eq!(errs[0].line, Some(line));
    }

    #[test]
    fn bad_chord_is_error() {
        let src = source_with("Info", "        \"Info\": [\"s\", \"Ctrl+i\"],\n");
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("\"Info\""), "{}", errs[0]);
        assert!(errs[0].contains("\"Ctrl+i\""), "{}", errs[0]);
    }

    #[test]
    fn conflicting_chord_is_error() {
        let src = source_with("Info", "        \"Info\": [\"f\"],\n")
            .replace("\"Confirm\": []", "\"Confirm\": [\"f\"]");
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0].contains("chord \"f\" is bound to both \"Confirm\" and \"Info\""),
            "{}",
            errs[0]
        );
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
            .replace("\"CursorLeft\": []", "\"CursorLeft\": [\"h\"]")
            .replace(
                "\"CursorJumpLeft\": []",
                "\"CursorJumpLeft\": [\"Shift+h\"]",
            );
        let k = KeymapDef::from_source("k.ron", &src).unwrap_or_default();
        assert_eq!(k.bindings.len(), 2);
    }

    #[test]
    fn missing_action_is_error() {
        let errs = errors(&source_with("Debug", ""));
        assert_eq!(
            errs,
            vec!["k.ron: missing action \"Debug\" (list it with [] to leave it unbound)"]
        );
    }

    #[test]
    fn zero_interval_is_error() {
        let src = source_with("", "").replace("interval_ms: 55", "interval_ms: 0");
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("interval_ms"), "{}", errs[0]);
    }

    #[test]
    fn all_errors_reported_together() {
        let src = source_with(
            "Confirm",
            "        \"Confirm\": [\"Nope\", \"f\"],\n        \"Bogus\": [],\n",
        )
        .replace("\"Cancel\": []", "\"Cancel\": [\"f\"]")
        .replace("interval_ms: 55", "interval_ms: 0");
        assert_eq!(errors(&src).len(), 4);
    }

    #[test]
    fn syntax_error_is_positioned() {
        let errs = KeymapDef::from_source("k.ron", "(\n  bindings: {\n  \"a\" [] },\n)")
            .err()
            .unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(3));
    }

    #[test]
    fn line_of_quoted_finds_keys_and_fields() {
        let src = "(\n \"Info\": [],\n    interval_ms: 3,\n)";
        assert_eq!(line_of_quoted(src, "Info"), Some(2));
        assert_eq!(line_of_quoted(src, "interval_ms"), Some(3));
        assert_eq!(line_of_quoted(src, "nope"), None);
    }

    #[test]
    fn embedded_keymap_matches_adr_0006() {
        let k = KeymapDef::load().unwrap_or_default();
        let get = |s: &str| {
            Chord::parse(s)
                .ok()
                .and_then(|c| k.bindings.get(&c).copied())
        };
        assert_eq!(get("l"), Some(Action::CursorRight));
        assert_eq!(get("Right"), Some(Action::CursorRight));
        assert_eq!(get("Shift+j"), Some(Action::CursorJumpDown));
        assert_eq!(get("f"), Some(Action::Confirm));
        assert_eq!(get("Escape"), Some(Action::Cancel));
        assert_eq!(get("Shift+Tab"), Some(Action::PrevUnit));
        assert_eq!(get("Shift+e"), Some(Action::ToggleAutoEnd));
        assert_eq!(get("F12"), Some(Action::Debug));
        assert_eq!(
            k.repeat,
            RepeatDef {
                delay_ms: 170,
                interval_ms: 55
            }
        );
    }
}
