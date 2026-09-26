//! Tests of the battle rules. Maps are drawn in ASCII: `.` plain, `f` forest
//! (cost 2, +2 Def), `#` wall, `?` a terrain id missing from the table.
//!
//! Test units have 10 HP and 0 in every other stat except Mov 3, so combat
//! is exact: every strike hits (hit 100, no avoid), never crits, deals the
//! weapon's might, and each side strikes once.
//!
//! Test weapons are swords without a trait, named by their numbers
//! (`w:min:max:might:hit:crit`, see [`weapon_id`]); each setup's item table is
//! built from the ids its units carry, plus a few fixed items
//! ([`fixed_items`]).

use std::collections::BTreeMap;
use std::sync::Arc;

use proptest::prelude::*;

use super::*;
use crate::class::{ArmourWeight, ClassDef, UnitTag, UnitTags, WeaponProficiency};
use crate::combat::DamageType;
use crate::geom::Grid;
use crate::item::{
    ArmourDef, ConsumableDef, ItemDef, Loadout, WEAPON_SLOTS, WeaponDef, WeaponInstance,
};
use crate::movement::reachable;
use crate::stats::{Growths, StatValue, Stats};
use crate::terrain::{MovementTypeId, TerrainId, TerrainRules};
use crate::weapon::WeaponKind;

const OPEN: [&str; 5] = [
    "........", //
    "........", //
    "..f.....", //
    "........", //
    "........", //
];

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn terrain() -> TerrainTable {
    let rules = |name: &str, cost: Option<u8>, defense| TerrainRules {
        name: name.into(),
        move_cost: vec![cost],
        defense,
        avoid: 0,
        heal_percent: 0,
    };
    TerrainTable {
        movement_types: vec!["foot".into()],
        terrains: vec![
            rules("Plain", Some(1), 0),
            rules("Forest", Some(2), 2),
            rules("Wall", None, 0),
        ],
    }
}

fn class(id: &str, tags: UnitTags) -> ClassDef {
    ClassDef {
        id: ClassId(id.into()),
        name: id.into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 3,
        base: Stats::default(),
        caps: Stats::default(),
        growths: Growths::default(),
        weapons: [
            WeaponKind::Sword,
            WeaponKind::Spear,
            WeaponKind::Axe,
            WeaponKind::Bow,
            WeaponKind::Gauntlet,
        ]
        .map(|kind| WeaponProficiency {
            kind,
            start: WeaponRank::E,
            max: WeaponRank::S,
        })
        .to_vec(),
        armour: vec![ArmourWeight::Light],
        tags,
        promotes_to: vec![],
        active: None,
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    }
}

fn classes() -> ClassTable {
    let flier = UnitTags::from_tags(&[UnitTag::Flying]);
    ClassTable {
        classes: [class("fighter", UnitTags::default()), class("flier", flier)]
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect(),
        hard_ceilings: Stats::from_growable([99; 7], 15),
        ..ClassTable::default()
    }
}

fn map(rows: &[&str]) -> BattleMap {
    let cells = rows
        .iter()
        .flat_map(|r| r.chars())
        .map(|c| match c {
            '.' => TerrainId(0),
            'f' => TerrainId(1),
            '#' => TerrainId(2),
            _ => TerrainId(9),
        })
        .collect();
    let w = u16::try_from(rows[0].len()).unwrap();
    let h = u16::try_from(rows.len()).unwrap();
    BattleMap {
        name: "Test".into(),
        tiles: Grid::from_cells(w, h, cells).unwrap(),
    }
}

/// A test sword's id: `w:min:max:might:hit:crit`.
fn weapon_id(min: u32, max: u32, might: StatValue, hit: StatValue, crit: StatValue) -> ItemId {
    ItemId(format!("w:{min}:{max}:{might}:{hit}:{crit}"))
}

/// A hit-100 test sword.
fn weapon(min: u32, max: u32, might: StatValue) -> ItemId {
    weapon_id(min, max, might, 100, 0)
}

fn sword(name: &str) -> WeaponDef {
    WeaponDef {
        name: name.into(),
        kind: WeaponKind::Sword,
        rank: WeaponRank::E,
        might: 0,
        hit: 100,
        crit: 0,
        weight: 0,
        min_range: 1,
        max_range: 1,
        damage_type: DamageType::Physical,
        durability: 20,
        effective: vec![],
        price: 0,
    }
}

/// The weapon a `w:…` id describes.
fn parse_weapon(id: &ItemId) -> Option<WeaponDef> {
    let n: Vec<i32> =
        id.0.strip_prefix("w:")?
            .split(':')
            .map(|x| x.parse().unwrap())
            .collect();
    let range = |i: usize| u32::try_from(n[i]).unwrap();
    Some(WeaponDef {
        min_range: range(0),
        max_range: range(1),
        might: n[2],
        hit: n[3],
        crit: n[4],
        ..sword(&id.0)
    })
}

/// Items every test table has: `flier_bow` (range 1–2, might 3, ×3 against
/// fliers), `master_sword` (rank S), `axe`, `vest` (Def +1), `mail`
/// (Medium, Def +2), `potion` (heals 4), `elixir` (heals all).
fn fixed_items() -> ItemTable {
    let flier_bow = WeaponDef {
        kind: WeaponKind::Bow,
        might: 3,
        max_range: 2,
        effective: vec![(UnitTag::Flying, 3)],
        ..sword("Flier Bow")
    };
    let master_sword = WeaponDef {
        rank: WeaponRank::S,
        might: 5,
        ..sword("Master Sword")
    };
    let axe = WeaponDef {
        kind: WeaponKind::Axe,
        might: 2,
        ..sword("Axe")
    };
    let armour = |name: &str, weight_class, def| ArmourDef {
        name: name.into(),
        weight_class,
        bonus: Stats {
            def,
            ..Stats::default()
        },
        weight: 0,
        price: 0,
    };
    let consumable = |name: &str, effect| ConsumableDef {
        name: name.into(),
        effect,
        price: 0,
    };
    let entries = [
        ("flier_bow", ItemDef::Weapon(flier_bow)),
        ("master_sword", ItemDef::Weapon(master_sword)),
        ("axe", ItemDef::Weapon(axe)),
        (
            "vest",
            ItemDef::Armour(armour("Vest", ArmourWeight::Light, 1)),
        ),
        (
            "mail",
            ItemDef::Armour(armour("Mail", ArmourWeight::Medium, 2)),
        ),
        (
            "potion",
            ItemDef::Consumable(consumable("Potion", ConsumableEffect::Heal(4))),
        ),
        (
            "elixir",
            ItemDef::Consumable(consumable("Elixir", ConsumableEffect::HealFull)),
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

/// The fixed items plus every `w:…` weapon the units carry.
fn items_for<'a>(units: impl IntoIterator<Item = &'a Unit>) -> ItemTable {
    let mut table = fixed_items();
    for u in units {
        for w in u.loadout.weapons.iter().flatten() {
            if let Some(def) = parse_weapon(&w.def) {
                table.items.insert(w.def.clone(), ItemDef::Weapon(def));
            }
        }
    }
    table
}

/// `u` carrying `weapons` in slots 0.., the first equipped.
fn carrying(u: Unit, weapons: &[ItemId]) -> Unit {
    let mut loadout = Loadout {
        equipped: (!weapons.is_empty()).then_some(0),
        ..Loadout::default()
    };
    for (slot, id) in weapons.iter().enumerate() {
        loadout.weapons[slot] = Some(WeaponInstance {
            def: id.clone(),
            durability_left: 20,
        });
    }
    Unit { loadout, ..u }
}

fn item(id: &str) -> ItemId {
    ItemId::new(id)
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
        stats: Stats::from_growable([10, 0, 0, 0, 0, 0, 0], 3),
        hp: 10,
        faction,
        pos,
        acted: false,
        is_lord: false,
        weapon_ranks: BTreeMap::new(),
        map_label: "Un".into(),
        weapon_exp: BTreeMap::new(),
        loadout: Loadout::default(),
        consumables: vec![],
    };
    carrying(u, &[weapon(1, 1, 3)])
}

fn lord(id: u32, pos: Pos) -> Unit {
    Unit {
        is_lord: true,
        ..unit(id, Faction::Player, pos)
    }
}

/// With a weapon of this might.
fn armed(u: Unit, might: StatValue) -> Unit {
    carrying(u, &[weapon(1, 1, might)])
}

/// Lord 1 at (0,0) and unit 2 at (0,2); enemies 3 at (7,0) and 4 at (7,2):
/// out of each other's reach.
fn cast() -> Vec<Unit> {
    vec![
        lord(1, p(0, 0)),
        unit(2, Faction::Player, p(0, 2)),
        unit(3, Faction::Enemy, p(7, 0)),
        unit(4, Faction::Enemy, p(7, 2)),
    ]
}

fn setup(units: Vec<Unit>) -> BattleSetup {
    BattleSetup {
        map: map(&OPEN),
        terrain: Arc::new(terrain()),
        classes: Arc::new(classes()),
        items: Arc::new(items_for(&units)),
        pack: BattlePack {
            items: vec![item("potion"), item("elixir")],
            cap: 3,
        },
        units,
        reinforcements: vec![],
        objective: Objective::Rout { turn_limit: None },
        rewind_charges: 3,
        seed: 7,
    }
}

fn start(setup: BattleSetup) -> BattleState {
    let (state, events) = BattleState::new(setup);
    assert!(!events.is_empty());
    state
}

fn with_objective(units: Vec<Unit>, objective: Objective) -> BattleState {
    start(BattleSetup {
        objective,
        ..setup(units)
    })
}

fn act(s: &mut BattleState, id: u32, dest: Pos, action: UnitAction) -> Vec<Event> {
    s.apply(&Command::Act {
        unit: UnitId(id),
        dest,
        action,
    })
    .unwrap()
}

fn attack(target: u32) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot: 0,
    }
}

