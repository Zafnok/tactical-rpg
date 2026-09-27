//! Tests of the art file: the embedded data matches `combat-arts.md`, and
//! every validation rule reports its error.

use trpg_core::{CombatMods, Debuff, StatKind, TimedMods, UnitTag};

use super::*;

fn embedded() -> ArtTable {
    load().unwrap_or_else(|e| panic!("{e:?}"))
}

/// The default effect changed by `f`.
fn with(f: impl FnOnce(&mut ArtEffect)) -> ArtEffect {
    let mut e = ArtEffect::default();
    f(&mut e);
    e
}

fn get(t: &ArtTable, id: &str) -> ArtDef {
    t.get(&ArtId(id.into()))
        .cloned()
        .unwrap_or_else(|| panic!("no art {id}"))
}

/// `combat-arts.md`, *Chapter 1 arts*: every art's name, kind, rank and
/// cost.
#[test]
fn the_embedded_arts_match_the_design_table() {
    use WeaponKind::{Axe, Bow, Gauntlet, Spear, Sword};
    use WeaponRank::{D, E};
    let rows = [
        ("flowing_cut", "Flowing Cut", Sword, E, 2),
        ("guard_break", "Guard Break", Sword, D, 4),
        ("unhorse", "Unhorse", Spear, E, 2),
        ("line_pierce", "Line Pierce", Spear, D, 4),
        ("crushing_swing", "Crushing Swing", Axe, E, 2),
        ("armor_cleave", "Armor Cleave", Axe, D, 4),
        ("close_shot", "Close Shot", Bow, E, 2),
        ("pinning_shot", "Pinning Shot", Bow, D, 3),
        ("pressure_point", "Pressure Point", Gauntlet, E, 2),
        ("sidestep", "Sidestep", Gauntlet, D, 3),
    ];
    let t = embedded();
    assert_eq!(t.arts.len(), rows.len());
    for (id, name, kind, rank, cost) in rows {
        let def = get(&t, id);
        assert_eq!(
            (def.name.as_str(), def.kind, def.rank, def.cost),
            (name, kind, Some(rank), cost),
            "{id}"
        );
    }
}

/// `combat-arts.md`, *Chapter 1 arts*: every art's effect.
#[test]
fn the_embedded_art_effects_match_the_design_table() {
    let debuff = |stat, amount| Some(Debuff { stat, amount });
    let sidestep = TimedMods {
        stats: vec![],
        combat: CombatMods {
            avoid: 20,
            ..CombatMods::default()
        },
    };
    let rows = [
        ("flowing_cut", with(|e| e.sword_followup = Some((3, 2)))),
        (
            "guard_break",
            with(|e| {
                e.hit = 10;
                e.no_counter = true;
            }),
        ),
        (
            "unhorse",
            with(|e| {
                e.hit = 10;
                e.effective = vec![(UnitTag::Mounted, 3)];
            }),
        ),
        ("line_pierce", with(|e| e.line_pierce = true)),
        (
            "crushing_swing",
            with(|e| {
                e.hit = 20;
                e.axe_min_damage = Some(8);
            }),
        ),
        (
            "armor_cleave",
            with(|e| e.effective = vec![(UnitTag::Armored, 2)]),
        ),
        (
            "close_shot",
            with(|e| {
                e.hit = -10;
                e.min_range = Some(1);
            }),
        ),
        (
            "pinning_shot",
            with(|e| e.on_first_hit = debuff(StatKind::Mov, 3)),
        ),
        (
            "pressure_point",
            with(|e| e.on_first_hit = debuff(StatKind::Spd, 3)),
        ),
        ("sidestep", with(|e| e.stance = Some(sidestep))),
    ];
    let t = embedded();
    for (id, effect) in rows {
        assert_eq!(get(&t, id).effect, effect, "{id}");
    }
}

/// No Chapter 1 weapon has arts of its own (`combat-arts.md`: no special
/// weapons in Chapter 1), and the embedded items pass the checks.
#[test]
fn chapter_1_weapons_have_no_arts() {
    let items = crate::item::load().unwrap_or_else(|e| panic!("{e:?}"));
    for (id, item) in &items.items {
        if let ItemDef::Weapon(w) = item {
            assert!(w.arts.is_empty(), "{}", id.0);
        }
    }
    assert_eq!(check_references(&embedded(), &items), []);
}

fn errors(source: &str) -> Vec<String> {
    match from_source("arts.ron", source) {
        Ok(_) => Vec::new(),
        Err(e) => e.into_iter().map(|e| e.to_string()).collect(),
    }
}

fn one(fields: &str) -> String {
    format!("(arts: [({fields})])")
}

