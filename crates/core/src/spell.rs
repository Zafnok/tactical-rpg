//! Spells: innate abilities with uses per battle (`docs/design/magic.md`).
//!
//! The numbers live in `assets/data/spells.ron`, loaded by `trpg-content`
//! into a [`SpellTable`].
//!
//! # Rules
//!
//! - **Not items.** A spell takes no loadout slot, has no durability and can't
//!   be traded. A unit casts the spells in its [`learned`](Unit::learned) set.
//! - **Knowing** ([`Unit::known_spells`], [`Unit::refresh_spells`]): a class
//!   spell is known once the unit's class level (from
//!   [`class_records`](Unit::class_records)) reaches the spell's level, in its
//!   **current class** or in any class it has **mastered** (class level at
//!   the cap). So a reclass out of an unmastered class leaves that class's
//!   spells behind, and a return to it brings them back. A personal spell is
//!   known once the character level reaches its level, whatever the class.
//! - **Uses per battle** ([`SpellState`]): every learned spell refills to its
//!   [`uses`](SpellDef::uses) at the start of each battle
//!   ([`Unit::prepare_for_battle`]). Casting spends 1 use per combat or cast,
//!   however many strikes the combat has. A spell at 0 uses can't be cast and
//!   can't counter.
//! - **Attack spells** fight like a weapon: [`SpellDef::weapon_stats`] gives
//!   magical damage, weight 0, no kind (so no rank speed and no weapon EXP),
//!   no trait, never broken, and the spell's element. One can be the unit's
//!   [equipped](crate::item::Equipped) attack, which counters.
//! - **Heal spells** restore `min(heal_power + Mag, max HP − HP)` to an ally;
//!   no roll, no counter. The battle rules are in [`crate::battle`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::class::{ClassTable, UnitTag};
use crate::combat::{DamageType, WeaponStats, WeaponTrait};
use crate::magic::Element;
use crate::stats::StatValue;
use crate::unit::Unit;

/// String id of a spell, e.g. `"fire"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpellId(pub String);

impl SpellId {
    /// An id from a string.
    pub fn new(id: &str) -> Self {
        Self(id.to_owned())
    }
}

/// Names the terrain change a spell makes when cast on a tile, e.g.
/// `"burn_forest"`. Tile casts and their effects are ticket 0310.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TerrainEffectId(pub String);

/// What a spell does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellKind {
    /// Fights like a magical weapon.
    Attack {
        /// Might.
        might: StatValue,
        /// Base hit.
        hit: StatValue,
        /// Base crit.
        crit: StatValue,
        /// Extra `(tag, multiplier)` pairs, as for weapons.
        effective: Vec<(UnitTag, u8)>,
    },
    /// Restores HP to an ally.
    Heal {
        /// Added to the caster's Mag.
        heal_power: StatValue,
    },
}

/// One spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellDef {
    /// String id.
    pub id: SpellId,
    /// Display name.
    pub name: String,
    /// Attack or heal, with its numbers.
    pub kind: SpellKind,
    /// Element (affinities; the tile cast, 0310).
    pub element: Element,
    /// Smallest range, in tiles (Manhattan).
    pub min_range: u32,
    /// Largest range, in tiles (Manhattan).
    pub max_range: u32,
    /// Uses per battle.
    pub uses: u8,
    /// The terrain change of a tile cast, if the spell allows one (0310).
    pub terrain_effect: Option<TerrainEffectId>,
}

impl SpellDef {
    /// Whether this is an attack spell.
    pub fn is_attack(&self) -> bool {
        matches!(self.kind, SpellKind::Attack { .. })
    }

    /// Whether a unit `distance` tiles away is in range.
    pub fn in_range(&self, distance: u32) -> bool {
        (self.min_range..=self.max_range).contains(&distance)
    }

    /// The combat numbers of an attack spell (see the module docs), or
    /// `None` for a heal.
    pub fn weapon_stats(&self) -> Option<WeaponStats> {
        let SpellKind::Attack {
            might,
            hit,
            crit,
            effective,
        } = &self.kind
        else {
            return None;
        };
        Some(WeaponStats {
            kind: None,
            trait_: WeaponTrait::None,
            might: *might,
            hit: *hit,
            crit: *crit,
            weight: 0,
            min_range: self.min_range,
            max_range: self.max_range,
            damage_type: DamageType::Magical,
            effective: effective.clone(),
            broken: false,
            element: self.element,
        })
    }
}

