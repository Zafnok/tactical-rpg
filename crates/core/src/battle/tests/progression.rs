//! Unit EXP, class points and the ally EXP pool in battle (ticket 0601).
//! The test classes have caps of 0, so a level up rolls 7 times and gains
//! nothing: the events are exact.

use super::*;
use crate::unit::ClassRecord;

/// [`setup`] with levels on: level cap 99, 10 CP per class level.
fn leveled(units: Vec<Unit>) -> BattleSetup {
    BattleSetup {
        classes: Arc::new(leveling(classes())),
        ..setup(units)
    }
}

fn exp(unit: u32, amount: u32) -> Event {
    Event::ExpGained {
        unit: UnitId(unit),
        amount,
    }
}

fn cp(unit: u32, class: &str, amount: ClassPoints) -> Event {
    Event::ClassPointsGained {
        unit: UnitId(unit),
        class: ClassId(class.into()),
        amount,
    }
}

/// The EXP, level and class-point events of `events`.
fn progress(events: &[Event]) -> Vec<Event> {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::ExpGained { .. }
                    | Event::LeveledUp { .. }
                    | Event::ClassPointsGained { .. }
                    | Event::ClassLeveledUp { .. }
                    | Event::ClassMastered { .. }
                    | Event::SkillLearned { .. }
                    | Event::SpellLearned { .. }
            )
        })
        .cloned()
        .collect()
}

fn level_exp(s: &BattleState, id: u32) -> (Level, u32) {
    let u = s.unit(UnitId(id)).unwrap();
    (u.level, u.exp)
}

