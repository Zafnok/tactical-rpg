//! Tests of the Preparations rules, on the item tests' fixtures: a fencer
//! (swords up to rank E, light armour, 3 weapon slots; Str 5, Spd 7) and
//! their items (every weapon weighs 2, mail weighs 2, the ring gives Spd +2).

use std::sync::Arc;

use super::*;
use crate::battle::tests::setup;
use crate::class::ClassTable;
use crate::item::tests::{classes, classes_with, id, items, mage_class, unit};
use crate::item::{BattlePack, Stock, WeaponInstance};

const U: UnitId = UnitId(1);

fn copy(item: &str, durability_left: u32) -> WeaponInstance {
    WeaponInstance {
        def: id(item),
        durability_left,
    }
}

/// The fencer alone, with `weapons` and `others` in the stock and an empty
/// pack of cap 2.
fn prep_with(classes: ClassTable, weapons: &[&str], others: &[&str]) -> BattleSetup {
    let mut stock = Stock {
        weapons: weapons.iter().map(|w| copy(w, 20)).collect(),
        ..Stock::default()
    };
    for o in others {
        stock.add(id(o));
    }
    BattleSetup {
        classes: Arc::new(classes),
        items: Arc::new(items()),
        units: vec![unit()],
        stock,
        pack: BattlePack {
            items: vec![],
            cap: 2,
        },
        ..setup(vec![])
    }
}

fn prep(weapons: &[&str], others: &[&str]) -> BattleSetup {
    prep_with(classes(), weapons, others)
}

fn stock_weapons(s: &BattleSetup) -> Vec<&str> {
    s.stock.weapons.iter().map(|w| w.def.0.as_str()).collect()
}

fn slot_weapon(s: &BattleSetup, slot: usize) -> Option<&str> {
    s.units[0].loadout.weapon(slot).map(|w| w.def.0.as_str())
}

#[test]
fn a_unit_has_its_classes_weapon_slots_then_armour_and_accessory() {
    let s = prep(&[], &[]);
    assert_eq!(
        s.gear_slots(U),
        [
            GearSlot::Weapon(0),
            GearSlot::Weapon(1),
            GearSlot::Weapon(2),
            GearSlot::Armour,
            GearSlot::Accessory
        ]
    );
    let mage = prep_with(classes_with(mage_class()), &[], &[]);
    assert_eq!(mage.gear_slots(U), [GearSlot::Armour, GearSlot::Accessory]);
    assert!(s.gear_slots(UnitId(9)).is_empty());
    assert_eq!(s.player_units().count(), 1);
}

#[test]
fn only_player_units_are_prepared() {
    let mut s = prep(&["sword"], &["vest", "potion"]);
    s.units[0].faction = Faction::Enemy;
    assert_eq!(s.player_units().count(), 0);
    assert!(s.gear_slots(U).is_empty());
    assert_eq!(s.attack_speed(U, None), None);
    assert_eq!(s.unusable(U, &id("bow")), None);
    let before = s.clone();
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(0)),
        Err(PrepError::NoUnit)
    );
    assert_eq!(s.gear_to_stock(U, GearSlot::Armour), Err(PrepError::NoUnit));
    assert_eq!(s, before);
}

#[test]
fn a_stock_weapon_goes_in_a_slot_and_is_equipped_if_nothing_is() {
    let mut s = prep(&["javelin", "sword"], &[]);
    s.stock.weapons[1].durability_left = 7;
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(1), &StockItem::Weapon(1)),
        Ok(())
    );
    assert_eq!(s.units[0].loadout.weapon(1), Some(&copy("sword", 7)));
    assert_eq!(stock_weapons(&s), ["javelin"]);
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Weapon(1)));
    // A second weapon leaves the equipped one alone.
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(0)),
        Ok(())
    );
    assert_eq!(slot_weapon(&s, 0), Some("javelin"));
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Weapon(1)));
    assert!(s.stock.weapons.is_empty());
}

