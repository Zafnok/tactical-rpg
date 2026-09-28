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
    // With the lord selected, back only drops the selection.
    h.keys("f Up d");
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

/// The left cell of the tile under the cursor on row `y`, found from the
/// corner marks' right-hand vertical arms (the only 1 × 3 px overlays), which
/// sit in the tile's last pixel column.
fn cursor_x(h: &Harness, y: i32) -> Option<i32> {
    h.game()
        .buffer()
        .overlays()
        .iter()
        .map(|o| o.rect)
        .filter(|r| (r.w, r.h) == (1, 3) && r.y / 16 == y)
        .map(|r| (r.x + 1) / 8 - 2)
        .max()
}

#[test]
fn arrows_move_the_cursor_and_the_panel_follows() {
    let mut h = quick_battle();
    // The lord at (3, 5), drawn from cell 26 on row 16.
    assert_eq!(cursor_x(&h, 16), Some(26));
    assert_eq!(panel(&h)[4], "Test Lord");
    h.keys("Right Right Right");
    assert_eq!(cursor_x(&h, 16), Some(32));
    assert_eq!(panel(&h)[0], "Plain");
    assert_eq!(panel(&h)[4], "");
    // Held: stops at the map's right edge (x = 13, cell 46).
    h.hold("Right", 1.0);
    assert_eq!(cursor_x(&h, 16), Some(46));
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

/// The two glyphs drawn on the tile whose left cell is `(x, y)`.
fn tile(h: &Harness, x: i32, y: i32) -> String {
    let buf = h.game().buffer();
    (x..x + 2)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// The key-help line.
fn help(h: &Harness) -> String {
    let buf = h.game().buffer();
    (0..100)
        .map(|x| buf.get(x, 31).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The lord (at (3, 5), cells 26..28 of row 16) selected, the path steered
/// two tiles right onto the fort at (5, 5) (cells 30..32).
fn lord_path() -> Harness {
    let mut h = quick_battle();
    h.keys("f Right Right");
    h
}

/// The lord selected: blue move and red attack ranges, the path from the
/// lord's tile edge to an arrowhead on the fort, no cursor frame there,
/// a double-line panel border.
#[test]
fn selected_unit_with_ranges_and_path_snapshot() {
    let h = lord_path();
    assert_eq!(help(&h), "arrows move · f move here · d cancel");
    assert_snapshot!(h.snapshot());
}

/// The lord walked to the fort, its action menu open beside it.
#[test]
fn action_menu_snapshot() {
    let mut h = lord_path();
    h.keys("f").wait(0.5);
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    assert_snapshot!(h.snapshot());
}

#[test]
fn select_move_and_wait_dims_the_unit_and_lowercases_its_label() {
    let mut h = lord_path();
    h.keys("f").wait(0.5).keys("f");
    // The lord stands on the fort, x + 2, lowercase: it has acted.
    assert_eq!(tile(&h, 30, 16), "lo");
    assert_eq!(tile(&h, 26, 16), "..");
    assert_eq!(panel(&h)[4], "Test Lord");
    // Browsing again, on a unit that can't act.
    assert_eq!(help(&h), "arrows move · e info · s next unit · d back");
    // It is no longer selectable.
    h.keys("f");
    assert_eq!(help(&h), "arrows move · e info · s next unit · d back");
}

#[test]
fn cancelling_the_menu_then_the_selection_restores_the_unit() {
    let mut h = quick_battle();
    let before = h.snapshot();
    h.keys("f Right Right f").wait(0.5);
    assert_eq!(tile(&h, 30, 16), "Lo");
    // Back to the steered path: the lord back on its tile.
    h.keys("d");
    assert_eq!(tile(&h, 26, 16), "Lo");
    assert_eq!(tile(&h, 30, 16), "[]");
    assert_eq!(help(&h), "arrows move · f move here · d cancel");
    // Back to browsing, the cursor on the lord (its pulse restarted, as
    // when the battle opened): the screen exactly as it was.
    h.keys("d");
    assert_eq!(cursor_x(&h, 16), Some(26));
    assert_eq!(help(&h), "f select · e info · s next unit · d back");
    assert_eq!(h.snapshot(), before);
}

#[test]
fn confirm_on_an_enemy_toggles_its_range() {
    let mut h = quick_battle();
    // To the brigand at (8, 2): cells 36..38, row 13.
    h.keys("Right Right Right Right Right Up Up Up");
    assert_eq!(panel(&h)[4], "Brigand");
    let bg = |h: &Harness| h.game().buffer().get(36, 18).map(|c| c.bg);
    let plain = bg(&h);
    h.keys("f");
    assert_ne!(bg(&h), plain, "(8, 7) is in its range");
    assert!(help(&h).ends_with("d hide range"), "{}", help(&h));
    h.keys("f");
    assert_eq!(bg(&h), plain);
    h.keys("f d");
    assert_eq!(bg(&h), plain);
    assert_eq!(h.top_screen(), "battle");
}

#[test]
fn the_lord_fights_the_near_brigand_on_turn_one() {
    let mut h = quick_battle();
    // The lord to (6, 4), beside the brigand at (7, 4).
    h.keys("f Right Right Right Up f").wait(0.5);
    // Attack: two swords reach, so the weapon list; the iron sword; the
    // forecast against the brigand.
    h.keys("f f");
    assert_eq!(panel(&h)[1], "Test Lord     Brigand");
    assert_eq!(help(&h), "arrows next target · f attack · d back");
    // Attack, then a tap skips the playback.
    h.keys("f");
    assert_eq!(help(&h), "f skip · hold f fast");
    h.keys("f");
    // The lord has acted, lowercase at (6, 4) (cells 32..34, row 15).
    assert_eq!(tile(&h, 32, 15), "lo");
    assert!(help(&h).ends_with("s next unit · d back"), "{}", help(&h));
}