#[test]
fn a_player_attack_gives_exp_and_class_points_after_the_combat() {
    let mut units = cast();
    units[3].pos = p(3, 2);
    let mut s = start(leveled(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    // Equal levels, damage dealt: 20 EXP, 2 CP. The enemy gets nothing.
    let n = events.len();
    assert_eq!(
        events[n - 3..],
        [
            exp(2, 20),
            cp(2, "fighter", 2),
            Event::UnitActed { unit: UnitId(2) }
        ]
    );
    assert_eq!(progress(&events).len(), 2);
    assert_eq!(level_exp(&s, 2), (1, 20));
    assert_eq!(
        s.unit(UnitId(2)).unwrap().class_records[&ClassId("fighter".into())],
        ClassRecord {
            class_level: 1,
            class_points: 2
        }
    );
    assert_eq!(level_exp(&s, 4), (1, 0));
}

#[test]
fn a_kill_gives_the_kill_award_after_the_fall() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 10);
    units[3].pos = p(3, 2);
    units[3].level = 5;
    let mut s = start(leveled(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    // d = +4: 2 × (35 / 3 + 32) = 2 × 43 = 86; CP 2 + 2.
    let n = events.len();
    assert_eq!(
        events[n - 4..],
        [
            Event::UnitFell { unit: UnitId(4) },
            exp(2, 86),
            cp(2, "fighter", 4),
            Event::UnitActed { unit: UnitId(2) },
        ]
    );
}

#[test]
fn killing_a_boss_adds_40() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 10);
    units[3].pos = p(3, 2);
    units[3].role = Role::Boss;
    let mut s = start(leveled(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    assert_eq!(progress(&events)[0], exp(2, 100));
}

#[test]
fn dealing_no_damage_gives_2() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 0);
    units[3].pos = p(1, 2);
    let mut s = start(leveled(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    assert_eq!(progress(&events), [exp(2, 2), cp(2, "fighter", 2)]);
}

#[test]
fn a_counter_gives_the_defender_exp_on_the_enemy_phase() {
    let mut units = cast();
    units[3].pos = p(1, 2);
    let mut s = start(leveled(units));
    end(&mut s);
    let events = act(&mut s, 4, p(1, 2), attack(2));
    assert_eq!(progress(&events), [exp(2, 20), cp(2, "fighter", 2)]);
}

#[test]
fn a_defender_that_cant_counter_gets_nothing() {
    let mut units = cast();
    // The enemy's bow reaches 2 tiles; the player's sword only 1.
    units[3] = carrying(units[3].clone(), &[weapon(2, 2, 1)]);
    units[3].pos = p(2, 2);
    let mut s = start(leveled(units));
    end(&mut s);
    let events = act(&mut s, 4, p(2, 2), attack(2));
    assert_eq!(progress(&events), []);
}

#[test]
fn a_unit_that_falls_gets_nothing() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 1);
    units[3] = armed(units[3].clone(), 12);
    units[3].pos = p(3, 2);
    let mut s = start(leveled(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    assert!(events.contains(&Event::UnitFell { unit: UnitId(2) }));
    assert_eq!(progress(&events), []);
}

#[test]
fn crossing_100_levels_up_in_battle_with_7_rolls() {
    let mut units = cast();
    units[1].exp = 90;
    units[3].pos = p(3, 2);
    let mut s = start(leveled(units));
    let mut rng = s.rng.clone();
    let events = act(&mut s, 2, p(2, 2), attack(4));
    assert_eq!(
        progress(&events),
        [
            exp(2, 20),
            Event::LeveledUp {
                unit: UnitId(2),
                level: 2,
                gains: StatGains::default()
            },
            cp(2, "fighter", 2),
        ]
    );
    assert_eq!(level_exp(&s, 2), (2, 10));
    // Two strikes that hit (2 hit rolls and a crit roll each), then 7
    // level-up rolls, nothing else.
    for _ in 0..2 * 3 + 7 {
        rng.roll_percent();
    }
    assert_eq!(s.rng, rng);
}

#[test]
fn class_points_can_master_the_class() {
    let mut units = cast();
    units[1].class_records.insert(
        ClassId("fighter".into()),
        ClassRecord {
            class_level: 9,
            class_points: 88,
        },
    );
    units[3].pos = p(3, 2);
    let mut s = start(leveled(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    let fighter = ClassId("fighter".into());
    assert_eq!(
        progress(&events)[1..],
        [
            cp(2, "fighter", 2),
            Event::ClassLeveledUp {
                unit: UnitId(2),
                class: fighter.clone(),
                class_level: 10
            },
            Event::ClassMastered {
                unit: UnitId(2),
                class: fighter
            },
        ]
    );
}

#[test]
fn heals_tile_casts_and_non_combat_actives_give_their_awards() {
    // A heal: 24 EXP, 2 CP.
    let mut healer = unit(2, Faction::Player, p(0, 2));
    healer.learned = BTreeSet::from([SpellId::new("heal"), SpellId::new("fire")]);
    let mut hurt = lord(1, p(0, 0));
    hurt.hp = 3;
    let mut s = start(leveled(vec![
        hurt,
        healer,
        unit(3, Faction::Enemy, p(7, 0)),
    ]));
    let heal = UnitAction::Cast {
        spell: SpellId::new("heal"),
        target: CastTarget::Unit(UnitId(1)),
        active: None,
    };
    let events = act(&mut s, 2, p(0, 1), heal);
    assert_eq!(progress(&events), [exp(2, 24), cp(2, "fighter", 2)]);

    // A tile cast on the forest at (2,2): 24 EXP, 2 CP.
    end(&mut s);
    end(&mut s);
    let fire = UnitAction::Cast {
        spell: SpellId::new("fire"),
        target: CastTarget::Tile(p(2, 2)),
        active: None,
    };
    let events = act(&mut s, 2, p(1, 2), fire);
    assert_eq!(progress(&events), [exp(2, 24), cp(2, "fighter", 2)]);
    assert_eq!(level_exp(&s, 2), (1, 48));

    // Brace, a non-combat active: 20 EXP, 2 CP.
    let class = skill_class("brace");
    let mut s = start(leveled(vec![
        Unit {
            class: class.clone(),
            ..lord(1, p(0, 0))
        },
        unit(3, Faction::Enemy, p(7, 0)),
    ]));
    let brace = UnitAction::UseSkill {
        skill: SkillId::new("brace"),
        target: None,
    };
    let events = act(&mut s, 1, p(0, 0), brace);
    assert_eq!(progress(&events), [exp(1, 20), cp(1, &class.0, 2)]);
}

#[test]
fn enemies_and_neutrals_never_gain() {
    let mut units = cast();
    units[3].pos = p(1, 2);
    let mut s = start(leveled(units));
    end(&mut s);
    // Enemy 4 attacks and is countered: only the player unit gains.
    let events = act(&mut s, 4, p(1, 2), attack(2));
    assert!(progress(&events).iter().all(|e| !matches!(
        e,
        Event::ExpGained {
            unit: UnitId(4),
            ..
        }
    )));
    assert_eq!(level_exp(&s, 4), (1, 0));
    assert_eq!(s.exp_pool(), 0);
}

/// Lord 1 and players 5 and 8 take part; player 2 is at the level cap,
/// player 7 falls, player 9 is a reinforcement still to come. Ally 6 hits
/// enemy 3 (20 EXP into the pool); enemy 4 kills player 7. The map is
/// survived at the end of turn 1.
fn pool_battle() -> BattleSetup {
    let mut capped = unit(2, Faction::Player, p(0, 2));
    capped.level = 99;
    let units = vec![
        lord(1, p(0, 0)),
        capped,
        unit(5, Faction::Player, p(0, 4)),
        unit(8, Faction::Player, p(1, 4)),
        unit(7, Faction::Player, p(6, 4)),
        armed(unit(4, Faction::Enemy, p(7, 4)), 10),
        unit(6, Faction::Ally, p(5, 0)),
        unit(3, Faction::Enemy, p(7, 0)),
    ];
    BattleSetup {
        objective: Objective::Survive { turns: 1 },
        reinforcements: vec![Reinforcement {
            turn: 5,
            unit: unit(9, Faction::Player, p(0, 3)),
        }],
        ..leveled(units)
    }
}

#[test]
fn ally_exp_goes_to_the_pool_and_is_shared_on_a_win() {
    let mut s = start(pool_battle());
    end(&mut s);
    let events = act(&mut s, 4, p(7, 4), attack(7));
    assert!(events.contains(&Event::UnitFell { unit: UnitId(7) }));
    end(&mut s);
    // The ally hits enemy 3 and takes its counter: 20 EXP, to the pool.
    let events = act(&mut s, 6, p(6, 0), attack(3));
    assert_eq!(progress(&events), []);
    assert_eq!(s.exp_pool(), 20);
    assert_eq!(level_exp(&s, 6), (1, 0));
    // Turn 1 ends: survived. 20 / 3 = 6 each, 2 lost, in unit order,
    // before the battle ends.
    let events = end(&mut s);
    assert_eq!(
        events,
        [exp(1, 6), exp(5, 6), exp(8, 6), ended(Outcome::Victory)]
    );
    assert_eq!(s.exp_pool(), 0);
    assert_eq!(level_exp(&s, 2), (99, 0));
}

#[test]
fn a_big_pool_share_can_level_up_more_than_once() {
    let mut s = start(pool_battle());
    s.exp_pool = 700;
    let mut rng = s.rng.clone();
    end(&mut s);
    act(&mut s, 4, p(7, 4), attack(7));
    end(&mut s);
    let events = end(&mut s);
    // 700 / 3 = 233 each: two level ups each.
    let levels = events
        .iter()
        .filter(|e| matches!(e, Event::LeveledUp { .. }))
        .count();
    assert_eq!(levels, 6);
    assert_eq!(level_exp(&s, 1), (3, 33));
    // Enemy 4's killing strike (3 rolls), then 6 level ups of 7 rolls.
    for _ in 0..3 + 6 * 7 {
        rng.roll_percent();
    }
    assert_eq!(s.rng, rng);
}

#[test]
fn the_pool_is_lost_on_a_defeat() {
    let mut units = vec![
        lord(1, p(0, 0)),
        armed(unit(3, Faction::Enemy, p(1, 0)), 10),
        unit(6, Faction::Ally, p(7, 4)),
    ];
    units[0].hp = 1;
    let mut s = start(leveled(units));
    s.exp_pool = 50;
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), attack(1));
    assert_eq!(events.last(), Some(&ended(Outcome::Defeat)));
    assert_eq!(progress(&events), []);
}

#[test]
fn the_pool_survives_a_save() {
    let mut s = start(pool_battle());
    s.exp_pool = 42;
    let text = ron::to_string(&s).unwrap();
    let loaded: BattleState = ron::from_str(&text).unwrap();
    assert_eq!(loaded.exp_pool(), 42);
}

#[test]
fn nothing_is_gained_with_levels_off() {
    // The default test tables have no level cap: nobody gains.
    let mut units = cast();
    units[3].pos = p(3, 2);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    assert_eq!(progress(&events), []);
}

fn skill_action(skill: &str, target: Option<u32>) -> UnitAction {
    UnitAction::UseSkill {
        skill: SkillId::new(skill),
        target: target.map(UnitId),
    }
}

#[test]
fn a_healing_active_gives_the_heal_award() {
    let class = skill_class("sanctuary");
    let mut wounded = unit(2, Faction::Player, p(1, 0));
    wounded.hp = 4;
    let mut s = start(leveled(vec![
        Unit {
            class: class.clone(),
            ..lord(1, p(1, 1))
        },
        wounded,
        unit(3, Faction::Enemy, p(7, 4)),
    ]));
    let events = act(&mut s, 1, p(1, 1), skill_action("sanctuary", None));
    // The higher of 20 (active) and 24 (heal).
    assert_eq!(progress(&events), [exp(1, 24), cp(1, &class.0, 2)]);
}

/// Shover 1 at (1,1) shoves enemy 3 (at (0,1), `hp`) off the map's edge
/// (5 collision damage); enemy 4 stays far away. Returns the events.
fn shove_off_the_edge(hp: StatValue, boss: bool) -> Vec<Event> {
    let mut target = unit(3, Faction::Enemy, p(0, 1));
    target.hp = hp;
    target.role = if boss { Role::Boss } else { Role::Regular };
    let mut s = start(leveled(vec![
        Unit {
            class: skill_class("shove"),
            ..lord(1, p(1, 1))
        },
        target,
        unit(4, Faction::Enemy, p(7, 4)),
    ]));
    act(&mut s, 1, p(1, 1), skill_action("shove", Some(3)))
}

#[test]
fn a_shove_that_kills_gives_the_kill_award() {
    let class = skill_class("shove").0;
    // Survives: the active's 20.
    let events = shove_off_the_edge(10, false);
    assert_eq!(progress(&events), [exp(1, 20), cp(1, &class, 2)]);
    // Falls: a kill at equal levels, 60 EXP and 4 CP, after the fall.
    let events = shove_off_the_edge(3, false);
    let fell = events
        .iter()
        .position(|e| *e == Event::UnitFell { unit: UnitId(3) });
    let gained = events.iter().position(|e| *e == exp(1, 60));
    assert!(fell.is_some() && fell < gained, "{events:?}");
    assert_eq!(progress(&events), [exp(1, 60), cp(1, &class, 4)]);
    // A boss at equal levels: 60 + 40.
    assert_eq!(progress(&shove_off_the_edge(3, true))[0], exp(1, 100));
}

#[test]
fn a_shove_kill_below_the_active_award_gives_the_active_award() {
    // Lord at level 11 against a level-1 enemy: a kill is worth 14.
    let class = skill_class("shove");
    let mut shover = Unit {
        class: class.clone(),
        ..lord(1, p(1, 1))
    };
    shover.level = 11;
    let mut target = unit(3, Faction::Enemy, p(0, 1));
    target.hp = 3;
    let mut s = start(leveled(vec![
        shover,
        target,
        unit(4, Faction::Enemy, p(7, 4)),
    ]));
    let events = act(&mut s, 1, p(1, 1), skill_action("shove", Some(3)));
    assert_eq!(progress(&events), [exp(1, 20), cp(1, &class.0, 4)]);
}

#[test]
fn a_shove_that_fells_a_friend_gives_no_kill_award() {
    // Enemy 3 is pushed into player 2 (2 HP), who falls; enemy 3 stands.
    let class = skill_class("shove").0;
    let mut friend = unit(2, Faction::Player, p(3, 1));
    friend.hp = 2;
    let mut s = start(leveled(vec![
        Unit {
            class: skill_class("shove"),
            ..lord(1, p(0, 0))
        },
        unit(3, Faction::Enemy, p(2, 1)),
        friend,
    ]));
    let events = act(&mut s, 1, p(1, 1), skill_action("shove", Some(3)));
    assert!(events.contains(&Event::UnitFell { unit: UnitId(2) }));
    assert_eq!(progress(&events), [exp(1, 20), cp(1, &class, 2)]);
}
