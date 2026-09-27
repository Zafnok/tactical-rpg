//! Tests of the shop rules. Test items: `sword` (weapon, 1000 gold, 40
//! durability), `bow` (weapon, 480 gold, 45 durability), `free_knife`
//! (weapon, 0 gold), `vest` (armour, 300), `ring` (accessory, 999),
//! `potion` (consumable, 300).

use proptest::prelude::*;

use super::*;
use crate::class::ArmourWeight;
use crate::combat::DamageType;
use crate::item::{AccessoryDef, ArmourDef, ConsumableDef, ConsumableEffect};
use crate::stats::Stats;
use crate::weapon::{WeaponKind, WeaponRank};

fn weapon(price: u32, durability: u32) -> ItemDef {
    ItemDef::Weapon(WeaponDef {
        name: "w".into(),
        kind: WeaponKind::Sword,
        rank: WeaponRank::E,
        might: 5,
        hit: 90,
        crit: 0,
        weight: 5,
        min_range: 1,
        max_range: 1,
        damage_type: DamageType::Physical,
        durability,
        effective: vec![],
        price,
    })
}

fn items() -> ItemTable {
    let entries = [
        ("sword", weapon(1000, 40)),
        ("bow", weapon(480, 45)),
        ("free_knife", weapon(0, 10)),
        (
            "vest",
            ItemDef::Armour(ArmourDef {
                name: "Vest".into(),
                weight_class: ArmourWeight::Light,
                bonus: Stats::default(),
                weight: 0,
                price: 300,
            }),
        ),
        (
            "ring",
            ItemDef::Accessory(AccessoryDef {
                name: "Ring".into(),
                bonus: Stats::default(),
                price: 999,
            }),
        ),
        (
            "potion",
            ItemDef::Consumable(ConsumableDef {
                name: "Potion".into(),
                effect: ConsumableEffect::Heal(10),
                price: 300,
            }),
        ),
    ];
    ItemTable {
        items: entries
            .into_iter()
            .map(|(k, v)| (ItemId::new(k), v))
            .collect(),
        ..ItemTable::default()
    }
}

fn id(s: &str) -> ItemId {
    ItemId::new(s)
}

fn shop(kind: ShopKind, stock: &[&str]) -> Shop {
    Shop {
        kind,
        stock: stock.iter().map(|s| id(s)).collect(),
    }
}

fn armoury() -> Shop {
    shop(
        ShopKind::Armoury,
        &["sword", "vest", "ring", "potion", "ghost"],
    )
}

fn vendor() -> Shop {
    shop(ShopKind::Vendor, &["potion", "sword"])
}

fn smith() -> Shop {
    shop(ShopKind::Blacksmith, &[])
}

fn copy(def: &str, left: u32) -> WeaponInstance {
    WeaponInstance {
        def: id(def),
        durability_left: left,
    }
}

fn weapon_def(name: &str) -> WeaponDef {
    items().weapon(&id(name)).cloned().unwrap()
}

#[test]
fn repair_cost_table() {
    // (weapon, durability left, cost): ceil(price / 2 × missing / durability).
    let cases = [
        ("sword", 0, 500),  // fully used: half the price
        ("sword", 20, 250), // half used
        ("sword", 39, 13),  // 1 missing: ceil(12.5)
        ("sword", 40, 0),   // nothing missing
        ("bow", 38, 38),    // 7 of 45 missing: ceil(37.33)
        ("bow", 0, 240),
        ("free_knife", 0, 0),
    ];
    for (name, left, cost) in cases {
        assert_eq!(
            repair_cost(&weapon_def(name), &copy(name, left)),
            cost,
            "{name} at {left}"
        );
    }
}

#[test]
fn repair_cost_never_overflows() {
    let mut def = weapon_def("sword");
    def.price = u32::MAX;
    def.durability = u32::MAX;
    assert_eq!(repair_cost(&def, &copy("sword", 0)), u32::MAX / 2 + 1);
    // A zero-durability weapon (invalid data) doesn't divide by zero.
    def.durability = 0;
    assert_eq!(repair_cost(&def, &copy("sword", 0)), 0);
}

