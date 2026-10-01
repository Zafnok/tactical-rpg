//! Scripted tests of the Key bindings screen through the real game (ticket
//! 0815, `docs/design/controls.md` *Rebinding keys*), opened from the debug
//! menu until the Options menu (0805) exists.
//!
//! Rows, top to bottom: Cursor up, down, left, right, Confirm, Cancel, End
//! turn, Select, Confirm end turn, Previous / Next ready unit, Unit info, …,
//! Restore defaults (one `Up` from the first row).

use insta::assert_snapshot;
use trpg_ui::harness::Harness;
use trpg_ui::input::{Action, Chord, Layout, LayoutBindings};

/// From the first row down to Confirm, and to Unit info.
const TO_CONFIRM: &str = "Down Down Down Down";
const TO_INFO: &str = "Down Down Down Down Down Down Down Down Down Down Down";

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{e}"))
}

/// A later launch with `layout`, on the Key bindings screen.
fn open(layout: Layout) -> Harness {
    let mut h = Harness::with_layout(layout);
    reopen(&mut h);
    h
}

/// Opens the screen from the title, with `h`'s current keys.
fn reopen(h: &mut Harness) {
    let km = h.game().ctx().keymap.clone();
    let key = |a| km.primary(a).map(|c| c.to_string()).unwrap_or_default();
    let (down, confirm) = (key(Action::CursorDown), key(Action::Confirm));
    h.keys(&format!("F2 {down} {down} {down} {down} {confirm}"));
    assert_eq!(h.screens(), ["title", "debug_menu", "key_bindings"]);
}

fn bindings(h: &Harness, layout: Layout) -> LayoutBindings {
    h.game().ctx().layout_bindings(layout)
}

