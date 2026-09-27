//! Class skills: passives, actives paid with durability or spell uses, and
//! timed effects (`docs/design/progression.md`, *Skills*;
//! `docs/design/combat-arts.md`, *Class actives now cost durability*).
//!
//! The skills live in `assets/data/skills.ron`, loaded by `trpg-content` into
//! a [`SkillTable`]. How a battle uses them is in [`crate::battle`].
//!
//! # Rules
//!
//! - **Families and ranks.** Every skill has a `family` and a `rank`
//!   (`White Magic 1` and `2`). A unit only ever uses the highest rank it has
//!   of a family.
//! - **Learning** ([`Unit::learn_skill`]): passives are learned when a class
//!   is mastered (0601/0603 call it) and kept for good, in
//!   [`learned_skills`](Unit::learned_skills). Learning a higher rank
//!   replaces the lower one; learning a rank already known, or a lower one,
//!   does nothing.
//! - **Usable skills** ([`Unit::usable_skills`]): the learned passives, the
//!   **current class's** active (from class level 1) and the active of every
//!   **mastered** class (its [class record](Unit::class_records) at the class
//!   level cap), keeping the highest rank per family. So a reclass away from
//!   an unmastered class leaves its active behind.
//! - **Passives** are always on. Their conditions ([`Condition`]) are judged
//!   for each combat ([`SkillContext`]); their bonuses feed the combat maths
//!   as [`CombatMods`] and stat bonuses ([`Bonuses`]).
//! - **Actives** cost [`SkillCost::Durability`] (combat actives: the
//!   attacking weapon; other actives: the equipped weapon) or
//!   [`SkillCost::ExtraSpellUse`] (spell actives: 1 use on top of the cast).
//!   [`check_cost`] and [`pay_cost`] hold the rules (`combat-arts.md`,
//!   *Using an art* 3–5): a durability cost needs an unbroken weapon with
//!   `durability_left ≥ cost`; a spell active needs `uses_left ≥ 2`. The cost
//!   is paid once, when the action is committed. A weapon brought to 0 by it
//!   breaks after the action ([`Paid::broke`]). Combat Arts
//!   ([`crate::art`]) use the same helpers.
//! - **Timed effects** ([`TimedEffect`]) last until the start of a phase
//!   ([`TimedEffect::until`]), before anyone acts: a buff or a stance until
//!   its user's side's next phase, a Combat Art's debuff until the end of
//!   its target's next phase ([`TimedEffect::debuff_until`]). The same
//!   effect (same [source](EffectSource) skill or art) on a unit refreshes
//!   instead of stacking. Their stat bonuses count in combat; Mov also
//!   counts for movement ([`Unit::move_points`]). A negative bonus never
//!   takes a stat below 0 ([`Bonuses::apply`]).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::art::ArtId;
use crate::battle::{Event, Phase};
use crate::class::ClassTable;
use crate::combat::CombatMods;
use crate::spell::SpellId;
use crate::stats::{StatKind, StatValue, Stats};
use crate::unit::Unit;
use crate::weapon::WeaponKind;

/// String id of a skill, e.g. `"keen_edge"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SkillId(pub String);

impl SkillId {
    /// An id from a string.
    pub fn new(id: &str) -> Self {
        Self(id.to_owned())
    }
}

/// What an active skill costs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillCost {
    /// This much durability of the attacking (combat actives) or equipped
    /// (other actives) weapon.
    Durability(u32),
    /// One use of the spell being cast, on top of the cast's own use.
    ExtraSpellUse,
}

/// When a passive bonus applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Condition {
    /// Always.
    Always,
    /// The unit fights with a weapon of this kind.
    WeaponKindEquipped(WeaponKind),
    /// It isn't the unit's side's phase.
    NotOwnPhase,
    /// The unit's HP is at most half its max HP (`hp × 2 ≤ max`).
    HpAtMostHalf,
    /// The unit moved at least this many tiles this turn before attacking.
    MovedAtLeast(u32),
    /// The opponent fights with a weapon of this kind.
    AgainstWeaponKind(WeaponKind),
}

