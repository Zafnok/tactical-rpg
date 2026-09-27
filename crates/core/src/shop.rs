//! Gold, shops and chests: buying, selling and repairing, in battle (the
//! `Shop` action, [`crate::battle`]) and between chapters ([`ShopSession`]).
//!
//! Source: `docs/design/weapons-and-items.md`, "Money and shops" (0003).
//!
//! # Rules
//!
//! - **Gold** is party-wide and never negative (`u32`). Gains saturate.
//! - **Shops** ([`Shop`]) have a kind and a stock list:
//!   - an **Armoury** sells the weapons, armour and accessories on its list;
//!   - a **Vendor** sells the consumables on its list;
//!   - a **Blacksmith** repairs weapons (its list is empty).
//!
//!   Armouries and vendors **buy** any item the party has, for half its
//!   price rounded down ([`sell_price`]). Blacksmiths don't buy.
//! - **Repair** ([`repair_cost`]) brings a weapon back to full durability
//!   for `ceil(price / 2 × missing / durability)` gold, where `missing` is
//!   the durability it lost. A weapon at full durability can't be repaired.
//! - Every function here checks before it changes anything: on `Err`, the
//!   gold and the item are unchanged.
//! - **Chests** hold [`Loot`]: gold or one item.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::item::{ItemDef, ItemId, ItemTable, Stock, WeaponDef, WeaponInstance};

/// Party gold.
pub type Gold = u32;

/// What a shop does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ShopKind {
    /// Sells weapons, armour and accessories; buys anything.
    Armoury,
    /// Sells consumables; buys anything.
    Vendor,
    /// Repairs weapons.
    Blacksmith,
}

impl ShopKind {
    /// Whether this kind of shop may have `item` on its list.
    pub fn stocks(self, item: &ItemDef) -> bool {
        match self {
            ShopKind::Armoury => !matches!(item, ItemDef::Consumable(_)),
            ShopKind::Vendor => matches!(item, ItemDef::Consumable(_)),
            ShopKind::Blacksmith => false,
        }
    }

    /// Whether this kind of shop buys items from the party.
    pub fn buys(self) -> bool {
        self != ShopKind::Blacksmith
    }
}

/// A shop: on a map tile, or in a town between chapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shop {
    /// What it does.
    pub kind: ShopKind,
    /// What it sells, in menu order.
    pub stock: Vec<ItemId>,
}

/// What a chest holds (or, later, a village gives).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Loot {
    /// This much gold.
    Gold(Gold),
    /// One item (weapons at full durability).
    Item(ItemId),
}

/// Why a shop transaction was refused. Nothing changed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShopError {
    /// The party can't pay.
    NotEnoughGold {
        /// The price.
        cost: Gold,
        /// The party's gold.
        gold: Gold,
    },
    /// This shop doesn't sell the item.
    NotSoldHere(ItemId),
    /// This shop doesn't buy items (a blacksmith).
    DoesNotBuy,
    /// This shop doesn't repair (not a blacksmith).
    DoesNotRepair,
    /// The weapon is at full durability.
    NothingToRepair(ItemId),
    /// The item to sell or repair isn't there.
    NoItem,
    /// The item isn't a weapon, so it can't be repaired.
    NotAWeapon(ItemId),
    /// The item id is not in the item table.
    UnknownItem(ItemId),
}

impl fmt::Display for ShopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShopError::NotEnoughGold { cost, gold } => {
                write!(f, "costs {cost} gold, the party has {gold}")
            }
            ShopError::NotSoldHere(i) => write!(f, "\"{}\" isn't sold here", i.0),
            ShopError::DoesNotBuy => f.write_str("this shop doesn't buy items"),
            ShopError::DoesNotRepair => f.write_str("this shop doesn't repair"),
            ShopError::NothingToRepair(i) => write!(f, "\"{}\" needs no repair", i.0),
            ShopError::NoItem => f.write_str("no such item"),
            ShopError::NotAWeapon(i) => write!(f, "\"{}\" is not a weapon", i.0),
            ShopError::UnknownItem(i) => write!(f, "unknown item \"{}\"", i.0),
        }
    }
}

impl std::error::Error for ShopError {}

/// The price at which a shop buys an item from the party: half its price,
/// rounded down.
pub fn sell_price(item: &ItemDef) -> Gold {
    item.price() / 2
}

/// The gold to bring `copy` of `weapon` back to full durability:
/// `ceil(price / 2 × missing / durability)`, in integer maths. 0 when
/// nothing is missing.
pub fn repair_cost(weapon: &WeaponDef, copy: &WeaponInstance) -> Gold {
    let durability = u64::from(weapon.durability.max(1));
    let missing = u64::from(weapon.durability.saturating_sub(copy.durability_left));
    let cost = (u64::from(weapon.price) * missing).div_ceil(2 * durability);
    Gold::try_from(cost).unwrap_or(Gold::MAX)
}

/// Takes `cost` from `gold`, or refuses if there isn't enough.
pub fn pay(gold: &mut Gold, cost: Gold) -> Result<(), ShopError> {
    *gold = gold
        .checked_sub(cost)
        .ok_or(ShopError::NotEnoughGold { cost, gold: *gold })?;
    Ok(())
}

