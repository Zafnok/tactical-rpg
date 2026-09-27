//! Tests of the `Shop` and `Open` actions. The map (see [`shop_map`]) has an
//! armoury at (1,0), a vendor at (2,0), a blacksmith at (3,0), chests at
//! (0,1) (300 gold), (1,1) (a tonic) and (2,1) (an iron sword), and, for
//! the enemies at (7,0) and (7,2), an armoury at (6,0) and a chest at (6,2).

use super::*;
use crate::map::TileFeature;
use crate::shop::{Loot, Shop, ShopError, ShopKind};

fn shop_of(kind: ShopKind, stock: &[&str]) -> TileFeature {
    TileFeature::Shop(Shop {
        kind,
        stock: stock.iter().map(|s| item(s)).collect(),
    })
}

fn armoury() -> TileFeature {
    shop_of(ShopKind::Armoury, &["iron", "leather", "plate", "charm"])
}

fn shop_map() -> BattleMap {
    let mut m = map(&OPEN);
    m.features = [
        (p(1, 0), armoury()),
        (p(2, 0), shop_of(ShopKind::Vendor, &["tonic", "potion"])),
        (p(3, 0), shop_of(ShopKind::Blacksmith, &[])),
        (p(0, 1), TileFeature::Chest(Loot::Gold(300))),
        (p(1, 1), TileFeature::Chest(Loot::Item(item("tonic")))),
        (p(2, 1), TileFeature::Chest(Loot::Item(item("iron")))),
        (p(6, 0), armoury()),
        (p(6, 2), TileFeature::Chest(Loot::Gold(1))),
    ]
    .into_iter()
    .collect();
    m
}

fn shop_battle(units: Vec<Unit>, gold: Gold) -> BattleState {
    start(BattleSetup {
        map: shop_map(),
        gold,
        ..setup(units)
    })
}

/// The standard cast with `gold`.
fn shopping(gold: Gold) -> BattleState {
    shop_battle(cast(), gold)
}

fn txns(list: &[ShopTxn]) -> UnitAction {
    UnitAction::Shop {
        txns: list.to_vec(),
    }
}

fn buy(id: &str) -> ShopTxn {
    ShopTxn::Buy { item: item(id) }
}

fn sell(from: SellFrom) -> ShopTxn {
    ShopTxn::Sell { from }
}

fn gold(g: Gold) -> Event {
    Event::GoldChanged { gold: g }
}

fn bought(unit: u32, id: &str, price: Gold, to: Destination) -> Event {
    Event::Bought {
        unit: UnitId(unit),
        item: item(id),
        price,
        to,
    }
}

fn moved(unit: u32, path: &[Pos]) -> Event {
    Event::UnitMoved {
        unit: UnitId(unit),
        path: path.to_vec(),
    }
}

fn acted(unit: u32) -> Event {
    Event::UnitActed { unit: UnitId(unit) }
}

fn copy(id: &str, left: u32) -> WeaponInstance {
    WeaponInstance {
        def: item(id),
        durability_left: left,
    }
}

fn u1(s: &BattleState) -> &Unit {
    s.unit(UnitId(1)).unwrap()
}

#[test]
fn bought_weapons_fill_free_slots_then_the_stock() {
    let mut s = shopping(2000);
    let events = act(
        &mut s,
        1,
        p(1, 0),
        txns(&[buy("iron"), buy("iron"), buy("iron")]),
    );
    assert_eq!(
        events,
        [
            moved(1, &[p(0, 0), p(1, 0)]),
            bought(1, "iron", 400, Destination::WeaponSlot(1)),
            gold(1600),
            bought(1, "iron", 400, Destination::WeaponSlot(2)),
            gold(1200),
            bought(1, "iron", 400, Destination::Stock),
            gold(800),
            acted(1),
        ]
    );
    let u = u1(&s);
    assert_eq!(u.pos, p(1, 0));
    assert!(u.acted);
    assert_eq!(u.loadout.weapon(1), Some(&copy("iron", 20)));
    assert_eq!(u.loadout.weapon(2), Some(&copy("iron", 20)));
    // Still the first weapon equipped.
    assert_eq!(u.loadout.equipped, Some(0));
    assert_eq!(s.gold(), 800);
    assert_eq!(s.stock().weapons, [copy("iron", 20)]);
}

