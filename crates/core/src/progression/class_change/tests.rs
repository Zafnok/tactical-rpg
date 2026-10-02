//! Tests of promotion and reclass. The classes' base stats, Mov, weapons
//! and armour are `progression.md`'s.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use super::*;
use crate::class::{ArmourWeight, ClassLevel, Tier, UnitTags, WeaponProficiency};
use crate::geom::Pos;
use crate::item::{
    ArmourDef, ConsumableDef, ConsumableEffect, Equipped, ItemDef, SealDef, WeaponDef,
    WeaponInstance, WeaponRules,
};
use crate::skill::{SkillDef, SkillId, SkillKind, SkillTable};
use crate::spell::SpellId;
use crate::stats::{Growths, StatValue};
use crate::terrain::MovementTypeId;
use crate::unit::{Faction, UnitId};
use crate::weapon::{WeaponKind, WeaponRank};

use ArmourWeight::{Heavy, Light, Medium};
use ClassChangeError as Refused;
use WeaponKind::{Axe, Spear, Sword};
use WeaponRank::{A, B, C, D, E};

fn cid(id: &str) -> ClassId {
    ClassId(id.into())
}

fn item(id: &str) -> ItemId {
    ItemId::new(id)
}

struct Spec {
    id: &'static str,
    tier: Tier,
    mov: StatValue,
    base: [StatValue; 7],
    weapons: &'static [(WeaponKind, WeaponRank, WeaponRank)],
    armour: &'static [ArmourWeight],
    promotes_to: &'static [&'static str],
    spells: &'static [(ClassLevel, &'static str)],
}

fn def(s: &Spec) -> ClassDef {
    ClassDef {
        id: cid(s.id),
        name: s.id.into(),
        tier: s.tier,
        movement_type: MovementTypeId(0),
        move_points: s.mov,
        base: Stats::from_growable(s.base, s.mov),
        growths: Growths([50; 7]),
        weapons: s
            .weapons
            .iter()
            .map(|&(kind, start, max)| WeaponProficiency { kind, start, max })
            .collect(),
        armour: s.armour.to_vec(),
        tags: UnitTags::default(),
        promotes_to: s.promotes_to.iter().map(|c| cid(c)).collect(),
        active: Some(SkillId::new(&format!("{}_active", s.id))),
        passives: vec![],
        enemy_only: false,
        lord_only: false,
        weapon_slots: 3,
        spells: s
            .spells
            .iter()
            .map(|&(level, spell)| (level, SpellId::new(spell)))
            .collect(),
        affinities: vec![],
    }
}

const SPECS: [Spec; 10] = [
    Spec {
        id: "guard",
        tier: 1,
        mov: 4,
        base: [20, 7, 0, 4, 2, 9, 0],
        weapons: &[(Spear, D, C)],
        armour: &[Medium, Heavy],
        promotes_to: &["bulwark", "iron_rider"],
        spells: &[],
    },
    Spec {
        id: "rider",
        tier: 1,
        mov: 7,
        base: [20, 6, 0, 5, 5, 5, 1],
        weapons: &[(Sword, E, C), (Spear, D, C)],
        armour: &[Light, Medium],
        // The Lancer isn't in this table: an unknown class is never offered.
        promotes_to: &["iron_rider", "lancer"],
        spells: &[],
    },
    Spec {
        id: "bulwark",
        tier: 2,
        mov: 4,
        base: [28, 11, 0, 6, 4, 14, 2],
        weapons: &[(Spear, C, A), (Axe, D, B)],
        armour: &[Medium, Heavy],
        promotes_to: &[],
        spells: &[],
    },
    Spec {
        id: "iron_rider",
        tier: 2,
        mov: 6,
        base: [27, 10, 0, 6, 6, 11, 2],
        weapons: &[(Spear, C, A), (Axe, D, B), (Sword, D, B)],
        armour: &[Medium, Heavy],
        promotes_to: &[],
        spells: &[],
    },
    Spec {
        id: "mage",
        tier: 1,
        mov: 5,
        base: [16, 1, 6, 5, 5, 1, 5],
        weapons: &[(Sword, E, D)],
        armour: &[Light],
        promotes_to: &["sorcerer"],
        spells: &[(1, "fire"), (5, "frost")],
    },
    Spec {
        id: "sorcerer",
        tier: 2,
        mov: 5,
        base: [20, 2, 10, 7, 7, 2, 8],
        weapons: &[(Sword, D, C)],
        armour: &[Light],
        promotes_to: &["archmage"],
        spells: &[(1, "force")],
    },
    Spec {
        id: "archmage",
        tier: 3,
        mov: 5,
        base: [25, 3, 14, 9, 9, 3, 11],
        weapons: &[],
        armour: &[Light],
        promotes_to: &[],
        spells: &[(1, "fire"), (1, "frost"), (1, "force")],
    },
    Spec {
        id: "brigand",
        tier: 1,
        mov: 5,
        base: [20, 6, 0, 2, 4, 3, 0],
        weapons: &[(Axe, D, C)],
        armour: &[Light],
        promotes_to: &[],
        spells: &[],
    },
    Spec {
        id: "exile",
        tier: 1,
        mov: 5,
        base: [19, 6, 1, 7, 7, 4, 3],
        weapons: &[(Sword, D, C)],
        armour: &[Light, Medium],
        promotes_to: &["blade_heir"],
        spells: &[],
    },
    Spec {
        id: "blade_heir",
        tier: 2,
        mov: 6,
        base: [25, 9, 1, 11, 12, 6, 5],
        weapons: &[(Sword, C, A)],
        armour: &[Light],
        promotes_to: &[],
        spells: &[],
    },
];

