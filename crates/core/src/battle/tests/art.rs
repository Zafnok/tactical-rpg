//! Combat Arts in battle (ticket 0312): one test per art of
//! [`test_arts`] (the Chapter 1 arts of `combat-arts.md`), costs, breaking,
//! debuff and stance expiry, Line Pierce, bosses, weapon EXP and the
//! preview.
//!
//! Weapons here have their kind's trait ([`art_setup`]); units are the test
//! units (10 HP, 0 in every other stat, Mov 3) at rank E unless
//! [`ranked`].

use super::*;
use crate::art::{ArtNote, Debuff};
use crate::combat::{CombatHp, SideForecast, if_all_hit};
use crate::skill::{CostError, TimedEffect};

fn aid(id: &str) -> ArtId {
    ArtId::new(id)
}

/// A test weapon of `kind`: might `might`, hit 100, crit 0, weight 0,
/// range `min..=max`, durability 20.
fn typed(kind: WeaponKind, min: u32, max: u32, might: StatValue) -> WeaponDef {
    WeaponDef {
        kind,
        might,
        min_range: min,
        max_range: max,
        ..sword("typed")
    }
}

/// `blade` (sword 3), `big_blade` (sword 10), `moon_blade` (sword 3 with
/// the `riposte` weapon art), `pike` (spear 3), `long_pike` (spear 3, range
/// 1–2), `far_pike` (spear 3, range 1–3), `hatchet` (axe 1), `heavy_axe` (axe 3), `longbow` (bow 3, range
/// 2), `knuckles` (gauntlet 3).
fn art_weapons() -> Vec<(&'static str, WeaponDef)> {
    use WeaponKind::{Axe, Bow, Gauntlet, Spear, Sword};
    vec![
        ("blade", typed(Sword, 1, 1, 3)),
        ("big_blade", typed(Sword, 1, 1, 10)),
        (
            "moon_blade",
            WeaponDef {
                arts: vec![aid("riposte")],
                ..typed(Sword, 1, 1, 3)
            },
        ),
        ("pike", typed(Spear, 1, 1, 3)),
        ("long_pike", typed(Spear, 1, 2, 3)),
        ("far_pike", typed(Spear, 1, 3, 3)),
        ("hatchet", typed(Axe, 1, 1, 1)),
        ("heavy_axe", typed(Axe, 1, 1, 3)),
        ("longbow", typed(Bow, 2, 2, 3)),
        ("knuckles", typed(Gauntlet, 1, 1, 3)),
    ]
}

/// The test classes plus `cavalier` (Mounted) and `knight` (Armored).
fn art_classes() -> ClassTable {
    let mut table = classes();
    for (id, tag) in [("cavalier", UnitTag::Mounted), ("knight", UnitTag::Armored)] {
        let c = class(id, UnitTags::from_tags(&[tag]));
        table.classes.insert(c.id.clone(), c);
    }
    table
}

/// [`setup`] with [`art_weapons`], every kind's trait and [`art_classes`].
fn art_setup(units: Vec<Unit>) -> BattleSetup {
    let mut items = items_for(&units);
    for (id, def) in art_weapons() {
        items.items.insert(item(id), ItemDef::Weapon(def));
    }
    let rules = crate::combat::CombatRules::default();
    items.traits = [
        WeaponKind::Sword,
        WeaponKind::Spear,
        WeaponKind::Axe,
        WeaponKind::Bow,
        WeaponKind::Gauntlet,
    ]
    .into_iter()
    .map(|k| (k, rules.type_trait(k)))
    .collect();
    BattleSetup {
        items: Arc::new(items),
        classes: Arc::new(art_classes()),
        ..setup(units)
    }
}

fn battle(units: Vec<Unit>) -> BattleState {
    start(art_setup(units))
}

/// `u` carrying only weapon `id`, equipped, at full durability.
fn with(u: Unit, id: &str) -> Unit {
    carrying(u, &[item(id)])
}

/// `u` at `rank` in every weapon kind.
fn ranked(mut u: Unit, rank: WeaponRank) -> Unit {
    for kind in [
        WeaponKind::Sword,
        WeaponKind::Spear,
        WeaponKind::Axe,
        WeaponKind::Bow,
        WeaponKind::Gauntlet,
    ] {
        u.weapon_ranks.insert(kind, rank);
    }
    u
}

/// A rank-D player unit `id` at `pos` with weapon `weapon`.
fn artist(id: u32, pos: Pos, weapon: &str) -> Unit {
    ranked(with(unit(id, Faction::Player, pos), weapon), WeaponRank::D)
}

/// `u` with these stats (HP, Str, Mag, Dex, Spd, Def, Res; Mov 3).
fn stats(u: Unit, v: [StatValue; 7]) -> Unit {
    Unit {
        stats: Stats::from_growable(v, 3),
        hp: v[0],
        ..u
    }
}

fn in_class(u: Unit, class: &str) -> Unit {
    Unit {
        class: ClassId(class.into()),
        ..u
    }
}

fn set_durability(mut u: Unit, left: u32) -> Unit {
    if let Some(Some(w)) = u.loadout.weapons.first_mut() {
        w.durability_left = left;
    }
    u
}

fn art_attack(target: u32, art: &str) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
        active: None,
        art: Some(aid(art)),
    }
}

fn names(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|e| {
            let text = format!("{e:?}");
            text.split([' ', '{', '(']).next().unwrap_or("").to_owned()
        })
        .collect()
}

