//! Scripted tests of the first-launch layout picker through the real game
//! (ADR-0007 layer 4, `docs/design/controls.md`).

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

#[test]
fn first_launch_shows_the_picker() {
    let h = Harness::new();
    assert_eq!(h.screens(), ["title", "layout_picker"]);
    assert_snapshot!(h.snapshot());
}

#[test]
fn left_handed_focused() {
    let mut h = Harness::new();
    h.keys("Down");
    assert_eq!(h.top_screen(), "layout_picker");
    assert_snapshot!(h.snapshot());
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = FontAtlasDef::load().unwrap_or_default();
    let mut h = Harness::new();
    for script in ["", "Down"] {
        h.keys(script);
        let snap = h.snapshot();
        let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
        for g in glyphs.chars().filter(|&c| c != '\n') {
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
    }
}

#[test]
fn picker_works_with_either_hand() {
    // Up/Down or w/s choose; f, j, Enter or Space pick.
    for (script, layout) in [
        ("f", Layout::RightHanded),
        ("Down Enter", Layout::LeftHanded),
        ("s j", Layout::LeftHanded),
        ("s w Space", Layout::RightHanded),
        ("Up Up j", Layout::RightHanded),
    ] {
        let mut h = Harness::new();
        h.keys(script);
        assert_eq!(h.screens(), ["title"], "{script}");
        assert_eq!(h.game().ctx().layout(), Some(layout), "{script}");
    }
}

#[test]
fn picker_cannot_be_cancelled() {
    let mut h = Harness::new();
    h.keys("d k Escape Backspace");
    assert_eq!(h.top_screen(), "layout_picker");
    assert!(!h.quit_requested());
}

#[test]
fn the_choice_persists_across_restart() {
    let mut h = Harness::new();
    h.keys("s j");
    let h = Harness::with_storage(h.into_storage());
    assert_eq!(h.screens(), ["title"], "second launch skips the picker");
    assert_eq!(h.game().ctx().layout(), Some(Layout::LeftHanded));
}

#[test]
fn left_handed_wasd_moves_and_j_confirms() {
    let mut h = Harness::new();
    h.keys("s j");
    // Title menu: `s` moves down, `w` back to New Game, `j` confirms.
    h.keys("s w j");
    assert_eq!(h.top_screen(), "placeholder");
    // The help text names the left-handed keys.
    assert!(h.snapshot().contains("press k to go back"));
    h.keys("k");
    assert_eq!(h.top_screen(), "title");
    assert!(h.snapshot().contains("wasd move · j select · k back"));
    // Right-handed keys do nothing now.
    h.keys("f Down");
    assert_eq!(h.top_screen(), "title");
    h.keys("s s j");
    assert!(h.quit_requested());
}

#[test]
fn right_handed_arrows_move_and_f_confirms() {
    let mut h = Harness::new();
    h.keys("Enter");
    h.keys("j");
    assert_eq!(h.top_screen(), "title", "j does nothing right-handed");
    h.keys("f");
    assert_eq!(h.top_screen(), "placeholder");
    h.keys("d Down Down f");
    assert!(h.quit_requested());
}
