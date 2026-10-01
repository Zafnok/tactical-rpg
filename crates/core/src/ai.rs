//! The AI that plays the Enemy and Other phases (ticket 0501): which unit
//! acts next, where it moves and what it does. Pure and deterministic
//! (ADR-0004): it reads a [`BattleState`] and returns the [`Command`] to
//! apply, the same commands the player issues. The same state always gives
//! the same command.
//!
//! # Rules
//!
//! [`next_command`] plans against the current state on every call:
//!
//! - **Battle over:** `None`.
//! - **A unit waiting to move after its attack**
//!   ([`BattleState::pending_move`]) answers first, with
//!   [`Command::MoveAfter`]: to the tile, among its own and
//!   [`BattleState::move_after_tiles`], farthest (Manhattan) from the
//!   nearest unit hostile to it; its own tile on a tie, then the first
//!   tile in the order given.
//! - Otherwise every **ready** unit of the current phase (not yet acted;
//!   reinforcements that arrived this phase are already done) decides what
//!   it would do (below), and the first in this **order** acts: units that
//!   attack, highest score first; then the others, lowest flow distance
//!   (below) from their tile first; then lowest id. When no unit is left to
//!   act, `None`: the AI is done and the caller ends the phase with
//!   [`Command::EndPhase`].
//! - **Targets** are the units hostile to the acting unit
//!   ([`Faction::is_hostile_to`]): a Neutral unit has none.
//!
//! It works in any phase. The game asks it only in the Enemy and Other
//! phases; the soak tests also let it play the player's side.
//!
//! ## A unit's decision
//!
//! From the tiles it can stop on (only its own tile when
//! [`Stationary`](AiBehavior::Stationary)):
//!
//! 1. [`Healer`](AiBehavior::Healer): casts a heal spell with uses left on
//!    the most injured ally it can reach (most HP missing, then lowest id),
//!    from the tile outside the danger zone if it can, then with the best
//!    terrain, then the lowest `(y, x)`, then the lowest spell id. Else it
//!    attacks, if it can. Else it moves to the tile outside the danger zone
//!    if it can, then nearest (Manhattan) to an ally, then cheapest to
//!    reach, then the lowest `(y, x)`, and waits. The danger zone is every
//!    tile a hostile unit could attack this turn
//!    ([`danger_zone`]).
//! 2. [`Aggressive`](AiBehavior::Aggressive): attacks if it can. Else it
//!    moves to the tile with the lowest flow distance (then cheapest to
//!    reach, so it stays put rather than move for nothing, then lowest
//!    `(y, x)`) and waits; with no target it can walk to, it waits where
//!    it is.
//! 3. [`Guard`](AiBehavior::Guard): attacks if it can (a target in its
//!    threat area this turn), else holds its tile (below).
//!    [`Stationary`](AiBehavior::Stationary): the same, from its own tile.
//!
//! A unit **holding its tile** waits there. One that may use arts and
//! actives (below) first uses a non-combat active (Brace, War Cry…) if a
//! hostile unit could attack its tile this turn (the danger zone): the
//! first the battle accepts, lowest skill id first, with no target and then
//! on each unit (lowest id first). It never uses one that would spend its
//! equipped weapon's last durability.
//!
//! It never uses consumables: enemies carry none, and only healers heal
//! (Nick, `weapons-and-items.md`).
//!
//! ## Attack choice
//!
//! Every stoppable tile × every target in range × every weapon the unit can
//! wield and attack spell with uses left (tile casts are never used) is
//! scored from its forecast:
//!
//! ```text
//! score = damage × E[HP dealt] + kill × P(kill) + lord × (target is a lord)
//!       − risk × E[HP taken] + terrain × (tile's Def + Avoid / 10)
//! ```
//!
//! with the weights of [`AiWeights`] (`assets/data/ai.ron`). The
//! expectations play the forecast's strikes out in order, each hitting with
//! its displayed hit chance (follow-ups included), ignoring crits and
//! stopping when a unit falls, as combat does. The highest score wins; ties
//! go to the lowest `(target id, y, x, weapon or spell id, slot)`, then to
//! the plain attack, then to the lowest art or active id (an active before
//! an art of the same id). A unit that can attack always does, whatever the
//! score.
//!
//! ## Arts and actives (ticket 0503)
//!
//! Only the units the battle lets use them do ([`Unit::may_use_arts`],
//! `combat-arts.md`): bosses among enemies and combat green units (and
//! player units, when the AI plays them). For such a unit every attack
//! above is also scored with each Combat Art it can use with that weapon
//! and each of its combat actives, one per attack, from the forecast the
//! battle gives with it (its range changes included):
//!
//! ```text
//! score = the attack's score with the art or active
//!       + damage × E[HP dealt] + kill × P(kill) + lord   (Line Pierce's strike)
//!       − durability × its durability cost   (or − spell_use, a spell active)
//! ```
//!
//! Line Pierce's strike at the unit behind the target is counted as if the
//! attacker is still standing after the combat. What an art or active does
//! beyond the forecast's numbers (a debuff, a stance on later turns, a move
//! or healing after the attack) scores nothing.
//!
//! ## Flow distance
//!
//! For each tile, the cheapest cost for the mover's movement type to walk
//! from it to a tile next to one of its targets, ignoring units (Dijkstra
//! from every target at once). A tile it can't walk from has none.