fn defaults(h: &Harness, layout: Layout) -> LayoutBindings {
    LayoutBindings::defaults(&h.game().ctx().content.keymap, layout)
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

/// The message row under the panel.
fn message(h: &Harness) -> String {
    let snap = h.snapshot();
    snap.lines().nth(29).unwrap_or("").trim().to_owned()
}

#[test]
fn the_list_snapshot() {
    let h = open(Layout::RightHanded);
    assert_snapshot!(h.snapshot());
}

#[test]
fn the_left_handed_list_shows_that_layouts_keys() {
    let h = open(Layout::LeftHanded);
    assert!(h.snapshot().contains(" Key bindings · Left-handed "));
    assert!(row(&h, "Confirm").contains(" j "));
    assert!(row(&h, "Cursor up").contains(" w "));
}

#[test]
fn the_capture_prompt_snapshot() {
    let mut h = open(Layout::RightHanded);
    h.keys(TO_CONFIRM).keys("Right f");
    assert!(row(&h, "Confirm").contains("Press a key…"));
    assert_snapshot!(h.snapshot());
}

/// Acceptance: binding `e` to Confirm's slot 2 moves it from Unit info.
#[test]
fn a_key_bound_twice_moves_and_the_loser_shows_not_mapped() {
    let mut h = open(Layout::RightHanded);
    h.keys(TO_CONFIRM).keys("Right f e");
    assert!(row(&h, "Unit info").contains("! not mapped"));
    assert!(row(&h, "Confirm").contains(" f "));
    assert!(row(&h, "Confirm").contains(" e "));
    assert_snapshot!(h.snapshot());
    // Nothing changes in the game until the screen closes.
    assert_eq!(h.game().ctx().keymap.action(chord("e")), Some(Action::Info));
    h.keys("d");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
    h.keys("d");
    let keymap = &h.game().ctx().keymap;
    assert_eq!(keymap.action(chord("e")), Some(Action::Confirm));
    assert_eq!(keymap.primary(Action::Info), None);
    // `e` confirms at the title.
    h.keys("e");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

/// Acceptance: with Confirm's only key moved away, Cancel doesn't leave.
#[test]
fn leaving_is_blocked_until_confirm_has_a_key_again() {
    let mut h = open(Layout::RightHanded);
    // Unit info takes `f`, Confirm's only key.
    h.keys(TO_INFO).keys("f f");
    assert!(row(&h, "Confirm").contains("! not mapped"));
    h.keys("d");
    assert_eq!(h.top_screen(), "key_bindings");
    assert_eq!(message(&h), "Give Confirm a key first");
    assert_snapshot!(h.snapshot());
    // Escape is Cancel too, and is blocked the same way.
    h.keys("Escape");
    assert_eq!(h.top_screen(), "key_bindings");
    // `f` still confirms here (the keys the screen was opened with): give
    // Confirm `g`, and leaving works.
    h.keys("Up Up Up Up Up Up Up f g");
    assert!(row(&h, "Confirm").contains(" g "));
    assert_eq!(message(&h), "");
    h.keys("d");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
    h.keys("f");
    assert_eq!(h.screens(), ["title", "debug_menu"], "f is Unit info now");
    // The debug menu is still on "Key bindings": `g` confirms it.
    h.keys("g");
    assert_eq!(h.top_screen(), "key_bindings");
}

/// Acceptance: Esc in capture changes nothing, Delete in capture is
/// ignored, Delete on a slot empties it.
#[test]
fn escape_backs_out_of_capture_and_delete_empties_a_slot() {
    let mut h = open(Layout::RightHanded);
    let before = h.snapshot();
    h.keys(TO_CONFIRM).keys("f");
    assert!(row(&h, "Confirm").contains("Press a key…"));
    h.keys("Delete");
    assert!(row(&h, "Confirm").contains("Press a key…"));
    h.keys("Escape");
    assert_eq!(h.top_screen(), "key_bindings");
    assert!(row(&h, "Confirm").contains(" f "));
    h.keys("Up Up Up Up");
    assert_eq!(h.snapshot(), before);
    // Delete on a slot.
    h.keys(TO_INFO);
    assert!(row(&h, "Unit info").contains(" e "));
    h.keys("Delete");
    assert!(row(&h, "Unit info").contains("! not mapped"));
    h.keys("d d");
    assert_eq!(h.game().ctx().keymap.primary(Action::Info), None);
    assert_eq!(h.game().ctx().keymap.action(chord("Delete")), None);
}

/// The Debug key can't be bound in a build with debug tools, and doesn't
/// open the debug menu over this screen.
#[test]
fn the_debug_key_is_refused_while_capturing() {
    let mut h = open(Layout::RightHanded);
    h.keys("F2");
    assert_eq!(h.top_screen(), "key_bindings");
    h.keys("f F2");
    assert_eq!(h.top_screen(), "key_bindings");
    if trpg_ui::screen::DEBUG_TOOLS {
        assert_eq!(message(&h), "That key can't be used");
        assert!(row(&h, "Cursor up").contains("Press a key…"));
    }
}

/// Acceptance: the keys the screen was opened with steer it, even after
/// every cursor key has been moved to other actions.
#[test]
fn navigation_survives_moving_every_cursor_key_away() {
    let mut h = open(Layout::RightHanded);
    // Unit info takes Up, Down and Left; Rewind (three rows on) takes Right.
    h.keys(TO_INFO).keys("f Up Right f Down Right f Left");
    h.keys("Down Down Down f Right");
    for cursor in ["Cursor up", "Cursor down", "Cursor left", "Cursor right"] {
        assert!(row(&h, cursor).contains("! not mapped"), "{cursor}");
    }
    h.keys("d");
    assert_eq!(message(&h), "Give Cursor up a key first");
    // The arrows still move: back up to the cursor rows to give them
    // `i` `k` `j` `l`. Rewind is 14 rows below Cursor up.
    h.keys("Up Up Up Up Up Up Up Up Up Up Up Up Up Up");
    h.keys("f i Down f k Down f j Down f l");
    h.keys("d");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
    let keymap = &h.game().ctx().keymap;
    assert_eq!(keymap.primary(Action::CursorUp), Some(chord("i")));
    assert_eq!(keymap.primary(Action::CursorRight), Some(chord("l")));
    assert_eq!(keymap.action(chord("Up")), Some(Action::Info));
    // In the game the new keys steer: `k` is Cursor down in the debug menu
    // (from "Key bindings", past the two class change tools after it, round
    // to the first), and the arrows no longer move there.
    h.keys("Down k k k f");
    assert_eq!(h.top_screen(), "glyph_sampler");
}

/// Acceptance: edits persist across a restart, per layout.
#[test]
fn edits_survive_a_restart_and_stay_with_their_layout() {
    let mut h = open(Layout::RightHanded);
    h.keys(TO_CONFIRM).keys("Right f g d d");
    let edited = bindings(&h, Layout::RightHanded);
    assert_eq!(
        edited.slots(Action::Confirm),
        [Some(chord("f")), Some(chord("g")), None]
    );
    let mut h = Harness::with_storage(h.into_storage());
    assert_eq!(bindings(&h, Layout::RightHanded), edited);
    assert_eq!(
        bindings(&h, Layout::LeftHanded),
        defaults(&h, Layout::LeftHanded)
    );
    h.keys("g");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("d");
    // The screen shows the saved keys when opened again.
    reopen(&mut h);
    assert!(row(&h, "Confirm").contains(" g "));
}

/// Acceptance: Restore defaults resets only the layout in use.
#[test]
fn restore_defaults_leaves_the_other_layout_alone() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    let mut left = bindings(&h, Layout::LeftHanded);
    assert_eq!(left.bind(Action::Confirm, 1, chord("h")), Ok(None));
    assert_eq!(
        h.ctx_mut()
            .set_layout_bindings(Layout::LeftHanded, left.clone()),
        Ok(())
    );
    reopen(&mut h);
    h.keys(TO_CONFIRM).keys("Right f g d d");
    assert_ne!(
        bindings(&h, Layout::RightHanded),
        defaults(&h, Layout::RightHanded)
    );
    reopen(&mut h);
    // One Up from the first row is Restore defaults. It asks first, and
    // Cancel there answers no without leaving the screen.
    h.keys("Up f");
    assert!(
        h.snapshot()
            .contains("Restore the default keys for Right-handed?")
    );
    assert_snapshot!(h.snapshot());
    h.keys("d");
    assert_eq!(h.top_screen(), "key_bindings");
    assert!(row(&h, "Confirm").contains(" g "));
    h.keys("f f");
    assert!(!row(&h, "Confirm").contains(" g "));
    h.keys("d d");
    assert_eq!(
        bindings(&h, Layout::RightHanded),
        defaults(&h, Layout::RightHanded)
    );
    assert_eq!(bindings(&h, Layout::LeftHanded), left);
    // And across a restart.
    let h = Harness::with_storage(h.into_storage());
    assert_eq!(
        bindings(&h, Layout::RightHanded),
        defaults(&h, Layout::RightHanded)
    );
    assert_eq!(bindings(&h, Layout::LeftHanded), left);
}

/// Sign-off: Select and Confirm end turn get their own keys.
#[test]
fn the_optional_split_keys_can_be_given_keys() {
    let mut h = open(Layout::RightHanded);
    // Select is 7 rows down, Confirm end turn the next.
    h.keys("Down Down Down Down Down Down Down f g Down f Enter d d");
    let keymap = &h.game().ctx().keymap;
    assert_eq!(keymap.select_action(), Action::Select);
    assert_eq!(keymap.action(chord("g")), Some(Action::Select));
    assert_eq!(
        keymap.end_turn_accept_actions(),
        [Action::ConfirmEndTurn, Action::Confirm]
    );
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = trpg_content::FontAtlasDef::load().unwrap_or_default();
    let mut h = open(Layout::RightHanded);
    h.keys("f");
    let snap = h.snapshot();
    let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
    for g in glyphs.chars().filter(|&c| c != '\n') {
        assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
    }
}
