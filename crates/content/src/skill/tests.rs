//! Tests of the skill file: the embedded data matches `progression.md`, and
//! every validation rule reports its error.

use trpg_core::{
    ClassDef, ClassId, CombatMods, Condition, StatKind, TimedMods, UnitTags, WeaponKind,
};

use super::*;

fn embedded() -> SkillTable {
    load().unwrap_or_else(|e| panic!("{e:?}"))
}

fn get(t: &SkillTable, id: &str) -> SkillDef {
    t.get(&SkillId(id.into()))
        .cloned()
        .unwrap_or_else(|| panic!("no skill {id}"))
}

fn passive(effects: Vec<PassiveEffect>) -> SkillKind {
    SkillKind::Passive(effects)
}

fn when(mods: CombatMods, when: Condition) -> SkillKind {
    passive(vec![PassiveEffect::CombatMod { mods, when }])
}

fn active(cost: SkillCost, effect: ActiveEffect) -> SkillKind {
    SkillKind::Active { cost, effect }
}

fn strike(with: WeaponReq, mods: CombatMods) -> ActiveEffect {
    ActiveEffect::Strike {
        with,
        mods,
        range: 0,
        stance: None,
        post_move: 0,
        drain: false,
    }
}

fn m() -> CombatMods {
    CombatMods::default()
}

/// Checks each `(id, family, rank, kind)` against the embedded table.
fn check_rows(cases: Vec<(&str, &str, u8, SkillKind)>) {
    let t = embedded();
    for (id, family, rank, kind) in cases {
        let def = get(&t, id);
        assert_eq!((def.family.as_str(), def.rank), (family, rank), "{id}");
        assert_eq!(def.kind, kind, "{id}");
    }
}

/// The skill table's rows for a sample of passives, one per condition type
/// and one per kind of effect, against `progression.md`.
#[test]
fn the_embedded_passives_match_the_design_table() {
    let cases = vec![
        (
            "sword_focus_1",
            "sword_focus",
            1,
            when(
                CombatMods { crit: 10, ..m() },
                Condition::WeaponKindEquipped(WeaponKind::Sword),
            ),
        ),
        (
            "steadfast_2",
            "steadfast",
            2,
            passive(vec![PassiveEffect::StatWhile {
                stat: StatKind::Def,
                amount: 4,
                when: Condition::NotOwnPhase,
            }]),
        ),
        (
            "fury",
            "fury",
            1,
            when(CombatMods { crit: 15, ..m() }, Condition::HpAtMostHalf),
        ),
        (
            "charge_1",
            "charge",
            1,
            when(CombatMods { might: 2, ..m() }, Condition::MovedAtLeast(4)),
        ),
        (
            "sky_dodge_1",
            "sky_dodge",
            1,
            when(
                CombatMods { avoid: 10, ..m() },
                Condition::AgainstWeaponKind(WeaponKind::Bow),
            ),
        ),
        (
            "evasion_1",
            "evasion",
            1,
            when(CombatMods { avoid: 10, ..m() }, Condition::Always),
        ),
        (
            "white_magic_2",
            "white_magic",
            2,
            passive(vec![PassiveEffect::HealBonus(4)]),
        ),
        (
            "leadership_2",
            "leadership",
            2,
            passive(vec![PassiveEffect::AllyAura {
                radius: 2,
                mods: CombatMods {
                    hit: 10,
                    avoid: 10,
                    ..m()
                },
            }]),
        ),
    ];
    check_rows(cases);
}