use std::cell::RefCell;
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap};
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::art::ArtId;
use crate::battle::{BattleState, CastTarget, Command, PendingMove, Phase, UnitAction};
use crate::combat::{CombatHp, Forecast, Side, SideForecast, strike_order};
use crate::geom::{Grid, Pos};
use crate::item::{Equipped, WEAPON_SLOTS};
use crate::magic::Affinity;
use crate::movement::{Reach, TileSet, danger_zone, reachable};
use crate::skill::{ActiveEffect, SkillCost, SkillId, SkillKind};
use crate::spell::{SpellId, SpellKind};
use crate::stats::StatValue;
use crate::terrain::MovementTypeId;
use crate::unit::{Faction, Unit, UnitId};

/// How an AI unit behaves. Set per unit by chapter data.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub enum AiBehavior {
    /// Attacks the best target in reach, else charges the nearest.
    #[default]
    Aggressive,
    /// Attacks only a target in its threat area this turn, else holds.
    Guard,
    /// Never moves: attacks only from its own tile (bosses on forts).
    Stationary,
    /// Heals the most injured ally in reach, else keeps out of danger near
    /// its allies.
    Healer,
}

/// The AI's tunable numbers (`assets/data/ai.ron`). See the module docs for
/// how each is used. The default is all zeros (every attack scores 0), a
/// placeholder for when the file fails to load; the game's values are
/// [`AiWeights::STARTING`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiWeights {
    /// Score per expected HP of damage dealt.
    pub damage: u32,
    /// Score for a certain kill (times the kill chance).
    pub kill: u32,
    /// Score for attacking a lord.
    pub lord: u32,
    /// Score lost per expected HP of damage taken.
    pub risk: u32,
    /// Score per point of the attacking tile's Def + Avoid / 10.
    pub terrain: u32,
    /// Score lost per point of durability an art or active costs.
    pub durability: u32,
    /// Score lost for the extra spell use a spell active costs.
    pub spell_use: u32,
}

impl AiWeights {
    /// The starting values (*tunable*), as in `assets/data/ai.ron`.
    pub const STARTING: AiWeights = AiWeights {
        damage: 10,
        kill: 300,
        lord: 50,
        risk: 5,
        terrain: 5,
        durability: 5,
        spell_use: 15,
    };
}

