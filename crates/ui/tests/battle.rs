//! Scripted tests of the battle screen through the real game (ADR-0007
//! layer 4), reached from the title screen's debug Quick Battle item.

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// At the title with the right-handed layout, then Quick Battle.
fn quick_battle() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down f");
    h
}

/// The Quick Battle screen at the start of the battle: `test_small.map`
/// centred in the viewport, three ready player units and three enemies, all
/// at full HP, the cursor on the lord, the side panel showing the lord and
/// its tile, and the help line.
#[test]
fn quick_battle_renders() {
    let h = quick_battle();
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_snapshot!(h.snapshot());
}

#[test]
fn back_returns_to_the_title() {
    let mut h = quick_battle();
    h.keys("f Up");
    assert_eq!(h.top_screen(), "battle");
    h.keys("d");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = FontAtlasDef::load().unwrap_or_default();
    let snap = quick_battle().snapshot();
    let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
    for g in glyphs.chars().filter(|&c| c != '\n') {
        assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
    }
}

/// The side panel's rows 1..9, inside the border, trimmed.
fn panel(h: &Harness) -> Vec<String> {
    let buf = h.game().buffer();
    (1..9)
        .map(|y| {
            (71..99)
                .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .collect()
}

/// The column of the cursor's `[` on row `y` (not a fort's).
fn bracket_x(h: &Harness, y: i32) -> Option<i32> {
    let buf = h.game().buffer();
    let fort = h.game().ctx().palette.lookup("fort");
    (0..70).find(|&x| {
        buf.get(x, y)
            .is_some_and(|c| c.glyph == '[' && Some(c.fg) != fort)
    })
}

#[test]
fn arrows_move_the_cursor_and_the_panel_follows() {
    let mut h = quick_battle();
    // The lord at (3, 5), drawn from cell 26 on row 16.
    assert_eq!(bracket_x(&h, 16), Some(25));
    assert_eq!(panel(&h)[4], "Test Lord");
    h.keys("Right Right Right");
    assert_eq!(bracket_x(&h, 16), Some(31));
    assert_eq!(panel(&h)[0], "Plain");
    assert_eq!(panel(&h)[4], "");
    // Held: stops at the map's right edge (x = 13, cell 46).
    h.hold("Right", 1.0);
    assert_eq!(bracket_x(&h, 16), Some(45));
    assert_eq!(panel(&h)[0], "Plain");
    // Two tiles left is a fort.
    h.keys("Left Left");
    assert_eq!(panel(&h)[0], "Fort");
    assert_eq!(panel(&h)[1], "DEF +2  AVO +20");
}

#[test]
fn next_unit_jumps_between_ready_units() {
    let mut h = quick_battle();
    // Reading order: archer (2, 4), lord (3, 5), knight (4, 6).
    h.keys("s");
    assert_eq!(panel(&h)[4], "Test Knight");
    h.keys("s");
    assert_eq!(panel(&h)[4], "Test Archer");
    h.keys("s");
    assert_eq!(panel(&h)[4], "Test Lord");
    h.keys("a");
    assert_eq!(panel(&h)[4], "Test Archer");
}
