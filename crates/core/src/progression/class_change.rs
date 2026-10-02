//! Promotion and reclass (`docs/design/progression.md`, *Promotion
//! (branching)* and *Reclass*; ticket 0603).
//!
//! Both happen **between battles** (Nick, ticket 0603: no seal works in a
//! battle), on a unit of the party and the party's [`Stock`], which holds
//! the seals and receives what the new class can't carry. They aren't
//! [`Command`](crate::battle::Command)s: no battle is running.
//!
//! # Rules
//!
//! - **Promotion** ([`promote`]): the unit's current class must be
//!   **mastered** (its class level at [`ClassTable::class_level_cap`]), the
//!   target must be one of its `promotes_to`, not enemy-only, and lord-only
//!   only for the lord, and the stock must hold the **seal of the target's
//!   tier** ([`SealKind::Tier`]), which is used up. Each stat gains
//!   `max(0, new.base − old.base)`, never past its hard ceiling
//!   ([`promotion_gains`]); current HP rises with max HP. The character
//!   level and EXP don't change.
//! - **Reclass** ([`reclass`]): always uses up a **Reclass Seal**
//!   ([`SealKind::Reclass`]), a return to a class already unlocked included.
//!   The target must be another class than the current one (*Claude's
//!   starting rule*: a seal is never spent on a change that changes
//!   nothing), not enemy-only, lord-only only for the lord, and one of: a
//!   **tier-1** class; a class the unit has **already unlocked**; a class
//!   that a class the unit has **mastered** promotes to
//!   ([`reclass_targets`]). Stats never change (no bonus, nothing lowered);
//!   the level and EXP don't either.
//! - **Entering the class** (both): the unit's class record for it is kept
//!   if it has one (saved progress comes back), else it starts at class
//!   level 1. Mov becomes the class's. Weapon ranks rise to the class's
//!   start ranks (never lowered; ranks in other kinds are kept). The unit's
//!   spells are refreshed ([`Unit::refresh_spells`]): the new class's spells
//!   up to its class level are learned ([`Event::SpellLearned`]), and those
//!   of a class left unmastered are lost until it returns to it. Its actives
//!   follow the class records by themselves ([`Unit::usable_skills`]):
//!   the new class's active at once, a left class's only if mastered.
//!   Passives are kept. Weapons in slots the class doesn't have
//!   ([`Unit::fit_weapon_slots`]) and armour it can't wear
//!   ([`Unit::fit_armour`]) go to the stock ([`Event::ItemStowed`]), and the
//!   unit ends with its [default equip](Unit::default_equip) if what it had
//!   equipped is gone.
//! - **Errors change nothing**: everything is checked before the unit or
//!   the stock is touched.

use std::fmt;

use crate::battle::Event;
use crate::class::{ClassDef, ClassId, ClassTable};
use crate::item::{ItemId, ItemTable, SealKind, Stock};
use crate::progression::{StatGains, apply_gains};
use crate::spell::SpellTable;
use crate::stats::{StatKind, Stats};
use crate::unit::{ClassRecord, Unit, raise_to_start_ranks};

/// The content tables a class change reads.
#[derive(Debug, Clone, Copy)]
pub struct ChangeTables<'a> {
    /// The class tree.
    pub classes: &'a ClassTable,
    /// Items (the seals, and the unit's weapons and armour).
    pub items: &'a ItemTable,
    /// Spells (for the default equip of a unit left without a weapon).
    pub spells: &'a SpellTable,
}

/// Why a promotion or a reclass was refused. Nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassChangeError {
    /// A class id (the unit's or the target) is not in the class table.
    UnknownClass(ClassId),
    /// The unit hasn't mastered its current class, so it can't promote.
    NotMastered(ClassId),
    /// The unit's class doesn't promote to this class.
    NotAPromotion {
        /// The unit's class.
        from: ClassId,
        /// The class asked for.
        to: ClassId,
    },
    /// Only enemies use this class.
    EnemyOnly(ClassId),
    /// Only the lord can be in this class.
    LordOnly(ClassId),
    /// The stock has no seal of this kind.
    NoSeal(SealKind),
    /// A reclass into the class the unit is already in.
    SameClass(ClassId),
    /// A Reclass Seal can't take the unit to this class: it isn't tier 1,
    /// the unit hasn't unlocked it, and hasn't mastered a class that
    /// promotes to it.
    NotReachable(ClassId),
}

