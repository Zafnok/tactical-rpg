//! Battle file tests, against the embedded content (the test map, the
//! placeholder characters and scenes).

use trpg_core::{AiBehavior, Phase, TriggerWhen};

use super::*;
use crate::Content;

fn content() -> Content {
    crate::load_embedded().unwrap_or_else(|e| panic!("{e}"))
}

fn refs(c: &Content) -> BattleRefs<'_> {
    BattleRefs {
        maps: &c.maps,
        terrain: &c.terrain.rules,
        classes: &c.classes,
        items: &c.items,
        characters: &c.characters,
        dialogue: &c.dialogue,
    }
}

/// A valid battle on `test_small`: the lead at (3, 5) and the knight at
/// (3, 6), a level-3 boss brigand called Garth with a steel axe on (8, 3),
/// the rogue (Guard AI) arriving on turn 2.
const OK: &str = r#"(
    id: "t",
    map: "test_small",
    player_slots: [
        (character: "lead", pos: (3, 5)),
        (character: "test_knight", pos: (3, 6)),
    ],
    enemies: [
        (template: "test_brigand", level: Some(3), pos: (8, 3), boss: true, name: Some("Garth"),
         loadout: Some((weapons: ["steel_axe"]))),
    ],
    reinforcements: [
        (turn: 2, unit: (character: "test_rogue", pos: (12, 3), ai: Guard)),
    ],
    pack_cap: 3,
    default_pack: ["potion", "elixir"],
    clear_gold: 700,
    objective: DefeatUnit(unit: "test_rogue", turn_limit: Some(9)),
    triggers: [(when: TurnStart(turn: 1, phase: Player), scene: "test", once: true)],
    difficulty: Hard,
    seed: 42,
)"#;

fn load(c: &Content, source: &str) -> Result<BattleDef, Vec<ContentError>> {
    from_source("b.ron", "t", source, &refs(c))
}

/// The messages of `source` with `from` replaced by `to` (which must be
/// in it).
fn errors(c: &Content, from: &str, to: &str) -> Vec<String> {
    assert!(OK.contains(from), "{from}");
    load(c, &OK.replacen(from, to, 1))
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|e| {
            assert_eq!(e.file, "b.ron");
            e.message
        })
        .collect()
}

#[test]
fn a_valid_battle_loads() {
    let c = content();
    let def = load(&c, OK).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(def.id, "t");
    assert_eq!(def.map, c.maps["test_small"].map);
    let slots: Vec<_> = def
        .player_slots
        .iter()
        .map(|s| (s.character.0.as_str(), s.pos))
        .collect();
    assert_eq!(
        slots,
        [("lead", Pos::new(3, 5)), ("test_knight", Pos::new(3, 6))]
    );
    // Enemies are numbered after the two slots, then the reinforcements.
    let garth = &def.enemies[0];
    assert_eq!(garth.id, UnitId(3));
    assert_eq!(garth.name, "Garth");
    assert_eq!(garth.map_label, "Ga");
    assert_eq!(garth.level, 3);
    assert_eq!(garth.role, Role::Boss);
    assert_eq!(garth.faction, Faction::Enemy);
    assert_eq!(
        garth.loadout.weapon(0).map(|w| w.def.0.as_str()),
        Some("steel_axe")
    );
    assert_eq!(garth.loadout.weapon(1), None);
    let rogue = &def.reinforcements[0];
    assert_eq!(rogue.turn, 2);
    assert_eq!(rogue.unit.id, UnitId(4));
    assert_eq!(rogue.unit.ai, AiBehavior::Guard);
    assert_eq!(rogue.unit.role, Role::Regular);
    assert_eq!(rogue.unit.name, "Test Rogue");
    assert!(!def.preparations);
    assert_eq!(def.pack_cap, 3);
    assert_eq!(
        def.default_pack,
        [ItemId::new("potion"), ItemId::new("elixir")]
    );
    assert_eq!(def.clear_gold, 700);
    assert_eq!(
        def.objective,
        Objective::DefeatUnit {
            unit: UnitId(4),
            turn_limit: Some(9)
        }
    );
    assert_eq!(
        def.triggers[0].when,
        TriggerWhen::TurnStart {
            turn: 1,
            phase: Phase::Player
        }
    );
    assert_eq!(def.difficulty, Difficulty::Hard);
    assert_eq!(def.seed, 42);
}

