//! The battle's Combat Art rules (ticket 0312): validating an art for an
//! attack, paying for it, its stance and debuff, Line Pierce's strike, the
//! boss check and the attack preview. The rules are in the parent module's
//! docs.

use super::{BattleState, CommandError, Event, Fight, Phase, Pos, Step, UnitAction};
use crate::art::{ArtEffect, ArtId, ArtNote};
use crate::combat::{CombatHp, Forecast, SideForecast, StrikePlan, forecast, if_all_hit};
use crate::item::{Equipped, ItemId};
use crate::skill::{
    CostSource, EffectSource, SkillCost, SkillId, TimedEffect, check_cost, pay_cost,
};
use crate::unit::{Unit, UnitId};

/// A validated use of a Combat Art in an attack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ArtUse {
    /// The art.
    pub art: ArtId,
    /// The attacking weapon's loadout slot, which pays.
    pub slot: usize,
    /// The attacking weapon.
    pub weapon: ItemId,
    /// The durability it costs.
    pub cost: u32,
    /// The weapon's durability before and after paying.
    pub durability: (u32, u32),
    /// What it does.
    pub effect: ArtEffect,
}

/// What an attack would do, for the UI's forecast panel
/// ([`BattleState::preview_attack`]): the numbers, the art or active
/// chosen, what it costs and the effects the numbers don't show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttackPreview {
    /// The combat's numbers, with every skill and art bonus.
    pub forecast: Forecast,
    /// The Combat Art chosen, if any.
    pub art: Option<ArtId>,
    /// The combat active chosen, if any.
    pub active: Option<SkillId>,
    /// The attacking weapon's durability before and after, when the art or
    /// active costs durability (`Guard Break (20 → 16)`).
    pub durability: Option<(u32, u32)>,
    /// The art's effects that aren't numbers (`no counter`, `pierces`…).
    pub notes: Vec<ArtNote>,
    /// Line Pierce's strike: the unit behind the target and the strike's
    /// numbers, if there is one to hit.
    pub pierce: Option<(UnitId, SideForecast)>,
    /// The combat if every strike hit and none crit, from both units'
    /// current HP (the forecast's strike list and kill mark).
    pub plan: StrikePlan,
}

/// Refuses a unit that may not use arts and active skills
/// ([`Unit::may_use_arts`]).
pub(super) fn check_arts_allowed(unit: &Unit) -> Result<(), CommandError> {
    if unit.may_use_arts() {
        Ok(())
    } else {
        Err(CommandError::ArtsNotAllowed(unit.id))
    }
}

impl BattleState {
    /// Validates `unit` using art `id` in an attack with `with`.
    pub(super) fn plan_art(
        &self,
        unit: &Unit,
        id: &ArtId,
        with: &Equipped,
    ) -> Result<ArtUse, CommandError> {
        let tables = &self.tables;
        let def = tables
            .arts
            .get(id)
            .ok_or_else(|| CommandError::UnknownArt(id.clone()))?;
        let not_usable = || CommandError::ArtNotUsable {
            unit: unit.id,
            art: id.clone(),
        };
        // Arts need a weapon: never with a spell.
        let Equipped::Weapon(slot) = *with else {
            return Err(not_usable());
        };
        let fits = unit
            .arts_for(slot, &tables.classes, &tables.items, &tables.arts)
            .contains(&def);
        let copy = unit
            .loadout
            .weapon(slot)
            .filter(|_| fits)
            .ok_or_else(not_usable)?;
        check_cost(unit, def.skill_cost(), &CostSource::Weapon(slot)).map_err(|error| {
            CommandError::CannotPayArt {
                art: id.clone(),
                error,
            }
        })?;
        let before = copy.durability_left;
        Ok(ArtUse {
            art: id.clone(),
            slot,
            weapon: copy.def.clone(),
            cost: def.cost,
            durability: (before, before.saturating_sub(def.cost)),
            effect: def.effect.clone(),
        })
    }

