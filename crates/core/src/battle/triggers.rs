//! Battle dialogue triggers (ticket 0705): the story moments of a map, as
//! data. A battle's [`Trigger`]s name a scene (a dialogue scene id, checked
//! by `trpg-content`) and the moment it plays. After every command (and at
//! the battle's start) the battle inserts an [`Event::SceneTriggered`] into
//! the events, right where its moment is, so the UI plays it in order. The
//! rules are in the parent module's docs.
//!
//! In a battle file (RON, 0801) a trigger reads:
//!
//! ```ron
//! (when: TurnStart(turn: 3, phase: Player), scene: "ch01_hint", once: true)
//! (when: UnitEntersArea(who: Faction(Player), area: (x: 10, y: 5, w: 2, h: 2)), scene: "ch01_fort", once: true)
//! (when: CombatStart(unit: "harl", against: Some("ana")), scene: "ch01_harl_ana", once: true)
//! (when: CombatStart(unit: "harl"), scene: "ch01_harl", once: true)
//! (when: UnitFell(unit: "harl"), scene: "ch01_harl_death", once: true)
//! (when: UnitFell(unit: "tamsin", mode: Some(Casual)), scene: "ch01_tamsin_retreat", once: true)
//! (when: Talk(a: "ana", b: "rook", recruit: true), scene: "ch02_rook_joins", once: true)
//! ```

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{BattleState, CommandError, Event, Phase, Step, Turn};
use crate::geom::Pos;
use crate::unit::{CharacterId, Faction, Unit, UnitId};

/// How fallen player units are treated (`docs/design/death-and-difficulty.md`),
/// picked at New Game and kept by the campaign (0801). In a battle it only
/// picks which [`TriggerWhen::UnitFell`] scene plays: a death quote or a
/// retreat line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameMode {
    /// A fallen player unit dies.
    #[default]
    Classic,
    /// A fallen player unit retreats and is back for the next battle.
    Casual,
}

/// A rectangle of tiles: `w × h` tiles from `(x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileRect {
    /// Left column.
    pub x: i32,
    /// Top row.
    pub y: i32,
    /// Width in tiles.
    pub w: i32,
    /// Height in tiles.
    pub h: i32,
}

impl TileRect {
    /// Whether `pos` is inside.
    pub fn contains(&self, pos: Pos) -> bool {
        (self.x..self.x.saturating_add(self.w)).contains(&pos.x)
            && (self.y..self.y.saturating_add(self.h)).contains(&pos.y)
    }
}

/// Which units a [`TriggerWhen::UnitEntersArea`] watches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Who {
    /// The unit of this named character.
    Character(CharacterId),
    /// Any unit of this faction.
    Faction(Faction),
}

impl Who {
    fn matches(&self, unit: &Unit) -> bool {
        match self {
            Who::Character(c) => unit.character.as_ref() == Some(c),
            Who::Faction(f) => unit.faction == *f,
        }
    }
}

/// The moment a [`Trigger`]'s scene plays.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriggerWhen {
    /// When `phase` of `turn` starts, after its banner.
    TurnStart {
        /// The turn.
        turn: Turn,
        /// The phase.
        phase: Phase,
    },
    /// When a unit matching `who` ends a move inside `area` (a
    /// [`Event::UnitMoved`], so a unit that doesn't move never enters).
    UnitEntersArea {
        /// Who counts.
        who: Who,
        /// The tiles.
        area: TileRect,
    },
    /// When `unit` is in a combat, attacking or attacked, just before it
    /// (a boss engaged). With `against`, only against that character; a
    /// trigger with `against` for the pair replaces the ones without.
    CombatStart {
        /// The character.
        unit: CharacterId,
        /// Only against this character.
        #[serde(default)]
        against: Option<CharacterId>,
        /// With `once`: once per opponent instead of once in all.
        #[serde(default)]
        per_opponent: bool,
    },
    /// When `unit` falls, before it leaves the map (a death quote). With
    /// `mode`, only in that [`GameMode`] (a Classic death quote and a
    /// Casual retreat line).
    UnitFell {
        /// The character.
        unit: CharacterId,
        /// Only in this mode.
        #[serde(default)]
        mode: Option<GameMode>,
    },
    /// When `a` chooses [`UnitAction::Talk`](super::UnitAction::Talk) next
    /// to `b`. With `recruit`, `b` joins the player.
    Talk {
        /// Who talks (chooses the action).
        a: CharacterId,
        /// Who is talked to.
        b: CharacterId,
        /// Whether `b` joins the player's side.
        #[serde(default)]
        recruit: bool,
    },
}

