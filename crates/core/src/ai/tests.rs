//! Tests of the AI. Scenarios are drawn in ASCII: terrain `.` plain, `f`
//! forest (cost 2, Def 1, Avoid 20: terrain bonus 3), `#` wall and `~`
//! water (both impassable); units stand on plain, `L` a player lord, `P` a
//! player unit, `E` an enemy, `A` an ally, `N` a neutral unit, numbered 1,
//! 2… in reading order.
//!
//! Test units have 20 HP, Mov 4 and 0 in every other stat, and carry a
//! `sword` (might 5, hit 100, range 1), so combat is exact: every strike
//! hits for the weapon's might and each side strikes once.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

use proptest::prelude::*;

use super::*;
use crate::art::ArtTable;
use crate::battle::{BattleSetup, Event, Objective, Reinforcement};
use crate::class::{ClassDef, ClassId, ClassTable, UnitTags, WeaponProficiency};
use crate::combat::DamageType;
use crate::item::{
    BattlePack, ConsumableDef, ConsumableEffect, ItemDef, ItemId, ItemTable, Loadout, Stock,
    WeaponDef, WeaponInstance,
};
use crate::magic::Element;
use crate::map::BattleMap;
use crate::skill::{Condition, PassiveEffect, SkillDef, SkillId, SkillKind, SkillTable};
use crate::spell::{SpellDef, SpellState, SpellTable};
use crate::stats::Stats;
use crate::terrain::{TerrainId, TerrainRules, TerrainTable};
use crate::unit::Role;
use crate::weapon::{WeaponKind, WeaponRank};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn terrain() -> TerrainTable {
    let rules = |name: &str, cost, defense, avoid| TerrainRules {
        name: name.into(),
        move_cost: vec![cost],
        defense,
        avoid,
        heal_percent: 0,
    };
    TerrainTable {
        movement_types: vec!["foot".into()],
        terrains: vec![
            rules("Plain", Some(1), 0, 0),
            rules("Forest", Some(2), 1, 20),
            rules("Wall", None, 0, 0),
            rules("Water", None, 0, 10),
        ],
    }
}

fn classes() -> ClassTable {
    let fighter = ClassDef {
        id: ClassId("fighter".into()),
        name: "Fighter".into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 4,
        base: Stats::default(),
        caps: Stats::default(),
        growths: crate::stats::Growths::default(),
        weapons: [WeaponKind::Sword, WeaponKind::Bow]
            .map(|kind| WeaponProficiency {
                kind,
                start: WeaponRank::E,
                max: WeaponRank::S,
            })
            .to_vec(),
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: None,
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    };
    ClassTable {
        classes: BTreeMap::from([(fighter.id.clone(), fighter)]),
        ..ClassTable::default()
    }
}

/// A sword-kind weapon.
fn weapon(name: &str, might: StatValue, hit: StatValue, (min, max): (u32, u32)) -> WeaponDef {
    WeaponDef {
        name: name.into(),
        kind: WeaponKind::Sword,
        rank: WeaponRank::E,
        might,
        hit,
        crit: 0,
        weight: 0,
        min_range: min,
        max_range: max,
        damage_type: DamageType::Physical,
        durability: 20,
        effective: vec![],
        arts: vec![],
        price: 0,
    }
}

/// `sword` (might 5), `blade` (3), `lance` (10), `flaky` (5, hit 50),
/// `javelin` (4, range 1–2), `bow` (a bow: 5, range 2), `potion` (heals 10).
fn items() -> ItemTable {
    let bow = WeaponDef {
        kind: WeaponKind::Bow,
        ..weapon("Bow", 5, 100, (2, 2))
    };
    let entries = [
        ("sword", ItemDef::Weapon(weapon("Sword", 5, 100, (1, 1)))),
        ("blade", ItemDef::Weapon(weapon("Blade", 3, 100, (1, 1)))),
        ("lance", ItemDef::Weapon(weapon("Lance", 10, 100, (1, 1)))),
        ("flaky", ItemDef::Weapon(weapon("Flaky", 5, 50, (1, 1)))),
        (
            "javelin",
            ItemDef::Weapon(weapon("Javelin", 4, 100, (1, 2))),
        ),
        ("bow", ItemDef::Weapon(bow)),
        (
            "potion",
            ItemDef::Consumable(ConsumableDef {
                name: "Potion".into(),
                effect: ConsumableEffect::Heal(10),
                price: 0,
            }),
        ),
    ];
    ItemTable {
        items: entries
            .into_iter()
            .map(|(k, v)| (ItemId::new(k), v))
            .collect(),
        ..ItemTable::default()
    }
}

/// `fire` (attack: might 5, hit 100, range 1–2, 5 uses) and `heal` (power
/// 5, range 1, 3 uses).
fn spells() -> SpellTable {
    let fire = SpellDef {
        id: SpellId::new("fire"),
        name: "Fire".into(),
        kind: SpellKind::Attack {
            might: 5,
            hit: 100,
            crit: 0,
            effective: vec![],
        },
        element: Element::Fire,
        min_range: 1,
        max_range: 2,
        uses: 5,
        terrain_effect: None,
    };
    let heal = SpellDef {
        id: SpellId::new("heal"),
        name: "Heal".into(),
        kind: SpellKind::Heal { heal_power: 5 },
        element: Element::None,
        min_range: 1,
        max_range: 1,
        uses: 3,
        terrain_effect: None,
    };
    SpellTable {
        spells: [fire, heal]
            .into_iter()
            .map(|s| (s.id.clone(), s))
            .collect(),
    }
}

