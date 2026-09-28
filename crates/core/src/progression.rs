//! Unit EXP, level ups, class points, class levels and mastery
//! (`docs/design/progression.md`).
//!
//! # Rules
//!
//! - **EXP** ([`exp_for_combat`] and friends): FE GBA's shape, doubled. `100`
//!   EXP make a character level; EXP carries over. Only Player-faction units
//!   gain EXP and class points; an Ally-faction unit's EXP goes into the
//!   battle's EXP pool instead (see [`crate::battle`]).
//! - **Level cap** ([`ClassTable::level_cap`], data): a unit at the cap gains
//!   no EXP (shown as `--`) and has 0 EXP; an award that reaches the cap is
//!   cut to what reaching it takes.
//! - **Level up** ([`level_up`]): the exact procedure of `progression.md`.
//!   The growths, caps and tier are the unit's **current class**'s; the
//!   talent stat gets +20% ([`growth`]). Every level up makes exactly 7
//!   [`roll_percent`](RandomSource::roll_percent) calls (one per stat, in stat
//!   order, eligible or not), then one
//!   [`roll_below`](RandomSource::roll_below) per safety-net pick. Current HP
//!   rises with max HP. A level up can teach personal spells (character
//!   level); class spells and skills come from class levels.
//! - **Class points** ([`grant_class_points`]) go to the current class's
//!   record only. Class level = `1 + CP / cp_per_class_level[tier]`, capped at
//!   [`ClassTable::class_level_cap`] (mastery); CP stop counting at mastery.
//!   Each class level reached may teach class spells; mastery teaches the
//!   class's passives (its active becomes permanent through
//!   [`Unit::usable_skills`], which reads the class records).
//! - **Tier tables** ([`ClassTable::min_gains`],
//!   [`ClassTable::cp_per_class_level`]): a tier missing from a table uses
//!   the highest tier it has (the tables never decrease, and content
//!   validation requires an entry for every tier in use).

use serde::{Deserialize, Serialize};

use crate::battle::Event;
use crate::class::{ClassDef, ClassLevel, ClassPoints, ClassTable, Tier};
use crate::rng::RandomSource;
use crate::skill::SkillTable;
use crate::stats::{GrowthValue, StatKind, StatValue};
use crate::unit::{ClassRecord, Level, Unit};

/// EXP for one character level.
pub const EXP_PER_LEVEL: u32 = 100;

/// Most EXP one award gives (one level at most).
pub const MAX_AWARD: u32 = 100;

/// Least EXP a combat gives.
pub const MIN_COMBAT_EXP: u32 = 2;

/// Extra kill EXP for a boss.
pub const BOSS_KILL_BONUS: i64 = 40;

/// EXP for healing an ally with a spell.
pub const HEAL_EXP: u32 = 24;

/// EXP for a tile cast (as for a heal, `magic.md`).
pub const TILE_CAST_EXP: u32 = 24;

/// EXP for using an active skill that isn't a combat.
pub const ACTIVE_SKILL_EXP: u32 = 20;

/// Class points for taking part in a combat.
pub const COMBAT_CP: ClassPoints = 2;

/// Extra class points for a kill.
pub const KILL_CP: ClassPoints = 2;

/// Class points for a heal, a tile cast or a non-combat active.
pub const ACTION_CP: ClassPoints = 2;

/// Growth added to a character's talent stat, in percent.
pub const TALENT_BONUS: GrowthValue = 20;

/// What a unit's strikes did in one combat, for its EXP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatResult {
    /// It took part but dealt no damage and killed nobody.
    NoDamage,
    /// It dealt damage (≥ 1 HP) and the target survived.
    Damaged,
    /// It killed the target.
    Killed,
}

/// EXP for a unit of `unit_level` after a combat against a target of
/// `target_level` (`progression.md`, *EXP formulas*). With
/// `d = target_level − unit_level`:
/// - no damage: `2`;
/// - damage: `clamp(2 × ((31 + d) / 3), 2, 100)`;
/// - kill: `clamp(2 × ((31 + d) / 3 + max(0, 20 + 3d)) + (boss ? 40 : 0), 2, 100)`.
pub fn exp_for_combat(
    unit_level: Level,
    target_level: Level,
    result: CombatResult,
    target_is_boss: bool,
) -> u32 {
    let d = i64::from(target_level) - i64::from(unit_level);
    // Rust's `/` rounds toward zero, as the design asks.
    let base = (31 + d) / 3;
    let exp = match result {
        CombatResult::NoDamage => return MIN_COMBAT_EXP,
        CombatResult::Damaged => 2 * base,
        CombatResult::Killed => {
            let boss = if target_is_boss { BOSS_KILL_BONUS } else { 0 };
            2 * (base + (20 + 3 * d).max(0)) + boss
        }
    };
    let exp = exp.clamp(i64::from(MIN_COMBAT_EXP), i64::from(MAX_AWARD));
    u32::try_from(exp).unwrap_or(MAX_AWARD)
}

