use trpg_core::{
    ArmourWeight, ClassId, ClassTable, CombatRules, Faction, Forecast, LoadoutDef, Pos,
    SideForecast, TerrainRules, TerrainTable, Unit, UnitId, forecast,
};

use super::*;
use crate::palette::PaletteDef;
use crate::terrain::TerrainDef;

fn items() -> ItemTable {
    let t = load();
    assert!(t.is_ok(), "{t:?}");
    t.unwrap_or_default()
}

fn id(s: &str) -> ItemId {
    ItemId::new(s)
}

// ---- The embedded file matches weapons-and-items.md ----------------------------

/// One row of the design's weapon table: `[might, hit, crit, weight]`,
/// range, durability, price.
fn row(
    name: &str,
    (kind, rank): (WeaponKind, WeaponRank),
    [might, hit, crit, weight]: [StatValue; 4],
    (range, durability, price): (u32, u32, u32),
) -> WeaponDef {
    WeaponDef {
        name: name.into(),
        kind,
        rank,
        might,
        hit,
        crit,
        weight,
        min_range: range,
        max_range: range,
        damage_type: DamageType::Physical,
        durability,
        effective: vec![],
        price,
    }
}

/// Every starter weapon, exactly as in the design's table.
#[test]
fn weapons_match_the_design() {
    use WeaponKind::{Axe, Bow, Gauntlet, Spear, Sword};
    use WeaponRank::{D, E};
    let table = [
        (
            "iron_sword",
            row("Iron Sword", (Sword, E), [5, 90, 0, 2], (1, 20, 460)),
        ),
        (
            "steel_sword",
            row("Steel Sword", (Sword, D), [8, 75, 0, 4], (1, 25, 600)),
        ),
        (
            "iron_spear",
            row("Iron Spear", (Spear, E), [7, 80, 0, 4], (1, 20, 360)),
        ),
        (
            "steel_spear",
            row("Steel Spear", (Spear, D), [10, 70, 0, 6], (1, 25, 480)),
        ),
        (
            "iron_axe",
            row("Iron Axe", (Axe, E), [8, 75, 0, 6], (1, 20, 270)),
        ),
        (
            "steel_axe",
            row("Steel Axe", (Axe, D), [11, 65, 0, 8], (1, 25, 360)),
        ),
        (
            "iron_bow",
            row("Iron Bow", (Bow, E), [6, 85, 0, 3], (2, 20, 540)),
        ),
        (
            "steel_bow",
            row("Steel Bow", (Bow, D), [9, 70, 0, 5], (2, 25, 720)),
        ),
        (
            "iron_gauntlets",
            row("Iron Gauntlets", (Gauntlet, E), [3, 95, 5, 1], (1, 20, 400)),
        ),
        (
            "steel_gauntlets",
            row(
                "Steel Gauntlets",
                (Gauntlet, D),
                [5, 85, 5, 2],
                (1, 25, 560),
            ),
        ),
    ];
    let t = items();
    for (key, expected) in &table {
        assert_eq!(t.weapon(&id(key)), Some(expected), "{key}");
    }
    let weapons = t
        .items
        .values()
        .filter(|d| matches!(d, ItemDef::Weapon(_)))
        .count();
    assert_eq!(weapons, table.len());
}

#[test]
fn gear_and_consumables_match_the_design() {
    let t = items();
    let stats = |f: fn(&mut Stats)| {
        let mut s = Stats::default();
        f(&mut s);
        s
    };
    let armour = [
        (
            "leather_vest",
            ArmourWeight::Light,
            stats(|s| s.def = 1),
            0,
            300,
        ),
        (
            "chain_mail",
            ArmourWeight::Medium,
            stats(|s| s.def = 3),
            2,
            800,
        ),
        (
            "iron_plate",
            ArmourWeight::Heavy,
            stats(|s| s.def = 5),
            5,
            1500,
        ),
        (
            "warded_robe",
            ArmourWeight::Light,
            stats(|s| s.res = 2),
            0,
            700,
        ),
    ];
    for (key, weight_class, bonus, weight, price) in armour {
        let a = t.armour(&id(key));
        assert_eq!(
            a.map(|a| (a.weight_class, a.bonus, a.weight, a.price)),
            Some((weight_class, bonus, weight, price)),
            "{key}"
        );
    }
    let accessories = [
        ("speed_ring", stats(|s| s.spd = 2), 2000),
        ("power_ring", stats(|s| s.str = 2), 2000),
        ("focus_charm", stats(|s| s.dex = 2), 1500),
    ];
    for (key, bonus, price) in accessories {
        let a = t.accessory(&id(key));
        assert_eq!(a.map(|a| (a.bonus, a.price)), Some((bonus, price)), "{key}");
    }
    assert_eq!(
        t.consumable(&id("potion")).map(|c| (c.effect, c.price)),
        Some((ConsumableEffect::Heal(10), 300))
    );
    assert_eq!(
        t.consumable(&id("elixir")).map(|c| (c.effect, c.price)),
        Some((ConsumableEffect::HealFull, 3000))
    );
    assert_eq!(t.items.len(), 10 + 4 + 3 + 2);
    assert_eq!(
        t.get(&id("warded_robe")).map(ItemDef::name),
        Some("Warded Robe")
    );
}