/// What a unit's [`Condition`]s are judged against, for one combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SkillContext {
    /// Kind of the weapon it fights with (`None`: a spell, or nothing).
    pub weapon: Option<WeaponKind>,
    /// Whether it fights with an attack spell.
    pub spell: bool,
    /// Kind of the opponent's weapon.
    pub opponent_weapon: Option<WeaponKind>,
    /// Whether it is the unit's side's phase.
    pub own_phase: bool,
    /// HP going into the combat.
    pub hp: StatValue,
    /// Max HP.
    pub max_hp: StatValue,
    /// Tiles moved this turn before attacking (0 when attacked).
    pub moved: u32,
}

impl Condition {
    /// Whether the condition holds in `ctx`.
    pub fn holds(self, ctx: &SkillContext) -> bool {
        match self {
            Condition::Always => true,
            Condition::WeaponKindEquipped(kind) => ctx.weapon == Some(kind),
            Condition::NotOwnPhase => !ctx.own_phase,
            Condition::HpAtMostHalf => ctx.hp.saturating_mul(2) <= ctx.max_hp,
            Condition::MovedAtLeast(n) => ctx.moved >= n,
            Condition::AgainstWeaponKind(kind) => ctx.opponent_weapon == Some(kind),
        }
    }
}

/// One effect of a passive skill.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PassiveEffect {
    /// A stat bonus in combat while `when` holds.
    StatWhile {
        /// The stat.
        stat: StatKind,
        /// Added to it.
        amount: StatValue,
        /// When.
        when: Condition,
    },
    /// Combat bonuses while `when` holds.
    CombatMod {
        /// The bonuses.
        mods: CombatMods,
        /// When.
        when: Condition,
    },
    /// Heal spells restore this much more HP.
    HealBonus(StatValue),
    /// Attack spells get this much more might.
    SpellMight(StatValue),
    /// After an attack while `when` holds, the unit may move up to `tiles`
    /// tiles (`turn-structure.md`, skill-granted movement).
    PostActionMove {
        /// How far.
        tiles: u32,
        /// When (judged for the attack).
        when: Condition,
    },
    /// Other allied units within `radius` tiles (Manhattan) of the skill's
    /// owner get `mods` in their combats (the lord's Leadership).
    AllyAura {
        /// Reach in tiles.
        radius: u32,
        /// The bonuses.
        mods: CombatMods,
    },
}

/// What a combat active must attack with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponReq {
    /// Any weapon (not a spell: durability is paid from the weapon).
    Any,
    /// A weapon of this kind.
    Kind(WeaponKind),
    /// An attack spell.
    Spell,
}

impl WeaponReq {
    /// Whether an attack with a weapon of kind `weapon` (`None` for a spell)
    /// meets the requirement; `spell` says whether it is a spell.
    pub fn allows(self, weapon: Option<WeaponKind>, spell: bool) -> bool {
        match self {
            WeaponReq::Any => weapon.is_some() && !spell,
            WeaponReq::Kind(kind) => weapon == Some(kind) && !spell,
            WeaponReq::Spell => spell,
        }
    }
}

/// Who a non-combat active's buff reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Area {
    /// The user only.
    Own,
    /// Every other allied unit within `radius` tiles (Manhattan) of the
    /// user; not the user.
    Allies {
        /// Reach in tiles.
        radius: u32,
    },
}

/// The bonuses of a timed effect: stats and combat numbers.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TimedMods {
    /// Stat bonuses in combat.
    pub stats: Vec<(StatKind, StatValue)>,
    /// Combat bonuses.
    pub combat: CombatMods,
}

/// A combat active's stance rider (Nick: "some might only apply for the
/// combat you used it, some might only apply for enemy's next turn, and
/// some might apply for both"). A bonus for the combat only is the
/// active's `mods`; a stance lasts from the attack until the start of the
/// user's next phase, and counts in the attack's own combat only if
/// `this_combat` says so.
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stance {
    /// The bonuses.
    pub mods: TimedMods,
    /// Whether it also counts in the combat that applies it.
    #[serde(default)]
    pub this_combat: bool,
}

