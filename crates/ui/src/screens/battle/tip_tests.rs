//! Tests of the one-time tips on the battle screen (ticket 0406): each
//! trigger fires once per profile, tips take the keys until dismissed, and
//! their text names the keys the player has bound.

use insta::assert_snapshot;
use trpg_content::TipTrigger;
use trpg_core::{Command, Event, Objective, Phase, StatGains, UnitId};

use super::banner::PHASE_BANNER_S;
use super::testing::{battle_with, skirmish, skirmish_charged, through_ai_phases};
use super::*;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::input::Layout;
use crate::screen::tests::ctx;
use crate::tips::{TIPS_SEEN_KEY, reset_tips};

/// A context with tips on.
fn tipped() -> Ctx {
    let mut c = ctx();
    c.tips_enabled = true;
    c
}

fn quick() -> BattleScreen {
    BattleScreen::new(quick_battle(&ctx().content).unwrap())
}

fn render(screen: &BattleScreen, c: &Ctx) -> GlyphBuffer {
    let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank);
    screen.draw(c, &mut buf);
    buf
}

/// One frame of `dt` seconds with `actions`.
fn frame(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], dt: f32) {
    s.update(c, &FrameInput::new(actions.to_vec(), dt, vec![]));
}

/// One instant frame with `actions`.
fn press(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
    frame(s, c, actions, 0.0);
}

/// Whether some row of the rendered screen contains `text`.
fn shows(s: &BattleScreen, c: &Ctx, text: &str) -> bool {
    let buf = render(s, c);
    (0..i32::from(CONSOLE_H)).any(|y| {
        let row: String = (0..i32::from(CONSOLE_W))
            .map(|x| buf.get(x, y).map_or(' ', |cell| cell.glyph))
            .collect();
        row.contains(text)
    })
}

/// Dismisses every tip on screen, returning the triggers it took.
fn dismiss_all(s: &mut BattleScreen, c: &mut Ctx) -> Vec<TipTrigger> {
    let mut seen = Vec::new();
    for _ in 0..10 {
        press(s, c, &[]);
        let Some(t) = s.shown_tip() else {
            return seen;
        };
        seen.push(t);
        press(s, c, &[Action::Confirm]);
    }
    panic!("tips never ran out: {seen:?}");
}

#[test]
fn the_first_battle_shows_the_start_tip_and_confirm_dismisses_it() {
    let mut c = tipped();
    let mut s = quick();
    assert_eq!(s.shown_tip(), None);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstBattleStart));
    assert!(shows(&s, &c, "Your move"));
    assert_eq!(s.help(&c), "f close");
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.shown_tip(), None);
    assert!(!shows(&s, &c, "Your move"));
    assert_eq!(s.mode(), &Mode::default());
}

#[test]
fn the_start_tip_waits_for_the_first_player_phase_banner() {
    let mut c = tipped();
    let state = quick_battle(&c.content).unwrap();
    let events = [Event::PhaseStarted {
        turn: 1,
        phase: Phase::Player,
    }];
    // The banner closes by itself: the tip shows.
    let mut s = BattleScreen::start(state.clone(), &events);
    press(&mut s, &mut c, &[]);
    assert!(s.banner().is_some());
    assert_eq!(s.shown_tip(), None);
    assert!(!shows(&s, &c, "Your move"));
    frame(&mut s, &mut c, &[], PHASE_BANNER_S * 0.9);
    assert_eq!(s.shown_tip(), None);
    frame(&mut s, &mut c, &[], PHASE_BANNER_S * 0.2);
    assert_eq!(s.banner(), None);
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstBattleStart));
    assert!(shows(&s, &c, "Your move"));
    // The Confirm that closes the banner doesn't also close the tip.
    let mut c = tipped();
    let mut s = BattleScreen::start(state, &events);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.banner(), None);
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstBattleStart));
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.shown_tip(), None);
    assert_eq!(s.mode(), &Mode::default());
}

#[test]
fn cancel_dismisses_too_and_other_keys_wait() {
    let mut c = tipped();
    let mut s = quick();
    let start = s.cursor().pos;
    press(&mut s, &mut c, &[Action::CursorRight, Action::Confirm]);
    // Neither the cursor move nor the select got through, and the tip is
    // still up: the first action of the frame was swallowed, then the
    // Confirm dismissed it.
    assert_eq!(s.cursor().pos, start);
    assert_eq!(s.shown_tip(), None);
    assert_eq!(s.mode(), &Mode::default());
    // Cancel dismisses a tip as well.
    let mut c = tipped();
    let mut s = quick();
    press(&mut s, &mut c, &[]);
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!(s.shown_tip(), None);
    assert!(matches!(s.mode(), Mode::Idle { .. }), "{:?}", s.mode());
}