#[test]
fn sell_price_is_half_rounded_down() {
    let t = items();
    let price = |s: &str| sell_price(t.get(&id(s)).unwrap());
    assert_eq!(price("sword"), 500);
    assert_eq!(price("ring"), 499);
    assert_eq!(price("vest"), 150);
    assert_eq!(price("free_knife"), 0);
}

#[test]
fn pay_takes_gold_or_refuses() {
    let mut gold = 100;
    assert_eq!(pay(&mut gold, 100), Ok(()));
    assert_eq!(gold, 0);
    assert_eq!(
        pay(&mut gold, 1),
        Err(ShopError::NotEnoughGold { cost: 1, gold: 0 })
    );
    assert_eq!(gold, 0);
}

#[test]
fn kinds_stock_and_buy() {
    let t = items();
    let stocks = |k: ShopKind, s: &str| k.stocks(t.get(&id(s)).unwrap());
    assert!(stocks(ShopKind::Armoury, "sword"));
    assert!(stocks(ShopKind::Armoury, "vest"));
    assert!(stocks(ShopKind::Armoury, "ring"));
    assert!(!stocks(ShopKind::Armoury, "potion"));
    assert!(stocks(ShopKind::Vendor, "potion"));
    assert!(!stocks(ShopKind::Vendor, "sword"));
    assert!(!stocks(ShopKind::Blacksmith, "sword"));
    assert!(!stocks(ShopKind::Blacksmith, "potion"));
    assert!(ShopKind::Armoury.buys());
    assert!(ShopKind::Vendor.buys());
    assert!(!ShopKind::Blacksmith.buys());
}

#[test]
fn buy_pays_the_price() {
    let t = items();
    let mut gold = 1500;
    assert_eq!(buy(&armoury(), &t, &mut gold, &id("sword")), Ok(1000));
    assert_eq!(gold, 500);
    assert_eq!(buy(&vendor(), &t, &mut gold, &id("potion")), Ok(300));
    assert_eq!(gold, 200);
}

#[test]
fn buy_errors_change_nothing() {
    let t = items();
    let mut gold = 900;
    let cases = [
        (
            armoury(),
            "sword",
            ShopError::NotEnoughGold {
                cost: 1000,
                gold: 900,
            },
        ),
        (armoury(), "bow", ShopError::NotSoldHere(id("bow"))),
        // Listed, but not what an armoury sells.
        (armoury(), "potion", ShopError::NotSoldHere(id("potion"))),
        (vendor(), "sword", ShopError::NotSoldHere(id("sword"))),
        (armoury(), "ghost", ShopError::UnknownItem(id("ghost"))),
        (smith(), "sword", ShopError::NotSoldHere(id("sword"))),
    ];
    for (shop, item, err) in cases {
        assert_eq!(buy(&shop, &t, &mut gold, &id(item)), Err(err), "{item}");
        assert_eq!(gold, 900);
    }
}

#[test]
fn sell_gives_half_price() {
    let t = items();
    let mut gold = 10;
    assert_eq!(sell(&armoury(), &t, &mut gold, &id("ring")), Ok(499));
    assert_eq!(gold, 509);
    // Shops buy anything, listed or not.
    assert_eq!(sell(&vendor(), &t, &mut gold, &id("bow")), Ok(240));
    assert_eq!(gold, 749);
    let mut rich = u32::MAX - 1;
    assert_eq!(sell(&vendor(), &t, &mut rich, &id("sword")), Ok(500));
    assert_eq!(rich, u32::MAX);
}

#[test]
fn sell_errors_change_nothing() {
    let t = items();
    let mut gold = 10;
    assert_eq!(
        sell(&smith(), &t, &mut gold, &id("ring")),
        Err(ShopError::DoesNotBuy)
    );
    assert_eq!(
        sell(&armoury(), &t, &mut gold, &id("ghost")),
        Err(ShopError::UnknownItem(id("ghost")))
    );
    assert_eq!(gold, 10);
}

