//! Preparations (ticket 0408): changing a [`BattleSetup`] before its battle
//! starts. The player moves gear between the deployed units' loadouts and
//! the party stock, and fills the battle pack from the stock.
//!
//! # Rules
//!
//! Source: `docs/design/weapons-and-items.md` (Loadout, Battle pack).
//!
//! - **Who.** Only the setup's player units (the deployed ones) change.
//! - **Gear** ([`BattleSetup::gear_from_stock`], [`BattleSetup::gear_to_stock`]):
//!   a unit has the weapon slots of its class, one armour and one accessory
//!   ([`BattleSetup::gear_slots`]). A stock item goes in a slot of its kind,
//!   and what was there goes back to the stock (a weapon takes the place in
//!   the stock of the one taken, and keeps its durability).
//! - **Usable only** ([`BattleSetup::unusable`]): a weapon needs a class that
//!   uses its kind and the unit's rank; armour needs a class that wears its
//!   weight. Anything else can't be put in a slot (the loadout rules in
//!   [`crate::item`] would let a unit carry a weapon it can't wield;
//!   Preparations doesn't).
//! - **Equipped.** After a change, a unit whose equipped weapon is gone (or
//!   that had nothing equipped) gets its
//!   [default equip](crate::unit::Unit::default_equip).
//! - **Pack** ([`BattleSetup::pack_from_stock`], [`BattleSetup::pack_to_stock`]):
//!   only consumables, only from the stock, never more than the pack's cap.
//!   A battle with Preparations starts with an empty pack: the player brings
//!   only what they own (Nick, 0408).
//! - **Attack speed** ([`BattleSetup::attack_speed`]): the combat formula
//!   (`Spd + rank speed − burden`, gear-adjusted) for the weapon in a slot,
//!   without skill bonuses.

use std::fmt;
use std::sync::Arc;

use crate::battle::BattleSetup;
use crate::class::{ArmourWeight, ClassDef};
use crate::item::{Equipped, ItemDef, ItemId, ItemTable, WEAPON_SLOTS};
use crate::spell::SpellTable;
use crate::stats::StatValue;
use crate::terrain::TerrainRules;
use crate::unit::{Faction, Unit, UnitId};
use crate::weapon::{WeaponKind, WeaponRank};

/// One slot of a unit's loadout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GearSlot {
    /// This weapon slot.
    Weapon(usize),
    /// The armour.
    Armour,
    /// The accessory.
    Accessory,
}

/// One thing in the party stock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockItem {
    /// The weapon copy at this index of [`Stock::weapons`](crate::item::Stock).
    Weapon(usize),
    /// One of this counted item (armour, accessory, consumable).
    Item(ItemId),
}

/// Why a unit can't use an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unusable {
    /// Its class doesn't use this kind of weapon.
    Kind(WeaponKind),
    /// The weapon needs this rank, and the unit's is lower.
    Rank(WeaponRank),
    /// Its class doesn't wear armour of this weight.
    Armour(ArmourWeight),
}

/// Why a Preparations change was refused. Nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepError {
    /// No player unit with that id (or its class is unknown).
    NoUnit,
    /// The unit's class doesn't have that weapon slot.
    NoSlot,
    /// The stock doesn't hold that item.
    NotInStock,
    /// The item doesn't go in that slot (e.g. armour in a weapon slot).
    WrongSlot,
    /// The unit can't use the item.
    Unusable(Unusable),
    /// The slot is empty.
    EmptySlot,
    /// The pack already holds as many items as its cap.
    PackFull,
    /// Only consumables go in the pack.
    NotConsumable,
    /// The pack doesn't hold that item.
    NotInPack,
}

impl fmt::Display for PrepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            PrepError::NoUnit => "no such player unit",
            PrepError::NoSlot => "the unit's class doesn't have that slot",
            PrepError::NotInStock => "the stock doesn't hold that item",
            PrepError::WrongSlot => "the item doesn't go in that slot",
            PrepError::Unusable(_) => "the unit can't use that item",
            PrepError::EmptySlot => "the slot is empty",
            PrepError::PackFull => "the pack is full",
            PrepError::NotConsumable => "only consumables go in the pack",
            PrepError::NotInPack => "the pack doesn't hold that item",
        })
    }
}

