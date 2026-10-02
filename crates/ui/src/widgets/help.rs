//! Help text that names keys and controller buttons. Names always come
//! from the active [`Keymap`], never string literals, because each layout
//! binds actions to different keys (ADR-0015) and the player can rebind
//! them (ADR-0031); and they follow the [`Device`] the player pressed last
//! (`docs/design/controls.md`, *Controller*): keys on the keyboard, the
//! pad's own button names on a controller.

use std::ops::Deref;

use crate::input::{Action, Device, Keymap};

/// What help text names keys from: the bindings, and the device whose keys
/// or buttons to name. Screens get it from
/// [`Ctx::help_keys`](crate::screen::Ctx::help_keys). Dereferences to the
/// [`Keymap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpKeys<'a> {
    keymap: &'a Keymap,
    device: Device,
}

impl<'a> HelpKeys<'a> {
    /// `keymap`'s keys, or its buttons as a pad names them, by `device`.
    pub fn new(keymap: &'a Keymap, device: Device) -> Self {
        Self { keymap, device }
    }

    /// `keymap`'s keyboard keys, whatever the player is using (a picture
    /// of the keyboard, a keyboard-only key).
    pub fn keyboard(keymap: &'a Keymap) -> Self {
        Self::new(keymap, Device::Keyboard)
    }

    /// The device whose keys or buttons are named.
    pub fn device(self) -> Device {
        self.device
    }
}

impl Deref for HelpKeys<'_> {
    type Target = Keymap;

    fn deref(&self) -> &Keymap {
        self.keymap
    }
}

/// Separator between the parts of a help line.
pub const SEPARATOR: &str = " · ";

/// What help text and tips show for an action with no key, or no button
/// on a controller (`docs/design/controls.md`, *Rebinding keys*).
pub const NOT_MAPPED: &str = "! not mapped";

/// The name of the key for `action` (its [`Keymap::primary`] chord), e.g.
/// `f` or `Shift+Space`; on a controller, of its button
/// ([`Keymap::primary_button`]) as that pad labels it, e.g. `A`.
/// [`NOT_MAPPED`] if it has none.
pub fn key_name(keys: HelpKeys<'_>, action: Action) -> String {
    let name = match keys.device {
        Device::Keyboard => keys.primary(action).map(|c| c.to_string()),
        Device::Pad(kind) => {
            let button = keys.primary_button(action);
            button.map(|b| kind.button_name(b).to_owned())
        }
    };
    name.unwrap_or_else(|| NOT_MAPPED.to_owned())
}

/// Every key for `action`, joined with `/` (e.g. `f/j/Enter/Space`): its
/// slots in [`Keymap::chords_for`] order, then its fixed keys
/// ([`Keymap::fixed_chords_for`]). On a controller, every button for it
/// ([`Keymap::buttons_for`]). [`NOT_MAPPED`] if its slots are empty.
pub fn all_key_names(keys: HelpKeys<'_>, action: Action) -> String {
    let names: Vec<String> = match keys.device {
        Device::Keyboard => {
            let own = keys.chords_for(action);
            // The fixed keys alone don't count as a key of its own.
            let fixed = if own.is_empty() {
                Vec::new()
            } else {
                Keymap::fixed_chords_for(action)
            };
            own.iter().chain(&fixed).map(ToString::to_string).collect()
        }
        Device::Pad(kind) => {
            let buttons = keys.buttons_for(action);
            let name = |&b| kind.button_name(b).to_owned();
            buttons.iter().map(name).collect()
        }
    };
    if names.is_empty() {
        return NOT_MAPPED.to_owned();
    }
    names.join("/")
}