/// What an active skill does.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActiveEffect {
    /// A combat active: an option of an attack (or of an attack spell's
    /// cast), for that one combat.
    Strike {
        /// What the attack must use.
        with: WeaponReq,
        /// Bonuses for this combat.
        #[serde(default)]
        mods: CombatMods,
        /// Added to the weapon's max range for this attack.
        #[serde(default)]
        range: u32,
        /// A stance rider: a timed effect on the user until the start of
        /// its next phase (see [`Stance`]).
        #[serde(default)]
        stance: Option<Stance>,
        /// After the attack the user may move up to this many tiles.
        #[serde(default)]
        post_move: u32,
        /// The user heals half the HP its strikes removed (rounded down).
        #[serde(default)]
        drain: bool,
    },
    /// A timed effect on the units of `area` until the start of the user's
    /// next phase.
    Buff {
        /// Who gets it.
        area: Area,
        /// The bonuses.
        mods: TimedMods,
    },
    /// Heals every other allied unit within `radius` tiles by the user's
    /// Mag + `power`.
    Heal {
        /// Reach in tiles.
        radius: u32,
        /// Added to Mag.
        power: StatValue,
    },
    /// Pushes an adjacent hostile unit 1 tile straight away (`magic.md`'s
    /// push rule; see [`crate::battle`]). A unit that can't be pushed there,
    /// or is pushed into a burning tile, takes `collision` damage.
    Push {
        /// Collision damage (not reduced by Def or Res).
        collision: StatValue,
    },
}

/// Passive or active, with its effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkillKind {
    /// Always on.
    Passive(Vec<PassiveEffect>),
    /// Used on purpose, for a cost.
    Active {
        /// What it costs.
        cost: SkillCost,
        /// What it does.
        effect: ActiveEffect,
    },
}

/// One skill.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SkillDef {
    /// String id.
    pub id: SkillId,
    /// Display name.
    pub name: String,
    /// Family: a higher rank of the same family supersedes a lower one.
    pub family: String,
    /// Rank within the family, from 1.
    pub rank: u8,
    /// Passive or active.
    pub kind: SkillKind,
}

impl SkillDef {
    /// Whether it is an active skill.
    pub fn is_active(&self) -> bool {
        matches!(self.kind, SkillKind::Active { .. })
    }

    /// Whether it is a combat active (chosen as an option of an attack).
    pub fn is_combat(&self) -> bool {
        matches!(
            self.kind,
            SkillKind::Active {
                effect: ActiveEffect::Strike { .. },
                ..
            }
        )
    }

    /// A passive's effects (none for an active).
    pub fn passive_effects(&self) -> &[PassiveEffect] {
        match &self.kind {
            SkillKind::Passive(effects) => effects,
            SkillKind::Active { .. } => &[],
        }
    }
}

/// Every skill.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillTable {
    /// Skills by id.
    pub skills: BTreeMap<SkillId, SkillDef>,
}

impl SkillTable {
    /// The skill `id`.
    pub fn get(&self, id: &SkillId) -> Option<&SkillDef> {
        self.skills.get(id)
    }
}

/// Where a timed effect comes from: the same source refreshes it, never
/// stacks it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EffectSource {
    /// An active skill (a buff or a stance rider).
    Skill(SkillId),
    /// A Combat Art (a stance on its user or a debuff on its target).
    Art(ArtId),
}

impl From<SkillId> for EffectSource {
    fn from(id: SkillId) -> Self {
        EffectSource::Skill(id)
    }
}

impl From<ArtId> for EffectSource {
    fn from(id: ArtId) -> Self {
        EffectSource::Art(id)
    }
}