/// Every combat in `events`: `(defender, forecast, outcome)`.
fn combats(events: &[Event]) -> Vec<(UnitId, Forecast, CombatOutcome)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::CombatResolved {
                defender,
                forecast,
                outcome,
                ..
            } => Some((*defender, *forecast, outcome.clone())),
            _ => None,
        })
        .collect()
}

fn preview(s: &BattleState, id: u32, dest: Pos, action: &UnitAction) -> AttackPreview {
    s.preview_attack(UnitId(id), dest, action)
        .unwrap_or_else(|e| panic!("{e}"))
}

fn hp(s: &BattleState, id: u32) -> StatValue {
    s.unit(UnitId(id)).map_or(0, |u| u.hp)
}

fn left(s: &BattleState, id: u32) -> u32 {
    s.unit(UnitId(id))
        .and_then(|u| u.loadout.weapon(0))
        .map_or(0, |w| w.durability_left)
}

fn art_applied(unit: u32, art: &str, until: Phase) -> Event {
    Event::EffectApplied {
        unit: UnitId(unit),
        source: EffectSource::Art(aid(art)),
        until,
    }
}

fn weapon_exp(events: &[Event], unit: u32) -> Option<u32> {
    events.iter().find_map(|e| match e {
        Event::WeaponExpGained {
            unit: u, amount, ..
        } if *u == UnitId(unit) => Some(*amount),
        _ => None,
    })
}

// ---- Sword -----------------------------------------------------------------

#[test]
fn flowing_cut_raises_the_sword_follow_up_to_three_halves() {
    // Spd 4 against 0: two strikes. Might 10 against Def 0: 10, then 12
    // normally (×6/5) and 15 with the art (×3/2).
    let s = battle(vec![
        stats(artist(1, p(0, 0), "big_blade"), [30, 0, 0, 0, 4, 0, 0]),
        stats(unit(3, Faction::Enemy, p(1, 0)), [40, 0, 0, 0, 0, 0, 0]),
    ]);
    let plain = preview(&s, 1, p(0, 0), &attack(3));
    let art = preview(&s, 1, p(0, 0), &art_attack(3, "flowing_cut"));
    assert_eq!(
        (
            plain.forecast.attacker.damage,
            plain.forecast.attacker.followup_damage
        ),
        (10, 12)
    );
    assert_eq!(
        art.forecast.attacker,
        SideForecast {
            followup_damage: 15,
            ..plain.forecast.attacker
        }
    );
    assert_eq!(art.forecast.defender, plain.forecast.defender);
    assert_eq!(art.durability, Some((20, 18)));
    assert_eq!(art.notes, []);
}

#[test]
fn guard_break_adds_hit_and_stops_the_counter() {
    // Enemy Spd 10: avoid 20, so hit 80 without the art.
    let mut s = battle(vec![
        artist(1, p(0, 0), "blade"),
        stats(unit(3, Faction::Enemy, p(1, 0)), [20, 0, 0, 0, 10, 0, 0]),
    ]);
    let plain = preview(&s, 1, p(0, 0), &attack(3));
    let art = preview(&s, 1, p(0, 0), &art_attack(3, "guard_break"));
    assert_eq!(
        (plain.forecast.attacker.hit, art.forecast.attacker.hit),
        (80, 90)
    );
    assert!(plain.forecast.defender.is_some());
    assert_eq!(art.forecast.defender, None);
    assert_eq!(art.art, Some(aid("guard_break")));
    assert_eq!(art.durability, Some((20, 16)));
    assert_eq!(art.notes, [ArtNote::NoCounter]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "guard_break"));
    assert_eq!(
        events[..2],
        [
            Event::ArtUsed {
                unit: UnitId(1),
                art: aid("guard_break"),
                weapon: item("blade"),
                durability_before: 20,
                durability_after: 16,
            },
            Event::DurabilitySpent {
                unit: UnitId(1),
                slot: 0,
                item: item("blade"),
                amount: 4,
                left: 16,
            },
        ]
    );
    assert_eq!(names(&events)[2], "CombatResolved");
    let (_, forecast, outcome) = &combats(&events)[0];
    assert_eq!(*forecast, art.forecast);
    assert!(outcome.strikes.iter().all(|s| s.by == Side::Attacker));
    assert_eq!((left(&s, 1), hp(&s, 1)), (16, 10));
}

// ---- Spear -----------------------------------------------------------------

#[test]
fn unhorse_is_times_three_against_mounted_and_plain_against_others() {
    let s = battle(vec![
        with(unit(1, Faction::Player, p(0, 0)), "pike"),
        stats(
            in_class(unit(3, Faction::Enemy, p(1, 0)), "cavalier"),
            [20, 0, 0, 0, 10, 0, 0],
        ),
        stats(unit(4, Faction::Enemy, p(0, 1)), [20, 0, 0, 0, 10, 0, 0]),
    ]);
    let numbers = |target, art: bool| {
        let action = if art {
            art_attack(target, "unhorse")
        } else {
            attack(target)
        };
        let a = preview(&s, 1, p(0, 0), &action).forecast.attacker;
        (a.damage, a.hit, a.effective)
    };
    // Might 3: ×2 against Mounted (the spear's trait), ×3 with the art.
    assert_eq!(numbers(3, false), (6, 80, true));
    assert_eq!(numbers(3, true), (9, 90, true));
    assert_eq!(numbers(4, false), (3, 80, false));
    assert_eq!(numbers(4, true), (3, 90, false));
}

