//! Preparations (ticket 0408): changing a [`BattleSetup`] before its battle
//! starts. The player moves gear between the deployed units' loadouts and
//! the party stock, and fills the battle pack from the stock.
//!
//! # Rules
//!
//! Source: `docs/design/weapons-and-items.md` (Loadout, Battle pack).
//!
//! - **Who** ([`Preparations`]). Every unit of the army: the setup's player
//!   units (the deployed ones) and the bench (roster units left out of this
//!   battle). Gear is traded between any of them (Nick, 0408: a benched
//!   archer's better bow can go to the one who fights), through the stock
//!   or straight from one unit to another
//!   ([`Preparations::gear_from_unit`]): the taker's slot gets the item, and
//!   what it held goes back to the giver's slot, or to the stock if the
//!   giver can't use it. Once the battle starts there is no trading.
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

use crate::battle::{BattleSetup, BattleState};
use crate::class::{ArmourWeight, ClassDef, ClassTable};
use crate::item::{Equipped, ItemDef, ItemId, ItemTable, Stock, WEAPON_SLOTS, WeaponInstance};
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
    /// No such unit of the army (or its class is unknown).
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

/// The tables the gear rules read.
#[derive(Clone, Copy)]
struct Tables<'a> {
    classes: &'a ClassTable,
    items: &'a ItemTable,
    spells: &'a SpellTable,
}

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
fn refit(unit: &mut Unit, class: &ClassDef, t: Tables<'_>) {
    let ok = match &unit.loadout.equipped {
        Some(Equipped::Weapon(slot)) => unit.usable_weapon(*slot, class, t.items).is_some(),
        Some(Equipped::Spell(_)) => true,
        None => false,
    };
    if !ok {
        unit.loadout.equipped = unit.default_equip(class, t.items, t.spells);
    }
}

/// The loadout slots of `unit`: its class's weapon slots, then armour and
/// accessory. Empty if its class is unknown.
fn slots_of(unit: &Unit, classes: &ClassTable) -> Vec<GearSlot> {
    let Some(class) = classes.get(&unit.class) else {
        return Vec::new();
    };
    (0..weapon_slots(class))
        .map(GearSlot::Weapon)
        .chain([GearSlot::Armour, GearSlot::Accessory])
        .collect()
}

/// Moves `item` from `stock` into `slot` of `unit`; what the slot held goes
/// to `stock`.
fn from_stock(
    unit: &mut Unit,
    stock: &mut Stock,
    slot: GearSlot,
    item: &StockItem,
    t: Tables<'_>,
) -> Result<(), PrepError> {
    let class = t.classes.get(&unit.class).ok_or(PrepError::NoUnit)?;
    if matches!(slot, GearSlot::Weapon(s) if s >= weapon_slots(class)) {
        return Err(PrepError::NoSlot);
    }
    let id = match item {
        StockItem::Weapon(i) => stock.weapons.get(*i).map(|w| w.def.clone()),
        StockItem::Item(id) => (stock.count(id) > 0).then(|| id.clone()),
    }
    .ok_or(PrepError::NotInStock)?;
    let fits = match (slot, item) {
        (GearSlot::Weapon(_), StockItem::Weapon(_)) => t.items.weapon(&id).is_some(),
        (GearSlot::Armour, StockItem::Item(_)) => t.items.armour(&id).is_some(),
        (GearSlot::Accessory, StockItem::Item(_)) => t.items.accessory(&id).is_some(),
        _ => false,
    };
    if !fits {
        return Err(PrepError::WrongSlot);
    }
    if let Some(why) = unusable(unit, class, t.items, &id) {
        return Err(PrepError::Unusable(why));
    }
    match (slot, item) {
        (GearSlot::Weapon(s), StockItem::Weapon(i)) => {
            let copy = stock.weapons.remove(*i);
            if let Some(old) = unit.loadout.weapons[s].replace(copy) {
                stock.weapons.insert(*i, old);
            }
        }
        (GearSlot::Armour | GearSlot::Accessory, _) => {
            stock.take(&id);
            let worn = if slot == GearSlot::Armour {
                &mut unit.loadout.armour
            } else {
                &mut unit.loadout.accessory
            };
            if let Some(old) = worn.replace(id) {
                stock.add(old);
            }
        }
        (GearSlot::Weapon(_), StockItem::Item(_)) => {}
    }
    refit(unit, class, t);
    Ok(())
}

