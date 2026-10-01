//! Scripted tests of playing with a controller through the real game
//! (ADR-0007 layer 4; `docs/design/controls.md`, *Controller*; ticket
//! 0219). A controller gives the same actions as the keys, so most tests
//! play the same thing twice, once with keys and once with buttons, and
//! compare the screens. Help bars still name keys (button names are ticket
//! 0220).

use trpg_ui::harness::{FRAME_DT, Harness};
use trpg_ui::input::Layout;

fn title() -> Harness {
    Harness::with_layout(Layout::RightHanded)
}

/// At the title with the right-handed layout, then Quick Battle, chosen
/// with the keyboard.
fn quick_battle() -> Harness {
    let mut h = title();
    h.keys("Down f");
    h
}

/// Whether some row of the screen contains `text`.
fn shows(h: &Harness, text: &str) -> bool {
    let buf = h.game().buffer();
    (0..32).any(|y| {
        let row: String = (0..100)
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect();
        row.contains(text)
    })
}

/// Plays `keys` on one Quick Battle and `buttons` on another and checks
/// both end on the same screen, which isn't the one they started from.
fn same_in_battle(keys: &str, buttons: &str) {
    let mut with_keys = quick_battle();
    let mut with_pad = quick_battle();
    let start = with_keys.snapshot();
    with_keys.keys(keys).wait(0.5);
    with_pad.pad(buttons).wait(0.5);
    assert_eq!(with_pad.screens(), with_keys.screens(), "{buttons}");
    assert_eq!(with_pad.snapshot(), with_keys.snapshot(), "{buttons}");
    assert_ne!(with_keys.snapshot(), start, "{keys} did nothing");
}

#[test]
fn the_title_menu_works_with_a_pad() {
    let mut h = title();
    // The bottom button confirms, the right one backs out.
    h.pad("South");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.pad("East");
    assert_eq!(h.screens(), ["title"]);
    // The D-pad and the left stick both move the menu.
    h.pad("DpadDown South");
    assert_eq!(h.screens(), ["title", "battle"]);
    let mut h = title();
    h.pad("LeftStickDown LeftStickDown South");
    assert!(h.quit_requested());
}

#[test]
fn quick_battle_starts_the_same_from_a_pad() {
    let mut h = title();
    h.pad("DpadDown South");
    assert_eq!(h.snapshot(), quick_battle().snapshot());
}

#[test]
fn each_default_button_does_what_its_key_does() {
    // Cursor: D-pad and left stick.
    same_in_battle("Right Right Up", "DpadRight DpadRight DpadUp");
    same_in_battle("Left Down", "LeftStickLeft LeftStickDown");
    same_in_battle("Right Up", "LeftStickRight LeftStickUp");
    // Confirm selects the lord; Cancel drops the selection again.
    same_in_battle("f", "South");
    same_in_battle("f Up d", "South DpadUp East");
    // Cancel with nothing to cancel opens the map menu.
    same_in_battle("d", "East");
    // Shoulders: next and previous ready unit.
    same_in_battle("s", "RightShoulder");
    same_in_battle("s s a", "RightShoulder RightShoulder LeftShoulder");
    // Top face: unit info. Left face: the danger zone.
    same_in_battle("e", "North");
    same_in_battle("w", "West");
    // Start: the end-turn prompt. Back / Select: auto-end.
    same_in_battle("Space", "Start");
    same_in_battle("Shift+Space", "Select");
    // Left trigger: rewind (after a move, so there is something to rewind).
    same_in_battle("f f f r", "South South South LeftTrigger");
}

#[test]
fn unused_buttons_do_nothing() {
    let mut h = quick_battle();
    h.pad("RightTrigger LeftStickPress RightStickPress");
    h.pad("RightStickUp RightStickDown RightStickLeft RightStickRight");
    // The same as pressing a key bound to nothing seven times (the cursor
    // pulses, so time has to pass in both).
    let mut idle = quick_battle();
    idle.keys("q q q q q q q");
    assert_eq!(h.snapshot(), idle.snapshot());
}

#[test]
fn start_twice_ends_the_turn() {
    let mut h = quick_battle();
    h.pad("Start");
    assert!(shows(&h, "End turn with 4 units ready?"));
    // Cancel backs out; Start pressed twice ends the turn.
    h.pad("East");
    assert!(!shows(&h, "units ready?"));
    h.pad("Start Start");
    assert!(shows(&h, "ENEMY PHASE"));
}