/// The skill table's rows for a sample of actives, one per kind of effect,
/// against `progression.md` and `combat-arts.md`'s costs.
#[test]
fn the_embedded_actives_match_the_design_tables() {
    let t = embedded();
    let dur = SkillCost::Durability;
    let cases = vec![
        (
            "keen_edge",
            "keen_edge",
            1,
            active(
                dur(3),
                strike(
                    WeaponReq::Kind(WeaponKind::Sword),
                    CombatMods {
                        hit: 30,
                        crit: 10,
                        ..m()
                    },
                ),
            ),
        ),
        (
            "heavy_blow",
            "heavy_blow",
            1,
            active(
                dur(3),
                strike(
                    WeaponReq::Any,
                    CombatMods {
                        might: 5,
                        single_strike: true,
                        ..m()
                    },
                ),
            ),
        ),
        (
            "overcast",
            "overcast",
            1,
            active(
                SkillCost::ExtraSpellUse,
                strike(WeaponReq::Spell, CombatMods { might: 5, ..m() }),
            ),
        ),
        (
            "fortify",
            "fortify",
            1,
            active(
                dur(5),
                ActiveEffect::Buff {
                    area: Area::Own,
                    mods: TimedMods {
                        stats: vec![(StatKind::Def, 8), (StatKind::Res, 8)],
                        combat: m(),
                    },
                },
            ),
        ),
        (
            "sanctuary_2",
            "sanctuary",
            2,
            active(
                dur(5),
                ActiveEffect::Heal {
                    radius: 2,
                    power: 5,
                },
            ),
        ),
        (
            "shove",
            "shove",
            1,
            active(dur(3), ActiveEffect::Push { collision: 5 }),
        ),
    ];
    check_rows(cases);
    assert_eq!(get(&t, "keen_edge").name, "Keen Edge");
    // Every tier 1–2 skill plus the Flier's.
    assert_eq!(t.skills.len(), 48);
}

#[test]
fn every_class_skill_exists_with_the_right_kind() {
    let classes = crate::class::load(Some(
        &crate::terrain::TerrainDef::load(None)
            .unwrap_or_default()
            .rules
            .movement_types,
    ))
    .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(check_references(&embedded(), &classes), []);
    // And every skill belongs to some class.
    let named: BTreeSet<&SkillId> = classes
        .classes
        .values()
        .flat_map(|c| c.active.iter().chain(&c.passives))
        .collect();
    for id in embedded().skills.keys() {
        assert!(named.contains(id), "{} is in no class", id.0);
    }
}

fn errors_of(body: &str) -> Vec<String> {
    let source = format!("(skills: [{body}])");
    match from_source("skills.ron", &source) {
        Ok(_) => vec![],
        Err(e) => e.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn family_and_rank_default_to_the_id_and_1() {
    let t = from_source(
        "skills.ron",
        r#"(skills: [(id: "a", name: "A", kind: Active(cost: Durability(1), effect: Push(collision: 5)))])"#,
    )
    .unwrap_or_default();
    let a = get(&t, "a");
    assert_eq!((a.family.as_str(), a.rank), ("a", 1));
}

#[test]
fn validation_errors() {
    let push = "kind: Active(cost: Durability(1), effect: Push(collision: 5))";
    let cases = [
        (
            format!(r#"(id: "", name: "x", {push})"#),
            vec!["a skill id is empty"],
        ),
        (
            format!(r#"(id: "a", name: "x", rank: 0, {push})"#),
            vec!["skill \"a\": rank must be at least 1"],
        ),
        (
            format!(r#"(id: "a", name: "x", {push}), (id: "a", name: "y", family: "b", {push})"#),
            vec!["duplicate skill id \"a\""],
        ),
        (
            format!(
                r#"(id: "a", name: "x", family: "f", rank: 2, {push}), (id: "b", name: "y", family: "f", rank: 2, {push})"#
            ),
            vec!["skill \"b\": family \"f\" already has a rank 2"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: Durability(0), effect: Push(collision: 5)))"#.into(),
            vec!["skill \"a\": a durability cost must be at least 1"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: Durability(2), effect: Strike(with: Spell)))"#
                .into(),
            vec!["skill \"a\": a spell active costs ExtraSpellUse, not durability"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: ExtraSpellUse, effect: Strike(with: Any)))"#
                .into(),
            vec!["skill \"a\": only spell actives (Strike with Spell) cost ExtraSpellUse"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: ExtraSpellUse, effect: Push(collision: 5)))"#.into(),
            vec!["skill \"a\": only spell actives (Strike with Spell) cost ExtraSpellUse"],
        ),
        (
            r#"(id: "a", name: "x", kind: Passive([AllyAura(radius: 0, mods: ())]))"#.into(),
            vec!["skill \"a\": an aura's radius must be at least 1"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: Durability(1), effect: Buff(area: Allies(radius: 0), mods: ())))"#
                .into(),
            vec!["skill \"a\": a buff's radius must be at least 1"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: Durability(1), effect: Push(collision: -1)))"#
                .into(),
            vec!["skill \"a\": collision damage can't be negative"],
        ),
        (
            r#"(id: "a", name: "x", kind: Active(cost: Durability(1), effect: Heal(radius: 0, power: 5)))"#
                .into(),
            vec!["skill \"a\": a heal's radius must be at least 1"],
        ),
    ];
    for (body, expected) in cases {
        assert_eq!(errors_of(&body), expected, "{body}");
    }
    // Valid shapes pass.
    let ok = [
        r#"(id: "a", name: "x", kind: Active(cost: ExtraSpellUse, effect: Strike(with: Spell, drain: true)))"#,
        r#"(id: "a", name: "x", kind: Active(cost: Durability(1), effect: Buff(area: Own, mods: (stats: [(Def, 1)]))))"#,
        r#"(id: "a", name: "x", kind: Passive([AllyAura(radius: 1, mods: (hit: 1)), HealBonus(1)]))"#,
        r#"(id: "a", name: "x", kind: Active(cost: Durability(1), effect: Heal(radius: 1, power: 5)))"#,
        r#"(id: "a", name: "x", kind: Active(cost: Durability(1), effect: Push(collision: 0)))"#,
    ];
    for body in ok {
        assert_eq!(errors_of(body), Vec::<String>::new(), "{body}");
    }
}