impl std::error::Error for PrepError {}

/// Weapon slots a unit of `class` has.
fn weapon_slots(class: &ClassDef) -> usize {
    usize::from(class.weapon_slots).min(WEAPON_SLOTS)
}

/// Why `unit`, of `class`, can't use `item`, if it can't.
fn unusable(unit: &Unit, class: &ClassDef, items: &ItemTable, item: &ItemId) -> Option<Unusable> {
    match items.get(item)? {
        ItemDef::Weapon(w) if class.weapon(w.kind).is_none() => Some(Unusable::Kind(w.kind)),
        ItemDef::Weapon(w) if unit.rank(w.kind) < w.rank => Some(Unusable::Rank(w.rank)),
        ItemDef::Armour(a) if !class.armour.contains(&a.weight_class) => {
            Some(Unusable::Armour(a.weight_class))
        }
        _ => None,
    }
}

/// Gives `unit` its default equip unless it has a spell or a weapon it can
/// wield equipped.
fn refit(unit: &mut Unit, class: &ClassDef, items: &ItemTable, spells: &SpellTable) {
    let ok = match &unit.loadout.equipped {
        Some(Equipped::Weapon(slot)) => unit.usable_weapon(*slot, class, items).is_some(),
        Some(Equipped::Spell(_)) => true,
        None => false,
    };
    if !ok {
        unit.loadout.equipped = unit.default_equip(class, items, spells);
    }
}

/// Preparations: see the module docs.
impl BattleSetup {
    /// The player units: the ones Preparations may change, in unit order.
    pub fn player_units(&self) -> impl Iterator<Item = &Unit> {
        self.units.iter().filter(|u| u.faction == Faction::Player)
    }

    /// Index in `units` of player unit `unit`.
    fn player_index(&self, unit: UnitId) -> Result<usize, PrepError> {
        self.units
            .iter()
            .position(|u| u.id == unit && u.faction == Faction::Player)
            .ok_or(PrepError::NoUnit)
    }

    /// The loadout slots of player unit `unit`: its class's weapon slots,
    /// then armour and accessory. Empty if there is no such unit.
    pub fn gear_slots(&self, unit: UnitId) -> Vec<GearSlot> {
        let class = self
            .player_index(unit)
            .ok()
            .and_then(|i| self.classes.get(&self.units[i].class));
        let Some(class) = class else {
            return Vec::new();
        };
        (0..weapon_slots(class))
            .map(GearSlot::Weapon)
            .chain([GearSlot::Armour, GearSlot::Accessory])
            .collect()
    }

    /// Why player unit `unit` can't use `item`; `None` if it can (or if
    /// either is unknown).
    pub fn unusable(&self, unit: UnitId, item: &ItemId) -> Option<Unusable> {
        let u = &self.units[self.player_index(unit).ok()?];
        let class = self.classes.get(&u.class)?;
        unusable(u, class, &self.items, item)
    }