fn classes() -> ClassTable {
    let mut classes: BTreeMap<ClassId, ClassDef> =
        SPECS.iter().map(def).map(|c| (c.id.clone(), c)).collect();
    let mut flag = |id: &str, set: fn(&mut ClassDef)| {
        if let Some(c) = classes.get_mut(&cid(id)) {
            set(c);
        }
    };
    flag("brigand", |c| c.enemy_only = true);
    flag("exile", |c| c.lord_only = true);
    flag("blade_heir", |c| c.lord_only = true);
    flag("archmage", |c| c.weapon_slots = 0);
    ClassTable {
        classes,
        min_gains: vec![2, 2, 3],
        cp_per_class_level: vec![10, 17, 25],
        class_level_cap: 10,
        level_cap: 99,
        hard_ceilings: Stats::from_growable([80, 50, 50, 50, 50, 50, 50], 15),
    }
}

fn items() -> ItemTable {
    let weapon = |name: &str, kind| {
        ItemDef::Weapon(WeaponDef {
            name: name.into(),
            kind,
            rank: E,
            might: 5,
            hit: 90,
            crit: 0,
            weight: 2,
            min_range: 1,
            max_range: 1,
            damage_type: crate::combat::DamageType::Physical,
            durability: 20,
            effective: vec![],
            arts: vec![],
            price: 400,
        })
    };
    let armour = |name: &str, weight_class| {
        ItemDef::Armour(ArmourDef {
            name: name.into(),
            weight_class,
            bonus: Stats::default(),
            weight: 0,
            price: 300,
        })
    };
    let seal = |name: &str, kind| {
        ItemDef::Seal(SealDef {
            name: name.into(),
            kind,
        })
    };
    let entries = [
        ("iron_spear", weapon("Iron Spear", Spear)),
        ("iron_sword", weapon("Iron Sword", Sword)),
        ("chain_mail", armour("Chain Mail", Medium)),
        ("leather_vest", armour("Leather Vest", Light)),
        (
            "potion",
            ItemDef::Consumable(ConsumableDef {
                name: "Potion".into(),
                effect: ConsumableEffect::Heal(10),
                price: 300,
            }),
        ),
        ("tier_2_seal", seal("Tier 2 Seal", SealKind::Tier(2))),
        ("tier_3_seal", seal("Tier 3 Seal", SealKind::Tier(3))),
        ("reclass_seal", seal("Reclass Seal", SealKind::Reclass)),
    ];
    ItemTable {
        items: entries.into_iter().map(|(k, v)| (item(k), v)).collect(),
        traits: BTreeMap::new(),
        rules: WeaponRules::default(),
    }
}

/// Every class's active, as a skill with nothing in it.
fn skills() -> SkillTable {
    SkillTable {
        skills: SPECS
            .iter()
            .map(|s| SkillDef {
                id: SkillId::new(&format!("{}_active", s.id)),
                name: s.id.into(),
                family: s.id.into(),
                rank: 1,
                kind: SkillKind::Passive(vec![]),
            })
            .map(|s| (s.id.clone(), s))
            .collect(),
    }
}

struct World {
    classes: ClassTable,
    items: ItemTable,
    spells: SpellTable,
}

impl World {
    fn new() -> World {
        World {
            classes: classes(),
            items: items(),
            spells: SpellTable::default(),
        }
    }

