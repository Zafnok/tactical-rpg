//! Replay and save/load determinism (ADR-0007 layer 5): the same setup, seed
//! and commands give identical events, and a battle saved mid-way, loaded and
//! continued gives the same events as one played straight through. The
//! script includes terrain magic, so the changed tiles and the burning
//! forest are part of what must replay and survive a save, and levels are
//! on, so unit EXP, level ups (which roll the battle's RNG) and class points
//! are too. Dialogue triggers (0705) fire along the way, so which have
//! fired must replay and survive a save as well.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use trpg_core::{
    ActiveEffect, BattleMap, BattlePack, BattleSetup, BattleState, CastTarget, CharacterId,
    ClassDef, ClassId, ClassTable, CombatMods, Command, ConsumableDef, ConsumableEffect,
    DamageType, EffectDuration, Element, Equipped, Event, Faction, Grid, Growths, ItemDef, ItemId,
    ItemTable, LoadoutDef, MovementTypeId, Objective, Phase, Pos, SkillCost, SkillDef, SkillId,
    SkillKind, SkillTable, SpellDef, SpellId, SpellKind, SpellState, SpellTable, Stance, StatKind,
    Stats, Stock, TerrainEffect, TerrainId, TerrainRules, TerrainTable, TileRect, TimedMods,
    Trigger, TriggerWhen, Unit, UnitAction, UnitId, UnitTags, WeaponDef, WeaponKind,
    WeaponProficiency, WeaponRank, WeaponReq, Who,
};

const FOREST: TerrainId = TerrainId(1);
const BURNING: TerrainId = TerrainId(2);
const BURNT: TerrainId = TerrainId(3);
const WATER: TerrainId = TerrainId(4);
const ICE: TerrainId = TerrainId(5);

