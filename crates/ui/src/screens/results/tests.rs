//! Tests of the results screen on rewards made by hand over the Quick
//! Battle's units: the lord (90 EXP) levels up from a 21 EXP bonus, the
//! knight (10 EXP) doesn't, the archer gets none.

use insta::assert_snapshot;
use trpg_core::{BattleState, Event, Faction, StatGains, Unit, UnitId};

use super::*;
use crate::harness::{FRAME_DT, Harness};
use crate::screen::tests::ctx;
use crate::screens::battle::progress::{LEVEL_BANNER_ROW, LEVEL_TEXT_X, Page};
use crate::screens::battle::quick_battle;

fn state() -> BattleState {
    quick_battle(&ctx().content).unwrap()
}

/// The Quick Battle's first three units: the lord with 90 EXP, the knight with
/// 10 and the archer.
fn deployed(s: &BattleState) -> Vec<Unit> {
    let players = s.units().iter().filter(|u| u.faction == Faction::Player);
    let mut units: Vec<Unit> = players.take(3).cloned().collect();
    units[0].exp = 90;
    units[1].exp = 10;
    units
}

/// 500 gold and 3 unused charges: the lord levels up, the knight gains,
/// the archer gets nothing.
fn rewards(s: &BattleState) -> BattleRewards {
    let exp = |id, amount| Event::ExpGained {
        unit: UnitId(id),
        amount,
    };
    BattleRewards {
        clear_gold: 500,
        unused_charges: 3,
        bonus_exp: 21,
        deployed: deployed(s),
        events: vec![
            exp(1, 21),
            Event::LeveledUp {
                unit: UnitId(1),
                level: 2,
                gains: StatGains([1, 1, 0, 0, 2, 0, 0]),
            },
            exp(2, 21),
        ],
        ..BattleRewards::default()
    }
}

fn screen() -> ResultsScreen {
    let s = state();
    ResultsScreen::new(&rewards(&s), &s, 3, 1500)
}

/// One frame of `dt` seconds with `actions` pressed and `held` held.
fn frame(s: &mut ResultsScreen, actions: &[Action], held: &[Action], dt: f32) -> bool {
    let input = FrameInput::new(actions.to_vec(), dt, held.to_vec());
    matches!(s.update(&mut ctx(), &input), Transition::Pop)
}

fn press(s: &mut ResultsScreen, action: Action) -> bool {
    frame(s, &[action], &[], 0.0)
}

fn wait(s: &mut ResultsScreen, dt: f32) -> bool {
    frame(s, &[], &[], dt)
}

fn drawn(s: &ResultsScreen) -> GlyphBuffer {
    let c = ctx();
    let blank = Cell::new(
        '?',
        c.palette.get(UiColor::Text),
        c.palette.get(UiColor::Text),
    );
    let mut buf = GlyphBuffer::new(100, 32, blank);
    s.draw(&c, &mut buf);
    buf
}

fn row_text(buf: &GlyphBuffer, y: i32) -> String {
    let row: String = (0..100)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect();
    row.trim().to_owned()
}

fn shows(buf: &GlyphBuffer, text: &str) -> bool {
    (0..32).any(|y| row_text(buf, y).contains(text))
}

/// The EXP each row shows.
fn shown(s: &ResultsScreen) -> Vec<u32> {
    s.rows().iter().map(|r| s.exp_shown(r)).collect()
}

#[test]
fn rows_are_the_deployed_units_before_the_bonus() {
    let s = screen();
    assert_eq!(s.name(), "results");
    let rows: Vec<_> = s
        .rows()
        .iter()
        .map(|r| (r.name.as_str(), r.class.as_str(), r.level, r.exp, r.gained))
        .collect();
    assert_eq!(
        rows,
        [
            ("Test Lord", "Exile", 1, 90, 21),
            ("Test Knight", "Guard", 1, 10, 21),
            ("Test Archer", "Archer", 1, 0, 0),
        ]
    );
    // The level up, without the EXP bars the screen shows itself.
    let pages = s.pages().unwrap().pages();
    assert!(matches!(pages, [Page::LevelUp(p)] if p.unit == UnitId(1)));
}