/// `skirmish`: after attacking with a bow, move 1 tile.
fn skills() -> SkillTable {
    let skirmish = SkillDef {
        id: SkillId::new("skirmish"),
        name: "Skirmish".into(),
        family: "skirmish".into(),
        rank: 1,
        kind: SkillKind::Passive(vec![PassiveEffect::PostActionMove {
            tiles: 1,
            when: Condition::WeaponKindEquipped(WeaponKind::Bow),
        }]),
    };
    SkillTable {
        skills: BTreeMap::from([(skirmish.id.clone(), skirmish)]),
    }
}

fn unit(id: u32, faction: Faction, pos: Pos) -> Unit {
    let u = Unit {
        id: UnitId(id),
        character: None,
        name: format!("u{id}"),
        class: ClassId("fighter".into()),
        level: 1,
        exp: 0,
        class_records: BTreeMap::new(),
        stats: Stats::from_growable([20, 0, 0, 0, 0, 0, 0], 4),
        hp: 20,
        faction,
        pos,
        acted: false,
        is_lord: false,
        role: Role::Regular,
        ai: AiBehavior::Aggressive,
        weapon_ranks: BTreeMap::new(),
        map_label: "Un".into(),
        weapon_exp: BTreeMap::new(),
        loadout: Loadout::default(),
        consumables: vec![],
        personal_spells: vec![],
        learned: BTreeSet::new(),
        spells: SpellState::default(),
        learned_skills: BTreeSet::new(),
        effects: Vec::new(),
    };
    carrying(u, &["sword"])
}

/// `u` carrying `weapons` in slots 0.., the first equipped (none: nothing).
fn carrying(u: Unit, weapons: &[&str]) -> Unit {
    let mut loadout = Loadout::default();
    for (slot, id) in weapons.iter().enumerate() {
        loadout.weapons[slot] = Some(WeaponInstance {
            def: ItemId::new(id),
            durability_left: 20,
        });
    }
    Unit { loadout, ..u }
}

/// The map and units of an ASCII scene (see the module docs).
fn scene(rows: &[&str]) -> (BattleMap, Vec<Unit>) {
    let mut cells = Vec::new();
    let mut units = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let pos = p(i32::try_from(x).unwrap(), i32::try_from(y).unwrap());
            let id = u32::try_from(units.len() + 1).unwrap();
            let faction = match c {
                'L' | 'P' => Some(Faction::Player),
                'E' => Some(Faction::Enemy),
                'A' => Some(Faction::Ally),
                'N' => Some(Faction::Neutral),
                _ => None,
            };
            if let Some(faction) = faction {
                units.push(Unit {
                    is_lord: c == 'L',
                    ..unit(id, faction, pos)
                });
            }
            cells.push(match c {
                'f' => TerrainId(1),
                '#' => TerrainId(2),
                '~' => TerrainId(3),
                _ => TerrainId(0),
            });
        }
    }
    let w = u16::try_from(rows[0].len()).unwrap();
    let h = u16::try_from(rows.len()).unwrap();
    let map = BattleMap::new("Test", Grid::from_cells(w, h, cells).unwrap());
    (map, units)
}

fn setup(map: BattleMap, units: Vec<Unit>) -> BattleSetup {
    BattleSetup {
        map,
        terrain: Arc::new(terrain()),
        classes: Arc::new(classes()),
        items: Arc::new(items()),
        spells: Arc::new(spells()),
        skills: Arc::new(skills()),
        arts: Arc::new(ArtTable::default()),
        pack: BattlePack {
            items: vec![ItemId::new("potion")],
            cap: 3,
        },
        gold: 0,
        stock: Stock::default(),
        units,
        reinforcements: vec![],
        objective: Objective::Survive { turns: 99 },
        rewind_charges: 0,
        seed: 1,
    }
}

/// The scene's battle, its units changed by `edit`, at the start of the
/// Enemy phase (or the Other phase when there are no enemies).
fn enemy_phase(rows: &[&str], edit: impl FnOnce(&mut Vec<Unit>)) -> BattleState {
    let (map, mut units) = scene(rows);
    edit(&mut units);
    let (mut state, _) = BattleState::new(setup(map, units));
    state.apply(&Command::EndPhase).unwrap();
    state
}

/// The scene's battle, as drawn, at the start of the Enemy phase.
fn scene_units(rows: &[&str]) -> BattleState {
    enemy_phase(rows, |_| {})
}

fn edit(units: &mut [Unit], id: u32, f: impl FnOnce(&mut Unit)) {
    f(units.iter_mut().find(|u| u.id == UnitId(id)).unwrap());
}

fn next(state: &BattleState) -> Command {
    next_command(state, &AiWeights::default()).unwrap()
}

fn act(unit: u32, dest: Pos, action: UnitAction) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest,
        action,
    }
}

fn wait(unit: u32, dest: Pos) -> Command {
    act(unit, dest, UnitAction::Wait)
}

