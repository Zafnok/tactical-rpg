//! Scripted tests of rebinding controller buttons through the real game
//! (ticket 0816, `docs/design/controls.md` *Rebinding buttons*): the Key
//! bindings screen's controller side, used with a controller only once it
//! is open (the debug menu's key opens it until the Options menu, 0805).
//!
//! Rows, top to bottom: the keyboard / controller switch, Cursor up, down,
//! left, right, Confirm, Cancel, End turn, Select, Confirm end turn,
//! Previous / Next ready unit, Unit info, …, Restore defaults.

use insta::assert_snapshot;
use trpg_ui::harness::Harness;
use trpg_ui::input::{Action, Button, Device, Layout, PadBindings, PadKind};
use trpg_ui::screen::KEYBINDINGS_KEY;
use trpg_ui::{MemoryStorage, Storage};

/// From the first row down to Confirm, and to Unit info.
const TO_CONFIRM: &str = "DpadDown DpadDown DpadDown DpadDown";
const TO_INFO: &str = "DpadDown DpadDown DpadDown DpadDown DpadDown DpadDown DpadDown \
                       DpadDown DpadDown DpadDown DpadDown";
/// From Unit info back up to Confirm.
const INFO_TO_CONFIRM: &str = "DpadUp DpadUp DpadUp DpadUp DpadUp DpadUp DpadUp";

/// The Key bindings screen, opened from the debug menu with a pad of
/// `kind`: on its controller side.
fn open(kind: PadKind) -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.use_pad(kind);
    reopen(&mut h);
    h
}

/// Opens the screen from the title with the default cursor buttons and the
/// button now on Confirm.
fn reopen(h: &mut Harness) {
    let confirm = h.game().ctx().keymap.primary_button(Action::Confirm);
    let confirm = confirm.map(Button::name).unwrap_or_default();
    h.keys("F2");
    h.pad(&format!("DpadDown DpadDown DpadDown DpadDown {confirm}"));
    assert_eq!(h.screens(), ["title", "debug_menu", "key_bindings"]);
}

fn buttons(h: &Harness) -> PadBindings {
    h.game().ctx().pad_bindings()
}

fn default_buttons(h: &Harness) -> PadBindings {
    PadBindings::defaults(&h.game().ctx().content.keymap)
}

/// `action`'s button slots as names, `-` for empty.
fn slots(h: &Harness, action: Action) -> [&'static str; 3] {
    buttons(h)
        .slots(action)
        .map(|b| b.map_or("-", Button::name))
}

/// The glyph row of the snapshot that shows `label`'s action.
fn row(h: &Harness, label: &str) -> String {
    let snap = h.snapshot();
    snap.lines()
        .take_while(|l| !l.starts_with("---"))
        .find(|l| l.contains(&format!(" {label}  ")))
        .unwrap_or_else(|| panic!("no row for {label}"))
        .to_owned()
}

/// Row `n` of the screen: 29 is the message under the panel, 31 the help.
fn line(h: &Harness, n: usize) -> String {
    let snap = h.snapshot();
    snap.lines().nth(n).unwrap_or("").trim().to_owned()
}

#[test]
fn the_controller_side_snapshot() {
    let h = open(PadKind::Xbox);
    assert!(row(&h, "Cursor up").contains("L-stick ↑"));
    assert_eq!(line(&h, 31), "D-pad/L-stick move · A change · B back");
    assert_snapshot!(h.snapshot());
}

#[test]
fn the_choices_and_the_capture_prompt_snapshots() {
    let mut h = open(PadKind::PlayStation);
    h.pad(TO_INFO).pad("DpadRight South");
    assert_snapshot!("choices_playstation", h.snapshot());
    h.pad("South");
    assert!(row(&h, "Unit info").contains("Press a button…"));
    assert_eq!(line(&h, 31), "Press a button… · hold any button to cancel");
    assert_snapshot!("capture_prompt_playstation", h.snapshot());
}