#[test]
fn defaults_fill_what_a_battle_leaves_out() {
    let c = content();
    let source = r#"(
        id: "t",
        map: "test_small",
        player_slots: [(character: "lead", pos: (3, 5))],
        objective: Seize(pos: (5, 5)),
        difficulty: Easy,
        seed: 0,
    )"#;
    let def = load(&c, source).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(def.enemies.is_empty() && def.reinforcements.is_empty());
    assert_eq!(def.pack_cap, c.items.rules.default_pack_cap);
    assert!(def.default_pack.is_empty());
    assert_eq!(def.clear_gold, 0);
    assert!(def.triggers.is_empty());
    assert_eq!(
        def.objective,
        Objective::Seize {
            pos: Pos::new(5, 5),
            by_lord: false,
            turn_limit: None
        }
    );
    let rout = source.replace("Seize(pos: (5, 5))", "Rout()");
    let def = load(&c, &rout).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(def.objective, Objective::Rout { turn_limit: None });
    let survive = source.replace("Seize(pos: (5, 5))", "Survive(turns: 8)");
    let def = load(&c, &survive).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(def.objective, Objective::Survive { turns: 8 });
}

#[test]
fn file_level_errors() {
    let c = content();
    assert_eq!(
        errors(&c, "id: \"t\"", "id: \"x\""),
        ["id \"x\" must match the file name \"t\""]
    );
    assert_eq!(
        errors(&c, "\"test_small\"", "\"nowhere\""),
        ["no map \"nowhere\""]
    );
    assert_eq!(
        errors(&c, "pack_cap: 3,", "pack_cap: 3, preparations: true,"),
        [NO_PREPARATIONS]
    );
    assert_eq!(
        NO_PREPARATIONS,
        "Preparations screen not built yet, ticket 0408"
    );
    // A RON error is positioned.
    let e = load(&c, "(id: 1)").err().unwrap_or_default();
    assert_eq!(e.len(), 1);
    assert_eq!(e[0].line, Some(1));
}

#[test]
fn slot_errors() {
    let c = content();
    assert_eq!(
        errors(&c, "\"lead\", pos: (3, 5)", "\"nobody\", pos: (3, 5)"),
        ["player slot 1 (\"nobody\"): no character \"nobody\""]
    );
    assert_eq!(
        errors(&c, "\"lead\", pos: (3, 5)", "\"lead\", pos: (30, 5)"),
        ["player slot 1 (\"lead\"): (30, 5) is outside the 14×8 map"]
    );
    // (0, 0) is sea.
    assert_eq!(
        errors(&c, "\"lead\", pos: (3, 5)", "\"lead\", pos: (0, 0)"),
        ["player slot 1 (\"lead\"): a Exile can't stand on (0, 0)"]
    );
    assert_eq!(
        errors(&c, "pos: (3, 6)", "pos: (3, 5)"),
        ["player slot 2 (\"test_knight\"): (3, 5) is already taken by player slot 1 (\"lead\")"]
    );
    assert_eq!(
        errors(&c, "\"test_knight\"", "\"lead\""),
        [
            "player slot 2 (\"lead\"): character \"lead\" is placed twice",
            "Ellery and Ellery are both labelled \"El\" on the map; give one a map_label"
        ]
    );
}