fn attack(unit: u32, dest: Pos, target: u32, slot: usize) -> Command {
    act(
        unit,
        dest,
        UnitAction::Attack {
            target: UnitId(target),
            slot,
            active: None,
            art: None,
        },
    )
}

/// The unit a command moves or acts with.
fn actor(command: &Command) -> Option<UnitId> {
    match command {
        Command::Act { unit, .. } | Command::MoveAfter { unit, .. } => Some(*unit),
        _ => None,
    }
}

/// What the game does next: the AI's command, else end the phase (`None`
/// once the battle is over).
fn step(state: &BattleState) -> Option<Command> {
    match next_command(state, &AiWeights::default()) {
        Some(command) => Some(command),
        None => state.outcome().is_none().then_some(Command::EndPhase),
    }
}

/// Applies the AI's commands until the phase ends (the last, `EndPhase`,
/// included), returning them.
fn play_phase(state: &mut BattleState) -> Vec<Command> {
    let mut commands = Vec::new();
    while let Some(command) = step(state) {
        state.apply(&command).unwrap();
        let end = command == Command::EndPhase;
        commands.push(command);
        if end {
            break;
        }
    }
    commands
}

// --- Attack choice -------------------------------------------------------

#[test]
fn takes_a_guaranteed_kill_over_a_chip_hit() {
    // Both players take the same 5 damage; only player 1 falls to it.
    for (weak, other) in [(1, 3), (3, 1)] {
        let state = enemy_phase(&["P.E.P"], |u| edit(u, weak, |x| x.hp = 5));
        let dest = if weak == 1 { p(1, 0) } else { p(3, 0) };
        assert_eq!(
            next(&state),
            attack(2, dest, weak, 0),
            "kill {weak}, not {other}"
        );
    }
}

#[test]
fn prefers_more_damage() {
    // A blade (3) or a lance (10): the lance in slot 1, though `blade`
    // sorts first. Damage counts.
    let state = enemy_phase(&["P.E"], |u| {
        *u = vec![u[0].clone(), carrying(u[1].clone(), &["blade", "lance"])];
    });
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 1));
}

#[test]
fn prefers_the_lord_when_equal() {
    for (lord, other) in [(1, 3), (3, 1)] {
        let state = enemy_phase(&["P.E.P"], |u| edit(u, lord, |x| x.is_lord = true));
        let dest = if lord == 1 { p(1, 0) } else { p(3, 0) };
        assert_eq!(
            next(&state),
            attack(2, dest, lord, 0),
            "lord {lord}, not {other}"
        );
    }
}

#[test]
fn equal_attacks_go_to_the_lowest_target_then_tile() {
    // Two identical targets: the lower id.
    assert_eq!(next(&scene_units(&["P.E.P"])), attack(2, p(1, 0), 1, 0));
    // One target, four tiles next to it: the lowest (y, x) it can reach
    // ((2,0) is 5 steps away).
    let state = scene_units(&["....", "..P.", "....", "..E."]);
    assert_eq!(next(&state), attack(2, p(1, 1), 1, 0));
}

#[test]
fn equal_weapons_go_to_the_lowest_id_then_slot() {
    // Two swords (slots 0 and 1) and a blade weaker than both: slot 0.
    // Two blades behind a sword of equal numbers: `blade` < `sword`.
    let state = enemy_phase(&["P.E"], |u| {
        u[1] = carrying(u[1].clone(), &["sword", "sword"]);
    });
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 0));
    let state = enemy_phase(&["P.E"], |u| {
        edit(u, 1, |x| x.hp = 3);
        u[1] = carrying(u[1].clone(), &["sword", "blade", "blade"]);
    });
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 1));
}

#[test]
fn avoids_a_counter_when_it_can() {
    // A javelin (range 1–2) against a sword (range 1): from 2 tiles away no
    // counter, so less risk.
    let state = enemy_phase(&["P...E"], |u| u[1] = carrying(u[1].clone(), &["javelin"]));
    assert_eq!(next(&state), attack(2, p(2, 0), 1, 0));
}

#[test]
fn prefers_cover_when_attacking() {
    // Tiles next to the target: (0,0) and (2,0) plain, (1,1) forest (cost
    // 2: still reachable). The forest wins.
    let state = scene_units(&[".P..", ".f..", "....", "..E."]);
    assert_eq!(next(&state), attack(2, p(1, 1), 1, 0));
}

#[test]
fn casts_attack_spells() {
    // No weapon, only Fire (range 1–2): it casts from 2 tiles away.
    let state = enemy_phase(&["P...E"], |u| {
        u[1] = carrying(u[1].clone(), &[]);
        u[1].learned = BTreeSet::from([SpellId::new("fire")]);
    });
    let expected = act(
        2,
        p(2, 0),
        UnitAction::Cast {
            spell: SpellId::new("fire"),
            target: CastTarget::Unit(UnitId(1)),
            active: None,
        },
    );
    assert_eq!(next(&state), expected);
}

