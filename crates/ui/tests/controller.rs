//! Scripted tests of playing with a controller through the real game
//! (ADR-0007 layer 4; `docs/design/controls.md`, *Controller*; ticket
//! 0219). A controller gives the same actions as the keys, so most tests
//! play the same thing twice, once with keys and once with buttons, and
//! compare the screens. Help bars and tips name whatever was pressed last
//! (ticket 0220), so the pad's screen is compared as the keyboard would
//! show it ([`as_keys`]); the button names have their own tests below.

use insta::assert_snapshot;
use trpg_ui::harness::{FRAME_DT, Harness};
use trpg_ui::input::{Action, Chord, Device, Layout, PadKind};

fn title() -> Harness {
    Harness::with_layout(Layout::RightHanded)
}

/// At the title with the right-handed layout, then Quick Battle, chosen
/// with the keyboard, and its `PLAYER PHASE` banner closed.
fn quick_battle() -> Harness {
    let mut h = title();
    h.keys("Down f f");
    h
}

/// The screen as it would look with the keyboard pressed last: the same
/// screen, naming keys.
fn as_keys(h: &mut Harness) -> String {
    h.snapshot_as(Device::Keyboard)
}

/// The battle's two bottom rows: the toggles' state and the help bar
/// (without the debug-menu hint at its right).
fn help_rows(h: &Harness) -> String {
    let buf = h.game().buffer();
    let row = |y| {
        let row: String = (0..100)
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect();
        row.trim().trim_end_matches("F2 debug").trim().to_owned()
    };
    format!("{}\n{}", row(30), row(31))
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
    assert_eq!(as_keys(&mut with_pad), with_keys.snapshot(), "{buttons}");
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
    h.pad("LeftStickDown LeftStickDown LeftStickDown South");
    assert!(h.quit_requested());
}

#[test]
fn quick_battle_starts_the_same_from_a_pad() {
    let mut h = title();
    h.pad("DpadDown South South");
    assert_eq!(as_keys(&mut h), quick_battle().snapshot());
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
    // Buttons that do nothing don't switch the help to button names.
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
    assert_eq!(as_keys(&mut with_pad), with_keys.snapshot());
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
    assert_eq!(as_keys(&mut with_pad), with_keys.snapshot());
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
    assert_eq!(as_keys(&mut with_pad), with_keys.snapshot());
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
    // The `PLAYER PHASE` banner.
    mixed.pad("South");
    // Select the lord, move, drop the selection, open the map menu.
    mixed.keys("f").pad("DpadUp").keys("d").pad("East");
    let mut keys_only = quick_battle();
    keys_only.keys("f Up d d");
    assert_eq!(as_keys(&mut mixed), keys_only.snapshot());
    // The pad was pressed last, so the map menu's help names its buttons.
    assert!(shows(&mixed, "B back"));
    assert!(!shows(&mixed, "d back"));
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
    assert_eq!(as_keys(&mut h), title().snapshot());
    // The title's help names the pad's buttons from then on.
    assert!(shows(&h, "A select"));
    h.pad("South");
    assert_eq!(h.top_screen(), "mode_select");
}

/// Ticket 0220 (`docs/design/controls.md`, *Switching between keyboard and
/// controller*): help text follows whatever was pressed last.
#[test]
fn the_battle_help_bar_names_buttons_after_a_pad_press_and_keys_after_a_key() {
    let mut h = quick_battle();
    assert!(shows(&h, "f select"));
    // A button: the Confirm button's name, and the rest of the bar.
    h.pad("DpadRight DpadLeft");
    assert!(shows(&h, "A select"));
    assert!(!shows(&h, "f select"));
    assert_eq!(
        help_rows(&h),
        "X danger zone: OFF · Back auto-end: OFF\n\
         A select · Y info · RB next unit · LT rewind · B menu · Start end turn"
    );
    // A key: the keys again.
    h.keys("Right Left");
    assert!(shows(&h, "f select"));
    assert_eq!(
        help_rows(&h),
        "w danger zone: OFF · Shift+Space auto-end: OFF\n\
         f select · e info · s next unit · r rewind · d menu · Space end turn"
    );
    // And back, in the frame of the press itself.
    h.hold_pad("DpadUp", 0.0);
    assert!(shows(&h, "D-pad/stick move"));
}

/// The names follow the pad in use: Xbox letters, Sony's shapes,
/// Nintendo's letters (its right button, `A`, confirms).
#[test]
fn the_help_bar_names_the_buttons_of_the_pad_in_use() {
    let rows = |kind| {
        let mut h = quick_battle();
        h.use_pad(kind).pad("DpadRight DpadLeft");
        help_rows(&h)
    };
    assert_eq!(rows(PadKind::Generic), rows(PadKind::Xbox));
    assert_snapshot!("help_bar_keyboard", help_rows(&quick_battle()));
    assert_snapshot!("help_bar_xbox", rows(PadKind::Xbox));
    assert_snapshot!("help_bar_playstation", rows(PadKind::PlayStation));
    assert_snapshot!("help_bar_nintendo", rows(PadKind::Nintendo));
}