/// The command the unit acting next issues, planned against `state` (see
/// the module docs): a [`Command::MoveAfter`] or a unit's [`Command::Act`],
/// always one `state` accepts. `None` when the AI has nothing left to do:
/// the battle is over, or no unit of the phase is left to act (the caller
/// then applies [`Command::EndPhase`]).
pub fn next_command(state: &BattleState, weights: &AiWeights) -> Option<Command> {
    if state.outcome().is_some() {
        return None;
    }
    let command = if let Some(pending) = state.pending_move() {
        move_after(state, pending)
    } else {
        let planner = Planner::new(state, weights);
        state
            .units()
            .iter()
            .filter(|u| Phase::of(u.faction) == state.phase() && !u.acted)
            .filter_map(|u| planner.decide(u))
            .min_by(Decision::order)?
            .command
    };
    debug_assert!(
        state.clone().apply(&command).is_ok(),
        "the AI chose a command the battle refuses: {command:?}"
    );
    Some(command)
}

/// The waiting unit's move after its attack (see the module docs).
fn move_after(state: &BattleState, pending: PendingMove) -> Command {
    let unit = pending.unit;
    let to = state.unit(unit).and_then(|u| {
        let hostiles: Vec<Pos> = hostiles_of(state, u).map(|h| h.pos).collect();
        let room = |p: Pos| {
            hostiles
                .iter()
                .map(|&h| Pos::manhattan(h, p))
                .min()
                .unwrap_or(u32::MAX)
        };
        let mut best = (room(u.pos), None);
        for tile in state.move_after_tiles() {
            let r = room(tile);
            if r > best.0 {
                best = (r, Some(tile));
            }
        }
        best.1
    });
    Command::MoveAfter { unit, to }
}

/// The units hostile to `unit`.
fn hostiles_of<'a>(state: &'a BattleState, unit: &'a Unit) -> impl Iterator<Item = &'a Unit> {
    state
        .units()
        .iter()
        .filter(|t| unit.faction.is_hostile_to(t.faction))
}

/// What one ready unit would do, and where that puts it in the order.
#[derive(Debug, Clone)]
struct Decision {
    unit: UnitId,
    rank: Rank,
    command: Command,
}

/// A decision's place in the order, before the unit id.
#[derive(Debug, Clone, Copy)]
enum Rank {
    /// An attack with this score: first, highest first.
    Attack(f64),
    /// Anything else, by flow distance from the unit's tile: lowest first.
    Other(u32),
}

impl Decision {
    /// Which of `a` and `b` acts first (`Less`: `a`).
    fn order(a: &Decision, b: &Decision) -> Ordering {
        let rank = match (a.rank, b.rank) {
            (Rank::Attack(x), Rank::Attack(y)) => y.total_cmp(&x),
            (Rank::Attack(_), Rank::Other(_)) => Ordering::Less,
            (Rank::Other(_), Rank::Attack(_)) => Ordering::Greater,
            (Rank::Other(x), Rank::Other(y)) => x.cmp(&y),
        };
        rank.then(a.unit.cmp(&b.unit))
    }
}

/// A Combat Art or a combat active used with an attack.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Technique {
    Active(SkillId),
    Art(ArtId),
}

impl Technique {
    /// Its id.
    fn id(&self) -> &str {
        match self {
            Technique::Active(skill) => &skill.0,
            Technique::Art(art) => &art.0,
        }
    }
}

/// A technique an [`Arm`] can attack with.
struct Extra {
    technique: Technique,
    /// Added to the arm's max range.
    range: u32,
    /// Score its cost loses.
    penalty: f64,
}

/// Something a unit can attack with, and its range.
struct Arm {
    with: Equipped,
    min: u32,
    max: u32,
    /// The arts and combat actives to try with it, in tie-break order (none
    /// for a unit that may not use them).
    extras: Vec<Extra>,
}

impl Arm {
    /// The action attacking `target` with it, using `technique`.
    fn action(&self, target: UnitId, technique: Option<&Technique>) -> UnitAction {
        let (mut active, mut art) = (None, None);
        match technique {
            Some(Technique::Active(skill)) => active = Some(skill.clone()),
            Some(Technique::Art(id)) => art = Some(id.clone()),
            None => {}
        }
        match &self.with {
            Equipped::Weapon(slot) => UnitAction::Attack {
                target,
                slot: *slot,
                active,
                art,
            },
            Equipped::Spell(spell) => UnitAction::Cast {
                spell: spell.clone(),
                target: CastTarget::Unit(target),
                active,
            },
        }
    }