fn end(s: &mut BattleState) -> Vec<Event> {
    s.apply(&Command::EndPhase).unwrap()
}

fn started(turn: Turn, phase: Phase) -> Event {
    Event::PhaseStarted { turn, phase }
}

fn ended(outcome: Outcome) -> Event {
    Event::BattleEnded { outcome }
}

/// Ends phases `n` times, returning every phase that started.
fn walk(s: &mut BattleState, n: usize) -> Vec<(Turn, Phase)> {
    let mut seen = Vec::new();
    for _ in 0..n {
        let events = end(s);
        assert_eq!(events.len(), 1, "{events:?}");
        if let Some(Event::PhaseStarted { turn, phase }) = events.first() {
            seen.push((*turn, *phase));
        }
    }
    seen
}

/// Applies `cmd`, expecting `err`, and checks the state didn't change.
fn refused(s: &mut BattleState, cmd: &Command, err: CommandError) {
    let before = s.clone();
    assert_eq!(s.apply(cmd), Err(err), "{cmd:?}");
    assert_eq!(*s, before, "{cmd:?} changed the state");
}

fn refused_act(s: &mut BattleState, id: u32, dest: Pos, action: UnitAction, err: CommandError) {
    let cmd = Command::Act {
        unit: UnitId(id),
        dest,
        action,
    };
    refused(s, &cmd, err);
}

// ---- Phases and turns ------------------------------------------------------

#[test]
fn phase_helpers() {
    assert_eq!(Phase::of(Faction::Player), Phase::Player);
    assert_eq!(Phase::of(Faction::Enemy), Phase::Enemy);
    assert_eq!(Phase::of(Faction::Ally), Phase::Other);
    assert_eq!(Phase::of(Faction::Neutral), Phase::Other);
    assert_eq!(Phase::Player.next(), Phase::Enemy);
    assert_eq!(Phase::Enemy.next(), Phase::Other);
    assert_eq!(Phase::Other.next(), Phase::Player);
}

#[test]
fn new_starts_turn_one_player_phase() {
    let (s, events) = BattleState::new(setup(cast()));
    assert_eq!(events, [started(1, Phase::Player)]);
    assert_eq!((s.turn(), s.phase(), s.outcome()), (1, Phase::Player, None));
    assert_eq!(s.units(), cast().as_slice());
    assert!(s.fallen().is_empty());
    assert!(s.reinforcements().is_empty());
    assert_eq!(s.rewind_charges(), 3);
    assert_eq!(s.objective(), Objective::Rout { turn_limit: None });
    assert_eq!(s.map(), &map(&OPEN));
    assert_eq!(s.terrain(), &terrain());
    assert_eq!(s.classes(), &classes());
    assert_eq!(s.unit(UnitId(3)).map(|u| u.pos), Some(p(7, 0)));
    assert_eq!(s.unit(UnitId(9)), None);
}

#[test]
fn three_turns_with_other_units() {
    use Phase::{Enemy, Other, Player};
    let mut units = cast();
    units.push(unit(5, Faction::Ally, p(0, 4)));
    let mut s = start(setup(units));
    assert_eq!(
        walk(&mut s, 9),
        [
            (1, Enemy),
            (1, Other),
            (2, Player),
            (2, Enemy),
            (2, Other),
            (3, Player),
            (3, Enemy),
            (3, Other),
            (4, Player),
        ]
    );
    assert_eq!((s.turn(), s.phase()), (4, Player));
}

#[test]
fn three_turns_without_other_units_skip_the_other_phase() {
    use Phase::{Enemy, Player};
    let mut s = start(setup(cast()));
    assert_eq!(
        walk(&mut s, 6),
        [
            (1, Enemy),
            (2, Player),
            (2, Enemy),
            (3, Player),
            (3, Enemy),
            (4, Player)
        ]
    );
}

#[test]
fn a_phase_without_enemies_is_skipped() {
    use Phase::{Other, Player};
    let units = vec![
        lord(1, p(0, 0)),
        unit(5, Faction::Neutral, p(4, 4)),
        unit(6, Faction::Ally, p(5, 4)),
    ];
    let mut s = with_objective(units, Objective::Survive { turns: 9 });
    assert_eq!(
        walk(&mut s, 4),
        [(1, Other), (2, Player), (2, Other), (3, Player)]
    );
}

#[test]
fn other_phase_units_act_in_the_other_phase() {
    let mut units = cast();
    units.push(unit(5, Faction::Ally, p(0, 4)));
    units.push(unit(6, Faction::Neutral, p(1, 4)));
    let mut s = start(setup(units));
    end(&mut s);
    end(&mut s);
    assert_eq!(s.phase(), Phase::Other);
    assert_eq!(
        act(&mut s, 5, p(0, 4), UnitAction::Wait),
        [Event::UnitActed { unit: UnitId(5) }]
    );
    act(&mut s, 6, p(1, 3), UnitAction::Wait);
}

#[test]
fn units_are_ready_again_only_at_their_own_phase() {
    let mut s = start(setup(cast()));
    act(&mut s, 1, p(0, 0), UnitAction::Wait);
    end(&mut s);
    // Still done (drawn dimmed) during the enemy phase.
    assert!(s.unit(UnitId(1)).unwrap().acted);
    act(&mut s, 3, p(6, 0), UnitAction::Wait);
    end(&mut s);
    assert!(!s.unit(UnitId(1)).unwrap().acted);
    assert!(s.unit(UnitId(3)).unwrap().acted);
    act(&mut s, 1, p(1, 0), UnitAction::Wait);
    end(&mut s);
    assert!(!s.unit(UnitId(3)).unwrap().acted);
}

#[test]
fn ending_a_phase_with_units_still_ready_is_allowed() {
    let mut s = start(setup(cast()));
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    assert!(!s.unit(UnitId(1)).unwrap().acted);
}

// ---- Acting ------------------------------------------------------------------

#[test]
fn wait_moves_and_marks_the_unit_done() {
    let mut s = start(setup(cast()));
    assert_eq!(
        act(&mut s, 2, p(2, 1), UnitAction::Wait),
        [
            Event::UnitMoved {
                unit: UnitId(2),
                path: vec![p(0, 2), p(0, 1), p(1, 1), p(2, 1)],
            },
            Event::UnitActed { unit: UnitId(2) },
        ]
    );
    let u = s.unit(UnitId(2)).unwrap();
    assert_eq!((u.pos, u.acted), (p(2, 1), true));
}

#[test]
fn waiting_in_place_has_no_move_event() {
    let mut s = start(setup(cast()));
    assert_eq!(
        act(&mut s, 1, p(0, 0), UnitAction::Wait),
        [Event::UnitActed { unit: UnitId(1) }]
    );
}

