//! The "Scene camera" debug tool through the Harness (ticket 0228,
//! ADR-0048): the test map is a backdrop behind a window of the console,
//! and a snapshot pins both the scene and where the window is.

use insta::assert_snapshot;
use trpg_ui::harness::Harness;
use trpg_ui::input::{Layout, PadKind};

/// The title with the scene camera opened from the debug menu (its last
/// tool but one: only the map skin comes after it).
fn opened() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F2 Up Up f");
    assert_eq!(h.screens(), ["title", "debug_menu", "scene_camera"]);
    h
}

/// The snapshot's backdrop header line.
fn header(snapshot: &str) -> &str {
    snapshot
        .lines()
        .find(|l| l.starts_with("--- backdrop: "))
        .unwrap_or("no backdrop")
}

#[test]
fn each_whole_zoom_is_a_header_line_and_the_scene() {
    let mut h = opened();
    let one = h.snapshot();
    assert_eq!(
        header(&one),
        "--- backdrop: clip 26,4 48x14  origin -80.0,-48.0  zoom 1 ---"
    );
    // After the header comes the scene's own snapshot: the whole map.
    let scene = h.game().buffer().backdrop().unwrap().scene();
    assert_eq!((scene.width(), scene.height()), (28, 8));
    assert!(one.ends_with(&scene.to_snapshot(&h.game().ctx().palette)));
    assert_snapshot!("scene_camera_1x", one);
    h.keys("f");
    let two = h.snapshot();
    assert_eq!(
        header(&two),
        "--- backdrop: clip 26,4 48x14  origin 16.0,8.0  zoom 2 ---"
    );
    assert_snapshot!("scene_camera_2x", two);
    h.keys("f");
    let three = h.snapshot();
    assert_eq!(
        header(&three),
        "--- backdrop: clip 26,4 48x14  origin 48.0,26.7  zoom 3 ---"
    );
    assert_snapshot!("scene_camera_3x", three);
    h.keys("f");
    let four = h.snapshot();
    assert_eq!(
        header(&four),
        "--- backdrop: clip 26,4 48x14  origin 64.0,36.0  zoom 4 ---"
    );
    assert_snapshot!("scene_camera_4x", four);
    // Round again: no step past 4×, none in between.
    h.keys("f");
    assert_eq!(h.snapshot(), one);
    assert_eq!(h.sounds().iter().filter(|s| *s == "menu_select").count(), 5);
}

#[test]
fn holding_a_cursor_key_pans_by_the_pixel() {
    let mut h = opened();
    h.keys("f f f");
    // 60 scene pixels a second, at 60 frames a second: a pixel a frame.
    h.hold("Right", 0.5);
    let origin = h.game().buffer().backdrop().unwrap().origin_px();
    assert!((origin.0 - 94.0).abs() < 0.05, "{origin:?}");
    assert!((origin.1 - 36.0).abs() < 0.05, "{origin:?}");
    assert_eq!(
        header(&h.snapshot()),
        "--- backdrop: clip 26,4 48x14  origin 94.0,36.0  zoom 4 ---"
    );
    // Part of a pixel: a quarter of a second up is 15 px; one frame is 1.
    h.hold("Up", 1.0 / 240.0);
    let origin = h.game().buffer().backdrop().unwrap().origin_px();
    assert!((origin.1 - 35.75).abs() < 0.05, "{origin:?}");
    // The window stops at the scene's edge.
    h.hold("Left", 5.0).hold("Down", 5.0);
    assert_eq!(
        header(&h.snapshot()),
        "--- backdrop: clip 26,4 48x14  origin 0.0,72.0  zoom 4 ---"
    );
    // The same keys, the same frame.
    let mut again = opened();
    again.keys("f f f");
    again.hold("Right", 0.5).hold("Up", 1.0 / 240.0);
    again.hold("Left", 5.0).hold("Down", 5.0);
    assert_eq!(again.snapshot(), h.snapshot());
}

#[test]
fn the_window_shows_the_scene_and_the_box_over_it_hides_it() {
    let h = opened();
    let buf = h.game().buffer();
    let clip = buf.backdrop().unwrap().clip();
    // Inside the window: see-through, but for the box's corner.
    assert!(buf.get(clip.x, clip.y).unwrap().see_through);
    assert!(buf.get(clip.x + clip.w - 1, clip.y).unwrap().see_through);
    let corner = buf.get(clip.x + clip.w - 1, clip.y + clip.h - 1).unwrap();
    assert!(!corner.see_through);
    // Outside it: nothing is.
    assert!(!buf.get(clip.x - 1, clip.y).unwrap().see_through);
    assert!(!buf.get(clip.x, clip.y + clip.h).unwrap().see_through);
    let snapshot = h.snapshot();
    assert!(snapshot.contains("bg:see-through"), "{snapshot}");
    assert!(snapshot.contains("A text box drawn"), "{snapshot}");
}

#[test]
fn cancel_closes_it_and_the_next_frame_has_no_backdrop() {
    let mut h = opened();
    // The debug key does nothing over a debug tool.
    h.keys("F2");
    assert_eq!(h.top_screen(), "scene_camera");
    h.clear_audio().keys("d");
    assert_eq!(h.top_screen(), "debug_menu");
    assert_eq!(h.sounds(), ["menu_cancel"]);
    assert!(h.game().buffer().backdrop().is_none());
    assert!(!h.snapshot().contains("backdrop"));
}

#[test]
fn a_sprite_skinned_map_is_a_scene_of_sprites() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.with_map_skin("sprite").keys("F2 Up Up f");
    let scene = h.game().buffer().backdrop().unwrap().scene();
    // 14 × 8 tiles of 24 px.
    assert_eq!((scene.width(), scene.height()), (42, 12));
    assert!(scene.sprites().len() >= 14 * 8);
    // The console's own items stay its own: the scene's aren't copied in.
    assert!(h.game().buffer().items().is_empty());
    assert!(h.snapshot().contains(" sprite "));
}

#[test]
fn the_help_line_names_the_keys_or_the_buttons() {
    let mut h = opened();
    let keys = h.snapshot();
    assert!(keys.contains("arrows pan (hold)"), "{keys}");
    assert!(keys.contains("f zoom"), "{keys}");
    assert!(keys.contains("d back"), "{keys}");
    h.use_pad(PadKind::Xbox).pad("South");
    let pad = h.snapshot();
    assert!(pad.contains("D-pad/L-stick pan (hold)"), "{pad}");
    assert!(pad.contains("A zoom"), "{pad}");
    assert!(pad.contains("B back"), "{pad}");
    assert_eq!(h.game().buffer().backdrop().unwrap().zoom(), 2);
    h.hold_pad("DpadRight", 0.25);
    let origin = h.game().buffer().backdrop().unwrap().origin_px();
    assert!((origin.0 - 31.0).abs() < 0.05, "{origin:?}");
    h.pad("East");
    assert_eq!(h.top_screen(), "debug_menu");
}
