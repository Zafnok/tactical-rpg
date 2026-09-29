//! Tests of the art rules: effects on a weapon and mods, notes, and which
//! arts a unit knows and can use (rank, class, weapon arts, cost).

use std::collections::BTreeMap;

use super::*;
use crate::class::{ClassDef, ClassId, UnitTags, WeaponProficiency};
use crate::combat::DamageType;
use crate::geom::Pos;
use crate::item::{ItemDef, ItemId, Loadout, WeaponDef, WeaponInstance};
use crate::magic::Element;
use crate::stats::{Growths, Stats};
use crate::terrain::MovementTypeId;
use crate::unit::{Faction, UnitId};

fn aid(id: &str) -> ArtId {
    ArtId::new(id)
}

fn art(id: &str, kind: WeaponKind, rank: Option<WeaponRank>, cost: u32) -> ArtDef {
    ArtDef {
        id: aid(id),
        name: id.into(),
        kind,
        rank,
        cost,
        effect: ArtEffect::default(),
    }
}

/// Sword arts `cut` (E, 2), `break` (D, 4), `finale` (A, 1); spear art
/// `thrust` (E, 2); weapon art `moonlight` (sword, 3).
fn arts() -> ArtTable {
    use WeaponKind::{Spear, Sword};
    use WeaponRank::{A, D, E};
    let all = [
        art("cut", Sword, Some(E), 2),
        art("break", Sword, Some(D), 4),
        art("finale", Sword, Some(A), 1),
        art("thrust", Spear, Some(E), 2),
        art("moonlight", Sword, None, 3),
    ];
    ArtTable {
        arts: all.into_iter().map(|a| (a.id.clone(), a)).collect(),
    }
}

fn weapon(kind: WeaponKind, rank: WeaponRank, arts: &[&str]) -> ItemDef {
    ItemDef::Weapon(WeaponDef {
        name: "W".into(),
        kind,
        rank,
        might: 5,
        hit: 90,
        crit: 0,
        weight: 2,
        min_range: 1,
        max_range: 1,
        damage_type: DamageType::Physical,
        durability: 20,
        effective: vec![],
        arts: arts.iter().map(|a| aid(a)).collect(),
        price: 0,
    })
}

/// `sword` (E), `moon_blade` (a sword with `moonlight`, rank C), `spear`.
fn items() -> ItemTable {
    ItemTable {
        items: BTreeMap::from([
            (
                ItemId::new("sword"),
                weapon(WeaponKind::Sword, WeaponRank::E, &[]),
            ),
            (
                ItemId::new("moon_blade"),
                weapon(WeaponKind::Sword, WeaponRank::C, &["moonlight"]),
            ),
            (
                ItemId::new("spear"),
                weapon(WeaponKind::Spear, WeaponRank::E, &[]),
            ),
        ]),
        ..ItemTable::default()
    }
}

fn class(id: &str, kinds: &[WeaponKind]) -> ClassDef {
    ClassDef {
        id: ClassId(id.into()),
        name: id.into(),
        tier: 1,
        movement_type: MovementTypeId(0),
        move_points: 5,
        base: Stats::default(),
        growths: Growths::default(),
        weapons: kinds
            .iter()
            .map(|&kind| WeaponProficiency {
                kind,
                start: WeaponRank::E,
                max: WeaponRank::S,
            })
            .collect(),
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
    }
}

/// `swordsman` (swords), `soldier` (spears), `sage` (nothing).
fn classes() -> ClassTable {
    let all = [
        class("swordsman", &[WeaponKind::Sword]),
        class("soldier", &[WeaponKind::Spear]),
        class("sage", &[]),
    ];
    ClassTable {
        classes: all.into_iter().map(|c| (c.id.clone(), c)).collect(),
        ..ClassTable::default()
    }
}