    fn tables(&self) -> ChangeTables<'_> {
        ChangeTables {
            classes: &self.classes,
            items: &self.items,
            spells: &self.spells,
        }
    }

    /// A level-7 player unit of `class` with 42 EXP and these stats.
    fn unit(&self, class: &str, stats: [StatValue; 7]) -> Unit {
        let mut u = Unit::generic(
            UnitId(1),
            &cid(class),
            &self.classes,
            7,
            Faction::Player,
            Pos::new(0, 0),
        )
        .unwrap();
        u.exp = 42;
        u.stats = Stats::from_growable(stats, u.stats.mov);
        u.hp = u.stats.hp;
        u
    }

    /// A lord in `class` (generic units can't be in a lord-only class).
    fn lord(&self, class: &str) -> Unit {
        let mut u = self.unit("guard", [25, 9, 2, 9, 9, 6, 4]);
        u.is_lord = true;
        u.class = cid(class);
        u.class_records = BTreeMap::from([(cid(class), ClassRecord::UNLOCKED)]);
        u
    }
}

/// Sets `unit`'s record in `class` to class level `level`.
fn at_class_level(unit: &mut Unit, class: &str, level: ClassLevel) {
    unit.class_records.insert(
        cid(class),
        ClassRecord {
            class_level: level,
            class_points: u32::from(level - 1) * 10,
        },
    );
}

fn master(unit: &mut Unit, class: &str) {
    at_class_level(unit, class, 10);
}

fn stock(seals: &[&str]) -> Stock {
    let mut stock = Stock::default();
    for s in seals {
        stock.add(item(s));
    }
    stock
}

fn growable(unit: &Unit) -> [StatValue; 7] {
    StatKind::GROWABLE.map(|k| unit.stats.get(k))
}

fn spells(unit: &Unit) -> Vec<&str> {
    unit.learned.iter().map(|s| s.0.as_str()).collect()
}

fn usable(unit: &Unit, w: &World) -> Vec<String> {
    unit.usable_skills(&w.classes, &skills())
        .iter()
        .map(|s| s.id.0.clone())
        .collect()
}

const GUARD_STATS: [StatValue; 7] = [24, 9, 1, 6, 4, 12, 2];

/// Hand-worked (`progression.md`, shared promotions): Iron Rider base
/// `27 10 0 6 6 11 2` less Guard base `20 7 0 4 2 9 0` = `+7 +3 0 +2 +4 +2 +2`.
#[test]
fn a_guard_promotes_to_iron_rider_with_the_base_stat_gap() {
    let w = World::new();
    let mut u = w.unit("guard", GUARD_STATS);
    u.hp = 10;
    master(&mut u, "guard");
    u.weapon_ranks.insert(Axe, B);
    let mut s = stock(&["tier_2_seal", "tier_2_seal", "tier_3_seal"]);
    let events = promote(&mut u, &cid("iron_rider"), &mut s, &w.tables());
    let gains = StatGains([7, 3, 0, 2, 4, 2, 2]);
    assert_eq!(
        events,
        Ok(vec![Event::Promoted {
            unit: UnitId(1),
            from: cid("guard"),
            to: cid("iron_rider"),
            gains,
        }])
    );
    assert_eq!(growable(&u), [31, 12, 1, 8, 8, 14, 4]);
    // Current HP rose by the HP bonus; Mov is the new class's.
    assert_eq!((u.hp, u.stats.mov), (17, 6));
    // The character level and EXP don't reset.
    assert_eq!((u.level, u.exp), (7, 42));
    assert_eq!(u.class, cid("iron_rider"));
    assert_eq!(
        u.class_records,
        BTreeMap::from([
            (
                cid("guard"),
                ClassRecord {
                    class_level: 10,
                    class_points: 90
                }
            ),
            (cid("iron_rider"), ClassRecord::UNLOCKED),
        ])
    );
    // Ranks rise to the start ranks, never fall.
    assert_eq!(
        u.weapon_ranks,
        BTreeMap::from([(Sword, D), (Spear, C), (Axe, B)])
    );
    // One seal of the target's tier is used up.
    assert_eq!(s, stock(&["tier_2_seal", "tier_3_seal"]));
    // The old class's active is kept (mastered), the new one's is gained.
    assert_eq!(usable(&u, &w), ["guard_active", "iron_rider_active"]);
}

/// The same class from the Rider (base `20 6 0 5 5 5 1`):
/// `+7 +4 0 +1 +1 +6 +1`.
#[test]
fn a_shared_promotion_gives_each_parent_its_own_bonus() {
    let w = World::new();
    let mut u = w.unit("rider", GUARD_STATS);
    master(&mut u, "rider");
    let mut s = stock(&["tier_2_seal"]);
    let events = promote(&mut u, &cid("iron_rider"), &mut s, &w.tables()).unwrap();
    assert_eq!(
        events[0],
        Event::Promoted {
            unit: UnitId(1),
            from: cid("rider"),
            to: cid("iron_rider"),
            gains: StatGains([7, 4, 0, 1, 1, 6, 1]),
        }
    );
    assert_eq!(growable(&u), [31, 13, 1, 7, 5, 18, 3]);
    assert_eq!(s, Stock::default());
}