#[test]
fn a_valid_file_loads() {
    let t = from_source(
        "arts.ron",
        &one(r#"id: "a", name: "A", kind: Sword, rank: None, cost: 1"#),
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let a = t.get(&ArtId("a".into()));
    assert_eq!(a.map(|a| (a.rank, a.cost)), Some((None, 1)));
    assert_eq!(a.map(|a| &a.effect), Some(&ArtEffect::default()));
}

#[test]
fn every_rule_reports_its_error() {
    let cases = [
        (
            r#"id: "", name: "A", kind: Sword, rank: None, cost: 1"#,
            "an art id is empty",
        ),
        (
            r#"id: "a", name: "A", kind: Sword, rank: None, cost: 0"#,
            "art \"a\": cost must be at least 1",
        ),
        (
            r#"id: "a", name: "A", kind: Spear, rank: None, cost: 1, effect: (sword_followup: Some((3, 2)))"#,
            "sword_followup is for Sword arts only",
        ),
        (
            r#"id: "a", name: "A", kind: Sword, rank: None, cost: 1, effect: (sword_followup: Some((3, 0)))"#,
            "sword_followup's denominator must be at least 1",
        ),
        (
            r#"id: "a", name: "A", kind: Bow, rank: None, cost: 1, effect: (axe_min_damage: Some(8))"#,
            "axe_min_damage is for Axe arts only",
        ),
        (
            r#"id: "a", name: "A", kind: Bow, rank: None, cost: 1, effect: (min_range: Some(0))"#,
            "min_range must be at least 1",
        ),
        (
            r#"id: "a", name: "A", kind: Bow, rank: None, cost: 1, effect: (on_first_hit: Some((stat: Mov, amount: 0)))"#,
            "a debuff's amount must be at least 1",
        ),
        (
            r#"id: "a", name: "A", kind: Bow, rank: None, cost: 1, effect: (effective: [(Flying, 0)])"#,
            "an effectiveness multiplier must be at least 1",
        ),
    ];
    for (fields, message) in cases {
        let found = errors(&one(fields));
        assert!(
            found.len() == 1 && found[0].contains(message),
            "{fields}: {found:?}"
        );
    }
    // Fine at the edges.
    for fields in [
        r#"id: "a", name: "A", kind: Sword, rank: None, cost: 1, effect: (sword_followup: Some((3, 1)))"#,
        r#"id: "a", name: "A", kind: Axe, rank: None, cost: 1, effect: (axe_min_damage: Some(8))"#,
        r#"id: "a", name: "A", kind: Bow, rank: None, cost: 1, effect: (min_range: Some(1), effective: [(Flying, 1)])"#,
        r#"id: "a", name: "A", kind: Bow, rank: None, cost: 1, effect: (on_first_hit: Some((stat: Mov, amount: 1)))"#,
    ] {
        assert_eq!(errors(&one(fields)), Vec::<String>::new(), "{fields}");
    }
}

#[test]
fn duplicates_unknown_fields_and_kinds_are_errors() {
    let art = r#"(id: "a", name: "A", kind: Sword, rank: Some(E), cost: 1)"#;
    let found = errors(&format!("(arts: [{art}, {art}])"));
    assert!(
        found.len() == 1 && found[0].contains("duplicate art id \"a\""),
        "{found:?}"
    );
    assert_eq!(
        errors(&one(
            r#"id: "a", name: "A", kind: Staff, rank: None, cost: 1"#
        ))
        .len(),
        1
    );
    assert_eq!(
        errors(&one(
            r#"id: "a", name: "A", kind: Sword, rank: None, cost: 1, effect: (might: 3)"#
        ))
        .len(),
        1
    );
    // Every problem of one art is reported.
    let found = errors(&one(
        r#"id: "", name: "A", kind: Bow, rank: None, cost: 0, effect: (min_range: Some(0))"#,
    ));
    assert_eq!(found.len(), 3, "{found:?}");
}

#[test]
fn weapon_arts_must_exist_be_weapon_arts_and_match_the_kind() {
    let arts = from_source(
        "arts.ron",
        r#"(arts: [
            (id: "moonlight", name: "Moonlight", kind: Sword, rank: None, cost: 3),
            (id: "cut", name: "Cut", kind: Sword, rank: Some(E), cost: 2),
            (id: "volley", name: "Volley", kind: Bow, rank: None, cost: 2),
        ])"#,
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let mut items = crate::item::load().unwrap_or_else(|e| panic!("{e:?}"));
    let give = |items: &mut ItemTable, art: &str| {
        if let Some(ItemDef::Weapon(w)) = items.items.get_mut(&trpg_core::ItemId::new("iron_sword"))
        {
            w.arts = vec![ArtId(art.into())];
        }
    };
    give(&mut items, "moonlight");
    assert_eq!(check_references(&arts, &items), []);
    for (art, message) in [
        ("nope", "unknown art \"nope\""),
        ("cut", "\"cut\" is a rank art"),
        ("volley", "\"volley\" is a Bow art, not a Sword one"),
    ] {
        give(&mut items, art);
        let found: Vec<String> = check_references(&arts, &items)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(
            found.len() == 1
                && found[0].contains("weapon \"iron_sword\"")
                && found[0].contains(message),
            "{art}: {found:?}"
        );
    }
}