/// Plain, forest, burning, burnt, water and ice.
fn terrain() -> Arc<TerrainTable> {
    let rules = |name: &str, cost| TerrainRules {
        name: name.into(),
        move_cost: vec![cost],
        defense: 0,
        avoid: 0,
        heal_percent: 0,
    };
    Arc::new(TerrainTable {
        movement_types: vec!["foot".into()],
        terrains: vec![
            rules("Plain", Some(1)),
            rules("Forest", Some(2)),
            rules("Burning", None),
            rules("Burnt", Some(1)),
            rules("Water", Some(5)),
            rules("Ice", Some(1)),
        ],
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
        // Only HP and Res grow: sturdier units, same weapon damage.
        growths: Growths([80, 0, 0, 0, 0, 0, 30]),
        weapons: vec![WeaponProficiency {
            kind: WeaponKind::Sword,
            start: WeaponRank::E,
            max: WeaponRank::S,
        }],
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: Some(SkillId::new("guard_strike")),
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    };
    Arc::new(ClassTable {
        classes: BTreeMap::from([(fighter.id.clone(), fighter)]),
        level_cap: 99,
        class_level_cap: 10,
        min_gains: vec![1],
        cp_per_class_level: vec![10],
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
        arts: vec![],
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
        character: Some(CharacterId(format!("c{id}"))),
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
        role: trpg_core::Role::Regular,
        ai: trpg_core::AiBehavior::Aggressive,
        weapon_ranks: BTreeMap::new(),
        map_label: "Un".into(),
        weapon_exp: BTreeMap::new(),
        loadout: trpg_core::Loadout::default(),
        personal_spells: vec![],
        learned: BTreeSet::from([
            SpellId::new("bolt"),
            SpellId::new("fire"),
            SpellId::new("frost"),
        ]),
        spells: SpellState::default(),
        learned_skills: BTreeSet::new(),
        effects: Vec::new(),
        skill_uses: trpg_core::SkillUses::default(),
        talent: None,
    }
    .with_loadout(&loadout, &classes(), &items())
    .unwrap_or_else(|e| panic!("{e}"))
}

/// `guard_strike`: a combat active (2 durability) with hit +10 and a stance
/// rider, Def +2 until the user's next phase.
fn skills() -> Arc<SkillTable> {
    let def = SkillDef {
        id: SkillId::new("guard_strike"),
        name: "Guard Strike".into(),
        family: "guard_strike".into(),
        rank: 1,
        kind: SkillKind::Active {
            cost: SkillCost::Durability(2),
            effect: ActiveEffect::Strike {
                with: WeaponReq::Any,
                mods: CombatMods {
                    hit: 10,
                    ..CombatMods::default()
                },
                range: 0,
                stance: Some(Stance {
                    mods: TimedMods {
                        stats: vec![(StatKind::Def, 2)],
                        combat: CombatMods::default(),
                    },
                    this_combat: true,
                }),
                post_move: 0,
                drain: false,
            },
        },
    };
    Arc::new(SkillTable {
        skills: BTreeMap::from([(def.id.clone(), def)]),
    })
}

/// `bolt`: an attack spell with the blade's numbers (60 hit, 5 might), range
/// 1–2, 3 uses; `fire` (forest → burning, then burnt) and `frost` (water →
/// ice), for tile casts.
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
    let fire = SpellDef {
        id: SpellId::new("fire"),
        name: "Fire".into(),
        element: Element::Fire,
        terrain_effect: Some(TerrainEffect {
            from: vec![FOREST],
            to: BURNING,
            lasts: EffectDuration::UntilCastersNextPhase {
                then: BURNT,
                damage: 5,
            },
        }),
        ..bolt.clone()
    };
    let frost = SpellDef {
        id: SpellId::new("frost"),
        name: "Frost".into(),
        element: Element::Ice,
        terrain_effect: Some(TerrainEffect {
            from: vec![WATER],
            to: ICE,
            lasts: EffectDuration::Permanent,
        }),
        ..bolt.clone()
    };
    Arc::new(SpellTable {
        spells: [bolt, fire, frost]
            .into_iter()
            .map(|s| (s.id.clone(), s))
            .collect(),
    })
}

/// Plain, with a forest at (1,4) and water at (2,4).
fn setup(seed: u64) -> BattleSetup {
    let mut tiles = Grid::filled(8, 5, TerrainId(0));
    for (x, t) in [(1, FOREST), (2, WATER)] {
        if let Some(tile) = tiles.get_mut(Pos::new(x, 4)) {
            *tile = t;
        }
    }
    BattleSetup {
        map: BattleMap::new("Replay", tiles),
        terrain: terrain(),
        classes: classes(),
        items: items(),
        spells: spells(),
        skills: skills(),
        arts: Arc::default(),
        supports: Arc::default(),
        bonds: trpg_core::SupportBook::default(),
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
        objective: Objective::Survive { turns: 8 },
        rewind_charges: 3,
        seed,
        triggers: triggers(),
        mode: trpg_core::GameMode::Classic,
        battle_notes: vec![],
    }
}

/// One trigger of each kind: turn 2's player phase; a player unit ending a
/// move on row 1; unit 3 in a combat (once) and unit 4 (every time); unit 4
/// at half HP; unit 3 falling (it never does); units 1 and 2 talking.
fn triggers() -> Vec<Trigger> {
    let c = |id: u32| CharacterId(format!("c{id}"));
    let t = |when, scene: &str, once| Trigger {
        when,
        scene: scene.into(),
        once,
    };
    vec![
        t(
            TriggerWhen::TurnStart {
                turn: 2,
                phase: Phase::Player,
            },
            "turn_2",
            true,
        ),
        t(
            TriggerWhen::UnitEntersArea {
                who: Who::Faction(Faction::Player),
                area: TileRect {
                    x: 0,
                    y: 1,
                    w: 8,
                    h: 1,
                },
            },
            "row_1",
            true,
        ),
        t(
            TriggerWhen::CombatStart {
                unit: c(3),
                against: None,
            },
            "engage_3",
            true,
        ),
        t(
            TriggerWhen::CombatStart {
                unit: c(4),
                against: None,
            },
            "engage_4",
            false,
        ),
        t(TriggerWhen::HalfHp { unit: c(4) }, "half_4", true),
        t(
            TriggerWhen::UnitFell {
                unit: c(3),
                mode: None,
                recruit: true,
            },
            "fall_3",
            true,
        ),
        t(TriggerWhen::Talk { a: c(1), b: c(2) }, "talk", true),
    ]
}

fn attack(unit: u32, x: i32, y: i32, target: u32) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest: Pos::new(x, y),
        action: UnitAction::Attack {
            target: UnitId(target),
            slot: 0,
            active: None,
            art: None,
        },
    }
}