#[test]
fn line_pierce_strikes_the_unit_behind_the_target() {
    // Str 2: 5 damage a strike.
    let mut s = battle(vec![
        stats(artist(1, p(0, 0), "pike"), [10, 2, 0, 0, 0, 0, 0]),
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(3, 0)),
    ]);
    let action = art_attack(3, "line_pierce");
    let shown = preview(&s, 1, p(1, 0), &action);
    assert_eq!(shown.notes, [ArtNote::Pierces]);
    assert_eq!(
        shown.pierce.map(|(id, f)| (id, f.damage, f.strikes)),
        Some((UnitId(4), 5, 1))
    );
    let events = act(&mut s, 1, p(1, 0), action);
    let fights = combats(&events);
    assert_eq!(fights.len(), 2);
    let (victim, forecast, outcome) = &fights[1];
    assert_eq!(*victim, UnitId(4));
    assert_eq!(forecast.defender, None);
    assert_eq!(forecast.attacker.strikes, 1);
    assert_eq!(outcome.strikes.len(), 1);
    // The target took a strike and countered; the unit behind took one.
    assert_eq!((hp(&s, 3), hp(&s, 4), hp(&s, 1)), (5, 5, 7));
    // One weapon EXP award: base 4, dealt 5 + 5 = 10 → +2.
    assert_eq!(weapon_exp(&events, 1), Some(6));
    // The pierce comes after the combat, before weapon EXP.
    let n = names(&events);
    let at = |name: &str| n.iter().position(|x| x == name).unwrap();
    assert!(at("CombatResolved") < at("WeaponExpGained"));
    assert_eq!(
        n.iter().rposition(|x| x == "CombatResolved"),
        Some(at("WeaponExpGained") - 1)
    );
}

#[test]
fn a_pierce_is_a_second_combat_for_unit_exp_and_class_points() {
    let units = vec![
        stats(artist(1, p(0, 0), "pike"), [10, 2, 0, 0, 0, 0, 0]),
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(3, 0)),
    ];
    let mut s = start(BattleSetup {
        classes: Arc::new(leveling(art_classes())),
        ..art_setup(units)
    });
    let events = act(&mut s, 1, p(1, 0), art_attack(3, "line_pierce"));
    let awards: Vec<(u32, bool)> = events
        .iter()
        .filter_map(|e| match e {
            Event::ExpGained { amount, .. } => Some((*amount, true)),
            Event::ClassPointsGained { amount, .. } => Some((*amount, false)),
            _ => None,
        })
        .collect();
    // Both units took damage and stand: a damage award for each combat.
    assert_eq!(awards, [(20, true), (2, false), (20, true), (2, false)]);
}

#[test]
fn a_pierce_that_hits_counts_as_a_hit_for_weapon_exp() {
    // The target dodges everything (avoid 120) and strikes 4 times; the
    // attacker survives and the pierce hits.
    let mut s = battle(vec![
        stats(artist(1, p(0, 0), "pike"), [30, 0, 0, 0, 0, 0, 0]),
        stats(unit(3, Faction::Enemy, p(1, 0)), [10, 0, 0, 0, 60, 0, 0]),
        unit(4, Faction::Enemy, p(2, 0)),
    ]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "line_pierce"));
    assert_eq!((hp(&s, 3), hp(&s, 4)), (10, 7));
    // Base 4 (a strike hit), dealt 3 → +0.
    assert_eq!(weapon_exp(&events, 1), Some(4));
}

#[test]
fn only_line_pierce_pierces() {
    let mut s = battle(vec![
        artist(1, p(0, 0), "blade"),
        unit(3, Faction::Enemy, p(1, 0)),
        unit(4, Faction::Enemy, p(2, 0)),
    ]);
    let action = art_attack(3, "guard_break");
    assert_eq!(preview(&s, 1, p(0, 0), &action).pierce, None);
    let events = act(&mut s, 1, p(0, 0), action);
    assert_eq!(combats(&events).len(), 1);
    assert_eq!(hp(&s, 4), 10);
}

/// The unit Line Pierce would strike: unit 1 with `weapon` at `(0,0)`
/// attacking unit 3 at `target`, with enemies 4.. at `others`.
fn pierced(weapon: &str, target: Pos, others: &[Pos]) -> Option<UnitId> {
    let mut units = vec![artist(1, p(0, 0), weapon), unit(3, Faction::Enemy, target)];
    units.extend(
        (4..)
            .zip(others)
            .map(|(id, &pos)| unit(id, Faction::Enemy, pos)),
    );
    preview(&battle(units), 1, p(0, 0), &art_attack(3, "line_pierce"))
        .pierce
        .map(|(id, _)| id)
}

#[test]
fn line_pierce_follows_straight_and_diagonal_lines() {
    // Straight at distance 2: the next tile on, not 2 further.
    assert_eq!(
        pierced("long_pike", p(0, 2), &[p(0, 3), p(0, 4)]),
        Some(UnitId(4))
    );
    assert_eq!(pierced("long_pike", p(0, 2), &[p(0, 4)]), None);
    // Diagonal (a range-2 attack, Nick): the next diagonal tile.
    assert_eq!(
        pierced("long_pike", p(1, 1), &[p(2, 2), p(2, 1), p(1, 2)]),
        Some(UnitId(4))
    );
    // Any other angle (range 3) has no tile behind.
    assert_eq!(
        pierced("far_pike", p(1, 2), &[p(2, 3), p(2, 4), p(1, 3)]),
        None
    );
    // Towards the map's top-left too.
    let mut s = battle(vec![
        artist(1, p(3, 3), "long_pike"),
        unit(3, Faction::Enemy, p(2, 2)),
        unit(4, Faction::Enemy, p(1, 1)),
    ]);
    let events = act(&mut s, 1, p(3, 3), art_attack(3, "line_pierce"));
    let fights = combats(&events);
    assert_eq!(fights.len(), 2);
    assert_eq!(fights[1].0, UnitId(4));
}