/// EXP for healing an ally with a spell.
pub fn exp_for_heal() -> u32 {
    HEAL_EXP
}

/// EXP for a tile cast.
pub fn exp_for_tile_cast() -> u32 {
    TILE_CAST_EXP
}

/// EXP for using a non-combat active skill.
pub fn exp_for_active_skill() -> u32 {
    ACTIVE_SKILL_EXP
}

/// Class points for taking part in a combat, `killed` or not.
pub fn cp_for_combat(killed: bool) -> ClassPoints {
    COMBAT_CP + if killed { KILL_CP } else { 0 }
}

/// The growth of `stat` for `unit` in `class`: the class growth, +20 for the
/// unit's talent. Mov never grows.
pub fn growth(unit: &Unit, class: &ClassDef, stat: StatKind) -> GrowthValue {
    if stat == StatKind::Mov {
        return 0;
    }
    let talent = if unit.talent == Some(stat) {
        TALENT_BONUS
    } else {
        0
    };
    class.growths.get(stat).saturating_add(talent)
}

/// Points gained per growable stat in one level up, in stat order
/// ([`StatKind::GROWABLE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct StatGains(pub [StatValue; 7]);

impl StatGains {
    /// The gain of `kind` (0 for Mov).
    pub fn get(&self, kind: StatKind) -> StatValue {
        StatKind::GROWABLE
            .iter()
            .position(|&k| k == kind)
            .map_or(0, |i| self.0[i])
    }

    /// How many stats gained at least 1 point.
    pub fn count(&self) -> usize {
        self.0.iter().filter(|&&g| g > 0).count()
    }
}

/// The minimum stat gains per level up for `tier` (see the module docs for
/// a missing tier; an empty table gives 0).
pub fn min_gains(classes: &ClassTable, tier: Tier) -> u8 {
    classes
        .min_gains(tier)
        .or_else(|| classes.min_gains.last().copied())
        .unwrap_or(0)
}

/// Class points per class level for `tier` (see the module docs for a
/// missing tier; an empty table gives 0: no class level ups).
pub fn cp_per_class_level(classes: &ClassTable, tier: Tier) -> ClassPoints {
    classes
        .cp_per_class_level(tier)
        .or_else(|| classes.cp_per_class_level.last().copied())
        .unwrap_or(0)
}

/// Rolls one level up of `unit` in `class` (its current class), with at
/// least `min_gains` stats gaining when that many can (the exact procedure
/// of `progression.md`). Doesn't change the unit: see [`apply_gains`].
///
/// A stat is eligible when it is below its cap and its [`growth`] is above 0.
/// Every stat, eligible or not, takes one `roll()` `r`; an eligible one gains
/// `growth / 100 + (r < growth % 100 ? 1 : 0)`, never past its cap. Then,
/// while fewer than `min(min_gains, eligible)` stats have gained, one of
/// the eligible stats that haven't gains 1, picked with
/// `roll_below(sum of their growths)` weighted by growth.
pub fn level_up(
    unit: &Unit,
    class: &ClassDef,
    min_gains: u8,
    rng: &mut impl RandomSource,
) -> StatGains {
    let mut gains = StatGains::default();
    let mut eligible = [false; 7];
    let mut growths = [0; 7];
    for (i, &kind) in StatKind::GROWABLE.iter().enumerate() {
        let r = GrowthValue::from(rng.roll_percent());
        let g = growth(unit, class, kind);
        let room = class.caps.get(kind) - unit.stats.get(kind);
        if g == 0 || room <= 0 {
            continue;
        }
        eligible[i] = true;
        growths[i] = u32::from(g);
        let points = g / 100 + GrowthValue::from(r < g % 100);
        gains.0[i] = StatValue::from(points).min(room);
    }
    let eligible_count = eligible.iter().filter(|&&e| e).count();
    let need = usize::from(min_gains).min(eligible_count);
    // Each pick adds a gain, so `need − gains` picks at most.
    for _ in gains.count()..need {
        // Candidates have 0 points so far, so their growth is below 100.
        let candidates: Vec<usize> = (0..7).filter(|&i| eligible[i] && gains.0[i] == 0).collect();
        let total = candidates.iter().map(|&i| growths[i]).sum();
        let r = rng.roll_below(total);
        let mut sum = 0;
        let Some(&pick) = candidates.iter().find(|&&i| {
            sum += growths[i];
            sum > r
        }) else {
            break;
        };
        gains.0[pick] = 1;
    }
    gains
}