impl fmt::Display for ClassChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClassChangeError::UnknownClass(c) => write!(f, "unknown class \"{}\"", c.0),
            ClassChangeError::NotMastered(c) => write!(f, "\"{}\" isn't mastered", c.0),
            ClassChangeError::NotAPromotion { from, to } => {
                write!(f, "\"{}\" doesn't promote to \"{}\"", from.0, to.0)
            }
            ClassChangeError::EnemyOnly(c) => write!(f, "\"{}\" is only for enemies", c.0),
            ClassChangeError::LordOnly(c) => write!(f, "\"{}\" is only for the lord", c.0),
            ClassChangeError::NoSeal(SealKind::Tier(tier)) => {
                write!(f, "no seal for tier {tier} in stock")
            }
            ClassChangeError::NoSeal(SealKind::Reclass) => f.write_str("no reclass seal in stock"),
            ClassChangeError::SameClass(c) => write!(f, "already in \"{}\"", c.0),
            ClassChangeError::NotReachable(c) => {
                write!(f, "a reclass seal can't reach \"{}\"", c.0)
            }
        }
    }
}

impl std::error::Error for ClassChangeError {}

/// Whether `unit` has mastered class `id`.
pub fn has_mastered(unit: &Unit, id: &ClassId, classes: &ClassTable) -> bool {
    unit.class_records
        .get(id)
        .is_some_and(|r| r.class_level >= classes.class_level_cap)
}

/// The promotion bonus of `unit` going from class `from` to class `to`, per
/// growable stat: `max(0, to.base − from.base)`, cut so the stat stays at or
/// below its hard ceiling in `ceilings`.
pub fn promotion_gains(unit: &Unit, from: &ClassDef, to: &ClassDef, ceilings: &Stats) -> StatGains {
    let mut gains = StatGains::default();
    for (i, &kind) in StatKind::GROWABLE.iter().enumerate() {
        let bonus = (to.base.get(kind) - from.base.get(kind)).max(0);
        let room = (ceilings.get(kind) - unit.stats.get(kind)).max(0);
        gains.0[i] = bonus.min(room);
    }
    gains
}

/// Whether `unit` may ever be in `class`: not enemy-only, and lord-only
/// only for the lord.
fn allowed(unit: &Unit, class: &ClassDef) -> Result<(), ClassChangeError> {
    if class.enemy_only {
        return Err(ClassChangeError::EnemyOnly(class.id.clone()));
    }
    if class.lord_only && !unit.is_lord {
        return Err(ClassChangeError::LordOnly(class.id.clone()));
    }
    Ok(())
}

fn class<'a>(classes: &'a ClassTable, id: &ClassId) -> Result<&'a ClassDef, ClassChangeError> {
    classes
        .get(id)
        .ok_or_else(|| ClassChangeError::UnknownClass(id.clone()))
}

/// The classes `unit`'s class promotes to that the unit may be in, in the
/// class's `promotes_to` order: what a promotion offers. Doesn't check
/// mastery or the seal ([`promote`] does).
pub fn promotion_targets<'a>(unit: &Unit, classes: &'a ClassTable) -> Vec<&'a ClassDef> {
    let Some(current) = classes.get(&unit.class) else {
        return Vec::new();
    };
    current
        .promotes_to
        .iter()
        .filter_map(|id| classes.get(id))
        .filter(|c| allowed(unit, c).is_ok())
        .collect()
}

/// Whether a Reclass Seal can take `unit` to `target` (see the module docs).
fn reachable(unit: &Unit, target: &ClassDef, classes: &ClassTable) -> bool {
    let through_mastery = || {
        unit.class_records
            .keys()
            .filter(|id| has_mastered(unit, id, classes))
            .filter_map(|id| classes.get(id))
            .any(|c| c.promotes_to.contains(&target.id))
    };
    target.tier == 1 || unit.class_records.contains_key(&target.id) || through_mastery()
}

/// Every class a Reclass Seal can take `unit` to, by tier then id: what a
/// reclass offers. Doesn't check the seal ([`reclass`] does).
pub fn reclass_targets<'a>(unit: &Unit, classes: &'a ClassTable) -> Vec<&'a ClassDef> {
    let mut targets: Vec<&ClassDef> = classes
        .classes
        .values()
        .filter(|c| c.id != unit.class)
        .filter(|c| allowed(unit, c).is_ok())
        .filter(|c| reachable(unit, c, classes))
        .collect();
    // Stable: ids stay in order within a tier.
    targets.sort_by_key(|c| c.tier);
    targets
}