/// A bonus (or, with negative stats, a debuff) on a unit that lasts until
/// the start of a phase.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimedEffect {
    /// What gave it (the same source refreshes, never stacks).
    pub source: EffectSource,
    /// The bonuses.
    pub mods: TimedMods,
    /// It ends at the start of this phase: the user's side's next phase for
    /// a buff or a stance; for a debuff, the phase after the target's next
    /// one ([`TimedEffect::debuff_until`]).
    pub until: Phase,
}

impl TimedEffect {
    /// When a debuff on a unit of `target`'s phase ends: at the end of that
    /// side's next phase, which is the start of the phase after it. Every
    /// phase slot is started in turn order, even one then skipped, so the
    /// two moments are the same. A debuff is only ever put on a unit hostile
    /// to the current phase's side, so "next" is never the current phase.
    pub fn debuff_until(target: Phase) -> Phase {
        target.next()
    }
}

/// Stat and combat bonuses gathered from skills and effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bonuses {
    /// Added to each stat.
    pub stats: Stats,
    /// Combat bonuses.
    pub combat: CombatMods,
}

impl Bonuses {
    /// Adds `amount` to `stat`.
    fn add_stat(&mut self, stat: StatKind, amount: StatValue) {
        self.stats
            .set(stat, self.stats.get(stat).saturating_add(amount));
    }

    /// Adds a timed effect's bonuses.
    pub fn add_timed(&mut self, mods: &TimedMods) {
        for &(stat, amount) in &mods.stats {
            self.add_stat(stat, amount);
        }
        self.combat.add(&mods.combat);
    }

    /// Adds `other`.
    pub fn add(&mut self, other: &Bonuses) {
        for stat in StatKind::ALL {
            self.add_stat(stat, other.stats.get(stat));
        }
        self.combat.add(&other.combat);
    }

    /// `stats` with the stat bonuses added (Max HP included). A negative
    /// bonus (a debuff) never takes a stat below 0, nor lowers one already
    /// below 0.
    pub fn apply(&self, stats: Stats) -> Stats {
        let mut out = stats;
        for stat in StatKind::ALL {
            let base = stats.get(stat);
            let bonus = self.stats.get(stat);
            let value = base.saturating_add(bonus);
            let value = if bonus < 0 {
                value.max(base.min(0))
            } else {
                value
            };
            out.set(stat, value);
        }
        out
    }
}

/// The bonuses of the passives in `usable` whose conditions hold in `ctx`.
/// [`PassiveEffect::SpellMight`] counts only when fighting with a spell.
pub fn passive_bonuses(usable: &[&SkillDef], ctx: &SkillContext) -> Bonuses {
    let mut out = Bonuses::default();
    for effect in usable.iter().flat_map(|s| s.passive_effects()) {
        match effect {
            PassiveEffect::StatWhile { stat, amount, when } if when.holds(ctx) => {
                out.add_stat(*stat, *amount);
            }
            PassiveEffect::CombatMod { mods, when } if when.holds(ctx) => out.combat.add(mods),
            PassiveEffect::SpellMight(might) if ctx.spell => out.combat.might += might,
            _ => {}
        }
    }
    out
}

/// The bonuses of `effects`.
pub fn effect_bonuses(effects: &[TimedEffect]) -> Bonuses {
    let mut out = Bonuses::default();
    for e in effects {
        out.add_timed(&e.mods);
    }
    out
}

/// Extra HP heal spells restore, from the passives in `usable`.
pub fn heal_bonus(usable: &[&SkillDef]) -> StatValue {
    usable
        .iter()
        .flat_map(|s| s.passive_effects())
        .map(|e| match e {
            PassiveEffect::HealBonus(n) => *n,
            _ => 0,
        })
        .sum()
}