#[test]
fn a_lower_base_gives_nothing_and_the_hard_ceiling_cuts_the_bonus() {
    let w = World::new();
    // Bulwark base 28 11 0 6 4 14 2 from Rider 20 6 0 5 5 5 1: Spd 4 − 5 < 0.
    let rider = w.classes.get(&cid("rider")).unwrap();
    let bulwark = w.classes.get(&cid("bulwark")).unwrap();
    let ceilings = &w.classes.hard_ceilings;
    let u = w.unit("rider", [78, 49, 50, 6, 4, 45, 2]);
    assert_eq!(
        promotion_gains(&u, rider, bulwark, ceilings),
        StatGains([2, 1, 0, 1, 0, 5, 1])
    );
    // A stat already past its ceiling gains nothing (and isn't lowered).
    let over = w.unit("rider", [90, 5, 0, 6, 4, 50, 2]);
    assert_eq!(
        promotion_gains(&over, rider, bulwark, ceilings),
        StatGains([0, 5, 0, 1, 0, 0, 1])
    );
    // Applied: HP 78 → 80, and current HP with it.
    let mut u = w.unit("guard", [78, 49, 50, 6, 4, 45, 2]);
    u.hp = 70;
    master(&mut u, "guard");
    let mut s = stock(&["tier_2_seal"]);
    promote(&mut u, &cid("bulwark"), &mut s, &w.tables()).unwrap();
    assert_eq!(growable(&u), [80, 50, 50, 8, 6, 50, 4]);
    assert_eq!(u.hp, 72);
}

