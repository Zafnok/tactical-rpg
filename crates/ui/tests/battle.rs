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

/// The Quick Battle screen: `test_small.map` centred in the viewport, three
/// player units (the archer has acted: lowercase and dimmed; the knight is
/// wounded) and three enemies (one badly wounded), the empty side panel and
/// the help line.
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