    /// The farthest it reaches with any of its techniques.
    fn reach(&self) -> u32 {
        let bonus = self.extras.iter().map(|e| e.range).max().unwrap_or(0);
        self.max.saturating_add(bonus)
    }
}

/// Flow distances by tile (see the module docs).
type Flow = Grid<Option<u32>>;

/// One `next_command` call's planning: the state and what is worth
/// working out once for every unit (flow fields, danger zones).
struct Planner<'a> {
    state: &'a BattleState,
    weights: &'a AiWeights,
    flows: RefCell<BTreeMap<(Faction, MovementTypeId), Rc<Flow>>>,
    dangers: RefCell<BTreeMap<Faction, Rc<TileSet>>>,
}

impl<'a> Planner<'a> {
    fn new(state: &'a BattleState, weights: &'a AiWeights) -> Self {
        Planner {
            state,
            weights,
            flows: RefCell::default(),
            dangers: RefCell::default(),
        }
    }

    /// What `unit` would do, or `None` if it can't act (its class or tile
    /// is unknown to the battle).
    fn decide(&self, unit: &Unit) -> Option<Decision> {
        let state = self.state;
        let class = state.classes().get(&unit.class)?;
        let reach = reachable(
            state.map(),
            state.terrain(),
            state.classes(),
            state.units(),
            unit.id,
        )
        .ok()?;
        let dests: Vec<Pos> = if unit.ai == AiBehavior::Stationary {
            vec![unit.pos]
        } else {
            reach.stoppable().iter().collect()
        };
        let flow = self.flow(unit.faction, class.movement_type);
        let distance = flow.get(unit.pos).copied().flatten().unwrap_or(u32::MAX);
        let other = |command| Decision {
            unit: unit.id,
            rank: Rank::Other(distance),
            command,
        };
        let wait_at = |dest| Command::Act {
            unit: unit.id,
            dest,
            action: UnitAction::Wait,
        };
        let attack = self
            .best_attack(unit, &reach, &dests)
            .map(|(score, command)| Decision {
                unit: unit.id,
                rank: Rank::Attack(score),
                command,
            });
        Some(match unit.ai {
            AiBehavior::Healer => match self.heal(unit, &dests) {
                Some(command) => other(command),
                None => {
                    attack.unwrap_or_else(|| other(wait_at(self.retreat(unit, &reach, &dests))))
                }
            },
            AiBehavior::Aggressive => attack.unwrap_or_else(|| {
                other(wait_at(approach(&reach, &dests, &flow).unwrap_or(unit.pos)))
            }),
            AiBehavior::Guard | AiBehavior::Stationary => attack
                .unwrap_or_else(|| other(self.stand(unit).unwrap_or_else(|| wait_at(unit.pos)))),
        })
    }