/// Traits, rank speed, thresholds, broken penalties and weapon EXP numbers
/// are the design's (which `CombatRules` and `WeaponRules` default to).
#[test]
fn rules_and_traits_match_the_design() {
    let t = items();
    assert_eq!(t.rules, WeaponRules::default());
    assert_eq!(t.combat_rules(), CombatRules::default());
    let rules = CombatRules::default();
    for kind in ALL_KINDS {
        assert_eq!(t.trait_of(kind), rules.type_trait(kind), "{kind:?}");
    }
}

// ---- Validation -------------------------------------------------------------------

const RULES: &str = "rules: (rank_speed: (0, 0, 1, 2, 3, 4), rank_exp: (30, 70, 120, 180, 250), \
    broken_might_divisor: 2, broken_hit_penalty: 20, exp_hit: 2, exp_miss: 1, \
    exp_damage_divisor: 5, exp_art_multiplier: 2, default_pack_cap: 6),";

const KINDS: &str = "kinds: [(kind: Sword, trait: SwordFollowUp), (kind: Spear, trait: None), \
    (kind: Axe, trait: None), (kind: Bow, trait: None), (kind: Gauntlet, trait: None)],";

fn weapon(id: &str, extra: &str) -> String {
    format!(
        "\n        (id: \"{id}\", name: \"W\", kind: Sword, rank: E, might: 5, hit: 90, crit: 0, \
         weight: 2, range: (1, 1), durability: 20, price: 1{extra}),"
    )
}

fn file(rules: &str, kinds: &str, weapons: &str, rest: &str) -> String {
    format!("(\n    {rules}\n    {kinds}\n    weapons: [{weapons}\n    ],\n    {rest}\n)")
}

const REST: &str = "armour: [], accessories: [], consumables: [],";

