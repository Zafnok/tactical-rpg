//! Translation from macroquad key codes to `trpg-ui`'s hardware-agnostic
//! [`Key`], and feeding a frame's keyboard events into [`InputState`].

use macroquad::prelude::{
    KeyCode, get_frame_time, get_keys_pressed, get_keys_released, is_key_down,
};
use trpg_ui::input::{Action, Chord, InputState, Key};

/// The game key for a macroquad key code; `None` for keys the game ignores.
pub fn to_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::A => Key::A,
        KeyCode::B => Key::B,
        KeyCode::C => Key::C,
        KeyCode::D => Key::D,
        KeyCode::E => Key::E,
        KeyCode::F => Key::F,
        KeyCode::G => Key::G,
        KeyCode::H => Key::H,
        KeyCode::I => Key::I,
        KeyCode::J => Key::J,
        KeyCode::K => Key::K,
        KeyCode::L => Key::L,
        KeyCode::M => Key::M,
        KeyCode::N => Key::N,
        KeyCode::O => Key::O,
        KeyCode::P => Key::P,
        KeyCode::Q => Key::Q,
        KeyCode::R => Key::R,
        KeyCode::S => Key::S,
        KeyCode::T => Key::T,
        KeyCode::U => Key::U,
        KeyCode::V => Key::V,
        KeyCode::W => Key::W,
        KeyCode::X => Key::X,
        KeyCode::Y => Key::Y,
        KeyCode::Z => Key::Z,
        KeyCode::Key0 => Key::Digit0,
        KeyCode::Key1 => Key::Digit1,
        KeyCode::Key2 => Key::Digit2,
        KeyCode::Key3 => Key::Digit3,
        KeyCode::Key4 => Key::Digit4,
        KeyCode::Key5 => Key::Digit5,
        KeyCode::Key6 => Key::Digit6,
        KeyCode::Key7 => Key::Digit7,
        KeyCode::Key8 => Key::Digit8,
        KeyCode::Key9 => Key::Digit9,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter | KeyCode::KpEnter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Space => Key::Space,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,
        _ => return None,
    })
}

/// Feeds this frame's key presses and releases into `input` and returns the
/// actions for the frame.
pub fn poll(input: &mut InputState) -> Vec<Action> {
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    for key in get_keys_released().into_iter().filter_map(to_key) {
        input.key_up(key);
    }
    for key in get_keys_pressed().into_iter().filter_map(to_key) {
        input.key_down(Chord { key, shift });
    }
    input.update(get_frame_time())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_game_key_has_a_key_code() {
        let codes = [
            KeyCode::A,
            KeyCode::Z,
            KeyCode::Key0,
            KeyCode::Key9,
            KeyCode::Up,
            KeyCode::Right,
            KeyCode::Enter,
            KeyCode::Escape,
            KeyCode::Space,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::F1,
            KeyCode::F12,
        ];
        let keys: Vec<String> = codes
            .into_iter()
            .filter_map(to_key)
            .map(|k| k.to_string())
            .collect();
        assert_eq!(
            keys,
            [
                "a",
                "z",
                "0",
                "9",
                "Up",
                "Right",
                "Enter",
                "Escape",
                "Space",
                "Tab",
                "Backspace",
                "F1",
                "F12"
            ]
        );
        assert_eq!(to_key(KeyCode::KpEnter), Some(Key::Enter));
    }

    #[test]
    fn other_key_codes_are_ignored() {
        for code in [
            KeyCode::LeftShift,
            KeyCode::Comma,
            KeyCode::Kp1,
            KeyCode::F13,
        ] {
            assert_eq!(to_key(code), None);
        }
    }
}