/// [`promote`] or [`reclass`].
type Change =
    fn(&mut Unit, &ClassId, &mut Stock, &ChangeTables<'_>) -> Result<Vec<Event>, ClassChangeError>;

/// Runs `change` on copies and checks it fails with `error` and changes
/// neither the unit nor the stock.
fn refused(
    change: Change,
    (unit, stock, w): (&Unit, &Stock, &World),
    target: &str,
    error: &ClassChangeError,
) {
    let (mut u, mut s) = (unit.clone(), stock.clone());
    assert_eq!(
        change(&mut u, &cid(target), &mut s, &w.tables()).as_ref(),
        Err(error),
        "{target}"
    );
    assert_eq!((&u, &s), (unit, stock), "{target}");
}

#[test]
fn a_refused_promotion_changes_nothing() {
    let mut w = World::new();
    // A Guard table whose Guard could, wrongly, promote into an enemy-only
    // and a lord-only class.
    if let Some(guard) = w.classes.classes.get_mut(&cid("guard")) {
        guard
            .promotes_to
            .extend([cid("brigand"), cid("blade_heir")]);
    }
    let seals = stock(&["tier_2_seal", "reclass_seal"]);
    let fresh = w.unit("guard", GUARD_STATS);
    let mut mastered = fresh.clone();
    master(&mut mastered, "guard");
    // Not mastered: class level 9 isn't enough.
    let mut nine = fresh.clone();
    at_class_level(&mut nine, "guard", 9);
    for unit in [&fresh, &nine] {
        refused(
            promote,
            (unit, &seals, &w),
            "iron_rider",
            &Refused::NotMastered(cid("guard")),
        );
    }
    // Not one of its promotions (a tier-2 class of another line, its own
    // class, a tier-1 class).
    for target in ["sorcerer", "guard", "mage"] {
        let error = Refused::NotAPromotion {
            from: cid("guard"),
            to: cid(target),
        };
        refused(promote, (&mastered, &seals, &w), target, &error);
    }
    // No seal, or only seals of another kind or tier.
    for seals in [&[][..], &["tier_3_seal", "reclass_seal", "potion"]] {
        refused(
            promote,
            (&mastered, &stock(seals), &w),
            "iron_rider",
            &Refused::NoSeal(SealKind::Tier(2)),
        );
    }
    refused(
        promote,
        (&mastered, &seals, &w),
        "brigand",
        &Refused::EnemyOnly(cid("brigand")),
    );
    refused(
        promote,
        (&mastered, &stock(&["tier_2_seal"]), &w),
        "blade_heir",
        &Refused::LordOnly(cid("blade_heir")),
    );
    refused(
        promote,
        (&mastered, &seals, &w),
        "nope",
        &Refused::UnknownClass(cid("nope")),
    );
    let mut lost = mastered.clone();
    lost.class = cid("gone");
    refused(
        promote,
        (&lost, &seals, &w),
        "iron_rider",
        &Refused::UnknownClass(cid("gone")),
    );
}

#[test]
fn the_lord_promotes_in_its_own_line() {
    let w = World::new();
    let mut lord = w.lord("exile");
    master(&mut lord, "exile");
    let before = growable(&lord);
    let mut s = stock(&["tier_2_seal"]);
    assert!(promote(&mut lord, &cid("blade_heir"), &mut s, &w.tables()).is_ok());
    assert_eq!(lord.class, cid("blade_heir"));
    // Blade Heir 25 9 1 11 12 6 5 less Exile 19 6 1 7 7 4 3.
    let gained: Vec<StatValue> = growable(&lord)
        .iter()
        .zip(before)
        .map(|(a, b)| a - b)
        .collect();
    assert_eq!(gained, [6, 3, 0, 4, 5, 2, 2]);
}

#[test]
fn promotion_teaches_the_spells_of_the_new_class_and_keeps_the_old() {
    let w = World::new();
    let mut u = w.unit("mage", [20, 2, 9, 6, 6, 2, 7]);
    master(&mut u, "mage");
    u.refresh_spells(&w.classes);
    assert_eq!(spells(&u), ["fire", "frost"]);
    let mut s = stock(&["tier_2_seal"]);
    let events = promote(&mut u, &cid("sorcerer"), &mut s, &w.tables()).unwrap();
    assert_eq!(
        events[1..],
        [Event::SpellLearned {
            unit: UnitId(1),
            spell: SpellId::new("force"),
        }]
    );
    assert_eq!(spells(&u), ["fire", "force", "frost"]);
    assert_eq!(usable(&u, &w), ["mage_active", "sorcerer_active"]);
}

fn copy(id: &str) -> WeaponInstance {
    WeaponInstance {
        def: item(id),
        durability_left: 20,
    }
}

#[test]
fn a_class_without_weapon_slots_sends_the_weapons_to_the_stock() {
    let w = World::new();
    let mut u = w.unit("sorcerer", [24, 3, 12, 8, 8, 3, 9]);
    master(&mut u, "sorcerer");
    u.loadout.weapons[0] = Some(copy("iron_sword"));
    u.loadout.weapons[2] = Some(copy("iron_spear"));
    u.loadout.equipped = Some(Equipped::Weapon(0));
    u.loadout.armour = Some(item("leather_vest"));
    let mut s = stock(&["tier_3_seal"]);
    let events = promote(&mut u, &cid("archmage"), &mut s, &w.tables()).unwrap();
    let learned = |spell| Event::SpellLearned {
        unit: UnitId(1),
        spell: SpellId::new(spell),
    };
    let stowed = |id| Event::ItemStowed {
        unit: UnitId(1),
        item: item(id),
    };
    assert_eq!(
        events[1..],
        [
            learned("fire"),
            learned("frost"),
            stowed("iron_sword"),
            stowed("iron_spear")
        ]
    );
    assert_eq!(u.loadout.weapon_count(), 0);
    assert_eq!(s.weapons, [copy("iron_sword"), copy("iron_spear")]);
    // Nothing left to equip (no attack spell in this test's spell table);
    // armour the class wears stays on.
    assert_eq!(u.loadout.equipped, None);
    assert_eq!(u.loadout.armour, Some(item("leather_vest")));
    assert_eq!(s.items, BTreeMap::new());
}

/// A Guard at class level 6 with a spear (equipped), a sword and chain mail.
fn armed_guard(w: &World) -> Unit {
    let mut u = w.unit("guard", GUARD_STATS);
    at_class_level(&mut u, "guard", 6);
    u.loadout.weapons[0] = Some(copy("iron_spear"));
    u.loadout.weapons[1] = Some(copy("iron_sword"));
    u.loadout.equipped = Some(Equipped::Weapon(0));
    u.loadout.armour = Some(item("chain_mail"));
    u.learned_skills = BTreeSet::from([SkillId::new("some_passive")]);
    u
}

#[test]
fn reclass_keeps_stats_and_progress_and_always_costs_a_seal() {
    let w = World::new();
    let mut u = armed_guard(&w);
    u.hp = 11;
    let mut s = stock(&["reclass_seal", "reclass_seal", "tier_2_seal"]);
    let events = reclass(&mut u, &cid("mage"), &mut s, &w.tables());
    assert_eq!(
        events,
        Ok(vec![
            Event::Reclassed {
                unit: UnitId(1),
                from: cid("guard"),
                to: cid("mage"),
            },
            Event::SpellLearned {
                unit: UnitId(1),
                spell: SpellId::new("fire"),
            },
            // The Mage wears light armour only.
            Event::ItemStowed {
                unit: UnitId(1),
                item: item("chain_mail"),
            },
        ])
    );
    // Stats, HP, level and EXP never change; Mov is the class's.
    assert_eq!(growable(&u), GUARD_STATS);
    assert_eq!((u.hp, u.level, u.exp, u.stats.mov), (11, 7, 42, 5));
    assert_eq!(u.class, cid("mage"));
    assert_eq!(u.class_records[&cid("mage")], ClassRecord::UNLOCKED);
    assert_eq!(u.class_records[&cid("guard")].class_level, 6);
    assert_eq!(s.count(&item("reclass_seal")), 1);
    assert_eq!(s.count(&item("chain_mail")), 1);
    assert_eq!(u.loadout.armour, None);
    // The Mage can't wield the spear it had equipped: the sword replaces it.
    // The spear stays in its slot, and its rank is kept for later.
    assert_eq!(u.loadout.equipped, Some(Equipped::Weapon(1)));
    assert_eq!(u.loadout.weapon_count(), 2);
    assert_eq!(u.weapon_ranks, BTreeMap::from([(Sword, E), (Spear, D)]));
    // The unmastered Guard's active stays behind; passives are kept.
    assert_eq!(usable(&u, &w), ["mage_active"]);
    assert!(u.learned_skills.contains(&SkillId::new("some_passive")));

    // Back to the Guard: another seal, its class level 6 is still there,
    // and the unmastered Mage's spell and active are left behind.
    let before = u.clone();
    let events = reclass(&mut u, &cid("guard"), &mut s, &w.tables());
    assert_eq!(
        events,
        Ok(vec![Event::Reclassed {
            unit: UnitId(1),
            from: cid("mage"),
            to: cid("guard"),
        }])
    );
    assert_eq!(u.class_records, before.class_records);
    assert_eq!(
        u.class_records[&cid("guard")],
        ClassRecord {
            class_level: 6,
            class_points: 50
        }
    );
    assert_eq!(spells(&u), Vec::<&str>::new());
    assert_eq!(usable(&u, &w), ["guard_active"]);
    assert_eq!(growable(&u), GUARD_STATS);
    assert_eq!(s.count(&item("reclass_seal")), 0);
    // The sword it had equipped can't be wielded by a Guard: the spear is.
    assert_eq!(u.loadout.equipped, Some(Equipped::Weapon(0)));
    // No seal left: the next change is refused.
    refused(
        reclass,
        (&u, &s, &w),
        "mage",
        &ClassChangeError::NoSeal(SealKind::Reclass),
    );
}

#[test]
fn spells_of_a_class_come_back_with_its_saved_class_level() {
    let w = World::new();
    let mut u = w.unit("mage", [20, 2, 9, 6, 6, 2, 7]);
    at_class_level(&mut u, "mage", 5);
    u.refresh_spells(&w.classes);
    u.loadout.equipped = Some(Equipped::Spell(SpellId::new("fire")));
    let mut s = stock(&["reclass_seal", "reclass_seal"]);
    reclass(&mut u, &cid("guard"), &mut s, &w.tables()).unwrap();
    // `Unit::refresh_spells` dropped them, and unequipped the lost spell.
    assert_eq!(spells(&u), Vec::<&str>::new());
    assert_eq!(u.loadout.equipped, None);
    let events = reclass(&mut u, &cid("mage"), &mut s, &w.tables()).unwrap();
    let learned: Vec<&Event> = events[1..].iter().collect();
    assert_eq!(
        learned,
        [
            &Event::SpellLearned {
                unit: UnitId(1),
                spell: SpellId::new("fire")
            },
            &Event::SpellLearned {
                unit: UnitId(1),
                spell: SpellId::new("frost")
            }
        ]
    );
    assert_eq!(u.class_records[&cid("mage")].class_level, 5);
    // A mastered class's spells and active are kept on leaving it.
    master(&mut u, "mage");
    s.add(item("reclass_seal"));
    reclass(&mut u, &cid("rider"), &mut s, &w.tables()).unwrap();
    assert_eq!(spells(&u), ["fire", "frost"]);
    assert_eq!(usable(&u, &w), ["mage_active", "rider_active"]);
    // A unit with nothing equipped ends with what its new class can wield.
    let mut v = w.unit("mage", [20, 2, 9, 6, 6, 2, 7]);
    v.loadout.weapons[1] = Some(copy("iron_sword"));
    assert_eq!(v.loadout.equipped, None);
    s.add(item("reclass_seal"));
    reclass(&mut v, &cid("rider"), &mut s, &w.tables()).unwrap();
    assert_eq!(v.loadout.equipped, Some(Equipped::Weapon(1)));
}

#[test]
fn a_refused_reclass_changes_nothing() {
    let w = World::new();
    let seals = stock(&["reclass_seal", "tier_2_seal"]);
    let guard = armed_guard(&w);
    refused(
        reclass,
        (&guard, &seals, &w),
        "guard",
        &Refused::SameClass(cid("guard")),
    );
    refused(
        reclass,
        (&guard, &seals, &w),
        "brigand",
        &Refused::EnemyOnly(cid("brigand")),
    );
    refused(
        reclass,
        (&guard, &seals, &w),
        "exile",
        &Refused::LordOnly(cid("exile")),
    );
    // A tier-2 class whose prerequisite isn't mastered (class level 6), or
    // that no class it mastered promotes to; a tier-3 class.
    let mut other = guard.clone();
    master(&mut other, "guard");
    for (unit, target) in [
        (&guard, "iron_rider"),
        (&other, "sorcerer"),
        (&other, "archmage"),
    ] {
        refused(
            reclass,
            (unit, &seals, &w),
            target,
            &Refused::NotReachable(cid(target)),
        );
    }
    refused(
        reclass,
        (&guard, &seals, &w),
        "nope",
        &Refused::UnknownClass(cid("nope")),
    );
    // Only tier seals: a promotion seal isn't a Reclass Seal.
    refused(
        reclass,
        (&guard, &stock(&["tier_2_seal", "tier_3_seal"]), &w),
        "mage",
        &Refused::NoSeal(SealKind::Reclass),
    );
    let mut lost = guard.clone();
    lost.class = cid("gone");
    refused(
        reclass,
        (&lost, &seals, &w),
        "mage",
        &Refused::UnknownClass(cid("gone")),
    );
}

fn ids(classes: &[&ClassDef]) -> Vec<String> {
    classes.iter().map(|c| c.id.0.clone()).collect()
}

#[test]
fn a_seal_reaches_tier_one_mastered_prerequisites_and_unlocked_classes() {
    let w = World::new();
    let mut u = w.unit("guard", GUARD_STATS);
    // Every tier-1 class but its own, the enemy-only and the lord-only.
    assert_eq!(ids(&reclass_targets(&u, &w.classes)), ["mage", "rider"]);
    // A mastered class opens what it promotes to, by tier then id.
    master(&mut u, "guard");
    assert_eq!(
        ids(&reclass_targets(&u, &w.classes)),
        ["mage", "rider", "bulwark", "iron_rider"]
    );
    // No bonus that way: a reclass never changes stats.
    let mut s = stock(&["reclass_seal", "reclass_seal", "reclass_seal"]);
    reclass(&mut u, &cid("bulwark"), &mut s, &w.tables()).unwrap();
    assert_eq!(growable(&u), GUARD_STATS);
    // A class already unlocked stays reachable without its prerequisite.
    let mut v = w.unit("mage", GUARD_STATS);
    v.class_records
        .insert(cid("archmage"), ClassRecord::UNLOCKED);
    assert_eq!(
        ids(&reclass_targets(&v, &w.classes)),
        ["guard", "rider", "archmage"]
    );
    assert!(reclass(&mut v, &cid("archmage"), &mut s, &w.tables()).is_ok());
    // The lord gets its own line too, and comes back to it like anyone.
    let mut lord = w.lord("guard");
    assert_eq!(
        ids(&reclass_targets(&lord, &w.classes)),
        ["exile", "mage", "rider"]
    );
    assert!(reclass(&mut lord, &cid("exile"), &mut s, &w.tables()).is_ok());
    assert_eq!(lord.class, cid("exile"));
    // A unit of a class missing from the table is offered tier 1.
    let mut gone = w.unit("guard", GUARD_STATS);
    gone.class = cid("gone");
    assert_eq!(reclass_targets(&gone, &w.classes).len(), 3);
}

#[test]
fn promotions_offered_are_the_classes_the_unit_may_enter() {
    let mut w = World::new();
    let u = w.unit("guard", GUARD_STATS);
    assert_eq!(
        ids(&promotion_targets(&u, &w.classes)),
        ["bulwark", "iron_rider"]
    );
    // Unknown classes are skipped.
    let rider = w.unit("rider", GUARD_STATS);
    assert_eq!(ids(&promotion_targets(&rider, &w.classes)), ["iron_rider"]);
    assert_eq!(
        ids(&promotion_targets(&w.lord("exile"), &w.classes)),
        ["blade_heir"]
    );
    // Nothing for a top-tier class or an unknown one.
    let top = w.unit("bulwark", GUARD_STATS);
    assert_eq!(promotion_targets(&top, &w.classes).len(), 0);
    let mut gone = u.clone();
    gone.class = cid("gone");
    assert_eq!(promotion_targets(&gone, &w.classes).len(), 0);
    // Never an enemy-only class, nor a lord-only one for another unit.
    if let Some(guard) = w.classes.classes.get_mut(&cid("guard")) {
        guard.promotes_to = vec![cid("brigand"), cid("blade_heir"), cid("bulwark")];
    }
    assert_eq!(ids(&promotion_targets(&u, &w.classes)), ["bulwark"]);
}

#[test]
fn mastery_is_the_class_level_cap() {
    let w = World::new();
    let mut u = w.unit("guard", GUARD_STATS);
    assert!(!has_mastered(&u, &cid("guard"), &w.classes));
    at_class_level(&mut u, "guard", 9);
    assert!(!has_mastered(&u, &cid("guard"), &w.classes));
    master(&mut u, "guard");
    assert!(has_mastered(&u, &cid("guard"), &w.classes));
    assert!(!has_mastered(&u, &cid("rider"), &w.classes));
}

#[test]
fn error_messages() {
    let c = cid("duelist");
    let cases = [
        (
            Refused::UnknownClass(c.clone()),
            "unknown class \"duelist\"",
        ),
        (
            Refused::NotMastered(c.clone()),
            "\"duelist\" isn't mastered",
        ),
        (
            Refused::NotAPromotion {
                from: cid("guard"),
                to: c.clone(),
            },
            "\"guard\" doesn't promote to \"duelist\"",
        ),
        (
            Refused::EnemyOnly(c.clone()),
            "\"duelist\" is only for enemies",
        ),
        (
            Refused::LordOnly(c.clone()),
            "\"duelist\" is only for the lord",
        ),
        (
            Refused::NoSeal(SealKind::Tier(2)),
            "no seal for tier 2 in stock",
        ),
        (
            Refused::NoSeal(SealKind::Reclass),
            "no reclass seal in stock",
        ),
        (Refused::SameClass(c.clone()), "already in \"duelist\""),
        (
            Refused::NotReachable(c),
            "a reclass seal can't reach \"duelist\"",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
}

/// The promotions of the test table: `(from, to)`.
const PROMOTIONS: [(&str, &str); 5] = [
    ("guard", "bulwark"),
    ("guard", "iron_rider"),
    ("rider", "iron_rider"),
    ("mage", "sorcerer"),
    ("sorcerer", "archmage"),
];

proptest! {
    #[test]
    fn promotion_never_lowers_a_stat_or_passes_a_ceiling(
        stats in prop::array::uniform7(0..=80i32),
        hurt in 0..80i32,
        which in 0..PROMOTIONS.len(),
        level in 1u32..99,
        exp in 0u32..100,
    ) {
        let w = World::new();
        let ceilings = w.classes.hard_ceilings;
        let (from, to) = PROMOTIONS[which];
        let mut stats: [StatValue; 7] = stats;
        for (v, kind) in stats.iter_mut().zip(StatKind::GROWABLE) {
            *v = (*v).min(ceilings.get(kind));
        }
        let mut u = w.unit(from, stats);
        u.level = level;
        u.exp = exp;
        u.hp = (u.stats.hp - hurt).max(1);
        master(&mut u, from);
        let before = u.clone();
        let mut s = stock(&["tier_2_seal", "tier_3_seal"]);
        let events = promote(&mut u, &cid(to), &mut s, &w.tables());
        prop_assert!(events.is_ok());
        for kind in StatKind::GROWABLE {
            prop_assert!(u.stats.get(kind) >= before.stats.get(kind), "{:?}", kind);
            prop_assert!(u.stats.get(kind) <= ceilings.get(kind), "{:?}", kind);
        }
        prop_assert_eq!((u.level, u.exp), (level, exp));
        // Current HP rose by exactly the max-HP gain.
        prop_assert_eq!(u.hp - before.hp, u.stats.hp - before.stats.hp);
        prop_assert_eq!(s.items.values().sum::<u32>(), 1);
    }

    #[test]
    fn reclass_never_changes_stats_level_or_exp(
        stats in prop::array::uniform7(0..=80i32),
        from in 0..SPECS.len(),
        to in 0..SPECS.len(),
        mastered in any::<bool>(),
        level in 1u32..99,
        exp in 0u32..100,
    ) {
        let w = World::new();
        let (from, to) = (SPECS[from].id, SPECS[to].id);
        let mut u = w.lord(from);
        u.stats = Stats::from_growable(stats, u.stats.mov);
        u.hp = u.stats.hp.max(1);
        u.level = level;
        u.exp = exp;
        if mastered {
            master(&mut u, from);
        }
        let before = u.clone();
        let mut s = stock(&["reclass_seal", "reclass_seal"]);
        let result = reclass(&mut u, &cid(to), &mut s, &w.tables());
        prop_assert_eq!(growable(&u), growable(&before));
        prop_assert_eq!((u.hp, u.level, u.exp), (before.hp, level, exp));
        let seals = s.count(&item("reclass_seal"));
        if result.is_ok() {
            prop_assert_eq!(&u.class, &cid(to));
            prop_assert_eq!(seals, 1);
            // Every class it had unlocked keeps its record.
            for (class, record) in &before.class_records {
                prop_assert_eq!(u.class_records.get(class), Some(record));
            }
        } else {
            prop_assert_eq!(&u, &before);
            prop_assert_eq!(seals, 2);
        }
    }
}
