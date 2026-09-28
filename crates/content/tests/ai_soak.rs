//! Ticket 0501's soak test: the AI plays both sides (a temporary player-side
//! AI using the same code) of 100 seeded battles on `test_small.map`, to the
//! end or turn 50. Every command it issues must be accepted, and nothing
//! may panic.

use std::sync::Arc;

use trpg_content::{Content, character_unit};
use trpg_core::{
    AiBehavior, BattlePack, BattleSetup, BattleState, ClassId, Command, Faction, ItemId,
    LoadoutDef, Objective, Pos, SimRng, Stock, Unit, UnitId, next_command,
};

/// Generic units the seeds pick from: class, level and weapons.
const GENERICS: [(&str, u32, &[&str]); 7] = [
    ("swordsman", 2, &["iron_sword"]),
    ("archer", 2, &["iron_bow", "iron_sword"]),
    ("mage", 2, &[]),
    ("cleric", 2, &[]),
    ("raider", 2, &["iron_axe"]),
    ("brigand", 1, &["iron_axe", "steel_axe"]),
    ("guard", 1, &["iron_spear"]),
];

const BEHAVIOURS: [AiBehavior; 4] = [
    AiBehavior::Aggressive,
    AiBehavior::Guard,
    AiBehavior::Stationary,
    AiBehavior::Healer,
];

/// `0..n`, from the seed's RNG.
fn pick(rng: &mut SimRng, n: usize) -> usize {
    usize::try_from(rng.next_u32()).unwrap_or(0) % n
}

/// A free tile a foot unit can stand on.
fn free_tile(
    rng: &mut SimRng,
    content: &Content,
    map: &trpg_core::BattleMap,
    units: &[Unit],
) -> Pos {
    let Some(foot) = content.terrain.rules.movement_type("foot") else {
        panic!("no foot movement type");
    };
    let tiles: Vec<Pos> = map
        .tiles
        .positions()
        .filter(|&p| {
            let walkable = map
                .tiles
                .get(p)
                .is_some_and(|&t| content.terrain.rules.move_cost(t, foot).is_some());
            walkable && units.iter().all(|u| u.pos != p)
        })
        .collect();
    tiles[pick(rng, tiles.len())]
}

/// Seed `seed`'s battle: the three test characters and 0–2 generics on the
/// player's side, 2–6 enemies, maybe an ally and a villager, at random free
/// tiles with random behaviours, some carrying potions.
fn battle(content: &Content, seed: u64) -> BattleState {
    let mut rng = SimRng::new(seed);
    let map = content.maps["test_small"].map.clone();
    let (classes, items) = (&content.classes, &content.items);
    let mut units: Vec<Unit> = Vec::new();
    let mut next_id = 0;
    for name in ["test_lord", "test_knight", "test_archer"] {
        next_id += 1;
        let def = &content.characters.characters[&trpg_core::CharacterId(name.into())];
        let pos = free_tile(&mut rng, content, &map, &units);
        units.push(
            character_unit(def, UnitId(next_id), classes, items, Faction::Player, pos)
                .unwrap_or_else(|e| panic!("{name}: {e}")),
        );
    }
    let sides = [
        (Faction::Player, pick(&mut rng, 3)),
        (Faction::Enemy, 2 + pick(&mut rng, 5)),
        (Faction::Ally, pick(&mut rng, 2)),
        (Faction::Neutral, pick(&mut rng, 2)),
    ];
    for (faction, count) in sides {
        for _ in 0..count {
            next_id += 1;
            let (class, level, weapons) = GENERICS[pick(&mut rng, GENERICS.len())];
            let pos = free_tile(&mut rng, content, &map, &units);
            let loadout = LoadoutDef {
                weapons: weapons.iter().map(|w| ItemId::new(w)).collect(),
                ..LoadoutDef::default()
            };
            let mut unit = Unit::generic(
                UnitId(next_id),
                &ClassId(class.into()),
                classes,
                level,
                faction,
                pos,
            )
            .and_then(|u| Ok(u.with_loadout(&loadout, classes, items)?))
            .unwrap_or_else(|e| panic!("{class}: {e}"));
            if pick(&mut rng, 2) == 0 {
                unit.consumables = vec![ItemId::new("potion")];
            }
            units.push(unit);
        }
    }
    for u in &mut units {
        u.ai = BEHAVIOURS[pick(&mut rng, BEHAVIOURS.len())];
    }
    let (state, _) = BattleState::new(BattleSetup {
        map,
        terrain: Arc::new(content.terrain.rules.clone()),
        classes: Arc::new(classes.clone()),
        items: Arc::new(items.clone()),
        spells: Arc::new(content.spells.clone()),
        skills: Arc::new(content.skills.clone()),
        arts: Arc::new(content.arts.clone()),
        pack: BattlePack {
            items: vec![ItemId::new("potion"); 3],
            cap: items.rules.default_pack_cap,
        },
        gold: 0,
        stock: Stock::default(),
        units,
        reinforcements: vec![],
        objective: Objective::Rout { turn_limit: None },
        rewind_charges: 0,
        seed,
    });
    state
}

#[test]
fn ai_plays_100_seeded_battles_to_the_end() {
    let content = trpg_content::load_embedded().unwrap();
    let mut finished = 0;
    for seed in 0..100 {
        let mut state = battle(&content, seed);
        let mut commands = 0;
        while state.turn() <= 50 {
            if state.outcome().is_some() {
                finished += 1;
                break;
            }
            // As the battle screen does: the AI's command, else end the phase.
            let command = next_command(&state, &content.ai).unwrap_or(Command::EndPhase);
            if let Err(e) = state.apply(&command) {
                panic!("seed {seed}: {command:?} refused: {e}");
            }
            commands += 1;
            // Each phase is at most one command per unit, a move after
            // each, and its end.
            assert!(commands < 50 * 3 * 40, "seed {seed}: no end in sight");
        }
    }
    // Most battles are decided well before turn 50 (guards and stationary
    // units can stall a few).
    assert!(finished >= 50, "only {finished} of 100 battles ended");
}
