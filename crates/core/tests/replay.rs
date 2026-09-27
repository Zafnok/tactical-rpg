//! Replay and save/load determinism (ADR-0007 layer 5): the same setup, seed
//! and commands give identical events, and a battle saved mid-way, loaded and
//! continued gives the same events as one played straight through.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use trpg_core::{
    BattleMap, BattlePack, BattleSetup, BattleState, CastTarget, ClassDef, ClassId, ClassTable,
    Command, ConsumableDef, ConsumableEffect, DamageType, Element, Equipped, Event, Faction, Grid,
    Growths, ItemDef, ItemId, ItemTable, LoadoutDef, MovementTypeId, Objective, Pos, SpellDef,
    SpellId, SpellKind, SpellState, SpellTable, Stats, Stock, TerrainId, TerrainRules,
    TerrainTable, Unit, UnitAction, UnitId, UnitTags, WeaponDef, WeaponKind, WeaponProficiency,
    WeaponRank,
};

fn terrain() -> Arc<TerrainTable> {
    Arc::new(TerrainTable {
        movement_types: vec!["foot".into()],
        terrains: vec![TerrainRules {
            name: "Plain".into(),
            move_cost: vec![Some(1)],
            defense: 0,
            avoid: 0,
            heal_percent: 0,
        }],
    })
}

fn classes() -> Arc<ClassTable> {
    let fighter = ClassDef {
        id: ClassId("fighter".into()),
        name: "Fighter".into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 3,
        base: Stats::default(),
        caps: Stats::default(),
        growths: Growths::default(),
        weapons: vec![WeaponProficiency {
            kind: WeaponKind::Sword,
            start: WeaponRank::E,
            max: WeaponRank::S,
        }],
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
    Arc::new(ClassTable {
        classes: BTreeMap::from([(fighter.id.clone(), fighter)]),
        ..ClassTable::default()
    })
}

fn blade(might: i32) -> ItemDef {
    ItemDef::Weapon(WeaponDef {
        name: "Blade".into(),
        kind: WeaponKind::Sword,
        rank: WeaponRank::E,
        might,
        hit: 60,
        crit: 0,
        weight: 0,
        min_range: 1,
        max_range: 1,
        damage_type: DamageType::Physical,
        durability: 20,
        effective: vec![],
        price: 0,
    })
}

/// `blade` (60 hit, 5 might), `dull_blade` (might 3) and `potion`.
fn items() -> Arc<ItemTable> {
    let potion = ItemDef::Consumable(ConsumableDef {
        name: "Potion".into(),
        effect: ConsumableEffect::Heal(10),
        price: 0,
    });
    Arc::new(ItemTable {
        items: BTreeMap::from([
            (ItemId::new("blade"), blade(5)),
            (ItemId::new("dull_blade"), blade(3)),
            (ItemId::new("potion"), potion),
        ]),
        ..ItemTable::default()
    })
}

/// A sturdy unit (40 HP) with a 60-hit, 5-damage weapon (and a 3-damage
/// spare): nobody falls in the script, and every strike's roll matters.
fn unit(id: u32, faction: Faction, x: i32, y: i32) -> Unit {
    let loadout = LoadoutDef {
        weapons: vec![ItemId::new("blade"), ItemId::new("dull_blade")],
        ..LoadoutDef::default()
    };
    Unit {
        id: UnitId(id),
        character: None,
        name: format!("u{id}"),
        class: ClassId("fighter".into()),
        level: 1,
        exp: 0,
        class_records: BTreeMap::new(),
        stats: Stats::from_growable([40, 0, 0, 0, 0, 0, 0], 3),
        hp: 40,
        faction,
        pos: Pos::new(x, y),
        acted: false,
        is_lord: id == 1,
        weapon_ranks: BTreeMap::new(),
        map_label: "Un".into(),
        weapon_exp: BTreeMap::new(),
        loadout: trpg_core::Loadout::default(),
        consumables: vec![ItemId::new("potion")],
        personal_spells: vec![],
        learned: BTreeSet::from([SpellId::new("bolt")]),
        spells: SpellState::default(),
    }
    .with_loadout(&loadout, &classes(), &items())
    .unwrap_or_else(|e| panic!("{e}"))
}

/// `bolt`: an attack spell with the blade's numbers (60 hit, 5 might), range
/// 1–2, 3 uses.
fn spells() -> Arc<SpellTable> {
    let bolt = SpellDef {
        id: SpellId::new("bolt"),
        name: "Bolt".into(),
        kind: SpellKind::Attack {
            might: 5,
            hit: 60,
            crit: 0,
            effective: vec![],
        },
        element: Element::None,
        min_range: 1,
        max_range: 2,
        uses: 3,
        terrain_effect: None,
    };
    Arc::new(SpellTable {
        spells: BTreeMap::from([(bolt.id.clone(), bolt)]),
    })
}

fn setup(seed: u64) -> BattleSetup {
    BattleSetup {
        map: BattleMap::new("Replay", Grid::filled(8, 5, TerrainId(0))),
        terrain: terrain(),
        classes: classes(),
        items: items(),
        spells: spells(),
        pack: BattlePack {
            items: vec![ItemId::new("potion")],
            cap: 1,
        },
        gold: 0,
        stock: Stock::default(),
        units: vec![
            unit(1, Faction::Player, 0, 0),
            unit(2, Faction::Player, 0, 2),
            unit(3, Faction::Enemy, 3, 0),
            unit(4, Faction::Enemy, 3, 2),
        ],
        reinforcements: vec![],
        objective: Objective::Survive { turns: 5 },
        rewind_charges: 3,
        seed,
    }
}

fn attack(unit: u32, x: i32, y: i32, target: u32) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest: Pos::new(x, y),
        action: UnitAction::Attack {
            target: UnitId(target),
            slot: 0,
        },
    }
}

