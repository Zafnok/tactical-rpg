//! Tests of turn rewind on the battle screen (ticket 0307): attack, rewind,
//! and the battle is exactly the one before the attack; charges are
//! enforced; the rewind screen's list and map preview.

use insta::assert_snapshot;
use trpg_core::{BattleState, Command, UnitAction, UnitId};

use super::BattleScreen;
use super::layout::HELP_ROW;
use super::rewind::RewindScreen;
use super::rewind::{CHARGES_ROW, LIST_ROW, PROMPT_ROW, TITLE_ROW};
use super::testing::skirmish_charged;
use crate::harness::{FRAME_DT, Harness};
use crate::input::Action;
use crate::screen::tests::ctx;
use crate::screen::{Ctx, FrameInput, Screen};

use Action::{Cancel, Confirm, CursorDown, CursorRight, CursorUp, Rewind};

fn ron(state: &BattleState) -> String {
    ron::to_string(state).unwrap_or_else(|e| panic!("{e}"))
}

/// Presses each action in its own frame, then lets `wait` seconds pass.
fn press(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], wait: f32) {
    for &a in actions {
        s.update(c, &FrameInput::new(vec![a], FRAME_DT, vec![]));
    }
    let mut left = wait;
    while left > 0.0 {
        s.update(c, &FrameInput::new(vec![], FRAME_DT, vec![]));
        left -= FRAME_DT;
    }
}

/// The skirmish with `charges`, after the lord's sword attack on the
/// brigand (played out) and the archer's wait: the screen and the battle
/// before the attack.
fn attacked(c: &mut Ctx, charges: u8) -> (BattleScreen, BattleState) {
    let start = skirmish_charged(c, 20, charges);
    let mut s = BattleScreen::new(start.clone());
    // Select the lord, one step right, move there.
    press(&mut s, c, &[Confirm, CursorRight, Confirm], 0.5);
    // Attack, the iron sword, the brigand, attack, skip the playback.
    press(
        &mut s,
        c,
        &[Confirm, Confirm, CursorRight, Confirm, Confirm],
        0.1,
    );
    assert_eq!(s.history().len(), 1);
    assert!(matches!(
        s.history().commands()[0],
        Command::Act {
            unit: UnitId(1),
            action: UnitAction::Attack { .. },
            ..
        }
    ));
    (s, start)
}

