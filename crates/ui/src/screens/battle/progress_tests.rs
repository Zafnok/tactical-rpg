//! Harness tests of the EXP bar and level-up screen (ticket 0602): a lord
//! one EXP short of a level up attacks, the playback is skipped, the EXP
//! bar fills and wraps, the level-up page shows exactly the `LeveledUp`
//! event's numbers and closes on Confirm, and the battle is the one `core`
//! made; a class mastery; a heal (no playback) goes straight to the EXP bar.

use insta::assert_snapshot;
use trpg_core::{
    BattleState, CastTarget, ClassRecord, Command, Event, Phase, Pos, SpellId, StatKind,
    UnitAction, UnitId,
};

use super::BattleScreen;
use super::banner::Banner;
use super::layout::HELP_ROW;
use super::mode::Mode;
use super::progress::{
    EXP_BOX, LEVEL_BANNER_ROW, LEVEL_TEXT_X, PROGRESS_TIMINGS, Page, STAT_ROW, stat_row,
};
use super::testing::{battle, quick_units, wait};
use crate::harness::{FRAME_DT, Harness};
use crate::input::Action;
use crate::screen::tests::ctx;
use crate::screen::{Ctx, FrameInput, Screen};
use trpg_content::TipTrigger;

/// Seconds the EXP bar is up.
const EXP_S: f32 = PROGRESS_TIMINGS.exp_fill + PROGRESS_TIMINGS.exp_hold;

/// The skirmish (the lord at (6, 2), the brigand at (8, 2) with 20 HP),
/// the lord with `exp` EXP and its Exile record `record`.
fn skirmish(c: &Ctx, exp: u32, record: Option<ClassRecord>) -> BattleState {
    let (map, mut units) = quick_units(c);
    units[0].pos = Pos::new(6, 2);
    units[0].exp = exp;
    if let Some(r) = record {
        let class = units[0].class.clone();
        units[0].class_records.insert(class, r);
    }
    units[2].pos = Pos::new(8, 4);
    battle(c, map, units)
}

/// The lord's sword attack on the brigand from (7, 2).
fn attack() -> Command {
    Command::Act {
        unit: UnitId(1),
        dest: Pos::new(7, 2),
        action: UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: None,
            art: None,
        },
    }
}

/// `state` after the attack, and its events.
fn expected(state: &BattleState) -> (BattleState, Vec<Event>) {
    let mut after = state.clone();
    let events = after.apply(&attack()).unwrap_or_else(|e| panic!("{e}"));
    (after, events)
}

/// The keys of the attack: select the lord, one step right, the iron
/// sword on the brigand, attack, then skip the playback.
const ATTACK_KEYS: &str = "f Right f";
const TARGET_KEYS: &str = "f f Right f d";

/// Cells `x..x + w` of row `y`.
fn text(h: &Harness, x: i32, y: i32, w: i32) -> String {
    let buf = h.game().buffer();
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// The help text: the row left of the right-aligned debug hint.
fn help(h: &Harness) -> String {
    text(h, 0, HELP_ROW, 90).trim().to_owned()
}

/// Whether some row shows `s`.
fn shows(h: &Harness, s: &str) -> bool {
    (0..32).any(|y| text(h, 0, y, 100).contains(s))
}

/// The skirmish in the harness, the attack sent and its playback skipped.
fn attacked(state: BattleState) -> Harness {
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(state)));
    h.keys(ATTACK_KEYS).wait(0.5);
    h.keys(TARGET_KEYS);
    h
}

