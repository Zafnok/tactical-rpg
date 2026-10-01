//! Every legal command (ticket 0504): the [`Command`]s
//! [`BattleState::apply`] accepts in a state, for the playtest bots
//! (ADR-0033) and the random-play tests. Pure and deterministic
//! (ADR-0004): it reads a [`BattleState`], and the same state always gives
//! the same list, in the same order.
//!
//! # What is listed
//!
//! [`legal_commands`] builds candidates from the state and keeps those the
//! battle's own validation accepts ([`BattleState::check`]), so the rules
//! live in one place and any loaded content is handled: a class, item,
//! spell, skill or art missing from its table just means that option is
//! refused, and so not listed.
//!
//! - **Battle over:** nothing.
//! - **A unit waiting to move after its attack**
//!   ([`BattleState::pending_move`]): only its [`Command::MoveAfter`]s,
//!   staying first, then each tile of [`BattleState::move_after_tiles`] in
//!   that order.
//! - Otherwise [`Command::EndPhase`] first, then, for each **ready** unit of
//!   the current phase (not yet acted) in [`BattleState::units`] order:
//!   1. [`Command::Equip`]: each weapon slot in order, then each learned
//!      attack spell (id order).
//!   2. [`Command::Talk`]: for each tile it can stop on (row-major), each
//!      unit it can talk to there ([`BattleState::talk_targets`] order).
//!   3. [`Command::Act`]: for each tile it can stop on (row-major), these
//!      actions, in this order:
//!      - `Wait`;
//!      - `Attack`: for each unit (unit order), each weapon slot, plain,
//!        then with each combat active the unit can use (id order), then
//!        with each Combat Art for that weapon ([`Unit::arts_for`] order);
//!      - `Cast` of each learned spell (id order): at each unit (unit
//!        order), plain then with each combat active, then at each tile
//!        (row-major);
//!      - `UseItem`: each battle-pack index, on each unit;
//!      - `UseSkill`: each non-combat active the unit can use (id order),
//!        with no target, then on each unit;
//!      - `Seize`, then `Open`;
//!      - `Shop`: the visits below.
//!
//! # Exclusions
//!
//! Only **shop visits** are cut short. A `Shop` action is any sequence of
//! transactions, too many to list; only these visits are listed, each if
//! accepted: every single transaction (buying each item the shop lists, in
//! its order; selling the weapon in each slot, the armour, the accessory,
//! then each battle-pack item; repairing the weapon in each slot), then
//! buying two of the shop's first item. Every other visit of two or more
//! transactions is legal but not listed.
//!
//! Nothing else is left out.
//!
//! # Cost
//!
//! Attacks and casts at a unit are only tried on units within the acting
//! unit's attack reach of the tile (its farthest-reaching weapon or spell
//! plus its largest combat-active range bonus): every one farther away is
//! out of range, so this only saves work. Each unit's moves are worked out
//! once.

use crate::battle::{BattleState, CastTarget, Command, Phase, SellFrom, ShopTxn, UnitAction};
use crate::geom::Pos;
use crate::item::{Equipped, WEAPON_SLOTS};
use crate::movement::reachable;
use crate::shop::Shop;
use crate::skill::{ActiveEffect, SkillId, SkillKind};
use crate::unit::Unit;

/// Every command [`BattleState::apply`] accepts in `state`, but for the
/// shop visits the module docs leave out, in the order they give.
pub fn legal_commands(state: &BattleState) -> Vec<Command> {
    if state.outcome().is_some() {
        return Vec::new();
    }
    if let Some(pending) = state.pending_move() {
        return std::iter::once(None)
            .chain(state.move_after_tiles().into_iter().map(Some))
            .map(|to| Command::MoveAfter {
                unit: pending.unit,
                to,
            })
            .collect();
    }
    let mut out = vec![Command::EndPhase];
    let ready = state
        .units()
        .iter()
        .filter(|u| Phase::of(u.faction) == state.phase())
        .filter(|u| !u.acted);
    for unit in ready {
        unit_commands(state, unit, &mut out);
    }
    out
}

/// Ready `unit`'s legal equips, talks and acts, added to `out`.
fn unit_commands(state: &BattleState, unit: &Unit, out: &mut Vec<Command>) {
    let equips = (0..WEAPON_SLOTS)
        .map(Equipped::Weapon)
        .chain(unit.learned.iter().cloned().map(Equipped::Spell))
        .map(|equipped| Command::Equip {
            unit: unit.id,
            equipped,
        });
    out.extend(equips.filter(|c| state.check(c).is_ok()));
    let Ok(reach) = reachable(
        state.map(),
        state.terrain(),
        state.classes(),
        state.units(),
        unit.id,
    ) else {
        return;
    };
    let dests: Vec<Pos> = reach.stoppable().iter().collect();
    for &dest in &dests {
        let talks = state
            .talk_targets(unit.id, dest)
            .into_iter()
            .map(|target| Command::Talk {
                unit: unit.id,
                dest,
                target,
            });
        out.extend(talks.filter(|c| state.check(c).is_ok()));
    }
    let options = Options::of(state, unit);
    for dest in dests {
        let Some(path) = reach.path_to(dest) else {
            continue;
        };
        let acts = options
            .actions(state, unit, dest)
            .into_iter()
            .filter(|a| state.check_action(unit, dest, &path, a).is_ok())
            .map(|action| Command::Act {
                unit: unit.id,
                dest,
                action,
            });
        out.extend(acts);
    }
}