/// Three turns of both sides trading blows, then equips (a weapon, then a
/// spell), potions, a wait, an enemy spell (countered with the equipped
/// spell) and a player spell out of the target's reach.
fn script() -> Vec<Command> {
    let mut cmds = Vec::new();
    for _ in 0..3 {
        cmds.extend([
            attack(1, 2, 0, 3),
            attack(2, 2, 2, 4),
            Command::EndPhase,
            attack(3, 3, 0, 1),
            attack(4, 3, 2, 2),
            Command::EndPhase,
        ]);
    }
    cmds.extend([
        Command::Equip {
            unit: UnitId(2),
            equipped: Equipped::Weapon(1),
        },
        Command::Equip {
            unit: UnitId(2),
            equipped: Equipped::Spell(SpellId::new("bolt")),
        },
        Command::Act {
            unit: UnitId(2),
            dest: Pos::new(1, 2),
            action: UnitAction::UseItem {
                pack_index: 0,
                target: UnitId(2),
            },
        },
        Command::Act {
            unit: UnitId(1),
            dest: Pos::new(1, 1),
            action: UnitAction::Wait,
        },
        Command::EndPhase,
        Command::Act {
            unit: UnitId(3),
            dest: Pos::new(3, 0),
            action: UnitAction::UseItem {
                pack_index: 0,
                target: UnitId(3),
            },
        },
        cast(4, 3, 2, 2),
        Command::EndPhase,
        cast(1, 2, 1, 3),
    ]);
    cmds
}

fn cast(unit: u32, x: i32, y: i32, target: u32) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest: Pos::new(x, y),
        action: UnitAction::Cast {
            spell: SpellId::new("bolt"),
            target: CastTarget::Unit(UnitId(target)),
        },
    }
}

/// Applies `cmds` to `state`, returning every event.
fn run(state: &mut BattleState, cmds: &[Command]) -> Vec<Event> {
    cmds.iter()
        .flat_map(|c| state.apply(c).unwrap_or_else(|e| panic!("{c:?}: {e}")))
        .collect()
}

fn play(seed: u64) -> (BattleState, Vec<Event>) {
    let (mut state, mut events) = BattleState::new(setup(seed));
    events.extend(run(&mut state, &script()));
    (state, events)
}

#[test]
fn same_seed_and_commands_give_identical_events() {
    let (state_a, a) = play(42);
    let (state_b, b) = play(42);
    assert_eq!(a, b);
    assert_eq!(state_a, state_b);
    // The rolls matter: the script has both hits and misses…
    let strikes: Vec<bool> = a
        .iter()
        .filter_map(|e| match e {
            Event::CombatResolved { outcome, .. } => Some(outcome.strikes.iter().map(|s| s.hit)),
            _ => None,
        })
        .flatten()
        .collect();
    // 12 combats of one strike each way, then 2 casts (one not countered).
    assert_eq!(strikes.len(), 27);
    assert_eq!(
        a.iter()
            .filter(|e| matches!(e, Event::SpellUsesChanged { .. }))
            .count(),
        3
    );
    assert!(strikes.contains(&true) && strikes.contains(&false));
    // …so another seed plays out differently.
    assert_ne!(play(43).1, a);
}

#[test]
fn saving_and_loading_mid_battle_changes_nothing() {
    let cmds = script();
    let (straight, events) = play(42);
    for split in 0..=cmds.len() {
        let (mut state, mut log) = BattleState::new(setup(42));
        log.extend(run(&mut state, &cmds[..split]));
        let saved = ron::to_string(&state).unwrap();
        let mut loaded: BattleState = ron::from_str(&saved).unwrap();
        loaded.restore_tables(terrain(), classes(), items(), spells());
        assert_eq!(loaded, state, "split {split}");
        log.extend(run(&mut loaded, &cmds[split..]));
        assert_eq!(log, events, "split {split}");
        assert_eq!(loaded, straight, "split {split}");
    }
}
