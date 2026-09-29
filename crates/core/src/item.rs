//! Items: weapons, armour, accessories and consumables; a unit's loadout, the
//! shared battle pack, the party stock, weapon ranks and weapon EXP.
//!
//! Source: `docs/design/weapons-and-items.md` (0003). The numbers live in
//! `assets/data/items.ron`, loaded by `trpg-content` into an [`ItemTable`].
//!
//! # Rules
//!
//! - **Loadout** ([`Loadout`]): up to [`WEAPON_SLOTS`] weapons (only the first
//!   `class.weapon_slots` slots may hold one), one armour, one accessory. One
//!   weapon or learned attack spell is *equipped* ([`Equipped`]): it counters,
//!   and a weapon's trait applies. A unit may carry a weapon it can't wield,
//!   but never equip it.
//! - **Default equip** ([`Unit::default_equip`]): the first weapon the unit can
//!   wield, else its first learned attack spell (id order). Used when a battle
//!   starts with nothing equipped, when the equipped weapon is sold, and when
//!   a promotion stows it ([`Unit::fit_weapon_slots`]).
//! - **Fewer weapon slots** ([`Unit::fit_weapon_slots`], called by promotion,
//!   0603): weapons in slots the new class doesn't have go to the party
//!   stock. A 0-slot class (tier-3+ magic) carries no weapons at all.
//! - **Wielding**: the class can use the weapon's kind, and the unit's rank in
//!   that kind is at least the weapon's rank. A unit with no rank recorded in
//!   a kind counts as rank E.
//! - **Gear-adjusted stats** ([`Unit::effective_stats`]): permanent stats plus
//!   the armour and accessory bonuses. Gear may push a stat up but never above
//!   the hard ceiling (a stat already above the ceiling
//!   isn't lowered). Max HP stays the permanent `stats.hp` (no gear gives HP).
//! - **Durability**: normal attacks cost none. [`WeaponInstance::spend_durability`]
//!   (Combat Arts and active skills) lowers it, never below 0. At 0 the weapon is broken:
//!   it still fights with the penalties in [`WeaponRules`].
//! - **Weapon EXP** ([`WeaponRules::weapon_exp`]): after a combat, a unit that
//!   struck gains `base + dealt / 5` in the kind it fought with, where
//!   `base` is 2 if a strike hit, else 1 (doubled with a Combat Art), and
//!   `dealt` is the HP its strikes removed. Its rank rises to the highest
//!   threshold reached, never above the class's max rank. EXP counts from
//!   the current rank's threshold (a unit given rank D by its class starts at
//!   30) and stops at the class's max rank's threshold.
//! - **Battle pack** ([`BattlePack`]): the player side's shared consumables.
//!   The cap limits only what is brought in; items gained in battle go in
//!   anyway.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::art::ArtId;
use crate::battle::Event;
use crate::class::{ArmourWeight, ClassDef, ClassTable, UnitTag};
use crate::combat::{
    CombatMods, CombatRules, CombatantInput, DamageType, WeaponStats, WeaponTrait,
};
use crate::magic::Element;
use crate::movement::AttackRange;
use crate::spell::{SpellDef, SpellId, SpellState, SpellTable};
use crate::stats::{StatKind, StatValue, Stats};
use crate::terrain::TerrainRules;
use crate::unit::Unit;
use crate::weapon::{WeaponKind, WeaponRank};

/// String id of an item, e.g. `"iron_sword"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl ItemId {
    /// An id from a string.
    pub fn new(id: &str) -> Self {
        Self(id.to_owned())
    }
}

/// Weapon slots in every loadout.
pub const WEAPON_SLOTS: usize = 3;