/// What a unit could choose from on any tile: its skills, arts and reach.
struct Options {
    /// Its combat actives, id order.
    strikes: Vec<SkillId>,
    /// Its non-combat actives, id order.
    others: Vec<SkillId>,
    /// Its arts' ids for the weapon in each slot.
    arts: Vec<Vec<crate::art::ArtId>>,
    /// Its [`attack_reach`].
    reach: u32,
}

impl Options {
    fn of(state: &BattleState, unit: &Unit) -> Options {
        let usable = unit.usable_skills(state.classes(), state.skills());
        let actives = |combat: bool| {
            usable
                .iter()
                .filter(|d| d.is_active())
                .filter(|d| d.is_combat() == combat)
                .map(|d| d.id.clone())
                .collect()
        };
        let arts = (0..WEAPON_SLOTS)
            .map(|slot| {
                unit.arts_for(slot, state.classes(), state.items(), state.arts())
                    .iter()
                    .map(|a| a.id.clone())
                    .collect()
            })
            .collect();
        Options {
            strikes: actives(true),
            others: actives(false),
            arts,
            reach: attack_reach(state, unit),
        }
    }

    /// The candidate actions at `dest`, in the module docs' order.
    fn actions(&self, state: &BattleState, unit: &Unit, dest: Pos) -> Vec<UnitAction> {
        let mut out = vec![UnitAction::Wait];
        let near: Vec<&Unit> = state
            .units()
            .iter()
            .filter(|t| Pos::manhattan(dest, t.pos) <= self.reach)
            .collect();
        for t in &near {
            for slot in 0..WEAPON_SLOTS {
                let attack = |active, art| UnitAction::Attack {
                    target: t.id,
                    slot,
                    active,
                    art,
                };
                out.push(attack(None, None));
                out.extend(self.strikes.iter().map(|s| attack(Some(s.clone()), None)));
                out.extend(
                    self.arts[slot]
                        .iter()
                        .map(|a| attack(None, Some(a.clone()))),
                );
            }
        }
        for spell in &unit.learned {
            for t in &near {
                let cast = |active| UnitAction::Cast {
                    spell: spell.clone(),
                    target: CastTarget::Unit(t.id),
                    active,
                };
                out.push(cast(None));
                out.extend(self.strikes.iter().map(|s| cast(Some(s.clone()))));
            }
            out.extend(state.map().tiles.positions().map(|pos| UnitAction::Cast {
                spell: spell.clone(),
                target: CastTarget::Tile(pos),
                active: None,
            }));
        }
        for pack_index in 0..state.pack().items.len() {
            out.extend(state.units().iter().map(|t| UnitAction::UseItem {
                pack_index,
                target: t.id,
            }));
        }
        for skill in &self.others {
            let targets = std::iter::once(None).chain(state.units().iter().map(|t| Some(t.id)));
            out.extend(targets.map(|target| UnitAction::UseSkill {
                skill: skill.clone(),
                target,
            }));
        }
        out.extend([UnitAction::Seize, UnitAction::Open]);
        if let Some(shop) = state.map().shop(dest) {
            out.extend(
                shop_visits(state, shop)
                    .into_iter()
                    .map(|txns| UnitAction::Shop { txns }),
            );
        }
        out
    }
}

/// How far (Manhattan) from its tile `unit` can attack, cast or heal a
/// unit: its farthest-reaching weapon or learned spell, plus its largest
/// combat-active range bonus. Arts never add range (they can only lower
/// the minimum, like Close Shot). 0 if it has neither.
fn attack_reach(state: &BattleState, unit: &Unit) -> u32 {
    let weapons = unit
        .loadout
        .weapons
        .iter()
        .flatten()
        .filter_map(|w| state.items().weapon(&w.def))
        .map(|d| d.max_range);
    let spells = unit
        .learned
        .iter()
        .filter_map(|s| state.spells().get(s))
        .map(|d| d.max_range);
    let bonus = unit
        .usable_skills(state.classes(), state.skills())
        .iter()
        .filter_map(|d| match &d.kind {
            SkillKind::Active {
                effect: ActiveEffect::Strike { range, .. },
                ..
            } => Some(*range),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    weapons
        .chain(spells)
        .max()
        .unwrap_or(0)
        .saturating_add(bonus)
}

/// The shop visits listed at `shop` (see the module docs), before
/// validation.
fn shop_visits(state: &BattleState, shop: &Shop) -> Vec<Vec<ShopTxn>> {
    let buys = shop
        .stock
        .iter()
        .map(|item| ShopTxn::Buy { item: item.clone() });
    let sells = (0..WEAPON_SLOTS)
        .map(SellFrom::Weapon)
        .chain([SellFrom::Armour, SellFrom::Accessory])
        .chain((0..state.pack().items.len()).map(SellFrom::Pack))
        .map(|from| ShopTxn::Sell { from });
    let repairs = (0..WEAPON_SLOTS).map(|slot| ShopTxn::Repair { slot });
    let mut out: Vec<Vec<ShopTxn>> = buys.chain(sells).chain(repairs).map(|t| vec![t]).collect();
    out.extend(
        shop.stock
            .first()
            .map(|item| vec![ShopTxn::Buy { item: item.clone() }; 2]),
    );
    out
}