#[test]
fn a_tip_shows_once_per_profile() {
    let mut c = tipped();
    let mut s = quick();
    assert_eq!(dismiss_all(&mut s, &mut c), [TipTrigger::FirstBattleStart]);
    assert_eq!(
        c.storage.read(TIPS_SEEN_KEY).unwrap().as_deref(),
        Some("battle_start")
    );
    // Not again on this screen, nor on a new battle with the same storage.
    press(&mut s, &mut c, &[]);
    assert_eq!(s.shown_tip(), None);
    let mut again = quick();
    assert_eq!(dismiss_all(&mut again, &mut c), []);
    // Resetting the tips brings it back.
    reset_tips(&mut *c.storage).unwrap();
    let mut fresh = quick();
    assert_eq!(
        dismiss_all(&mut fresh, &mut c),
        [TipTrigger::FirstBattleStart]
    );
}

#[test]
fn tips_are_off_unless_enabled() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.shown_tip(), None);
    assert!(matches!(s.mode(), Mode::Selected(_)), "{:?}", s.mode());
    assert_eq!(c.storage.read(TIPS_SEEN_KEY).unwrap(), None);
}

#[test]
fn selecting_a_unit_shows_the_selection_tip_then_the_in_range_tip() {
    let mut c = tipped();
    let mut s = quick();
    dismiss_all(&mut s, &mut c);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Selected(_)));
    // The brigand at (7, 4) is within the lord's reach.
    assert_eq!(
        dismiss_all(&mut s, &mut c),
        [TipTrigger::FirstUnitSelected, TipTrigger::FirstEnemyInRange]
    );
    // Selecting again shows nothing.
    press(&mut s, &mut c, &[Action::Cancel]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(dismiss_all(&mut s, &mut c), []);
}

#[test]
fn no_in_range_tip_when_no_enemy_is_near() {
    let mut c = tipped();
    let quick = quick_battle(&c.content).unwrap();
    let mut units = quick.units().to_vec();
    for u in units.iter_mut().filter(|u| u.faction == Faction::Enemy) {
        u.pos = Pos::new(13, 0);
    }
    let rout = Objective::Rout { turn_limit: None };
    let mut s = BattleScreen::new(battle_with(&c, quick.map().clone(), units, rout));
    dismiss_all(&mut s, &mut c);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(dismiss_all(&mut s, &mut c), [TipTrigger::FirstUnitSelected]);
}

#[test]
fn the_forecast_shows_its_tip() {
    let mut c = tipped();
    let mut s = BattleScreen::new(skirmish(&c, 100));
    dismiss_all(&mut s, &mut c);
    // Select the lord, step right, open the menu: Attack, first weapon.
    press(&mut s, &mut c, &[Action::Confirm]);
    dismiss_all(&mut s, &mut c);
    press(&mut s, &mut c, &[Action::CursorRight, Action::Confirm]);
    frame(&mut s, &mut c, &[], 1.0);
    press(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Targeting(_)), "{:?}", s.mode());
    assert_eq!(dismiss_all(&mut s, &mut c), [TipTrigger::FirstForecast]);
    // The forecast is still there under the dismissed tip.
    assert!(matches!(s.mode(), Mode::Targeting(_)));
    // A tip waits out the combat that follows.
    press(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Combat(_)), "{:?}", s.mode());
    s.tips.fire(TipTrigger::FirstLowHp);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.shown_tip(), None);
}

#[test]
fn a_badly_hurt_unit_shows_the_low_hp_tip() {
    let mut c = tipped();
    let quick = quick_battle(&c.content).unwrap();
    let mut units = quick.units().to_vec();
    units[1].hp = 1;
    let rout = Objective::Rout { turn_limit: None };
    let mut s = BattleScreen::new(battle_with(&c, quick.map().clone(), units, rout));
    assert_eq!(
        dismiss_all(&mut s, &mut c),
        [TipTrigger::FirstBattleStart, TipTrigger::FirstLowHp]
    );
}

#[test]
fn a_player_level_up_shows_its_tip_but_an_enemys_does_not() {
    let mut c = tipped();
    let mut s = quick();
    dismiss_all(&mut s, &mut c);
    let up = |unit| Event::LeveledUp {
        unit: UnitId(unit),
        level: 2,
        gains: StatGains::default(),
    };
    s.note_level_ups(&[up(4)]);
    assert_eq!(dismiss_all(&mut s, &mut c), []);
    s.note_level_ups(&[up(1)]);
    assert_eq!(dismiss_all(&mut s, &mut c), [TipTrigger::FirstLevelUp]);
}

#[test]
fn resting_on_an_enemy_shows_the_danger_zone_tip() {
    let mut c = tipped();
    let mut s = quick();
    dismiss_all(&mut s, &mut c);
    s.cursor.jump(Pos::new(8, 2));
    assert_eq!(
        dismiss_all(&mut s, &mut c),
        [TipTrigger::DangerZoneAvailable]
    );
    assert!(!shows(&s, &c, "Danger zone"));
}

#[test]
fn the_enemy_phase_tip_shows_over_its_banner_and_holds_it() {
    let mut c = tipped();
    let mut s = quick();
    dismiss_all(&mut s, &mut c);
    press(&mut s, &mut c, &[Action::EndTurn]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstEnemyPhase));
    assert!(shows(&s, &c, "Enemy phase"));
    // The banner waits while the tip is up, however long the player reads.
    frame(&mut s, &mut c, &[], 30.0);
    assert_eq!(s.state().phase(), Phase::Enemy);
    assert!(s.banner().is_some());
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.shown_tip(), None);
    // Then the banner runs its course, the enemies act and the player
    // phase comes back.
    through_ai_phases(&mut s, &mut c, 30.0);
    assert_eq!(s.state().phase(), Phase::Player);
    assert_eq!(s.state().turn(), 2);
}