/// Harness cells `x..x + w` of row `y`, trimmed.
fn text(h: &Harness, x: i32, y: i32, w: i32) -> String {
    let buf = h.game().buffer();
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The side panel's row `y`, inside its border.
fn panel(h: &Harness, y: i32) -> String {
    text(h, 71, y, 28)
}

fn help(h: &Harness) -> String {
    text(h, 0, HELP_ROW, 100)
}

#[test]
fn attack_then_rewind_restores_exactly_the_state_before_the_attack() {
    let mut c = ctx();
    let (mut s, start) = attacked(&mut c, 3);
    assert_ne!(ron(s.state()), ron(&start));
    press(&mut s, &mut c, &[Rewind], 0.0);
    let r = s.rewind().expect("open");
    assert_eq!(r.entries().len(), 1);
    assert!(
        r.entries()[0]
            .line
            .starts_with("Turn 1 · Test Lord attacked Brigand ("),
        "{}",
        r.entries()[0].line
    );
    // Confirm asks, Cancel backs out of the prompt, Confirm twice rewinds.
    press(&mut s, &mut c, &[Confirm], 0.0);
    assert!(s.rewind().is_some_and(RewindScreen::is_confirming));
    press(&mut s, &mut c, &[Cancel], 0.0);
    assert!(s.rewind().is_some_and(|r| !r.is_confirming()));
    assert_eq!(s.history().charges_left(), 3);
    press(&mut s, &mut c, &[Confirm, Confirm], 0.0);
    assert!(s.rewind().is_none());
    assert_eq!(ron(s.state()), ron(&start));
    assert_eq!(s.history().charges_left(), 2);
    assert!(s.history().is_empty());
    assert!(matches!(s.mode(), super::mode::Mode::Idle { .. }));
}

#[test]
fn the_same_attack_after_a_rewind_gives_the_same_result() {
    let mut c = ctx();
    let (mut s, _) = attacked(&mut c, 3);
    let first = ron(s.state());
    press(&mut s, &mut c, &[Rewind, Confirm, Confirm], 0.0);
    // The cursor is still on the brigand: back to the lord and again.
    press(
        &mut s,
        &mut c,
        &[Action::CursorLeft, Action::CursorLeft],
        0.0,
    );
    press(&mut s, &mut c, &[Confirm, CursorRight, Confirm], 0.5);
    press(
        &mut s,
        &mut c,
        &[Confirm, Confirm, CursorRight, Confirm, Confirm],
        0.1,
    );
    assert_eq!(ron(s.state()), first);
}

#[test]
fn with_no_charges_the_screen_says_so_and_cant_confirm() {
    let mut c = ctx();
    let (mut s, start) = attacked(&mut c, 0);
    let after = ron(s.state());
    press(&mut s, &mut c, &[Rewind], 0.0);
    let r = s.rewind().expect("open");
    assert!(!r.can_rewind());
    press(&mut s, &mut c, &[Confirm, Confirm], 0.0);
    assert!(s.rewind().is_some_and(|r| !r.is_confirming()));
    assert_eq!(ron(s.state()), after);
    assert_ne!(after, ron(&start));
    // Rewind closes it again.
    press(&mut s, &mut c, &[Rewind], 0.0);
    assert!(s.rewind().is_none());

    let mut h = Harness::with_screen(Box::new(attacked(&mut c, 0).0));
    h.keys("r");
    assert_eq!(panel(&h, CHARGES_ROW), "No charges left");
    assert_eq!(help(&h), "arrows choose · d close");
    h.keys("f");
    assert_eq!(panel(&h, PROMPT_ROW), "");
}

#[test]
fn rewind_opens_only_while_browsing() {
    let mut c = ctx();
    let mut s = BattleScreen::new(skirmish_charged(&c, 20, 3));
    // A unit selected: Rewind does nothing.
    press(&mut s, &mut c, &[Confirm, Rewind], 0.0);
    assert!(s.rewind().is_none());
    press(&mut s, &mut c, &[Cancel, Rewind], 0.0);
    let r = s.rewind().expect("open while browsing");
    assert!(r.entries().is_empty());
    assert!(!r.can_rewind());
    // Nothing listed: Confirm does nothing, Cancel closes.
    press(&mut s, &mut c, &[Confirm], 0.0);
    assert!(s.rewind().is_some_and(|r| !r.is_confirming()));
    press(&mut s, &mut c, &[Cancel], 0.0);
    assert!(s.rewind().is_none());

    let mut h = Harness::with_screen(Box::new(BattleScreen::new(skirmish_charged(&c, 20, 3))));
    h.keys("r");
    assert_eq!(panel(&h, TITLE_ROW), "Rewind");
    assert_eq!(panel(&h, CHARGES_ROW), "3 of 3 charges left");
    assert_eq!(panel(&h, LIST_ROW), "Nothing to rewind yet");
    assert_eq!(help(&h), "d close");
}

#[test]
fn the_list_is_newest_first_and_the_map_shows_the_highlighted_point() {
    let mut c = ctx();
    let (mut s, start) = attacked(&mut c, 3);
    // The archer (at (8, 4)) waits: two entries.
    press(&mut s, &mut c, &[Action::NextUnit], 0.0);
    press(&mut s, &mut c, &[Confirm, Confirm], 0.3);
    press(&mut s, &mut c, &[CursorUp, Confirm], 0.0);
    assert_eq!(s.history().len(), 2, "{:?}", s.history().commands());
    let archer_waited = s.state().clone();
    press(&mut s, &mut c, &[Rewind], 0.0);
    let r = s.rewind().expect("open");
    let lines: Vec<&str> = r.entries().iter().map(|e| e.line.as_str()).collect();
    assert_eq!(lines[0], "Turn 1 · Test Archer waited", "{lines:?}");
    assert!(lines[1].starts_with("Turn 1 · Test Lord attacked"));
    assert_eq!(r.entries()[0].point, 1);
    assert_ne!(r.focused().map(|e| &e.before), Some(&archer_waited));
    // Down highlights the attack: the map before it. Down stops at the
    // oldest; Up at the newest.
    press(&mut s, &mut c, &[CursorDown, CursorDown], 0.0);
    let r = s.rewind().expect("open");
    assert_eq!(r.focused().map(|e| ron(&e.before)), Some(ron(&start)));
    press(&mut s, &mut c, &[CursorUp, CursorUp], 0.0);
    assert_eq!(
        s.rewind().and_then(|r| r.focused()).map(|e| e.point),
        Some(1)
    );
    // Rewinding to the archer's wait keeps the attack.
    press(&mut s, &mut c, &[Confirm, Confirm], 0.0);
    assert_eq!(s.history().len(), 1);
    assert!(s.state().unit(UnitId(3)).is_some_and(|u| !u.acted));
    assert!(s.state().unit(UnitId(1)).is_some_and(|u| u.acted));
}

/// The rewind screen after the lord's attack, the prompt up: the map shows
/// the battle before the attack (the lord back at (6, 2), not dimmed).
#[test]
fn rewind_screen_snapshot() {
    let mut c = ctx();
    let mut h = Harness::with_screen(Box::new(attacked(&mut c, 3).0));
    h.keys("r");
    assert_eq!(panel(&h, TITLE_ROW), "Rewind");
    assert_eq!(panel(&h, CHARGES_ROW), "3 of 3 charges left");
    assert!(panel(&h, LIST_ROW).starts_with("Turn 1 · Test Lord"));
    assert_eq!(help(&h), "arrows choose · f rewind here · d close");
    h.keys("f");
    assert_eq!(panel(&h, PROMPT_ROW), "Use 1 of 3 charges?");
    assert_eq!(help(&h), "f rewind · d back");
    assert_snapshot!(h.snapshot());
}