/// Ticket 0229: a PS4 pad's left centre button is `Share`, where a PS5
/// pad's is `Create`.
#[test]
fn a_playstation_4_pad_names_auto_end_share() {
    let rows = |kind| {
        let mut h = quick_battle();
        h.use_pad(kind).pad("DpadRight DpadLeft");
        help_rows(&h)
    };
    let ps4 = rows(PadKind::PlayStation4);
    assert!(ps4.contains("Share auto-end: OFF"), "{ps4}");
    assert_eq!(ps4.replace("Share", "Create"), rows(PadKind::PlayStation));
}

/// Two pads of different kinds: the bar names the one pressed last.
#[test]
fn the_help_bar_follows_the_pad_pressed_last() {
    let mut h = quick_battle();
    h.use_pad(PadKind::PlayStation).pad("DpadRight");
    assert!(shows(&h, "R1 next unit"));
    assert!(shows(&h, "Options end turn"));
    h.use_pad(PadKind::Nintendo).pad("DpadLeft");
    assert!(shows(&h, "X info"));
    assert!(shows(&h, "+ end turn"));
}

/// The battle's first tip (`{Cursor}`, `{Select}`, `{NextUnit}`, `{Info}`
/// and its `{Confirm} close`) on the keyboard and on each kind of pad.
#[test]
fn a_tip_names_the_buttons_of_the_pad_in_use() {
    let tip = |kind: Option<PadKind>| {
        let mut h = title();
        h.with_tips();
        match kind {
            Some(kind) => h.use_pad(kind).pad("DpadDown South South"),
            None => h.keys("Down f f"),
        };
        assert_eq!(h.screens(), ["title", "battle"]);
        h.wait(0.5);
        h
    };
    let keys = tip(None);
    assert!(shows(&keys, "Press f on one of"));
    assert!(shows(&keys, "f close"));
    let xbox = tip(Some(PadKind::Xbox));
    assert!(shows(
        &xbox,
        "Steer the cursor with D-pad/stick. Press A on one of"
    ));
    assert!(shows(&xbox, "A close"));
    let sony = tip(Some(PadKind::PlayStation));
    assert!(shows(&sony, "Press ✕ on one of"));
    assert!(shows(&sony, "✕ close"));
    assert_snapshot!("tip_keyboard", keys.snapshot());
    assert_snapshot!("tip_xbox", xbox.snapshot());
    assert_snapshot!("tip_playstation", sony.snapshot());
    assert_snapshot!("tip_nintendo", tip(Some(PadKind::Nintendo)).snapshot());
    // Pressing a key while the tip is up switches it back to keys.
    let mut h = tip(Some(PadKind::PlayStation));
    h.keys("q");
    assert!(shows(&h, "✕ close"), "an unbound key doesn't count");
    h.keys("Up");
    assert!(shows(&h, "Press f on one of"));
    assert!(shows(&h, "f close"));
}

/// An action with no button shows `! not mapped` on a pad: here Select,
/// given a key of its own, so Confirm no longer picks on the map.
#[test]
fn an_action_with_no_button_shows_not_mapped_on_a_pad() {
    let mut h = quick_battle();
    let mut keys = h.game().ctx().layout_bindings(Layout::RightHanded);
    let g = Chord::parse("g").unwrap();
    assert_eq!(keys.bind(Action::Select, 0, g), Ok(None));
    h.ctx_mut()
        .set_layout_bindings(Layout::RightHanded, keys)
        .unwrap();
    h.keys("Right Left");
    assert!(shows(&h, "g select"));
    h.pad("DpadRight DpadLeft");
    assert!(shows(&h, "! not mapped select"));
    assert!(!shows(&h, "A select"));
}

/// The longest battle help bar (an enemy's range shown, the cursor on an
/// enemy) fits the screen on every kind of pad.
#[test]
fn the_longest_help_bar_fits_on_every_pad() {
    for kind in [
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::Nintendo,
        PadKind::Generic,
    ] {
        let mut h = quick_battle();
        h.use_pad(kind);
        // One move, so there is something to rewind; then sweep the map
        // for an enemy and show its range.
        h.pad("South South South").wait(0.5);
        let mut sweep = ["DpadRight", "DpadLeft"].into_iter().cycle();
        'rows: for _ in 0..30 {
            let along = sweep.next().unwrap_or_default();
            for step in 0..40 {
                if shows(&h, " range ·") {
                    break 'rows;
                }
                h.pad(if step < 39 { along } else { "DpadUp" });
            }
        }
        h.pad("South");
        let rows = help_rows(&h);
        let help = rows.lines().nth(1).unwrap_or_default();
        assert!(help.contains("hide range"), "{kind:?}: {help}");
        assert!(help.contains("rewind"), "{kind:?}: {help}");
        assert!(help.ends_with("end turn"), "{kind:?}: {help}");
        assert!(help.chars().count() < 100, "{kind:?}: {help}");
    }
}