#[test]
fn enemy_errors() {
    let c = content();
    assert_eq!(
        errors(&c, "(template: \"test_brigand\",", "(template: \"nope\","),
        ["enemy 1: no generic template \"nope\""]
    );
    assert_eq!(
        errors(
            &c,
            "(template: \"test_brigand\",",
            "(template: \"test_brigand\", character: \"test_rogue\","
        ),
        ["enemy 1: give a template or a character, not both"]
    );
    assert_eq!(
        errors(&c, "(template: \"test_brigand\",", "("),
        ["enemy 1: give a template or a character, not both"]
    );
    assert_eq!(
        errors(&c, "character: \"test_rogue\"", "character: \"nope\""),
        [
            "reinforcement 1: no character \"nope\"",
            "objective: no enemy of character \"test_rogue\" to defeat"
        ]
    );
    let cap = c.classes.level_cap;
    assert_eq!(
        errors(&c, "level: Some(3)", "level: Some(0)"),
        [format!("enemy 1: level 0 is outside 1..={cap}")]
    );
    let over = cap + 1;
    assert_eq!(
        errors(&c, "level: Some(3)", &format!("level: Some({over})")),
        [format!("enemy 1: level {over} is outside 1..={cap}")]
    );
    for ok in [1, cap] {
        let level = format!("level: Some({ok})");
        assert_eq!(errors(&c, "level: Some(3)", &level), Vec::<String>::new());
    }
    assert_eq!(
        errors(&c, "pos: (12, 3),", "pos: (12, 3), level: Some(2),"),
        ["reinforcement 1: a character's level comes from characters.ron"]
    );
    assert_eq!(
        errors(&c, "[\"steel_axe\"]", "[\"nope\"]"),
        [format!("enemy 1: {}", loadout_error(&c))]
    );
    assert_eq!(
        errors(&c, "(turn: 2,", "(turn: 0,"),
        ["reinforcement 1: turn 0; turns start at 1"]
    );
    // An enemy on a player's tile; a reinforcement may wait on one.
    assert_eq!(
        errors(&c, "pos: (8, 3)", "pos: (3, 6)"),
        ["enemy 1: (3, 6) is already taken by player slot 2 (\"test_knight\")"]
    );
    assert_eq!(
        errors(&c, "pos: (12, 3)", "pos: (3, 6)"),
        Vec::<String>::new()
    );
    // The rogue both an enemy and a reinforcement.
    assert_eq!(
        errors(
            &c,
            "(template: \"test_brigand\", level: Some(3),",
            "(character: \"test_rogue\","
        ),
        ["reinforcement 1: character \"test_rogue\" is placed twice"]
    );
}

/// The loadout error of a brigand given an unknown weapon.
fn loadout_error(c: &Content) -> String {
    let mut template = c.characters.generics["test_brigand"].clone();
    template.loadout.weapons = vec![ItemId::new("nope")];
    template
        .unit(
            UnitId(3),
            &c.classes,
            &c.items,
            Faction::Enemy,
            Pos::new(8, 3),
        )
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default()
}

#[test]
fn objective_errors() {
    let c = content();
    assert_eq!(
        errors(&c, "unit: \"test_rogue\"", "unit: \"lead\""),
        ["objective: no enemy of character \"lead\" to defeat"]
    );
    assert_eq!(
        errors(
            &c,
            "DefeatUnit(unit: \"test_rogue\", turn_limit: Some(9))",
            "Seize(pos: (20, 1))"
        ),
        ["objective: the seize tile (20, 1) is outside the map"]
    );
    assert_eq!(
        errors(
            &c,
            "DefeatUnit(unit: \"test_rogue\", turn_limit: Some(9))",
            "Survive(turns: 0)"
        ),
        ["objective: survive 0 turns"]
    );
    assert_eq!(
        errors(&c, "turn_limit: Some(9)", "turn_limit: Some(0)"),
        ["objective: turn limit 0"]
    );
}

#[test]
fn pack_errors() {
    let c = content();
    assert_eq!(
        errors(&c, "pack_cap: 3", "pack_cap: 1"),
        ["default_pack: 2 items, more than the pack cap 1"]
    );
    assert_eq!(
        errors(&c, "\"elixir\"]", "\"iron_sword\"]"),
        ["default_pack: \"iron_sword\" isn't a consumable"]
    );
    assert_eq!(
        errors(&c, "\"elixir\"]", "\"nope\"]"),
        ["default_pack: no item \"nope\""]
    );
}

#[test]
fn label_and_trigger_errors() {
    let c = content();
    // Two named enemies labelled "Ro".
    let clash = errors(
        &c,
        "(template: \"test_brigand\", level: Some(3), pos: (8, 3), boss: true, name: Some(\"Garth\"),",
        "(character: \"test_archer\", pos: (8, 3), name: Some(\"Robin\"),",
    );
    assert_eq!(clash.len(), 1, "{clash:?}");
    assert!(clash[0].contains("\"Ro\""), "{clash:?}");
    assert_eq!(
        errors(&c, "scene: \"test\"", "scene: \"nope\""),
        ["trigger 0: no dialogue scene \"nope\""]
    );
}

#[test]
fn embedded_battles_load() {
    let c = content();
    assert!(c.battles.contains_key("test"));
    assert!(c.battles.contains_key("quick"));
    let quick = &c.battles["quick"];
    assert_eq!(quick.reinforcements.len(), 1);
    assert_eq!(quick.difficulty.rewind_charges(), 3);
    // Every battle: data only, loaded and checked like any other.
    for (id, def) in &c.battles {
        assert_eq!(&def.id, id);
    }
}
