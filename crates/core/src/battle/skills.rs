//! The battle's skill rules (ticket 0311): combat actives, non-combat
//! actives, skill bonuses in combat, moves after an attack and timed-effect
//! expiry. The rules are in the parent module's docs.

use std::collections::{BTreeMap, VecDeque};

use super::arts::{ArtUse, check_arts_allowed};
use super::{BattleState, CommandError, Event, PendingMove, Step};
use crate::art::ArtId;
use crate::combat::{CombatMods, CombatantInput};
use crate::geom::Pos;
use crate::item::Equipped;
use crate::movement::TileSet;
use crate::skill::{
    ActiveEffect, Area, Bonuses, CostError, CostSource, EffectSource, SkillContext, SkillCost,
    SkillDef, SkillId, SkillKind, Stance, TimedEffect, TimedMods, auras, check_cost,
    effect_bonuses, heal_bonus, passive_bonuses, pay_cost, post_move_tiles,
};
use crate::stats::StatValue;
use crate::terrain::MovementTypeId;
use crate::unit::{Unit, UnitId};

/// An attack or attack-spell cast to validate.
pub(super) struct AttackPlan<'a> {
    /// Where the attacker ends its move.
    pub dest: Pos,
    /// Tiles it moved this turn.
    pub moved: u32,
    /// The unit attacked.
    pub target: UnitId,
    /// The weapon or spell it attacks with.
    pub with: Equipped,
    /// The combat active chosen, if any.
    pub active: Option<&'a SkillId>,
    /// The Combat Art chosen, if any.
    pub art: Option<&'a ArtId>,
}

/// What [`BattleState::fighters`] needs about the attacker's side.
pub(super) struct Fight<'a> {
    /// Where the attacker stands.
    pub dest: Pos,
    /// Tiles it moved this turn.
    pub moved: u32,
    /// The weapon or spell it attacks with.
    pub with: &'a Equipped,
    /// The validated combat active, if any.
    pub active: Option<&'a ActiveUse>,
    /// The validated Combat Art, if any.
    pub art: Option<&'a ArtUse>,
}

/// A validated use of an active skill: what to pay and what it does in a
/// combat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActiveUse {
    /// The skill.
    pub skill: SkillId,
    /// Its cost.
    pub cost: SkillCost,
    /// What pays it.
    pub source: CostSource,
    /// Combat bonuses (combat actives).
    pub mods: CombatMods,
    /// Max range bonus (combat actives).
    pub range: u32,
    /// Stance rider (combat actives).
    pub stance: Option<Stance>,
    /// Tiles the user may move after the attack (combat actives).
    pub post_move: u32,
    /// Heals the user by half the HP its strikes removed (combat actives).
    pub drain: bool,
}

/// What a validated non-combat active does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SkillStep {
    /// Puts a timed effect on each target.
    Buff {
        targets: Vec<UnitId>,
        mods: TimedMods,
    },
    /// Heals each unit by its amount.
    Heal { heals: Vec<(UnitId, StatValue)> },
    /// Moves `target` from `from` to `to`; a collision hurts it (and the
    /// unit it hit, `struck`) by `damage`.
    Push {
        target: UnitId,
        from: Pos,
        to: Pos,
        collided: Option<Pos>,
        struck: Option<UnitId>,
        damage: StatValue,
    },
}

/// Adds `bonuses` to a combatant.
fn apply(input: &mut CombatantInput, bonuses: &Bonuses) {
    input.stats = bonuses.apply(input.stats);
    input.mods.add(&bonuses.combat);
}

impl BattleState {
    /// The active skill `id` if `unit` can use it now: an id in the table
    /// that is one of its [usable](Unit::usable_active) actives.
    fn usable_active(&self, unit: &Unit, id: &SkillId) -> Result<&SkillDef, CommandError> {
        if self.tables.skills.get(id).is_none() {
            return Err(CommandError::UnknownSkill(id.clone()));
        }
        unit.usable_active(id, &self.tables.classes, &self.tables.skills)
            .ok_or_else(|| CommandError::SkillNotUsable {
                unit: unit.id,
                skill: id.clone(),
            })
    }