#[test]
fn attack_trades_blows_using_each_units_tile() {
    let mut units = cast();
    units[3].pos = p(3, 2);
    let mut s = start(setup(units));
    // Unit 2 moves onto the forest (+2 Def) next to enemy 4 and attacks.
    let events = act(&mut s, 2, p(2, 2), attack(4));
    let [
        Event::UnitMoved { .. },
        Event::CombatResolved {
            attacker,
            defender,
            forecast,
            outcome,
        },
        Event::WeaponExpGained { .. },
        Event::WeaponExpGained { .. },
        Event::UnitActed { unit },
    ] = events.as_slice()
    else {
        panic!("{events:?}");
    };
    assert_eq!(
        (*attacker, *defender, *unit),
        (UnitId(2), UnitId(4), UnitId(2))
    );
    assert_eq!(forecast.attacker.damage, 3);
    assert_eq!(forecast.attacker.hit, 100);
    assert_eq!(forecast.attacker.strikes, 1);
    assert_eq!(forecast.defender.map(|d| d.damage), Some(1));
    assert_eq!((outcome.attacker_hp, outcome.defender_hp), (9, 7));
    assert_eq!(outcome.strikes.len(), 2);
    assert_eq!(s.unit(UnitId(2)).unwrap().hp, 9);
    assert_eq!(s.unit(UnitId(4)).unwrap().hp, 7);

    // Now enemy 4 attacks unit 2, which is still on the forest.
    end(&mut s);
    let events = act(&mut s, 4, p(3, 2), attack(2));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    assert_eq!(forecast.attacker.damage, 1);
    assert_eq!(s.unit(UnitId(2)).unwrap().hp, 8);
    assert_eq!(s.unit(UnitId(4)).unwrap().hp, 4);
}

#[test]
fn combat_uses_the_class_tags_and_weapon_rank() {
    let mut units = cast();
    units[1] = carrying(units[1].clone(), &[item("flier_bow")]);
    units[1].weapon_ranks = BTreeMap::from([(WeaponKind::Bow, WeaponRank::S)]);
    units[3].pos = p(2, 2);
    units[3].class = ClassId("flier".into());
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    // ×3 against fliers; rank S bow: +4 attack speed = 2 strikes.
    assert!(forecast.attacker.effective);
    assert_eq!(forecast.attacker.damage, 9);
    assert_eq!(forecast.attacker.strikes, 2);
    // The flier can't counter at range 2.
    assert_eq!(forecast.defender, None);
}

#[test]
fn a_rank_below_s_is_read_from_the_unit() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 1);
    // Rank C gives +1 attack speed; the enemy's Spd 0, so 1 strike. Missing
    // ranks count as E (+0) for the defender.
    units[1].weapon_ranks = BTreeMap::from([(WeaponKind::Sword, WeaponRank::C)]);
    units[1].stats.spd = 3;
    units[3].pos = p(1, 2);
    units[3] = armed(units[3].clone(), 1);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    // AS 3 + 1 = 4 vs 0: exactly the first threshold.
    assert_eq!(forecast.attacker.strikes, 2);
}

#[test]
fn a_killing_blow_removes_the_defender() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 10);
    units[3].pos = p(3, 2);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    assert!(matches!(
        events.as_slice(),
        [
            Event::UnitMoved { .. },
            Event::CombatResolved { .. },
            Event::WeaponExpGained { .. },
            Event::UnitFell { unit: UnitId(4) },
            Event::UnitActed { unit: UnitId(2) },
        ]
    ));
    assert_eq!(s.unit(UnitId(4)), None);
    assert_eq!(s.fallen().len(), 1);
    assert_eq!((s.fallen()[0].id, s.fallen()[0].hp), (UnitId(4), 0));
    // Enemy 3 is left: not routed yet.
    assert_eq!(s.outcome(), None);
    // The tile is free for another unit.
    act(&mut s, 1, p(0, 0), UnitAction::Wait);
    end(&mut s);
    act(&mut s, 3, p(4, 0), UnitAction::Wait);
    end(&mut s);
    act(&mut s, 2, p(3, 2), UnitAction::Wait);
}

#[test]
fn an_attacker_that_falls_to_a_counter_doesnt_act() {
    let mut units = cast();
    units[1] = armed(units[1].clone(), 1);
    // 12 - 2 forest Def = 10.
    units[3] = armed(units[3].clone(), 12);
    units[3].pos = p(3, 2);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(2, 2), attack(4));
    assert!(matches!(
        events.as_slice(),
        [
            Event::UnitMoved { .. },
            Event::CombatResolved { .. },
            Event::WeaponExpGained {
                unit: UnitId(4),
                ..
            },
            Event::UnitFell { unit: UnitId(2) },
        ]
    ));
    assert_eq!(s.fallen()[0].id, UnitId(2));
    assert_eq!(s.unit(UnitId(4)).unwrap().hp, 9);
    // A non-lord falling with others left is not a defeat.
    assert_eq!(s.outcome(), None);
}

// ---- Errors: each leaves the state unchanged -----------------------------------

#[test]
fn error_battle_over() {
    let mut s = start(setup(vec![lord(1, p(0, 0))]));
    assert_eq!(s.outcome(), Some(Outcome::Victory));
    refused(&mut s, &Command::EndPhase, CommandError::BattleOver);
    refused_act(
        &mut s,
        1,
        p(0, 0),
        UnitAction::Wait,
        CommandError::BattleOver,
    );
}

#[test]
fn error_unknown_unit() {
    let mut s = start(BattleSetup {
        reinforcements: vec![Reinforcement {
            turn: 2,
            unit: unit(9, Faction::Player, p(4, 4)),
        }],
        ..setup(cast())
    });
    let err = |id| CommandError::UnknownUnit(UnitId(id));
    refused_act(&mut s, 99, p(0, 0), UnitAction::Wait, err(99));
    // Not arrived yet.
    refused_act(&mut s, 9, p(4, 4), UnitAction::Wait, err(9));
    refused_act(&mut s, 2, p(0, 2), attack(99), err(99));
}

#[test]
fn error_unit_fallen() {
    let mut units = cast();
    units[0] = armed(units[0].clone(), 10);
    units[2].pos = p(2, 0);
    let mut s = start(setup(units));
    act(&mut s, 1, p(1, 0), attack(3));
    let err = CommandError::UnitFallen(UnitId(3));
    refused_act(&mut s, 2, p(1, 1), attack(3), err.clone());
    end(&mut s);
    refused_act(&mut s, 3, p(2, 0), UnitAction::Wait, err);
}

#[test]
fn error_not_its_phase() {
    let mut s = start(setup(cast()));
    refused_act(
        &mut s,
        3,
        p(7, 0),
        UnitAction::Wait,
        CommandError::NotItsPhase {
            unit: UnitId(3),
            phase: Phase::Player,
        },
    );
    end(&mut s);
    refused_act(
        &mut s,
        1,
        p(0, 0),
        UnitAction::Wait,
        CommandError::NotItsPhase {
            unit: UnitId(1),
            phase: Phase::Enemy,
        },
    );
}

#[test]
fn error_already_acted() {
    let mut s = start(setup(cast()));
    act(&mut s, 1, p(1, 0), UnitAction::Wait);
    refused_act(
        &mut s,
        1,
        p(1, 0),
        UnitAction::Wait,
        CommandError::AlreadyActed(UnitId(1)),
    );
}

#[test]
fn error_unknown_class() {
    let mut units = cast();
    units[1].class = ClassId("ghost".into());
    units[3].pos = p(1, 1);
    units[3].class = ClassId("wraith".into());
    let mut s = start(setup(units));
    refused_act(
        &mut s,
        2,
        p(0, 2),
        UnitAction::Wait,
        CommandError::UnknownClass(ClassId("ghost".into())),
    );
    // The target's class is looked up for the combat.
    refused_act(
        &mut s,
        1,
        p(1, 0),
        attack(4),
        CommandError::UnknownClass(ClassId("wraith".into())),
    );
}

#[test]
fn error_off_map() {
    let mut units = cast();
    units[1].pos = p(20, 20);
    units[3].pos = p(-1, 0);
    let mut s = start(setup(units));
    refused_act(
        &mut s,
        2,
        p(20, 20),
        UnitAction::Wait,
        CommandError::OffMap(UnitId(2)),
    );
    refused_act(
        &mut s,
        1,
        p(0, 0),
        attack(4),
        CommandError::OffMap(UnitId(4)),
    );
}