#[test]
fn the_bars_wait_then_fill_over_their_time() {
    let mut s = screen();
    assert_eq!(shown(&s), [90, 10, 0]);
    assert!(!wait(&mut s, INTRO_S));
    assert_eq!(shown(&s), [90, 10, 0]);
    assert!(!s.filled());
    wait(&mut s, FILL_S / 3.0);
    assert_eq!(shown(&s), [97, 17, 0]);
    wait(&mut s, FILL_S / 3.0);
    assert_eq!(shown(&s), [104, 24, 0]);
    assert!(!s.filled());
    wait(&mut s, FILL_S / 3.0 + 0.01);
    assert_eq!(shown(&s), [111, 31, 0]);
    assert!(s.filled());
    // Full bars wait for a press.
    assert!(!wait(&mut s, 10.0));
    assert!(!s.paging());
    // Bad frame times count as nothing.
    let mut s = screen();
    wait(&mut s, f32::NAN);
    wait(&mut s, -1.0);
    wait(&mut s, INTRO_S + FILL_S / 3.0);
    assert_eq!(shown(&s), [97, 17, 0]);
}

#[test]
fn holding_confirm_fills_faster_and_a_press_fills_at_once() {
    let mut s = screen();
    frame(&mut s, &[], &[Action::Confirm], INTRO_S / 4.0);
    frame(&mut s, &[], &[Action::Confirm], FILL_S / 12.0);
    assert_eq!(shown(&s), [97, 17, 0]);
    // Holding Cancel doesn't.
    frame(&mut s, &[], &[Action::Cancel], FILL_S / 3.0);
    assert_eq!(shown(&s), [104, 24, 0]);
    assert!(!press(&mut s, Action::Cancel));
    assert_eq!(shown(&s), [111, 31, 0]);
    assert!(s.filled() && !s.paging());
}

#[test]
fn a_press_on_full_bars_shows_the_level_up_which_waits_for_its_own() {
    for key in [Action::Confirm, Action::Cancel] {
        let mut s = screen();
        // Fill, then go on.
        assert!(!press(&mut s, key));
        assert!(!s.paging());
        assert!(!press(&mut s, key));
        assert!(s.paging());
        let buf = drawn(&s);
        assert!(!shows(&buf, GOLD_LABEL));
        // The page, moved from the map view to the middle of the screen.
        let x = LEVEL_TEXT_X + 15;
        let banner: String = (x..x + 9)
            .map(|x| buf.get(x, LEVEL_BANNER_ROW).unwrap().glyph)
            .collect();
        assert_eq!(banner, "LEVEL UP!");
        assert!(shows(&buf, "Test Lord"));
        assert!(shows(&buf, "Lv 1 → 2"));
        assert!(!shows(&buf, "Str   6 → 7   +1"));
        // It reveals its stats and then waits, however long.
        assert!(!wait(&mut s, 30.0));
        assert!(shows(&drawn(&s), "Str   6 → 7   +1"));
        assert!(!press(&mut s, Action::CursorDown));
        assert!(press(&mut s, key), "{key:?} closes the last page");
    }
}

#[test]
fn a_press_mid_reveal_shows_every_stat_before_closing() {
    let mut s = screen();
    press(&mut s, Action::Confirm);
    press(&mut s, Action::Confirm);
    assert!(!shows(&drawn(&s), "Spd"));
    assert!(!press(&mut s, Action::Cancel));
    assert!(shows(&drawn(&s), "Spd   7 → 9   +2"));
    assert!(press(&mut s, Action::Cancel));
}

#[test]
fn without_a_level_up_the_press_on_full_bars_closes() {
    let s0 = state();
    let mut r = rewards(&s0);
    r.events.remove(1);
    r.deployed[0].exp = 20;
    let mut s = ResultsScreen::new(&r, &s0, 3, 1500);
    assert_eq!(s.pages(), None);
    assert!(!press(&mut s, Action::Confirm));
    assert_eq!(shown(&s), [41, 31, 0]);
    assert!(press(&mut s, Action::Confirm));
}

#[test]
fn every_charge_used_shows_no_exp_line_and_closes_on_one_press() {
    let s0 = state();
    let r = BattleRewards {
        clear_gold: 500,
        deployed: deployed(&s0),
        ..BattleRewards::default()
    };
    let mut s = ResultsScreen::new(&r, &s0, 3, 500);
    assert!(s.rows().is_empty());
    assert!(s.filled());
    let buf = drawn(&s);
    assert_eq!(
        row_text(&buf, GOLD_ROW),
        format!("║   {GOLD_LABEL}                   +500 (now 500)         ║")
    );
    assert!(!shows(&buf, BONUS_LABEL));
    assert!(!shows(&buf, "EXP"));
    assert!(!shows(&buf, "─"));
    assert!(shows(&buf, "f continue"));
    assert!(press(&mut s, Action::Confirm));
}