/// The seal of `kind` in `stock`, or why there is none.
fn seal(stock: &Stock, kind: SealKind, items: &ItemTable) -> Result<ItemId, ClassChangeError> {
    stock
        .seal(kind, items)
        .cloned()
        .ok_or(ClassChangeError::NoSeal(kind))
}

/// Puts `unit` in class `to` (see *Entering the class* in the module docs),
/// adding its events.
fn enter(
    unit: &mut Unit,
    to: &ClassDef,
    stock: &mut Stock,
    tables: &ChangeTables<'_>,
    events: &mut Vec<Event>,
) {
    let id = unit.id;
    unit.class = to.id.clone();
    unit.class_records
        .entry(to.id.clone())
        .or_insert(ClassRecord::UNLOCKED);
    unit.stats.mov = to.move_points;
    raise_to_start_ranks(&mut unit.weapon_ranks, to);
    let changes = unit.refresh_spells(tables.classes);
    events.extend(
        changes
            .gained
            .into_iter()
            .map(|spell| Event::SpellLearned { unit: id, spell }),
    );
    let weapons = unit.fit_weapon_slots(to, tables.items, tables.spells, stock);
    let armour = unit.fit_armour(to, tables.items, stock);
    events.extend(
        weapons
            .into_iter()
            .chain(armour)
            .map(|item| Event::ItemStowed { unit: id, item }),
    );
    if unit.loadout.equipped.is_none() {
        unit.loadout.equipped = unit.default_equip(to, tables.items, tables.spells);
    }
}

/// Promotes `unit` into `target` with the seal of its tier from `stock`
/// (see the module docs): [`Event::Promoted`], then an
/// [`Event::SpellLearned`] per spell learned and an [`Event::ItemStowed`]
/// per item sent to the stock. On `Err` nothing changed.
pub fn promote(
    unit: &mut Unit,
    target: &ClassId,
    stock: &mut Stock,
    tables: &ChangeTables<'_>,
) -> Result<Vec<Event>, ClassChangeError> {
    let classes = tables.classes;
    let from = class(classes, &unit.class)?;
    let to = class(classes, target)?;
    if !has_mastered(unit, &from.id, classes) {
        return Err(ClassChangeError::NotMastered(from.id.clone()));
    }
    if !from.promotes_to.contains(target) {
        return Err(ClassChangeError::NotAPromotion {
            from: from.id.clone(),
            to: target.clone(),
        });
    }
    allowed(unit, to)?;
    let seal = seal(stock, SealKind::Tier(to.tier), tables.items)?;
    stock.take(&seal);
    let gains = promotion_gains(unit, from, to, &classes.hard_ceilings);
    apply_gains(unit, &gains);
    let mut events = vec![Event::Promoted {
        unit: unit.id,
        from: from.id.clone(),
        to: to.id.clone(),
        gains,
    }];
    enter(unit, to, stock, tables, &mut events);
    Ok(events)
}

/// Changes `unit`'s class to `target` with a Reclass Seal from `stock` (see
/// the module docs): [`Event::Reclassed`], then an [`Event::SpellLearned`]
/// per spell learned and an [`Event::ItemStowed`] per item sent to the
/// stock. On `Err` nothing changed.
pub fn reclass(
    unit: &mut Unit,
    target: &ClassId,
    stock: &mut Stock,
    tables: &ChangeTables<'_>,
) -> Result<Vec<Event>, ClassChangeError> {
    let classes = tables.classes;
    let from = class(classes, &unit.class)?;
    let to = class(classes, target)?;
    if from.id == to.id {
        return Err(ClassChangeError::SameClass(to.id.clone()));
    }
    allowed(unit, to)?;
    if !reachable(unit, to, classes) {
        return Err(ClassChangeError::NotReachable(to.id.clone()));
    }
    let seal = seal(stock, SealKind::Reclass, tables.items)?;
    stock.take(&seal);
    let mut events = vec![Event::Reclassed {
        unit: unit.id,
        from: from.id.clone(),
        to: to.id.clone(),
    }];
    enter(unit, to, stock, tables, &mut events);
    Ok(events)
}

#[cfg(test)]
mod tests;
