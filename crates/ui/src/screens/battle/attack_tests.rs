//! Harness tests of attacking (ticket 0404): the weapon list, targeting,
//! the forecast against `core`'s numbers, the combat playback (Cancel skips, fast
//! forward, falls) and the battle it leaves.

use insta::assert_snapshot;
use trpg_core::{
    BattleState, Command, Equipped, Event, Pos, StatValue, UnitAction, UnitId, forecast,
};

use super::BattleScreen;
use super::forecast::{HP_ROW, LEFT_X, RIGHT_X, STRIKE_ROW};
use super::layout::{HELP_BAR, HELP_ROW};
use super::playback::{Beat, Playback, TIMINGS};
use super::testing::skirmish;
use crate::harness::{FRAME_DT, Harness};
use crate::screen::tests::ctx;

/// Where the lord attacks from.
const DEST: Pos = Pos::new(7, 2);

/// The brigand's tile's left cell (`test_small` is centred: tile `(x, y)`
/// starts at cell `(2 × (x + 10), y + 11)`).
const BRIGAND_CELL: (i32, i32) = (36, 13);

/// The skirmish in the harness.
fn harness(brigand_hp: StatValue) -> Harness {
    Harness::with_screen(Box::new(BattleScreen::new(skirmish(&ctx(), brigand_hp))))
}

/// Cells `x..x + w` of row `y`.
fn text(h: &Harness, x: i32, y: i32, w: i32) -> String {
    let buf = h.game().buffer();
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// Row `y`, trimmed.
fn row(h: &Harness, y: i32) -> String {
    text(h, 0, y, 100).trim().to_owned()
}

/// The side panel's row `y`, inside its border, trimmed.
fn panel(h: &Harness, y: i32) -> String {
    text(h, 71, y, 28).trim().to_owned()
}

fn help(h: &Harness) -> String {
    row(h, HELP_ROW)
}

/// The message line.
fn message(h: &Harness) -> String {
    row(h, HELP_BAR.y)
}

/// The lord selected, moved one step right to (7, 2) and its action menu
/// open (focused on `Attack`).
fn lord_menu(brigand_hp: StatValue) -> Harness {
    let mut h = harness(brigand_hp);
    h.keys("f Right f").wait(0.5);
    h
}

/// The lord targeting the brigand with its iron sword (slot 0).
fn lord_on_brigand(brigand_hp: StatValue) -> Harness {
    let mut h = lord_menu(brigand_hp);
    h.keys("f f Right");
    assert_eq!(panel(&h, 2), "Test Lord     Brigand");
    h
}

/// The lord's sword attack on the brigand applied to the skirmish directly:
/// the battle and events the screen should end up with.
fn expected(brigand_hp: StatValue) -> (BattleState, BattleState, Vec<Event>) {
    let before = skirmish(&ctx(), brigand_hp);
    let mut after = before.clone();
    let cmd = Command::Act {
        unit: UnitId(1),
        dest: DEST,
        action: UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: None,
            art: None,
        },
    };
    let events = after.apply(&cmd).unwrap_or_else(|e| panic!("{e}"));
    (before, after, events)
}

/// The playback the screen builds for [`expected`].
fn expected_playback(brigand_hp: StatValue) -> Playback {
    let (before, after, events) = expected(brigand_hp);
    Playback::new(&events, before.units(), after.fallen(), TIMINGS).expect("a combat")
}

/// Seconds of playback the `f` that confirms an attack plays: its press
/// frame (Confirm held: fast) and its release frame.
fn confirm_frames() -> f32 {
    FRAME_DT * TIMINGS.fast + FRAME_DT
}

#[test]
fn attack_offers_the_weapons_that_reach_then_cycles_targets_in_order() {
    let mut h = lord_menu(20);
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    // Two swords reach: the weapon list, with their stats.
    h.keys("f");
    let all = (0..30).map(|y| row(&h, y)).collect::<Vec<_>>().join("\n");
    assert!(
        all.contains("Iron Sword   Mt  5  Hit  90  Crit  0  Rng 1"),
        "{all}"
    );
    assert!(
        all.contains("Steel Sword  Mt  8  Hit  75  Crit  0  Rng 1"),
        "{all}"
    );
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    // Targets in (y, x) order: the raider at (7, 1), then the brigand.
    h.keys("f");
    assert_eq!(help(&h), "arrows next target · f attack · d back");
    assert_eq!(panel(&h, 2), "Test Lord     Raider");
    assert_eq!(panel(&h, 3), "Iron Sword    Steel Axe");
    for (keys, target) in [
        ("Right", "Brigand"),
        ("Right", "Raider"),
        ("Left", "Brigand"),
        ("Down", "Raider"),
        ("Up", "Brigand"),
        ("s", "Raider"),
        ("a", "Brigand"),
    ] {
        h.keys(keys);
        assert_eq!(panel(&h, 2), format!("Test Lord     {target}"), "{keys}");
    }
    // Back to the weapon list, the menu, the path.
    h.keys("d");
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    h.keys("Down f");
    assert_eq!(panel(&h, 3), "Steel Sword   Steel Axe");
    h.keys("d d d");
    assert_eq!(help(&h), "arrows move · f move here · d cancel");
}