#[test]
fn error_unknown_terrain() {
    let rows = ["?......."; 5];
    let mut units = cast();
    units[3].pos = p(1, 0);
    let mut s = start(BattleSetup {
        map: map(&rows),
        ..setup(units)
    });
    // The lord stands on the unknown tile.
    refused_act(
        &mut s,
        1,
        p(0, 0),
        attack(4),
        CommandError::UnknownTerrain(p(0, 0)),
    );
}

#[test]
fn error_cannot_stop() {
    let rows = [
        "...#....", //
        "........", //
        "........", //
        "........", //
        "........", //
    ];
    let mut units = cast();
    units[3].pos = p(1, 2);
    let mut s = start(BattleSetup {
        map: map(&rows),
        ..setup(units)
    });
    for dest in [
        p(0, 2),  // a friend's tile
        p(1, 2),  // an enemy's tile
        p(3, 0),  // a wall
        p(4, 0),  // too far
        p(-1, 0), // off the map
    ] {
        refused_act(
            &mut s,
            1,
            dest,
            UnitAction::Wait,
            CommandError::CannotStop(dest),
        );
    }
}

#[test]
fn error_not_hostile() {
    let mut units = cast();
    units.push(unit(5, Faction::Neutral, p(1, 1)));
    units.push(unit(6, Faction::Ally, p(2, 0)));
    let mut s = start(setup(units));
    for target in [2, 1, 5, 6] {
        refused_act(
            &mut s,
            1,
            p(1, 0),
            attack(target),
            CommandError::NotHostile(UnitId(target)),
        );
    }
}

#[test]
fn error_no_weapon() {
    let mut units = cast();
    units[1].loadout = Loadout::default();
    units[3].pos = p(1, 2);
    let mut s = start(setup(units));
    refused_act(
        &mut s,
        2,
        p(0, 2),
        attack(4),
        CommandError::EmptySlot {
            unit: UnitId(2),
            slot: 0,
        },
    );
}

#[test]
fn error_out_of_range() {
    let mut units = cast();
    units[3].pos = p(1, 2);
    units[0] = carrying(units[0].clone(), &[weapon(2, 2, 3)]);
    let mut s = start(setup(units));
    // Range 1, from (0,1) to (1,2): 2 tiles.
    refused_act(
        &mut s,
        2,
        p(0, 1),
        attack(4),
        CommandError::OutOfRange {
            target: UnitId(4),
            distance: 2,
        },
    );
    // A range-2-only weapon can't hit an adjacent enemy.
    refused_act(
        &mut s,
        1,
        p(1, 1),
        attack(4),
        CommandError::OutOfRange {
            target: UnitId(4),
            distance: 1,
        },
    );
}

#[test]
fn error_cannot_seize() {
    let seize = |by_lord| Objective::Seize {
        pos: p(1, 1),
        by_lord,
        turn_limit: None,
    };
    // Not a seize map.
    let mut s = start(setup(cast()));
    let no = CommandError::CannotSeize;
    refused_act(&mut s, 1, p(1, 1), UnitAction::Seize, no.clone());
    // Not the seize tile.
    let mut s = with_objective(cast(), seize(false));
    refused_act(&mut s, 1, p(1, 0), UnitAction::Seize, no.clone());
    // Not a lord.
    let mut s = with_objective(cast(), seize(true));
    refused_act(&mut s, 2, p(1, 1), UnitAction::Seize, no.clone());
    // Not a player unit.
    let mut units = cast();
    units[2].pos = p(3, 1);
    let mut s = with_objective(units, seize(false));
    end(&mut s);
    refused_act(&mut s, 3, p(1, 1), UnitAction::Seize, no);
}

#[test]
fn error_messages() {
    let cases = [
        (CommandError::BattleOver, "the battle is over"),
        (CommandError::UnknownUnit(UnitId(4)), "no unit with id 4"),
        (CommandError::UnitFallen(UnitId(4)), "unit 4 has fallen"),
        (
            CommandError::NotItsPhase {
                unit: UnitId(4),
                phase: Phase::Enemy,
            },
            "unit 4 doesn't act in the Enemy phase",
        ),
        (
            CommandError::AlreadyActed(UnitId(4)),
            "unit 4 has already acted",
        ),
        (
            CommandError::UnknownClass(ClassId("x".into())),
            "unknown class \"x\"",
        ),
        (CommandError::OffMap(UnitId(4)), "unit 4 is outside the map"),
        (
            CommandError::UnknownTerrain(p(1, 2)),
            "unknown terrain at (1, 2)",
        ),
        (CommandError::CannotStop(p(1, 2)), "can't stop at (1, 2)"),
        (
            CommandError::NotHostile(UnitId(4)),
            "unit 4 is not an enemy",
        ),
        (
            CommandError::EmptySlot {
                unit: UnitId(4),
                slot: 2,
            },
            "unit 4 has no weapon in slot 2",
        ),
        (
            CommandError::CannotWield {
                unit: UnitId(4),
                item: item("axe"),
            },
            "unit 4 can't wield \"axe\"",
        ),
        (
            CommandError::NoItem {
                unit: UnitId(4),
                index: 1,
            },
            "unit 4 has no item 1",
        ),
        (CommandError::UnknownItem(item("x")), "unknown item \"x\""),
        (
            CommandError::NotConsumable(item("x")),
            "\"x\" can't be used",
        ),
        (
            CommandError::BadItemTarget(UnitId(4)),
            "unit 4 can't be given an item from here",
        ),
        (
            CommandError::OutOfRange {
                target: UnitId(4),
                distance: 3,
            },
            "unit 4 is out of range (3 tiles)",
        ),
        (CommandError::CannotSeize, "can't seize here"),
    ];
    for (err, text) in cases {
        assert_eq!(err.to_string(), text);
    }
    assert_eq!(
        CommandError::from(MoveError::UnknownUnit(UnitId(4))),
        CommandError::UnknownUnit(UnitId(4))
    );
}

// ---- Objectives and loss -------------------------------------------------------

#[test]
fn turn_limits_by_objective() {
    let limit = Some(4);
    assert_eq!(Objective::Rout { turn_limit: limit }.turn_limit(), limit);
    assert_eq!(
        Objective::DefeatUnit {
            unit: UnitId(1),
            turn_limit: limit
        }
        .turn_limit(),
        limit
    );
    assert_eq!(
        Objective::Seize {
            pos: p(0, 0),
            by_lord: true,
            turn_limit: limit
        }
        .turn_limit(),
        limit
    );
    assert_eq!(Objective::Survive { turns: 4 }.turn_limit(), None);
}

/// Lord 1 and unit 2 (might 12: kills even on the forest) near enemies 3
/// and 4 (on the forest).
fn skirmish() -> Vec<Unit> {
    vec![
        armed(lord(1, p(0, 0)), 12),
        armed(unit(2, Faction::Player, p(0, 2)), 12),
        unit(3, Faction::Enemy, p(2, 0)),
        unit(4, Faction::Enemy, p(2, 2)),
    ]
}

#[test]
fn rout_is_won_when_the_last_enemy_falls() {
    let mut s = with_objective(skirmish(), Objective::Rout { turn_limit: None });
    act(&mut s, 1, p(1, 0), attack(3));
    assert_eq!(s.outcome(), None);
    let events = act(&mut s, 2, p(1, 2), attack(4));
    assert_eq!(events.last(), Some(&ended(Outcome::Victory)));
    assert_eq!(s.outcome(), Some(Outcome::Victory));
}

#[test]
fn a_rout_with_no_enemies_and_no_player_units_are_decided_at_once() {
    let (s, events) = BattleState::new(setup(vec![lord(1, p(0, 0))]));
    assert_eq!(events, [ended(Outcome::Victory)]);
    assert_eq!(s.outcome(), Some(Outcome::Victory));
    let (s, events) = BattleState::new(setup(vec![unit(3, Faction::Enemy, p(0, 0))]));
    assert_eq!(events, [ended(Outcome::Defeat)]);
    assert_eq!(s.outcome(), Some(Outcome::Defeat));
}

#[test]
fn a_fallen_enemy_lord_or_ally_doesnt_decide_anything() {
    let mut units = skirmish();
    units[2].is_lord = true;
    units.push(unit(5, Faction::Ally, p(3, 1)));
    let mut s = start(setup(units));
    act(&mut s, 1, p(1, 0), attack(3));
    assert_eq!(s.outcome(), None);
    end(&mut s);
    // Enemy 4 (might 3) hits the ally three times over three turns.
    for _ in 0..3 {
        act(&mut s, 4, p(3, 2), attack(5));
        walk(&mut s, 3);
    }
    act(&mut s, 4, p(3, 2), attack(5));
    assert_eq!(s.unit(UnitId(5)), None);
    assert_eq!(s.outcome(), None);
}