/// How far the passives in `usable` let a unit move after an attack in
/// `ctx` (the largest; 0 if none).
pub fn post_move_tiles(usable: &[&SkillDef], ctx: &SkillContext) -> u32 {
    usable
        .iter()
        .flat_map(|s| s.passive_effects())
        .map(|e| match e {
            PassiveEffect::PostActionMove { tiles, when } if when.holds(ctx) => *tiles,
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

/// The ally auras of the passives in `usable`: `(skill, radius, mods)`.
pub fn auras<'a>(
    usable: &[&'a SkillDef],
) -> impl Iterator<Item = (&'a SkillId, u32, &'a CombatMods)> {
    usable.iter().flat_map(|s| {
        s.passive_effects().iter().filter_map(move |e| match e {
            PassiveEffect::AllyAura { radius, mods } => Some((&s.id, *radius, mods)),
            _ => None,
        })
    })
}

/// What pays a cost.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CostSource {
    /// The weapon in this loadout slot.
    Weapon(usize),
    /// This spell's uses.
    Spell(SpellId),
}

/// Why a cost can't be paid.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CostError {
    /// A durability cost with a spell, or a spell-use cost with a weapon.
    WrongSource,
    /// No weapon to pay with (empty slot, or nothing equipped).
    NoWeapon,
    /// The weapon is broken.
    WeaponBroken,
    /// The weapon has less durability left than the cost.
    NotEnoughDurability {
        /// Durability left.
        left: u32,
        /// The cost.
        cost: u32,
    },
    /// The spell has fewer than 2 uses left (the cast plus the extra use).
    NotEnoughUses {
        /// Uses left.
        left: u8,
    },
}

impl fmt::Display for CostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CostError::WrongSource => f.write_str("it can't be paid that way"),
            CostError::NoWeapon => f.write_str("no weapon to pay with"),
            CostError::WeaponBroken => f.write_str("the weapon is broken"),
            CostError::NotEnoughDurability { left, cost } => {
                write!(f, "it costs {cost} durability and the weapon has {left}")
            }
            CostError::NotEnoughUses { left } => {
                write!(f, "it needs 2 spell uses and the spell has {left}")
            }
        }
    }
}

impl std::error::Error for CostError {}

/// Whether `unit` can pay `cost` from `from` (see the module docs).
pub fn check_cost(unit: &Unit, cost: SkillCost, from: &CostSource) -> Result<(), CostError> {
    match (cost, from) {
        (SkillCost::Durability(cost), CostSource::Weapon(slot)) => {
            let copy = unit.loadout.weapon(*slot).ok_or(CostError::NoWeapon)?;
            if copy.is_broken() {
                return Err(CostError::WeaponBroken);
            }
            if copy.durability_left < cost {
                return Err(CostError::NotEnoughDurability {
                    left: copy.durability_left,
                    cost,
                });
            }
            Ok(())
        }
        (SkillCost::ExtraSpellUse, CostSource::Spell(spell)) => {
            let left = unit.spells.uses_left(spell);
            if left < 2 {
                return Err(CostError::NotEnoughUses { left });
            }
            Ok(())
        }
        _ => Err(CostError::WrongSource),
    }
}

/// What [`pay_cost`] did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paid {
    /// [`Event::DurabilitySpent`] or [`Event::SpellUsesChanged`], to emit
    /// now.
    pub events: Vec<Event>,
    /// [`Event::ItemBroke`] if the payment broke the weapon: emit it after
    /// the action (the action still uses the unbroken weapon).
    pub broke: Option<Event>,
}

/// Checks ([`check_cost`]) and pays `cost` from `from`. On `Err` nothing
/// changed.
pub fn pay_cost(unit: &mut Unit, cost: SkillCost, from: &CostSource) -> Result<Paid, CostError> {
    check_cost(unit, cost, from)?;
    let id = unit.id;
    let mut paid = Paid::default();
    match (cost, from) {
        (SkillCost::Durability(amount), CostSource::Weapon(slot)) => {
            paid.broke = unit.spend_durability(*slot, amount);
            if let Some(copy) = unit.loadout.weapon(*slot) {
                paid.events.push(Event::DurabilitySpent {
                    unit: id,
                    slot: *slot,
                    item: copy.def.clone(),
                    amount,
                    left: copy.durability_left,
                });
            }
        }
        (SkillCost::ExtraSpellUse, CostSource::Spell(spell)) => {
            if let Some(uses_left) = unit.spells.spend(spell) {
                paid.events.push(Event::SpellUsesChanged {
                    unit: id,
                    spell: spell.clone(),
                    uses_left,
                });
            }
        }
        // `check_cost` refused every other pair.
        _ => {}
    }
    Ok(paid)
}

