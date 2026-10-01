//! Scripted tests of the class-choice screen (ticket 0603) through the real
//! game (ADR-0007 layer 4), reached from the debug menu's class change
//! tools: a test knight (a mastered Guard) with one of every seal.

use insta::assert_snapshot;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// At the title with the right-handed layout, then the debug menu's
/// `Class change: promote` (`reclass` one further down).
fn class_change(reclass: bool) -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F2 Down Down Down Down");
    if reclass {
        h.keys("Down");
    }
    h.keys("f");
    assert_eq!(h.screens(), ["title", "debug_menu", "class_change"]);
    h
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

/// Whether some row of the screen contains `text`.
fn shows(h: &Harness, text: &str) -> bool {
    (0..32).any(|y| row(h, y).contains(text))
}

fn help(h: &Harness) -> String {
    row(h, 31)
}

/// Promote the knight into the second branch: the class changes and the
/// stats are the Iron Rider's bonus on the Guard's (HP 28 → 35, …).
#[test]
fn promoting_with_a_seal_changes_the_class_and_raises_the_stats() {
    let mut h = class_change(false);
    assert!(row(&h, 2).starts_with("Test Knight  Guard  Lv 12"));
    assert!(row(&h, 2).ends_with("Tier 2 Seal ×1"), "{}", row(&h, 2));
    assert_eq!(help(&h), "Left/Right class · f choose · d back");
    assert!(shows(&h, "Bulwark") && shows(&h, "Iron Rider"));
    h.keys("Right f");
    assert!(shows(&h, "Promote Test Knight to Iron Rider?"));
    assert_eq!(help(&h), "f yes · d no");
    h.keys("f");
    // The bonus plays out one stat at a time; a press shows them all.
    assert!(shows(&h, "PROMOTED!"));
    assert!(shows(&h, "Guard → Iron Rider"));
    assert!(!shows(&h, "Res   0 → 2   +2"));
    assert_eq!(help(&h), "f skip · hold f fast");
    h.keys("f");
    for line in [
        "HP   28 → 35  +7",
        "Str  11 → 14  +3",
        "Mag   0",
        "Dex   7 → 9   +2",
        "Spd   4 → 8   +4",
        "Def  49 → 50  +1",
        "Res   0 → 2   +2",
    ] {
        assert!(shows(&h, line), "{line}\n{}", h.snapshot());
    }
    assert_eq!(help(&h), "f continue");
    assert_snapshot!(h.snapshot());
    // Then back to where the screen was opened from.
    h.keys("f");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn the_bonus_plays_by_itself_too() {
    let mut h = class_change(false);
    h.keys("f f");
    h.wait(3.0);
    assert!(shows(&h, "Res   0 → 2   +2"));
    assert_eq!(help(&h), "f continue");
    // It waits for the player.
    assert_eq!(h.screens(), ["title", "debug_menu", "class_change"]);
}

#[test]
fn backing_out_changes_nothing() {
    let mut h = class_change(false);
    h.keys("f d");
    assert!(!shows(&h, "Promote Test Knight"));
    assert_eq!(help(&h), "Left/Right class · f choose · d back");
    h.keys("d");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
    // Opened again, the seal is still there.
    h.keys("f");
    assert!(row(&h, 2).ends_with("Tier 2 Seal ×1"), "{}", row(&h, 2));
}

/// A reclass lists every class the seal reaches, scrolling, with the class
/// level saved in a class the unit has been in.
#[test]
fn reclassing_lists_the_classes_and_keeps_the_stats() {
    let mut h = class_change(true);
    assert!(row(&h, 2).ends_with("Reclass Seal ×1"), "{}", row(&h, 2));
    assert_eq!(row(&h, 3), "more →");
    assert_snapshot!("reclass_choice", h.snapshot());
    h.keys("Right Right Right Right Right");
    assert!(shows(&h, "Rider"));
    assert!(shows(&h, "CL 6"));
    assert!(row(&h, 3).starts_with("← more"), "{}", row(&h, 3));
    h.keys("f");
    assert!(shows(&h, "Change Test Knight to Rider?"));
    assert!(shows(&h, "Uses 1 Reclass Seal (1 in stock)."));
    h.keys("f");
    assert!(shows(&h, "Test Knight  Guard → Rider"));
    assert_eq!(help(&h), "f continue");
    h.keys("f");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

/// A rebound key works, and the help names it.
#[test]
fn the_screen_follows_the_players_keys() {
    use trpg_ui::input::{Action, Chord};
    let mut h = class_change(false);
    let layout = Layout::RightHanded;
    let mut keys = h.game().ctx().layout_bindings(layout);
    let p = Chord::parse("p").expect("a key");
    assert_eq!(keys.bind(Action::Confirm, 0, p), Ok(None));
    if let Err(e) = h.ctx_mut().set_layout_bindings(layout, keys) {
        panic!("saving key bindings: {e}");
    }
    h.keys("p");
    assert!(shows(&h, "Promote Test Knight to Bulwark?"));
    assert_eq!(help(&h), "p yes · d no");
}