#[test]
fn defeat_unit_needs_that_unit() {
    let objective = Objective::DefeatUnit {
        unit: UnitId(4),
        turn_limit: None,
    };
    let mut units = skirmish();
    units.push(unit(5, Faction::Enemy, p(7, 4)));
    let mut s = with_objective(units, objective);
    act(&mut s, 1, p(1, 0), attack(3));
    assert_eq!(s.outcome(), None);
    let events = act(&mut s, 2, p(1, 2), attack(4));
    assert_eq!(events.last(), Some(&ended(Outcome::Victory)));
}

#[test]
fn seize_wins_on_the_tile() {
    let seize = |by_lord| Objective::Seize {
        pos: p(2, 1),
        by_lord,
        turn_limit: None,
    };
    let mut s = with_objective(cast(), seize(true));
    act(&mut s, 2, p(1, 1), UnitAction::Wait);
    assert_eq!(s.outcome(), None);
    assert_eq!(
        act(&mut s, 1, p(2, 1), UnitAction::Seize),
        [
            Event::UnitMoved {
                unit: UnitId(1),
                path: vec![p(0, 0), p(1, 0), p(2, 0), p(2, 1)],
            },
            Event::Seized {
                unit: UnitId(1),
                pos: p(2, 1),
            },
            Event::UnitActed { unit: UnitId(1) },
            ended(Outcome::Victory),
        ]
    );
    // Any player unit may seize when the map doesn't need the lord.
    let mut s = with_objective(cast(), seize(false));
    let events = act(&mut s, 2, p(2, 1), UnitAction::Seize);
    assert_eq!(events.last(), Some(&ended(Outcome::Victory)));
}

#[test]
fn survive_wins_when_turn_n_ends() {
    for others in [false, true] {
        let mut units = cast();
        if others {
            units.push(unit(5, Faction::Ally, p(0, 4)));
        }
        let per_turn = if others { 3 } else { 2 };
        let mut s = with_objective(units, Objective::Survive { turns: 3 });
        // Through the end of turn 2 (N - 1): still going.
        walk(&mut s, 2 * per_turn);
        assert_eq!((s.turn(), s.phase(), s.outcome()), (3, Phase::Player, None));
        walk(&mut s, per_turn - 1);
        // Turn 3's last phase ends.
        assert_eq!(end(&mut s), [ended(Outcome::Victory)]);
        assert_eq!((s.turn(), s.outcome()), (3, Some(Outcome::Victory)));
    }
}

#[test]
fn a_turn_limit_loses_when_turn_n_ends() {
    let limited = Objective::Rout {
        turn_limit: Some(2),
    };
    let mut s = with_objective(cast(), limited);
    walk(&mut s, 2);
    assert_eq!((s.turn(), s.outcome()), (2, None));
    walk(&mut s, 1);
    assert_eq!(end(&mut s), [ended(Outcome::Defeat)]);
    assert_eq!((s.turn(), s.outcome()), (2, Some(Outcome::Defeat)));
}

#[test]
fn meeting_the_objective_on_the_last_turn_wins() {
    let limited = Objective::Rout {
        turn_limit: Some(2),
    };
    let mut s = with_objective(skirmish(), limited);
    walk(&mut s, 2);
    assert_eq!(s.turn(), 2);
    act(&mut s, 1, p(1, 0), attack(3));
    act(&mut s, 2, p(1, 2), attack(4));
    assert_eq!(s.outcome(), Some(Outcome::Victory));
}

#[test]
fn losing_the_lord_is_defeat() {
    let mut units = cast();
    units[2] = armed(units[2].clone(), 10);
    units[2].pos = p(2, 0);
    let mut s = start(setup(units));
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), attack(1));
    assert_eq!(
        &events[2..],
        [
            // 10 damage into 10 HP: 2 + 10 / 5.
            Event::WeaponExpGained {
                unit: UnitId(3),
                kind: WeaponKind::Sword,
                amount: 4,
            },
            Event::UnitFell { unit: UnitId(1) },
            Event::UnitActed { unit: UnitId(3) },
            ended(Outcome::Defeat),
        ]
    );
    // Unit 2 is still standing.
    assert!(s.unit(UnitId(2)).is_some());
}

#[test]
fn losing_every_player_unit_is_defeat() {
    let units = vec![
        unit(2, Faction::Player, p(0, 0)),
        armed(unit(3, Faction::Enemy, p(2, 0)), 10),
    ];
    let mut s = start(setup(units));
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), attack(2));
    assert_eq!(events.last(), Some(&ended(Outcome::Defeat)));
}

// ---- Reinforcements ------------------------------------------------------------

fn reinforcement(turn: Turn, unit: Unit) -> Reinforcement {
    Reinforcement { turn, unit }
}

#[test]
fn reinforcements_arrive_done_at_their_phase_and_act_next_turn() {
    let mut s = start(BattleSetup {
        reinforcements: vec![reinforcement(2, unit(9, Faction::Enemy, p(5, 4)))],
        ..setup(cast())
    });
    walk(&mut s, 2);
    // Player phase 2: not yet.
    assert_eq!(s.reinforcements().len(), 1);
    assert_eq!(s.unit(UnitId(9)), None);
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(9)]
            },
            started(2, Phase::Enemy),
        ]
    );
    assert!(s.reinforcements().is_empty());
    assert!(s.unit(UnitId(9)).unwrap().acted);
    refused_act(
        &mut s,
        9,
        p(5, 4),
        UnitAction::Wait,
        CommandError::AlreadyActed(UnitId(9)),
    );
    // Other enemies act as usual.
    act(&mut s, 3, p(7, 0), UnitAction::Wait);
    walk(&mut s, 2);
    assert_eq!((s.turn(), s.phase()), (3, Phase::Enemy));
    act(&mut s, 9, p(4, 4), UnitAction::Wait);
}

#[test]
fn a_reinforcement_waits_while_its_tile_is_occupied() {
    let wave = vec![
        reinforcement(2, unit(9, Faction::Enemy, p(3, 4))),
        reinforcement(2, unit(10, Faction::Enemy, p(4, 4))),
        // Same tile as 10: waits for it to move.
        reinforcement(2, unit(11, Faction::Enemy, p(4, 4))),
    ];
    let mut units = cast();
    units[1].pos = p(3, 3);
    let mut s = start(BattleSetup {
        reinforcements: wave,
        ..setup(units)
    });
    // Unit 2 blocks (3,4).
    act(&mut s, 2, p(3, 4), UnitAction::Wait);
    walk(&mut s, 2);
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(10)]
            },
            started(2, Phase::Enemy),
        ]
    );
    let waiting: Vec<UnitId> = s.reinforcements().iter().map(|r| r.unit.id).collect();
    assert_eq!(waiting, [UnitId(9), UnitId(11)]);
    walk(&mut s, 1);
    // Turn 3: unit 2 steps off, so 9 arrives; 10 then moves off 11's tile.
    act(&mut s, 2, p(2, 4), UnitAction::Wait);
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(9)]
            },
            started(3, Phase::Enemy),
        ]
    );
    act(&mut s, 10, p(5, 4), UnitAction::Wait);
    walk(&mut s, 1);
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(11)]
            },
            started(4, Phase::Enemy),
        ]
    );
}

#[test]
fn arrivals_start_a_phase_that_would_be_skipped() {
    let mut s = start(BattleSetup {
        reinforcements: vec![
            reinforcement(1, unit(9, Faction::Ally, p(4, 4))),
            reinforcement(2, unit(10, Faction::Neutral, p(5, 4))),
        ],
        ..setup(cast())
    });
    end(&mut s);
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(9)]
            },
            started(1, Phase::Other),
        ]
    );
    walk(&mut s, 2);
    assert_eq!(
        end(&mut s),
        [
            Event::UnitsArrived {
                units: vec![UnitId(10)]
            },
            started(2, Phase::Other),
        ]
    );
    // Unit 9 has been here since turn 1: ready.
    act(&mut s, 9, p(4, 3), UnitAction::Wait);
}

