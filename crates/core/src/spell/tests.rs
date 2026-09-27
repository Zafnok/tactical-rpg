use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;
use crate::class::{ClassDef, ClassId, UnitTags};
use crate::geom::Pos;
use crate::stats::{Growths, Stats};
use crate::terrain::MovementTypeId;
use crate::unit::{ClassRecord, Faction, UnitId};

fn sid(id: &str) -> SpellId {
    SpellId::new(id)
}

fn attack(id: &str, uses: u8) -> SpellDef {
    SpellDef {
        id: sid(id),
        name: id.into(),
        kind: SpellKind::Attack {
            might: 5,
            hit: 90,
            crit: 3,
            effective: vec![(UnitTag::Flying, 2)],
        },
        element: Element::Fire,
        min_range: 1,
        max_range: 2,
        uses,
        terrain_effect: Some(TerrainEffect {
            from: vec![TerrainId(1)],
            to: TerrainId(2),
            lasts: EffectDuration::UntilCastersNextPhase {
                then: TerrainId(3),
                damage: 5,
            },
        }),
    }
}

fn heal(id: &str, uses: u8) -> SpellDef {
    SpellDef {
        id: sid(id),
        name: id.into(),
        kind: SpellKind::Heal { heal_power: 10 },
        element: Element::None,
        min_range: 1,
        max_range: 1,
        uses,
        terrain_effect: None,
    }
}

/// `fire` (10 uses), `force` (8), `heal` (8), `mend` (4).
fn table() -> SpellTable {
    SpellTable {
        spells: [
            attack("fire", 10),
            attack("force", 8),
            heal("heal", 8),
            heal("mend", 4),
        ]
        .into_iter()
        .map(|s| (s.id.clone(), s))
        .collect(),
    }
}

fn class(id: &str, spells: &[(u8, &str)]) -> ClassDef {
    ClassDef {
        id: ClassId(id.into()),
        name: id.into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 5,
        base: Stats::default(),
        caps: Stats::default(),
        growths: Growths::default(),
        weapons: vec![],
        armour: vec![],
        tags: UnitTags::default(),
        promotes_to: vec![],
        active: None,
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: spells.iter().map(|&(l, s)| (l, sid(s))).collect(),
        affinities: vec![],
    }
}

/// `mage` (fire at class level 1, force at 5), `cleric` (heal at 1, mend at
/// 3) and `sage` (mend at 1; 0 weapon slots). Class level cap 10.
fn classes() -> ClassTable {
    let sage = ClassDef {
        weapon_slots: 0,
        ..class("sage", &[(1, "mend")])
    };
    ClassTable {
        classes: [
            class("mage", &[(1, "fire"), (5, "force")]),
            class("cleric", &[(1, "heal"), (3, "mend")]),
            sage,
        ]
        .into_iter()
        .map(|c| (c.id.clone(), c))
        .collect(),
        class_level_cap: 10,
        ..ClassTable::default()
    }
}

fn record(class_level: u8) -> ClassRecord {
    ClassRecord {
        class_level,
        class_points: 0,
    }
}

/// A level-`level` generic mage at class level 1.
fn mage(level: u32) -> Unit {
    let mut u = Unit::generic(
        UnitId(1),
        &ClassId("mage".into()),
        &classes(),
        level,
        Faction::Player,
        Pos::new(0, 0),
    )
    .unwrap();
    u.learned.clear();
    u
}

fn set(ids: &[&str]) -> BTreeSet<SpellId> {
    ids.iter().map(|s| sid(s)).collect()
}

#[test]
fn spell_id_and_def_helpers() {
    assert_eq!(SpellId::new("fire"), SpellId("fire".into()));
    let fire = attack("fire", 10);
    assert!(fire.is_attack());
    assert!(!heal("heal", 8).is_attack());
    assert_eq!(
        (
            fire.in_range(0),
            fire.in_range(1),
            fire.in_range(2),
            fire.in_range(3)
        ),
        (false, true, true, false)
    );
    assert_eq!(
        table().get(&sid("mend")).map(|s| s.uses),
        Some(4),
        "lookup by id"
    );
    assert_eq!(table().get(&sid("nope")), None);
}

#[test]
fn an_attack_spell_fights_like_a_weightless_magical_weapon() {
    assert_eq!(
        attack("fire", 10).weapon_stats(),
        Some(WeaponStats {
            kind: None,
            trait_: WeaponTrait::None,
            might: 5,
            hit: 90,
            crit: 3,
            weight: 0,
            min_range: 1,
            max_range: 2,
            damage_type: DamageType::Magical,
            effective: vec![(UnitTag::Flying, 2)],
            broken: false,
            element: Element::Fire,
        })
    );
    assert_eq!(heal("heal", 8).weapon_stats(), None);
}