#[test]
fn spells_without_uses_and_heals_are_not_attacks() {
    let state = enemy_phase(&["P.E"], |u| {
        u[1] = carrying(u[1].clone(), &["blade"]);
        u[1].learned = BTreeSet::from([SpellId::new("fire"), SpellId::new("heal")]);
    });
    let caster = state.unit(UnitId(2)).unwrap();
    let with = |u: &Unit| {
        arms(&state, u)
            .into_iter()
            .map(|a| a.with)
            .collect::<Vec<_>>()
    };
    // `blade` (slot 0) sorts before `fire`; Heal is no attack.
    assert_eq!(
        with(caster),
        [Equipped::Weapon(0), Equipped::Spell(SpellId::new("fire"))]
    );
    let mut spent = caster.clone();
    spent.spells.uses_left.insert(SpellId::new("fire"), 0);
    assert_eq!(with(&spent), [Equipped::Weapon(0)]);
    // A weapon it can't wield (an unknown class) isn't an arm either.
    let lost = Unit {
        class: ClassId("nobody".into()),
        ..spent
    };
    assert!(with(&lost).is_empty());
}

#[test]
fn neutral_units_attack_nobody() {
    let state = enemy_phase(&["PN"], |_| {});
    assert_eq!(state.phase(), Phase::Other);
    assert_eq!(next(&state), wait(2, p(1, 0)));
}

#[test]
fn allies_fight_enemies_in_the_other_phase() {
    let mut state = scene_units(&["A.E....P"]);
    // Enemy phase: the enemy attacks the ally (the player is too far).
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 0));
    state.apply(&next(&state)).unwrap();
    state.apply(&Command::EndPhase).unwrap();
    assert_eq!(state.phase(), Phase::Other);
    assert_eq!(next(&state), attack(1, p(0, 0), 2, 0));
}

// --- Behaviours ------------------------------------------------------------

#[test]
fn aggressive_units_charge_the_nearest_target() {
    // 8 tiles away with Mov 4: it walks 4 tiles straight at the target.
    let state = scene_units(&["P........E"]);
    assert_eq!(next(&state), wait(2, p(5, 0)));
}

#[test]
fn guard_holds_when_nothing_is_in_reach() {
    let state = enemy_phase(&["P........E"], |u| u[1].ai = AiBehavior::Guard);
    assert_eq!(next(&state), wait(2, p(9, 0)));
}

#[test]
fn guard_attacks_when_something_is_in_reach() {
    // 5 tiles away: Mov 4 + range 1 reaches.
    let state = enemy_phase(&["P....E"], |u| u[1].ai = AiBehavior::Guard);
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 0));
}

#[test]
fn stationary_units_never_move() {
    let far = enemy_phase(&["P.E"], |u| u[1].ai = AiBehavior::Stationary);
    assert_eq!(next(&far), wait(2, p(2, 0)));
    let near = enemy_phase(&["PE"], |u| u[1].ai = AiBehavior::Stationary);
    assert_eq!(next(&near), attack(2, p(1, 0), 1, 0));
    // A bow reaches 2 tiles from its own tile.
    let archer = enemy_phase(&["P.E"], |u| {
        u[1].ai = AiBehavior::Stationary;
        u[1] = carrying(u[1].clone(), &["bow"]);
    });
    assert_eq!(next(&archer), attack(2, p(2, 0), 1, 0));
}

#[test]
fn stationary_units_stay_put_for_a_whole_battle() {
    let mut state = enemy_phase(&["P.....", "......", "....EE"], |u| {
        u[1].ai = AiBehavior::Stationary;
    });
    for _ in 0..6 {
        play_phase(&mut state);
        if state.outcome().is_some() {
            break;
        }
        let start = state.unit(UnitId(2)).map(|u| u.pos);
        assert_eq!(start, Some(p(4, 2)));
    }
}

#[test]
fn never_walks_into_walls_water_or_hostile_tiles() {
    // The only way round the wall is along the bottom row, under the water:
    // 4 steps take it to (3,3).
    let state = scene_units(&[
        "P.#...", //
        "..#..E", //
        "..#~~.", //
        "......", //
    ]);
    assert_eq!(next(&state), wait(2, p(3, 3)));
    // A player blocking a corridor can't be walked through.
    let state = scene_units(&["#####", "L.P.E", "#####"]);
    assert_eq!(next(&state), attack(3, p(3, 1), 2, 0));
}

#[test]
fn unreachable_targets_leave_aggressive_units_in_place() {
    let state = scene_units(&["P.~.E"]);
    assert_eq!(next(&state), wait(2, p(4, 0)));
}

#[test]
fn healers_heal_the_most_injured_ally() {
    // Enemies 2 (−5 HP) and 4 (−8 HP) either side of the healer (3): it
    // heals 4. With both at −5, the lower id (2).
    let run = |hp4| {
        let state = enemy_phase(&["P.......", "..E.E.E."], |u| {
            edit(u, 2, |x| x.hp = 15);
            edit(u, 4, |x| x.hp = hp4);
            edit(u, 3, |x| {
                x.ai = AiBehavior::Healer;
                x.learned = BTreeSet::from([SpellId::new("heal")]);
            });
        });
        let weights = AiWeights::default();
        let planner = Planner::new(&state, &weights);
        planner
            .decide(state.unit(UnitId(3)).unwrap())
            .unwrap()
            .command
    };
    let heal = |target| UnitAction::Cast {
        spell: SpellId::new("heal"),
        target: CastTarget::Unit(UnitId(target)),
        active: None,
    };
    assert!(matches!(run(12), Command::Act { action, .. } if action == heal(4)));
    assert!(matches!(run(15), Command::Act { action, .. } if action == heal(2)));
    // 4 unhurt: 2 is the only one to heal.
    assert!(matches!(run(20), Command::Act { action, .. } if action == heal(2)));
}