#[test]
fn repair_restores_durability() {
    let t = items();
    let mut gold = 300;
    let mut w = copy("sword", 20);
    assert_eq!(repair(&smith(), &t, &mut gold, &mut w), Ok(250));
    assert_eq!((gold, w.durability_left), (50, 40));
    // A broken weapon can be repaired.
    let mut broken = copy("bow", 0);
    let mut gold = 240;
    assert_eq!(repair(&smith(), &t, &mut gold, &mut broken), Ok(240));
    assert_eq!((gold, broken.durability_left), (0, 45));
}

#[test]
fn repair_errors_change_nothing() {
    let t = items();
    let cases = [
        (armoury(), copy("sword", 20), ShopError::DoesNotRepair),
        (vendor(), copy("sword", 20), ShopError::DoesNotRepair),
        (
            smith(),
            copy("sword", 40),
            ShopError::NothingToRepair(id("sword")),
        ),
        (
            smith(),
            copy("sword", 0),
            ShopError::NotEnoughGold {
                cost: 500,
                gold: 499,
            },
        ),
        (smith(), copy("vest", 0), ShopError::NotAWeapon(id("vest"))),
        (
            smith(),
            copy("ghost", 0),
            ShopError::UnknownItem(id("ghost")),
        ),
    ];
    for (shop, w, err) in cases {
        let mut gold = 499;
        let mut after = w.clone();
        assert_eq!(repair(&shop, &t, &mut gold, &mut after), Err(err));
        assert_eq!((gold, after), (499, w));
    }
}

#[test]
fn add_to_stock_by_kind() {
    let t = items();
    let mut stock = Stock::default();
    assert_eq!(add_to_stock(&mut stock, &t, &id("sword")), Ok(()));
    assert_eq!(add_to_stock(&mut stock, &t, &id("potion")), Ok(()));
    assert_eq!(
        add_to_stock(&mut stock, &t, &id("ghost")),
        Err(ShopError::UnknownItem(id("ghost")))
    );
    assert_eq!(stock.weapons, [copy("sword", 40)]);
    assert_eq!(stock.count(&id("potion")), 1);
    assert_eq!(stock.items.len(), 1);
}

#[test]
fn errors_display() {
    let cases = [
        (
            ShopError::NotEnoughGold { cost: 5, gold: 2 },
            "costs 5 gold, the party has 2",
        ),
        (ShopError::NotSoldHere(id("x")), "\"x\" isn't sold here"),
        (ShopError::DoesNotBuy, "this shop doesn't buy items"),
        (ShopError::DoesNotRepair, "this shop doesn't repair"),
        (ShopError::NothingToRepair(id("x")), "\"x\" needs no repair"),
        (ShopError::NoItem, "no such item"),
        (ShopError::NotAWeapon(id("x")), "\"x\" is not a weapon"),
        (ShopError::UnknownItem(id("x")), "unknown item \"x\""),
    ];
    for (e, text) in cases {
        assert_eq!(e.to_string(), text);
    }
}

// ---- ShopSession ------------------------------------------------------------

#[test]
fn session_buys_into_the_stock() {
    let (t, shop) = (items(), armoury());
    let (mut gold, mut stock) = (1500, Stock::default());
    let mut s = ShopSession::new(&shop, &t, &mut gold, &mut stock);
    assert_eq!(s.shop(), &shop);
    assert_eq!(s.buy(&id("sword")), Ok(1000));
    assert_eq!(s.buy(&id("vest")), Ok(300));
    assert_eq!(
        s.buy(&id("ring")),
        Err(ShopError::NotEnoughGold {
            cost: 999,
            gold: 200
        })
    );
    assert_eq!(s.gold(), 200);
    assert_eq!(s.stock().weapons, [copy("sword", 40)]);
    assert_eq!(s.stock().count(&id("vest")), 1);
    assert_eq!((gold, stock.items.len()), (200, 1));
}

#[test]
fn session_sells_from_the_stock() {
    let (t, shop) = (items(), vendor());
    let mut gold = 0;
    let mut stock = Stock::default();
    stock.add(id("ring"));
    stock.weapons.push(copy("bow", 3));
    let mut s = ShopSession::new(&shop, &t, &mut gold, &mut stock);
    assert_eq!(s.sell_item(&id("ring")), Ok(499));
    assert_eq!(s.sell_item(&id("ring")), Err(ShopError::NoItem));
    assert_eq!(s.sell_weapon(1), Err(ShopError::NoItem));
    assert_eq!(s.sell_weapon(0), Ok(240));
    assert_eq!(s.sell_weapon(0), Err(ShopError::NoItem));
    assert_eq!((gold, stock), (739, Stock::default()));
}

