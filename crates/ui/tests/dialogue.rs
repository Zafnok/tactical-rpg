//! Scripted tests of the dialogue screen through the real game (ADR-0007
//! layer 4): the test scene (`assets/dialogue/test.dlg`) opened from the
//! debug menu, full-screen and over the battle map.

use insta::assert_snapshot;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// Text boxes in the test scene.
const TEST_BOXES: usize = 7;

/// At the title, then the debug menu's "Play test scene".
fn full_screen() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F12 Down Down f");
    assert_eq!(h.screens(), ["title", "debug_menu", "dialogue"]);
    h
}

/// In Quick Battle, then the debug menu's "Play test scene (overlay)".
fn over_the_map() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down f F12 Down Down Down f");
    assert_eq!(h.screens(), ["title", "battle", "dialogue"]);
    h
}

/// Presses `f` until the dialogue closes; returns how many presses it took.
fn play_to_the_end(h: &mut Harness) -> usize {
    for presses in 1..=100 {
        h.keys("f");
        if h.top_screen() != "dialogue" {
            return presses;
        }
    }
    panic!("the scene never ended");
}

/// The text of row `y`, trimmed.
fn row(h: &Harness, y: i32) -> String {
    let buf = h.game().buffer();
    (0..100)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

#[test]
fn the_full_screen_scene_plays_to_the_end() {
    let mut h = full_screen();
    // One press reveals each box, the next moves on.
    assert_eq!(play_to_the_end(&mut h), 2 * TEST_BOXES);
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn the_overlay_scene_plays_to_the_end() {
    let mut h = over_the_map();
    assert_eq!(play_to_the_end(&mut h), 2 * TEST_BOXES);
    assert_eq!(h.screens(), ["title", "battle"]);
}

#[test]
fn waiting_reveals_the_text_so_one_press_moves_on() {
    let mut h = full_screen();
    for _ in 0..TEST_BOXES {
        h.wait(2.0).keys("f");
    }
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn skipping_asks_first() {
    let mut h = full_screen();
    h.keys("d");
    assert!(row(&h, 23).starts_with("│   Skip scene?  "));
    assert!(row(&h, 24).starts_with("│   f yes / d no  "));
    // No: the scene goes on.
    h.keys("d");
    assert_eq!(h.top_screen(), "dialogue");
    h.keys("f f");
    assert!(row(&h, 21).contains(" Test Knight "));
    // Yes: it closes.
    h.keys("d f");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn holding_confirm_fast_forwards() {
    let mut normal = full_screen();
    normal.keys("f f").wait(0.05);
    let mut fast = full_screen();
    // The press that moves on to the knight's line stays held.
    fast.keys("f").hold("f", 0.05);
    assert!(row(&fast, 23).contains("You're late."));
    assert!(!row(&normal, 23).contains("You're late."));
}

/// The narration that opens the test scene, fully shown, under the caption.
#[test]
fn narration_snapshot() {
    let mut h = full_screen();
    h.keys("f");
    assert_snapshot!(h.snapshot());
}

/// The lord's first line, part-way through its reveal.
#[test]
fn mid_reveal_snapshot() {
    let mut h = full_screen();
    h.keys("f f f f").wait(0.1);
    assert!(row(&h, 23).starts_with("│   Better late t  "));
    assert_snapshot!(h.snapshot());
}

/// The knight's first line, fully shown, with the `▼`.
#[test]
fn fully_revealed_snapshot() {
    let mut h = full_screen();
    h.keys("f f f");
    assert_snapshot!(h.snapshot());
}

/// The lord answering, over the Quick Battle map.
#[test]
fn overlay_on_the_battle_map_snapshot() {
    let mut h = over_the_map();
    h.keys("f f f f f");
    assert_snapshot!(h.snapshot());
}