#[test]
fn line_pierce_needs_a_hostile_unit_behind() {
    // An ally behind is never struck; nor is an empty tile.
    let ally = battle(vec![
        artist(1, p(0, 0), "pike"),
        unit(2, Faction::Player, p(2, 0)),
        unit(3, Faction::Enemy, p(1, 0)),
    ]);
    assert_eq!(
        preview(&ally, 1, p(0, 0), &art_attack(3, "line_pierce")).pierce,
        None
    );
    let empty = battle(vec![
        artist(1, p(0, 0), "pike"),
        unit(3, Faction::Enemy, p(1, 0)),
    ]);
    let mut copy = empty.clone();
    let events = act(&mut copy, 1, p(0, 0), art_attack(3, "line_pierce"));
    assert_eq!(combats(&events).len(), 1);
    // The attacker's own old tile is behind the target once it has moved.
    let mut runner = artist(1, p(2, 0), "pike");
    runner.stats.mov = 5;
    let moved = battle(vec![runner, unit(3, Faction::Enemy, p(1, 0))]);
    assert_eq!(
        preview(&moved, 1, p(0, 0), &art_attack(3, "line_pierce")).pierce,
        None
    );
}

#[test]
fn line_pierce_still_strikes_after_the_target_falls() {
    let mut s = battle(vec![
        artist(1, p(0, 0), "pike"),
        Unit {
            hp: 3,
            ..unit(3, Faction::Enemy, p(1, 0))
        },
        Unit {
            hp: 3,
            ..unit(4, Faction::Enemy, p(2, 0))
        },
        unit(5, Faction::Enemy, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "line_pierce"));
    assert_eq!(combats(&events).len(), 2);
    let fell: Vec<&Event> = events
        .iter()
        .filter(|e| matches!(e, Event::UnitFell { .. }))
        .collect();
    assert_eq!(
        fell,
        [
            &Event::UnitFell { unit: UnitId(3) },
            &Event::UnitFell { unit: UnitId(4) },
        ]
    );
}

#[test]
fn line_pierce_needs_the_attacker_standing() {
    // The target's counter fells the attacker: no pierce.
    let mut s = battle(vec![
        Unit {
            hp: 3,
            ..artist(1, p(0, 0), "pike")
        },
        unit(2, Faction::Player, p(7, 4)),
        stats(unit(3, Faction::Enemy, p(1, 0)), [20, 0, 0, 0, 0, 0, 0]),
        unit(4, Faction::Enemy, p(2, 0)),
    ]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "line_pierce"));
    assert_eq!(combats(&events).len(), 1);
    assert_eq!(hp(&s, 4), 10);
    assert!(events.contains(&Event::UnitFell { unit: UnitId(1) }));
}

// ---- Axe -------------------------------------------------------------------

#[test]
fn crushing_swing_adds_hit_and_raises_the_axe_minimum() {
    // Might 1 against Def 5: 0, raised to the axe minimum.
    let s = battle(vec![
        with(unit(1, Faction::Player, p(0, 0)), "hatchet"),
        stats(unit(3, Faction::Enemy, p(1, 0)), [20, 0, 0, 0, 10, 5, 0]),
    ]);
    let plain = preview(&s, 1, p(0, 0), &attack(3)).forecast.attacker;
    let art = preview(&s, 1, p(0, 0), &art_attack(3, "crushing_swing"))
        .forecast
        .attacker;
    assert_eq!((plain.damage, plain.hit), (5, 80));
    assert_eq!((art.damage, art.hit), (8, 100));
}

#[test]
fn armor_cleave_is_effective_against_armored() {
    let s = battle(vec![
        artist(1, p(0, 0), "heavy_axe"),
        in_class(unit(3, Faction::Enemy, p(1, 0)), "knight"),
        unit(4, Faction::Enemy, p(0, 1)),
    ]);
    let numbers = |action: UnitAction| {
        let a = preview(&s, 1, p(0, 0), &action).forecast.attacker;
        (a.damage, a.effective)
    };
    // Might 3, but never below the axe minimum 5; ×2 against Armored.
    assert_eq!(numbers(attack(3)), (5, false));
    assert_eq!(numbers(art_attack(3, "armor_cleave")), (6, true));
    assert_eq!(numbers(art_attack(4, "armor_cleave")), (5, false));
}

// ---- Bow -------------------------------------------------------------------

#[test]
fn close_shot_lets_a_bow_shoot_an_adjacent_enemy() {
    let mut s = battle(vec![
        with(unit(1, Faction::Player, p(0, 0)), "longbow"),
        unit(3, Faction::Enemy, p(1, 0)),
    ]);
    refused_act(
        &mut s,
        1,
        p(0, 0),
        attack(3),
        CommandError::OutOfRange {
            target: UnitId(3),
            distance: 1,
        },
    );
    let near = preview(&s, 1, p(0, 0), &art_attack(3, "close_shot")).forecast;
    assert_eq!((near.attacker.hit, near.attacker.damage), (90, 3));
    // The enemy's sword counters the adjacent archer.
    assert!(near.defender.is_some());
    // From 2 tiles it just costs hit.
    let far = preview(&s, 1, p(0, 1), &art_attack(3, "close_shot")).forecast;
    assert_eq!((far.attacker.hit, far.defender), (90, None));
    act(&mut s, 1, p(0, 0), art_attack(3, "close_shot"));
    assert_eq!(left(&s, 1), 18);
}