fn errors(src: &str) -> Vec<String> {
    from_source("it.ron", src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_minimal_file_loads() {
    let src = file(
        RULES,
        KINDS,
        &weapon("w", ", effective: [(Armored, 2)], damage_type: Magical"),
        "armour: [(id: \"a\", name: \"A\", weight_class: Heavy, bonus: (hp: 1, str: 2, mag: 3, dex: 4, spd: 5, def: 6, res: 7, mov: 8), weight: 1, price: 2)],\n    \
         accessories: [(id: \"r\", name: \"R\", bonus: (), price: 3)],\n    \
         consumables: [(id: \"p\", name: \"P\", effect: Heal(1), price: 4)],",
    );
    let t = from_source("it.ron", &src);
    assert!(t.is_ok(), "{t:?}");
    let t = t.unwrap_or_default();
    let w = t.weapon(&id("w"));
    assert_eq!(
        w.map(|w| (w.damage_type, w.effective.clone())),
        Some((DamageType::Magical, vec![(UnitTag::Armored, 2)]))
    );
    assert_eq!(
        t.armour(&id("a")).map(|a| a.bonus),
        Some(Stats::from_growable([1, 2, 3, 4, 5, 6, 7], 8))
    );
    assert_eq!(
        t.accessory(&id("r")).map(|a| a.bonus),
        Some(Stats::default())
    );
    assert_eq!(t.trait_of(WeaponKind::Spear), WeaponTrait::None);
}

#[test]
fn weapon_errors() {
    let weapons = [
        weapon("a", "").replace("range: (1, 1)", "range: (0, 1)"),
        weapon("b", "").replace("range: (1, 1)", "range: (3, 2)"),
        weapon("c", "").replace("durability: 20", "durability: 0"),
        weapon("d", "").replace("might: 5", "might: -1"),
        weapon("a", ""),
    ]
    .concat();
    let src = file(RULES, KINDS, &weapons, REST);
    assert_eq!(
        errors(&src),
        [
            "it.ron:5: weapon \"a\": range (0, 1) must have 1 <= min <= max",
            "it.ron:6: weapon \"b\": range (3, 2) must have 1 <= min <= max",
            "it.ron:7: weapon \"c\": durability must be at least 1",
            "it.ron:8: weapon \"d\": might, hit, crit and weight can't be negative",
            "it.ron:5: duplicate item id \"a\"",
        ]
    );
}

#[test]
fn ids_are_unique_across_lists_and_not_empty() {
    let src = file(
        RULES,
        KINDS,
        &weapon("x", ""),
        "armour: [(id: \"x\", name: \"A\", weight_class: Light, bonus: (), weight: -1, price: 0)],\n    \
         accessories: [(id: \"\", name: \"R\", bonus: (), price: 3)],\n    \
         consumables: [(id: \"p\", name: \"P\", effect: Heal(0), price: 4)],",
    );
    assert_eq!(
        errors(&src),
        [
            "it.ron:5: armour \"x\": weight can't be negative",
            "it.ron:5: duplicate item id \"x\"",
            "it.ron:8: an item id is empty",
            "it.ron:9: consumable \"p\": Heal must be at least 1",
        ]
    );
}

#[test]
fn kind_and_rule_errors() {
    let kinds = "kinds: [(kind: Sword, trait: None), (kind: Sword, trait: None)],";
    let rules = RULES
        .replace("(30, 70, 120, 180, 250)", "(30, 30, 120, 180, 250)")
        .replace("broken_might_divisor: 2", "broken_might_divisor: 0")
        .replace("exp_damage_divisor: 5", "exp_damage_divisor: 0");
    let src = file(&rules, kinds, "", REST);
    assert_eq!(
        errors(&src),
        [
            "it.ron:2: rank_exp must be above 0 and strictly increasing",
            "it.ron:2: broken_might_divisor must be at least 1",
            "it.ron:2: exp_damage_divisor must be at least 1",
            "it.ron:3: kind Sword is listed twice",
            "it.ron:3: kind Spear has no trait entry",
            "it.ron:3: kind Axe has no trait entry",
            "it.ron:3: kind Bow has no trait entry",
            "it.ron:3: kind Gauntlet has no trait entry",
        ]
    );
    // Divisors of exactly 1 are fine.
    let one = RULES
        .replace("broken_might_divisor: 2", "broken_might_divisor: 1")
        .replace("exp_damage_divisor: 5", "exp_damage_divisor: 1");
    assert!(from_source("it.ron", &file(&one, KINDS, "", REST)).is_ok());
    let zero = RULES.replace("(30, 70,", "(0, 70,");
    assert_eq!(
        errors(&file(&zero, KINDS, "", REST)),
        ["it.ron:2: rank_exp must be above 0 and strictly increasing"]
    );
}

#[test]
fn syntax_and_unknown_fields_are_positioned() {
    let src = file(RULES, KINDS, &weapon("w", ", colour: 3"), REST);
    let errs = from_source("it.ron", &src).err().unwrap_or_default();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].line, Some(5));
    assert!(errs[0].message.contains("colour"));
    assert_eq!(from_source("it.ron", "").err().map(|e| e.len()), Some(1));
}

// ---- Worked examples W1–W4 from real units and items.ron --------------------------

fn classes() -> ClassTable {
    let t = TerrainDef::load(PaletteDef::load().ok().as_ref()).unwrap_or_default();
    crate::class::load(Some(&t.rules.movement_types)).unwrap_or_default()
}

fn terrain(name: &str) -> TerrainRules {
    let t: TerrainTable = TerrainDef::load(PaletteDef::load().ok().as_ref())
        .unwrap_or_default()
        .rules;
    let found = t.terrains.into_iter().find(|r| r.name == name);
    assert!(found.is_some(), "no terrain {name}");
    found.unwrap_or_else(|| TerrainRules {
        name: String::new(),
        move_cost: vec![],
        defense: 0,
        avoid: 0,
        heal_percent: 0,
    })
}

/// A real unit of `class` with the example's `[HP, Str, Dex, Spd, Def]`,
/// its weapon rank in `kind` set to `rank` and the loadout from items.ron.
fn unit(
    class: &str,
    [hp, str, dex, spd, def]: [StatValue; 5],
    (kind, rank): (WeaponKind, WeaponRank),
    weapons: &[&str],
    armour: Option<&str>,
) -> Unit {
    let classes = classes();
    let made = Unit::generic(
        UnitId(1),
        &ClassId(class.into()),
        &classes,
        1,
        Faction::Player,
        Pos::new(0, 0),
    );
    assert!(made.is_ok(), "{made:?}");
    let mut u = made.unwrap_or_else(|_| unreachable!());
    u.stats = Stats::from_growable([hp, str, 0, dex, spd, def, 0], u.stats.mov);
    u.hp = hp;
    u.weapon_ranks.insert(kind, rank);
    let loadout = LoadoutDef {
        weapons: weapons.iter().map(|w| id(w)).collect(),
        armour: armour.map(id),
        accessory: None,
    };
    let u = u.with_loadout(&loadout, &classes, &items());
    assert!(u.is_ok(), "{u:?}");
    u.unwrap_or_else(|_| unreachable!())
}