#[test]
fn player_reinforcements_on_turn_one_arrive_at_the_start() {
    let (mut s, events) = BattleState::new(BattleSetup {
        reinforcements: vec![reinforcement(1, unit(9, Faction::Player, p(4, 4)))],
        ..setup(cast())
    });
    assert_eq!(
        events,
        [
            Event::UnitsArrived {
                units: vec![UnitId(9)]
            },
            started(1, Phase::Player),
        ]
    );
    assert!(s.unit(UnitId(9)).unwrap().acted);
    walk(&mut s, 2);
    act(&mut s, 9, p(4, 3), UnitAction::Wait);
}

// ---- Equipping -------------------------------------------------------------------

fn equip(unit: u32, slot: usize) -> Command {
    Command::Equip {
        unit: UnitId(unit),
        slot,
    }
}

#[test]
fn equip_is_free_and_doesnt_end_the_action() {
    let mut units = cast();
    units[1] = carrying(units[1].clone(), &[weapon(1, 1, 3), weapon(1, 2, 5)]);
    let mut s = start(setup(units));
    assert_eq!(
        s.apply(&equip(2, 1)),
        Ok(vec![Event::Equipped {
            unit: UnitId(2),
            slot: 1,
        }])
    );
    let u = s.unit(UnitId(2)).unwrap();
    assert_eq!((u.loadout.equipped, u.acted), (Some(1), false));
    // Re-equipping the same slot is allowed; the unit can still act.
    assert!(s.apply(&equip(2, 1)).is_ok());
    act(&mut s, 2, p(0, 2), UnitAction::Wait);
}

#[test]
fn equip_errors() {
    let mut units = cast();
    units[1] = carrying(
        units[1].clone(),
        &[weapon(1, 1, 3), item("master_sword"), item("ghost")],
    );
    let mut s = start(setup(units));
    let empty = |slot| CommandError::EmptySlot {
        unit: UnitId(2),
        slot,
    };
    refused(
        &mut s,
        &equip(1, 1),
        CommandError::EmptySlot {
            unit: UnitId(1),
            slot: 1,
        },
    );
    refused(&mut s, &equip(2, 3), empty(3));
    refused(&mut s, &equip(2, 99), empty(99));
    // Rank S needed; the unit has E.
    refused(
        &mut s,
        &equip(2, 1),
        CommandError::CannotWield {
            unit: UnitId(2),
            item: item("master_sword"),
        },
    );
    refused(
        &mut s,
        &equip(2, 2),
        CommandError::UnknownItem(item("ghost")),
    );
    refused(&mut s, &equip(99, 0), CommandError::UnknownUnit(UnitId(99)));
    refused(
        &mut s,
        &equip(3, 0),
        CommandError::NotItsPhase {
            unit: UnitId(3),
            phase: Phase::Player,
        },
    );
    act(&mut s, 1, p(0, 0), UnitAction::Wait);
    refused(&mut s, &equip(1, 0), CommandError::AlreadyActed(UnitId(1)));
    // A fallen unit, and a battle that's over.
    let mut units = skirmish();
    units[2].pos = p(1, 1);
    let mut s = start(setup(units));
    act(&mut s, 1, p(1, 0), attack(3));
    refused(&mut s, &equip(3, 0), CommandError::UnitFallen(UnitId(3)));
    act(&mut s, 2, p(1, 2), attack(4));
    assert_eq!(s.outcome(), Some(Outcome::Victory));
    refused(&mut s, &equip(2, 0), CommandError::BattleOver);
}

// ---- Attacking with a loadout ------------------------------------------------------

fn attack_with(target: u32, slot: usize) -> UnitAction {
    UnitAction::Attack {
        target: UnitId(target),
        slot,
    }
}

#[test]
fn attacking_with_another_weapon_equips_it() {
    let mut units = cast();
    units[1] = carrying(units[1].clone(), &[weapon(1, 1, 3), weapon(1, 1, 6)]);
    units[3].pos = p(1, 2);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack_with(4, 1));
    let [
        Event::Equipped {
            unit: UnitId(2),
            slot: 1,
        },
        Event::CombatResolved { forecast, .. },
        ..,
    ] = events.as_slice()
    else {
        panic!("{events:?}");
    };
    assert_eq!(forecast.attacker.damage, 6);
    assert_eq!(s.unit(UnitId(2)).unwrap().loadout.equipped, Some(1));
    // Attacking with the equipped weapon: no Equipped event.
    end(&mut s);
    let events = act(&mut s, 4, p(1, 2), attack(2));
    assert!(matches!(events.first(), Some(Event::CombatResolved { .. })));
    // The counter used the newly equipped might-6 weapon.
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    assert_eq!(forecast.defender.map(|d| d.damage), Some(6));
}

#[test]
fn attack_weapon_errors() {
    let mut units = cast();
    units[1] = carrying(
        units[1].clone(),
        &[weapon(1, 1, 3), item("master_sword"), weapon(2, 2, 3)],
    );
    units[3].pos = p(1, 2);
    let mut s = start(setup(units));
    for slot in [3, 99] {
        refused_act(
            &mut s,
            2,
            p(0, 2),
            attack_with(4, slot),
            CommandError::EmptySlot {
                unit: UnitId(2),
                slot,
            },
        );
    }
    refused_act(
        &mut s,
        2,
        p(0, 2),
        attack_with(4, 1),
        CommandError::CannotWield {
            unit: UnitId(2),
            item: item("master_sword"),
        },
    );
    // The chosen weapon's range counts, not the equipped one's.
    refused_act(
        &mut s,
        2,
        p(0, 2),
        attack_with(4, 2),
        CommandError::OutOfRange {
            target: UnitId(4),
            distance: 1,
        },
    );
    act(&mut s, 2, p(0, 1), attack_with(4, 2));
}

#[test]
fn only_a_wieldable_equipped_weapon_counters() {
    let mut units = cast();
    units[3].pos = p(1, 2);
    // Carries a usable weapon, but has the unusable one equipped.
    units[3] = carrying(units[3].clone(), &[item("master_sword"), weapon(1, 1, 3)]);
    let mut s = start(setup(units.clone()));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    assert_eq!(forecast.defender, None);
    // Nothing equipped: no counter either.
    units[3].loadout.equipped = None;
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    assert_eq!(forecast.defender, None);
}

#[test]
fn gear_adjusts_combat_stats() {
    let mut units = cast();
    units[3].pos = p(1, 2);
    units[3].loadout.armour = Some(item("vest"));
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    // Might 3 − Def 1 from the vest.
    assert_eq!(forecast.attacker.damage, 2);
    assert_eq!(forecast.defender.map(|d| d.damage), Some(3));
}

#[test]
fn a_broken_weapon_still_attacks_with_the_penalties() {
    let mut units = cast();
    units[1] = carrying(units[1].clone(), &[weapon(1, 1, 5)]);
    if let Some(w) = units[1].loadout.weapons[0].as_mut() {
        w.durability_left = 0;
    }
    units[3].pos = p(1, 2);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let Some(Event::CombatResolved { forecast, .. }) = events.first() else {
        panic!("{events:?}");
    };
    assert!(forecast.attacker.broken);
    assert_eq!(forecast.attacker.damage, 2);
    assert_eq!(forecast.attacker.hit, 80);
    // Normal attacks cost no durability.
    let u = s.unit(UnitId(4)).unwrap();
    assert_eq!(
        u.loadout.weapons[0].as_ref().map(|w| w.durability_left),
        Some(20)
    );
}

// ---- Weapon EXP --------------------------------------------------------------------

#[test]
fn weapon_exp_after_combat_and_rank_up() {
    let mut units = cast();
    units[1].weapon_exp.insert(WeaponKind::Sword, 29);
    units[3].pos = p(1, 2);
    // The enemy misses every time: base 1, nothing dealt.
    units[3] = carrying(units[3].clone(), &[weapon_id(1, 1, 3, 0, 0)]);
    let mut s = start(setup(units));
    let events = act(&mut s, 2, p(0, 2), attack(4));
    assert_eq!(
        events[1..],
        [
            Event::WeaponExpGained {
                unit: UnitId(2),
                kind: WeaponKind::Sword,
                amount: 2,
            },
            Event::WeaponRankUp {
                unit: UnitId(2),
                kind: WeaponKind::Sword,
                rank: WeaponRank::D,
            },
            Event::WeaponExpGained {
                unit: UnitId(4),
                kind: WeaponKind::Sword,
                amount: 1,
            },
            Event::UnitActed { unit: UnitId(2) },
        ]
    );
    let u = s.unit(UnitId(2)).unwrap();
    assert_eq!(u.rank(WeaponKind::Sword), WeaponRank::D);
    assert_eq!(u.weapon_exp[&WeaponKind::Sword], 31);
    assert_eq!(s.unit(UnitId(4)).unwrap().weapon_exp[&WeaponKind::Sword], 1);
}