#[test]
fn the_weapon_a_slot_held_takes_the_stock_place_of_the_one_taken() {
    let mut s = prep(&["axe", "javelin", "bow"], &[]);
    s.units[0].loadout.weapons[0] = Some(copy("sword", 3));
    s.units[0].loadout.equipped = Some(Equipped::Weapon(0));
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(1)),
        Ok(())
    );
    assert_eq!(slot_weapon(&s, 0), Some("javelin"));
    assert_eq!(stock_weapons(&s), ["axe", "sword", "bow"]);
    assert_eq!(s.stock.weapons[1].durability_left, 3);
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Weapon(0)));
}

#[test]
fn unusable_gear_is_refused_with_the_reason() {
    let mut s = prep(&["bow", "steel_sword", "sword"], &["mail", "vest", "ring"]);
    assert_eq!(
        s.unusable(U, &id("bow")),
        Some(Unusable::Kind(WeaponKind::Bow))
    );
    assert_eq!(
        s.unusable(U, &id("steel_sword")),
        Some(Unusable::Rank(WeaponRank::D))
    );
    assert_eq!(
        s.unusable(U, &id("mail")),
        Some(Unusable::Armour(ArmourWeight::Medium))
    );
    for ok in ["sword", "vest", "ring", "potion", "nothing"] {
        assert_eq!(s.unusable(U, &id(ok)), None, "{ok}");
    }
    let before = s.clone();
    for (i, why) in [
        (0, Unusable::Kind(WeaponKind::Bow)),
        (1, Unusable::Rank(WeaponRank::D)),
    ] {
        assert_eq!(
            s.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(i)),
            Err(PrepError::Unusable(why))
        );
    }
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Armour, &StockItem::Item(id("mail"))),
        Err(PrepError::Unusable(Unusable::Armour(ArmourWeight::Medium)))
    );
    assert_eq!(s, before);
    // With the rank, the steel sword is fine.
    s.units[0]
        .weapon_ranks
        .insert(WeaponKind::Sword, WeaponRank::D);
    assert_eq!(s.unusable(U, &id("steel_sword")), None);
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(1)),
        Ok(())
    );
}

#[test]
fn armour_and_accessories_swap_with_the_stock() {
    let mut s = prep(&[], &["vest", "ring", "ring"]);
    s.units[0].loadout.armour = Some(id("mail"));
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Armour, &StockItem::Item(id("vest"))),
        Ok(())
    );
    assert_eq!(s.units[0].loadout.armour, Some(id("vest")));
    assert_eq!(s.stock.count(&id("vest")), 0);
    assert_eq!(s.stock.count(&id("mail")), 1);
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Accessory, &StockItem::Item(id("ring"))),
        Ok(())
    );
    assert_eq!(s.units[0].loadout.accessory, Some(id("ring")));
    assert_eq!(s.stock.count(&id("ring")), 1);
    // The same again: the worn ring goes back as the new one comes.
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Accessory, &StockItem::Item(id("ring"))),
        Ok(())
    );
    assert_eq!(s.stock.count(&id("ring")), 1);
    assert_eq!(s.units[0].loadout.armour, Some(id("vest")));
}

