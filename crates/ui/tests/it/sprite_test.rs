//! The "Sprite test" debug tool through the Harness (ticket 0231,
//! ADR-0038): the frame holds pictures as sprite items, and a snapshot
//! shows each as one line.

use insta::assert_snapshot;
use trpg_content::image::TEST_CARD_PATH;
use trpg_ui::Rect;
use trpg_ui::harness::Harness;
use trpg_ui::input::{Layout, PadKind};

/// The title with the sprite test opened from the debug menu (its last
/// tool but five: the class change tools, the voice test, the scene camera
/// and the map skin come after it).
fn opened() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F2 Up Up Up Up Up Up f");
    assert_eq!(h.screens(), ["title", "debug_menu", "sprite_test"]);
    h
}

#[test]
fn the_debug_menu_opens_the_sprite_test_and_cancel_closes_it() {
    let mut h = opened();
    let sprites = h.game().buffer().sprites();
    assert_eq!(sprites.len(), 9);
    assert!(sprites.iter().all(|s| s.image.path() == TEST_CARD_PATH));
    // The debug key does nothing over a debug tool.
    h.keys("F2");
    assert_eq!(h.top_screen(), "sprite_test");
    h.clear_audio().keys("d");
    assert_eq!(h.top_screen(), "debug_menu");
    assert_eq!(h.sounds(), ["menu_cancel"]);
    // The menu is drawn without the tool's pictures.
    assert!(h.game().buffer().sprites().is_empty());
}

#[test]
fn every_sprite_is_one_snapshot_line() {
    let h = opened();
    let snapshot = h.snapshot();
    let lines: Vec<&str> = snapshot
        .lines()
        .skip_while(|&l| l != "--- overlays ---")
        .skip(1)
        .collect();
    let items = h.game().buffer().items();
    assert_eq!(lines.len(), items.len());
    assert!(lines.iter().all(|l| l.contains(" sprite ")), "{lines:?}");
    // The picture half under the text box is listed by what is left of it.
    let cut: Vec<Rect> = h
        .game()
        .buffer()
        .sprites()
        .iter()
        .filter(|s| s.clip != s.dest)
        .map(|s| s.clip)
        .collect();
    assert_eq!(cut.len(), 2);
    assert!(lines[7].ends_with(" clip=176,208 80x32"), "{}", lines[7]);
    assert!(lines[8].ends_with(" clip=176,240 40x48"), "{}", lines[8]);
    assert_snapshot!(snapshot);
}

#[test]
fn the_help_line_names_the_button_after_a_controller_press() {
    let mut h = opened();
    let keys = h.snapshot();
    assert!(keys.contains("d back"), "{keys}");
    h.use_pad(PadKind::Xbox).pad("DpadDown");
    let pad = h.snapshot();
    assert!(pad.contains("B back"), "{pad}");
    h.pad("East");
    assert_eq!(h.top_screen(), "debug_menu");
}