/// A scene that plays at a moment of the battle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Trigger {
    /// When.
    pub when: TriggerWhen,
    /// The dialogue scene's id.
    pub scene: String,
    /// Whether it plays only the first time (else every time).
    pub once: bool,
}

/// A trigger that has fired: its index, and the opponent for a
/// [`TriggerWhen::CombatStart`] that fires once per opponent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub(super) struct Fired {
    trigger: usize,
    with: Option<UnitId>,
}

/// The fired-once record of a battle's triggers.
pub(super) type FiredSet = BTreeSet<Fired>;

impl BattleState {
    /// The battle's triggers.
    pub fn triggers(&self) -> &[Trigger] {
        &self.triggers
    }

    /// The campaign's mode for this battle.
    pub fn mode(&self) -> GameMode {
        self.mode
    }

    /// Whether trigger `index` has fired (with any opponent) and was
    /// marked once.
    pub fn has_fired(&self, index: usize) -> bool {
        self.fired.iter().any(|f| f.trigger == index)
    }

    /// The units unit `id` could talk to from `dest`: on the map, next to
    /// `dest`, with a [`TriggerWhen::Talk`] for the pair that hasn't fired
    /// (the action menu offers `Talk` only then). In unit order. Doesn't
    /// check that the unit can reach `dest` or still act.
    pub fn talk_targets(&self, id: UnitId, dest: Pos) -> Vec<UnitId> {
        let Some(unit) = self.unit(id) else {
            return Vec::new();
        };
        self.units
            .iter()
            .filter(|t| t.id != id && Pos::manhattan(dest, t.pos) == 1)
            .filter(|t| self.talk_trigger(unit, t).is_some())
            .map(|t| t.id)
            .collect()
    }

    /// Validates `unit` talking to `target` from `dest`.
    pub(super) fn plan_talk(
        &self,
        unit: &Unit,
        dest: Pos,
        target: UnitId,
    ) -> Result<Step, CommandError> {
        let other = self.living(target)?;
        let index = self
            .talk_trigger(unit, other)
            .filter(|_| target != unit.id)
            .ok_or(CommandError::CannotTalk(target))?;
        let distance = Pos::manhattan(dest, other.pos);
        if distance != 1 {
            return Err(CommandError::OutOfRange { target, distance });
        }
        Ok(Step::Talk {
            trigger: index,
            target,
        })
    }

    /// Carries out a validated talk to `target` with trigger `index`: the
    /// scene, then, if the trigger recruits, `target` joins the player's
    /// side, done until the next player phase.
    pub(super) fn talk(&mut self, index: usize, target: UnitId, events: &mut Vec<Event>) {
        let Some(trigger) = self.triggers.get(index).cloned() else {
            return;
        };
        self.fire(index, None, &trigger, events);
        if !matches!(trigger.when, TriggerWhen::Talk { recruit: true, .. }) {
            return;
        }
        if let Some(t) = self.unit_mut(target) {
            t.faction = Faction::Player;
            t.acted = true;
            events.push(Event::UnitRecruited { unit: target });
        }
    }

    /// The first unfired [`TriggerWhen::Talk`] for `unit` talking to
    /// `target`.
    fn talk_trigger(&self, unit: &Unit, target: &Unit) -> Option<usize> {
        let (Some(a), Some(b)) = (&unit.character, &target.character) else {
            return None;
        };
        (0..self.triggers.len()).find(|&i| {
            let t = &self.triggers[i];
            matches!(&t.when, TriggerWhen::Talk { a: ta, b: tb, .. } if ta == a && tb == b)
                && !self.spent(i, t, None)
        })
    }

    /// Whether trigger `index` may not fire again (with opponent `with`).
    fn spent(&self, index: usize, trigger: &Trigger, with: Option<UnitId>) -> bool {
        trigger.once
            && self.fired.contains(&Fired {
                trigger: index,
                with,
            })
    }

    /// Records trigger `index` as fired (if once) and emits its scene.
    fn fire(
        &mut self,
        index: usize,
        with: Option<UnitId>,
        trigger: &Trigger,
        events: &mut Vec<Event>,
    ) {
        if trigger.once {
            self.fired.insert(Fired {
                trigger: index,
                with,
            });
        }
        events.push(Event::SceneTriggered {
            scene: trigger.scene.clone(),
        });
    }