/// Three turns of both sides trading blows, then equips (a weapon, then a
/// spell), potions, a talk and a wait, an enemy spell (countered with the equipped
/// spell), a player spell out of the target's reach, then a forest burnt
/// (it burns out a round later), water frozen, and an attack with a combat
/// active whose stance lasts through the enemy phase.
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
        Command::Talk {
            unit: UnitId(1),
            dest: Pos::new(1, 1),
            target: UnitId(2),
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
            action: UnitAction::Wait,
        },
        cast(4, 3, 2, 2),
        Command::EndPhase,
        cast(1, 2, 1, 3),
        tile_cast(2, 1, 3, "fire", 1, 4),
        Command::EndPhase,
        Command::EndPhase,
        tile_cast(1, 2, 3, "frost", 2, 4),
        Command::EndPhase,
        Command::EndPhase,
        Command::Act {
            unit: UnitId(1),
            dest: Pos::new(2, 2),
            action: UnitAction::Attack {
                target: UnitId(4),
                slot: 0,
                active: Some(SkillId::new("guard_strike")),
                art: None,
            },
        },
        Command::EndPhase,
        Command::EndPhase,
    ]);
    cmds
}

fn tile_cast(unit: u32, x: i32, y: i32, spell: &str, tx: i32, ty: i32) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest: Pos::new(x, y),
        action: UnitAction::Cast {
            spell: SpellId::new(spell),
            target: CastTarget::Tile(Pos::new(tx, ty)),
            active: None,
        },
    }
}

fn cast(unit: u32, x: i32, y: i32, target: u32) -> Command {
    Command::Act {
        unit: UnitId(unit),
        dest: Pos::new(x, y),
        action: UnitAction::Cast {
            spell: SpellId::new("bolt"),
            target: CastTarget::Unit(UnitId(target)),
            active: None,
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
    // 12 combats of one strike each way, then 2 casts (one not countered),
    // then the combat with the active.
    assert_eq!(strikes.len(), 29);
    let skill_events = a
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::SkillUsed { .. }
                    | Event::DurabilitySpent { .. }
                    | Event::EffectApplied { .. }
                    | Event::EffectExpired { .. }
            )
        })
        .count();
    assert_eq!(skill_events, 4);
    // Four casts, and two counters with an equipped spell.
    assert_eq!(
        a.iter()
            .filter(|e| matches!(e, Event::SpellUsesChanged { .. }))
            .count(),
        6
    );
    assert!(strikes.contains(&true) && strikes.contains(&false));
    // The forest burnt then burnt out, and the water froze.
    let terrain: Vec<(TerrainId, TerrainId)> = a
        .iter()
        .filter_map(|e| match e {
            Event::TerrainChanged { from, to, .. } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    assert_eq!(terrain, [(FOREST, BURNING), (BURNING, BURNT), (WATER, ICE)]);
    // Player units gained EXP, levels and class levels on the way.
    let count = |f: fn(&Event) -> bool| a.iter().filter(|e| f(e)).count();
    assert!(count(|e| matches!(e, Event::ExpGained { .. })) > 0);
    assert!(count(|e| matches!(e, Event::LeveledUp { .. })) > 0);
    assert!(count(|e| matches!(e, Event::ClassLeveledUp { .. })) > 0);
    assert!(state_a.units().iter().any(|u| u.level > 1));
    // The triggers fired: turn 2's start; unit 3's engage line once; unit
    // 4's before each of its 8 combats (6 trades, its cast, the active
    // attack); the talk; row 1 once, though two moves end there; and
    // unit 4's half-HP line once.
    let scenes: Vec<&str> = a
        .iter()
        .filter_map(|e| match e {
            Event::SceneTriggered { scene } => Some(scene.as_str()),
            _ => None,
        })
        .collect();
    let seen = |s: &str| scenes.iter().filter(|&&x| x == s).count();
    assert_eq!(
        [
            seen("turn_2"),
            seen("engage_3"),
            seen("engage_4"),
            seen("row_1"),
            seen("talk"),
            seen("fall_3"),
            seen("half_4"),
        ],
        [1, 1, 8, 1, 1, 0, 1],
        "{scenes:?}"
    );
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
        loaded.restore_tables(&trpg_core::GameTables {
            terrain: terrain(),
            classes: classes(),
            items: items(),
            spells: spells(),
            skills: skills(),
            arts: Arc::default(),
            supports: Arc::default(),
        });
        assert_eq!(loaded, state, "split {split}");
        log.extend(run(&mut loaded, &cmds[split..]));
        assert_eq!(log, events, "split {split}");
        assert_eq!(loaded, straight, "split {split}");
    }
}