#[test]
fn pinning_shot_lowers_mov_until_the_end_of_the_targets_next_phase() {
    let mut s = battle(vec![
        artist(1, p(0, 0), "longbow"),
        unit(3, Faction::Enemy, p(2, 0)),
    ]);
    let shown = preview(&s, 1, p(0, 0), &art_attack(3, "pinning_shot"));
    let pin = Debuff {
        stat: StatKind::Mov,
        amount: 3,
    };
    assert_eq!(shown.notes, [ArtNote::Debuff(pin)]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "pinning_shot"));
    let n = names(&events);
    let applied = n.iter().position(|x| x == "EffectApplied").unwrap();
    assert!(n.iter().position(|x| x == "CombatResolved").unwrap() < applied);
    assert_eq!(
        events[applied],
        art_applied(3, "pinning_shot", Phase::Other)
    );
    let target = s.unit(UnitId(3)).unwrap();
    assert_eq!((target.move_points(), target.stats.mov), (0, 3));
    let reach = reachable(s.map(), s.terrain(), s.classes(), s.units(), UnitId(3)).unwrap();
    assert_eq!(reach.stoppable().len(), 1);
    // Still pinned through the enemy phase…
    let events = end(&mut s);
    assert_eq!(events, [started(1, Phase::Enemy)]);
    assert_eq!(s.unit(UnitId(3)).unwrap().move_points(), 0);
    // …and free once it ends.
    let events = end(&mut s);
    assert_eq!(
        events,
        [
            Event::EffectExpired {
                unit: UnitId(3),
                source: EffectSource::Art(aid("pinning_shot")),
            },
            started(2, Phase::Player),
        ]
    );
    assert_eq!(s.unit(UnitId(3)).unwrap().move_points(), 3);
}

#[test]
fn a_debuff_needs_a_hit_and_a_standing_target_and_refreshes() {
    // Every strike misses (avoid 120): no debuff, the cost is still paid.
    let mut s = battle(vec![
        artist(1, p(0, 0), "longbow"),
        stats(unit(3, Faction::Enemy, p(2, 0)), [10, 0, 0, 0, 60, 0, 0]),
    ]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "pinning_shot"));
    assert!(!names(&events).contains(&"EffectApplied".to_owned()));
    assert_eq!((left(&s, 1), hp(&s, 3)), (17, 10));
    // A target that falls gets nothing.
    let mut s = battle(vec![
        artist(1, p(0, 0), "longbow"),
        Unit {
            hp: 3,
            ..unit(3, Faction::Enemy, p(2, 0))
        },
        unit(4, Faction::Enemy, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "pinning_shot"));
    assert!(!names(&events).contains(&"EffectApplied".to_owned()));
    // A second pin refreshes the first.
    let mut s = battle(vec![
        artist(1, p(0, 0), "longbow"),
        artist(2, p(0, 4), "longbow"),
        unit(3, Faction::Enemy, p(2, 2)),
    ]);
    act(&mut s, 1, p(0, 2), art_attack(3, "pinning_shot"));
    act(&mut s, 2, p(2, 4), art_attack(3, "pinning_shot"));
    let target = s.unit(UnitId(3)).unwrap();
    assert_eq!((target.effects.len(), target.move_points()), (1, 0));
}

#[test]
fn a_boss_debuff_on_a_player_unit_lasts_through_the_next_player_phase() {
    let mut s = battle(vec![
        lord(1, p(0, 0)),
        Unit {
            role: Role::Boss,
            ..ranked(
                with(unit(3, Faction::Enemy, p(2, 0)), "longbow"),
                WeaponRank::D,
            )
        },
    ]);
    end(&mut s);
    let events = act(&mut s, 3, p(2, 0), art_attack(1, "pinning_shot"));
    assert!(events.contains(&art_applied(1, "pinning_shot", Phase::Enemy)));
    // Through the enemy phase's end and the whole player phase.
    end(&mut s);
    assert_eq!(s.unit(UnitId(1)).unwrap().move_points(), 0);
    let events = end(&mut s);
    assert_eq!(
        events.first(),
        Some(&Event::EffectExpired {
            unit: UnitId(1),
            source: EffectSource::Art(aid("pinning_shot")),
        })
    );
}

// ---- Gauntlet --------------------------------------------------------------

#[test]
fn pressure_point_lowers_spd_for_later_combats_not_below_zero() {
    // Unit 2 has Spd 6 against the target's 5: one strike, two once the
    // target is slowed to 2.
    let mut s = battle(vec![
        artist(1, p(0, 0), "knuckles"),
        stats(
            with(unit(2, Faction::Player, p(0, 2)), "blade"),
            [10, 0, 0, 0, 6, 0, 0],
        ),
        stats(unit(3, Faction::Enemy, p(1, 1)), [30, 0, 0, 0, 5, 0, 0]),
    ]);
    assert_eq!(
        preview(&s, 2, p(1, 2), &attack(3))
            .forecast
            .attacker
            .strikes,
        1
    );
    let shown = preview(&s, 1, p(1, 0), &art_attack(3, "pressure_point"));
    assert_eq!(
        shown.notes,
        [ArtNote::Debuff(Debuff {
            stat: StatKind::Spd,
            amount: 3,
        })]
    );
    let events = act(&mut s, 1, p(1, 0), art_attack(3, "pressure_point"));
    assert!(events.contains(&art_applied(3, "pressure_point", Phase::Other)));
    assert_eq!(
        preview(&s, 2, p(1, 2), &attack(3))
            .forecast
            .attacker
            .strikes,
        2
    );
    // Slowed through the enemy phase, back to Spd 5 once it ends.
    end(&mut s);
    assert_eq!(s.unit(UnitId(3)).map(|u| u.effects.len()), Some(1));
    let events = end(&mut s);
    assert!(events.contains(&Event::EffectExpired {
        unit: UnitId(3),
        source: EffectSource::Art(aid("pressure_point")),
    }));
    assert_eq!(
        preview(&s, 2, p(1, 2), &attack(3))
            .forecast
            .attacker
            .strikes,
        1
    );
    // Spd 2 − 3 stops at 0.
    let mut s = battle(vec![
        artist(1, p(0, 0), "knuckles"),
        stats(unit(3, Faction::Enemy, p(1, 0)), [30, 0, 0, 0, 2, 0, 0]),
    ]);
    act(&mut s, 1, p(0, 0), art_attack(3, "pressure_point"));
    let target = s.unit(UnitId(3)).unwrap();
    let slowed = crate::skill::effect_bonuses(&target.effects).apply(target.stats);
    assert_eq!(slowed.spd, 0);
}