/// A weapon's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponDef {
    /// Display name.
    pub name: String,
    /// Weapon kind; its trait comes from [`ItemTable::traits`].
    pub kind: WeaponKind,
    /// Lowest rank that can wield it.
    pub rank: WeaponRank,
    /// Might.
    pub might: StatValue,
    /// Base hit.
    pub hit: StatValue,
    /// Base crit.
    pub crit: StatValue,
    /// Weight (feeds the burden).
    pub weight: StatValue,
    /// Smallest range, in tiles.
    pub min_range: u32,
    /// Largest range, in tiles.
    pub max_range: u32,
    /// Str vs Def or Mag vs Res.
    pub damage_type: DamageType,
    /// Maximum durability.
    pub durability: u32,
    /// Extra `(tag, multiplier)` pairs beyond the kind's trait.
    pub effective: Vec<(UnitTag, u8)>,
    /// The weapon's own Combat Arts (weapon arts, `rank: None`), usable by
    /// anyone wielding it ([`crate::art`]). Empty for every Chapter 1 weapon.
    pub arts: Vec<ArtId>,
    /// Buy price in gold.
    pub price: u32,
}

/// An armour's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmourDef {
    /// Display name.
    pub name: String,
    /// Light, Medium or Heavy (classes limit which they wear).
    pub weight_class: ArmourWeight,
    /// Stat bonuses.
    pub bonus: Stats,
    /// Weight (feeds the burden).
    pub weight: StatValue,
    /// Buy price in gold.
    pub price: u32,
}

/// An accessory's data. Any unit can wear any accessory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessoryDef {
    /// Display name.
    pub name: String,
    /// Stat bonuses.
    pub bonus: Stats,
    /// Buy price in gold.
    pub price: u32,
}

/// What a consumable does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsumableEffect {
    /// Restore this much HP (never above max).
    Heal(StatValue),
    /// Restore all HP.
    HealFull,
}

/// A consumable's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumableDef {
    /// Display name.
    pub name: String,
    /// Its effect.
    pub effect: ConsumableEffect,
    /// Buy price in gold.
    pub price: u32,
}

/// Any item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemDef {
    /// A weapon (loadout weapon slots).
    Weapon(WeaponDef),
    /// Armour (the loadout armour slot).
    Armour(ArmourDef),
    /// An accessory (the loadout accessory slot).
    Accessory(AccessoryDef),
    /// A consumable (the battle pack).
    Consumable(ConsumableDef),
}

impl ItemDef {
    /// Display name.
    pub fn name(&self) -> &str {
        match self {
            ItemDef::Weapon(d) => &d.name,
            ItemDef::Armour(d) => &d.name,
            ItemDef::Accessory(d) => &d.name,
            ItemDef::Consumable(d) => &d.name,
        }
    }

    /// Buy price in gold.
    pub fn price(&self) -> u32 {
        match self {
            ItemDef::Weapon(d) => d.price,
            ItemDef::Armour(d) => d.price,
            ItemDef::Accessory(d) => d.price,
            ItemDef::Consumable(d) => d.price,
        }
    }
}

/// Item numbers that aren't per item (`items.ron`). [`Default`] is the
/// design's values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponRules {
    /// Attack speed bonus per rank, E to S.
    pub rank_speed: [StatValue; 6],
    /// Total weapon EXP for ranks D, C, B, A, S.
    pub rank_exp: [u32; 5],
    /// Broken weapon might divisor.
    pub broken_might_divisor: StatValue,
    /// Broken weapon hit penalty.
    pub broken_hit_penalty: StatValue,
    /// Weapon EXP base when a strike hit.
    pub exp_hit: u32,
    /// Weapon EXP base when every strike missed.
    pub exp_miss: u32,
    /// Damage dealt per extra weapon EXP point.
    pub exp_damage_divisor: u32,
    /// Base multiplier when a Combat Art was used.
    pub exp_art_multiplier: u32,
    /// Pack cap of a battle whose chapter doesn't set one.
    pub default_pack_cap: usize,
}

impl Default for WeaponRules {
    fn default() -> Self {
        Self {
            rank_speed: [0, 0, 1, 2, 3, 4],
            rank_exp: [30, 70, 120, 180, 250],
            broken_might_divisor: 2,
            broken_hit_penalty: 20,
            exp_hit: 2,
            exp_miss: 1,
            exp_damage_divisor: 5,
            exp_art_multiplier: 2,
            default_pack_cap: 6,
        }
    }
}