#[test]
fn errors_point_at_the_skill_and_parse_errors_have_a_position() {
    let source = "(skills: [\n  (id: \"a\", name: \"x\", rank: 0, kind: Active(cost: Durability(1), effect: Push(collision: 5))),\n])";
    let errors = from_source("skills.ron", source).err().unwrap_or_default();
    assert_eq!(errors.first().and_then(|e| e.line), Some(2));
    let errors = from_source("skills.ron", "(skills: [(id: 1)])")
        .err()
        .unwrap_or_default();
    assert_eq!(errors.len(), 1);
    assert!(errors.first().is_some_and(|e| e.line.is_some()));
    let errors = from_source("skills.ron", "(skills: [], extra: 1)")
        .err()
        .unwrap_or_default();
    assert_eq!(errors.len(), 1);
}

fn class(id: &str, active: Option<&str>, passives: &[&str]) -> ClassDef {
    ClassDef {
        id: ClassId(id.into()),
        name: id.into(),
        tier: 1,
        movement_type: trpg_core::MovementTypeId(0),
        move_points: 5,
        base: trpg_core::Stats::default(),
        caps: trpg_core::Stats::default(),
        growths: trpg_core::Growths::default(),
        weapons: vec![],
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: active.map(|s| SkillId(s.into())),
        passives: passives.iter().map(|s| SkillId((*s).into())).collect(),
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: vec![],
        affinities: vec![],
    }
}

#[test]
fn reference_errors_name_the_class_and_the_problem() {
    let classes = ClassTable {
        classes: [
            class("swordsman", Some("sword_focus_1"), &["keen_edge", "nope"]),
            class("guard", Some("brace"), &["steadfast_1"]),
        ]
        .into_iter()
        .map(|c| (c.id.clone(), c))
        .collect(),
        ..ClassTable::default()
    };
    let errors = check_references(&embedded(), &classes);
    let messages: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "class \"swordsman\": \"sword_focus_1\" is not an active",
            "class \"swordsman\": \"keen_edge\" is not a passive",
            "class \"swordsman\": unknown skill \"nope\"",
        ]
    );
    assert!(
        errors
            .iter()
            .all(|e| e.file == "assets/data/classes.ron" && e.line.is_some())
    );
    // A class missing from the file gets no line.
    let lost = ClassTable {
        classes: [class("lost", Some("nope"), &[])]
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect(),
        ..ClassTable::default()
    };
    let errors = check_references(&embedded(), &lost);
    assert_eq!(errors.first().map(|e| e.line), Some(None));
}