    /// Validates `unit` using combat active `id` in an attack with `with`.
    pub(super) fn plan_active(
        &self,
        unit: &Unit,
        id: &SkillId,
        with: &Equipped,
    ) -> Result<ActiveUse, CommandError> {
        let def = self.usable_active(unit, id)?;
        let SkillKind::Active {
            cost,
            effect:
                ActiveEffect::Strike {
                    with: needs,
                    mods,
                    range,
                    stance,
                    post_move,
                    drain,
                },
        } = &def.kind
        else {
            return Err(CommandError::WrongSkillKind(id.clone()));
        };
        let (kind, spell, source) = match with {
            Equipped::Weapon(slot) => (
                unit.loadout
                    .weapon(*slot)
                    .and_then(|w| self.tables.items.weapon(&w.def))
                    .map(|d| d.kind),
                false,
                CostSource::Weapon(*slot),
            ),
            Equipped::Spell(spell) => (None, true, CostSource::Spell(spell.clone())),
        };
        if !needs.allows(kind, spell) {
            return Err(CommandError::WrongWeaponForSkill(id.clone()));
        }
        check_cost(unit, *cost, &source).map_err(|error| CommandError::CannotPay {
            skill: id.clone(),
            error,
        })?;
        Ok(ActiveUse {
            skill: id.clone(),
            cost: *cost,
            source,
            mods: *mods,
            range: *range,
            stance: stance.clone(),
            post_move: *post_move,
            drain: *drain,
        })
    }

    /// Both sides of a combat as the combat maths sees them, with every
    /// skill bonus, and how far the attacker may move after it.
    pub(super) fn fighters(
        &self,
        attacker: &Unit,
        defender: &Unit,
        fight: &Fight,
    ) -> Result<(CombatantInput<'_>, CombatantInput<'_>, u32), CommandError> {
        let mut a = self.combatant(attacker, fight.dest, Some(fight.with))?;
        let mut d = self.combatant(defender, defender.pos, None)?;
        let kind = |c: &CombatantInput| c.weapon.as_ref().and_then(|w| w.kind);
        let (a_kind, d_kind) = (kind(&a), kind(&d));
        let a_ctx = SkillContext {
            weapon: a_kind,
            spell: matches!(fight.with, Equipped::Spell(_)),
            opponent_weapon: d_kind,
            own_phase: true,
            hp: attacker.hp,
            max_hp: attacker.stats.hp,
            moved: fight.moved,
        };
        let d_ctx = SkillContext {
            weapon: d_kind,
            spell: d.weapon.is_some() && defender.loadout.equipped_spell().is_some(),
            opponent_weapon: a_kind,
            own_phase: super::Phase::of(defender.faction) == self.phase,
            hp: defender.hp,
            max_hp: defender.stats.hp,
            moved: 0,
        };
        let a_usable = attacker.usable_skills(&self.tables.classes, &self.tables.skills);
        let mut a_bonus = self.bonuses(attacker, fight.dest, &a_usable, &a_ctx);
        if let Some(active) = fight.active {
            a_bonus.combat.add(&active.mods);
            // A stance rider counts in its own combat if it says so, once.
            if let Some(stance) = &active.stance
                && stance.this_combat
                && !attacker.has_effect(&EffectSource::Skill(active.skill.clone()))
            {
                a_bonus.add_timed(&stance.mods);
            }
            if let Some(w) = a.weapon.as_mut() {
                w.max_range = w.max_range.saturating_add(active.range);
            }
        }
        if let Some(art) = fight.art {
            // An art's stance counts in its own combat, once.
            if let Some(stance) = &art.effect.stance
                && !attacker.has_effect(&EffectSource::Art(art.art.clone()))
            {
                a_bonus.add_timed(stance);
            }
            if let Some(w) = a.weapon.as_mut() {
                art.effect.apply(w, &mut a_bonus.combat);
            }
        }
        apply(&mut a, &a_bonus);
        a.mods.add(&self.support_mods(attacker, fight.dest));
        d.mods.add(&self.support_mods(defender, defender.pos));
        let d_usable = defender.usable_skills(&self.tables.classes, &self.tables.skills);
        apply(
            &mut d,
            &self.bonuses(defender, defender.pos, &d_usable, &d_ctx),
        );
        let post_move =
            post_move_tiles(&a_usable, &a_ctx).max(fight.active.map_or(0, |x| x.post_move));
        Ok((a, d, post_move))
    }