/// Moves what `slot` of `unit` holds to `stock`.
fn to_stock(
    unit: &mut Unit,
    stock: &mut Stock,
    slot: GearSlot,
    t: Tables<'_>,
) -> Result<(), PrepError> {
    let class = t.classes.get(&unit.class).ok_or(PrepError::NoUnit)?;
    match slot {
        GearSlot::Weapon(s) => {
            let held = unit.loadout.weapons.get_mut(s).and_then(Option::take);
            stock.weapons.push(held.ok_or(PrepError::EmptySlot)?);
        }
        GearSlot::Armour => {
            stock.add(unit.loadout.armour.take().ok_or(PrepError::EmptySlot)?);
        }
        GearSlot::Accessory => {
            stock.add(unit.loadout.accessory.take().ok_or(PrepError::EmptySlot)?);
        }
    }
    refit(unit, class, t);
    Ok(())
}

/// What a loadout slot holds.
enum Held {
    Weapon(WeaponInstance),
    Item(ItemId),
}

impl Held {
    fn id(&self) -> &ItemId {
        match self {
            Held::Weapon(copy) => &copy.def,
            Held::Item(id) => id,
        }
    }
}

/// Empties `slot` of `unit`, giving what it held.
fn take_held(unit: &mut Unit, slot: GearSlot) -> Option<Held> {
    match slot {
        GearSlot::Weapon(s) => {
            let held = unit.loadout.weapons.get_mut(s).and_then(Option::take);
            held.map(Held::Weapon)
        }
        GearSlot::Armour => unit.loadout.armour.take().map(Held::Item),
        GearSlot::Accessory => unit.loadout.accessory.take().map(Held::Item),
    }
}

/// Puts `held` in `slot` of `unit` (a slot of its kind that the unit has).
fn put_held(unit: &mut Unit, slot: GearSlot, held: Held) {
    match (slot, held) {
        (GearSlot::Weapon(s), Held::Weapon(copy)) => {
            if let Some(place) = unit.loadout.weapons.get_mut(s) {
                *place = Some(copy);
            }
        }
        (GearSlot::Armour, Held::Item(id)) => unit.loadout.armour = Some(id),
        (GearSlot::Accessory, Held::Item(id)) => unit.loadout.accessory = Some(id),
        _ => {}
    }
}

/// Whether two slots hold the same kind of gear.
fn same_kind(a: GearSlot, b: GearSlot) -> bool {
    matches!(
        (a, b),
        (GearSlot::Weapon(_), GearSlot::Weapon(_))
            | (GearSlot::Armour, GearSlot::Armour)
            | (GearSlot::Accessory, GearSlot::Accessory)
    )
}

/// `unit`'s attack speed with the weapon in `slot` in hand (`None`: what it
/// has equipped). `None` if its class is unknown.
fn speed(unit: &Unit, slot: Option<usize>, t: Tables<'_>) -> Option<StatValue> {
    let class = t.classes.get(&unit.class)?;
    // Attack speed doesn't read the terrain.
    let ground = TerrainRules {
        name: String::new(),
        move_cost: Vec::new(),
        defense: 0,
        avoid: 0,
        heal_percent: 0,
    };
    let with = slot.map(Equipped::Weapon);
    let input = unit.combat_input(class, t.classes, t.items, t.spells, with.as_ref(), &ground);
    Some(input.attack_speed(&t.items.combat_rules()))
}

