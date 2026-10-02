//! Promotion and reclass (ticket 0603) with the real class tree and seals
//! (`assets/data/classes.ron`, `items.ron`): `progression.md`'s shared
//! promotion worked by hand, and every promotion in the tree.

use std::sync::OnceLock;

use trpg_content::Content;
use trpg_core::progression::class_change::ChangeTables;
use trpg_core::{
    ClassId, ClassRecord, Event, Faction, ItemId, Pos, StatGains, StatKind, Stock, Unit, UnitId,
    promote, reclass, reclass_targets,
};

fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| trpg_content::load_embedded().unwrap_or_else(|e| panic!("{e}")))
}

fn tables() -> ChangeTables<'static> {
    let c = content();
    ChangeTables {
        classes: &c.classes,
        items: &c.items,
        spells: &c.spells,
    }
}

fn cid(id: &str) -> ClassId {
    ClassId(id.into())
}

/// A level-1 unit that has mastered `class` (the lord, for a lord-only
/// class).
fn master_of(class: &ClassId) -> Unit {
    let c = content();
    let def = c.classes.get(class).unwrap_or_else(|| panic!("{class:?}"));
    // A generic unit can't start in a lord-only class: start it elsewhere.
    let start = if def.lord_only {
        cid("swordsman")
    } else {
        class.clone()
    };
    let mut u = Unit::generic(
        UnitId(1),
        &start,
        &c.classes,
        1,
        Faction::Player,
        Pos::new(0, 0),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    u.is_lord = def.lord_only;
    u.class = class.clone();
    u.stats = def.base;
    u.hp = u.stats.hp;
    let mastered = ClassRecord {
        class_level: c.classes.class_level_cap,
        class_points: 0,
    };
    u.class_records = [(class.clone(), mastered)].into();
    u.refresh_spells(&c.classes);
    u
}

fn seals() -> Stock {
    let mut stock = Stock::default();
    for id in ["tier_2_seal", "tier_3_seal", "reclass_seal"] {
        stock.add(ItemId::new(id));
    }
    stock
}

fn gains(from: &str, to: &str) -> StatGains {
    let mut u = master_of(&cid(from));
    let events = promote(&mut u, &cid(to), &mut seals(), &tables());
    match events.as_deref() {
        Ok([Event::Promoted { gains, .. }, ..]) => *gains,
        other => panic!("{from} → {to}: {other:?}"),
    }
}

/// `progression.md`, *Shared promotions*: the bonus is worked out from
/// whichever class the unit came from. Iron Rider `27 10 0 6 6 11 2`, Guard
/// `20 7 0 4 2 9 0`, Rider `20 6 0 5 5 5 1`.
#[test]
fn iron_rider_from_guard_and_from_rider() {
    assert_eq!(
        gains("guard", "iron_rider"),
        StatGains([7, 3, 0, 2, 4, 2, 2])
    );
    assert_eq!(
        gains("rider", "iron_rider"),
        StatGains([7, 4, 0, 1, 1, 6, 1])
    );
    // Mystic 21 2 9 7 7 2 9, from Mage 16 1 6 5 5 1 5 and Cleric 16 1 5 4 5 1 7.
    assert_eq!(gains("mage", "mystic"), StatGains([5, 1, 3, 2, 2, 1, 4]));
    assert_eq!(gains("cleric", "mystic"), StatGains([5, 1, 4, 3, 2, 1, 2]));
}

#[test]
fn every_promotion_in_the_tree_works_with_its_tiers_seal() {
    let c = content();
    let mut promotions = 0;
    for from in c.classes.classes.values() {
        for to in &from.promotes_to {
            let target = c.classes.get(to).unwrap_or_else(|| panic!("{to:?}"));
            let mut u = master_of(&from.id);
            let before = u.clone();
            let mut stock = seals();
            let events = promote(&mut u, to, &mut stock, &tables());
            assert!(events.is_ok(), "{:?} → {to:?}: {events:?}", from.id);
            assert_eq!(&u.class, to);
            assert_eq!(target.tier, from.tier + 1, "{to:?}");
            // The seal of the target's tier went, the others stayed.
            let seal = ItemId::new(&format!("tier_{}_seal", target.tier));
            assert_eq!(stock.count(&seal), 0, "{to:?}");
            assert_eq!(stock.items.values().sum::<u32>(), 2, "{to:?}");
            // From its base stats, a unit lands on the higher of the two
            // bases in every stat.
            for kind in StatKind::GROWABLE {
                let expected = from.base.get(kind).max(target.base.get(kind));
                assert_eq!(u.stats.get(kind), expected, "{to:?} {kind:?}");
            }
            assert_eq!(u.stats.mov, target.move_points, "{to:?}");
            assert_eq!((u.level, u.exp), (before.level, before.exp));
            assert!(u.validate_loadout(&c.classes, &c.items).is_ok());
            promotions += 1;
        }
    }
    // Tier 1: 9 classes with 2 branches each; tier 2: 15 promotions, the
    // Lancer's two included (progression.md's class tree).
    assert_eq!(promotions, 9 * 2 + 17);
}

#[test]
fn a_reclass_seal_reaches_the_tier_one_classes_of_the_tree() {
    let c = content();
    let mut u = master_of(&cid("swordsman"));
    let names: Vec<&str> = reclass_targets(&u, &c.classes)
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Archer",
            "Brawler",
            "Cleric",
            "Guard",
            "Mage",
            "Raider",
            "Rider",
            // What its mastered Swordsman promotes to.
            "Duelist",
            "Shadowblade"
        ]
    );
    let stats = u.stats;
    let mut stock = seals();
    let events = reclass(&mut u, &cid("mage"), &mut stock, &tables());
    assert!(events.is_ok(), "{events:?}");
    assert_eq!(stock.count(&ItemId::new("reclass_seal")), 0);
    // Stats stay; only Mov follows the class (both are 5 here).
    assert_eq!(u.stats, stats);
    let spells: Vec<&str> = u.learned.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(spells, ["fire"]);
}