    /// The best attack `unit` can make from `dests`, with its score.
    fn best_attack(&self, unit: &Unit, reach: &Reach, dests: &[Pos]) -> Option<(f64, Command)> {
        let state = self.state;
        let arms = arms(state, unit, self.weights);
        let longest = arms.iter().map(Arm::reach).max()?;
        let mut best: Option<(f64, Command)> = None;
        let mut targets: Vec<&Unit> = hostiles_of(state, unit).collect();
        targets.sort_by_key(|t| t.id);
        let own = CombatHp {
            current: unit.hp,
            max: unit.stats.hp,
        };
        for target in targets {
            let theirs = CombatHp {
                current: target.hp,
                max: target.stats.hp,
            };
            for &dest in dests {
                let distance = Pos::manhattan(dest, target.pos);
                if distance > longest {
                    continue;
                }
                let path = reach.path_to(dest).unwrap_or_default();
                let moved = u32::try_from(path.len().saturating_sub(1)).unwrap_or(u32::MAX);
                let terrain = terrain_bonus(state, dest);
                for arm in &arms {
                    let mut offer = |score: f64, action: UnitAction| {
                        if best.as_ref().is_none_or(|(top, _)| score > *top) {
                            let command = Command::Act {
                                unit: unit.id,
                                dest,
                                action,
                            };
                            best = Some((score, command));
                        }
                    };
                    if (arm.min..=arm.max).contains(&distance)
                        && let Some(forecast) =
                            state.plain_forecast(unit, dest, moved, target, &arm.with)
                    {
                        let odds = Odds::of(&forecast, own, theirs);
                        let score = self.score(&odds, target.is_lord, terrain);
                        offer(score, arm.action(target.id, None));
                    }
                    for extra in &arm.extras {
                        let action = arm.action(target.id, Some(&extra.technique));
                        let Some((forecast, pierce)) =
                            state.attack_forecast(unit, dest, &path, &action)
                        else {
                            continue;
                        };
                        let odds = Odds::of(&forecast, own, theirs);
                        let score = self.score(&odds, target.is_lord, terrain)
                            + self.pierce_score(own, pierce)
                            - extra.penalty;
                        offer(score, action);
                    }
                }
            }
        }
        best
    }

    /// What Line Pierce's strike `pierce` (the unit behind the target and
    /// the strike's forecast) adds to an attack's score (see the module
    /// docs).
    fn pierce_score(&self, own: CombatHp, pierce: Option<(UnitId, Forecast)>) -> f64 {
        let Some((forecast, victim)) = pierce.and_then(|(id, f)| Some((f, self.state.unit(id)?)))
        else {
            return 0.0;
        };
        let theirs = CombatHp {
            current: victim.hp,
            max: victim.stats.hp,
        };
        self.score(&Odds::of(&forecast, own, theirs), victim.is_lord, 0.0)
    }

    /// The non-combat active `unit`, holding its tile, uses instead of
    /// waiting, if any (see the module docs).
    fn stand(&self, unit: &Unit) -> Option<Command> {
        let state = self.state;
        if !unit.may_use_arts() || !self.danger(unit.faction).contains(unit.pos) {
            return None;
        }
        let slot = unit.loadout.equipped_slot()?;
        let left = unit.loadout.weapon(slot)?.durability_left;
        let mut ids: Vec<UnitId> = state.units().iter().map(|u| u.id).collect();
        ids.sort();
        let targets: Vec<Option<UnitId>> = std::iter::once(None)
            .chain(ids.into_iter().map(Some))
            .collect();
        let action = unit
            .usable_skills(state.classes(), state.skills())
            .into_iter()
            .filter(|def| {
                let spares_weapon = matches!(
                    def.kind,
                    SkillKind::Active { cost: SkillCost::Durability(cost), .. } if cost < left
                );
                spares_weapon && !def.is_combat()
            })
            .flat_map(|def| {
                targets.iter().map(|&target| UnitAction::UseSkill {
                    skill: def.id.clone(),
                    target,
                })
            })
            .find(|action| {
                state
                    .check_action(unit, unit.pos, &[unit.pos], action)
                    .is_ok()
            })?;
        Some(Command::Act {
            unit: unit.id,
            dest: unit.pos,
            action,
        })
    }

    /// An attack's score (see the module docs).
    fn score(&self, odds: &Odds, lord: bool, terrain: f64) -> f64 {
        let w = self.weights;
        let lord = if lord { f64::from(w.lord) } else { 0.0 };
        f64::from(w.damage) * odds.dealt + f64::from(w.kill) * odds.kill + lord
            - f64::from(w.risk) * odds.taken
            + f64::from(w.terrain) * terrain
    }