    /// Every skill bonus of `unit` standing on `pos` in a combat: its
    /// passives (`usable`, judged in `ctx`), its timed effects and the ally
    /// auras reaching it (each aura skill once).
    fn bonuses(&self, unit: &Unit, pos: Pos, usable: &[&SkillDef], ctx: &SkillContext) -> Bonuses {
        let mut out = passive_bonuses(usable, ctx);
        out.add(&effect_bonuses(&unit.effects));
        let mut reaching: BTreeMap<&SkillId, &CombatMods> = BTreeMap::new();
        for other in &self.units {
            if other.id == unit.id || !other.faction.is_allied_to(unit.faction) {
                continue;
            }
            let theirs = other.usable_skills(&self.tables.classes, &self.tables.skills);
            for (id, radius, mods) in auras(&theirs) {
                if Pos::manhattan(other.pos, pos) <= radius {
                    reaching.entry(id).or_insert(mods);
                }
            }
        }
        for mods in reaching.values() {
            out.combat.add(mods);
        }
        out
    }

    /// Whether a unit of movement type `mt` can enter `pos`.
    fn enterable(&self, pos: Pos, mt: MovementTypeId) -> bool {
        self.map
            .tiles
            .get(pos)
            .and_then(|&t| self.tables.terrain.move_cost(t, mt))
            .is_some()
    }

    /// Where `unit` (on the map) can move after its attack with `tiles` of
    /// post-action move: every tile it reaches in `1..=tiles` steps through
    /// empty tiles it can enter, with the path there (its tile first), in
    /// the order found (breadth-first, neighbours in `Dir::ALL` order).
    fn move_after_paths(&self, unit: &Unit, tiles: u32) -> Vec<Vec<Pos>> {
        let Ok(class) = self.class_of(unit) else {
            return Vec::new();
        };
        let mt = class.movement_type;
        let free = |p: Pos| {
            self.enterable(p, mt) && !self.units.iter().any(|u| u.id != unit.id && u.pos == p)
        };
        let map = &self.map.tiles;
        let mut seen = TileSet::new(map.width(), map.height());
        seen.insert(unit.pos);
        let mut found = Vec::new();
        let mut queue = VecDeque::from([vec![unit.pos]]);
        while let Some(path) = queue.pop_front() {
            let Some(&last) = path.last() else {
                continue;
            };
            let steps = u32::try_from(path.len() - 1).unwrap_or(u32::MAX);
            if steps >= tiles {
                continue;
            }
            for next in map.neighbors4(last) {
                if free(next) && seen.insert(next) {
                    let mut longer = path.clone();
                    longer.push(next);
                    found.push(longer.clone());
                    queue.push_back(longer);
                }
            }
        }
        found
    }

    /// The tiles the unit waiting to move after its attack can move to (none
    /// if no unit is waiting).
    pub fn move_after_tiles(&self) -> Vec<Pos> {
        let Some(pending) = self.pending_move else {
            return Vec::new();
        };
        self.unit(pending.unit)
            .map(|u| self.move_after_paths(u, pending.tiles))
            .unwrap_or_default()
            .iter()
            .filter_map(|path| path.last().copied())
            .collect()
    }