#[test]
fn a_battle_without_clear_gold_shows_no_gold_line() {
    let s0 = state();
    let mut r = rewards(&s0);
    r.clear_gold = 0;
    let buf = drawn(&ResultsScreen::new(&r, &s0, 3, 40));
    assert!(!shows(&buf, GOLD_LABEL));
    assert!(!shows(&buf, "now"));
    assert!(shows(&buf, "3 of 3"));
    assert!(shows(
        &buf,
        &format!("{BONUS_LABEL}                     +21")
    ));
}

#[test]
fn rows_show_the_level_reached_and_mark_the_level_up() {
    let mut s = screen();
    let lord = |s: &ResultsScreen| row_text(&drawn(s), UNIT_ROW);
    assert!(lord(&s).contains("Test Lord     Exile    Lv 1  EXP ██████████████████░░ 90"));
    assert!(!lord(&s).contains(LEVEL_UP));
    assert!(shows(&drawn(&s), "f skip · hold f fast"));
    wait(&mut s, INTRO_S + FILL_S * 0.5);
    // 100 EXP exactly: the level is reached.
    assert_eq!(shown(&s)[0], 100);
    assert!(lord(&s).contains("Lv 2  EXP ░░░░░░░░░░░░░░░░░░░░  0  LEVEL UP"));
    press(&mut s, Action::Confirm);
    let buf = drawn(&s);
    assert!(lord(&s).contains("Lv 2  EXP ██░░░░░░░░░░░░░░░░░░ 11  LEVEL UP"));
    // A blank row between units; the unit at the cap keeps its bar.
    assert_eq!(
        row_text(&buf, UNIT_ROW + 1),
        "║".to_owned() + &" ".repeat(70) + "║"
    );
    assert!(
        row_text(&buf, UNIT_ROW + 2)
            .contains("Test Knight   Guard    Lv 1  EXP ██████░░░░░░░░░░░░░░ 31")
    );
    assert!(!row_text(&buf, UNIT_ROW + 2).contains(LEVEL_UP));
    assert!(
        row_text(&buf, UNIT_ROW + 4)
            .contains("Test Archer   Archer   Lv 1  EXP ░░░░░░░░░░░░░░░░░░░░  0")
    );
    assert!(shows(&buf, "f continue"));
}

#[test]
fn a_big_army_drops_the_blank_rows() {
    let s0 = state();
    let unit = deployed(&s0).remove(1);
    let army = |n: usize| {
        let mut r = rewards(&s0);
        r.events.clear();
        r.deployed = vec![unit.clone(); n];
        drawn(&ResultsScreen::new(&r, &s0, 3, 0))
    };
    // Seven fit with blank rows between them; eight don't.
    let buf = army(7);
    assert!(!row_text(&buf, UNIT_ROW + 1).contains("Test Knight"));
    assert!(row_text(&buf, UNIT_ROW + 12).contains("Test Knight"));
    let buf = army(8);
    assert!(row_text(&buf, UNIT_ROW + 1).contains("Test Knight"));
    assert!(row_text(&buf, UNIT_ROW + 7).contains("Test Knight"));
    assert!(!row_text(&buf, UNIT_ROW + 8).contains("Test Knight"));
    // Never over the panel's border.
    let buf = army(40);
    assert!(row_text(&buf, PANEL.y + PANEL.h - 2).contains("Test Knight"));
    assert!(row_text(&buf, PANEL.y + PANEL.h - 1).starts_with('╚'));
}

#[test]
fn long_names_are_cut_to_their_columns() {
    let s0 = state();
    let mut r = rewards(&s0);
    r.deployed[0].name = "Bartholomew the Bold".into();
    let buf = drawn(&ResultsScreen::new(&r, &s0, 3, 0));
    assert!(row_text(&buf, UNIT_ROW).contains("Bartholomew t Exile    Lv 1"));
}

#[test]
fn results_snapshot() {
    let mut h = Harness::with_screen(Box::new(screen()));
    h.wait(INTRO_S + FILL_S + FRAME_DT);
    assert_snapshot!(h.snapshot());
}

#[test]
fn results_level_up_snapshot() {
    let mut h = Harness::with_screen(Box::new(screen()));
    h.keys("f f").wait(5.0);
    assert_eq!(h.top_screen(), "results");
    assert_snapshot!(h.snapshot());
    h.keys("f");
    assert_eq!(h.top_screen(), "");
}