    /// A healer's heal of the most injured ally it can reach, if any.
    fn heal(&self, unit: &Unit, dests: &[Pos]) -> Option<Command> {
        let state = self.state;
        let spells: Vec<_> = unit
            .learned
            .iter()
            .filter(|s| unit.spells.uses_left(s) > 0)
            .filter_map(|s| state.spells().get(s))
            .filter(|def| matches!(def.kind, SpellKind::Heal { .. }))
            .collect();
        if spells.is_empty() {
            return None;
        }
        let mut wounded: Vec<&Unit> = state
            .units()
            .iter()
            .filter(|a| {
                a.id != unit.id && unit.faction.is_allied_to(a.faction) && a.hp < a.stats.hp
            })
            .collect();
        wounded.sort_by_key(|a| (Reverse(a.stats.hp - a.hp), a.id));
        let danger = self.danger(unit.faction);
        let key = |&(dest, spell): &(Pos, &SpellId)| {
            (
                danger.contains(dest),
                Reverse(terrain_tenths(state, dest)),
                dest.y,
                dest.x,
                spell.clone(),
            )
        };
        wounded.iter().find_map(|ally| {
            let (dest, spell) = dests
                .iter()
                .flat_map(|&dest| {
                    let distance = Pos::manhattan(dest, ally.pos);
                    spells
                        .iter()
                        .filter(move |def| def.in_range(distance))
                        .map(move |def| (dest, &def.id))
                })
                .min_by_key(key)?;
            Some(Command::Act {
                unit: unit.id,
                dest,
                action: UnitAction::Cast {
                    spell: spell.clone(),
                    target: CastTarget::Unit(ally.id),
                    active: None,
                },
            })
        })
    }

    /// Where a healer with nobody to heal and nothing to attack goes.
    fn retreat(&self, unit: &Unit, reach: &Reach, dests: &[Pos]) -> Pos {
        let danger = self.danger(unit.faction);
        let allies: Vec<Pos> = self
            .state
            .units()
            .iter()
            .filter(|a| a.id != unit.id && unit.faction.is_allied_to(a.faction))
            .map(|a| a.pos)
            .collect();
        dests
            .iter()
            .copied()
            .min_by_key(|&d| {
                let near = allies.iter().map(|&a| Pos::manhattan(a, d)).min();
                (
                    danger.contains(d),
                    near.unwrap_or(0),
                    reach.cost(d).unwrap_or(u32::MAX),
                    d.y,
                    d.x,
                )
            })
            .unwrap_or(unit.pos)
    }

    /// The flow distances to `faction`'s targets for `movement_type`.
    fn flow(&self, faction: Faction, movement_type: MovementTypeId) -> Rc<Flow> {
        let mut flows = self.flows.borrow_mut();
        Rc::clone(
            flows
                .entry((faction, movement_type))
                .or_insert_with(|| Rc::new(flow_field(self.state, faction, movement_type))),
        )
    }

    /// Every tile a unit hostile to `faction` could attack this turn.
    fn danger(&self, faction: Faction) -> Rc<TileSet> {
        let state = self.state;
        let mut dangers = self.dangers.borrow_mut();
        Rc::clone(dangers.entry(faction).or_insert_with(|| {
            let (classes, items, spells) = (state.classes(), state.items(), state.spells());
            let zone = danger_zone(
                state.map(),
                state.terrain(),
                classes,
                state.units(),
                faction,
                |u| u.attack_ranges(classes, items, spells),
            );
            let tiles = &state.map().tiles;
            Rc::new(zone.unwrap_or_else(|_| TileSet::new(tiles.width(), tiles.height())))
        }))
    }
}

/// What `unit` can attack with: every weapon it can wield, then every
/// attack spell with uses left, ordered by `(id, slot)`, each with the
/// techniques to try with it.
fn arms(state: &BattleState, unit: &Unit, weights: &AiWeights) -> Vec<Arm> {
    let mut arms: Vec<(String, usize, Arm)> = Vec::new();
    if let Some(class) = state.classes().get(&unit.class) {
        for slot in 0..WEAPON_SLOTS {
            if let Some((copy, def)) = unit.usable_weapon(slot, class, state.items()) {
                let with = Equipped::Weapon(slot);
                let arm = Arm {
                    extras: extras(state, unit, &with, weights),
                    with,
                    min: def.min_range,
                    max: def.max_range,
                };
                arms.push((copy.def.0.clone(), slot, arm));
            }
        }
    }
    for spell in &unit.learned {
        if let Some(def) = unit.castable_attack(spell, state.spells()) {
            let with = Equipped::Spell(spell.clone());
            let arm = Arm {
                extras: extras(state, unit, &with, weights),
                with,
                min: def.min_range,
                max: def.max_range,
            };
            arms.push((spell.0.clone(), WEAPON_SLOTS, arm));
        }
    }
    arms.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    arms.into_iter().map(|(_, _, arm)| arm).collect()
}