/// Adds `gains` to `unit`'s stats; current HP rises with max HP.
pub fn apply_gains(unit: &mut Unit, gains: &StatGains) {
    for (i, &kind) in StatKind::GROWABLE.iter().enumerate() {
        let value = unit.stats.get(kind).saturating_add(gains.0[i]);
        unit.stats.set(kind, value);
    }
    unit.hp = unit.hp.saturating_add(gains.get(StatKind::Hp));
}

/// Gives `unit` `amount` EXP: [`Event::ExpGained`], then an
/// [`Event::LeveledUp`] per level (each rolled with `rng`, followed by an
/// [`Event::SpellLearned`] per personal spell it teaches). The amount is cut
/// to what reaching the level cap takes; at the cap, or with an unknown
/// class, nothing happens. Doesn't check the unit's faction.
pub fn grant_exp(
    unit: &mut Unit,
    amount: u32,
    classes: &ClassTable,
    rng: &mut impl RandomSource,
) -> Vec<Event> {
    let mut events = Vec::new();
    let cap = classes.level_cap;
    let Some(class) = classes.get(&unit.class) else {
        return events;
    };
    if unit.level >= cap || amount == 0 {
        return events;
    }
    let room = (u64::from(cap - unit.level) * u64::from(EXP_PER_LEVEL))
        .saturating_sub(u64::from(unit.exp));
    let amount = u32::try_from(u64::from(amount).min(room)).unwrap_or(amount);
    unit.exp += amount;
    events.push(Event::ExpGained {
        unit: unit.id,
        amount,
    });
    let floor = min_gains(classes, class.tier);
    // The amount was cut to the cap, so the levels stop exactly there.
    let levels = unit.exp / EXP_PER_LEVEL;
    unit.exp %= EXP_PER_LEVEL;
    for _ in 0..levels {
        let gains = level_up(unit, class, floor, rng);
        apply_gains(unit, &gains);
        unit.level += 1;
        events.push(Event::LeveledUp {
            unit: unit.id,
            level: unit.level,
            gains,
        });
        learn_spells(unit, classes, &mut events);
    }
    events
}

/// Gives `unit`'s current class `amount` class points:
/// [`Event::ClassPointsGained`], then for each class level reached an
/// [`Event::ClassLeveledUp`] and an [`Event::SpellLearned`] per spell it
/// teaches, and at mastery [`Event::ClassMastered`] and an
/// [`Event::SkillLearned`] per passive learned. A mastered class, an unknown
/// class or a tier with no CP per class level gets nothing. CP stop at
/// mastery's total.
pub fn grant_class_points(
    unit: &mut Unit,
    amount: ClassPoints,
    classes: &ClassTable,
    skills: &SkillTable,
) -> Vec<Event> {
    let mut events = Vec::new();
    let Some(class) = classes.get(&unit.class) else {
        return events;
    };
    let cap = classes.class_level_cap;
    let per_level = cp_per_class_level(classes, class.tier);
    let record = unit
        .class_records
        .entry(class.id.clone())
        .or_insert(ClassRecord::UNLOCKED);
    if record.class_level >= cap || per_level == 0 || amount == 0 {
        return events;
    }
    let mastery = per_level.saturating_mul(ClassPoints::from(cap - 1));
    let before = record.class_points;
    let after = before.saturating_add(amount).min(mastery);
    record.class_points = after;
    let reached = ClassLevel::try_from(1 + after / per_level)
        .unwrap_or(cap)
        .min(cap);
    let from = record.class_level;
    events.push(Event::ClassPointsGained {
        unit: unit.id,
        class: class.id.clone(),
        amount: after.saturating_sub(before),
    });
    for level in from.saturating_add(1)..=reached {
        if let Some(r) = unit.class_records.get_mut(&class.id) {
            r.class_level = level;
        }
        events.push(Event::ClassLeveledUp {
            unit: unit.id,
            class: class.id.clone(),
            class_level: level,
        });
        learn_spells(unit, classes, &mut events);
        if level == cap {
            events.push(Event::ClassMastered {
                unit: unit.id,
                class: class.id.clone(),
            });
            for passive in &class.passives {
                if unit.learn_skill(passive, skills) {
                    events.push(Event::SkillLearned {
                        unit: unit.id,
                        skill: passive.clone(),
                    });
                }
            }
        }
    }
    events
}

/// Learns the spells `unit` now knows and hadn't learned, with an
/// [`Event::SpellLearned`] each, in id order. Never forgets one: a level up
/// only ever adds knowledge, and a spell given outside the class and
/// personal lists (chapter data) must stay.
fn learn_spells(unit: &mut Unit, classes: &ClassTable, events: &mut Vec<Event>) {
    let known = unit.known_spells(classes);
    for spell in known.difference(&unit.learned) {
        events.push(Event::SpellLearned {
            unit: unit.id,
            spell: spell.clone(),
        });
    }
    unit.learned.extend(known);
}

#[cfg(test)]
mod tests;