/// Preparations: see the module docs.
impl BattleSetup {
    /// The player units: the deployed ones, in unit order.
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
        self.player_index(unit)
            .map_or_else(|_| Vec::new(), |i| slots_of(&self.units[i], &self.classes))
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
        let index = self.player_index(unit)?;
        let t = Tables {
            classes: &self.classes,
            items: &self.items,
            spells: &self.spells,
        };
        from_stock(&mut self.units[index], &mut self.stock, slot, item, t)
    }

    /// Moves what `slot` of player unit `unit` holds to the stock.
    pub fn gear_to_stock(&mut self, unit: UnitId, slot: GearSlot) -> Result<(), PrepError> {
        let index = self.player_index(unit)?;
        let t = Tables {
            classes: &self.classes,
            items: &self.items,
            spells: &self.spells,
        };
        to_stock(&mut self.units[index], &mut self.stock, slot, t)
    }

    /// Makes this setup the one battle `start` began with, as Preparations
    /// left it: every player unit takes the loadout of `start`'s unit with
    /// its id, and the stock and the pack are `start`'s. For continuing a
    /// suspended battle, whose first state is all that is saved of its
    /// Preparations.
    pub fn prepared_as(&mut self, start: &BattleState) {
        for unit in &mut self.units {
            let began = start.units().iter().find(|u| u.id == unit.id);
            if let (Faction::Player, Some(began)) = (unit.faction, began) {
                unit.loadout = began.loadout.clone();
            }
        }
        self.stock = start.stock().clone();
        self.pack = start.pack().clone();
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
        let t = Tables {
            classes: &self.classes,
            items: &self.items,
            spells: &self.spells,
        };
        speed(u, slot, t)
    }
}

/// A unit of the army on the Preparations screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrepUnit {
    /// A unit going into the battle, by its id in the setup.
    Deployed(UnitId),
    /// A unit left out of the battle, by its place on the bench.
    Benched(usize),
}

/// A battle being prepared: its setup, and the army's units left out of
/// it, whose gear can be traded all the same (see the module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preparations {
    /// The battle, with the deployed units, the stock and the pack.
    pub setup: BattleSetup,
    /// The roster's units that aren't in the battle, in roster order.
    pub bench: Vec<Unit>,
}

impl Preparations {
    /// Every unit of the army: the deployed ones in unit order, then the
    /// bench.
    pub fn units(&self) -> Vec<(PrepUnit, &Unit)> {
        let deployed = self
            .setup
            .player_units()
            .map(|u| (PrepUnit::Deployed(u.id), u));
        let benched = self.bench.iter().enumerate();
        deployed
            .chain(benched.map(|(i, u)| (PrepUnit::Benched(i), u)))
            .collect()
    }

    /// The unit `who`.
    pub fn unit(&self, who: PrepUnit) -> Option<&Unit> {
        match who {
            PrepUnit::Deployed(id) => self.setup.player_units().find(|u| u.id == id),
            PrepUnit::Benched(i) => self.bench.get(i),
        }
    }

    fn unit_mut(&mut self, who: PrepUnit) -> Option<&mut Unit> {
        match who {
            PrepUnit::Deployed(id) => {
                let units = &mut self.setup.units;
                units
                    .iter_mut()
                    .find(|u| u.id == id && u.faction == Faction::Player)
            }
            PrepUnit::Benched(i) => self.bench.get_mut(i),
        }
    }