/// Every spell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpellTable {
    /// Spells by id.
    pub spells: BTreeMap<SpellId, SpellDef>,
}

impl SpellTable {
    /// The spell `id`.
    pub fn get(&self, id: &SpellId) -> Option<&SpellDef> {
        self.spells.get(id)
    }
}

/// A unit's spell uses left in the current battle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct SpellState {
    /// Uses left per spell; a spell missing here has none.
    pub uses_left: BTreeMap<SpellId, u8>,
}

impl SpellState {
    /// Every spell of `learned` that `spells` knows, at its full uses.
    pub fn full(learned: &BTreeSet<SpellId>, spells: &SpellTable) -> SpellState {
        SpellState {
            uses_left: learned
                .iter()
                .filter_map(|id| Some((id.clone(), spells.get(id)?.uses)))
                .collect(),
        }
    }

    /// Uses left of `spell` (0 if it has none).
    pub fn uses_left(&self, spell: &SpellId) -> u8 {
        self.uses_left.get(spell).copied().unwrap_or(0)
    }

    /// Spends one use of `spell` and returns the uses left, or `None` (and
    /// no change) if it had none.
    pub fn spend(&mut self, spell: &SpellId) -> Option<u8> {
        let left = self.uses_left.get_mut(spell).filter(|n| **n > 0)?;
        *left -= 1;
        Some(*left)
    }
}

/// What [`Unit::refresh_spells`] changed, each in id order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpellChanges {
    /// Spells the unit now knows and didn't before.
    pub gained: Vec<SpellId>,
    /// Spells the unit knew and no longer does.
    pub lost: Vec<SpellId>,
}

/// A unit's spell rules.
impl Unit {
    /// Every spell the unit knows now (see the module docs). Class records of
    /// classes missing from `classes` are skipped.
    pub fn known_spells(&self, classes: &ClassTable) -> BTreeSet<SpellId> {
        let counts = |id: &crate::class::ClassId, level| {
            *id == self.class || level >= classes.class_level_cap
        };
        let from_classes = self
            .class_records
            .iter()
            .filter(|(id, record)| counts(id, record.class_level))
            .flat_map(|(id, record)| {
                classes
                    .get(id)
                    .into_iter()
                    .flat_map(|c| c.spells.iter())
                    .filter(|(level, _)| *level <= record.class_level)
                    .map(|(_, spell)| spell)
            });
        let personal = self
            .personal_spells
            .iter()
            .filter(|(level, _)| *level <= self.level)
            .map(|(_, spell)| spell);
        from_classes.chain(personal).cloned().collect()
    }

    /// Sets [`learned`](Unit::learned) to the [known spells](Unit::known_spells)
    /// and says what changed. A lost spell also loses its uses, and is
    /// unequipped if it was equipped. Call after a level up, a class level
    /// up, a promotion or a reclass (0601, 0603). New spells have no uses
    /// until the next battle starts.
    pub fn refresh_spells(&mut self, classes: &ClassTable) -> SpellChanges {
        let known = self.known_spells(classes);
        let changes = SpellChanges {
            gained: known.difference(&self.learned).cloned().collect(),
            lost: self.learned.difference(&known).cloned().collect(),
        };
        for spell in &changes.lost {
            self.spells.uses_left.remove(spell);
            if self.loadout.equipped_spell() == Some(spell) {
                self.loadout.equipped = None;
            }
        }
        self.learned = known;
        changes
    }

    /// The attack spell `spell` if the unit can fight with it now: learned,
    /// an attack spell in `spells`, and at least 1 use left.
    pub fn castable_attack<'a>(
        &self,
        spell: &SpellId,
        spells: &'a SpellTable,
    ) -> Option<&'a SpellDef> {
        let def = spells.get(spell)?;
        let ok =
            self.learned.contains(spell) && def.is_attack() && self.spells.uses_left(spell) > 0;
        ok.then_some(def)
    }

    /// The first learned attack spell in `spells`, in id order.
    pub fn first_attack_spell(&self, spells: &SpellTable) -> Option<SpellId> {
        self.learned
            .iter()
            .find(|s| spells.get(s).is_some_and(SpellDef::is_attack))
            .cloned()
    }
}

#[cfg(test)]
mod tests;