    /// Inserts the scenes the triggers fire into `events` (one command's,
    /// or the battle's start), each at its moment: before a combat
    /// ([`Event::CombatResolved`]) or a fall ([`Event::UnitFell`]), after
    /// a phase start ([`Event::PhaseStarted`]) or a move
    /// ([`Event::UnitMoved`]). A [`TriggerWhen::Talk`]'s scene is already
    /// in them.
    pub(super) fn fire_triggers(&mut self, events: Vec<Event>) -> Vec<Event> {
        if self.triggers.is_empty() {
            return events;
        }
        let mut out = Vec::with_capacity(events.len());
        for event in events {
            match &event {
                Event::CombatResolved {
                    attacker, defender, ..
                } => {
                    self.combat_scenes(*attacker, *defender, &mut out);
                    self.combat_scenes(*defender, *attacker, &mut out);
                    out.push(event);
                }
                Event::UnitFell { unit } => {
                    let unit = *unit;
                    self.fall_scenes(unit, &mut out);
                    out.push(event);
                }
                Event::PhaseStarted { turn, phase } => {
                    let (turn, phase) = (*turn, *phase);
                    out.push(event);
                    self.fire_matching(&mut out, |when| {
                        *when == TriggerWhen::TurnStart { turn, phase }
                    });
                }
                Event::UnitMoved { unit, path } => {
                    let (unit, end) = (*unit, path.last().copied());
                    out.push(event);
                    self.area_scenes(unit, end, &mut out);
                }
                _ => out.push(event),
            }
        }
        out
    }

    /// Fires, in list order, every trigger not spent (with no opponent)
    /// for which `test` holds.
    fn fire_matching(&mut self, events: &mut Vec<Event>, test: impl Fn(&TriggerWhen) -> bool) {
        for i in 0..self.triggers.len() {
            let trigger = self.triggers[i].clone();
            if test(&trigger.when) && !self.spent(i, &trigger, None) {
                self.fire(i, None, &trigger, events);
            }
        }
    }

    /// The scenes of unit `id` falling.
    fn fall_scenes(&mut self, id: UnitId, events: &mut Vec<Event>) {
        let Some(character) = self.any_unit(id).and_then(|u| u.character.clone()) else {
            return;
        };
        let mode = self.mode;
        self.fire_matching(events, |when| {
            matches!(when, TriggerWhen::UnitFell { unit, mode: m }
                if *unit == character && m.is_none_or(|m| m == mode))
        });
    }

    /// The scenes of unit `id` ending a move on `end`.
    fn area_scenes(&mut self, id: UnitId, end: Option<Pos>, events: &mut Vec<Event>) {
        let (Some(end), Some(unit)) = (end, self.any_unit(id).cloned()) else {
            return;
        };
        self.fire_matching(events, |when| {
            matches!(when, TriggerWhen::UnitEntersArea { who, area }
                if area.contains(end) && who.matches(&unit))
        });
    }

    /// The scenes of unit `id` going into a combat with `other`: its
    /// [`TriggerWhen::CombatStart`]s against `other`'s character, or, if
    /// it has none for that pair, its ones against anyone.
    fn combat_scenes(&mut self, id: UnitId, other: UnitId, events: &mut Vec<Event>) {
        let Some(me) = self.any_unit(id).and_then(|u| u.character.clone()) else {
            return;
        };
        let foe = self.any_unit(other).and_then(|u| u.character.clone());
        let for_me = |t: &Trigger| match &t.when {
            TriggerWhen::CombatStart { unit, against, .. } if *unit == me => Some(against.clone()),
            _ => None,
        };
        let paired = self
            .triggers
            .iter()
            .any(|t| for_me(t).is_some_and(|a| a.is_some() && a == foe));
        for i in 0..self.triggers.len() {
            let trigger = self.triggers[i].clone();
            let Some(against) = for_me(&trigger) else {
                continue;
            };
            let fits = match &against {
                Some(_) => against == foe,
                None => !paired,
            };
            let per_opponent = matches!(
                trigger.when,
                TriggerWhen::CombatStart {
                    per_opponent: true,
                    ..
                }
            );
            let with = per_opponent.then_some(other);
            if fits && !self.spent(i, &trigger, with) {
                self.fire(i, with, &trigger, events);
            }
        }
    }
}