#[test]
fn spell_state_fills_spends_and_stops_at_zero() {
    let state = SpellState::full(&set(&["fire", "mend", "ghost"]), &table());
    assert_eq!(
        state.uses_left,
        BTreeMap::from([(sid("fire"), 10), (sid("mend"), 4)])
    );
    let mut state = SpellState {
        uses_left: BTreeMap::from([(sid("mend"), 2)]),
    };
    assert_eq!(state.uses_left(&sid("mend")), 2);
    assert_eq!(state.uses_left(&sid("fire")), 0);
    assert_eq!(state.spend(&sid("mend")), Some(1));
    assert_eq!(state.spend(&sid("mend")), Some(0));
    let before = state.clone();
    assert_eq!(state.spend(&sid("mend")), None);
    assert_eq!(state.spend(&sid("fire")), None);
    assert_eq!(state, before);
}

#[test]
fn known_spells_follow_class_levels_and_the_character_level() {
    let cs = classes();
    let mut u = mage(3);
    assert_eq!(u.known_spells(&cs), set(&["fire"]));
    // Class level 5 in mage: force too. The character level doesn't count.
    u.class_records.insert(ClassId("mage".into()), record(5));
    assert_eq!(u.known_spells(&cs), set(&["fire", "force"]));
    u.class_records.insert(ClassId("mage".into()), record(4));
    assert_eq!(u.known_spells(&cs), set(&["fire"]));
    // Another class counts only once mastered (class level 10).
    u.class_records.insert(ClassId("cleric".into()), record(9));
    assert_eq!(u.known_spells(&cs), set(&["fire"]));
    u.class_records.insert(ClassId("cleric".into()), record(10));
    assert_eq!(u.known_spells(&cs), set(&["fire", "heal", "mend"]));
    // A record of a class the table doesn't have is skipped.
    u.class_records.insert(ClassId("ghost".into()), record(10));
    assert_eq!(u.known_spells(&cs), set(&["fire", "heal", "mend"]));
    // Personal spells follow the character level (3).
    u.personal_spells = vec![(3, sid("zap")), (4, sid("glow"))];
    assert_eq!(u.known_spells(&cs), set(&["fire", "heal", "mend", "zap"]));
    u.level = 4;
    assert_eq!(
        u.known_spells(&cs),
        set(&["fire", "heal", "mend", "zap", "glow"])
    );
}

fn changes(gained: &[&str], lost: &[&str]) -> SpellChanges {
    SpellChanges {
        gained: gained.iter().map(|s| sid(s)).collect(),
        lost: lost.iter().map(|s| sid(s)).collect(),
    }
}

/// Nick (2026-09-27): class spells are kept after a class change only if the
/// class was mastered.
#[test]
fn class_spells_stay_after_a_class_change_only_if_mastered() {
    let cs = classes();
    let mut u = mage(1);
    u.personal_spells = vec![(1, sid("zap"))];
    u.class_records.insert(ClassId("mage".into()), record(5));
    assert_eq!(
        u.refresh_spells(&cs),
        changes(&["fire", "force", "zap"], &[])
    );
    u.spells = SpellState::full(&u.learned, &table());
    u.loadout.equipped = Some(crate::item::Equipped::Spell(sid("fire")));
    // Reclass to cleric with mage unmastered: mage's spells stay behind
    // (their uses too, and fire is unequipped); the personal spell stays.
    u.class = ClassId("cleric".into());
    u.class_records
        .insert(ClassId("cleric".into()), ClassRecord::UNLOCKED);
    assert_eq!(
        u.refresh_spells(&cs),
        changes(&["heal"], &["fire", "force"])
    );
    assert_eq!(u.learned, set(&["heal", "zap"]));
    assert_eq!(u.spells.uses_left(&sid("fire")), 0);
    assert!(!u.spells.uses_left.contains_key(&sid("force")));
    assert_eq!(u.loadout.equipped, None);
    // Back to mage: its saved class level 5 brings both back.
    u.class = ClassId("mage".into());
    assert_eq!(
        u.refresh_spells(&cs),
        changes(&["fire", "force"], &["heal"])
    );
    // Mastered, then promoted to sage: everything from mage is kept.
    u.class_records.insert(ClassId("mage".into()), record(10));
    assert_eq!(u.refresh_spells(&cs), changes(&[], &[]));
    u.class = ClassId("sage".into());
    u.class_records
        .insert(ClassId("sage".into()), ClassRecord::UNLOCKED);
    assert_eq!(u.refresh_spells(&cs), changes(&["mend"], &[]));
    assert_eq!(u.learned, set(&["fire", "force", "mend", "zap"]));
    // Reclass out of the unmastered sage: mend goes (cleric's mend needs
    // class level 3); an equipped spell that is kept stays equipped.
    u.loadout.equipped = Some(crate::item::Equipped::Spell(sid("force")));
    u.class = ClassId("cleric".into());
    assert_eq!(u.refresh_spells(&cs), changes(&["heal"], &["mend"]));
    assert_eq!(
        u.loadout.equipped,
        Some(crate::item::Equipped::Spell(sid("force")))
    );
}