/// The techniques `unit` tries when attacking with `with`: its combat
/// actives, and with a weapon the Combat Arts it can pay for, ordered by id
/// (an active first). None for a unit that may not use them. The battle
/// refuses those that don't fit the attack ([`BattleState::attack_forecast`]).
fn extras(state: &BattleState, unit: &Unit, with: &Equipped, weights: &AiWeights) -> Vec<Extra> {
    if !unit.may_use_arts() {
        return Vec::new();
    }
    let mut out: Vec<Extra> = unit
        .usable_skills(state.classes(), state.skills())
        .into_iter()
        .filter_map(|def| match &def.kind {
            SkillKind::Active {
                cost,
                effect: ActiveEffect::Strike { range, .. },
            } => Some(Extra {
                technique: Technique::Active(def.id.clone()),
                range: *range,
                penalty: match cost {
                    SkillCost::Durability(points) => {
                        f64::from(weights.durability) * f64::from(*points)
                    }
                    SkillCost::ExtraSpellUse => f64::from(weights.spell_use),
                },
            }),
            _ => None,
        })
        .collect();
    if let Equipped::Weapon(slot) = *with {
        let arts = unit.usable_arts(slot, state.classes(), state.items(), state.arts());
        out.extend(arts.into_iter().map(|art| Extra {
            technique: Technique::Art(art.id.clone()),
            range: 0,
            penalty: f64::from(weights.durability) * f64::from(art.cost),
        }));
    }
    out.sort_by(|a, b| (a.technique.id(), &a.technique).cmp(&(b.technique.id(), &b.technique)));
    out
}

/// The terrain bonus of standing on `pos`: its Def + Avoid / 10.
fn terrain_bonus(state: &BattleState, pos: Pos) -> f64 {
    f64::from(terrain_tenths(state, pos)) / 10.0
}

/// The terrain bonus of standing on `pos`, in tenths: Def × 10 + Avoid.
fn terrain_tenths(state: &BattleState, pos: Pos) -> i32 {
    state
        .map()
        .tiles
        .get(pos)
        .and_then(|&t| state.terrain().get(t))
        .map_or(0, |r| i32::from(r.defense) * 10 + i32::from(r.avoid))
}

/// The tile of `dests` an attack-less `Aggressive` unit moves to, if any
/// has a flow distance.
fn approach(reach: &Reach, dests: &[Pos], flow: &Flow) -> Option<Pos> {
    dests
        .iter()
        .filter_map(|&d| {
            let distance = flow.get(d).copied().flatten()?;
            Some((distance, reach.cost(d).unwrap_or(u32::MAX), d.y, d.x))
        })
        .min()
        .map(|(_, _, y, x)| Pos::new(x, y))
}