#[test]
fn healers_heal_from_outside_the_danger_zone() {
    // The wounded enemy 2 stands at (5,0); the player threatens every tile
    // up to 5 steps from (0,0), so (4,0) is in danger even as a forest. The
    // healer (3) heals from (6,0) or (5,1): the lower y, unless (5,1) is a
    // forest.
    let plan = |rows: &[&str]| {
        let state = enemy_phase(rows, |u| {
            edit(u, 2, |x| x.hp = 10);
            edit(u, 3, |x| {
                x.ai = AiBehavior::Healer;
                x.learned = BTreeSet::from([SpellId::new("heal")]);
            });
        });
        let weights = AiWeights::default();
        let planner = Planner::new(&state, &weights);
        match planner
            .decide(state.unit(UnitId(3)).unwrap())
            .unwrap()
            .command
        {
            Command::Act { dest, .. } => dest,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(plan(&["P...fE..", "......E."]), p(6, 0));
    assert_eq!(plan(&["P....E..", ".....fE."]), p(5, 1));
}

#[test]
fn healers_with_nobody_to_heal_attack_or_keep_back() {
    let healer = |x: &mut Unit| {
        x.ai = AiBehavior::Healer;
        x.learned = BTreeSet::from([SpellId::new("heal")]);
    };
    // A target in reach: it attacks.
    let state = enemy_phase(&["P.E"], |u| edit(u, 2, healer));
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 0));
    // No target in reach: it moves next to its ally, out of danger.
    let state = enemy_phase(&["P..........E....E"], |u| {
        edit(u, 3, healer);
        u[2] = carrying(u[2].clone(), &[]);
    });
    // Its ally (2) moves first (nearer the player); the healer's own plan:
    assert_eq!(actor(&next(&state)), Some(UnitId(2)));
    let weights = AiWeights::default();
    let healer_plan = Planner::new(&state, &weights)
        .decide(state.unit(UnitId(3)).unwrap())
        .unwrap()
        .command;
    assert_eq!(healer_plan, wait(3, p(12, 0)));
}

#[test]
fn wounded_units_never_drink_potions() {
    // Enemies carry no consumables (Nick), and even one given a potion
    // fights on at 1 HP.
    let state = enemy_phase(&["P.E"], |u| {
        edit(u, 2, |x| {
            x.hp = 1;
            x.consumables = vec![ItemId::new("potion")];
        });
    });
    assert_eq!(next(&state), attack(2, p(1, 0), 1, 0));
}

// --- Order within the phase ------------------------------------------------

#[test]
fn attackers_act_first_best_score_first_then_movers_by_distance() {
    // 2 (Mov 1) can only chip player 1; 3 can kill player 4 (5 HP), as can
    // 6, then must walk; 5 must walk (6 is nearer).
    let mut state = enemy_phase(
        &[
            "PE.EP.......", //
            "............", //
            "...........E", //
            "......E.....", //
        ],
        |u| {
            edit(u, 4, |x| x.hp = 5);
            edit(u, 2, |x| x.stats.mov = 1);
        },
    );
    let order: Vec<Option<UnitId>> = play_phase(&mut state).iter().map(actor).collect();
    let ids = [3, 2, 6, 5].map(|i| Some(UnitId(i)));
    assert_eq!(&order[..4], &ids);
    assert_eq!(order[4..], [None], "then the phase ends");
}

#[test]
fn equal_movers_go_by_id() {
    let state = scene_units(&["E.......P.......E"]);
    assert_eq!(actor(&next(&state)), Some(UnitId(1)));
}

#[test]
fn decisions_order() {
    use Ordering::{Greater, Less};
    let d = |unit, rank| Decision {
        unit: UnitId(unit),
        rank,
        command: Command::EndPhase,
    };
    let order = |a: &Decision, b: &Decision| Decision::order(a, b);
    assert_eq!(
        order(&d(2, Rank::Attack(5.0)), &d(1, Rank::Attack(4.0))),
        Less
    );
    assert_eq!(
        order(&d(1, Rank::Attack(4.0)), &d(2, Rank::Attack(5.0))),
        Greater
    );
    assert_eq!(
        order(&d(1, Rank::Attack(4.0)), &d(2, Rank::Attack(4.0))),
        Less
    );
    assert_eq!(
        order(&d(2, Rank::Attack(-9.0)), &d(1, Rank::Other(0))),
        Less
    );
    assert_eq!(
        order(&d(1, Rank::Other(0)), &d(2, Rank::Attack(-9.0))),
        Greater
    );
    assert_eq!(order(&d(2, Rank::Other(3)), &d(1, Rank::Other(4))), Less);
    assert_eq!(order(&d(1, Rank::Other(4)), &d(2, Rank::Other(3))), Greater);
    assert_eq!(order(&d(2, Rank::Other(3)), &d(1, Rank::Other(3))), Greater);
}

#[test]
fn ends_the_phase_when_everyone_has_acted() {
    let mut state = scene_units(&["P........E"]);
    state.apply(&next(&state)).unwrap();
    assert_eq!(next_command(&state, &AiWeights::default()), None);
}