/// What moves the cursor: `arrows` when the four cursor actions are on the
/// arrow keys, otherwise their keys in up-left-down-right order (`wasd`).
/// On a controller, `D-pad/stick` or whatever the cursor is on
/// ([`Keymap::cursor_buttons_name`]). [`NOT_MAPPED`] if any cursor action
/// has no key. See [`Keymap::cursor_keys_name`].
pub fn cursor_keys_name(keys: HelpKeys<'_>) -> String {
    let name = match keys.device {
        Device::Keyboard => keys.cursor_keys_name(),
        Device::Pad(kind) => keys.cursor_buttons_name(kind),
    };
    name.unwrap_or_else(|| NOT_MAPPED.to_owned())
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
    use crate::input::{Button, KeymapDef, Layout, PadKind};

    const XBOX: Device = Device::Pad(PadKind::Xbox);
    const PLAYSTATION: Device = Device::Pad(PadKind::PlayStation);
    const NINTENDO: Device = Device::Pad(PadKind::Nintendo);

    /// The right-handed defaults: keys and buttons.
    fn defaults() -> Keymap {
        let def = KeymapDef::load().unwrap_or_else(|e| panic!("{e:?}"));
        Keymap::for_layout(&def, Layout::RightHanded)
    }

    #[test]
    fn help_keys_are_the_keymap_and_a_device() {
        let km = defaults();
        let keys = HelpKeys::new(&km, XBOX);
        assert_eq!(keys.device(), XBOX);
        assert_eq!(HelpKeys::keyboard(&km).device(), Device::Keyboard);
        assert_eq!(
            HelpKeys::keyboard(&km),
            HelpKeys::new(&km, Device::Keyboard)
        );
        assert_ne!(keys, HelpKeys::keyboard(&km));
        // It stands in for the keymap.
        assert_eq!(keys.select_action(), Action::Confirm);
        assert_eq!(keys.primary_button(Action::Confirm), Some(Button::South));
    }

    /// `docs/design/controls.md`, *Button names on screen*: the example
    /// battle help bar on each kind of pad, and the keys again.
    #[test]
    fn key_names_follow_the_device() {
        let km = defaults();
        let line = |device| {
            let keys = HelpKeys::new(&km, device);
            help_line(&[
                (Some(cursor_keys_name(keys)), "move"),
                (Some(key_name(keys, Action::Confirm)), "select"),
                (Some(key_name(keys, Action::Info)), "info"),
                (Some(key_name(keys, Action::NextUnit)), "next unit"),
                (Some(key_name(keys, Action::Rewind)), "rewind"),
                (Some(key_name(keys, Action::Cancel)), "back"),
                (Some(key_name(keys, Action::EndTurn)), "end turn"),
            ])
        };
        assert_eq!(
            line(Device::Keyboard),
            "arrows move · f select · e info · s next unit · r rewind · d back · Space end turn"
        );
        let xbox = "D-pad/stick move · A select · Y info · RB next unit · LT rewind · B back \
                    · Start end turn";
        assert_eq!(line(XBOX), xbox);
        assert_eq!(line(Device::Pad(PadKind::Generic)), xbox);
        assert_eq!(
            line(PLAYSTATION),
            "D-pad/stick move · ✕ select · △ info · R1 next unit · L2 rewind · ◯ back \
             · Options end turn"
        );
        assert_eq!(
            line(NINTENDO),
            "D-pad/stick move · A select · X info · R next unit · ZL rewind · B back \
             · + end turn"
        );
    }

    #[test]
    fn the_other_default_buttons_are_named_too() {
        let km = defaults();
        let name = |device, action| key_name(HelpKeys::new(&km, device), action);
        let names = |action| [XBOX, PLAYSTATION, NINTENDO].map(|d| name(d, action));
        assert_eq!(names(Action::PrevUnit), ["LB", "L1", "L"]);
        assert_eq!(names(Action::DangerZone), ["X", "□", "Y"]);
        assert_eq!(names(Action::ToggleAutoEnd), ["Back", "Create", "−"]);
        assert_eq!(names(Action::CursorLeft), ["←", "←", "←"]);
    }

    /// An action with no button shows `! not mapped` on a pad, whatever
    /// keys it has; and the other way round.
    #[test]
    fn an_action_with_no_button_is_not_mapped_on_a_pad() {
        let km = defaults();
        for device in [XBOX, PLAYSTATION, NINTENDO] {
            let keys = HelpKeys::new(&km, device);
            // Keyboard-only by default: the debug key; empty everywhere:
            // Select.
            assert_eq!(key_name(keys, Action::Debug), NOT_MAPPED);
            assert_eq!(key_name(keys, Action::Select), "! not mapped");
            assert_eq!(all_key_names(keys, Action::Debug), NOT_MAPPED);
        }
        assert_ne!(key_name(HelpKeys::keyboard(&km), Action::Debug), NOT_MAPPED);
        // Buttons but no keys.
        let pad_only = keymap(&[]).with_pad([(Button::South, Action::Confirm)]);
        assert_eq!(
            key_name(HelpKeys::new(&pad_only, XBOX), Action::Confirm),
            "A"
        );
        let keys = HelpKeys::keyboard(&pad_only);
        assert_eq!(key_name(keys, Action::Confirm), NOT_MAPPED);
        assert_eq!(all_key_names(keys, Action::Confirm), NOT_MAPPED);
        // The fixed Cancel key is a key, not a button.
        assert_eq!(
            key_name(HelpKeys::new(&pad_only, XBOX), Action::Cancel),
            NOT_MAPPED
        );
        assert_eq!(
            all_key_names(HelpKeys::new(&pad_only, XBOX), Action::Cancel),
            NOT_MAPPED
        );
    }

    #[test]
    fn all_key_names_lists_every_button_in_order_on_a_pad() {
        let km = defaults();
        let names = |device, action| all_key_names(HelpKeys::new(&km, device), action);
        assert_eq!(names(XBOX, Action::CursorUp), "↑/stick ↑");
        assert_eq!(names(PLAYSTATION, Action::Confirm), "✕");
        // No fixed extra on a pad: Cancel is its button alone.
        assert_eq!(names(NINTENDO, Action::Cancel), "B");
        assert_eq!(names(Device::Keyboard, Action::Cancel), "d/Escape");
        let two = keymap(&[]).with_pad([
            (Button::RightTrigger, Action::Info),
            (Button::North, Action::Info),
        ]);
        assert_eq!(
            all_key_names(HelpKeys::new(&two, PLAYSTATION), Action::Info),
            "R2/△"
        );
        assert_eq!(
            key_name(HelpKeys::new(&two, PLAYSTATION), Action::Info),
            "R2"
        );
    }

    #[test]
    fn the_cursor_on_a_pad_is_its_buttons_or_not_mapped() {
        let km = defaults();
        assert_eq!(
            cursor_keys_name(HelpKeys::new(&km, PLAYSTATION)),
            "D-pad/stick"
        );
        assert_eq!(cursor_keys_name(HelpKeys::keyboard(&km)), "arrows");
        // Keys but no buttons, and the other way round.
        let arrows = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("Right", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(HelpKeys::new(&arrows, XBOX)), NOT_MAPPED);
        let dpad = keymap(&[]).with_pad([
            (Button::DpadUp, Action::CursorUp),
            (Button::DpadLeft, Action::CursorLeft),
            (Button::DpadDown, Action::CursorDown),
            (Button::DpadRight, Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(HelpKeys::new(&dpad, XBOX)), "D-pad");
        assert_eq!(cursor_keys_name(HelpKeys::keyboard(&dpad)), NOT_MAPPED);
    }

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
        assert_eq!(
            all_key_names(HelpKeys::keyboard(&km), Action::Confirm),
            "Space/f/Enter"
        );
        assert_eq!(
            all_key_names(HelpKeys::keyboard(&km), Action::Cancel),
            "d/Escape"
        );
        assert_eq!(
            all_key_names(HelpKeys::keyboard(&km), Action::Info),
            NOT_MAPPED
        );
        // Esc alone doesn't count as a key of Cancel's own.
        assert_eq!(
            all_key_names(HelpKeys::keyboard(&keymap(&[])), Action::Cancel),
            NOT_MAPPED
        );
    }

    #[test]
    fn key_names() {
        let km = keymap(&[
            ("f", Action::Confirm),
            ("Shift+Space", Action::ToggleAutoEnd),
        ]);
        assert_eq!(key_name(HelpKeys::keyboard(&km), Action::Confirm), "f");
        assert_eq!(
            key_name(HelpKeys::keyboard(&km), Action::ToggleAutoEnd),
            "Shift+Space"
        );
        // Cancel shows its own keys, never the fixed Esc.
        assert_eq!(
            key_name(HelpKeys::keyboard(&km), Action::Cancel),
            "! not mapped"
        );
        assert_eq!(key_name(HelpKeys::keyboard(&km), Action::Info), NOT_MAPPED);
    }

    #[test]
    fn arrow_keys_are_called_arrows() {
        let km = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("Right", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(HelpKeys::keyboard(&km)), "arrows");
    }

    #[test]
    fn other_cursor_keys_are_listed() {
        let wasd = keymap(&[
            ("w", Action::CursorUp),
            ("a", Action::CursorLeft),
            ("s", Action::CursorDown),
            ("d", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(HelpKeys::keyboard(&wasd)), "wasd");
        // One arrow out of place, or shifted arrows, is not "arrows".
        let mixed = keymap(&[
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("l", Action::CursorRight),
        ]);
        assert_eq!(cursor_keys_name(HelpKeys::keyboard(&mixed)), "UpLeftDownl");
        let shifted = keymap(&[
            ("Shift+Up", Action::CursorUp),
            ("Shift+Left", Action::CursorLeft),
            ("Shift+Down", Action::CursorDown),
            ("Shift+Right", Action::CursorRight),
        ]);
        assert_eq!(
            cursor_keys_name(HelpKeys::keyboard(&shifted)),
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
        assert_eq!(cursor_keys_name(HelpKeys::keyboard(&km)), NOT_MAPPED);
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
