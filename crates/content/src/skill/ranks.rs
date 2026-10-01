//! The rank rule (`docs/design/progression.md` → *Superseding*; Nick): a
//! higher rank of a family is the same skill with bigger numbers, nothing
//! else. Anything else (a wider reach, another condition, a new kind of
//! effect) must be a skill of its own.
//!
//! Each rank is compared with the rank below it in its family:
//!
//! - both are passives with the same number of effects, or both actives;
//! - each pair of effects is the same kind, with the same fundamentals:
//!   its condition, stat, weapon requirement, area, every radius, its flags
//!   and its kind of cost;
//! - no number is lower and at least one is higher. A number may go from 0
//!   to more (Bow Focus 2 adds crit). A cost in durability may be anything.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Debug;

use trpg_core::{
    ActiveEffect, CombatMods, PassiveEffect, SkillCost, SkillDef, SkillId, SkillKind, StatKind,
    StatValue, TimedMods,
};

/// What keeps a rank from being the rank below it with bigger numbers.
#[derive(Default)]
struct Check {
    /// What differs that may not, or is lower.
    problems: Vec<String>,
    /// Whether a number is higher.
    raised: bool,
}

impl Check {
    /// A fundamental: it must be the same in both ranks.
    fn same<T: PartialEq + Debug>(&mut self, what: &str, lower: &T, higher: &T) {
        if lower != higher {
            self.problems
                .push(format!("its {what} is {higher:?}, not {lower:?}"));
        }
    }

    /// A number: it may not be lower in the higher rank.
    fn number(&mut self, what: &str, lower: impl Into<i64>, higher: impl Into<i64>) {
        let (lower, higher) = (lower.into(), higher.into());
        match higher.cmp(&lower) {
            Ordering::Less => self
                .problems
                .push(format!("its {what} is {higher}, less than {lower}")),
            Ordering::Greater => self.raised = true,
            Ordering::Equal => {}
        }
    }

    fn mods(&mut self, lower: &CombatMods, higher: &CombatMods) {
        self.number("hit", lower.hit, higher.hit);
        self.number("crit", lower.crit, higher.crit);
        self.number("might", lower.might, higher.might);
        self.number("avoid", lower.avoid, higher.avoid);
        self.number("attack speed", lower.attack_speed, higher.attack_speed);
        self.number("extra strikes", lower.extra_strikes, higher.extra_strikes);
        self.number("pierce", lower.pierce, higher.pierce);
        self.same("single_strike", &lower.single_strike, &higher.single_strike);
        self.same("double_crit", &lower.double_crit, &higher.double_crit);
        self.same(
            "ignore_terrain",
            &lower.ignore_terrain,
            &higher.ignore_terrain,
        );
        self.same(
            "sword_followup",
            &lower.sword_followup,
            &higher.sword_followup,
        );
    }

    fn timed(&mut self, lower: &TimedMods, higher: &TimedMods) {
        let total = |mods: &TimedMods, stat| -> StatValue {
            let of_stat = mods.stats.iter().filter(|(s, _)| *s == stat);
            of_stat.map(|(_, n)| *n).sum()
        };
        for stat in StatKind::ALL {
            self.number(
                &format!("{stat:?}"),
                total(lower, stat),
                total(higher, stat),
            );
        }
        self.mods(&lower.combat, &higher.combat);
    }

    fn passive(&mut self, lower: &PassiveEffect, higher: &PassiveEffect) {
        use PassiveEffect::{
            AllyAura, CombatMod, HealBonus, PostActionMove, SpellMight, StatWhile,
        };
        match (lower, higher) {
            (
                StatWhile { stat, amount, when },
                StatWhile {
                    stat: stat2,
                    amount: amount2,
                    when: when2,
                },
            ) => {
                self.same("stat", stat, stat2);
                self.same("condition", when, when2);
                self.number("amount", *amount, *amount2);
            }
            (
                CombatMod { mods, when },
                CombatMod {
                    mods: mods2,
                    when: when2,
                },
            ) => {
                self.same("condition", when, when2);
                self.mods(mods, mods2);
            }
            (HealBonus(n), HealBonus(n2)) => self.number("heal bonus", *n, *n2),
            (SpellMight(n), SpellMight(n2)) => self.number("spell might", *n, *n2),
            (
                PostActionMove { tiles, when },
                PostActionMove {
                    tiles: tiles2,
                    when: when2,
                },
            ) => {
                self.same("condition", when, when2);
                self.number("tiles", *tiles, *tiles2);
            }
            (
                AllyAura { radius, mods },
                AllyAura {
                    radius: radius2,
                    mods: mods2,
                },
            ) => {
                self.same("radius", radius, radius2);
                self.mods(mods, mods2);
            }
            _ => self.other_kind(),
        }
    }

