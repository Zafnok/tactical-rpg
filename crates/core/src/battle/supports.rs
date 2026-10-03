//! The battle's support rules (ticket 1002): who gains support points and
//! the Hit/Avoid bonus of a partner nearby. The rules are in the parent
//! module's docs; the numbers and the pure rules are in [`crate::support`].

use std::sync::Arc;

use super::skills::SkillStep;
use super::{BattleState, Event};
use crate::combat::CombatMods;
use crate::geom::Pos;
use crate::support::{SupportBook, SupportPair, SupportTable};
use crate::unit::{Faction, Unit, UnitId};

impl BattleState {
    /// Every pair's support, with the points this battle has given.
    pub fn supports(&self) -> &SupportBook {
        &self.bonds
    }

    /// The support rules and pairs.
    pub fn support_table(&self) -> &SupportTable {
        &self.tables.supports
    }

    /// The support pair of `a` and `b`: two player units whose characters
    /// have a support (so never a unit with itself).
    fn support_pair(&self, a: &Unit, b: &Unit) -> Option<SupportPair> {
        if a.faction != Faction::Player || b.faction != Faction::Player {
            return None;
        }
        let pair = SupportPair::new(a.character.clone()?, b.character.clone()?);
        self.tables.supports.get(&pair).map(|_| pair)
    }

    /// `unit`'s support partners on the map within `range` tiles of `pos`
    /// (where `unit` stands, or would), in unit order.
    fn partners_near(&self, unit: &Unit, pos: Pos, range: u32) -> Vec<(UnitId, SupportPair)> {
        self.units
            .iter()
            .filter(|other| Pos::manhattan(pos, other.pos) <= range)
            .filter_map(|other| Some((other.id, self.support_pair(unit, other)?)))
            .collect()
    }

    /// What `unit`, standing on `pos`, gets in a combat from its supports:
    /// the Hit and Avoid of its best-ranked partner in range.
    pub(super) fn support_mods(&self, unit: &Unit, pos: Pos) -> CombatMods {
        let rules = &self.tables.supports.rules;
        let ranks = self
            .partners_near(unit, pos, rules.bonus_range)
            .into_iter()
            .filter_map(|(_, pair)| self.bonds.rank(&pair));
        let bonus = rules.best_bonus(ranks);
        CombatMods {
            hit: bonus.hit,
            avoid: bonus.avoid,
            ..CombatMods::default()
        }
    }

    /// Gives units `a` and `b` (on the map) up to `amount` support points,
    /// if they are a support pair, with the event if any were gained.
    pub(super) fn give_support(
        &mut self,
        a: UnitId,
        b: UnitId,
        amount: u32,
        events: &mut Vec<Event>,
    ) {
        let (Some(first), Some(second)) = (self.unit(a), self.unit(b)) else {
            return;
        };
        let Some(pair) = self.support_pair(first, second) else {
            return;
        };
        let table = Arc::clone(&self.tables.supports);
        let amount = self.bonds.gain(&pair, amount, &table);
        if amount > 0 {
            events.push(Event::SupportPoints { a, b, amount });
        }
    }

    /// Gives `amount` points to each support pair made of one of `units`
    /// (those still on the map) and a partner adjacent to it: each pair
    /// once, in the order found, the unit of `units` first.
    fn support_beside(&mut self, units: &[UnitId], amount: u32, events: &mut Vec<Event>) {
        let mut found = Vec::new();
        let mut pairs = Vec::new();
        for unit in units.iter().filter_map(|&id| self.unit(id)) {
            for (other, pair) in self.partners_near(unit, unit.pos, 1) {
                if !found.contains(&pair) {
                    found.push(pair);
                    pairs.push((unit.id, other));
                }
            }
        }
        for (a, b) in pairs {
            self.give_support(a, b, amount, events);
        }
    }

    /// The points of ending the player phase adjacent, for each pair.
    pub(super) fn support_adjacent(&mut self, events: &mut Vec<Event>) {
        let amount = self.tables.supports.rules.points.adjacent_at_phase_end;
        let everyone: Vec<UnitId> = self.units.iter().map(|u| u.id).collect();
        self.support_beside(&everyone, amount, events);
    }

    /// The points of fighting beside a partner: for each of `fighters`
    /// still on the map, each partner adjacent to it. A pair gets them
    /// once, even if both fought (a target and a Line Pierce's victim).
    pub(super) fn support_fighters(&mut self, fighters: &[UnitId], events: &mut Vec<Event>) {
        let amount = self.tables.supports.rules.points.fight_beside;
        self.support_beside(fighters, amount, events);
    }

    /// The units a non-combat active helps, and the points each one's pair
    /// with the user gets for it.
    pub(super) fn skill_support(&self, effect: &SkillStep) -> (Vec<UnitId>, u32) {
        let points = &self.tables.supports.rules.points;
        match effect {
            SkillStep::Buff { targets, .. } => (targets.clone(), points.buff),
            SkillStep::Heal { heals } => (heals.iter().map(|h| h.0).collect(), points.heal),
            SkillStep::Push { .. } => (Vec::new(), 0),
        }
    }
}