#[test]
fn sidestep_is_a_stance_from_this_combat_until_the_next_own_phase() {
    let mut s = battle(vec![
        artist(1, p(0, 0), "knuckles"),
        unit(3, Faction::Enemy, p(1, 0)),
        unit(4, Faction::Enemy, p(2, 1)),
    ]);
    // Gauntlet avoid 15; +20 in this combat already.
    let plain = preview(&s, 1, p(0, 0), &attack(3)).forecast;
    let art = preview(&s, 1, p(0, 0), &art_attack(3, "sidestep"));
    assert_eq!(plain.defender.map(|d| d.hit), Some(85));
    assert_eq!(art.forecast.defender.map(|d| d.hit), Some(65));
    assert!(matches!(art.notes[..], [ArtNote::Stance(_)]));
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "sidestep"));
    assert_eq!(
        names(&events)[..4],
        [
            "ArtUsed",
            "DurabilitySpent",
            "EffectApplied",
            "CombatResolved"
        ]
    );
    assert_eq!(events[2], art_applied(1, "sidestep", Phase::Player));
    // It helps on the enemy's turn too, once (no double count).
    end(&mut s);
    let counter = preview(&s, 4, p(0, 1), &attack(1)).forecast;
    assert_eq!(counter.attacker.hit, 65);
    let events = end(&mut s);
    assert!(events.contains(&Event::EffectExpired {
        unit: UnitId(1),
        source: EffectSource::Art(aid("sidestep")),
    }));
    assert_eq!(s.phase(), Phase::Player);
    assert!(s.unit(UnitId(1)).unwrap().effects.is_empty());
}

#[test]
fn a_stance_counts_once_when_already_on_the_user() {
    let mut s = battle(vec![
        artist(1, p(0, 0), "knuckles"),
        unit(3, Faction::Enemy, p(1, 0)),
    ]);
    if let Some(u) = s.units.iter_mut().find(|u| u.id == UnitId(1)) {
        u.add_effect(TimedEffect {
            source: EffectSource::Art(aid("sidestep")),
            mods: crate::skill::TimedMods {
                stats: vec![],
                combat: CombatMods {
                    avoid: 20,
                    ..CombatMods::default()
                },
            },
            until: Phase::Player,
        });
    }
    let art = preview(&s, 1, p(0, 0), &art_attack(3, "sidestep"));
    assert_eq!(art.forecast.defender.map(|d| d.hit), Some(65));
}

// ---- Costs and rules -------------------------------------------------------

#[test]
fn an_art_is_refused_when_it_cant_be_used_or_paid() {
    let mut s = battle(vec![
        lord(1, p(0, 0)),
        artist(2, p(0, 2), "blade"),
        unit(3, Faction::Enemy, p(1, 0)),
        unit(4, Faction::Enemy, p(1, 2)),
    ]);
    let not_usable = |unit, art: &str| CommandError::ArtNotUsable {
        unit: UnitId(unit),
        art: aid(art),
    };
    refused_act(
        &mut s,
        2,
        p(0, 2),
        art_attack(4, "nope"),
        CommandError::UnknownArt(aid("nope")),
    );
    // Rank E doesn't know Guard Break (D).
    refused_act(
        &mut s,
        1,
        p(0, 0),
        art_attack(3, "guard_break"),
        not_usable(1, "guard_break"),
    );
    // A spear art with a sword.
    refused_act(
        &mut s,
        2,
        p(0, 2),
        art_attack(4, "unhorse"),
        not_usable(2, "unhorse"),
    );
    // A weapon art of another weapon.
    refused_act(
        &mut s,
        2,
        p(0, 2),
        art_attack(4, "riposte"),
        not_usable(2, "riposte"),
    );
    // One art or one active, not both.
    refused_act(
        &mut s,
        2,
        p(0, 2),
        UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: Some(SkillId::new("keen")),
            art: Some(aid("flowing_cut")),
        },
        CommandError::ArtWithActive,
    );
    let cannot_pay = |art: &str, error| CommandError::CannotPayArt {
        art: aid(art),
        error,
    };
    let mut worn = battle(vec![
        set_durability(artist(1, p(0, 0), "blade"), 3),
        set_durability(artist(2, p(0, 2), "blade"), 0),
        unit(3, Faction::Enemy, p(1, 0)),
        unit(4, Faction::Enemy, p(1, 2)),
    ]);
    refused_act(
        &mut worn,
        2,
        p(0, 2),
        art_attack(4, "flowing_cut"),
        cannot_pay("flowing_cut", CostError::WeaponBroken),
    );
    // Unit 1 has 3 left: Flowing Cut (2) is paid in full.
    act(&mut worn, 1, p(0, 0), art_attack(3, "flowing_cut"));
    assert_eq!(left(&worn, 1), 1);
}