const RANKS: [WeaponRank; 6] = [
    WeaponRank::E,
    WeaponRank::D,
    WeaponRank::C,
    WeaponRank::B,
    WeaponRank::A,
    WeaponRank::S,
];

impl WeaponRules {
    /// Total weapon EXP at which `rank` is reached (0 for E).
    pub fn threshold(&self, rank: WeaponRank) -> u32 {
        match rank as usize {
            0 => 0,
            i => self.rank_exp[i - 1],
        }
    }

    /// The highest rank whose threshold `exp` reaches.
    pub fn rank_for(&self, exp: u32) -> WeaponRank {
        RANKS
            .into_iter()
            .rev()
            .find(|&r| exp >= self.threshold(r))
            .unwrap_or(WeaponRank::E)
    }

    /// Weapon EXP for one combat: `struck` strikes, `hits` of them hit,
    /// removing `dealt` HP in total. 0 if it didn't strike.
    pub fn weapon_exp(&self, struck: usize, hits: usize, dealt: StatValue, used_art: bool) -> u32 {
        if struck == 0 {
            return 0;
        }
        let base = if hits > 0 {
            self.exp_hit
        } else {
            self.exp_miss
        };
        let base = if used_art {
            base * self.exp_art_multiplier
        } else {
            base
        };
        let dealt = u32::try_from(dealt).unwrap_or(0);
        base + dealt / self.exp_damage_divisor.max(1)
    }
}

/// Every item and the weapon rules.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemTable {
    /// Items by id.
    pub items: BTreeMap<ItemId, ItemDef>,
    /// Each weapon kind's trait. A kind missing here has no trait.
    pub traits: BTreeMap<WeaponKind, WeaponTrait>,
    /// Rank, broken-weapon, weapon EXP and pack numbers.
    pub rules: WeaponRules,
}

impl ItemTable {
    /// The item `id`.
    pub fn get(&self, id: &ItemId) -> Option<&ItemDef> {
        self.items.get(id)
    }

    /// The weapon `id`, if it is one.
    pub fn weapon(&self, id: &ItemId) -> Option<&WeaponDef> {
        match self.items.get(id)? {
            ItemDef::Weapon(w) => Some(w),
            _ => None,
        }
    }

    /// The armour `id`, if it is one.
    pub fn armour(&self, id: &ItemId) -> Option<&ArmourDef> {
        match self.items.get(id)? {
            ItemDef::Armour(a) => Some(a),
            _ => None,
        }
    }

    /// The accessory `id`, if it is one.
    pub fn accessory(&self, id: &ItemId) -> Option<&AccessoryDef> {
        match self.items.get(id)? {
            ItemDef::Accessory(a) => Some(a),
            _ => None,
        }
    }

    /// The consumable `id`, if it is one.
    pub fn consumable(&self, id: &ItemId) -> Option<&ConsumableDef> {
        match self.items.get(id)? {
            ItemDef::Consumable(c) => Some(c),
            _ => None,
        }
    }

    /// The trait of every weapon of `kind`.
    pub fn trait_of(&self, kind: WeaponKind) -> WeaponTrait {
        self.traits.get(&kind).copied().unwrap_or(WeaponTrait::None)
    }

    /// The combat numbers of a weapon copy, or `None` if it isn't a weapon.
    pub fn weapon_stats(&self, weapon: &WeaponInstance) -> Option<WeaponStats> {
        let def = self.weapon(&weapon.def)?;
        Some(WeaponStats {
            kind: Some(def.kind),
            trait_: self.trait_of(def.kind),
            might: def.might,
            hit: def.hit,
            crit: def.crit,
            weight: def.weight,
            min_range: def.min_range,
            max_range: def.max_range,
            damage_type: def.damage_type,
            effective: def.effective.clone(),
            broken: weapon.is_broken(),
            element: Element::None,
        })
    }