#[test]
fn bought_gear_and_consumables_go_where_they_fit() {
    let mut s = shopping(2000);
    let events = act(
        &mut s,
        1,
        p(1, 0),
        txns(&[buy("plate"), buy("leather"), buy("leather"), buy("charm")]),
    );
    assert_eq!(
        events[1..],
        [
            // The fighter can't wear Medium armour.
            bought(1, "plate", 500, Destination::Stock),
            gold(1500),
            bought(1, "leather", 200, Destination::Armour),
            gold(1300),
            bought(1, "leather", 200, Destination::Stock),
            gold(1100),
            bought(1, "charm", 100, Destination::Accessory),
            gold(1000),
            acted(1),
        ]
    );
    assert_eq!(u1(&s).loadout.armour, Some(item("leather")));
    assert_eq!(u1(&s).loadout.accessory, Some(item("charm")));
    assert_eq!(s.stock().count(&item("plate")), 1);
    assert_eq!(s.stock().count(&item("leather")), 1);
    // A second charm goes to the stock.
    walk(&mut s, 2);
    act(&mut s, 1, p(1, 0), txns(&[buy("charm")]));
    assert_eq!(s.stock().count(&item("charm")), 1);
    assert_eq!(s.gold(), 900);
}

#[test]
fn bought_consumables_go_into_the_pack_past_its_cap() {
    let mut s = shopping(200);
    let events = act(
        &mut s,
        1,
        p(2, 0),
        txns(&[buy("tonic"), buy("tonic"), buy("potion")]),
    );
    assert_eq!(
        events[1..],
        [
            bought(1, "tonic", 50, Destination::Pack),
            gold(150),
            bought(1, "tonic", 50, Destination::Pack),
            gold(100),
            bought(1, "potion", 0, Destination::Pack),
            gold(100),
            acted(1),
        ]
    );
    assert_eq!(s.pack().cap, 3);
    assert_eq!(
        s.pack().items,
        ["potion", "elixir", "tonic", "tonic", "potion"].map(item)
    );
}

#[test]
fn an_unarmed_buyer_equips_the_weapon_it_can_wield() {
    let units = vec![
        carrying(lord(1, p(0, 0)), &[]),
        unit(3, Faction::Enemy, p(7, 0)),
    ];
    let mut s = shop_battle(units, 400);
    let events = act(&mut s, 1, p(1, 0), txns(&[buy("iron")]));
    assert_eq!(
        events[1..],
        [
            bought(1, "iron", 400, Destination::WeaponSlot(0)),
            Event::Equipped {
                unit: UnitId(1),
                slot: 0
            },
            gold(0),
            acted(1),
        ]
    );
    assert_eq!(u1(&s).loadout.equipped, Some(0));
}

#[test]
fn selling_gives_half_price_and_reequips() {
    let mut l = carrying(lord(1, p(0, 0)), &[item("iron"), weapon(1, 1, 3)]);
    l.loadout.armour = Some(item("leather"));
    l.loadout.accessory = Some(item("charm"));
    let units = vec![l, unit(3, Faction::Enemy, p(7, 0))];
    let mut s = shop_battle(units, 10);
    let events = act(
        &mut s,
        1,
        p(2, 0),
        txns(&[
            sell(SellFrom::Weapon(0)),
            sell(SellFrom::Armour),
            sell(SellFrom::Accessory),
            sell(SellFrom::Pack(1)),
        ]),
    );
    let sold = |id: &str, price| Event::Sold {
        unit: UnitId(1),
        item: item(id),
        price,
    };
    assert_eq!(
        events[1..],
        [
            sold("iron", 200),
            // The equipped weapon was sold: the next one it can wield.
            Event::Equipped {
                unit: UnitId(1),
                slot: 1
            },
            gold(210),
            sold("leather", 100),
            gold(310),
            sold("charm", 50),
            gold(360),
            sold("elixir", 0),
            gold(360),
            acted(1),
        ]
    );
    let u = u1(&s);
    assert_eq!(u.loadout.weapon(0), None);
    assert_eq!(u.loadout.equipped, Some(1));
    assert_eq!((&u.loadout.armour, &u.loadout.accessory), (&None, &None));
    assert_eq!(s.pack().items, [item("potion")]);
}

#[test]
fn selling_the_last_weapon_leaves_nothing_equipped() {
    let units = vec![
        carrying(lord(1, p(0, 0)), &[item("iron"), item("master_sword")]),
        unit(3, Faction::Enemy, p(7, 0)),
    ];
    let mut s = shop_battle(units, 0);
    // The master sword (rank S) can't be wielded.
    let events = act(&mut s, 1, p(1, 0), txns(&[sell(SellFrom::Weapon(0))]));
    assert!(!events.iter().any(|e| matches!(e, Event::Equipped { .. })));
    assert_eq!(u1(&s).loadout.equipped, None);
    assert_eq!(s.gold(), 200);
}