#[test]
fn reinforcements_that_just_arrived_dont_act() {
    let (map, units) = scene(&["P........."]);
    let newcomer = unit(9, Faction::Enemy, p(9, 0));
    let (mut state, _) = BattleState::new(BattleSetup {
        reinforcements: vec![Reinforcement {
            turn: 1,
            unit: newcomer,
        }],
        ..setup(map, units)
    });
    let events = state.apply(&Command::EndPhase).unwrap();
    assert!(events.contains(&Event::UnitsArrived {
        units: vec![UnitId(9)]
    }));
    assert_eq!(next_command(&state, &AiWeights::default()), None);
}

#[test]
fn nothing_to_do_once_the_battle_is_over() {
    let mut state = enemy_phase(&["P.E"], |u| edit(u, 1, |x| x.hp = 5));
    state.apply(&next(&state)).unwrap();
    assert!(state.outcome().is_some());
    assert_eq!(next_command(&state, &AiWeights::default()), None);
}

#[test]
fn units_the_battle_cant_move_are_skipped() {
    // An unknown class: the unit can't act, so the phase just ends.
    let state = enemy_phase(&["P.E"], |u| u[1].class = ClassId("nobody".into()));
    assert_eq!(next_command(&state, &AiWeights::default()), None);
}

// --- Move after an attack --------------------------------------------------

#[test]
fn a_move_after_an_attack_steps_away() {
    // An archer with Skirmish shoots from 2 tiles away, then steps back.
    let mut state = enemy_phase(&["P...E"], |u| {
        u[1] = carrying(u[1].clone(), &["bow"]);
        u[1].learned_skills = BTreeSet::from([SkillId::new("skirmish")]);
        u[1].stats.mov = 2;
    });
    let shot = next(&state);
    assert_eq!(shot, attack(2, p(2, 0), 1, 0));
    state.apply(&shot).unwrap();
    assert!(state.pending_move().is_some());
    assert_eq!(
        next(&state),
        Command::MoveAfter {
            unit: UnitId(2),
            to: Some(p(3, 0))
        }
    );
}

#[test]
fn a_move_after_an_attack_stays_when_nothing_is_farther() {
    // The archer (2) at (2,1) can only step to (1,1), nearer the player.
    let mut state = enemy_phase(&["###", "P.E", "###"], |u| {
        u[1] = carrying(u[1].clone(), &["bow"]);
        u[1].learned_skills = BTreeSet::from([SkillId::new("skirmish")]);
    });
    let shot = next(&state);
    assert_eq!(shot, attack(2, p(2, 1), 1, 0));
    state.apply(&shot).unwrap();
    assert_eq!(state.move_after_tiles(), [p(1, 1)]);
    let stay = Command::MoveAfter {
        unit: UnitId(2),
        to: None,
    };
    assert_eq!(next(&state), stay);
}

#[test]
fn move_after_keeps_the_first_of_equally_far_tiles() {
    // From (2,1), 2 from the player: (2,0), (3,1) and (2,2) are all 3 away;
    // the first in the battle's order wins.
    let mut state = enemy_phase(&["....", "P.E.", "...."], |u| {
        u[1] = carrying(u[1].clone(), &["bow"]);
        u[1].learned_skills = BTreeSet::from([SkillId::new("skirmish")]);
        u[1].ai = AiBehavior::Stationary;
    });
    state.apply(&next(&state)).unwrap();
    let tiles = state.move_after_tiles();
    let first_far = tiles
        .iter()
        .copied()
        .find(|&t| Pos::manhattan(t, p(0, 1)) == 3);
    assert!(first_far.is_some());
    assert_eq!(
        next(&state),
        Command::MoveAfter {
            unit: UnitId(2),
            to: first_far
        }
    );
}

// --- Numbers ---------------------------------------------------------------

fn side(damage: StatValue, followup: StatValue, hit: u8, strikes: u8) -> SideForecast {
    SideForecast {
        damage,
        followup_damage: followup,
        hit,
        crit: 0,
        strikes,
        effective: false,
        broken: false,
        affinity: None,
    }
}