    /// The combat rules with this table's rank speed and broken penalties.
    pub fn combat_rules(&self) -> CombatRules {
        CombatRules {
            rank_speed: self.rules.rank_speed,
            broken_might_divisor: self.rules.broken_might_divisor,
            broken_hit_penalty: self.rules.broken_hit_penalty,
            ..CombatRules::default()
        }
    }

    /// A new copy of weapon `id` at full durability.
    pub fn new_weapon(&self, id: &ItemId) -> Option<WeaponInstance> {
        let def = self.weapon(id)?;
        Some(WeaponInstance {
            def: id.clone(),
            durability_left: def.durability,
        })
    }
}

/// One owned copy of a weapon.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeaponInstance {
    /// The weapon.
    pub def: ItemId,
    /// Durability left; 0 = broken.
    pub durability_left: u32,
}

impl WeaponInstance {
    /// Whether it is broken (no durability left).
    pub fn is_broken(&self) -> bool {
        self.durability_left == 0
    }

    /// Spends `amount` durability (never below 0). Returns `true` if this
    /// broke it (it wasn't broken before and is now).
    pub fn spend_durability(&mut self, amount: u32) -> bool {
        let was_broken = self.is_broken();
        self.durability_left = self.durability_left.saturating_sub(amount);
        !was_broken && self.is_broken()
    }
}

/// A unit's equipped attack: it attacks by default and counters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Equipped {
    /// The weapon in this loadout slot.
    Weapon(usize),
    /// This learned attack spell.
    Spell(SpellId),
}

/// A unit's gear. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct Loadout {
    /// Weapon slots; only the first `class.weapon_slots` may be filled.
    pub weapons: [Option<WeaponInstance>; WEAPON_SLOTS],
    /// The equipped weapon or attack spell.
    pub equipped: Option<Equipped>,
    /// Worn armour.
    pub armour: Option<ItemId>,
    /// Worn accessory.
    pub accessory: Option<ItemId>,
}

impl Loadout {
    /// The weapon in `slot`, if any.
    pub fn weapon(&self, slot: usize) -> Option<&WeaponInstance> {
        self.weapons.get(slot)?.as_ref()
    }

    /// The equipped weapon's slot, if a weapon is equipped.
    pub fn equipped_slot(&self) -> Option<usize> {
        match self.equipped {
            Some(Equipped::Weapon(slot)) => Some(slot),
            _ => None,
        }
    }

    /// The equipped attack spell, if a spell is equipped.
    pub fn equipped_spell(&self) -> Option<&SpellId> {
        match &self.equipped {
            Some(Equipped::Spell(spell)) => Some(spell),
            _ => None,
        }
    }

    /// The equipped weapon, if any.
    pub fn equipped_weapon(&self) -> Option<&WeaponInstance> {
        self.weapon(self.equipped_slot()?)
    }

    /// Number of weapons carried.
    pub fn weapon_count(&self) -> usize {
        self.weapons.iter().flatten().count()
    }
}

/// A loadout as written in data: item ids, weapons at full durability.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoadoutDef {
    /// Weapons, in slot order (at most the class's weapon slots).
    pub weapons: Vec<ItemId>,
    /// Armour.
    pub armour: Option<ItemId>,
    /// Accessory.
    pub accessory: Option<ItemId>,
}

/// Why a loadout is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadoutError {
    /// The unit's class is not in the class table.
    UnknownClass(String),
    /// The item id is not in the item table.
    UnknownItem(ItemId),
    /// The item is in a slot of the wrong kind (e.g. a potion as armour).
    WrongSlot(ItemId),
    /// A weapon sits in a slot beyond the class's weapon slots.
    TooManyWeapons {
        /// Weapons carried (or the highest filled slot + 1).
        count: usize,
        /// The class's weapon slots.
        slots: usize,
    },
    /// The class can't wear this armour's weight.
    ArmourNotAllowed(ItemId),
    /// The equipped slot is empty.
    EquippedEmpty(usize),
    /// The equipped weapon can't be wielded (class kind or rank).
    CannotWield(ItemId),
    /// The equipped spell hasn't been learned.
    SpellNotLearned(SpellId),
}