#[test]
fn a_weapon_art_comes_with_its_weapon() {
    let mut s = battle(vec![
        with(unit(1, Faction::Player, p(0, 0)), "moon_blade"),
        stats(unit(3, Faction::Enemy, p(1, 0)), [20, 0, 0, 0, 10, 0, 0]),
    ]);
    let art = preview(&s, 1, p(0, 0), &art_attack(3, "riposte"));
    assert_eq!(
        (art.forecast.attacker.hit, art.durability),
        (85, Some((20, 19)))
    );
    act(&mut s, 1, p(0, 0), art_attack(3, "riposte"));
    assert_eq!(left(&s, 1), 19);
}

#[test]
fn a_weapon_brought_to_zero_by_an_art_breaks_after_the_combat() {
    let mut s = battle(vec![
        set_durability(artist(1, p(0, 0), "blade"), 4),
        Unit {
            hp: 3,
            ..unit(3, Faction::Enemy, p(1, 0))
        },
        unit(4, Faction::Enemy, p(7, 4)),
    ]);
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "guard_break"));
    let (_, forecast, outcome) = &combats(&events)[0];
    // Fought unbroken: full might.
    assert!(!forecast.attacker.broken);
    assert_eq!(outcome.strikes[0].damage, 3);
    let n = names(&events);
    assert_eq!(
        n,
        [
            "ArtUsed",
            "DurabilitySpent",
            "CombatResolved",
            "WeaponExpGained",
            "ItemBroke",
            "UnitFell",
            "UnitActed",
        ]
    );
    assert_eq!(
        events[4],
        Event::ItemBroke {
            unit: UnitId(1),
            item: item("blade"),
        }
    );
    assert_eq!(left(&s, 1), 0);
}

#[test]
fn an_art_costing_more_than_is_left_spends_the_rest_and_breaks_the_weapon() {
    // Guard Break costs 4; the blade has 3 left (Nick, 0414 review).
    let mut s = battle(vec![
        set_durability(artist(1, p(0, 0), "blade"), 3),
        unit(3, Faction::Enemy, p(1, 0)),
        unit(4, Faction::Enemy, p(7, 4)),
    ]);
    let action = art_attack(3, "guard_break");
    assert_eq!(preview(&s, 1, p(0, 0), &action).durability, Some((3, 0)));
    let events = act(&mut s, 1, p(0, 0), action);
    let (_, forecast, _) = &combats(&events)[0];
    assert!(!forecast.attacker.broken);
    assert!(forecast.defender.is_none());
    assert!(events.contains(&Event::DurabilitySpent {
        unit: UnitId(1),
        slot: 0,
        item: item("blade"),
        amount: 3,
        left: 0,
    }));
    assert!(events.contains(&Event::ItemBroke {
        unit: UnitId(1),
        item: item("blade"),
    }));
    assert_eq!(left(&s, 1), 0);
}

#[test]
fn a_counter_never_uses_an_art() {
    // The defender knows Flowing Cut and Guard Break; its counter is a
    // plain one (6/5 follow-up) and costs nothing.
    let mut s = battle(vec![
        stats(artist(1, p(0, 0), "big_blade"), [30, 0, 0, 0, 4, 0, 0]),
        stats(
            Unit {
                role: Role::Boss,
                ..unit(3, Faction::Enemy, p(1, 0))
            },
            [40, 0, 0, 0, 0, 0, 0],
        ),
    ]);
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), attack(1));
    let (_, forecast, _) = &combats(&events)[0];
    assert_eq!(
        forecast
            .defender
            .map(|d| (d.damage, d.followup_damage, d.strikes)),
        Some((10, 12, 2))
    );
    assert!(!names(&events).contains(&"ArtUsed".to_owned()));
    assert_eq!(left(&s, 1), 20);
}

#[test]
fn enemy_bosses_and_combat_green_units_use_arts_and_actives() {
    let grunt = |id, faction, pos| {
        in_class(
            ranked(with(unit(id, faction, pos), "blade"), WeaponRank::D),
            &skill_class("keen").0,
        )
    };
    let mut s = battle(vec![
        lord(1, p(0, 0)),
        grunt(3, Faction::Enemy, p(1, 0)),
        Unit {
            role: Role::Boss,
            ..grunt(4, Faction::Enemy, p(0, 1))
        },
        in_class(grunt(5, Faction::Enemy, p(7, 4)), &skill_class("brace").0),
    ]);
    end(&mut s);
    let not_boss = |id| CommandError::ArtsNotAllowed(UnitId(id));
    refused_act(
        &mut s,
        3,
        p(1, 0),
        art_attack(1, "flowing_cut"),
        not_boss(3),
    );
    refused_act(
        &mut s,
        3,
        p(1, 0),
        UnitAction::Attack {
            target: UnitId(1),
            slot: 0,
            active: Some(SkillId::new("keen")),
            art: None,
        },
        not_boss(3),
    );
    refused_act(
        &mut s,
        5,
        p(7, 4),
        UnitAction::UseSkill {
            skill: SkillId::new("brace"),
            target: None,
        },
        not_boss(5),
    );
    // A plain attack is fine; the boss may use an art.
    act(&mut s, 3, p(1, 0), attack(1));
    let events = act(&mut s, 4, p(0, 1), art_attack(1, "flowing_cut"));
    assert_eq!(names(&events)[0], "ArtUsed");
    // Combat green units use them; non-combat ones (villagers, beasts)
    // don't.
    let noncombatant = |u: Unit| Unit {
        role: Role::Noncombatant,
        ..u
    };
    let mut green = battle(vec![
        lord(1, p(0, 0)),
        grunt(2, Faction::Ally, p(0, 1)),
        noncombatant(grunt(6, Faction::Ally, p(2, 0))),
        in_class(grunt(7, Faction::Neutral, p(7, 4)), &skill_class("brace").0),
        noncombatant(in_class(
            grunt(8, Faction::Neutral, p(7, 2)),
            &skill_class("brace").0,
        )),
        unit(3, Faction::Enemy, p(1, 1)),
    ]);
    end(&mut green);
    end(&mut green);
    assert_eq!(green.phase(), Phase::Other);
    refused_act(
        &mut green,
        6,
        p(2, 0),
        art_attack(3, "flowing_cut"),
        not_boss(6),
    );
    let brace = UnitAction::UseSkill {
        skill: SkillId::new("brace"),
        target: None,
    };
    refused_act(&mut green, 8, p(7, 2), brace.clone(), not_boss(8));
    let events = act(&mut green, 2, p(0, 1), art_attack(3, "flowing_cut"));
    assert_eq!(names(&events)[0], "ArtUsed");
    let events = act(&mut green, 7, p(7, 4), brace);
    assert_eq!(names(&events)[0], "SkillUsed");
}

