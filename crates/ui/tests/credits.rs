//! Scripted tests of the credits screen through the real game (ADR-0007
//! layer 4; ticket 0808), with the right-handed layout: arrows scroll, `d`
//! backs out. The harness has debug tools on, so the title menu reads
//! New Game, Quick Battle, Credits, Quit.

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_ui::audio::AudioRequest;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// The title screen, then Credits chosen from its menu.
fn credits() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down Down f");
    h
}

/// The glyph rows of a snapshot (without its colour map).
fn glyphs(snapshot: &str) -> &str {
    snapshot.split("\n--- colours ---").next().unwrap_or("")
}

#[test]
fn the_title_menu_opens_the_credits() {
    let h = credits();
    assert_eq!(h.screens(), ["title", "credits"]);
    assert_snapshot!(h.snapshot());
}

#[test]
fn a_scrolled_page() {
    let mut h = credits();
    // Held for one second: the press, then a repeat every 55 ms from
    // 300 ms on (300, 355, … 960 ms), so 14 rows down.
    h.hold("Down", 1.0);
    assert_snapshot!(h.snapshot());
}

#[test]
fn cancel_returns_to_the_title() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    let title = h.snapshot();
    h.keys("Down Down f");
    assert_eq!(h.top_screen(), "credits");
    h.keys("Down Down d");
    assert_eq!(h.screens(), ["title"]);
    // Credits is still focused; back up at New Game it is the title as it
    // was.
    h.keys("Up Up");
    assert_eq!(h.snapshot(), title);
    // Escape also backs out, and Confirm does nothing here.
    h.keys("Down Down f f");
    assert_eq!(h.top_screen(), "credits");
    h.keys("Escape");
    assert_eq!(h.top_screen(), "title");
    assert!(!h.quit_requested());
}

#[test]
fn scrolling_stops_at_both_ends() {
    let mut h = credits();
    let top = h.snapshot();
    assert!(glyphs(&top).contains('▼') && !glyphs(&top).contains('▲'));
    h.keys("Up");
    assert_eq!(h.snapshot(), top);
    h.keys("Down");
    let one_down = h.snapshot();
    assert_ne!(one_down, top);
    assert!(glyphs(&one_down).contains('▲') && glyphs(&one_down).contains('▼'));
    h.keys("Up");
    assert_eq!(h.snapshot(), top);
    // Long enough to reach the end, and then some.
    h.hold("Down", 9.0);
    let bottom = h.snapshot();
    assert!(glyphs(&bottom).contains('▲') && !glyphs(&bottom).contains('▼'));
    assert!(glyphs(&bottom).contains("\"Terminus Font\" by Dimitar Toshkov Zhekov"));
    h.keys("Down");
    assert_eq!(h.snapshot(), bottom);
}

#[test]
fn every_credit_can_be_scrolled_to() {
    let mut h = credits();
    let entries = h.game().ctx().content.credits.entries.clone();
    assert!(entries.len() > 30);
    let mut seen = String::new();
    let mut last = String::new();
    loop {
        let snapshot = h.snapshot();
        if snapshot == last {
            break;
        }
        seen.push_str(glyphs(&snapshot));
        last = snapshot;
        h.keys("Down");
    }
    for e in &entries {
        let line = format!("\"{}\" by {}", e.title, e.author);
        assert!(seen.contains(&line), "{line} never shown");
        assert!(seen.contains(&e.license), "{} never shown", e.license);
    }
    for heading in ["Music", "Sound effects", "Fonts"] {
        assert!(seen.contains(&format!("│ {heading} ")), "{heading}");
    }
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = FontAtlasDef::load().unwrap_or_default();
    let mut h = credits();
    let mut last = String::new();
    loop {
        let snapshot = h.snapshot();
        if snapshot == last {
            break;
        }
        for g in glyphs(&snapshot).chars().filter(|&c| c != '\n') {
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
        last = snapshot;
        h.keys("Down");
    }
}

#[test]
fn sounds_and_music() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.wait(0.1).clear_audio().keys("Down Down f");
    assert_eq!(h.sounds(), ["menu_move", "menu_move", "menu_select"]);
    // A row scrolled ticks; a key that does nothing is silent.
    h.clear_audio().keys("Up f Down");
    assert_eq!(h.sounds(), ["menu_move"]);
    h.clear_audio().keys("d");
    assert_eq!(h.sounds(), ["menu_cancel"]);
    // The title music plays on throughout: nothing asks for music again.
    h.keys("f d");
    let music = h
        .audio_requests()
        .into_iter()
        .filter(|r| !matches!(r, AudioRequest::PlaySound { .. }))
        .count();
    assert_eq!(music, 0);
}