/// The flow distance of every tile to `faction`'s targets for
/// `movement_type` (see the module docs).
fn flow_field(state: &BattleState, faction: Faction, movement_type: MovementTypeId) -> Flow {
    let tiles = &state.map().tiles;
    let (width, height) = (tiles.width(), tiles.height());
    let enter = |pos| {
        tiles
            .get(pos)
            .and_then(|&t| state.terrain().move_cost(t, movement_type))
            .map(u32::from)
    };
    let mut dist = Grid::filled(width, height, None);
    let mut targets = TileSet::new(width, height);
    let mut queue = BinaryHeap::new();
    for t in state
        .units()
        .iter()
        .filter(|t| faction.is_hostile_to(t.faction))
    {
        targets.insert(t.pos);
        queue.push(Reverse((0, t.pos.y, t.pos.x)));
    }
    // Each tile's distance is settled the first time it comes off the
    // queue: the cheapest, as costs never go down.
    while let Some(Reverse((cost, y, x))) = queue.pop() {
        let pos = Pos::new(x, y);
        let Some(slot) = dist.get_mut(pos).filter(|d| d.is_none()) else {
            continue; // Off the map, or already settled.
        };
        *slot = Some(cost);
        // Walking from a neighbour onto `pos` costs entering it; the last
        // step, onto a target's tile, is free (only tiles next to it count).
        let step = if targets.contains(pos) {
            0
        } else {
            enter(pos).unwrap_or(0)
        };
        for next in tiles.neighbors4(pos) {
            if enter(next).is_some() && dist.get(next) == Some(&None) {
                queue.push(Reverse((cost + step, next.y, next.x)));
            }
        }
    }
    dist
}

/// The expected result of a combat, played out from its forecast.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Odds {
    /// Expected HP the defender loses.
    dealt: f64,
    /// Expected HP the attacker loses.
    taken: f64,
    /// Chance the defender falls.
    kill: f64,
}

impl Odds {
    /// The odds of `forecast` between an attacker and a defender with these
    /// HP (see the module docs).
    fn of(forecast: &Forecast, attacker: CombatHp, defender: CombatHp) -> Odds {
        let walk = Walk {
            forecast,
            order: strike_order(forecast),
            start: [attacker.current, defender.current],
            max: [attacker.max, defender.max],
        };
        let mut odds = Odds::default();
        walk.strike(0, walk.start, [0, 0], 1.0, &mut odds);
        odds
    }
}

/// Every hit-or-miss branch of a combat's strikes.
struct Walk<'a> {
    forecast: &'a Forecast,
    order: Vec<Side>,
    /// HP going in: `[attacker, defender]`.
    start: [StatValue; 2],
    /// Max HP: `[attacker, defender]`.
    max: [StatValue; 2],
}

impl Walk<'_> {
    /// Plays strike `i` on, from `hp` with `struck` strikes made per side,
    /// on a branch of this `chance`, adding its outcomes to `odds`.
    fn strike(&self, i: usize, hp: [StatValue; 2], struck: [u8; 2], chance: f64, odds: &mut Odds) {
        let striker = self.order.get(i).and_then(|&by| match by {
            Side::Attacker => Some((0, self.forecast.attacker)),
            Side::Defender => self.forecast.defender.map(|d| (1, d)),
        });
        let over = hp[0] <= 0 || hp[1] <= 0;
        let Some((me, numbers)) = striker.filter(|_| !over) else {
            odds.dealt += chance * f64::from((self.start[1] - hp[1]).max(0));
            odds.taken += chance * f64::from((self.start[0] - hp[0]).max(0));
            if hp[1] <= 0 {
                odds.kill += chance;
            }
            return;
        };
        let mut struck = struck;
        struck[me] += 1;
        // Both branches, even one with no chance: it adds nothing.
        let hit = f64::from(numbers.hit) / 100.0;
        let mut after = hp;
        let target = 1 - me;
        after[target] = struck_hp(&numbers, struck[me], hp[target], self.max[target]);
        self.strike(i + 1, after, struck, chance * hit, odds);
        self.strike(i + 1, hp, struck, chance * (1.0 - hit), odds);
    }
}

/// A target's HP after the `nth` strike of a side with `numbers` hits it
/// without a crit (an Absorb strike heals it, up to `max`).
fn struck_hp(numbers: &SideForecast, nth: u8, hp: StatValue, max: StatValue) -> StatValue {
    let damage = if nth > 1 {
        numbers.followup_damage
    } else {
        numbers.damage
    };
    if numbers.affinity == Some(Affinity::Absorb) {
        (hp + damage).min(max)
    } else {
        (hp - damage).max(0)
    }
}

#[cfg(test)]
mod tests;
