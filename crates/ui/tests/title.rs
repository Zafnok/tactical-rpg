//! Scripted tests of the title screen through the real game (ADR-0007
//! layer 4), with the right-handed layout already picked
//! (`docs/design/controls.md`): arrows move, `f` selects, `d` backs out.

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_ui::audio::AudioRequest;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// A launch after the right-handed layout was picked.
fn title() -> Harness {
    Harness::with_layout(Layout::RightHanded)
}

#[test]
fn title_renders() {
    let h = title();
    assert_eq!(h.top_screen(), "title");
    assert_snapshot!(h.snapshot());
}

#[test]
fn down_twice_then_select_quits() {
    let mut h = title();
    h.keys("Down Down f");
    assert!(h.quit_requested());
}

#[test]
fn up_wraps_to_quit() {
    let mut h = title();
    h.keys("Up f");
    assert!(h.quit_requested());
}

#[test]
fn select_opens_the_placeholder() {
    let mut h = title();
    h.keys("f");
    assert_eq!(h.top_screen(), "placeholder");
    assert_eq!(h.screens(), ["title", "placeholder"]);
    assert!(!h.quit_requested());
    assert_snapshot!(h.snapshot());
}

#[test]
fn back_returns_to_the_title() {
    let mut h = title();
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
    let mut h = title();
    h.keys("d Escape");
    assert_eq!(h.top_screen(), "title");
    assert!(!h.quit_requested());
}

#[test]
fn holding_down_repeats_and_wraps() {
    // Held for 0.43 s: the press plus repeats at 300, 355 and 410 ms makes
    // four moves over three items (the harness has debug tools on, so Quick
    // Battle is there), wrapping round to Quick Battle.
    let mut h = title();
    h.hold("Down", 0.43).wait(0.5).keys("f");
    assert_eq!(h.top_screen(), "battle");
}

#[test]
fn f2_opens_the_debug_menu() {
    let mut h = title();
    // F12 opens the browser's developer tools on the web; it does nothing here.
    h.keys("F12");
    assert_eq!(h.top_screen(), "title");
    h.keys("F2");
    assert_eq!(h.top_screen(), "debug_menu");
    h.keys("f");
    assert_eq!(h.top_screen(), "glyph_sampler");
    h.keys("d Down f");
    assert_eq!(h.top_screen(), "portrait_viewer");
    h.keys("d d");
    assert_eq!(h.top_screen(), "title");
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = FontAtlasDef::load().unwrap_or_default();
    let mut h = title();
    for script in ["", "Down", "Up f"] {
        h.keys(script);
        let snap = h.snapshot();
        let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
        for g in glyphs.chars().filter(|&c| c != '\n') {
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
    }
}

/// The music cues the run asked for, in order.
fn music(h: &Harness) -> Vec<String> {
    h.audio_requests()
        .into_iter()
        .filter_map(|r| match r {
            AudioRequest::PlayMusic { cue } => Some(cue),
            _ => None,
        })
        .collect()
}

#[test]
fn the_title_asks_for_its_music_once() {
    let mut h = title();
    h.wait(0.1);
    assert_eq!(
        h.audio_requests(),
        [AudioRequest::PlayMusic {
            cue: "title".into()
        }]
    );
    // Moving around the menu doesn't ask again.
    h.keys("Down Up f d");
    assert_eq!(music(&h), ["title"]);
}