#[test]
fn a_held_dpad_moves_the_cursor_like_a_held_arrow() {
    let mut with_keys = quick_battle();
    let mut with_pad = quick_battle();
    let start = with_keys.snapshot();
    with_keys.hold("Right", 1.0);
    with_pad.hold_pad("DpadRight", 1.0);
    assert_eq!(with_pad.snapshot(), with_keys.snapshot());
    assert_ne!(with_pad.snapshot(), start);
    // Released, it stops.
    let stopped = with_pad.snapshot();
    with_pad.wait(1.0);
    assert_eq!(with_pad.snapshot(), stopped);
    // The left stick repeats the same way.
    let mut with_stick = quick_battle();
    with_stick.hold_pad("LeftStickRight", 1.0);
    assert_eq!(with_stick.snapshot(), stopped);
}

/// The lord walks next to the near brigand and fights it, pad only: the
/// same fight as `battle.rs`'s keyboard one.
#[test]
fn the_lord_fights_the_near_brigand_with_a_pad() {
    let mut with_keys = quick_battle();
    with_keys.keys("f Right Right Right Up f").wait(0.5);
    with_keys.keys("f f f d f").wait(0.5);
    let mut with_pad = quick_battle();
    with_pad
        .pad("South DpadRight DpadRight DpadRight DpadUp South")
        .wait(0.5);
    with_pad.pad("South South South East South").wait(0.5);
    assert_eq!(with_pad.snapshot(), with_keys.snapshot());
    assert_eq!(with_pad.sounds(), with_keys.sounds());
    assert!(!with_pad.sounds().is_empty());
}

#[test]
fn holding_confirm_on_the_pad_fast_forwards_like_the_key() {
    // Into the fight's playback, then hold Confirm through it.
    let mut with_keys = quick_battle();
    with_keys.keys("f Right Right Right Up f").wait(0.5);
    with_keys.keys("f f f").hold("f", 0.3);
    let mut with_pad = quick_battle();
    with_pad.keys("f Right Right Right Up f").wait(0.5);
    with_pad.keys("f f f").hold_pad("South", 0.3);
    assert_eq!(with_pad.snapshot(), with_keys.snapshot());
    // Held is faster than not held.
    let mut unheld = quick_battle();
    unheld.keys("f Right Right Right Up f").wait(0.5);
    unheld.keys("f f f").wait(0.3 + FRAME_DT);
    assert_ne!(with_pad.snapshot(), unheld.snapshot());
}

#[test]
fn keys_and_buttons_work_side_by_side() {
    let mut mixed = title();
    mixed.pad("DpadDown").keys("f");
    assert_eq!(mixed.screens(), ["title", "battle"]);
    // Select the lord, move, drop the selection, open the map menu.
    mixed.keys("f").pad("DpadUp").keys("d").pad("East");
    let mut keys_only = quick_battle();
    keys_only.keys("f Up d d");
    assert_eq!(mixed.snapshot(), keys_only.snapshot());
    assert!(shows(&mixed, "d back"));
}

#[test]
fn the_first_launch_picker_works_with_a_pad() {
    let mut h = Harness::new();
    assert_eq!(h.screens(), ["title", "layout_picker"]);
    // Cancel doesn't leave it, as with the keys.
    h.pad("East");
    assert_eq!(h.top_screen(), "layout_picker");
    h.pad("DpadDown South");
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(h.game().ctx().layout(), Some(Layout::LeftHanded));
    // The pad goes on working with the layout it picked.
    h.pad("South");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

/// On the web build the title waits for a first press (ticket 0224): any
/// controller button counts, and does nothing else.
#[test]
fn a_button_ends_the_web_titles_wait() {
    let mut h = Harness::on_web_with_layout(Layout::RightHanded);
    // The right trigger is bound to nothing: it still counts.
    h.pad("RightTrigger");
    assert_eq!(h.snapshot(), title().snapshot());
    let mut h = Harness::on_web_with_layout(Layout::RightHanded);
    h.pad("South");
    assert_eq!(h.screens(), ["title"]);
    assert!(h.sounds().is_empty());
    assert_eq!(h.snapshot(), title().snapshot());
    h.pad("South");
    assert_eq!(h.top_screen(), "mode_select");
}