    /// Offers unit `id` its move after an attack, if it still stands, has
    /// `tiles` of post-action move and somewhere to go: it waits for a
    /// [`Command::MoveAfter`](super::Command::MoveAfter).
    pub(super) fn offer_move_after(&mut self, id: UnitId, tiles: u32, events: &mut Vec<Event>) {
        // No tiles of move (an attack without one) find no paths.
        let can_move = self
            .unit(id)
            .is_some_and(|u| !self.move_after_paths(u, tiles).is_empty());
        if can_move {
            self.pending_move = Some(PendingMove { unit: id, tiles });
            events.push(Event::MoveAfterOffered { unit: id, tiles });
        }
    }

    /// Validates [`Command::MoveAfter`](super::Command::MoveAfter) for unit
    /// `id`: the path to `to` (its tile first), or `None` to stay.
    pub(super) fn plan_move_after(
        &self,
        id: UnitId,
        to: Option<Pos>,
    ) -> Result<Option<Vec<Pos>>, CommandError> {
        let pending = match self.pending_move {
            Some(p) if p.unit == id => p,
            Some(p) => return Err(CommandError::MoveAfterPending(p.unit)),
            None => return Err(CommandError::NoMoveAfter(id)),
        };
        let Some(to) = to else {
            return Ok(None);
        };
        self.unit(id)
            .map(|u| self.move_after_paths(u, pending.tiles))
            .unwrap_or_default()
            .into_iter()
            .find(|path| path.last() == Some(&to))
            .map(Some)
            .ok_or(CommandError::CannotMoveAfter(to))
    }

    /// Carries out a validated move after an attack by unit `id` along
    /// `path` (`None`: it stays). Ends its action.
    pub(super) fn move_after(
        &mut self,
        id: UnitId,
        path: Option<Vec<Pos>>,
        events: &mut Vec<Event>,
    ) {
        if let Some(path) = path {
            if let (Some(unit), Some(&to)) = (self.unit_mut(id), path.last()) {
                unit.pos = to;
            }
            events.push(Event::UnitMoved { unit: id, path });
        }
        self.pending_move = None;
        events.push(Event::UnitActed { unit: id });
    }

    /// The other units allied to `unit` within `radius` tiles of `dest`, in
    /// unit order.
    fn allies_near(&self, unit: &Unit, dest: Pos, radius: u32) -> Vec<&Unit> {
        self.units
            .iter()
            .filter(|u| {
                u.id != unit.id
                    && unit.faction.is_allied_to(u.faction)
                    && Pos::manhattan(dest, u.pos) <= radius
            })
            .collect()
    }