#[test]
fn gear_must_be_in_stock_and_fit_the_slot() {
    let mut s = prep(&["sword"], &["vest", "ring", "potion"]);
    let before = s.clone();
    let item = |i: &str| StockItem::Item(id(i));
    let cases = [
        (GearSlot::Weapon(3), StockItem::Weapon(0), PrepError::NoSlot),
        (
            GearSlot::Weapon(0),
            StockItem::Weapon(1),
            PrepError::NotInStock,
        ),
        (GearSlot::Armour, item("mail"), PrepError::NotInStock),
        (GearSlot::Weapon(0), item("vest"), PrepError::WrongSlot),
        (GearSlot::Armour, StockItem::Weapon(0), PrepError::WrongSlot),
        (
            GearSlot::Accessory,
            StockItem::Weapon(0),
            PrepError::WrongSlot,
        ),
        (GearSlot::Armour, item("ring"), PrepError::WrongSlot),
        (GearSlot::Accessory, item("vest"), PrepError::WrongSlot),
        (GearSlot::Accessory, item("potion"), PrepError::WrongSlot),
    ];
    for (slot, item, error) in cases {
        assert_eq!(
            s.gear_from_stock(U, slot, &item),
            Err(error),
            "{slot:?} {item:?}"
        );
    }
    assert_eq!(s, before);
    // A class with no weapon slots takes no weapon.
    let mut mage = prep_with(classes_with(mage_class()), &["sword"], &[]);
    assert_eq!(
        mage.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(0)),
        Err(PrepError::NoSlot)
    );
}

#[test]
fn gear_goes_back_to_the_stock_and_the_next_weapon_is_equipped() {
    let mut s = prep(&["axe"], &[]);
    let loadout = &mut s.units[0].loadout;
    loadout.weapons[0] = Some(copy("sword", 4));
    loadout.weapons[2] = Some(copy("javelin", 20));
    loadout.equipped = Some(Equipped::Weapon(0));
    loadout.armour = Some(id("vest"));
    loadout.accessory = Some(id("ring"));
    assert_eq!(s.gear_to_stock(U, GearSlot::Weapon(0)), Ok(()));
    assert_eq!(stock_weapons(&s), ["axe", "sword"]);
    assert_eq!(s.stock.weapons[1].durability_left, 4);
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Weapon(2)));
    assert_eq!(s.gear_to_stock(U, GearSlot::Weapon(2)), Ok(()));
    assert_eq!(s.units[0].loadout.equipped, None);
    assert_eq!(s.gear_to_stock(U, GearSlot::Armour), Ok(()));
    assert_eq!(s.gear_to_stock(U, GearSlot::Accessory), Ok(()));
    assert_eq!(s.units[0].loadout, crate::item::Loadout::default());
    assert_eq!(s.stock.count(&id("vest")), 1);
    assert_eq!(s.stock.count(&id("ring")), 1);
    let before = s.clone();
    for slot in [
        GearSlot::Weapon(0),
        GearSlot::Weapon(7),
        GearSlot::Armour,
        GearSlot::Accessory,
    ] {
        assert_eq!(s.gear_to_stock(U, slot), Err(PrepError::EmptySlot));
    }
    assert_eq!(s, before);
}

#[test]
fn an_equipped_weapon_the_unit_can_wield_stays_equipped() {
    let mut s = prep(&["sword"], &[]);
    let loadout = &mut s.units[0].loadout;
    loadout.weapons[1] = Some(copy("javelin", 20));
    loadout.weapons[2] = Some(copy("sword", 20));
    loadout.equipped = Some(Equipped::Weapon(2));
    assert_eq!(s.gear_to_stock(U, GearSlot::Weapon(1)), Ok(()));
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Weapon(2)));
    // An equipped weapon it can't wield (old data) gives way.
    let mut s = prep(&["sword"], &[]);
    s.units[0].loadout.weapons[2] = Some(copy("bow", 20));
    s.units[0].loadout.equipped = Some(Equipped::Weapon(2));
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(1), &StockItem::Weapon(0)),
        Ok(())
    );
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Weapon(1)));
}

#[test]
fn an_equipped_spell_stays_equipped() {
    let mut s = prep(&["sword"], &[]);
    let spell = crate::spell::SpellId::new("fire");
    s.units[0].loadout.equipped = Some(Equipped::Spell(spell.clone()));
    assert_eq!(
        s.gear_from_stock(U, GearSlot::Weapon(0), &StockItem::Weapon(0)),
        Ok(())
    );
    assert_eq!(s.units[0].loadout.equipped, Some(Equipped::Spell(spell)));
}