#[test]
fn an_art_doubles_the_weapon_exp_base_but_not_the_damage_bonus() {
    // Might 10 against 30 HP: dealt 10, +2.
    let fresh = || {
        battle(vec![
            artist(1, p(0, 0), "big_blade"),
            stats(unit(3, Faction::Enemy, p(1, 0)), [30, 0, 0, 0, 0, 0, 0]),
        ])
    };
    let mut s = fresh();
    let events = act(&mut s, 1, p(0, 0), attack(3));
    assert_eq!(
        (weapon_exp(&events, 1), weapon_exp(&events, 3)),
        (Some(4), Some(2))
    );
    let mut s = fresh();
    let events = act(&mut s, 1, p(0, 0), art_attack(3, "flowing_cut"));
    // The defender's counter isn't an art: its base stays 2.
    assert_eq!(
        (weapon_exp(&events, 1), weapon_exp(&events, 3)),
        (Some(6), Some(2))
    );
}

#[test]
fn the_preview_is_validated_like_the_command() {
    let s = battle(vec![
        with_skill_class(artist(1, p(0, 0), "blade"), "keen"),
        unit(3, Faction::Enemy, p(1, 0)),
    ]);
    assert_eq!(
        s.preview_attack(UnitId(1), p(0, 0), &UnitAction::Wait),
        Err(CommandError::NotAnAttack)
    );
    assert_eq!(
        s.preview_attack(UnitId(1), p(0, 0), &art_attack(3, "nope")),
        Err(CommandError::UnknownArt(aid("nope")))
    );
    assert_eq!(
        s.preview_attack(UnitId(3), p(1, 0), &attack(1)),
        Err(CommandError::NotItsPhase {
            unit: UnitId(3),
            phase: Phase::Player,
        })
    );
    let plain = preview(&s, 1, p(0, 0), &attack(3));
    assert_eq!(
        (
            plain.art,
            plain.active,
            plain.durability,
            plain.notes,
            plain.pierce
        ),
        (None, None, None, vec![], None)
    );
    // A combat active's durability shows too.
    let keen = UnitAction::Attack {
        target: UnitId(3),
        slot: 0,
        active: Some(SkillId::new("keen")),
        art: None,
    };
    let shown = preview(&s, 1, p(0, 0), &keen);
    assert_eq!(
        (shown.active, shown.durability),
        (Some(SkillId::new("keen")), Some((20, 17)))
    );
    // The preview matches the combat.
    let mut copy = s.clone();
    let events = act(&mut copy, 1, p(0, 0), art_attack(3, "guard_break"));
    assert_eq!(
        combats(&events)[0].1,
        preview(&s, 1, p(0, 0), &art_attack(3, "guard_break")).forecast
    );
    // Its strike plan starts from both units' HP.
    let full = |id| {
        let u = s.unit(UnitId(id)).unwrap();
        CombatHp {
            current: u.hp,
            max: u.stats.hp,
        }
    };
    assert_eq!(plain.plan, if_all_hit(&plain.forecast, full(1), full(3)));
    let mut hurt = s.clone();
    hurt.units[1].hp = 1;
    let kill = preview(&hurt, 1, p(0, 0), &attack(3)).plan;
    assert!(kill.strikes[0].kills());
    assert_eq!(kill.defender_hp, 0);
}

#[test]
fn a_spell_active_shows_no_durability() {
    let s = battle(vec![
        Unit {
            learned: BTreeSet::from([SpellId::new("bolt")]),
            loadout: Loadout::default(),
            ..in_class(
                unit(1, Faction::Player, p(0, 0)),
                &skill_class("overcast").0,
            )
        },
        unit(3, Faction::Enemy, p(1, 0)),
    ]);
    let cast = UnitAction::Cast {
        spell: SpellId::new("bolt"),
        target: CastTarget::Unit(UnitId(3)),
        active: Some(SkillId::new("overcast")),
    };
    let shown = preview(&s, 1, p(0, 0), &cast);
    assert_eq!(shown.durability, None);
    assert_eq!(shown.active, Some(SkillId::new("overcast")));
}

/// `u` in the class whose active is `skill`.
fn with_skill_class(u: Unit, skill: &str) -> Unit {
    in_class(u, &skill_class(skill).0)
}