impl fmt::Display for LoadoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadoutError::UnknownClass(c) => write!(f, "unknown class \"{c}\""),
            LoadoutError::UnknownItem(i) => write!(f, "unknown item \"{}\"", i.0),
            LoadoutError::WrongSlot(i) => write!(f, "\"{}\" doesn't go in that slot", i.0),
            LoadoutError::TooManyWeapons { count, slots } => {
                write!(f, "{count} weapons, but the class has {slots} weapon slots")
            }
            LoadoutError::ArmourNotAllowed(i) => {
                write!(f, "the class can't wear \"{}\"", i.0)
            }
            LoadoutError::EquippedEmpty(s) => write!(f, "equipped slot {s} is empty"),
            LoadoutError::CannotWield(i) => write!(f, "can't wield \"{}\"", i.0),
            LoadoutError::SpellNotLearned(s) => {
                write!(f, "the equipped spell \"{}\" isn't learned", s.0)
            }
        }
    }
}

impl std::error::Error for LoadoutError {}

/// The player side's shared consumables for one battle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct BattlePack {
    /// The consumables, in pack order.
    pub items: Vec<ItemId>,
    /// How many could be brought in (items gained in battle may exceed it).
    pub cap: usize,
}

impl BattlePack {
    /// A pack of `items` with `cap`, or `None` if `items` exceeds the cap.
    pub fn bring(items: Vec<ItemId>, cap: usize) -> Option<BattlePack> {
        (items.len() <= cap).then_some(BattlePack { items, cap })
    }

    /// Adds an item gained in battle (ignores the cap).
    pub fn gain(&mut self, item: ItemId) {
        self.items.push(item);
    }
}

/// The party's items outside battle: everything not in a loadout.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct Stock {
    /// Armour, accessories and consumables, with counts.
    pub items: BTreeMap<ItemId, u32>,
    /// Weapon copies (each keeps its durability).
    pub weapons: Vec<WeaponInstance>,
}

impl Stock {
    /// Adds one `item` (not a weapon copy).
    pub fn add(&mut self, item: ItemId) {
        *self.items.entry(item).or_insert(0) += 1;
    }

    /// Removes one `item`; `false` (and no change) if there is none.
    pub fn take(&mut self, item: &ItemId) -> bool {
        match self.items.get_mut(item) {
            Some(n) if *n > 1 => {
                *n -= 1;
                true
            }
            Some(_) => {
                self.items.remove(item);
                true
            }
            None => false,
        }
    }

    /// How many of `item` are in stock.
    pub fn count(&self, item: &ItemId) -> u32 {
        self.items.get(item).copied().unwrap_or(0)
    }

    /// Moves `items` from the stock into a pack with `cap`. Nothing changes
    /// if the stock lacks one of them or they exceed the cap.
    pub fn pack(&mut self, items: Vec<ItemId>, cap: usize) -> Option<BattlePack> {
        let mut after = self.clone();
        if !items.iter().all(|i| after.take(i)) {
            return None;
        }
        let pack = BattlePack::bring(items, cap)?;
        *self = after;
        Some(pack)
    }

    /// Returns a pack's unused items to the stock after a battle.
    pub fn unpack(&mut self, pack: BattlePack) {
        for item in pack.items {
            self.add(item);
        }
    }
}

/// A unit's gear-related rules.
impl Unit {
    /// The rank in `kind` (E when none is recorded).
    pub fn rank(&self, kind: WeaponKind) -> WeaponRank {
        self.weapon_ranks
            .get(&kind)
            .copied()
            .unwrap_or(WeaponRank::E)
    }