#[test]
fn a_unit_that_doesnt_strike_gains_no_weapon_exp() {
    let mut units = cast();
    units[1] = carrying(units[1].clone(), &[weapon(1, 2, 3)]);
    units[3].pos = p(2, 2);
    let mut s = start(setup(units));
    // Range 2: no counter, so only the attacker gains.
    let events = act(&mut s, 2, p(0, 2), attack(4));
    let gains: Vec<UnitId> = events
        .iter()
        .filter_map(|e| match e {
            Event::WeaponExpGained { unit, .. } => Some(*unit),
            _ => None,
        })
        .collect();
    assert_eq!(gains, [UnitId(2)]);
    assert!(s.unit(UnitId(4)).unwrap().weapon_exp.is_empty());
}

// ---- Items ---------------------------------------------------------------------------

fn use_item(pack_index: usize, target: u32) -> UnitAction {
    UnitAction::UseItem {
        pack_index,
        target: UnitId(target),
    }
}

#[test]
fn a_potion_heals_and_ends_the_action() {
    let mut units = cast();
    units[1].hp = 3;
    let mut s = start(setup(units));
    assert_eq!(
        act(&mut s, 2, p(1, 2), use_item(0, 2)),
        [
            Event::UnitMoved {
                unit: UnitId(2),
                path: vec![p(0, 2), p(1, 2)],
            },
            Event::ItemUsed {
                unit: UnitId(2),
                item: item("potion"),
                target: UnitId(2),
            },
            Event::Healed {
                target: UnitId(2),
                amount: 4,
            },
            Event::UnitActed { unit: UnitId(2) },
        ]
    );
    assert_eq!(s.unit(UnitId(2)).unwrap().hp, 7);
    assert_eq!(s.pack().items, [item("elixir")]);
    assert_eq!(s.pack().cap, 3);
    assert!(s.unit(UnitId(2)).unwrap().acted);
}

#[test]
fn healing_never_exceeds_max_hp() {
    let mut units = cast();
    units[0].hp = 8;
    units[1].hp = 1;
    let mut s = start(setup(units));
    let events = act(&mut s, 1, p(0, 0), use_item(0, 1));
    assert_eq!(
        events[1],
        Event::Healed {
            target: UnitId(1),
            amount: 2,
        }
    );
    assert_eq!(s.unit(UnitId(1)).unwrap().hp, 10);
    // The elixir heals all.
    let events = act(&mut s, 2, p(0, 2), use_item(0, 2));
    assert_eq!(
        events[1],
        Event::Healed {
            target: UnitId(2),
            amount: 9,
        }
    );
    assert!(s.pack().items.is_empty());
}

#[test]
fn an_item_can_be_used_on_an_adjacent_ally() {
    let mut units = cast();
    units[1].hp = 2;
    units.push(unit(5, Faction::Ally, p(1, 1)));
    units[4].hp = 5;
    let mut s = start(setup(units));
    // The lord steps to (0,1): next to unit 2 at (0,2) and ally 5 at (1,1).
    let events = act(&mut s, 1, p(0, 1), use_item(0, 2));
    assert_eq!(
        events[1..3],
        [
            Event::ItemUsed {
                unit: UnitId(1),
                item: item("potion"),
                target: UnitId(2),
            },
            Event::Healed {
                target: UnitId(2),
                amount: 4,
            },
        ]
    );
    assert_eq!(s.unit(UnitId(2)).unwrap().hp, 6);
    assert_eq!(s.unit(UnitId(1)).unwrap().hp, 10);
    act(&mut s, 2, p(1, 2), use_item(0, 5));
    assert_eq!(s.unit(UnitId(5)).unwrap().hp, 10);
}

#[test]
fn use_item_errors() {
    let mut units = cast();
    units[3].pos = p(1, 1);
    let mut s = start(BattleSetup {
        pack: BattlePack {
            items: vec![item("potion"), item("axe"), item("ghost")],
            cap: 3,
        },
        ..setup(units)
    });
    let refuse = |s: &mut BattleState, dest, action, err| refused_act(s, 1, dest, action, err);
    refuse(
        &mut s,
        p(0, 0),
        use_item(3, 1),
        CommandError::NoItem {
            unit: UnitId(1),
            index: 3,
        },
    );
    refuse(
        &mut s,
        p(0, 0),
        use_item(1, 1),
        CommandError::NotConsumable(item("axe")),
    );
    refuse(
        &mut s,
        p(0, 0),
        use_item(2, 1),
        CommandError::UnknownItem(item("ghost")),
    );
    // An adjacent enemy; an ally too far away; unknown units.
    refuse(
        &mut s,
        p(1, 0),
        use_item(0, 4),
        CommandError::BadItemTarget(UnitId(4)),
    );
    refuse(
        &mut s,
        p(0, 0),
        use_item(0, 2),
        CommandError::BadItemTarget(UnitId(2)),
    );
    refuse(
        &mut s,
        p(0, 0),
        use_item(0, 99),
        CommandError::UnknownUnit(UnitId(99)),
    );
    // Fallen targets.
    let mut units = skirmish();
    units[2].pos = p(1, 1);
    let mut s = start(setup(units));
    act(&mut s, 1, p(1, 0), attack(3));
    refused_act(
        &mut s,
        2,
        p(1, 2),
        use_item(0, 3),
        CommandError::UnitFallen(UnitId(3)),
    );
}

#[test]
fn enemies_use_their_own_consumables() {
    let mut units = cast();
    units[2].consumables = vec![item("potion")];
    units[2].hp = 1;
    let mut s = start(setup(units));
    end(&mut s);
    act(&mut s, 3, p(7, 0), use_item(0, 3));
    assert_eq!(s.unit(UnitId(3)).unwrap().hp, 5);
    assert!(s.unit(UnitId(3)).unwrap().consumables.is_empty());
    // The player's pack is untouched; enemy 4 has nothing to use.
    assert_eq!(s.pack().items.len(), 2);
    refused_act(
        &mut s,
        4,
        p(7, 2),
        use_item(0, 4),
        CommandError::NoItem {
            unit: UnitId(4),
            index: 0,
        },
    );
}

#[test]
fn the_state_exposes_items_and_pack() {
    let s = start(setup(cast()));
    assert_eq!(s.items(), &items_for(&cast()));
    assert_eq!(s.pack().items, [item("potion"), item("elixir")]);
}

// ---- Saving --------------------------------------------------------------------

#[test]
fn state_round_trips_through_ron_and_needs_its_tables_back() {
    let mut s = start(BattleSetup {
        reinforcements: vec![reinforcement(3, unit(9, Faction::Enemy, p(5, 4)))],
        ..setup(skirmish())
    });
    act(&mut s, 1, p(1, 0), attack(3));
    let text = ron::to_string(&s).unwrap();
    let mut loaded: BattleState = ron::from_str(&text).unwrap();
    // Without tables: every Act fails, harmlessly.
    refused_act(
        &mut loaded,
        2,
        p(1, 2),
        attack(4),
        CommandError::UnknownClass(ClassId("fighter".into())),
    );
    loaded.restore_tables(
        Arc::new(terrain()),
        Arc::new(classes()),
        Arc::new(items_for(&skirmish())),
    );
    assert_eq!(loaded, s);
    let cmd = Command::Act {
        unit: UnitId(2),
        dest: p(1, 2),
        action: attack(4),
    };
    assert_eq!(loaded.apply(&cmd), s.apply(&cmd));
}

#[test]
fn commands_and_events_round_trip_through_ron() {
    let cmd = Command::Act {
        unit: UnitId(2),
        dest: p(1, 2),
        action: attack(4),
    };
    let text = ron::to_string(&cmd).unwrap();
    assert_eq!(ron::from_str::<Command>(&text).unwrap(), cmd);
    let mut s = start(setup(skirmish()));
    let events = s.apply(&cmd).unwrap();
    let text = ron::to_string(&events).unwrap();
    assert_eq!(ron::from_str::<Vec<Event>>(&text).unwrap(), events);
}

// ---- Property: random legal play -------------------------------------------------