#[test]
fn tips_wait_for_the_player_phase_to_settle() {
    let mut c = tipped();
    let mut s = quick();
    dismiss_all(&mut s, &mut c);
    press(&mut s, &mut c, &[Action::EndTurn]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    // A unit found badly hurt while the enemy plays: the tip waits.
    s.tips.fire(TipTrigger::FirstLowHp);
    // (The enemy-phase tip is queued behind it and shows first.)
    let shown = dismiss_all(&mut s, &mut c);
    assert_eq!(shown, [TipTrigger::FirstEnemyPhase]);
    // The player phase's banner comes first; the tip waits for it.
    through_ai_phases(&mut s, &mut c, 30.0);
    assert_eq!(s.state().phase(), Phase::Player);
    assert!(s.banner().is_some());
    assert_eq!(s.shown_tip(), None);
    for _ in 0..4 {
        frame(&mut s, &mut c, &[], 30.0);
    }
    assert!(s.banner().is_none());
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstLowHp));
}

#[test]
fn tips_wait_while_the_rewind_screen_is_open() {
    let mut c = tipped();
    let mut s = BattleScreen::new(skirmish_charged(&c, 100, 3));
    dismiss_all(&mut s, &mut c);
    press(&mut s, &mut c, &[Action::Rewind]);
    assert!(s.rewind().is_some());
    s.tips.fire(TipTrigger::FirstLowHp);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.shown_tip(), None);
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstLowHp));
}

/// A battle like the Quick Battle, with `edit` applied to its units.
fn edited(c: &Ctx, edit: impl FnOnce(&mut Vec<Unit>)) -> BattleScreen {
    let quick = quick_battle(&c.content).unwrap();
    let mut units = quick.units().to_vec();
    edit(&mut units);
    let rout = Objective::Rout { turn_limit: None };
    BattleScreen::new(battle_with(c, quick.map().clone(), units, rout))
}

#[test]
fn low_hp_is_half_of_max_hp_or_less_of_a_unit_still_standing() {
    let low = |hp| {
        let mut c = tipped();
        let mut s = edited(&c, |units| {
            units[1].stats.hp = 20;
            units[1].hp = hp;
        });
        dismiss_all(&mut s, &mut c).contains(&TipTrigger::FirstLowHp)
    };
    assert!(low(10));
    assert!(!low(11));
    assert!(!low(0));
    // An enemy's low HP is not a player's worry.
    let mut c = tipped();
    let mut s = edited(&c, |units| units[3].hp = 1);
    assert_eq!(dismiss_all(&mut s, &mut c), [TipTrigger::FirstBattleStart]);
}

#[test]
fn the_start_tip_is_for_the_first_turn_of_the_player_phase_only() {
    let mut c = tipped();
    let mut state = quick_battle(&c.content).unwrap();
    state.apply(&Command::EndPhase).unwrap();
    assert_eq!((state.turn(), state.phase()), (1, Phase::Enemy));
    let mut s = BattleScreen::new(state.clone());
    assert_eq!(dismiss_all(&mut s, &mut c), [TipTrigger::FirstEnemyPhase]);
    // Nor does a start tip wait behind it for the next player phase.
    through_ai_phases(&mut s, &mut c, 30.0);
    for _ in 0..5 {
        frame(&mut s, &mut c, &[], 30.0);
    }
    assert_eq!(s.state().phase(), Phase::Player);
    let later = dismiss_all(&mut s, &mut c);
    assert!(!later.contains(&TipTrigger::FirstBattleStart), "{later:?}");
    state.apply(&Command::EndPhase).unwrap();
    assert_eq!((state.turn(), state.phase()), (2, Phase::Player));
    let mut s = BattleScreen::new(state);
    assert_eq!(dismiss_all(&mut s, &mut c), []);
}

#[test]
fn a_tip_names_the_keys_the_player_has_bound() {
    let mut c = tipped();
    c.use_layout(Layout::LeftHanded);
    let mut s = quick();
    press(&mut s, &mut c, &[]);
    assert!(shows(&s, &c, "Steer the cursor with wasd"));
    assert!(shows(&s, &c, "Press j on one of"));
    assert_eq!(s.help(&c), "j close");
}

#[test]
fn a_tip_shows_not_mapped_for_an_action_with_no_key() {
    let mut c = tipped();
    let mut keys = c.layout_bindings(Layout::RightHanded);
    keys.clear(Action::NextUnit, 0);
    c.set_layout_bindings(Layout::RightHanded, keys).unwrap();
    let mut s = quick();
    press(&mut s, &mut c, &[]);
    assert!(shows(&s, &c, "your units to select it. ! not mapped jumps"));
}

#[test]
fn the_start_tip_renders() {
    let mut c = tipped();
    let mut s = quick();
    press(&mut s, &mut c, &[]);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}