    /// Validates `unit` using non-combat active `id` from `dest` on
    /// `target`.
    pub(super) fn plan_skill(
        &self,
        unit: &Unit,
        dest: Pos,
        id: &SkillId,
        target: Option<UnitId>,
    ) -> Result<Step, CommandError> {
        check_arts_allowed(unit)?;
        let def = self.usable_active(unit, id)?;
        let SkillKind::Active { cost, effect } = &def.kind else {
            return Err(CommandError::WrongSkillKind(id.clone()));
        };
        // The cost is checked first, so a skill that can't be paid for says
        // so whoever is in reach. Its own uses need no weapon; any other
        // cost is the equipped weapon's.
        let source = match cost {
            SkillCost::Uses(_) => Ok(CostSource::Own(id.clone())),
            SkillCost::Durability(_) | SkillCost::ExtraSpellUse => unit
                .loadout
                .equipped_slot()
                .map(CostSource::Weapon)
                .ok_or(CostError::NoWeapon),
        }
        .and_then(|s| check_cost(unit, *cost, &s).map(|()| s))
        .map_err(|error| CommandError::CannotPay {
            skill: id.clone(),
            error,
        })?;
        let bad_target = || CommandError::BadSkillTarget(id.clone());
        let nobody = || CommandError::NoSkillTargets(id.clone());
        let step = match effect {
            ActiveEffect::Strike { .. } => return Err(CommandError::WrongSkillKind(id.clone())),
            ActiveEffect::Buff { area, mods } => {
                if target.is_some() {
                    return Err(bad_target());
                }
                let targets = match area {
                    Area::Own => vec![unit.id],
                    Area::Allies { radius } => self
                        .allies_near(unit, dest, *radius)
                        .iter()
                        .map(|u| u.id)
                        .collect(),
                };
                if targets.is_empty() {
                    return Err(nobody());
                }
                SkillStep::Buff {
                    targets,
                    mods: mods.clone(),
                }
            }
            ActiveEffect::Heal { radius, power } => {
                if target.is_some() {
                    return Err(bad_target());
                }
                // White Magic counts for Sanctuary too (Nick).
                let usable = unit.usable_skills(&self.tables.classes, &self.tables.skills);
                let mag = unit
                    .effective_stats(&self.tables.classes, &self.tables.items)
                    .mag
                    .saturating_add(heal_bonus(&usable));
                let heals: Vec<(UnitId, StatValue)> = self
                    .allies_near(unit, dest, *radius)
                    .iter()
                    .filter_map(|u| {
                        let missing = u.stats.hp - u.hp;
                        let amount = power.saturating_add(mag).clamp(0, missing.max(0));
                        (amount > 0).then_some((u.id, amount))
                    })
                    .collect();
                if heals.is_empty() {
                    return Err(nobody());
                }
                SkillStep::Heal { heals }
            }
            ActiveEffect::Push { collision } => {
                let target = target.ok_or_else(bad_target)?;
                self.plan_push(unit, dest, target, id, *collision)?
            }
        };
        Ok(Step::Skill {
            active: Box::new(ActiveUse {
                skill: id.clone(),
                cost: *cost,
                source,
                mods: CombatMods::default(),
                range: 0,
                stance: None,
                post_move: 0,
                drain: false,
            }),
            effect: step,
        })
    }

    /// Validates `unit` at `dest` pushing unit `target` with skill `id`,
    /// whose collisions deal `collision` damage.
    fn plan_push(
        &self,
        unit: &Unit,
        dest: Pos,
        target: UnitId,
        id: &SkillId,
        collision: StatValue,
    ) -> Result<SkillStep, CommandError> {
        let other = self.living(target)?;
        if !unit.faction.is_hostile_to(other.faction) || Pos::manhattan(dest, other.pos) != 1 {
            return Err(CommandError::BadSkillTarget(id.clone()));
        }
        let from = other.pos;
        let to = Pos::new(2 * from.x - dest.x, 2 * from.y - dest.y);
        let mt = self.class_of(other)?.movement_type;
        // The pusher has left its old tile; the pushed unit leaves `from`.
        let free = |p: Pos| {
            self.enterable(p, mt)
                && !self
                    .units
                    .iter()
                    .any(|u| u.id != unit.id && u.id != target && u.pos == p)
        };
        let struck = self
            .units
            .iter()
            .find(|u| u.id != unit.id && u.pos == to)
            .map(|u| u.id);
        let landing = if self.burning.iter().any(|b| b.pos == to) {
            // `from` is next to the fire and free, so a landing is always
            // found by distance 1.
            self.map.tiles.neighbors4(to).find(|&p| free(p))
        } else if free(to) {
            None
        } else {
            // Blocked: it stays where it is.
            Some(from)
        };
        if let Some(landing) = landing {
            return Ok(SkillStep::Push {
                target,
                from,
                to: landing,
                collided: Some(to),
                struck,
                damage: collision,
            });
        }
        Ok(SkillStep::Push {
            target,
            from,
            to,
            collided: None,
            struck: None,
            damage: 0,
        })
    }

