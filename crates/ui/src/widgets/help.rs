//! Help text that names keys. Key names always come from the active
//! [`Keymap`], never string literals, because each layout binds actions to
//! different keys (ADR-0015).

use crate::input::{Action, Key, Keymap};

/// Separator between the parts of a help line.
pub const SEPARATOR: &str = " · ";

/// The name of the key for `action` (its [`Keymap::primary`] chord), e.g.
/// `f` or `Shift+Space`; `None` if unbound.
pub fn key_name(keymap: &Keymap, action: Action) -> Option<String> {
    keymap.primary(action).map(|c| c.to_string())
}

/// What moves the cursor: `arrows` when the four cursor actions are on the
/// arrow keys, otherwise their keys in up-left-down-right order (`wasd`).
/// `None` if any cursor action is unbound.
pub fn cursor_keys_name(keymap: &Keymap) -> Option<String> {
    let order = [
        (Action::CursorUp, Key::Up),
        (Action::CursorLeft, Key::Left),
        (Action::CursorDown, Key::Down),
        (Action::CursorRight, Key::Right),
    ];
    let chords = order
        .iter()
        .map(|&(action, _)| keymap.primary(action))
        .collect::<Option<Vec<_>>>()?;
    let arrows = chords
        .iter()
        .zip(order)
        .all(|(chord, (_, arrow))| !chord.shift && chord.key == arrow);
    Some(if arrows {
        "arrows".to_owned()
    } else {
        chords.iter().map(ToString::to_string).collect()
    })
}

/// Joins `key label` hints with [`SEPARATOR`], leaving out hints whose key
/// is `None` (unbound).
pub fn help_line(hints: &[(Option<String>, &str)]) -> String {
    hints
        .iter()
        .filter_map(|(key, label)| key.as_ref().map(|k| format!("{k} {label}")))
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

#[cfg(test)]
mod tests {
    use trpg_content::{Chord, KeymapDef};

    use super::*;

    fn keymap(pairs: &[(&str, Action)]) -> Keymap {
        let def = KeymapDef {
            bindings: pairs
                .iter()
                .map(|&(c, a)| (Chord::parse(c).unwrap(), a))
                .collect(),
            repeat: trpg_content::RepeatDef::default(),
        };
        Keymap::from_def(&def)
    }

    #[test]
    fn key_names() {
        let km = keymap(&[
            ("f", Action::Confirm),
            ("Shift+Space", Action::ToggleAutoEnd),
        ]);
        assert_eq!(key_name(&km, Action::Confirm).as_deref(), Some("f"));
        assert_eq!(
            key_name(&km, Action::ToggleAutoEnd).as_deref(),
            Some("Shift+Space")
        );
        assert_eq!(key_name(&km, Action::Cancel), None);
    }

    #[test]
    fn arrow_keys_are_called_arrows() {
        let km = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("Right", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(&km).as_deref(), Some("arrows"));
    }

    #[test]
    fn other_cursor_keys_are_listed() {
        let wasd = keymap(&[
            ("w", Action::CursorUp),
            ("a", Action::CursorLeft),
            ("s", Action::CursorDown),
            ("d", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(&wasd).as_deref(), Some("wasd"));
        // One arrow out of place, or shifted arrows, is not "arrows".
        let mixed = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("l", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(&mixed).as_deref(), Some("UpLeftDownl"));
        let shifted = keymap(&[
            ("Shift+Up", Action::CursorUp),
            ("Shift+Left", Action::CursorLeft),
            ("Shift+Down", Action::CursorDown),
            ("Shift+Right", Action::CursorRight),
        ]);
        assert_eq!(
            cursor_keys_name(&shifted).as_deref(),
            Some("Shift+UpShift+LeftShift+DownShift+Right")
        );
    }

    #[test]
    fn unbound_cursor_key_gives_none() {
        let km = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
        ]);
        assert_eq!(cursor_keys_name(&km), None);
    }

    #[test]
    fn help_line_skips_unbound() {
        let line = help_line(&[
            (Some("arrows".into()), "move"),
            (None, "info"),
            (Some("f".into()), "select"),
            (Some("d".into()), "back"),
        ]);
        assert_eq!(line, "arrows move · f select · d back");
        assert_eq!(help_line(&[]), "");
        assert_eq!(help_line(&[(None, "x")]), "");
    }
}