    /// Moves what `from_slot` of unit `from` holds straight into `slot` of
    /// `who` (a slot of the same kind, on another unit). What `who`'s slot
    /// held goes back into `from_slot`, or to the stock if `from` can't use
    /// it.
    pub fn gear_from_unit(
        &mut self,
        who: PrepUnit,
        slot: GearSlot,
        from: PrepUnit,
        from_slot: GearSlot,
    ) -> Result<(), PrepError> {
        if who == from || !same_kind(slot, from_slot) {
            return Err(PrepError::WrongSlot);
        }
        let classes = Arc::clone(&self.setup.classes);
        let items = Arc::clone(&self.setup.items);
        let spells = Arc::clone(&self.setup.spells);
        let t = Tables {
            classes: &classes,
            items: &items,
            spells: &spells,
        };
        let class_of = |u: Option<&Unit>| u.and_then(|u| classes.get(&u.class));
        let taker_class = class_of(self.unit(who)).ok_or(PrepError::NoUnit)?;
        let giver_class = class_of(self.unit(from)).ok_or(PrepError::NoUnit)?;
        if matches!(slot, GearSlot::Weapon(s) if s >= weapon_slots(taker_class)) {
            return Err(PrepError::NoSlot);
        }
        // On a copy, so that a refusal changes nothing.
        let mut after = self.clone();
        let giver = after.unit_mut(from).ok_or(PrepError::NoUnit)?;
        let item = take_held(giver, from_slot).ok_or(PrepError::EmptySlot)?;
        let taker = after.unit_mut(who).ok_or(PrepError::NoUnit)?;
        if let Some(why) = unusable(taker, taker_class, &items, item.id()) {
            return Err(PrepError::Unusable(why));
        }
        let back = take_held(taker, slot);
        put_held(taker, slot, item);
        refit(taker, taker_class, t);
        let giver = after.unit_mut(from).ok_or(PrepError::NoUnit)?;
        let back = match back {
            Some(held) if unusable(giver, giver_class, &items, held.id()).is_none() => {
                put_held(giver, from_slot, held);
                None
            }
            other => other,
        };
        refit(giver, giver_class, t);
        match back {
            Some(Held::Weapon(copy)) => after.setup.stock.weapons.push(copy),
            Some(Held::Item(id)) => after.setup.stock.add(id),
            None => {}
        }
        *self = after;
        Ok(())
    }

    fn tables(&self) -> Tables<'_> {
        Tables {
            classes: &self.setup.classes,
            items: &self.setup.items,
            spells: &self.setup.spells,
        }
    }

    /// The loadout slots of `who` ([`BattleSetup::gear_slots`]).
    pub fn gear_slots(&self, who: PrepUnit) -> Vec<GearSlot> {
        self.unit(who)
            .map_or_else(Vec::new, |u| slots_of(u, &self.setup.classes))
    }

    /// Why `who` can't use `item` ([`BattleSetup::unusable`]).
    pub fn unusable(&self, who: PrepUnit, item: &ItemId) -> Option<Unusable> {
        let u = self.unit(who)?;
        let class = self.setup.classes.get(&u.class)?;
        unusable(u, class, &self.setup.items, item)
    }

    /// Moves `item` from the stock into `slot` of `who`
    /// ([`BattleSetup::gear_from_stock`]).
    pub fn gear_from_stock(
        &mut self,
        who: PrepUnit,
        slot: GearSlot,
        item: &StockItem,
    ) -> Result<(), PrepError> {
        match who {
            PrepUnit::Deployed(id) => self.setup.gear_from_stock(id, slot, item),
            PrepUnit::Benched(i) => {
                let t = Tables {
                    classes: &self.setup.classes,
                    items: &self.setup.items,
                    spells: &self.setup.spells,
                };
                let unit = self.bench.get_mut(i).ok_or(PrepError::NoUnit)?;
                from_stock(unit, &mut self.setup.stock, slot, item, t)
            }
        }
    }

    /// Moves what `slot` of `who` holds to the stock
    /// ([`BattleSetup::gear_to_stock`]).
    pub fn gear_to_stock(&mut self, who: PrepUnit, slot: GearSlot) -> Result<(), PrepError> {
        match who {
            PrepUnit::Deployed(id) => self.setup.gear_to_stock(id, slot),
            PrepUnit::Benched(i) => {
                let t = Tables {
                    classes: &self.setup.classes,
                    items: &self.setup.items,
                    spells: &self.setup.spells,
                };
                let unit = self.bench.get_mut(i).ok_or(PrepError::NoUnit)?;
                to_stock(unit, &mut self.setup.stock, slot, t)
            }
        }
    }

    /// `who`'s attack speed with the weapon in `slot` in hand
    /// ([`BattleSetup::attack_speed`]).
    pub fn attack_speed(&self, who: PrepUnit, slot: Option<usize>) -> Option<StatValue> {
        speed(self.unit(who)?, slot, self.tables())
    }
}

#[cfg(test)]
mod tests;