#[test]
fn the_forecast_shows_cores_numbers() {
    let h = lord_on_brigand(20);
    // The forecast, from the combat maths directly.
    let state = skirmish(&ctx(), 20);
    let lord = state.unit(UnitId(1)).expect("lord");
    let brigand = state.unit(UnitId(4)).expect("brigand");
    let terrain = |pos| {
        let id = *state.map().tiles.get(pos).expect("on the map");
        state.terrain().get(id).expect("terrain")
    };
    let input = |u: &trpg_core::Unit, with: Option<&Equipped>, pos| {
        let class = state.classes().get(&u.class).expect("class");
        u.combat_input(
            class,
            state.classes(),
            state.items(),
            state.spells(),
            with,
            terrain(pos),
        )
    };
    let lord_in = input(lord, Some(&Equipped::Weapon(0)), DEST);
    let brigand_in = input(brigand, None, brigand.pos);
    let rules = state.items().combat_rules();
    let fc = forecast(&rules, &lord_in, &brigand_in, 1).expect("in range");
    let counter = fc.defender.expect("the brigand counters");
    // Hit and crit, both sides.
    let at = |x, y, w| text(&h, x, y, w).trim().to_owned();
    assert_eq!(at(LEFT_X + 5, HP_ROW + 1, 3), fc.attacker.hit.to_string());
    assert_eq!(at(LEFT_X + 5, HP_ROW + 2, 3), fc.attacker.crit.to_string());
    assert_eq!(at(RIGHT_X + 5, HP_ROW + 1, 3), counter.hit.to_string());
    assert_eq!(at(RIGHT_X + 5, HP_ROW + 2, 3), counter.crit.to_string());
    // The lord doubles with a sword: 1st strike, the counter, the
    // follow-up with the sword bonus.
    assert_eq!((fc.attacker.strikes, counter.strikes), (2, 1));
    assert!(fc.attacker.followup_damage > fc.attacker.damage);
    let dmg = |d: StatValue| format!("{d} dmg");
    assert_eq!(at(LEFT_X, STRIKE_ROW, 7), dmg(fc.attacker.damage));
    assert_eq!(at(RIGHT_X + 2, STRIKE_ROW + 1, 7), dmg(counter.damage));
    assert_eq!(
        at(LEFT_X, STRIKE_ROW + 2, 7),
        dmg(fc.attacker.followup_damage)
    );
    let total = fc.attacker.damage + fc.attacker.followup_damage;
    assert_eq!(at(LEFT_X, STRIKE_ROW + 4, 7), format!("{total} ×2"));
    assert_eq!(
        at(RIGHT_X + 2, STRIKE_ROW + 4, 7),
        format!("{} ×1", counter.damage)
    );
}

/// The lord targeting the brigand: both sides strike, the lord twice.
#[test]
fn forecast_with_a_double_and_a_counter_snapshot() {
    assert_snapshot!(lord_on_brigand(20).snapshot());
}

/// The archer, two tiles below the brigand: it hits twice and the brigand
/// can't counter. The brigand at 12 HP falls to the second arrow: a skull.
#[test]
fn forecast_with_no_counter_and_a_kill_snapshot() {
    let mut h = harness(12);
    // To the archer at (8, 4); it attacks from where it stands.
    h.keys("Right Right Down Down f f f");
    assert_eq!(panel(&h, 2), "Test Archer   Brigand");
    assert_eq!(text(&h, RIGHT_X + 2, STRIKE_ROW, 10), "no counter");
    assert_eq!(help(&h), "arrows next target · f attack · d back");
    assert_snapshot!(h.snapshot());
}