    /// Emits [`Event::SkillUsed`] and pays for `active` (validated) by unit
    /// `id`. Returns the [`Event::ItemBroke`] to emit after the action.
    pub(super) fn pay(
        &mut self,
        id: UnitId,
        active: &ActiveUse,
        events: &mut Vec<Event>,
    ) -> Option<Event> {
        events.push(Event::SkillUsed {
            unit: id,
            skill: active.skill.clone(),
        });
        let unit = self.unit_mut(id)?;
        let paid = pay_cost(unit, active.cost, &active.source).ok()?;
        events.extend(paid.events);
        paid.broke
    }

    /// A combat active's effects after its combat, if the user still
    /// stands: its stance rider, then its drain (`dealt`: HP removed by
    /// `[attacker, defender]`).
    pub(super) fn after_strike(
        &mut self,
        id: UnitId,
        active: &ActiveUse,
        dealt: [StatValue; 2],
        events: &mut Vec<Event>,
    ) {
        let until = self.phase;
        let Some(unit) = self.unit_mut(id).filter(|u| u.hp > 0) else {
            return;
        };
        if let Some(stance) = &active.stance {
            let source = EffectSource::Skill(active.skill.clone());
            unit.add_effect(TimedEffect {
                source: source.clone(),
                mods: stance.mods.clone(),
                until,
            });
            events.push(Event::EffectApplied {
                unit: id,
                source,
                until,
            });
        }
        if active.drain {
            let amount = (dealt[0] / 2).clamp(0, (unit.stats.hp - unit.hp).max(0));
            if amount > 0 {
                unit.hp += amount;
                events.push(Event::Healed { target: id, amount });
            }
        }
    }

    /// Carries out unit `id`'s validated non-combat active. Units a Shove
    /// felled leave the map after the payment's `ItemBroke` (a cost in
    /// durability only).
    pub(super) fn use_skill(
        &mut self,
        id: UnitId,
        active: &ActiveUse,
        effect: SkillStep,
        events: &mut Vec<Event>,
    ) {
        let broke = self.pay(id, active, events);
        let skill = &active.skill;
        // Units a collision may have felled, in order.
        let mut hurt = Vec::new();
        match effect {
            SkillStep::Buff { targets, mods } => {
                let until = self.phase;
                for target in targets {
                    if let Some(u) = self.unit_mut(target) {
                        let source = EffectSource::Skill(skill.clone());
                        u.add_effect(TimedEffect {
                            source: source.clone(),
                            mods: mods.clone(),
                            until,
                        });
                        events.push(Event::EffectApplied {
                            unit: target,
                            source,
                            until,
                        });
                    }
                }
            }
            SkillStep::Heal { heals } => {
                for (target, amount) in heals {
                    if let Some(u) = self.unit_mut(target) {
                        u.hp += amount;
                        events.push(Event::Healed { target, amount });
                    }
                }
            }
            SkillStep::Push {
                target,
                from,
                to,
                collided,
                struck,
                damage,
            } => {
                if let Some(u) = self.unit_mut(target) {
                    let lost = damage.clamp(0, u.hp);
                    u.pos = to;
                    u.hp -= lost;
                    events.push(Event::Pushed {
                        unit: target,
                        from,
                        to,
                        collided,
                        damage: lost,
                    });
                }
                if let Some(u) = struck.and_then(|id| self.unit_mut(id)) {
                    let lost = damage.clamp(0, u.hp);
                    u.hp -= lost;
                    events.push(Event::CollisionDamage {
                        unit: u.id,
                        by: target,
                        damage: lost,
                    });
                }
                hurt.extend(std::iter::once(target).chain(struck));
            }
        }
        events.extend(broke);
        self.remove_fallen(&hurt, events);
    }

    /// Ends the timed effects that last until the current phase starts.
    pub(super) fn expire_effects(&mut self, events: &mut Vec<Event>) {
        let phase = self.phase;
        for u in &mut self.units {
            for source in u.expire_effects(phase) {
                events.push(Event::EffectExpired { unit: u.id, source });
            }
        }
    }
}