/// The forecast of `a` on `ta` attacking `d` on `td` from `distance`.
fn fight(a: &Unit, ta: &str, d: &Unit, td: &str, distance: u32) -> Forecast {
    let (classes, items) = (classes(), items());
    let (ta, td) = (terrain(ta), terrain(td));
    let input = |u: &Unit, t| {
        let class = classes.get(&u.class).unwrap_or_else(|| unreachable!());
        u.combat_input(class, &classes, &items, None, t)
    };
    let f = forecast(
        &items.combat_rules(),
        &input(a, &ta),
        &input(d, &td),
        distance,
    );
    assert!(f.is_some());
    f.unwrap_or_else(|| unreachable!())
}

fn side(damage: StatValue, followup: StatValue, hit: u8, crit: u8, strikes: u8) -> SideForecast {
    SideForecast {
        damage,
        followup_damage: followup,
        hit,
        crit,
        strikes,
        effective: false,
        broken: false,
        affinity: None,
    }
}

#[test]
fn w1_sword_follow_up_vs_axe() {
    let swordfighter = unit(
        "swordsman",
        [22, 8, 7, 9, 5],
        (WeaponKind::Sword, WeaponRank::E),
        &["iron_sword"],
        None,
    );
    let brigand = unit(
        "brigand",
        [20, 9, 3, 5, 4],
        (WeaponKind::Axe, WeaponRank::E),
        &["iron_axe"],
        None,
    );
    let f = fight(&swordfighter, "Plain", &brigand, "Plain", 1);
    assert_eq!(f.attacker, side(9, 10, 94, 3, 2));
    assert_eq!(f.defender, Some(side(12, 12, 63, 0, 1)));
}

#[test]
fn w2_spear_vs_cavalry_and_axe_minimum() {
    // A Rider stands in for the design's untagged soldier: its Mounted tag
    // changes nothing here (the axe has no effectiveness).
    let soldier = |def| {
        unit(
            "rider",
            [20, 7, 5, 6, def],
            (WeaponKind::Spear, WeaponRank::E),
            &["iron_spear"],
            Some("leather_vest"),
        )
    };
    let cavalier = unit(
        "iron_rider",
        [24, 8, 6, 7, 7],
        (WeaponKind::Axe, WeaponRank::C),
        &["iron_axe"],
        None,
    );
    let f = fight(&soldier(5), "Plain", &cavalier, "Plain", 1);
    let effective = SideForecast {
        effective: true,
        ..side(14, 14, 76, 1, 1)
    };
    assert_eq!(f.attacker, effective);
    assert_eq!(f.defender, Some(side(10, 10, 75, 2, 1)));
    // W2b: Def 20 (+1 vest). The axe still deals its minimum 5.
    let f = fight(&soldier(20), "Plain", &cavalier, "Plain", 1);
    assert_eq!(f.defender.map(|d| d.damage), Some(5));
}

#[test]
fn w3_bow_vs_flier_at_range_no_counter() {
    let archer = unit(
        "archer",
        [18, 6, 8, 7, 4],
        (WeaponKind::Bow, WeaponRank::E),
        &["iron_bow"],
        None,
    );
    let flier = unit(
        "flier",
        [18, 7, 7, 12, 4],
        (WeaponKind::Spear, WeaponRank::E),
        &["iron_spear"],
        None,
    );
    let f = fight(&archer, "Plain", &flier, "Forest", 2);
    let effective = SideForecast {
        effective: true,
        ..side(20, 20, 77, 3, 1)
    };
    assert_eq!(f.attacker, effective);
    assert_eq!(f.defender, None);
}

#[test]
fn w4_gauntlet_avoid_and_broken_weapon() {
    let mut fighter = unit(
        "raider",
        [26, 10, 5, 6, 5],
        (WeaponKind::Axe, WeaponRank::E),
        &["iron_axe"],
        None,
    );
    if let Some(w) = fighter.loadout.weapons[0].as_mut() {
        w.durability_left = 0;
    }
    let brawler = unit(
        "brawler",
        [22, 7, 8, 10, 3],
        (WeaponKind::Gauntlet, WeaponRank::E),
        &["iron_gauntlets"],
        None,
    );
    let f = fight(&fighter, "Plain", &brawler, "Plain", 1);
    let broken = SideForecast {
        broken: true,
        ..side(11, 11, 30, 0, 1)
    };
    assert_eq!(f.attacker, broken);
    assert_eq!(f.defender, Some(side(5, 5, 99, 8, 2)));
}
