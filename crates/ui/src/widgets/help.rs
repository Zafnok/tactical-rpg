//! Help text that names keys. Key names always come from the active
//! [`Keymap`], never string literals, because each layout binds actions to
//! different keys (ADR-0015) and the player can rebind them (ADR-0031).

use crate::input::{Action, Keymap};

/// Separator between the parts of a help line.
pub const SEPARATOR: &str = " · ";

/// What help text and tips show for an action with no key
/// (`docs/design/controls.md`, *Rebinding keys*).
pub const NOT_MAPPED: &str = "! not mapped";

/// The name of the key for `action` (its [`Keymap::primary`] chord), e.g.
/// `f` or `Shift+Space`; [`NOT_MAPPED`] if it has no key.
pub fn key_name(keymap: &Keymap, action: Action) -> String {
    keymap
        .primary(action)
        .map_or_else(|| NOT_MAPPED.to_owned(), |c| c.to_string())
}

/// Every key for `action`, joined with `/` (e.g. `f/j/Enter/Space`): its
/// slots in [`Keymap::chords_for`] order, then its fixed keys
/// ([`Keymap::fixed_chords_for`]). [`NOT_MAPPED`] if its slots are empty.
pub fn all_key_names(keymap: &Keymap, action: Action) -> String {
    let own = keymap.chords_for(action);
    if own.is_empty() {
        return NOT_MAPPED.to_owned();
    }
    let names: Vec<String> = own
        .iter()
        .chain(&Keymap::fixed_chords_for(action))
        .map(ToString::to_string)
        .collect();
    names.join("/")
}

/// What moves the cursor: `arrows` when the four cursor actions are on the
/// arrow keys, otherwise their keys in up-left-down-right order (`wasd`).
/// [`NOT_MAPPED`] if any cursor action has no key. See
/// [`Keymap::cursor_keys_name`].
pub fn cursor_keys_name(keymap: &Keymap) -> String {
    keymap
        .cursor_keys_name()
        .unwrap_or_else(|| NOT_MAPPED.to_owned())
}

/// Joins `key label` hints with [`SEPARATOR`], leaving out hints whose key
/// is `None` (hidden by the caller, e.g. a key that does nothing here).
pub fn help_line(hints: &[(Option<String>, &str)]) -> String {
    hints
        .iter()
        .filter_map(|(key, label)| key.as_ref().map(|k| format!("{k} {label}")))
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

#[cfg(test)]
mod tests {
    use trpg_content::{Chord, RepeatDef};

    use super::*;

    fn keymap(pairs: &[(&str, Action)]) -> Keymap {
        Keymap::new(
            pairs.iter().map(|&(c, a)| (Chord::parse(c).unwrap(), a)),
            RepeatDef::default(),
        )
    }

    #[test]
    fn all_key_names_lists_every_chord_in_slot_order_then_the_fixed_ones() {
        let km = keymap(&[
            ("Space", Action::Confirm),
            ("f", Action::Confirm),
            ("Enter", Action::Confirm),
            ("d", Action::Cancel),
        ]);
        assert_eq!(all_key_names(&km, Action::Confirm), "Space/f/Enter");
        assert_eq!(all_key_names(&km, Action::Cancel), "d/Escape");
        assert_eq!(all_key_names(&km, Action::Info), NOT_MAPPED);
        // Esc alone doesn't count as a key of Cancel's own.
        assert_eq!(all_key_names(&keymap(&[]), Action::Cancel), NOT_MAPPED);
    }

    #[test]
    fn key_names() {
        let km = keymap(&[
            ("f", Action::Confirm),
            ("Shift+Space", Action::ToggleAutoEnd),
        ]);
        assert_eq!(key_name(&km, Action::Confirm), "f");
        assert_eq!(key_name(&km, Action::ToggleAutoEnd), "Shift+Space");
        // Cancel shows its own keys, never the fixed Esc.
        assert_eq!(key_name(&km, Action::Cancel), "! not mapped");
        assert_eq!(key_name(&km, Action::Info), NOT_MAPPED);
    }

    #[test]
    fn arrow_keys_are_called_arrows() {
        let km = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("Right", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(&km), "arrows");
    }

    #[test]
    fn other_cursor_keys_are_listed() {
        let wasd = keymap(&[
            ("w", Action::CursorUp),
            ("a", Action::CursorLeft),
            ("s", Action::CursorDown),
            ("d", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(&wasd), "wasd");
        // One arrow out of place, or shifted arrows, is not "arrows".
        let mixed = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("l", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(&mixed), "UpLeftDownl");
        let shifted = keymap(&[
            ("Shift+Up", Action::CursorUp),
            ("Shift+Left", Action::CursorLeft),
            ("Shift+Down", Action::CursorDown),
            ("Shift+Right", Action::CursorRight),
        ]);
        assert_eq!(
            cursor_keys_name(&shifted),
            "Shift+UpShift+LeftShift+DownShift+Right"
        );
    }

    #[test]
    fn unbound_cursor_key_is_not_mapped() {
        let km = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
        ]);
        assert_eq!(cursor_keys_name(&km), NOT_MAPPED);
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