#[test]
fn a_level_up_fills_the_bar_then_shows_the_events_numbers() {
    let c = ctx();
    let state = skirmish(&c, 99, None);
    let (after, events) = expected(&state);
    let gains = events
        .iter()
        .find_map(|e| match e {
            Event::LeveledUp { level, gains, .. } => Some((*level, *gains)),
            _ => None,
        })
        .expect("a level up");
    assert_eq!(gains.0, 2);
    let mut h = attacked(state.clone());
    // The EXP bar, filled and wrapped: 99 + the award, past 100.
    h.wait(PROGRESS_TIMINGS.exp_fill + 0.05);
    assert_eq!(help(&h), "f skip · hold f fast");
    let gained = events.iter().find_map(|e| match e {
        Event::ExpGained { amount, .. } => Some(*amount),
        _ => None,
    });
    let shown = format!("{:>2}", (99 + gained.unwrap_or(0)) % 100);
    assert!(shows(&h, "EXP "), "the bar");
    assert!(shows(&h, &shown));
    // Everything inside the box: its right border is whole.
    let right = EXP_BOX.x + EXP_BOX.w - 1;
    assert_eq!(text(&h, right, EXP_BOX.y + 1, 1), "║");
    assert_eq!(text(&h, right - 3, EXP_BOX.y + 1, 2), shown);
    assert_snapshot!("exp_bar", h.snapshot());
    // Then the level-up page, its stats appearing one by one.
    h.wait(PROGRESS_TIMINGS.exp_hold);
    assert!(shows(&h, "LEVEL UP!"));
    assert_eq!(text(&h, LEVEL_TEXT_X, LEVEL_BANNER_ROW + 3, 6), "Lv 1 →");
    assert_eq!(text(&h, LEVEL_TEXT_X, STAT_ROW + 6, 3), "   ");
    h.wait(2.0);
    let lord = &state.units()[0];
    for (i, kind) in StatKind::GROWABLE.iter().enumerate() {
        let row = stat_row(*kind, lord.stats.get(*kind), gains.1.0[i]);
        let w = i32::try_from(row.chars().count()).unwrap_or(0);
        let y = STAT_ROW + i32::try_from(i).unwrap_or(0);
        assert_eq!(text(&h, LEVEL_TEXT_X, y, w), row);
    }
    assert_eq!(help(&h), "f continue");
    assert_snapshot!("level_up", h.snapshot());
    // Confirm closes it: browsing again.
    h.keys("f");
    assert!(!shows(&h, "LEVEL UP!"));
    assert!(help(&h).starts_with("arrows move"), "{}", help(&h));
    let now = after.unit(UnitId(1)).expect("the lord");
    assert_eq!(now.level, 2);
    assert_eq!(now.stats, {
        let mut s = lord.stats;
        for (i, kind) in StatKind::GROWABLE.iter().enumerate() {
            s.set(*kind, s.get(*kind) + gains.1.0[i]);
        }
        s
    });
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

#[test]
fn the_screen_waits_for_the_level_up_and_leaves_the_battle_cores() {
    use Action::{Cancel, Confirm, CursorRight};
    let mut c = ctx();
    let state = skirmish(&c, 99, None);
    let (after, _) = expected(&state);
    let mut s = BattleScreen::new(state);
    press(&mut s, &mut c, &[Confirm, CursorRight, Confirm], 0.5);
    press(
        &mut s,
        &mut c,
        &[Confirm, Confirm, CursorRight, Confirm],
        0.0,
    );
    // During the playback: nothing shown yet.
    assert!(matches!(s.mode(), Mode::Combat(_)));
    assert!(s.progress().is_none());
    press(&mut s, &mut c, &[Cancel], EXP_S + 5.0);
    let p = s.progress().expect("the level-up page waits");
    assert!(matches!(p.page(), Some(Page::LevelUp(_))));
    // Rewind and the map keys wait too.
    press(&mut s, &mut c, &[Action::Rewind, Action::CursorLeft], 0.0);
    assert!(s.rewind().is_none());
    // So do tips and banners.
    c.tips_enabled = true;
    s.tips.fire(TipTrigger::FirstLevelUp);
    press(&mut s, &mut c, &[], 0.0);
    assert_eq!(s.shown_tip(), None);
    let banner = Event::PhaseStarted {
        turn: 2,
        phase: Phase::Player,
    };
    s.banners.extend(Banner::for_event(&banner));
    assert!(s.banner().is_none());
    // Cancel closes it, like Confirm.
    press(&mut s, &mut c, &[Cancel], 0.0);
    assert!(s.progress().is_none());
    assert!(s.banner().is_some());
    assert_eq!(s.state(), &after);
}

#[test]
fn auto_end_waits_for_the_playback_and_the_exp_bar() {
    use Action::{Cancel, Confirm, CursorRight, ToggleAutoEnd};
    let mut c = ctx();
    // The knight and the archer have acted: the lord is the last one.
    let mut state = skirmish(&c, 0, None);
    wait(&mut state, 1);
    wait(&mut state, 2);
    let mut s = BattleScreen::new(state);
    press(&mut s, &mut c, &[ToggleAutoEnd], 0.0);
    press(&mut s, &mut c, &[Confirm, CursorRight, Confirm], 0.5);
    press(
        &mut s,
        &mut c,
        &[Confirm, Confirm, CursorRight, Confirm],
        0.1,
    );
    assert!(matches!(s.mode(), Mode::Combat(_)));
    assert_eq!(s.state().phase(), Phase::Player, "not during the playback");
    press(&mut s, &mut c, &[Cancel], 0.1);
    assert!(s.progress().is_some());
    assert_eq!(s.state().phase(), Phase::Player, "not during the EXP bar");
    press(&mut s, &mut c, &[], EXP_S);
    assert!(s.progress().is_none());
    assert_ne!(s.state().phase(), Phase::Player);
}

#[test]
fn auto_end_waits_for_a_playback_with_no_exp_bar_after_it() {
    use Action::{Confirm, CursorRight, ToggleAutoEnd};
    let mut c = ctx();
    // The lord is at the level cap with its class mastered: it gains
    // nothing, so no EXP bar follows the combat.
    let mastered = ClassRecord {
        class_level: 10,
        class_points: 90,
    };
    let mut state = skirmish(&c, 0, Some(mastered));
    let cap = state.classes().level_cap;
    let (map, mut units) = (state.map().clone(), state.units().to_vec());
    units[0].level = cap;
    state = battle(&c, map, units);
    wait(&mut state, 1);
    wait(&mut state, 2);
    let (_, events) = expected(&state);
    assert!(!events.iter().any(|e| matches!(e, Event::ExpGained { .. })));
    let mut s = BattleScreen::new(state);
    press(&mut s, &mut c, &[ToggleAutoEnd], 0.0);
    press(&mut s, &mut c, &[Confirm, CursorRight, Confirm], 0.5);
    press(
        &mut s,
        &mut c,
        &[Confirm, Confirm, CursorRight, Confirm],
        0.1,
    );
    assert!(matches!(s.mode(), Mode::Combat(_)));
    assert!(s.progress.is_none());
    assert_eq!(s.state().phase(), Phase::Player, "not during the playback");
}

#[test]
fn switching_auto_end_on_later_doesnt_end_the_phase() {
    let mut c = ctx();
    let mut state = skirmish(&c, 0, None);
    wait(&mut state, 1);
    wait(&mut state, 2);
    let mut s = BattleScreen::new(state);
    // The last unit waits with auto-end off; turning it on afterwards
    // leaves the phase to the player.
    let lord = &s.state().units()[0];
    let cmd = Command::Act {
        unit: lord.id,
        dest: lord.pos,
        action: UnitAction::Wait,
    };
    s.apply(&cmd);
    press(&mut s, &mut c, &[], 0.1);
    press(&mut s, &mut c, &[Action::ToggleAutoEnd], 0.1);
    assert!(s.auto_end());
    assert_eq!(s.state().phase(), Phase::Player);
}

#[test]
fn mastery_shows_the_banner_after_the_exp_bar() {
    let c = ctx();
    // One CP short of mastering the Exile (10 CP per class level, cap 10).
    let record = ClassRecord {
        class_level: 9,
        class_points: 89,
    };
    let state = skirmish(&c, 0, Some(record));
    let (_, events) = expected(&state);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::ClassMastered { .. }))
    );
    let mut h = attacked(state);
    h.wait(EXP_S + 0.05);
    assert!(shows(&h, "Exile  CL 9 → 10"));
    assert!(shows(&h, "CLASS MASTERED!"));
    assert_eq!(help(&h), "f continue");
    assert_snapshot!("class_mastered", h.snapshot());
    h.keys("f");
    assert!(!shows(&h, "CLASS MASTERED!"));
}

#[test]
fn a_heal_goes_straight_to_the_exp_bar() {
    let mut c = ctx();
    let (map, mut units) = quick_units(&c);
    units[0].pos = Pos::new(6, 2);
    units[0].hp = 5;
    units[1].pos = Pos::new(6, 3);
    units[1].learned.insert(SpellId::new("heal"));
    let mut s = BattleScreen::new(battle(&c, map, units));
    let heal = Command::Act {
        unit: UnitId(2),
        dest: Pos::new(6, 3),
        action: UnitAction::Cast {
            spell: SpellId::new("heal"),
            target: CastTarget::Unit(UnitId(1)),
            active: None,
        },
    };
    s.apply(&heal);
    assert!(!matches!(s.mode(), Mode::Combat(_)));
    let p = s.progress().expect("the knight's EXP bar");
    let Some(Page::Exp(exp)) = p.page() else {
        panic!("{:?}", p.page())
    };
    assert_eq!((exp.name.as_str(), exp.from), ("Test Knight", 0));
    press(&mut s, &mut c, &[], EXP_S + 0.1);
    assert!(s.progress().is_none());
}