#[test]
fn blacksmith_repairs_to_full() {
    let mut l = carrying(lord(1, p(0, 0)), &[weapon(1, 1, 3), item("iron")]);
    l.loadout.weapons[1] = Some(copy("iron", 5));
    let units = vec![l, unit(3, Faction::Enemy, p(7, 0))];
    let mut s = shop_battle(units, 150);
    let events = act(&mut s, 1, p(3, 0), txns(&[ShopTxn::Repair { slot: 1 }]));
    assert_eq!(
        events[1..],
        [
            // ceil(400 / 2 × 15 / 20) = 150.
            Event::Repaired {
                unit: UnitId(1),
                slot: 1,
                item: item("iron"),
                cost: 150
            },
            gold(0),
            acted(1),
        ]
    );
    assert_eq!(u1(&s).loadout.weapon(1), Some(&copy("iron", 20)));
}

#[test]
fn shop_errors_change_nothing() {
    let mut s = shopping(500);
    let shop_err = |txn, error| CommandError::Shop { txn, error };
    let cases = [
        (p(0, 1), txns(&[buy("iron")]), CommandError::NoShop(p(0, 1))),
        (p(0, 0), txns(&[buy("iron")]), CommandError::NoShop(p(0, 0))),
        (p(1, 0), txns(&[]), CommandError::NoTransactions),
        // The first buy would work: none is applied.
        (
            p(1, 0),
            txns(&[buy("iron"), buy("iron")]),
            shop_err(
                1,
                ShopError::NotEnoughGold {
                    cost: 400,
                    gold: 100,
                },
            ),
        ),
        (
            p(1, 0),
            txns(&[buy("tonic")]),
            shop_err(0, ShopError::NotSoldHere(item("tonic"))),
        ),
        (
            p(1, 0),
            txns(&[sell(SellFrom::Weapon(1))]),
            shop_err(0, ShopError::NoItem),
        ),
        (
            p(1, 0),
            txns(&[sell(SellFrom::Weapon(7))]),
            shop_err(0, ShopError::NoItem),
        ),
        (
            p(1, 0),
            txns(&[sell(SellFrom::Armour)]),
            shop_err(0, ShopError::NoItem),
        ),
        (
            p(1, 0),
            txns(&[sell(SellFrom::Accessory)]),
            shop_err(0, ShopError::NoItem),
        ),
        (
            p(1, 0),
            txns(&[sell(SellFrom::Pack(2))]),
            shop_err(0, ShopError::NoItem),
        ),
        (
            p(3, 0),
            txns(&[sell(SellFrom::Pack(0))]),
            shop_err(0, ShopError::DoesNotBuy),
        ),
        (
            p(1, 0),
            txns(&[ShopTxn::Repair { slot: 0 }]),
            shop_err(0, ShopError::DoesNotRepair),
        ),
        (
            p(3, 0),
            txns(&[ShopTxn::Repair { slot: 0 }]),
            shop_err(0, ShopError::NothingToRepair(weapon(1, 1, 3))),
        ),
        (
            p(3, 0),
            txns(&[ShopTxn::Repair { slot: 1 }]),
            shop_err(0, ShopError::NoItem),
        ),
    ];
    for (dest, action, err) in cases {
        refused_act(&mut s, 1, dest, action, err);
    }
}

#[test]
fn only_player_units_shop_and_open() {
    let mut s = shopping(500);
    walk(&mut s, 1);
    refused_act(
        &mut s,
        3,
        p(6, 0),
        txns(&[buy("iron")]),
        CommandError::PlayerOnly(UnitId(3)),
    );
    refused_act(
        &mut s,
        4,
        p(6, 2),
        UnitAction::Open,
        CommandError::PlayerOnly(UnitId(4)),
    );
}