    /// Moves `item` from the stock into `slot` of player unit `unit`; what
    /// the slot held goes to the stock.
    pub fn gear_from_stock(
        &mut self,
        unit: UnitId,
        slot: GearSlot,
        item: &StockItem,
    ) -> Result<(), PrepError> {
        let (classes, items) = (Arc::clone(&self.classes), Arc::clone(&self.items));
        let index = self.player_index(unit)?;
        let class = classes
            .get(&self.units[index].class)
            .ok_or(PrepError::NoUnit)?;
        if matches!(slot, GearSlot::Weapon(s) if s >= weapon_slots(class)) {
            return Err(PrepError::NoSlot);
        }
        let id = match item {
            StockItem::Weapon(i) => self.stock.weapons.get(*i).map(|w| w.def.clone()),
            StockItem::Item(id) => (self.stock.count(id) > 0).then(|| id.clone()),
        }
        .ok_or(PrepError::NotInStock)?;
        let fits = match (slot, item) {
            (GearSlot::Weapon(_), StockItem::Weapon(_)) => items.weapon(&id).is_some(),
            (GearSlot::Armour, StockItem::Item(_)) => items.armour(&id).is_some(),
            (GearSlot::Accessory, StockItem::Item(_)) => items.accessory(&id).is_some(),
            _ => false,
        };
        if !fits {
            return Err(PrepError::WrongSlot);
        }
        let u = &mut self.units[index];
        if let Some(why) = unusable(u, class, &items, &id) {
            return Err(PrepError::Unusable(why));
        }
        match (slot, item) {
            (GearSlot::Weapon(s), StockItem::Weapon(i)) => {
                let copy = self.stock.weapons.remove(*i);
                if let Some(old) = u.loadout.weapons[s].replace(copy) {
                    self.stock.weapons.insert(*i, old);
                }
            }
            (GearSlot::Armour | GearSlot::Accessory, _) => {
                self.stock.take(&id);
                let worn = if slot == GearSlot::Armour {
                    &mut u.loadout.armour
                } else {
                    &mut u.loadout.accessory
                };
                if let Some(old) = worn.replace(id) {
                    self.stock.add(old);
                }
            }
            (GearSlot::Weapon(_), StockItem::Item(_)) => {}
        }
        refit(u, class, &items, &self.spells);
        Ok(())
    }

    /// Moves what `slot` of player unit `unit` holds to the stock.
    pub fn gear_to_stock(&mut self, unit: UnitId, slot: GearSlot) -> Result<(), PrepError> {
        let (classes, items) = (Arc::clone(&self.classes), Arc::clone(&self.items));
        let index = self.player_index(unit)?;
        let u = &mut self.units[index];
        let class = classes.get(&u.class).ok_or(PrepError::NoUnit)?;
        match slot {
            GearSlot::Weapon(s) => {
                let held = u.loadout.weapons.get_mut(s).and_then(Option::take);
                self.stock.weapons.push(held.ok_or(PrepError::EmptySlot)?);
            }
            GearSlot::Armour => {
                self.stock
                    .add(u.loadout.armour.take().ok_or(PrepError::EmptySlot)?);
            }
            GearSlot::Accessory => {
                self.stock
                    .add(u.loadout.accessory.take().ok_or(PrepError::EmptySlot)?);
            }
        }
        refit(u, class, &items, &self.spells);
        Ok(())
    }

    /// Moves one consumable `item` from the stock into the pack.
    pub fn pack_from_stock(&mut self, item: &ItemId) -> Result<(), PrepError> {
        if self.items.consumable(item).is_none() {
            return Err(PrepError::NotConsumable);
        }
        if self.stock.count(item) == 0 {
            return Err(PrepError::NotInStock);
        }
        if self.pack.items.len() >= self.pack.cap {
            return Err(PrepError::PackFull);
        }
        self.stock.take(item);
        self.pack.items.push(item.clone());
        Ok(())
    }

    /// Moves one `item` (the last one packed) from the pack to the stock.
    pub fn pack_to_stock(&mut self, item: &ItemId) -> Result<(), PrepError> {
        let at = self.pack.items.iter().rposition(|i| i == item);
        let item = self.pack.items.remove(at.ok_or(PrepError::NotInPack)?);
        self.stock.add(item);
        Ok(())
    }

    /// Player unit `unit`'s attack speed with the weapon in `slot` in hand
    /// (`None`: what it has equipped). An empty slot, or a weapon it can't
    /// wield, counts as no weapon. `None` if there is no such unit.
    pub fn attack_speed(&self, unit: UnitId, slot: Option<usize>) -> Option<StatValue> {
        let u = &self.units[self.player_index(unit).ok()?];
        let class = self.classes.get(&u.class)?;
        // Attack speed doesn't read the terrain.
        let ground = TerrainRules {
            name: String::new(),
            move_cost: Vec::new(),
            defense: 0,
            avoid: 0,
            heal_percent: 0,
        };
        let with = slot.map(Equipped::Weapon);
        let input = u.combat_input(
            class,
            &self.classes,
            &self.items,
            &self.spells,
            with.as_ref(),
            &ground,
        );
        Some(input.attack_speed(&self.items.combat_rules()))
    }
}

#[cfg(test)]
mod tests;