/// Acceptance: move Confirm's button to Unit info; Confirm shows `! not
/// mapped` and leaving is blocked.
#[test]
fn a_button_moved_off_confirm_blocks_leaving_until_it_has_one_again() {
    let mut h = open(PadKind::Xbox);
    // Unit info: Change, then tap Confirm's own button.
    h.pad(TO_INFO).pad("South South South");
    assert!(row(&h, "Confirm").contains("! not mapped"));
    assert!(row(&h, "Unit info").contains(" A "));
    assert_snapshot!(h.snapshot());
    // Nothing changes in the game until the screen closes.
    assert_eq!(buttons(&h), default_buttons(&h));
    h.pad("East");
    assert_eq!(h.top_screen(), "key_bindings");
    assert_eq!(line(&h, 29), "Give Confirm a button first");
    // The buttons the screen was opened with still steer it: give Confirm
    // the right trigger, and leaving works.
    h.pad(INFO_TO_CONFIRM).pad("South South RightTrigger");
    assert!(row(&h, "Confirm").contains(" RT "));
    assert_eq!(line(&h, 29), "");
    h.pad("East");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
    assert_eq!(slots(&h, Action::Confirm), ["RightTrigger", "-", "-"]);
    assert_eq!(slots(&h, Action::Info), ["South", "-", "-"]);
    // In the game the new buttons count: the old Confirm button is Unit
    // info now, and the right trigger confirms.
    h.pad("East South");
    assert_eq!(h.screens(), ["title"]);
    h.pad("RightTrigger");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

/// Acceptance: holding a button during `Press a button…` backs out with
/// nothing changed; a tap binds; `Clear` empties a slot.
#[test]
fn a_hold_backs_out_a_tap_binds_and_clear_empties() {
    let mut h = open(PadKind::Xbox);
    let before = h.snapshot();
    h.pad(TO_INFO).pad("DpadRight South South");
    assert!(row(&h, "Unit info").contains("Press a button…"));
    h.hold_pad("RightTrigger", 1.2);
    assert!(!row(&h, "Unit info").contains("Press a button…"));
    assert!(!row(&h, "Unit info").contains("RT"));
    h.pad("DpadLeft")
        .pad(INFO_TO_CONFIRM)
        .pad("DpadUp DpadUp DpadUp DpadUp");
    assert_eq!(h.snapshot(), before, "nothing changed");
    // A hold just short of the time is a tap.
    h.pad(TO_INFO).pad("DpadRight South South");
    h.hold_pad("RightTrigger", 0.8);
    assert!(row(&h, "Unit info").contains(" Y                 RT "));
    // Clear: the second line of the choices.
    h.pad("DpadLeft South DpadDown South");
    assert!(!row(&h, "Unit info").contains(" Y "));
    assert!(row(&h, "Unit info").contains(" RT "));
    h.pad("East East");
    assert_eq!(slots(&h, Action::Info), ["-", "RightTrigger", "-"]);
    assert_eq!(h.screens(), ["title"]);
}

/// Acceptance: the screen can be used start to finish with pad events
/// only, and a turn played with the new button.
#[test]
fn a_rebound_button_plays_a_battle() {
    // Next ready unit also on the right stick, pushed right.
    let mut h = open(PadKind::Xbox);
    h.pad(TO_CONFIRM).pad("South South RightTrigger East East");
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(slots(&h, Action::Confirm), ["RightTrigger", "-", "-"]);
    // Quick Battle with the new Confirm, and the same with the keyboard.
    h.pad("DpadDown RightTrigger RightTrigger");
    assert_eq!(h.screens(), ["title", "battle"]);
    let mut keys = Harness::with_layout(Layout::RightHanded);
    keys.keys("Down f f");
    assert_eq!(h.cursor_tile(), keys.cursor_tile());
    // Pick the unit under the cursor and move it one tile.
    h.pad("RightTrigger DpadRight RightTrigger");
    keys.keys("f Right f");
    assert_eq!(h.cursor_tile(), keys.cursor_tile());
    assert_eq!(
        h.snapshot_as(Device::Keyboard),
        keys.snapshot_as(Device::Keyboard)
    );
    // The old button does nothing any more.
    let before = h.snapshot();
    h.pad("South");
    assert_eq!(h.snapshot(), before);
    // The help bar names the new button.
    assert!(h.snapshot().contains("RT "), "{}", h.snapshot());
}

#[test]
fn a_stick_direction_can_be_bound_like_a_button() {
    let mut h = open(PadKind::Xbox);
    // Cursor up's third slot: the right stick pushed up.
    h.pad("DpadRight DpadRight South South RightStickUp");
    assert!(row(&h, "Cursor up").contains("R-stick ↑"));
    h.pad("East");
    assert_eq!(
        slots(&h, Action::CursorUp),
        ["DpadUp", "LeftStickUp", "RightStickUp"]
    );
    // It moves the debug menu's cursor up, from "Key bindings".
    let before = h.snapshot();
    h.pad("RightStickUp");
    assert_ne!(h.snapshot(), before);
    h.pad("DpadDown");
    assert_eq!(h.snapshot(), before);
}

/// Acceptance: switching right/left-handed leaves the buttons unchanged.
#[test]
fn switching_layout_leaves_the_buttons_unchanged() {
    let mut h = open(PadKind::Xbox);
    h.pad(TO_INFO)
        .pad("DpadRight South South RightTrigger East East");
    let edited = buttons(&h);
    assert_eq!(slots(&h, Action::Info), ["North", "RightTrigger", "-"]);
    for layout in [Layout::LeftHanded, Layout::RightHanded] {
        assert_eq!(h.ctx_mut().choose_layout(layout), Ok(()));
        assert_eq!(buttons(&h), edited, "{layout}");
        let keymap = &h.game().ctx().keymap;
        assert_eq!(keymap.pad_action(Button::RightTrigger), Some(Action::Info));
        // The keys are that layout's own defaults.
        let ctx = h.game().ctx();
        assert!(!ctx.player_keys().is_custom(layout));
    }
    // The screen shows the same buttons under the other layout.
    assert_eq!(h.ctx_mut().choose_layout(Layout::LeftHanded), Ok(()));
    reopen(&mut h);
    assert!(h.snapshot().contains("Keyboard · Left-handed"));
    assert!(row(&h, "Unit info").contains(" Y                 RT "));
}

/// Acceptance: edits persist across a restart.
#[test]
fn edited_buttons_survive_a_restart() {
    let mut h = open(PadKind::Xbox);
    h.pad(TO_CONFIRM)
        .pad("DpadRight South South RightTrigger East East");
    let edited = buttons(&h);
    assert_eq!(slots(&h, Action::Confirm), ["South", "RightTrigger", "-"]);
    let mut h = Harness::with_storage(h.into_storage());
    assert_eq!(buttons(&h), edited);
    h.pad("RightTrigger");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.pad("East");
    // The screen shows the saved buttons when opened again, and Restore
    // defaults (one down from the last row) puts the defaults back.
    reopen(&mut h);
    assert!(row(&h, "Confirm").contains(" A                 RT "));
    h.pad("DpadUp DpadUp South");
    assert!(h.snapshot().contains("Restore the default buttons?"));
    h.pad("South East East");
    assert_eq!(buttons(&h), default_buttons(&h));
    let h = Harness::with_storage(h.into_storage());
    assert_eq!(buttons(&h), default_buttons(&h));
}

/// Acceptance: an old `version: 1` config loads with the default buttons.
#[test]
fn a_version_1_config_keeps_its_keys_and_gets_the_default_buttons() {
    let old = "PlayerKeys(version: 1, layouts: {\"RightHanded\": {\"Info\": [Some(\"g\")]}})";
    let mut storage = MemoryStorage::new();
    storage.write("layout", "RightHanded").ok();
    storage.write(KEYBINDINGS_KEY, old).ok();
    let mut h = Harness::with_storage(Box::new(storage));
    let ctx = h.game().ctx();
    assert_eq!(buttons(&h), default_buttons(&h));
    assert!(ctx.player_keys().is_custom(Layout::RightHanded));
    let info = ctx.keymap.primary(Action::Info).map(|c| c.to_string());
    assert_eq!(info.as_deref(), Some("g"));
    // The default buttons work.
    h.pad("South");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

/// With the keyboard, the controller side is one move away, and the
/// keyboard's own keys clear a slot and back out of a capture.
#[test]
fn the_keyboard_can_edit_the_buttons_from_the_switch_row() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F2 Down Down Down Down f");
    assert!(h.snapshot().contains("Key 1"));
    h.keys("Up Right");
    assert!(h.snapshot().contains("Button 1"));
    assert_eq!(
        line(&h, 31),
        "Left/Right keyboard or controller · arrows move · d back"
    );
    assert_snapshot!(h.snapshot());
    // Confirm goes straight to the prompt; Escape backs out.
    h.keys("Down f");
    assert!(row(&h, "Cursor up").contains("Press a button…"));
    h.keys("Escape");
    assert!(!row(&h, "Cursor up").contains("Press a button…"));
    // Delete empties a button slot; a button tapped at the prompt goes in.
    h.keys("Right Delete");
    assert!(!row(&h, "Cursor up").contains("L-stick"));
    h.keys("f").pad("RightStickUp");
    // The pad was used last now: the buttons are named as it names them.
    assert!(row(&h, "Cursor up").contains("R-stick ↑"));
    h.keys("d");
    assert_eq!(slots(&h, Action::CursorUp), ["DpadUp", "RightStickUp", "-"]);
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = trpg_content::FontAtlasDef::load().unwrap_or_default();
    for kind in [PadKind::Xbox, PadKind::PlayStation, PadKind::Nintendo] {
        let mut h = open(kind);
        h.pad("South");
        let snap = h.snapshot();
        let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
        for g in glyphs.chars().filter(|&c| c != '\n') {
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
    }
}