/// Buys `item` from `shop`: pays its price and returns it. The caller puts
/// the item where it goes.
pub fn buy(
    shop: &Shop,
    items: &ItemTable,
    gold: &mut Gold,
    item: &ItemId,
) -> Result<Gold, ShopError> {
    if !shop.stock.contains(item) {
        return Err(ShopError::NotSoldHere(item.clone()));
    }
    let def = items
        .get(item)
        .ok_or_else(|| ShopError::UnknownItem(item.clone()))?;
    if !shop.kind.stocks(def) {
        return Err(ShopError::NotSoldHere(item.clone()));
    }
    let price = def.price();
    pay(gold, price)?;
    Ok(price)
}

/// Sells `item` to `shop`: adds its sell price to `gold` and returns it.
/// The caller removes the item.
pub fn sell(
    shop: &Shop,
    items: &ItemTable,
    gold: &mut Gold,
    item: &ItemId,
) -> Result<Gold, ShopError> {
    if !shop.kind.buys() {
        return Err(ShopError::DoesNotBuy);
    }
    let def = items
        .get(item)
        .ok_or_else(|| ShopError::UnknownItem(item.clone()))?;
    let price = sell_price(def);
    *gold = gold.saturating_add(price);
    Ok(price)
}

/// Repairs `copy` at `shop` to full durability, paying for it. Returns the
/// cost.
pub fn repair(
    shop: &Shop,
    items: &ItemTable,
    gold: &mut Gold,
    copy: &mut WeaponInstance,
) -> Result<Gold, ShopError> {
    if shop.kind != ShopKind::Blacksmith {
        return Err(ShopError::DoesNotRepair);
    }
    let def = match items.get(&copy.def) {
        None => return Err(ShopError::UnknownItem(copy.def.clone())),
        Some(ItemDef::Weapon(w)) => w,
        Some(_) => return Err(ShopError::NotAWeapon(copy.def.clone())),
    };
    if copy.durability_left >= def.durability {
        return Err(ShopError::NothingToRepair(copy.def.clone()));
    }
    let cost = repair_cost(def, copy);
    pay(gold, cost)?;
    copy.durability_left = def.durability;
    Ok(cost)
}

/// Adds one `item` to `stock`: weapons as a new copy at full durability.
/// Refuses unknown items (nothing changes).
pub fn add_to_stock(stock: &mut Stock, items: &ItemTable, item: &ItemId) -> Result<(), ShopError> {
    match items.get(item) {
        None => Err(ShopError::UnknownItem(item.clone())),
        Some(ItemDef::Weapon(_)) => {
            stock.weapons.extend(items.new_weapon(item));
            Ok(())
        }
        Some(_) => {
            stock.add(item.clone());
            Ok(())
        }
    }
}

/// A shop visit outside battle (between chapters, or in a town node):
/// buying into and selling from the party's stock with the campaign's gold.
/// Uses the same rules as the in-battle `Shop` action.
#[derive(Debug)]
pub struct ShopSession<'a> {
    shop: &'a Shop,
    items: &'a ItemTable,
    gold: &'a mut Gold,
    stock: &'a mut Stock,
}

impl<'a> ShopSession<'a> {
    /// Opens `shop` for a party with `gold` and `stock`.
    pub fn new(
        shop: &'a Shop,
        items: &'a ItemTable,
        gold: &'a mut Gold,
        stock: &'a mut Stock,
    ) -> Self {
        Self {
            shop,
            items,
            gold,
            stock,
        }
    }

    /// The shop.
    pub fn shop(&self) -> &Shop {
        self.shop
    }

    /// The party's gold.
    pub fn gold(&self) -> Gold {
        *self.gold
    }

    /// The party's stock.
    pub fn stock(&self) -> &Stock {
        self.stock
    }

    /// Buys one `item` into the stock. Returns the price paid.
    pub fn buy(&mut self, item: &ItemId) -> Result<Gold, ShopError> {
        let mut gold = *self.gold;
        let price = buy(self.shop, self.items, &mut gold, item)?;
        add_to_stock(self.stock, self.items, item)?;
        *self.gold = gold;
        Ok(price)
    }

    /// Sells one `item` (not a weapon) from the stock. Returns the gold got.
    pub fn sell_item(&mut self, item: &ItemId) -> Result<Gold, ShopError> {
        if self.stock.count(item) == 0 {
            return Err(ShopError::NoItem);
        }
        let price = sell(self.shop, self.items, self.gold, item)?;
        self.stock.take(item);
        Ok(price)
    }

    /// Sells the stock's weapon copy `index`. Returns the gold got.
    pub fn sell_weapon(&mut self, index: usize) -> Result<Gold, ShopError> {
        let copy = self.stock.weapons.get(index).ok_or(ShopError::NoItem)?;
        let price = sell(self.shop, self.items, self.gold, &copy.def)?;
        self.stock.weapons.remove(index);
        Ok(price)
    }

    /// Repairs the stock's weapon copy `index`. Returns the cost.
    pub fn repair_weapon(&mut self, index: usize) -> Result<Gold, ShopError> {
        let copy = self.stock.weapons.get_mut(index).ok_or(ShopError::NoItem)?;
        repair(self.shop, self.items, self.gold, copy)
    }
}

#[cfg(test)]
mod tests;