    /// Whether the unit, in `class`, can wield `weapon`.
    pub fn can_wield(&self, weapon: &WeaponDef, class: &ClassDef) -> bool {
        class.weapon(weapon.kind).is_some() && self.rank(weapon.kind) >= weapon.rank
    }

    /// The weapon in `slot` if the unit can wield it.
    pub fn usable_weapon<'a>(
        &'a self,
        slot: usize,
        class: &ClassDef,
        items: &'a ItemTable,
    ) -> Option<(&'a WeaponInstance, &'a WeaponDef)> {
        let copy = self.loadout.weapon(slot)?;
        let def = items.weapon(&copy.def)?;
        self.can_wield(def, class).then_some((copy, def))
    }

    /// Permanent stats plus armour and accessory bonuses, each stat raised
    /// no higher than `classes.hard_ceilings` (see the module docs).
    pub fn effective_stats(&self, classes: &ClassTable, items: &ItemTable) -> Stats {
        let bonuses = [
            self.loadout
                .armour
                .as_ref()
                .and_then(|a| items.armour(a))
                .map(|a| a.bonus),
            self.loadout
                .accessory
                .as_ref()
                .and_then(|a| items.accessory(a))
                .map(|a| a.bonus),
        ];
        let mut stats = self.stats;
        for kind in StatKind::ALL {
            let base = self.stats.get(kind);
            let bonus: StatValue = bonuses.iter().flatten().map(|b| b.get(kind)).sum();
            let ceiling = base.max(classes.hard_ceilings.get(kind));
            stats.set(kind, base.saturating_add(bonus).min(ceiling));
        }
        stats
    }

    /// Weight of the worn armour (0 without).
    pub fn armour_weight(&self, items: &ItemTable) -> StatValue {
        self.loadout
            .armour
            .as_ref()
            .and_then(|a| items.armour(a))
            .map_or(0, |a| a.weight)
    }

    /// The ranges of every weapon in the loadout the unit can wield, then of
    /// every [castable](Unit::castable_attack) learned attack spell (id
    /// order), without duplicates. Empty if its class is unknown.
    pub fn attack_ranges(
        &self,
        classes: &ClassTable,
        items: &ItemTable,
        spells: &SpellTable,
    ) -> Vec<AttackRange> {
        let Some(class) = classes.get(&self.class) else {
            return Vec::new();
        };
        let mut ranges = Vec::new();
        for slot in 0..WEAPON_SLOTS {
            if let Some((_, def)) = self.usable_weapon(slot, class, items) {
                let range = (def.min_range, def.max_range);
                if !ranges.contains(&range) {
                    ranges.push(range);
                }
            }
        }
        for spell in &self.learned {
            if let Some(def) = self.castable_attack(spell, spells) {
                let range = (def.min_range, def.max_range);
                if !ranges.contains(&range) {
                    ranges.push(range);
                }
            }
        }
        ranges
    }

    /// The unit as the combat maths sees it, on `terrain`, fighting with
    /// `with` (`None`: its equipped attack). Its weapon is `None` if that is
    /// an empty slot, a weapon it can't wield, or a spell it can't cast
    /// ([`Unit::castable_attack`]: unknown, a heal, or no uses left).
    pub fn combat_input<'a>(
        &self,
        class: &ClassDef,
        classes: &ClassTable,
        items: &ItemTable,
        spells: &SpellTable,
        with: Option<&Equipped>,
        terrain: &'a TerrainRules,
    ) -> CombatantInput<'a> {
        let (weapon, weapon_rank) = match with.or(self.loadout.equipped.as_ref()) {
            Some(Equipped::Weapon(slot)) => match self.usable_weapon(*slot, class, items) {
                Some((copy, def)) => (items.weapon_stats(copy), self.rank(def.kind)),
                None => (None, WeaponRank::E),
            },
            Some(Equipped::Spell(spell)) => (
                self.castable_attack(spell, spells)
                    .and_then(SpellDef::weapon_stats),
                WeaponRank::E,
            ),
            None => (None, WeaponRank::E),
        };
        CombatantInput {
            stats: self.effective_stats(classes, items),
            tags: class.tags,
            affinities: class.affinities.clone(),
            weapon,
            weapon_rank,
            armour_weight: self.armour_weight(items),
            terrain,
            mods: CombatMods::default(),
        }
    }

    /// Checks the loadout against the unit's class and `items`.
    pub fn validate_loadout(
        &self,
        classes: &ClassTable,
        items: &ItemTable,
    ) -> Result<(), LoadoutError> {
        let class = classes
            .get(&self.class)
            .ok_or_else(|| LoadoutError::UnknownClass(self.class.0.clone()))?;
        let slots = usize::from(class.weapon_slots).min(WEAPON_SLOTS);
        if let Some(last) = self.loadout.weapons.iter().rposition(Option::is_some)
            && last >= slots
        {
            return Err(LoadoutError::TooManyWeapons {
                count: last + 1,
                slots,
            });
        }
        for copy in self.loadout.weapons.iter().flatten() {
            check_kind(items, &copy.def, |d| matches!(d, ItemDef::Weapon(_)))?;
        }
        if let Some(id) = &self.loadout.armour {
            check_kind(items, id, |d| matches!(d, ItemDef::Armour(_)))?;
            if let Some(a) = items.armour(id)
                && !class.armour.contains(&a.weight_class)
            {
                return Err(LoadoutError::ArmourNotAllowed(id.clone()));
            }
        }
        if let Some(id) = &self.loadout.accessory {
            check_kind(items, id, |d| matches!(d, ItemDef::Accessory(_)))?;
        }
        match &self.loadout.equipped {
            Some(Equipped::Weapon(slot)) => {
                let copy = self
                    .loadout
                    .weapon(*slot)
                    .ok_or(LoadoutError::EquippedEmpty(*slot))?;
                if self.usable_weapon(*slot, class, items).is_none() {
                    return Err(LoadoutError::CannotWield(copy.def.clone()));
                }
            }
            Some(Equipped::Spell(spell)) if !self.learned.contains(spell) => {
                return Err(LoadoutError::SpellNotLearned(spell.clone()));
            }
            Some(Equipped::Spell(_)) | None => {}
        }
        Ok(())
    }

    /// The unit with the loadout `def` (weapons at full durability), the
    /// first weapon it can wield equipped. Fails if the loadout is invalid.
    pub fn with_loadout(
        mut self,
        def: &LoadoutDef,
        classes: &ClassTable,
        items: &ItemTable,
    ) -> Result<Unit, LoadoutError> {
        let mut loadout = Loadout {
            armour: def.armour.clone(),
            accessory: def.accessory.clone(),
            ..Loadout::default()
        };
        if def.weapons.len() > WEAPON_SLOTS {
            let slots = classes
                .get(&self.class)
                .map_or(0, |c| usize::from(c.weapon_slots).min(WEAPON_SLOTS));
            return Err(LoadoutError::TooManyWeapons {
                count: def.weapons.len(),
                slots,
            });
        }
        for (slot, id) in def.weapons.iter().enumerate() {
            let copy = match items.get(id) {
                None => return Err(LoadoutError::UnknownItem(id.clone())),
                Some(_) => items
                    .new_weapon(id)
                    .ok_or_else(|| LoadoutError::WrongSlot(id.clone()))?,
            };
            loadout.weapons[slot] = Some(copy);
        }
        self.loadout = loadout;
        if let Some(class) = classes.get(&self.class) {
            self.loadout.equipped = self.first_usable_weapon(class, items).map(Equipped::Weapon);
        }
        self.validate_loadout(classes, items)?;
        Ok(self)
    }

    /// The first slot holding a weapon the unit can wield.
    fn first_usable_weapon(&self, class: &ClassDef, items: &ItemTable) -> Option<usize> {
        (0..WEAPON_SLOTS).find(|&s| self.usable_weapon(s, class, items).is_some())
    }

    /// What to equip when nothing is (see the module docs): the first weapon
    /// the unit can wield, else its first learned attack spell.
    pub fn default_equip(
        &self,
        class: &ClassDef,
        items: &ItemTable,
        spells: &SpellTable,
    ) -> Option<Equipped> {
        self.first_usable_weapon(class, items)
            .map(Equipped::Weapon)
            .or_else(|| self.first_attack_spell(spells).map(Equipped::Spell))
    }

    /// Readies the unit for a new battle: every learned spell at full uses,
    /// and the [default equip](Unit::default_equip) if nothing is equipped.
    /// Called for every unit (reinforcements too) by
    /// [`BattleState::new`](crate::battle::BattleState::new).
    pub fn prepare_for_battle(
        &mut self,
        classes: &ClassTable,
        items: &ItemTable,
        spells: &SpellTable,
    ) {
        self.spells = SpellState::full(&self.learned, spells);
        if self.loadout.equipped.is_none()
            && let Some(class) = classes.get(&self.class)
        {
            self.loadout.equipped = self.default_equip(class, items, spells);
        }
    }

    /// Moves the weapons in slots `class` (the unit's new class) doesn't
    /// have to `stock`, in slot order, and returns their ids. If the equipped
    /// weapon went (or `class` can't wield it), the
    /// [default equip](Unit::default_equip) replaces it.
    /// Promotion and reclass call this after changing the class (0603).
    pub fn fit_weapon_slots(
        &mut self,
        class: &ClassDef,
        items: &ItemTable,
        spells: &SpellTable,
        stock: &mut Stock,
    ) -> Vec<ItemId> {
        let slots = usize::from(class.weapon_slots).min(WEAPON_SLOTS);
        let mut stowed = Vec::new();
        for slot in slots..WEAPON_SLOTS {
            if let Some(copy) = self.loadout.weapons[slot].take() {
                stowed.push(copy.def.clone());
                stock.weapons.push(copy);
            }
        }
        if self
            .loadout
            .equipped_slot()
            .is_some_and(|s| self.usable_weapon(s, class, items).is_none())
        {
            self.loadout.equipped = self.default_equip(class, items, spells);
        }
        stowed
    }

    /// Spends `amount` durability of the weapon in `slot` (Combat Arts and
    /// class actives). Returns [`Event::ItemBroke`] if that broke it.
    pub fn spend_durability(&mut self, slot: usize, amount: u32) -> Option<Event> {
        let unit = self.id;
        let copy = self.loadout.weapons.get_mut(slot)?.as_mut()?;
        copy.spend_durability(amount).then(|| Event::ItemBroke {
            unit,
            item: copy.def.clone(),
        })
    }

    /// Adds weapon EXP in `kind`, capped at `max`'s threshold, and raises the
    /// rank (never above `max`). Returns the new rank if it rose.
    pub fn gain_weapon_exp(
        &mut self,
        kind: WeaponKind,
        amount: u32,
        max: WeaponRank,
        rules: &WeaponRules,
    ) -> Option<WeaponRank> {
        let rank = self.rank(kind);
        let exp = self.weapon_exp.get(&kind).copied().unwrap_or(0);
        let exp = exp.max(rules.threshold(rank)).saturating_add(amount);
        let exp = exp.min(rules.threshold(max).max(rules.threshold(rank)));
        self.weapon_exp.insert(kind, exp);
        let new_rank = rules.rank_for(exp).min(max).max(rank);
        self.weapon_ranks.insert(kind, new_rank);
        (new_rank > rank).then_some(new_rank)
    }
}

fn check_kind(
    items: &ItemTable,
    id: &ItemId,
    ok: impl Fn(&ItemDef) -> bool,
) -> Result<(), LoadoutError> {
    let def = items
        .get(id)
        .ok_or_else(|| LoadoutError::UnknownItem(id.clone()))?;
    if ok(def) {
        Ok(())
    } else {
        Err(LoadoutError::WrongSlot(id.clone()))
    }
}

#[cfg(test)]
mod tests;
