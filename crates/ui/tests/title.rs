//! Scripted tests of the title screen through the real game (ADR-0007
//! layer 4). Key names follow the default right-handed layout
//! (`docs/design/controls.md`): arrows move, `f` selects, `d` backs out.

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_ui::harness::Harness;

#[test]
fn title_renders() {
    let h = Harness::new();
    assert_eq!(h.top_screen(), "title");
    assert_snapshot!(h.snapshot());
}

#[test]
fn down_then_select_quits() {
    let mut h = Harness::new();
    h.keys("Down f");
    assert!(h.quit_requested());
}

#[test]
fn up_wraps_to_quit() {
    let mut h = Harness::new();
    h.keys("Up f");
    assert!(h.quit_requested());
}

#[test]
fn select_opens_the_placeholder() {
    let mut h = Harness::new();
    h.keys("f");
    assert_eq!(h.top_screen(), "placeholder");
    assert_eq!(h.screens(), ["title", "placeholder"]);
    assert!(!h.quit_requested());
    assert_snapshot!(h.snapshot());
}

#[test]
fn back_returns_to_the_title() {
    let mut h = Harness::new();
    let title = h.snapshot();
    h.keys("f d");
    assert_eq!(h.top_screen(), "title");
    assert_eq!(h.snapshot(), title);
    // Escape also backs out.
    h.keys("f Escape");
    assert_eq!(h.top_screen(), "title");
}

#[test]
fn back_on_the_title_does_nothing() {
    let mut h = Harness::new();
    h.keys("d Escape");
    assert_eq!(h.top_screen(), "title");
    assert!(!h.quit_requested());
}

#[test]
fn holding_down_repeats_and_wraps() {
    // Held for 0.3 s: the press plus repeats at 170, 225 and 280 ms makes
    // four moves over two items, back on New Game.
    let mut h = Harness::new();
    h.hold("Down", 0.3).wait(0.5).keys("f");
    assert_eq!(h.top_screen(), "placeholder");
}

#[test]
fn f12_opens_the_glyph_sampler() {
    let mut h = Harness::new();
    h.keys("F12");
    assert_eq!(h.top_screen(), "glyph_sampler");
    h.keys("d");
    assert_eq!(h.top_screen(), "title");
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = FontAtlasDef::load().unwrap_or_default();
    let mut h = Harness::new();
    for script in ["", "Down", "Up f"] {
        h.keys(script);
        let snap = h.snapshot();
        let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
        for g in glyphs.chars().filter(|&c| c != '\n') {
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
    }
}