/// Mid-way through the first strike's result: the lord's name back to
/// normal, `HIT -8` (or `MISS`) under the brigand, its bar draining.
#[test]
fn playback_mid_strike_snapshot() {
    let mut h = lord_on_brigand(20);
    h.keys("f");
    let pb = expected_playback(20);
    let result = pb
        .steps()
        .iter()
        .find(|s| matches!(s.beat, Beat::Result { strike: 0, .. }))
        .expect("a first strike");
    h.wait(result.start + 0.1 - confirm_frames());
    assert_eq!(help(&h), "d skip · hold f fast");
    assert_snapshot!(h.snapshot());
}

#[test]
fn after_the_playback_the_defenders_hp_is_the_battles() {
    let (_, after, events) = expected(20);
    assert!(!events.iter().any(|e| matches!(e, Event::UnitFell { .. })));
    let brigand = after.unit(UnitId(4)).expect("still standing");
    let mut h = lord_on_brigand(20);
    h.keys("f");
    h.wait(expected_playback(20).total());
    // Browsing again, the cursor on the brigand.
    assert_eq!(
        help(&h),
        "arrows move · f range · e info · s next unit · d back"
    );
    let hp = format!("HP {}/{}", brigand.hp, brigand.stats.hp);
    assert!(panel(&h, 7).starts_with(&hp), "{}", panel(&h, 7));
    // The lord has acted: lowercase at (7, 2).
    assert_eq!(text(&h, 34, 13, 2), "lo");
    assert_eq!(message(&h), "");
}

#[test]
fn a_kill_fades_the_unit_out_with_a_message_and_removes_it() {
    let (_, after, events) = expected(3);
    assert!(events.contains(&Event::UnitFell { unit: UnitId(4) }));
    assert!(after.unit(UnitId(4)).is_none());
    let pb = expected_playback(3);
    let fall = pb
        .steps()
        .iter()
        .find(|s| matches!(s.beat, Beat::Fall { .. }))
        .expect("a fall");
    let mut h = lord_on_brigand(3);
    h.keys("f");
    // Just before the fall: the brigand at 0 HP, still drawn.
    h.wait(fall.start - 0.05 - confirm_frames());
    assert_eq!(text(&h, BRIGAND_CELL.0, BRIGAND_CELL.1, 2), "Br");
    assert_eq!(message(&h), "");
    // During it: the message.
    h.wait(0.3);
    assert_eq!(message(&h), "Brigand has fallen.");
    // Faded, while the box stays up: nobody on its tile in the panel.
    h.wait(fall.len);
    assert_eq!(help(&h), "d skip · hold f fast");
    assert_eq!(panel(&h, 5), "");
    // Afterwards: the terrain where it stood, nobody in the panel.
    h.wait(pb.total());
    assert_eq!(text(&h, BRIGAND_CELL.0, BRIGAND_CELL.1, 2), "..");
    assert_eq!(panel(&h, 5), "");
    assert_eq!(message(&h), "");
    assert_eq!(help(&h), "arrows move · s next unit · d back");
}

#[test]
fn cancel_skips_the_playback_and_the_next_keys_work() {
    let mut h = lord_on_brigand(20);
    // A Confirm tap doesn't skip (holding it only speeds up).
    h.keys("f f");
    assert_eq!(help(&h), "d skip · hold f fast");
    h.keys("d");
    // Browsing already: the keys after the skip move the cursor.
    assert_eq!(
        help(&h),
        "arrows move · f range · e info · s next unit · d back"
    );
    h.keys("Left");
    assert_eq!(panel(&h, 5), "Test Lord");
    h.keys("s");
    assert_eq!(panel(&h, 5), "Test Archer");
    h.keys("f");
    assert_eq!(help(&h), "arrows move · f move here · d cancel");
}

#[test]
fn holding_confirm_plays_four_times_as_fast() {
    let total = expected_playback(20).total();
    let quarter = total / TIMINGS.fast + 0.1;
    let mut slow = lord_on_brigand(20);
    slow.keys("f").wait(quarter);
    assert_eq!(help(&slow), "d skip · hold f fast");
    let mut fast = lord_on_brigand(20);
    fast.keys("f").hold("f", quarter);
    assert_eq!(
        help(&fast),
        "arrows move · f range · e info · s next unit · d back"
    );
    // The same battle either way: the brigand's HP matches.
    slow.wait(total);
    assert_eq!(panel(&slow, 7), panel(&fast, 7));
}
