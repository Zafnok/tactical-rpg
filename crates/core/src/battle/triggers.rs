//! Battle dialogue triggers (ticket 0705,
//! `docs/design/battle-scenes-and-recruitment.md`): the story moments of a
//! map, as data. A battle's [`Trigger`]s name a scene (a dialogue scene id,
//! checked by `trpg-content`) and the moment it plays. After every command
//! (and at the battle's start) the battle inserts an
//! [`Event::SceneTriggered`] into the events, right where its moment is, so
//! the UI plays it in order. Some triggers also recruit a character, who
//! leaves the battlefield and joins the army after a won battle
//! ([`BattleState::recruited`]). The rules are in the parent module's docs.
//!
//! In a battle file (RON, 0801) a trigger reads:
//!
//! ```ron
//! (when: TurnStart(turn: 3, phase: Player), scene: "ch01_hint", once: true)
//! (when: UnitEntersArea(who: Faction(Player), area: (x: 10, y: 5, w: 2, h: 2)), scene: "ch01_fort", once: true)
//! (when: CombatStart(unit: "harl", against: Some("ana")), scene: "ch01_harl_ana", once: true)
//! (when: CombatStart(unit: "harl"), scene: "ch01_harl", once: true)
//! (when: HalfHp(unit: "harl"), scene: "ch01_harl_half", once: true)
//! (when: UnitFell(unit: "harl"), scene: "ch01_harl_death", once: true)
//! (when: UnitFell(unit: "tamsin", mode: Some(Casual)), scene: "ch01_tamsin_retreat", once: true)
//! (when: UnitFell(unit: "brom", recruit: true), scene: "ch02_brom_yields", once: true)
//! (when: Talk(a: "ana", b: "rook", recruit: true), scene: "ch02_rook_joins", once: true)
//! ```

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{BattleState, CommandError, Event, Phase, Step, Turn};
use crate::geom::Pos;
use crate::stats::StatValue;
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
    },
    /// When a combat leaves `unit` standing at half its max HP or less,
    /// just after that combat (a boss's "halfway" line).
    HalfHp {
        /// The character.
        unit: CharacterId,
    },
    /// When `unit` falls, before it leaves the map (a death quote). With
    /// `mode`, only in that [`GameMode`] (a Classic death quote and a
    /// Casual retreat line). With `recruit`, the fallen unit joins the
    /// army after a won battle ("joins you if defeated").
    UnitFell {
        /// The character.
        unit: CharacterId,
        /// Only in this mode.
        #[serde(default)]
        mode: Option<GameMode>,
        /// Whether it joins after the battle.
        #[serde(default)]
        recruit: bool,
    },
    /// When `a` and `b` talk: either chooses
    /// [`UnitAction::Talk`](super::UnitAction::Talk) next to the other (the
    /// scene decides who speaks first). With `recruit`, `b` leaves the
    /// battlefield and joins the army after a won battle.
    Talk {
        /// One of the two.
        a: CharacterId,
        /// The other; the one recruited.
        b: CharacterId,
        /// Whether `b` joins after the battle.
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

/// The indices of the `once` triggers that have fired.
pub(super) type FiredSet = BTreeSet<usize>;

impl BattleState {
    /// The battle's triggers.
    pub fn triggers(&self) -> &[Trigger] {
        &self.triggers
    }

    /// The campaign's mode for this battle.
    pub fn mode(&self) -> GameMode {
        self.mode
    }

    /// Whether trigger `index` is a `once` trigger that has fired.
    pub fn has_fired(&self, index: usize) -> bool {
        self.fired.contains(&index)
    }

    /// The units recruited so far, in order, as they were when they left
    /// the map: they join the army if the battle is won (0801).
    pub fn recruited(&self) -> &[Unit] {
        &self.recruited
    }

    /// The units unit `id` could talk to from `dest`: on the map, next to
    /// `dest`, with a [`TriggerWhen::Talk`] for the pair (either way round)
    /// that hasn't fired (the action menu offers `Talk` only then). In unit
    /// order. Doesn't check that the unit can reach `dest` or still act.
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
        Ok(Step::Talk { trigger: index })
    }

    /// Carries out a validated talk with trigger `index`: the scene, then,
    /// if the trigger recruits, its `b` leaves the map for the recruits.
    pub(super) fn talk(&mut self, index: usize, events: &mut Vec<Event>) {
        let Some(trigger) = self.triggers.get(index).cloned() else {
            return;
        };
        self.fire(index, &trigger, events);
        let TriggerWhen::Talk {
            b, recruit: true, ..
        } = &trigger.when
        else {
            return;
        };
        let at = self
            .units
            .iter()
            .position(|u| u.character.as_ref() == Some(b));
        if let Some(i) = at {
            let unit = self.units.remove(i);
            events.push(Event::UnitRecruited { unit: unit.id });
            self.recruited.push(unit);
        }
    }

    /// The first unfired [`TriggerWhen::Talk`] between `unit` and `target`,
    /// either way round.
    fn talk_trigger(&self, unit: &Unit, target: &Unit) -> Option<usize> {
        let (Some(me), Some(them)) = (&unit.character, &target.character) else {
            return None;
        };
        (0..self.triggers.len()).find(|&i| {
            let t = &self.triggers[i];
            let pair = match &t.when {
                TriggerWhen::Talk { a, b, .. } => (a == me && b == them) || (a == them && b == me),
                _ => false,
            };
            pair && !self.spent(i, t)
        })
    }

    /// Whether trigger `index` may not fire again.
    fn spent(&self, index: usize, trigger: &Trigger) -> bool {
        trigger.once && self.fired.contains(&index)
    }

    /// Records trigger `index` as fired (if once) and emits its scene.
    fn fire(&mut self, index: usize, trigger: &Trigger, events: &mut Vec<Event>) {
        if trigger.once {
            self.fired.insert(index);
        }
        events.push(Event::SceneTriggered {
            scene: trigger.scene.clone(),
        });
    }

    /// Inserts the scenes the triggers fire into `events` (one command's,
    /// or the battle's start), each at its moment: before a combat
    /// ([`Event::CombatResolved`]) or a fall ([`Event::UnitFell`]), after
    /// a combat (half HP), a phase start ([`Event::PhaseStarted`]) or a
    /// move ([`Event::UnitMoved`]). A fall that recruits is followed by
    /// [`Event::UnitRecruited`]. A [`TriggerWhen::Talk`]'s scene is already
    /// in them.
    pub(super) fn fire_triggers(&mut self, events: Vec<Event>) -> Vec<Event> {
        if self.triggers.is_empty() {
            return events;
        }
        let mut out = Vec::with_capacity(events.len());
        for event in events {
            match &event {
                Event::CombatResolved {
                    attacker,
                    defender,
                    outcome,
                    ..
                } => {
                    let (a, d) = (*attacker, *defender);
                    let left = [(a, outcome.attacker_hp), (d, outcome.defender_hp)];
                    self.combat_scenes(a, d, &mut out);
                    self.combat_scenes(d, a, &mut out);
                    out.push(event);
                    for (id, hp) in left {
                        self.half_hp_scenes(id, hp, &mut out);
                    }
                }
                Event::UnitFell { unit } => {
                    let unit = *unit;
                    let recruit = self.fall_scenes(unit, &mut out);
                    out.push(event);
                    if recruit && let Some(u) = self.any_unit(unit).cloned() {
                        self.recruited.push(u);
                        out.push(Event::UnitRecruited { unit });
                    }
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

    /// Fires, in list order, every trigger not spent for which `test`
    /// holds. Returns whether one of them recruits.
    fn fire_matching(
        &mut self,
        events: &mut Vec<Event>,
        test: impl Fn(&TriggerWhen) -> bool,
    ) -> bool {
        let mut recruits = false;
        for i in 0..self.triggers.len() {
            let trigger = self.triggers[i].clone();
            if test(&trigger.when) && !self.spent(i, &trigger) {
                self.fire(i, &trigger, events);
                recruits |= matches!(trigger.when, TriggerWhen::UnitFell { recruit: true, .. });
            }
        }
        recruits
    }

    /// The character of unit `id`, on the map or not.
    fn character_of(&self, id: UnitId) -> Option<CharacterId> {
        self.any_unit(id).and_then(|u| u.character.clone())
    }

    /// The scenes of unit `id` falling. Returns whether one recruits it.
    fn fall_scenes(&mut self, id: UnitId, events: &mut Vec<Event>) -> bool {
        let Some(character) = self.character_of(id) else {
            return false;
        };
        let mode = self.mode;
        self.fire_matching(events, |when| {
            matches!(when, TriggerWhen::UnitFell { unit, mode: m, .. }
                if *unit == character && m.is_none_or(|m| m == mode))
        })
    }

    /// The scenes of unit `id` left with `hp` HP by a combat: its
    /// [`TriggerWhen::HalfHp`]s, if it stands at half its max HP or less.
    fn half_hp_scenes(&mut self, id: UnitId, hp: StatValue, events: &mut Vec<Event>) {
        let Some(unit) = self.any_unit(id) else {
            return;
        };
        let (max, character) = (unit.stats.hp, unit.character.clone());
        let Some(character) = character.filter(|_| hp > 0 && hp * 2 <= max) else {
            return;
        };
        self.fire_matching(
            events,
            |when| matches!(when, TriggerWhen::HalfHp { unit } if *unit == character),
        );
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
        let Some(me) = self.character_of(id) else {
            return;
        };
        let foe = self.character_of(other);
        let for_me = |when: &TriggerWhen| match when {
            TriggerWhen::CombatStart { unit, against } if *unit == me => Some(against.clone()),
            _ => None,
        };
        let paired = self
            .triggers
            .iter()
            .any(|t| for_me(&t.when).is_some_and(|a| a.is_some() && a == foe));
        self.fire_matching(events, |when| match for_me(when) {
            Some(Some(against)) => Some(against) == foe,
            Some(None) => !paired,
            None => false,
        });
    }
}