fn hp(current: StatValue, max: StatValue) -> CombatHp {
    CombatHp { current, max }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn odds_of_sure_strikes() {
    let f = Forecast {
        attacker: side(4, 4, 100, 1),
        defender: Some(side(3, 3, 100, 1)),
    };
    let o = Odds::of(&f, hp(10, 10), hp(10, 10));
    assert_eq!(
        o,
        Odds {
            dealt: 4.0,
            taken: 3.0,
            kill: 0.0
        }
    );
    // A kill stops the counter.
    let o = Odds::of(&f, hp(10, 10), hp(4, 10));
    assert_eq!(
        o,
        Odds {
            dealt: 4.0,
            taken: 0.0,
            kill: 1.0
        }
    );
    // Damage past 0 HP doesn't count.
    let o = Odds::of(&f, hp(10, 10), hp(2, 10));
    assert_eq!(
        o,
        Odds {
            dealt: 2.0,
            taken: 0.0,
            kill: 1.0
        }
    );
}

#[test]
fn odds_of_chancy_strikes() {
    // 50% to hit twice for 5 (follow-up 6), against 4 HP of counter at 25%.
    let f = Forecast {
        attacker: side(5, 6, 50, 2),
        defender: Some(side(4, 4, 25, 1)),
    };
    let o = Odds::of(&f, hp(10, 10), hp(10, 10));
    // Dealt: 0 (both miss), 5, 6, 11 → 10 max: (0 + 5 + 6 + 10) / 4.
    assert!(close(o.dealt, 21.0 / 4.0), "{o:?}");
    assert!(close(o.kill, 0.25), "{o:?}");
    assert!(close(o.taken, 1.0), "{o:?}");
    // No counter, a missed strike never hits.
    let f = Forecast {
        attacker: side(5, 5, 0, 1),
        defender: None,
    };
    assert_eq!(Odds::of(&f, hp(10, 10), hp(10, 10)), Odds::default());
}

#[test]
fn odds_stop_when_the_attacker_falls() {
    // The counter kills the attacker (3 HP) before its follow-up.
    let f = Forecast {
        attacker: side(2, 2, 100, 2),
        defender: Some(side(5, 5, 100, 1)),
    };
    let o = Odds::of(&f, hp(3, 10), hp(10, 10));
    assert_eq!(
        o,
        Odds {
            dealt: 2.0,
            taken: 3.0,
            kill: 0.0
        }
    );
}

#[test]
fn absorbed_strikes_heal() {
    let absorb = SideForecast {
        affinity: Some(Affinity::Absorb),
        ..side(3, 3, 100, 2)
    };
    let f = Forecast {
        attacker: absorb,
        defender: None,
    };
    // Healed up to max: nothing dealt.
    assert_eq!(Odds::of(&f, hp(10, 10), hp(8, 10)), Odds::default());
    assert_eq!(struck_hp(&absorb, 1, 8, 10), 10);
    assert_eq!(struck_hp(&absorb, 1, 5, 10), 8);
    assert_eq!(struck_hp(&side(3, 7, 100, 2), 2, 10, 10), 3);
    assert_eq!(struck_hp(&side(3, 7, 100, 2), 1, 10, 10), 7);
    assert_eq!(struck_hp(&side(3, 7, 100, 2), 2, 5, 10), 0);
}

#[test]
fn scores_add_up_the_weights() {
    let state = scene_units(&["P.E"]);
    let weights = AiWeights {
        damage: 2,
        kill: 30,
        lord: 7,
        risk: 3,
        terrain: 5,
    };
    let planner = Planner::new(&state, &weights);
    let odds = Odds {
        dealt: 4.0,
        taken: 1.5,
        kill: 0.5,
    };
    // 2×4 + 30×0.5 + 7 − 3×1.5 + 5×3 = 8 + 15 + 7 − 4.5 + 15.
    assert!(close(planner.score(&odds, true, 3.0), 40.5));
    assert!(close(planner.score(&odds, false, 0.0), 18.5));
}

#[test]
fn terrain_bonus_is_def_plus_a_tenth_of_avoid() {
    let state = scene_units(&["Pf~E"]);
    assert!(close(terrain_bonus(&state, p(0, 0)), 0.0));
    assert!(close(terrain_bonus(&state, p(1, 0)), 3.0));
    assert!(close(terrain_bonus(&state, p(2, 0)), 1.0));
    assert!(close(terrain_bonus(&state, p(9, 9)), 0.0));
}

#[test]
fn flow_distances_walk_to_a_tile_next_to_a_target() {
    let state = scene_units(&[
        "P.f.#E", //
        "....#.", //
        "......", //
    ]);
    let flow = flow_field(&state, Faction::Enemy, MovementTypeId(0));
    let at = |x, y| flow.get(p(x, y)).copied().flatten();
    // Next to the player: 0. Then the cost of each tile walked onto.
    assert_eq!(at(0, 0), Some(0));
    assert_eq!(at(1, 0), Some(0));
    assert_eq!(at(0, 1), Some(0));
    assert_eq!(at(2, 0), Some(1));
    assert_eq!(at(3, 0), Some(3), "through the forest (2) or round it");
    assert_eq!(at(4, 0), None, "a wall");
    // Round the wall: (4,2), (3,2), (2,2), (1,2), (1,1), then (1,0).
    assert_eq!(at(5, 2), Some(6));
    assert_eq!(at(5, 0), Some(8));
    // No targets: no distances.
    let flow = flow_field(&state, Faction::Neutral, MovementTypeId(0));
    assert!(flow.cells().iter().all(Option::is_none));
}

#[test]
fn approach_prefers_the_cheapest_of_equally_near_tiles() {
    let state = scene_units(&["P...E"]);
    let reach = reachable(
        state.map(),
        state.terrain(),
        state.classes(),
        state.units(),
        UnitId(2),
    )
    .unwrap();
    let flow = flow_field(&state, Faction::Enemy, MovementTypeId(0));
    let dests: Vec<Pos> = reach.stoppable().iter().collect();
    assert_eq!(approach(&reach, &dests, &flow), Some(p(1, 0)));
    // Already next to the target: stays.
    assert_eq!(approach(&reach, &[p(4, 0), p(1, 0)], &flow), Some(p(1, 0)));
    let near = [p(1, 0), p(4, 0)];
    let far = flow_field(&state, Faction::Neutral, MovementTypeId(0));
    assert_eq!(approach(&reach, &near, &far), None);
}

// --- Validity and determinism ----------------------------------------------

#[test]
fn the_same_state_gives_the_same_command() {
    let state = enemy_phase(&["P..f....E", "..E...E..", "L....A..E"], |u| {
        edit(u, 3, |x| x.hp = 9);
    });
    let first = next(&state);
    assert_eq!(next(&state), first);
    assert_eq!(next(&state.clone()), first);
}

#[test]
fn the_ai_plays_both_sides_to_the_end() {
    let mut state = enemy_phase(
        &[
            "L...f....E.", //
            ".P..f..E...", //
            "...~~......", //
            ".A.......EE", //
        ],
        |_| {},
    );
    let mut commands = 0;
    while let Some(command) = step(&state) {
        state.apply(&command).unwrap();
        commands += 1;
        assert!(commands < 2000, "no end in sight");
        if state.turn() > 30 {
            break;
        }
    }
}

prop_compose! {
    /// A unit of a random faction, behaviour and gear on a 9×7 map.
    fn any_unit(id: u32)(
        x in 0..9i32,
        y in 0..7i32,
        faction in prop::sample::select(vec![Faction::Player, Faction::Enemy, Faction::Ally, Faction::Neutral]),
        ai in prop::sample::select(vec![AiBehavior::Aggressive, AiBehavior::Guard, AiBehavior::Stationary, AiBehavior::Healer]),
        hp in 1..=20,
        mov in 0..6,
        gear in 0..6usize,
    ) -> Unit {
        let weapons: &[&str] = [&["sword"][..], &["bow"], &["javelin", "blade"], &["flaky"], &[], &["lance", "bow"]][gear];
        let mut u = carrying(unit(id, faction, p(x, y)), weapons);
        u.hp = hp;
        u.ai = ai;
        u.stats.mov = mov;
        if gear == 4 {
            u.learned = BTreeSet::from([SpellId::new("fire"), SpellId::new("heal")]);
        }
        if gear == 1 {
            u.learned_skills = BTreeSet::from([SkillId::new("skirmish")]);
        }
        u
    }
}

fn any_battle() -> impl Strategy<Value = (Vec<Unit>, u64)> {
    let units = (1..=10u32).map(any_unit).collect::<Vec<_>>();
    (units, any::<u64>(), 1..=10usize).prop_map(|(units, seed, n)| {
        // One unit per tile; always a player unit (else the battle is over).
        let mut seen = BTreeSet::new();
        let mut kept: Vec<Unit> = units
            .into_iter()
            .take(n.max(2))
            .filter(|u| seen.insert(u.pos))
            .collect();
        kept[0].faction = Faction::Player;
        (kept, seed)
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Every command the AI issues is accepted, for either side, and the
    /// same state always gives the same command.
    #[test]
    fn every_ai_command_is_valid((units, seed) in any_battle()) {
        let (map, _) = scene(&[".........", "..f..f...", ".........", "...#.#...", ".........", ".f.....f.", "........."]);
        let (mut state, _) = BattleState::new(BattleSetup { seed, ..setup(map, units) });
        for _ in 0..300 {
            let Some(command) = step(&state) else {
                break;
            };
            prop_assert_eq!(step(&state), Some(command.clone()));
            let result = state.apply(&command);
            prop_assert!(result.is_ok(), "{:?} refused: {:?}", command, result);
            if state.turn() > 8 {
                break;
            }
        }
    }
}

/// Ticket 0501's budget: planning a 20-unit enemy phase on a 30×30 map
/// takes under 50 ms in release. Run with
/// `cargo test -p trpg-core --release -- --ignored enemy_phase_planning_time --nocapture`.
#[test]
#[ignore = "a timing measurement: run in release"]
fn enemy_phase_planning_time() {
    // 20 players face 20 enemies (every third unit with a javelin, a bow
    // and a sword) across the middle of a 30×30 map with forests and
    // walls, so most enemies can attack: the costly case.
    let mut rows = vec![String::new(); 30];
    for (y, row) in rows.iter_mut().enumerate() {
        for x in 0..30 {
            let c = match (x, y) {
                (11 | 13, 5..=24) if y % 2 == 1 => 'P',
                (16 | 18, 5..=24) if y % 2 == 1 => 'E',
                _ if (x * 7 + y * 3) % 11 == 0 => 'f',
                _ if (x * 5 + y) % 23 == 0 => '#',
                _ => '.',
            };
            row.push(c);
        }
    }
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let mut state = enemy_phase(&rows, |u| {
        for x in u.iter_mut() {
            x.stats.mov = 6;
            if x.id.0 % 3 == 0 {
                *x = carrying(x.clone(), &["javelin", "bow", "sword"]);
            }
        }
    });
    let enemies = state
        .units()
        .iter()
        .filter(|u| u.faction == Faction::Enemy)
        .count();
    assert_eq!(enemies, 20);
    let mut planning = std::time::Duration::ZERO;
    let mut commands = 0;
    loop {
        let t = Instant::now();
        let command = next_command(&state, &AiWeights::default());
        planning += t.elapsed();
        let Some(command) = command else {
            break;
        };
        state.apply(&command).unwrap();
        commands += 1;
    }
    #[allow(clippy::print_stdout)]
    {
        println!("enemy phase: {commands} commands planned in {planning:?}");
    }
}