/// A Swordsman at sword rank `rank` carrying `weapons` (durability 20).
fn swordsman(rank: WeaponRank, weapons: &[&str]) -> Unit {
    let mut u = Unit::generic(
        UnitId(1),
        &ClassId("swordsman".into()),
        &classes(),
        1,
        Faction::Player,
        Pos::new(0, 0),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    u.weapon_ranks.insert(WeaponKind::Sword, rank);
    let mut loadout = Loadout::default();
    for (slot, w) in weapons.iter().enumerate() {
        loadout.weapons[slot] = Some(WeaponInstance {
            def: ItemId::new(w),
            durability_left: 20,
        });
    }
    u.loadout = loadout;
    u
}

fn ids(arts: &[&ArtDef]) -> Vec<String> {
    arts.iter().map(|a| a.id.0.clone()).collect()
}

fn arts_for(u: &Unit, slot: usize) -> Vec<String> {
    ids(&u.arts_for(slot, &classes(), &items(), &arts()))
}

fn usable(u: &Unit, slot: usize) -> Vec<String> {
    ids(&u.usable_arts(slot, &classes(), &items(), &arts()))
}

fn stats() -> WeaponStats {
    WeaponStats {
        kind: Some(WeaponKind::Axe),
        trait_: WeaponTrait::AxeMinDamage(5),
        might: 8,
        hit: 75,
        crit: 0,
        weight: 6,
        min_range: 2,
        max_range: 2,
        damage_type: DamageType::Physical,
        effective: vec![(UnitTag::Flying, 3)],
        broken: false,
        element: Element::None,
    }
}

// ---- Effects ---------------------------------------------------------------

#[test]
fn the_default_effect_changes_nothing() {
    let (mut w, mut m) = (stats(), CombatMods::default());
    ArtEffect::default().apply(&mut w, &mut m);
    assert_eq!((w, m), (stats(), CombatMods::default()));
    assert_eq!(ArtEffect::default().notes(), []);
}

#[test]
fn an_effect_changes_the_weapon_and_the_mods() {
    let effect = ArtEffect {
        hit: -10,
        sword_followup: Some((3, 2)),
        effective: vec![(UnitTag::Armored, 2)],
        axe_min_damage: Some(8),
        min_range: Some(1),
        ..ArtEffect::default()
    };
    let mut w = stats();
    let mut m = CombatMods {
        hit: 4,
        ..CombatMods::default()
    };
    effect.apply(&mut w, &mut m);
    assert_eq!(
        m,
        CombatMods {
            hit: -6,
            sword_followup: Some((3, 2)),
            ..CombatMods::default()
        }
    );
    assert_eq!(
        w,
        WeaponStats {
            trait_: WeaponTrait::AxeMinDamage(8),
            min_range: 1,
            effective: vec![(UnitTag::Flying, 3), (UnitTag::Armored, 2)],
            ..stats()
        }
    );
}

#[test]
fn an_effect_without_a_follow_up_keeps_the_mods_one() {
    let mut m = CombatMods {
        sword_followup: Some((1, 1)),
        ..CombatMods::default()
    };
    ArtEffect::default().apply(&mut stats(), &mut m);
    assert_eq!(m.sword_followup, Some((1, 1)));
}

#[test]
fn notes_spell_out_the_effects_that_are_not_numbers() {
    let slow = Debuff {
        stat: StatKind::Spd,
        amount: 3,
    };
    let stance = TimedMods {
        stats: vec![],
        combat: CombatMods {
            avoid: 20,
            ..CombatMods::default()
        },
    };
    let effect = ArtEffect {
        hit: 10,
        no_counter: true,
        line_pierce: true,
        on_first_hit: Some(slow),
        stance: Some(stance.clone()),
        ..ArtEffect::default()
    };
    assert_eq!(
        effect.notes(),
        [
            ArtNote::NoCounter,
            ArtNote::Pierces,
            ArtNote::Debuff(slow),
            ArtNote::Stance(stance),
        ]
    );
}

#[test]
fn a_debuff_is_a_negative_stat_bonus() {
    let pin = Debuff {
        stat: StatKind::Mov,
        amount: 3,
    };
    assert_eq!(
        pin.mods(),
        TimedMods {
            stats: vec![(StatKind::Mov, -3)],
            combat: CombatMods::default(),
        }
    );
}

#[test]
fn an_art_costs_durability() {
    assert_eq!(
        art("x", WeaponKind::Bow, None, 3).skill_cost(),
        SkillCost::Durability(3)
    );
}

// ---- Learning and using ----------------------------------------------------

#[test]
fn rank_arts_are_learned_by_rank() {
    assert_eq!(arts_for(&swordsman(WeaponRank::E, &["sword"]), 0), ["cut"]);
    assert_eq!(
        arts_for(&swordsman(WeaponRank::D, &["sword"]), 0),
        ["cut", "break"]
    );
    // Lowest rank first, whatever the ids.
    assert_eq!(
        arts_for(&swordsman(WeaponRank::S, &["sword"]), 0),
        ["cut", "break", "finale"]
    );
}

#[test]
fn a_weapon_art_comes_with_its_weapon_whatever_the_rank() {
    let u = swordsman(WeaponRank::C, &["sword", "moon_blade"]);
    assert_eq!(arts_for(&u, 0), ["cut", "break"]);
    assert_eq!(arts_for(&u, 1), ["cut", "break", "moonlight"]);
}

#[test]
fn no_arts_without_a_weapon_the_unit_can_wield() {
    let u = swordsman(WeaponRank::D, &["sword", "spear", "moon_blade"]);
    // Empty slot.
    assert_eq!(arts_for(&u, 2), Vec::<String>::new());
    // A spear, which Swordsmen can't wield.
    assert_eq!(arts_for(&u, 1), Vec::<String>::new());
    // Rank D can't wield a rank-C blade, so no weapon art either.
    let low = swordsman(WeaponRank::D, &["moon_blade"]);
    assert_eq!(arts_for(&low, 0), Vec::<String>::new());
    // Unknown class.
    let lost = Unit {
        class: ClassId("nope".into()),
        ..swordsman(WeaponRank::D, &["sword"])
    };
    assert_eq!(arts_for(&lost, 0), Vec::<String>::new());
}

#[test]
fn after_a_reclass_arts_are_known_but_unusable() {
    let mut u = swordsman(WeaponRank::D, &["sword"]);
    u.class = ClassId("sage".into());
    assert_eq!(ids(&u.known_arts(&arts())), ["cut", "break"]);
    assert_eq!(arts_for(&u, 0), Vec::<String>::new());
    // Back in a sword class, they are usable again.
    u.class = ClassId("swordsman".into());
    assert_eq!(arts_for(&u, 0), ["cut", "break"]);
}

#[test]
fn known_arts_follow_recorded_ranks_only() {
    let mut u = swordsman(WeaponRank::E, &[]);
    u.weapon_ranks = BTreeMap::from([(WeaponKind::Sword, WeaponRank::D)]);
    assert_eq!(ids(&u.known_arts(&arts())), ["cut", "break"]);
    u.weapon_ranks.insert(WeaponKind::Spear, WeaponRank::E);
    assert_eq!(ids(&u.known_arts(&arts())), ["cut", "break", "thrust"]);
    u.weapon_ranks.clear();
    assert_eq!(ids(&u.known_arts(&arts())), Vec::<String>::new());
}

#[test]
fn only_affordable_arts_are_usable() {
    let mut u = swordsman(WeaponRank::D, &["sword"]);
    assert_eq!(usable(&u, 0), ["cut", "break"]);
    let set = |u: &mut Unit, left| {
        if let Some(Some(w)) = u.loadout.weapons.first_mut() {
            w.durability_left = left;
        }
    };
    set(&mut u, 4);
    assert_eq!(usable(&u, 0), ["cut", "break"]);
    set(&mut u, 3);
    assert_eq!(usable(&u, 0), ["cut"]);
    set(&mut u, 2);
    assert_eq!(usable(&u, 0), ["cut"]);
    set(&mut u, 1);
    assert_eq!(usable(&u, 0), Vec::<String>::new());
    // Broken: none, though every art is still for this weapon.
    set(&mut u, 0);
    assert_eq!(usable(&u, 0), Vec::<String>::new());
    assert_eq!(arts_for(&u, 0), ["cut", "break"]);
}

#[test]
fn the_table_finds_arts_by_id() {
    let t = arts();
    assert_eq!(t.get(&aid("cut")).map(|a| a.cost), Some(2));
    assert_eq!(t.get(&aid("nope")), None);
}
