//! Scripted tests of the credits screen through the real game (ADR-0007
//! layer 4; ticket 0808), with the right-handed layout: arrows scroll, `f`
//! stops and restarts the rolling, `d` backs out. The harness has debug
//! tools on, so the title menu reads New Game, Quick Battle, Credits, Quit.

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_ui::audio::AudioRequest;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// The title screen, then Credits chosen from its menu.
fn credits() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down Down Down f");
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
    // 300 ms on (300, 355, … 960 ms), so 14 rows down. Scrolling by hand
    // stopped the rolling, as the help line says.
    h.hold("Down", 1.0);
    assert_snapshot!(h.snapshot());
}

#[test]
fn the_list_rolls_by_itself() {
    let mut h = credits();
    let top = h.snapshot();
    assert!(top.contains("arrows scroll · f pause · d back"));
    // It rests at the top for two seconds first.
    h.wait(1.5);
    assert_eq!(h.snapshot(), top);
    // Then a row every half second: after 4.6 s in all, five or six rows
    // (the frames' times don't add up exactly), so the third entry leads.
    h.wait(3.1);
    let rolled = h.snapshot();
    assert_ne!(rolled, top);
    assert!(!glyphs(&rolled).contains("\"Aria\" by Kistol"));
    assert!(glyphs(&rolled).contains("\"Battle Theme A\" by cynicmusic"));
    assert!(glyphs(&rolled).contains('▲'));
    // Rolling makes no sound.
    assert!(h.clear_audio().wait(2.0).sounds().is_empty());
}

#[test]
fn confirm_stops_and_restarts_the_rolling() {
    let mut h = credits();
    h.wait(3.0).keys("f");
    let stopped = h.snapshot();
    assert!(stopped.contains("arrows scroll · f auto-scroll · d back"));
    h.wait(5.0);
    assert_eq!(h.snapshot(), stopped);
    h.keys("f").wait(2.0);
    let rolling = h.snapshot();
    assert!(rolling.contains("f pause"));
    assert_ne!(
        glyphs(&rolling).lines().nth(3),
        glyphs(&stopped).lines().nth(3),
        "the list moved on"
    );
}

#[test]
fn after_the_last_row_the_list_starts_again_from_the_top() {
    let mut h = credits();
    let top = h.snapshot();
    let rows = glyphs(&top).lines().count();
    assert_eq!(rows, 32);
    // Roll until the bottom is on screen (well under two minutes).
    let mut bottom = None;
    for _ in 0..240 {
        h.wait(0.5);
        let snapshot = h.snapshot();
        if !glyphs(&snapshot).contains('▼') {
            bottom = Some(snapshot);
            break;
        }
    }
    let bottom = bottom.unwrap_or_default();
    assert!(glyphs(&bottom).contains("\"Terminus Font\" by Dimitar Toshkov Zhekov"));
    // It rests there, then shows the top again.
    h.wait(1.0);
    assert_eq!(h.snapshot(), bottom);
    h.wait(1.5);
    assert_eq!(h.snapshot(), top);
}

#[test]
fn cancel_returns_to_the_title() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    let title = h.snapshot();
    h.keys("Down Down Down f");
    assert_eq!(h.top_screen(), "credits");
    h.keys("Down Down d");
    assert_eq!(h.screens(), ["title"]);
    // Credits is still focused; back up at New Game it is the title as it
    // was.
    h.keys("Up Up Up");
    assert_eq!(h.snapshot(), title);
    // Escape also backs out; Confirm doesn't leave.
    h.keys("Down Down Down f f");
    assert_eq!(h.top_screen(), "credits");
    h.keys("Escape");
    assert_eq!(h.top_screen(), "title");
    assert!(!h.quit_requested());
}

#[test]
fn scrolling_stops_at_both_ends() {
    let mut h = credits();
    assert!(h.snapshot().contains("f pause"));
    // Up has nowhere to go, but it still stops the rolling.
    h.keys("Up");
    let top = h.snapshot();
    assert!(top.contains("f auto-scroll"));
    assert!(glyphs(&top).contains('▼') && !glyphs(&top).contains('▲'));
    h.wait(5.0).keys("Up");
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
    h.wait(0.1).clear_audio().keys("Down Down Down f");
    assert_eq!(
        h.sounds(),
        ["menu_move", "menu_move", "menu_move", "menu_select"]
    );
    // A row scrolled ticks; Up at the top is silent; Confirm clicks.
    h.clear_audio().keys("Up f Down");
    assert_eq!(h.sounds(), ["menu_select", "menu_move"]);
    h.clear_audio().keys("d");
    assert_eq!(h.sounds(), ["menu_cancel"]);
    // The credits ask for the title music, which is already playing, so
    // it plays on: nothing stops or restarts it.
    h.clear_audio().keys("f");
    h.wait(1.0).keys("d");
    let music: Vec<AudioRequest> = h
        .audio_requests()
        .into_iter()
        .filter(|r| !matches!(r, AudioRequest::PlaySound { .. }))
        .collect();
    assert_eq!(
        music,
        [AudioRequest::PlayMusic {
            cue: "title".into()
        }]
    );
    assert!(h.music_commands().is_empty(), "{:?}", h.music_commands());
}