#[test]
fn units_are_created_knowing_their_spells() {
    let cs = classes();
    // A generic unit knows its class's class-level-1 spells.
    let generic = Unit::generic(
        UnitId(1),
        &ClassId("cleric".into()),
        &cs,
        20,
        Faction::Enemy,
        Pos::new(0, 0),
    )
    .unwrap();
    assert_eq!(generic.learned, set(&["heal"]));
    assert!(generic.personal_spells.is_empty());
    assert!(generic.spells.uses_left.is_empty());
    // A character adds its personal spells at its level.
    let def = crate::unit::CharacterDef {
        id: crate::unit::CharacterId("ada".into()),
        name: "Ada".into(),
        class: ClassId("mage".into()),
        level: 5,
        is_lord: false,
        talent: crate::stats::StatKind::Mag,
        base: Stats::default(),
        weapon_ranks: BTreeMap::new(),
        personal_spells: vec![(1, sid("spark")), (6, sid("storm"))],
        map_label: None,
        loadout: crate::item::LoadoutDef::default(),
    };
    let ada = Unit::from_character(UnitId(2), &def, &cs, Faction::Player, Pos::new(0, 0)).unwrap();
    assert_eq!(ada.learned, set(&["fire", "spark"]));
    assert_eq!(ada.personal_spells, def.personal_spells);
}

#[test]
fn castable_attack_needs_a_learned_attack_spell_with_uses() {
    let t = table();
    let mut u = mage(1);
    u.learned = set(&["fire", "heal"]);
    u.spells = SpellState::full(&u.learned, &t);
    assert_eq!(u.castable_attack(&sid("fire"), &t), t.get(&sid("fire")));
    // A heal, a spell not learned, one missing from the table.
    assert_eq!(u.castable_attack(&sid("heal"), &t), None);
    assert_eq!(u.castable_attack(&sid("force"), &t), None);
    u.learned.insert(sid("ghost"));
    assert_eq!(u.castable_attack(&sid("ghost"), &t), None);
    // No uses left.
    u.spells.uses_left.insert(sid("fire"), 0);
    assert_eq!(u.castable_attack(&sid("fire"), &t), None);
    u.spells.uses_left.insert(sid("fire"), 1);
    assert!(u.castable_attack(&sid("fire"), &t).is_some());
    // Uses without having learned it don't count.
    u.learned.remove(&sid("fire"));
    assert_eq!(u.castable_attack(&sid("fire"), &t), None);
}

#[test]
fn first_attack_spell_skips_heals_and_unknown_spells() {
    let t = table();
    let mut u = mage(1);
    u.learned = set(&["heal", "force", "fire"]);
    assert_eq!(u.first_attack_spell(&t), Some(sid("fire")));
    u.learned = set(&["aaa", "heal", "mend"]);
    assert_eq!(u.first_attack_spell(&t), None);
    u.learned = set(&["aaa", "force"]);
    assert_eq!(u.first_attack_spell(&t), Some(sid("force")));
}

proptest! {
    /// Spending never goes below 0 and never above the spell's uses; each
    /// successful spend lowers the count by exactly 1.
    #[test]
    fn uses_stay_within_zero_and_max(spends in prop::collection::vec(0usize..3, 0..40)) {
        let t = table();
        let names = ["fire", "mend", "ghost"];
        let mut state = SpellState::full(&set(&names), &t);
        for i in spends {
            let spell = sid(names[i]);
            let before = state.uses_left(&spell);
            let spent = state.spend(&spell);
            let after = state.uses_left(&spell);
            if let Some(left) = spent {
                prop_assert_eq!(left, after);
                prop_assert_eq!(before, after + 1);
            } else {
                prop_assert_eq!(before, 0);
                prop_assert_eq!(after, 0);
            }
            for (id, n) in &state.uses_left {
                prop_assert!(*n <= t.get(id).map_or(0, |d| d.uses));
            }
        }
    }
}