/// Every legal command in `s`: `EndPhase`, and for each ready unit of the
/// phase, equipping each weapon it can wield, and each stoppable tile with
/// `Wait`, each attack in range (per weapon), each item use and a seize.
/// Consumables in test packs are all known; weapons are all known.
fn legal_commands(s: &BattleState) -> Vec<Command> {
    let mut out = vec![Command::EndPhase];
    let ready = s
        .units()
        .iter()
        .filter(|u| Phase::of(u.faction) == s.phase() && !u.acted);
    for u in ready {
        let class = s.classes().get(&u.class).unwrap();
        for slot in 0..WEAPON_SLOTS {
            if u.usable_weapon(slot, class, s.items()).is_some() {
                out.push(Command::Equip { unit: u.id, slot });
            }
        }
        let reach = reachable(s.map(), s.terrain(), s.classes(), s.units(), u.id).unwrap();
        for dest in reach.stoppable().iter() {
            let mut add = |action| {
                out.push(Command::Act {
                    unit: u.id,
                    dest,
                    action,
                });
            };
            add(UnitAction::Wait);
            let class = s.classes().get(&u.class).unwrap();
            for slot in 0..WEAPON_SLOTS {
                let Some((_, w)) = u.usable_weapon(slot, class, s.items()) else {
                    continue;
                };
                for t in s.units() {
                    let d = Pos::manhattan(dest, t.pos);
                    if u.faction.is_hostile_to(t.faction)
                        && (w.min_range..=w.max_range).contains(&d)
                    {
                        add(UnitAction::Attack { target: t.id, slot });
                    }
                }
            }
            let own = if u.faction == Faction::Player {
                s.pack().items.len()
            } else {
                u.consumables.len()
            };
            for pack_index in 0..own {
                for t in s.units() {
                    let near = t.id != u.id && Pos::manhattan(dest, t.pos) == 1;
                    if t.id == u.id || (near && !u.faction.is_hostile_to(t.faction)) {
                        add(UnitAction::UseItem {
                            pack_index,
                            target: t.id,
                        });
                    }
                }
            }
            if let Objective::Seize { pos, by_lord, .. } = s.objective()
                && pos == dest
                && u.faction == Faction::Player
                && (u.is_lord || !by_lord)
            {
                add(UnitAction::Seize);
            }
        }
    }
    out
}

const PROP_MAP: [&str; 6] = [
    "........", //
    "..#..f..", //
    "..#.....", //
    "....ff..", //
    ".#......", //
    "........", //
];

fn free_tiles() -> Vec<Pos> {
    let m = map(&PROP_MAP);
    m.tiles
        .positions()
        .filter(|&pos| m.tiles.get(pos) != Some(&TerrainId(2)))
        .collect()
}

prop_compose! {
    fn arb_unit()(
        hp in 1..=20,
        str in 0..=8,
        def in 0..=4,
        spd in 0..=8,
        dex in 0..=5,
        mov in 1..=5,
        range in prop::sample::select(vec![(1, 1), (1, 2), (2, 2)]),
        might in 0..=8,
        hit in 50..=100,
        crit in 0..=30,
        armed in prop::bool::weighted(0.9),
        second in prop::option::of(prop::sample::select(vec!["flier_bow", "master_sword", "axe"])),
        armour in prop::option::of(prop::sample::select(vec!["vest", "mail"])),
        consumables in prop::collection::vec(prop::sample::select(vec!["potion", "elixir"]), 0..=2),
        wounds in 0..=10_i32,
    ) -> Unit {
        let mut u = unit(0, Faction::Player, p(0, 0));
        u.stats = Stats::from_growable([hp, str, 0, dex, spd, def, 0], mov);
        u.hp = (hp - wounds).max(1);
        let mut weapons = Vec::new();
        if armed {
            weapons.push(weapon_id(range.0, range.1, might, hit, crit));
        }
        weapons.extend(second.map(item));
        u = carrying(u, &weapons);
        u.loadout.armour = armour.map(item);
        u.consumables = consumables.into_iter().map(item).collect();
        u
    }
}

prop_compose! {
    fn arb_setup()(
        players in 1usize..=3,
        enemies in 1usize..=3,
        others in 0usize..=2,
        extra in 0usize..=3,
        stats in prop::collection::vec(arb_unit(), 11),
        tiles in Just(free_tiles()).prop_shuffle(),
        extra_tiles in prop::collection::vec(prop::sample::select(free_tiles()), 3),
        extra_turns in prop::collection::vec(1u32..=4, 3),
        extra_factions in prop::collection::vec(0usize..4, 3),
        objective_kind in 0usize..4,
        limit in prop::option::of(1u32..=5),
        by_lord in any::<bool>(),
        seed in any::<u64>(),
        pack in prop::collection::vec(prop::sample::select(vec!["potion", "elixir"]), 0..=4),
    ) -> BattleSetup {
        let factions = [Faction::Player, Faction::Enemy, Faction::Ally, Faction::Neutral];
        let mut units = Vec::new();
        let mut stats = stats.into_iter();
        let mut id = 0;
        let mut next = |faction: Faction, pos: Pos, stats: &mut dyn Iterator<Item = Unit>| {
            id += 1;
            Unit { id: UnitId(id), faction, pos, ..stats.next().unwrap() }
        };
        let mut tiles = tiles.into_iter();
        for (n, faction) in [
            (players, Faction::Player),
            (enemies, Faction::Enemy),
            (others, if seed % 2 == 0 { Faction::Ally } else { Faction::Neutral }),
        ] {
            for _ in 0..n {
                units.push(next(faction, tiles.next().unwrap(), &mut stats));
            }
        }
        units[0].is_lord = true;
        let reinforcements: Vec<Reinforcement> = (0..extra)
            .map(|i| reinforcement(
                extra_turns[i],
                next(factions[extra_factions[i]], extra_tiles[i], &mut stats),
            ))
            .collect();
        let objective = match objective_kind {
            0 => Objective::Rout { turn_limit: limit },
            1 => Objective::Survive { turns: limit.unwrap_or(3) },
            2 => Objective::DefeatUnit { unit: units[players].id, turn_limit: limit },
            _ => Objective::Seize { pos: tiles.next().unwrap(), by_lord, turn_limit: limit },
        };
        let items = Arc::new(items_for(
            units.iter().chain(reinforcements.iter().map(|r| &r.unit)),
        ));
        BattleSetup {
            map: map(&PROP_MAP),
            reinforcements,
            objective,
            seed,
            items,
            pack: BattlePack { items: pack.into_iter().map(item).collect(), cap: 4 },
            ..setup(units)
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn random_legal_play_keeps_the_invariants(
        setup in arb_setup(),
        choices in prop::collection::vec(any::<u16>(), 0..120),
    ) {
        let (mut s, _) = BattleState::new(setup);
        let mut decided = s.outcome();
        let mut pack = s.pack().items.len();
        for choice in choices {
            if decided.is_some() {
                prop_assert_eq!(s.apply(&Command::EndPhase), Err(CommandError::BattleOver));
                break;
            }
            let legal = legal_commands(&s);
            let cmd = &legal[usize::from(choice) % legal.len()];
            let turn = s.turn();
            let events = s.apply(cmd);
            prop_assert!(events.is_ok(), "{:?} refused: {:?}", cmd, events);
            prop_assert!(s.turn() >= turn);
            // No two units on one tile; HP within 1..=max on the map, 0 when
            // fallen.
            let mut tiles: Vec<Pos> = s.units().iter().map(|u| u.pos).collect();
            tiles.sort();
            tiles.dedup();
            prop_assert_eq!(tiles.len(), s.units().len());
            for u in s.units() {
                prop_assert!(u.hp > 0 && u.hp <= u.stats.hp, "{:?}", u);
            }
            for u in s.fallen() {
                prop_assert_eq!(u.hp, 0);
            }
            // No item gains exist yet: the pack only shrinks. Loadouts keep
            // within their weapon slots.
            prop_assert!(s.pack().items.len() <= pack);
            pack = s.pack().items.len();
            for u in s.units() {
                let slots = usize::from(s.classes().get(&u.class).unwrap().weapon_slots);
                prop_assert!(u.loadout.weapon_count() <= slots.min(WEAPON_SLOTS));
            }
            // The outcome, once set, never changes.
            if let Some(o) = decided {
                prop_assert_eq!(s.outcome(), Some(o));
            }
            decided = s.outcome();
        }
    }
}