#[test]
fn chests_give_gold_consumables_and_equipment_once() {
    let mut s = shopping(5);
    let events = act(&mut s, 1, p(0, 1), UnitAction::Open);
    assert_eq!(
        events,
        [
            moved(1, &[p(0, 0), p(0, 1)]),
            Event::ChestOpened {
                unit: UnitId(1),
                pos: p(0, 1),
                loot: Loot::Gold(300)
            },
            gold(305),
            acted(1),
        ]
    );
    assert!(s.is_opened(p(0, 1)));
    assert!(!s.is_opened(p(1, 1)));
    let events = act(&mut s, 2, p(1, 1), UnitAction::Open);
    assert_eq!(
        events[1..],
        [
            Event::ChestOpened {
                unit: UnitId(2),
                pos: p(1, 1),
                loot: Loot::Item(item("tonic"))
            },
            acted(2),
        ]
    );
    assert_eq!(s.pack().items.last(), Some(&item("tonic")));
    walk(&mut s, 2);
    // Staying on the opened chest.
    refused_act(
        &mut s,
        1,
        p(0, 1),
        UnitAction::Open,
        CommandError::AlreadyOpened(p(0, 1)),
    );
    act(&mut s, 1, p(2, 1), UnitAction::Open);
    assert_eq!(s.stock().weapons, [copy("iron", 20)]);
    assert_eq!(s.gold(), 305);
    // Non-weapon equipment goes to the stock as a count.
    let mut m = shop_map();
    m.features
        .insert(p(0, 1), TileFeature::Chest(Loot::Item(item("charm"))));
    let mut s = start(BattleSetup {
        map: m,
        ..setup(cast())
    });
    act(&mut s, 1, p(0, 1), UnitAction::Open);
    assert_eq!(s.stock().count(&item("charm")), 1);
}

#[test]
fn open_errors_change_nothing() {
    let mut m = shop_map();
    m.features
        .insert(p(0, 1), TileFeature::Chest(Loot::Item(item("ghost"))));
    let mut s = start(BattleSetup {
        map: m,
        ..setup(cast())
    });
    refused_act(
        &mut s,
        1,
        p(1, 0),
        UnitAction::Open,
        CommandError::NoChest(p(1, 0)),
    );
    refused_act(
        &mut s,
        1,
        p(0, 0),
        UnitAction::Open,
        CommandError::NoChest(p(0, 0)),
    );
    refused_act(
        &mut s,
        1,
        p(0, 1),
        UnitAction::Open,
        CommandError::UnknownItem(item("ghost")),
    );
}

#[test]
fn gold_saturates() {
    let mut s = shopping(Gold::MAX - 1);
    act(&mut s, 1, p(0, 1), UnitAction::Open);
    assert_eq!(s.gold(), Gold::MAX);
}

#[test]
fn gold_stock_and_chests_are_saved() {
    let mut s = shopping(2000);
    act(&mut s, 1, p(1, 0), txns(&[buy("plate")]));
    act(&mut s, 2, p(0, 1), UnitAction::Open);
    let saved = ron::to_string(&s).unwrap();
    let mut loaded: BattleState = ron::from_str(&saved).unwrap();
    loaded.restore_tables(
        Arc::new(s.terrain().clone()),
        Arc::new(s.classes().clone()),
        Arc::new(s.items().clone()),
    );
    assert_eq!(loaded, s);
    assert_eq!(loaded.gold(), 1800);
    assert!(loaded.is_opened(p(0, 1)));
    assert_eq!(loaded.stock().count(&item("plate")), 1);
    assert_eq!(
        loaded.map().shop(p(1, 0)).map(|s| s.kind),
        Some(ShopKind::Armoury)
    );
}

#[test]
fn map_feature_lookups() {
    let m = shop_map();
    assert!(m.shop(p(1, 0)).is_some());
    assert_eq!(m.chest(p(1, 0)), None);
    assert_eq!(m.chest(p(0, 1)), Some(&Loot::Gold(300)));
    assert_eq!(m.shop(p(0, 1)), None);
    assert_eq!(m.shop(p(5, 5)), None);
    assert_eq!(m.chest(p(5, 5)), None);
}

#[test]
fn shop_errors_display() {
    let cases = [
        (
            CommandError::PlayerOnly(UnitId(3)),
            "unit 3 is not a player unit",
        ),
        (CommandError::NoShop(p(1, 2)), "no shop at (1, 2)"),
        (
            CommandError::NoTransactions,
            "nothing bought, sold or repaired",
        ),
        (
            CommandError::Shop {
                txn: 2,
                error: ShopError::NoItem,
            },
            "transaction 2: no such item",
        ),
        (CommandError::NoChest(p(1, 2)), "no chest at (1, 2)"),
        (
            CommandError::AlreadyOpened(p(1, 2)),
            "the chest at (1, 2) is already open",
        ),
    ];
    for (e, text) in cases {
        assert_eq!(e.to_string(), text);
    }
}