/// A unit's skill rules.
impl Unit {
    /// Learns skill `id` (see the module docs): a higher rank replaces the
    /// lower ranks of its family; a rank already known, a lower one or an
    /// id missing from `skills` changes nothing. Returns whether it learned.
    pub fn learn_skill(&mut self, id: &SkillId, skills: &SkillTable) -> bool {
        let Some(def) = skills.get(id) else {
            return false;
        };
        let same_family: Vec<&SkillDef> = self
            .learned_skills
            .iter()
            .filter_map(|s| skills.get(s))
            .filter(|s| s.family == def.family)
            .collect();
        if same_family.iter().any(|s| s.rank >= def.rank) {
            return false;
        }
        let lower: BTreeSet<SkillId> = same_family.iter().map(|s| s.id.clone()).collect();
        self.learned_skills.retain(|s| !lower.contains(s));
        self.learned_skills.insert(id.clone());
        true
    }

    /// Every skill the unit can use now (see the module docs), in id order.
    /// Ids missing from `skills` or `classes` are skipped.
    pub fn usable_skills<'a>(
        &self,
        classes: &ClassTable,
        skills: &'a SkillTable,
    ) -> Vec<&'a SkillDef> {
        let current = classes.get(&self.class).and_then(|c| c.active.as_ref());
        let mastered = self
            .class_records
            .iter()
            .filter(|(_, r)| r.class_level >= classes.class_level_cap)
            .filter_map(|(id, _)| classes.get(id)?.active.as_ref());
        let all: Vec<&'a SkillDef> = self
            .learned_skills
            .iter()
            .chain(current)
            .chain(mastered)
            .filter_map(|id| skills.get(id))
            .collect();
        let mut out: Vec<&'a SkillDef> = all
            .iter()
            .copied()
            .filter(|d| !all.iter().any(|o| o.family == d.family && o.rank > d.rank))
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out.dedup_by(|a, b| a.id == b.id);
        out
    }

    /// The active skill `id`, if the unit can use it now.
    pub fn usable_active<'a>(
        &self,
        id: &SkillId,
        classes: &ClassTable,
        skills: &'a SkillTable,
    ) -> Option<&'a SkillDef> {
        self.usable_skills(classes, skills)
            .into_iter()
            .find(|s| s.id == *id && s.is_active())
    }

    /// Adds `effect`, replacing one from the same source (refresh, never
    /// stack).
    pub fn add_effect(&mut self, effect: TimedEffect) {
        self.effects.retain(|e| e.source != effect.source);
        self.effects.push(effect);
    }

    /// Whether an effect from `source` is on the unit.
    pub fn has_effect(&self, source: &EffectSource) -> bool {
        self.effects.iter().any(|e| e.source == *source)
    }

    /// The unit's Mov with its timed effects (a Pinning Shot's Mov −3),
    /// never below 0: its move points. Movement ranges and the danger zone
    /// read this.
    pub fn move_points(&self) -> StatValue {
        effect_bonuses(&self.effects).apply(self.stats).mov
    }

    /// Removes the effects that end at the start of `phase`, returning
    /// their sources in the order they were added.
    pub fn expire_effects(&mut self, phase: Phase) -> Vec<EffectSource> {
        let (ended, kept): (Vec<TimedEffect>, Vec<TimedEffect>) = std::mem::take(&mut self.effects)
            .into_iter()
            .partition(|e| e.until == phase);
        self.effects = kept;
        ended.into_iter().map(|e| e.source).collect()
    }
}

#[cfg(test)]
mod tests;