#[test]
fn the_pack_takes_consumables_from_the_stock_up_to_its_cap() {
    let mut s = prep(&["sword"], &["potion", "potion", "potion", "vest"]);
    assert_eq!(
        s.pack_from_stock(&id("vest")),
        Err(PrepError::NotConsumable)
    );
    assert_eq!(
        s.pack_from_stock(&id("sword")),
        Err(PrepError::NotConsumable)
    );
    assert_eq!(s.pack_from_stock(&id("potion")), Ok(()));
    assert_eq!(s.pack_from_stock(&id("potion")), Ok(()));
    assert_eq!(s.pack.items, [id("potion"), id("potion")]);
    assert_eq!(s.stock.count(&id("potion")), 1);
    let before = s.clone();
    assert_eq!(s.pack_from_stock(&id("potion")), Err(PrepError::PackFull));
    assert_eq!(s, before);
    // Back to the stock, then the last one in stock goes in.
    assert_eq!(s.pack_to_stock(&id("potion")), Ok(()));
    assert_eq!(s.pack.items, [id("potion")]);
    assert_eq!(s.stock.count(&id("potion")), 2);
    assert_eq!(s.pack_to_stock(&id("potion")), Ok(()));
    assert_eq!(s.pack_to_stock(&id("potion")), Err(PrepError::NotInPack));
    s.stock.items.clear();
    assert_eq!(s.pack_from_stock(&id("potion")), Err(PrepError::NotInStock));
    assert!(s.pack.items.is_empty());
}

#[test]
fn the_last_packed_copy_goes_back_first() {
    let mut s = prep(&[], &[]);
    s.pack.items = vec![id("potion"), id("vest"), id("potion")];
    assert_eq!(s.pack_to_stock(&id("potion")), Ok(()));
    assert_eq!(s.pack.items, [id("potion"), id("vest")]);
}

#[test]
fn attack_speed_follows_the_weapon_in_hand_and_the_gear() {
    // Spd 7, Str 5 (carries 1 weight); every weapon weighs 2.
    let mut s = prep(&[], &[]);
    assert_eq!(s.attack_speed(U, None), Some(7), "no weapon");
    s.units[0].loadout.weapons[1] = Some(copy("sword", 20));
    assert_eq!(s.attack_speed(U, None), Some(7), "nothing equipped");
    assert_eq!(s.attack_speed(U, Some(1)), Some(6));
    assert_eq!(s.attack_speed(U, Some(0)), Some(7), "an empty slot");
    s.units[0].loadout.equipped = Some(Equipped::Weapon(1));
    assert_eq!(s.attack_speed(U, None), Some(6));
    // Mail weighs 2 more; the ring gives Spd +2.
    s.units[0].loadout.armour = Some(id("mail"));
    assert_eq!(s.attack_speed(U, None), Some(4));
    assert_eq!(s.attack_speed(U, Some(0)), Some(6));
    s.units[0].loadout.accessory = Some(id("ring"));
    assert_eq!(s.attack_speed(U, Some(1)), Some(6));
    // A weapon it can't wield counts as none.
    s.units[0].loadout.weapons[2] = Some(copy("bow", 20));
    assert_eq!(s.attack_speed(U, Some(2)), Some(8));
    assert_eq!(s.attack_speed(UnitId(9), None), None);
}

#[test]
fn errors_read_as_sentences() {
    let all = [
        PrepError::NoUnit,
        PrepError::NoSlot,
        PrepError::NotInStock,
        PrepError::WrongSlot,
        PrepError::Unusable(Unusable::Rank(WeaponRank::D)),
        PrepError::EmptySlot,
        PrepError::PackFull,
        PrepError::NotConsumable,
        PrepError::NotInPack,
    ];
    let texts: std::collections::BTreeSet<String> = all.iter().map(ToString::to_string).collect();
    assert_eq!(texts.len(), all.len());
    assert_eq!(PrepError::PackFull.to_string(), "the pack is full");
}