    /// Line Pierce's strike for `unit` attacking `defender` as `fight` says,
    /// from `distance` tiles: the hostile unit on the tile directly behind
    /// the target (one step past it, on a straight or diagonal line from
    /// `dest`; any other angle has none) and the numbers of one strike at
    /// it, with no counter. The strike ignores range, so its forecast is
    /// worked out at the target's distance.
    pub(super) fn plan_pierce(
        &self,
        unit: &Unit,
        defender: &Unit,
        fight: &Fight,
        distance: u32,
    ) -> Result<Option<(UnitId, Forecast)>, CommandError> {
        let (dx, dy) = (defender.pos.x - fight.dest.x, defender.pos.y - fight.dest.y);
        if dx != 0 && dy != 0 && dx.abs() != dy.abs() {
            return Ok(None);
        }
        let behind = Pos::new(defender.pos.x + dx.signum(), defender.pos.y + dy.signum());
        // The attacker has left its own tile, and is never hostile to itself.
        let Some(victim) = self
            .units
            .iter()
            .find(|u| u.pos == behind && unit.faction.is_hostile_to(u.faction))
        else {
            return Ok(None);
        };
        let (a, v, _) = self.fighters(unit, victim, fight)?;
        let rules = self.tables.items.combat_rules();
        Ok(forecast(&rules, &a, &v, distance).map(|mut f| {
            f.defender = None;
            f.attacker.strikes = 1;
            (victim.id, f)
        }))
    }

    /// Emits [`Event::ArtUsed`], pays for `art` (validated) by unit `id` and
    /// puts on its stance. Returns the [`Event::ItemBroke`] to emit after
    /// the combat.
    pub(super) fn commit_art(
        &mut self,
        id: UnitId,
        art: &ArtUse,
        events: &mut Vec<Event>,
    ) -> Option<Event> {
        events.push(Event::ArtUsed {
            unit: id,
            art: art.art.clone(),
            weapon: art.weapon.clone(),
            durability_before: art.durability.0,
            durability_after: art.durability.1,
        });
        let until = self.phase;
        let unit = self.unit_mut(id)?;
        let paid = pay_cost(
            unit,
            SkillCost::Durability(art.cost),
            &CostSource::Weapon(art.slot),
        )
        .ok()?;
        events.extend(paid.events);
        if let Some(stance) = &art.effect.stance {
            let source = EffectSource::Art(art.art.clone());
            unit.add_effect(TimedEffect {
                source: source.clone(),
                mods: stance.clone(),
                until,
            });
            events.push(Event::EffectApplied {
                unit: id,
                source,
                until,
            });
        }
        paid.broke
    }

    /// Puts `art`'s debuff, if it has one, on `target` if it still stands,
    /// until the end of its side's next phase.
    pub(super) fn debuff(&mut self, target: UnitId, art: &ArtUse, events: &mut Vec<Event>) {
        let Some(debuff) = art.effect.on_first_hit else {
            return;
        };
        let Some(unit) = self.unit_mut(target).filter(|u| u.hp > 0) else {
            return;
        };
        let until = TimedEffect::debuff_until(Phase::of(unit.faction));
        let source = EffectSource::Art(art.art.clone());
        unit.add_effect(TimedEffect {
            source: source.clone(),
            mods: debuff.mods(),
            until,
        });
        events.push(Event::EffectApplied {
            unit: target,
            source,
            until,
        });
    }

    /// What `unit` moving to `dest` and doing `action` (an attack, or an
    /// attack spell cast at a unit) would do, without doing it. It is
    /// validated exactly as [`BattleState::apply`] would validate the `Act`,
    /// with the same errors; any other action is [`CommandError::NotAnAttack`].
    pub fn preview_attack(
        &self,
        unit: UnitId,
        dest: Pos,
        action: &UnitAction,
    ) -> Result<AttackPreview, CommandError> {
        let (_, step) = self.plan(unit, dest, action)?;
        let Step::Attack(step) = step else {
            return Err(CommandError::NotAnAttack);
        };
        let active_durability = step.active.as_ref().and_then(|a| {
            let (SkillCost::Durability(cost), CostSource::Weapon(slot)) = (a.cost, &a.source)
            else {
                return None;
            };
            let left = self.unit(unit)?.loadout.weapon(*slot)?.durability_left;
            Some((left, left.saturating_sub(cost)))
        });
        let hp = |id| {
            self.unit(id)
                .map_or(CombatHp { current: 0, max: 0 }, |u| CombatHp {
                    current: u.hp,
                    max: u.stats.hp,
                })
        };
        Ok(AttackPreview {
            plan: if_all_hit(&step.forecast, hp(unit), hp(step.target)),
            forecast: step.forecast,
            art: step.art.as_ref().map(|a| a.art.clone()),
            active: step.active.as_ref().map(|a| a.skill.clone()),
            durability: step
                .art
                .as_ref()
                .map(|a| a.durability)
                .or(active_durability),
            notes: step
                .art
                .as_ref()
                .map(|a| a.effect.notes())
                .unwrap_or_default(),
            pierce: step.pierce.map(|(victim, f)| (victim, f.attacker)),
        })
    }
}