#[test]
fn session_blacksmith_repairs_and_doesnt_buy() {
    let (t, shop) = (items(), smith());
    let mut gold = 300;
    let mut stock = Stock::default();
    stock.add(id("ring"));
    stock.weapons.push(copy("sword", 20));
    let mut s = ShopSession::new(&shop, &t, &mut gold, &mut stock);
    assert_eq!(s.sell_item(&id("ring")), Err(ShopError::DoesNotBuy));
    assert_eq!(s.sell_weapon(0), Err(ShopError::DoesNotBuy));
    assert_eq!(
        s.buy(&id("sword")),
        Err(ShopError::NotSoldHere(id("sword")))
    );
    assert_eq!(s.repair_weapon(1), Err(ShopError::NoItem));
    assert_eq!(s.repair_weapon(0), Ok(250));
    assert_eq!(
        s.repair_weapon(0),
        Err(ShopError::NothingToRepair(id("sword")))
    );
    assert_eq!(gold, 50);
    assert_eq!(stock.weapons, [copy("sword", 40)]);
    assert_eq!(stock.count(&id("ring")), 1);
}

/// Every item in `stock`, weapons included.
fn held(stock: &Stock) -> u32 {
    stock.items.values().sum::<u32>() + u32::try_from(stock.weapons.len()).unwrap()
}

#[derive(Debug, Clone)]
enum Op {
    Buy(&'static str),
    SellItem(&'static str),
    SellWeapon(usize),
    Repair(usize),
}

fn arb_op() -> impl Strategy<Value = Op> {
    let names = || prop::sample::select(vec!["sword", "bow", "vest", "ring", "potion", "ghost"]);
    prop_oneof![
        names().prop_map(Op::Buy),
        names().prop_map(Op::SellItem),
        (0usize..4).prop_map(Op::SellWeapon),
        (0usize..4).prop_map(Op::Repair),
    ]
}

proptest! {
    #[test]
    fn random_shopping_conserves_gold_and_items(
        start in 0u32..5000,
        kind in prop::sample::select(vec![ShopKind::Armoury, ShopKind::Vendor, ShopKind::Blacksmith]),
        worn in prop::collection::vec(0u32..=40, 0..3),
        ops in prop::collection::vec(arb_op(), 0..40),
    ) {
        let t = items();
        let shop = Shop {
            kind,
            stock: if kind == ShopKind::Blacksmith {
                vec![]
            } else {
                ["sword", "bow", "vest", "ring", "potion"].map(id).to_vec()
            },
        };
        let mut gold = start;
        let mut stock = Stock {
            weapons: worn.iter().map(|&l| copy("sword", l)).collect(),
            ..Stock::default()
        };
        let initial = held(&stock);
        let (mut bought, mut sold, mut spent, mut earned) = (0u32, 0u32, 0u64, 0u64);
        let mut s = ShopSession::new(&shop, &t, &mut gold, &mut stock);
        for op in ops {
            let before = (s.gold(), s.stock().clone());
            let result = match op {
                Op::Buy(i) => s.buy(&id(i)).map(|p| { bought += 1; spent += u64::from(p); }),
                Op::SellItem(i) => s.sell_item(&id(i)).map(|p| { sold += 1; earned += u64::from(p); }),
                Op::SellWeapon(i) => s.sell_weapon(i).map(|p| { sold += 1; earned += u64::from(p); }),
                Op::Repair(i) => s.repair_weapon(i).map(|p| spent += u64::from(p)),
            };
            if result.is_err() {
                prop_assert_eq!((s.gold(), s.stock().clone()), before);
            }
            prop_assert_eq!(u64::from(s.gold()) + spent, u64::from(start) + earned);
            prop_assert_eq!(held(s.stock()) + sold, initial + bought);
        }
    }
}