    fn other_kind(&mut self) {
        self.problems.push("it has another kind of effect".into());
    }

    fn cost(&mut self, lower: SkillCost, higher: SkillCost) {
        match (lower, higher) {
            // A cost in durability may be anything.
            (SkillCost::Durability(_), SkillCost::Durability(_))
            | (SkillCost::ExtraSpellUse, SkillCost::ExtraSpellUse) => {}
            (SkillCost::Uses(n), SkillCost::Uses(n2)) => self.number("uses", n, n2),
            _ => self
                .problems
                .push(format!("its cost is {higher:?}, not like {lower:?}")),
        }
    }

    fn active(&mut self, lower: &ActiveEffect, higher: &ActiveEffect) {
        use ActiveEffect::{Buff, Heal, Push, Strike};
        match (lower, higher) {
            (
                Strike {
                    with,
                    mods,
                    range,
                    stance,
                    post_move,
                    drain,
                },
                Strike {
                    with: with2,
                    mods: mods2,
                    range: range2,
                    stance: stance2,
                    post_move: post_move2,
                    drain: drain2,
                },
            ) => {
                self.same("weapon", with, with2);
                self.mods(mods, mods2);
                self.number("range", *range, *range2);
                self.number("move after", *post_move, *post_move2);
                self.same("drain", drain, drain2);
                match (stance, stance2) {
                    (Some(stance), Some(stance2)) => {
                        self.same("this_combat", &stance.this_combat, &stance2.this_combat);
                        self.timed(&stance.mods, &stance2.mods);
                    }
                    (None, None) => {}
                    _ => self.problems.push("only one of them has a stance".into()),
                }
            }
            (
                Buff { area, mods },
                Buff {
                    area: area2,
                    mods: mods2,
                },
            ) => {
                self.same("area", area, area2);
                self.timed(mods, mods2);
            }
            (
                Heal { radius, power },
                Heal {
                    radius: radius2,
                    power: power2,
                },
            ) => {
                self.same("radius", radius, radius2);
                self.number("power", *power, *power2);
            }
            (
                Push { collision },
                Push {
                    collision: collision2,
                },
            ) => self.number("collision damage", *collision, *collision2),
            _ => self.other_kind(),
        }
    }

    fn kind(&mut self, lower: &SkillKind, higher: &SkillKind) {
        match (lower, higher) {
            (SkillKind::Passive(effects), SkillKind::Passive(effects2)) => {
                if effects.len() != effects2.len() {
                    self.problems.push(format!(
                        "it has {} effects, not {}",
                        effects2.len(),
                        effects.len()
                    ));
                }
                for (effect, effect2) in effects.iter().zip(effects2) {
                    self.passive(effect, effect2);
                }
            }
            (
                SkillKind::Active { cost, effect },
                SkillKind::Active {
                    cost: cost2,
                    effect: effect2,
                },
            ) => {
                self.cost(*cost, *cost2);
                self.active(effect, effect2);
            }
            _ => self
                .problems
                .push("one is a passive and the other an active".into()),
        }
    }
}

/// What is wrong with `higher` as the next rank after `lower`.
fn compare(lower: &SkillDef, higher: &SkillDef) -> Vec<String> {
    let mut check = Check::default();
    check.kind(&lower.kind, &higher.kind);
    if check.problems.is_empty() && !check.raised {
        check.problems.push("none of its numbers is higher".into());
    }
    check.problems
}

/// For every family with more than one rank, what keeps each rank from
/// being the rank below it with bigger numbers: the higher skill's id and
/// the problem, naming both skills. In family then rank order.
pub(super) fn rank_problems(skills: &BTreeMap<SkillId, SkillDef>) -> Vec<(SkillId, String)> {
    let mut families: BTreeMap<&str, Vec<&SkillDef>> = BTreeMap::new();
    for def in skills.values() {
        families.entry(&def.family).or_default().push(def);
    }
    let mut out = Vec::new();
    for ranks in families.values_mut() {
        ranks.sort_by_key(|def| def.rank);
        for pair in ranks.windows(2) {
            let (lower, higher) = (pair[0], pair[1]);
            // Two skills of one rank are reported as such.
            if lower.rank == higher.rank {
                continue;
            }
            out.extend(compare(lower, higher).into_iter().map(|problem| {
                let message = format!(
                    "skill \"{}\" (rank {}) must be \"{}\" (rank {}) with bigger numbers and nothing else: {problem}",
                    higher.id.0, higher.rank, lower.id.0, lower.rank
                );
                (higher.id.clone(), message)
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests;
