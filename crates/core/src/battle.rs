//! A battle: its state, the [`Command`]s that change it and the [`Event`]s
//! they produce (ADR-0004 rule 2). The UI animates events, the AI issues the
//! same commands as the player, and saves and replays store the state and
//! the commands.
//!
//! # Rules
//!
//! Sources: `docs/design/turn-structure.md` (phases, actions, reinforcements,
//! turn limits) and `docs/design/death-and-difficulty.md` (falling units,
//! loss conditions).
//!
//! - **Phases.** Each turn runs `Player → Enemy → Other` ([`Phase`]; `Other`
//!   is the `Ally` and `Neutral` factions), then `turn += 1`. The battle
//!   starts at turn 1, Player phase. [`Command::EndPhase`] moves to the next
//!   phase that has living units or reinforcements arriving; a phase without
//!   either is skipped (no event). The turn still advances after the Other
//!   phase's slot, whether or not it ran.
//! - **Phase start**, in this order: every unit of the phase becomes ready
//!   (`acted = false`); the phase's reinforcements that are due arrive,
//!   already done ([`Event::UnitsArrived`]); then [`Event::PhaseStarted`]
//!   (the banner). Units of other phases keep their `acted` flag (drawn
//!   dimmed until their own phase).
//! - **Auto-end is not here.** Ending the player phase when every unit has
//!   acted is a player setting handled by the battle screen, which issues
//!   `EndPhase` itself; AI phases end when the AI issues `EndPhase`.
//! - **Acting.** [`Command::Act`] commits a move and one action together, so
//!   the UI can let the player cancel a move freely before choosing the
//!   action. The unit must be on the map, belong to the current phase, not
//!   have acted, and `dest` must be a tile it can stop on
//!   ([`reachable`]). After the action it is done. No
//!   Canto: a unit never moves after its action (but see *Extending* below).
//! - **Attacking** names a loadout slot. The weapon there must be one the
//!   unit can wield, and the target a hostile unit on the map within its
//!   range *from `dest`*. Attacking equips that weapon ([`Event::Equipped`]
//!   if it changed). The defender counters with its equipped weapon, if it
//!   can wield it and the attacker is in range. The combat uses [`forecast`]
//!   and [`resolve`] with the battle's [`SimRng`] and the item table's
//!   [`combat_rules`](ItemTable::combat_rules); each side's stats are
//!   gear-adjusted ([`Unit::combat_input`]); terrain comes from each unit's
//!   tile. Normal attacks cost no durability.
//! - **Weapon EXP.** After a combat, each side still on the map that struck
//!   gains weapon EXP in its weapon's kind ([`Event::WeaponExpGained`], then
//!   [`Event::WeaponRankUp`] if its rank rose; attacker first), before
//!   anyone falls. See [`crate::item`] for the formula.
//! - **Items** ([`UnitAction::UseItem`]): a player unit uses a consumable
//!   from the shared [`BattlePack`]; other factions use their own
//!   ([`Unit::consumables`]). The target is the unit itself or a non-hostile
//!   unit adjacent to `dest`. The item is used up ([`Event::ItemUsed`]),
//!   heals ([`Event::Healed`], never above max HP) and ends the action.
//! - **Equipping** ([`Command::Equip`]) is free: any weapon of the loadout
//!   the unit can wield, by a ready unit of the current phase. It doesn't
//!   end the action. No trading: loadouts are fixed for the battle.
//! - **Falling.** A unit at 0 HP falls ([`Event::UnitFell`]): it leaves the
//!   map, can't act, can't be targeted and doesn't block tiles. It moves to
//!   [`BattleState::fallen`]. Classic vs Casual only matters when the
//!   campaign applies the result (0801).
//! - **Outcome.** Defeat, in both modes, when a player lord falls or no player
//!   unit is left on the map. Victory per [`Objective`]. Both are checked
//!   after every command and when a turn ends; nothing else changes them
//!   (arrivals can't create a victory or a defeat). Loss is checked first.
//!   Once decided ([`Event::BattleEnded`]) every command fails with
//!   [`CommandError::BattleOver`] and the outcome never changes.
//! - **Survive `N`** wins, and a `turn_limit` of `N` loses, when turn `N`'s
//!   last phase ends: the moment `turn` would become `N + 1`.
//! - **Reinforcements** ([`Reinforcement`]) for turn `N` arrive at the start
//!   of their faction's phase on turn `N`, never acting on arrival. One whose
//!   tile is occupied waits and tries again at the same point next turn;
//!   the others of its wave still arrive. Earlier entries go first.
//! - **Errors change nothing.** [`BattleState::apply`] validates the whole
//!   command before touching the state, so on `Err` the state is unchanged.
//!
//! # Extending
//!
//! Every [`Event`] sequence of an `Act` ends with [`Event::UnitActed`] (unless
//! the unit fell), optionally followed by [`Event::BattleEnded`]. Skills that
//! grant movement after an action (`turn-structure.md`; e.g. "after attacking,
//! move 1 tile away") add their move as a new event just before `UnitActed`,
//! decided by the skill's rules; no [`UnitAction`] needs to change. Combat
//! Arts (0312) spend durability with [`Unit::spend_durability`], which
//! gives the [`Event::ItemBroke`] to emit.
//!
//! # Saving
//!
//! `BattleState` is serde-serialisable (RON via `content`/`app`, ADR-0019):
//! the map, units, fallen units, pending reinforcements, objective, turn,
//! phase, RNG position, battle pack and outcome are all saved. The
//! **terrain, class and item tables are not**: they are shared content, held by `Arc` and skipped. A
//! deserialised state has empty tables (every `Act` fails with
//! [`CommandError::UnknownClass`]) until [`BattleState::restore_tables`] is
//! called with the game's tables, as loaded from the same content. The map is
//! saved because it is battle state (terrain magic changes tiles, 0310).

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::class::{ClassDef, ClassId, ClassTable};
use crate::combat::{CombatHp, CombatOutcome, CombatantInput, Forecast, Side, forecast, resolve};
use crate::geom::Pos;
use crate::item::{BattlePack, ConsumableEffect, ItemDef, ItemId, ItemTable};
use crate::map::BattleMap;
use crate::movement::{MoveError, reachable};
use crate::rng::SimRng;
use crate::stats::StatValue;
use crate::terrain::TerrainTable;
use crate::unit::{Faction, Unit, UnitId};
use crate::weapon::{WeaponKind, WeaponRank};

/// A turn number, from 1.
pub type Turn = u32;

/// A phase of a turn: which factions act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// The `Player` faction; controlled by the player.
    Player,
    /// The `Enemy` faction; AI.
    Enemy,
    /// The `Ally` and `Neutral` factions ("green" units); AI.
    Other,
}

impl Phase {
    /// Every phase, in turn order.
    pub const ALL: [Phase; 3] = [Phase::Player, Phase::Enemy, Phase::Other];

    /// The phase in which units of `faction` act.
    pub fn of(faction: Faction) -> Phase {
        match faction {
            Faction::Player => Phase::Player,
            Faction::Enemy => Phase::Enemy,
            Faction::Ally | Faction::Neutral => Phase::Other,
        }
    }

    /// The phase after this one (`Other` → `Player`, which starts a new turn).
    #[must_use]
    pub fn next(self) -> Phase {
        match self {
            Phase::Player => Phase::Enemy,
            Phase::Enemy => Phase::Other,
            Phase::Other => Phase::Player,
        }
    }
}

/// How a battle ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    /// The objective was met.
    Victory,
    /// A loss condition was met: game over.
    Defeat,
}

/// What the player must do to win. Every objective except [`Survive`]
/// may have a `turn_limit`: the player loses if it isn't met when that
/// turn's last phase ends.
///
/// [`Survive`]: Objective::Survive
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Objective {
    /// Defeat every enemy on the map.
    Rout {
        /// Last turn to do it in.
        turn_limit: Option<Turn>,
    },
    /// Defeat one unit (a boss).
    DefeatUnit {
        /// The unit to defeat.
        unit: UnitId,
        /// Last turn to do it in.
        turn_limit: Option<Turn>,
    },
    /// Move a player unit onto a tile and choose [`UnitAction::Seize`].
    Seize {
        /// The tile to seize.
        pos: Pos,
        /// Whether only a lord may seize.
        by_lord: bool,
        /// Last turn to do it in.
        turn_limit: Option<Turn>,
    },
    /// Don't lose until the end of turn `turns`.
    Survive {
        /// The turn whose end wins.
        turns: Turn,
    },
}

impl Objective {
    /// The turn limit, if the objective has one.
    pub fn turn_limit(&self) -> Option<Turn> {
        match *self {
            Objective::Rout { turn_limit }
            | Objective::DefeatUnit { turn_limit, .. }
            | Objective::Seize { turn_limit, .. } => turn_limit,
            Objective::Survive { .. } => None,
        }
    }
}

/// A unit that joins the battle later. Its faction decides its phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reinforcement {
    /// The turn it arrives on (at the start of its faction's phase).
    pub turn: Turn,
    /// The unit, at its arrival tile. Its id must be unique in the battle.
    pub unit: Unit,
}

/// Everything needed to start a battle. Content validation (map files,
/// 0803) guarantees unique unit ids, one unit per tile and units on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSetup {
    /// The battlefield.
    pub map: BattleMap,
    /// Terrain rules for the map's tiles.
    pub terrain: Arc<TerrainTable>,
    /// Classes of every unit.
    pub classes: Arc<ClassTable>,
    /// Every item units carry.
    pub items: Arc<ItemTable>,
    /// The player side's consumables.
    pub pack: BattlePack,
    /// The units on the map at the start.
    pub units: Vec<Unit>,
    /// Units that arrive later.
    pub reinforcements: Vec<Reinforcement>,
    /// How to win.
    pub objective: Objective,
    /// Rewind charges for this battle, from the map's difficulty tier
    /// (0801); spent by turn rewind (0307).
    pub rewind_charges: u8,
    /// Seed of the battle's [`SimRng`].
    pub seed: u64,
}

/// What a unit does after moving. Ends its action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitAction {
    /// Nothing.
    Wait,
    /// Fight `target` with the weapon in loadout slot `slot` (equips it).
    Attack {
        /// The unit attacked.
        target: UnitId,
        /// The weapon's loadout slot.
        slot: usize,
    },
    /// Use a consumable on `target` (the unit itself or an adjacent ally).
    UseItem {
        /// Index in the battle pack (player units) or the unit's own
        /// consumables (other factions).
        pack_index: usize,
        /// Who it is used on.
        target: UnitId,
    },
    /// Seize the objective tile (only on it, see [`Objective::Seize`]).
    Seize,
}

/// A request to change the battle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    /// Move `unit` to `dest` (its own tile to stay) and do `action`.
    Act {
        /// The unit acting.
        unit: UnitId,
        /// Where it ends its move.
        dest: Pos,
        /// What it does there.
        action: UnitAction,
    },
    /// Equip the weapon in loadout slot `slot` of `unit`. Free: doesn't end
    /// the action.
    Equip {
        /// The unit.
        unit: UnitId,
        /// The weapon's loadout slot.
        slot: usize,
    },
    /// End the current phase.
    EndPhase,
}

/// Something that happened, for the UI to show. See the module docs for the
/// order of events.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
    /// A phase began (the banner).
    PhaseStarted {
        /// The turn.
        turn: Turn,
        /// The phase.
        phase: Phase,
    },
    /// Reinforcements appeared at the start of their phase, already done.
    UnitsArrived {
        /// The arrivals, in arrival order.
        units: Vec<UnitId>,
    },
    /// A unit moved along `path` (its start tile first).
    UnitMoved {
        /// The unit.
        unit: UnitId,
        /// Every tile of the move.
        path: Vec<Pos>,
    },
    /// A unit equipped the weapon in `slot`.
    Equipped {
        /// The unit.
        unit: UnitId,
        /// The loadout slot.
        slot: usize,
    },
    /// A combat was fought.
    CombatResolved {
        /// Who attacked.
        attacker: UnitId,
        /// Who was attacked.
        defender: UnitId,
        /// The numbers the combat used.
        forecast: Forecast,
        /// Every strike and the final HP.
        outcome: CombatOutcome,
    },
    /// A unit gained weapon EXP after a combat.
    WeaponExpGained {
        /// The unit.
        unit: UnitId,
        /// The weapon kind.
        kind: WeaponKind,
        /// EXP gained.
        amount: u32,
    },
    /// A unit's weapon rank rose.
    WeaponRankUp {
        /// The unit.
        unit: UnitId,
        /// The weapon kind.
        kind: WeaponKind,
        /// The new rank.
        rank: WeaponRank,
    },
    /// A weapon's durability reached 0.
    ItemBroke {
        /// The unit carrying it.
        unit: UnitId,
        /// The weapon.
        item: ItemId,
    },
    /// A unit used up a consumable on `target`.
    ItemUsed {
        /// The user.
        unit: UnitId,
        /// The consumable.
        item: ItemId,
        /// Who it was used on.
        target: UnitId,
    },
    /// A unit regained HP.
    Healed {
        /// The unit healed.
        target: UnitId,
        /// HP restored (after the max-HP cap).
        amount: StatValue,
    },
    /// A unit reached 0 HP and left the map.
    UnitFell {
        /// The unit.
        unit: UnitId,
    },
    /// A unit seized the objective tile.
    Seized {
        /// The unit.
        unit: UnitId,
        /// The tile.
        pos: Pos,
    },
    /// A unit finished its action and is done until its next phase.
    UnitActed {
        /// The unit.
        unit: UnitId,
    },
    /// The battle is over.
    BattleEnded {
        /// How it ended.
        outcome: Outcome,
    },
}

/// Why a command was refused. The state is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The battle has ended.
    BattleOver,
    /// No unit with this id is or was in the battle (reinforcements that
    /// haven't arrived included).
    UnknownUnit(UnitId),
    /// The unit has fallen.
    UnitFallen(UnitId),
    /// The unit doesn't act in the current phase.
    NotItsPhase {
        /// The unit.
        unit: UnitId,
        /// The current phase.
        phase: Phase,
    },
    /// The unit has already acted this phase.
    AlreadyActed(UnitId),
    /// A unit's class is not in the class table (e.g. tables not restored
    /// after loading).
    UnknownClass(ClassId),
    /// A unit stands outside the map.
    OffMap(UnitId),
    /// A tile's terrain is not in the terrain table.
    UnknownTerrain(Pos),
    /// The unit can't end its move on this tile.
    CannotStop(Pos),
    /// The target isn't hostile to the attacker.
    NotHostile(UnitId),
    /// The loadout slot holds no weapon.
    EmptySlot {
        /// The unit.
        unit: UnitId,
        /// The slot.
        slot: usize,
    },
    /// The unit can't wield this weapon (class kind or rank).
    CannotWield {
        /// The unit.
        unit: UnitId,
        /// The weapon.
        item: ItemId,
    },
    /// No item at this index of the pack (or the unit's consumables).
    NoItem {
        /// The unit.
        unit: UnitId,
        /// The index.
        index: usize,
    },
    /// An item id is not in the item table (e.g. tables not restored).
    UnknownItem(ItemId),
    /// The item can't be used (it isn't a consumable).
    NotConsumable(ItemId),
    /// The item's target is neither the user nor an adjacent ally.
    BadItemTarget(UnitId),
    /// The target is outside the attacker's weapon range from `dest`.
    OutOfRange {
        /// The target.
        target: UnitId,
        /// Its distance from `dest`.
        distance: u32,
    },
    /// Seizing isn't possible: not a seize map, not the seize tile, not a
    /// player unit, or not a lord when the map needs one.
    CannotSeize,
}

impl From<MoveError> for CommandError {
    fn from(e: MoveError) -> Self {
        match e {
            MoveError::UnknownUnit(id) => CommandError::UnknownUnit(id),
            MoveError::UnknownClass(c) => CommandError::UnknownClass(c),
            MoveError::OffMap(id) => CommandError::OffMap(id),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::BattleOver => f.write_str("the battle is over"),
            CommandError::UnknownUnit(id) => write!(f, "no unit with id {}", id.0),
            CommandError::UnitFallen(id) => write!(f, "unit {} has fallen", id.0),
            CommandError::NotItsPhase { unit, phase } => {
                write!(f, "unit {} doesn't act in the {phase:?} phase", unit.0)
            }
            CommandError::AlreadyActed(id) => write!(f, "unit {} has already acted", id.0),
            CommandError::UnknownClass(c) => write!(f, "unknown class \"{}\"", c.0),
            CommandError::OffMap(id) => write!(f, "unit {} is outside the map", id.0),
            CommandError::UnknownTerrain(p) => {
                write!(f, "unknown terrain at ({}, {})", p.x, p.y)
            }
            CommandError::CannotStop(p) => write!(f, "can't stop at ({}, {})", p.x, p.y),
            CommandError::NotHostile(id) => write!(f, "unit {} is not an enemy", id.0),
            CommandError::EmptySlot { unit, slot } => {
                write!(f, "unit {} has no weapon in slot {slot}", unit.0)
            }
            CommandError::CannotWield { unit, item } => {
                write!(f, "unit {} can't wield \"{}\"", unit.0, item.0)
            }
            CommandError::NoItem { unit, index } => {
                write!(f, "unit {} has no item {index}", unit.0)
            }
            CommandError::UnknownItem(i) => write!(f, "unknown item \"{}\"", i.0),
            CommandError::NotConsumable(i) => write!(f, "\"{}\" can't be used", i.0),
            CommandError::BadItemTarget(id) => {
                write!(f, "unit {} can't be given an item from here", id.0)
            }
            CommandError::OutOfRange { target, distance } => {
                write!(f, "unit {} is out of range ({distance} tiles)", target.0)
            }
            CommandError::CannotSeize => f.write_str("can't seize here"),
        }
    }
}

impl std::error::Error for CommandError {}

/// The shared content tables a battle reads. Not saved (see the module docs).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Tables {
    terrain: Arc<TerrainTable>,
    classes: Arc<ClassTable>,
    items: Arc<ItemTable>,
}

/// A running battle. Changed only by [`BattleState::apply`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleState {
    #[serde(skip)]
    tables: Tables,
    map: BattleMap,
    turn: Turn,
    phase: Phase,
    units: Vec<Unit>,
    fallen: Vec<Unit>,
    pending: Vec<Reinforcement>,
    objective: Objective,
    rewind_charges: u8,
    pack: BattlePack,
    rng: SimRng,
    outcome: Option<Outcome>,
}

/// A validated action, ready to carry out.
enum Step {
    Wait,
    Attack {
        target: UnitId,
        slot: usize,
        forecast: Forecast,
        /// The attacker's and defender's weapon kinds, for weapon EXP.
        kinds: [Option<WeaponKind>; 2],
    },
    UseItem {
        index: usize,
        item: ItemId,
        effect: ConsumableEffect,
        target: UnitId,
    },
    Seize,
}

impl BattleState {
    /// Starts the battle at turn 1, Player phase. The events are the first
    /// phase start (with any turn-1 player reinforcements), or
    /// [`Event::BattleEnded`] if it is already decided (no player units, or a
    /// rout with no enemies).
    pub fn new(setup: BattleSetup) -> (BattleState, Vec<Event>) {
        let mut state = BattleState {
            tables: Tables {
                terrain: setup.terrain,
                classes: setup.classes,
                items: setup.items,
            },
            map: setup.map,
            turn: 1,
            phase: Phase::Player,
            units: setup.units,
            fallen: Vec::new(),
            pending: setup.reinforcements,
            objective: setup.objective,
            rewind_charges: setup.rewind_charges,
            pack: setup.pack,
            rng: SimRng::new(setup.seed),
            outcome: None,
        };
        let mut events = Vec::new();
        if let Some(outcome) = state.judge() {
            state.finish(outcome, &mut events);
        } else {
            // Never skipped: judge() found player units on the map.
            state.start_phase(&mut events);
        }
        (state, events)
    }

    /// Reattaches the content tables after deserialising (see the module
    /// docs). They must be the tables the battle was started with.
    pub fn restore_tables(
        &mut self,
        terrain: Arc<TerrainTable>,
        classes: Arc<ClassTable>,
        items: Arc<ItemTable>,
    ) {
        self.tables = Tables {
            terrain,
            classes,
            items,
        };
    }

    /// The battlefield.
    pub fn map(&self) -> &BattleMap {
        &self.map
    }

    /// Terrain rules.
    pub fn terrain(&self) -> &TerrainTable {
        &self.tables.terrain
    }

    /// Classes.
    pub fn classes(&self) -> &ClassTable {
        &self.tables.classes
    }

    /// Items.
    pub fn items(&self) -> &ItemTable {
        &self.tables.items
    }

    /// The player side's consumables.
    pub fn pack(&self) -> &BattlePack {
        &self.pack
    }

    /// The current turn, from 1.
    pub fn turn(&self) -> Turn {
        self.turn
    }

    /// The current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Every unit on the map (living), in a stable order.
    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// The unit `id`, if it is on the map.
    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    /// Units that fell, in the order they fell (HP 0).
    pub fn fallen(&self) -> &[Unit] {
        &self.fallen
    }

    /// Reinforcements that haven't arrived yet.
    pub fn reinforcements(&self) -> &[Reinforcement] {
        &self.pending
    }

    /// How to win.
    pub fn objective(&self) -> Objective {
        self.objective
    }

    /// Rewind charges the battle started with.
    pub fn rewind_charges(&self) -> u8 {
        self.rewind_charges
    }

    /// How the battle ended, once it has.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// Applies `cmd`: validates it, changes the state and returns what
    /// happened. On `Err` nothing changed.
    pub fn apply(&mut self, cmd: &Command) -> Result<Vec<Event>, CommandError> {
        if self.outcome.is_some() {
            return Err(CommandError::BattleOver);
        }
        let mut events = Vec::new();
        match cmd {
            Command::EndPhase => self.end_phase(&mut events),
            Command::Equip { unit, slot } => {
                self.check_ready(*unit)?;
                self.check_wield(*unit, *slot)?;
                self.equip(*unit, *slot, &mut events);
            }
            Command::Act { unit, dest, action } => {
                let (path, step) = self.plan(*unit, *dest, action)?;
                self.act(*unit, path, step, &mut events);
            }
        }
        Ok(events)
    }

    /// Validates an `Act`: the move's path and the action to carry out.
    fn plan(
        &self,
        id: UnitId,
        dest: Pos,
        action: &UnitAction,
    ) -> Result<(Vec<Pos>, Step), CommandError> {
        let unit = self.check_ready(id)?;
        let reach = reachable(
            &self.map,
            &self.tables.terrain,
            &self.tables.classes,
            &self.units,
            id,
        )?;
        let path = reach
            .path_to(dest)
            .filter(|_| reach.is_stoppable(dest))
            .ok_or(CommandError::CannotStop(dest))?;
        let step = match *action {
            UnitAction::Wait => Step::Wait,
            UnitAction::Attack { target, slot } => {
                let (forecast, kinds) = self.plan_attack(unit, dest, target, slot)?;
                Step::Attack {
                    target,
                    slot,
                    forecast,
                    kinds,
                }
            }
            UnitAction::UseItem { pack_index, target } => {
                self.plan_item(unit, dest, pack_index, target)?
            }
            UnitAction::Seize => {
                self.check_seize(unit, dest)?;
                Step::Seize
            }
        };
        Ok((path, step))
    }

    /// The living unit `id` if it may act now: its phase, not yet acted.
    fn check_ready(&self, id: UnitId) -> Result<&Unit, CommandError> {
        let unit = self.living(id)?;
        if Phase::of(unit.faction) != self.phase {
            return Err(CommandError::NotItsPhase {
                unit: id,
                phase: self.phase,
            });
        }
        if unit.acted {
            return Err(CommandError::AlreadyActed(id));
        }
        Ok(unit)
    }

    /// Checks unit `id` can wield the weapon in `slot`.
    fn check_wield(&self, id: UnitId, slot: usize) -> Result<(), CommandError> {
        let unit = self.living(id)?;
        let copy = unit
            .loadout
            .weapon(slot)
            .ok_or(CommandError::EmptySlot { unit: id, slot })?;
        let class = self.class_of(unit)?;
        if self.tables.items.get(&copy.def).is_none() {
            return Err(CommandError::UnknownItem(copy.def.clone()));
        }
        match unit.usable_weapon(slot, class, &self.tables.items) {
            Some(_) => Ok(()),
            None => Err(CommandError::CannotWield {
                unit: id,
                item: copy.def.clone(),
            }),
        }
    }

    /// The forecast of `unit` attacking `target` from `dest` with the weapon
    /// in `slot`, and both sides' weapon kinds.
    fn plan_attack(
        &self,
        unit: &Unit,
        dest: Pos,
        target: UnitId,
        slot: usize,
    ) -> Result<(Forecast, [Option<WeaponKind>; 2]), CommandError> {
        let defender = self.living(target)?;
        if !unit.faction.is_hostile_to(defender.faction) {
            return Err(CommandError::NotHostile(target));
        }
        self.check_wield(unit.id, slot)?;
        let a = self.combatant(unit, dest, Some(slot))?;
        let d = self.combatant(defender, defender.pos, None)?;
        let distance = Pos::manhattan(dest, defender.pos);
        let forecast = forecast(&self.tables.items.combat_rules(), &a, &d, distance)
            .ok_or(CommandError::OutOfRange { target, distance })?;
        let kind = |c: &CombatantInput| c.weapon.as_ref().and_then(|w| w.kind);
        Ok((forecast, [kind(&a), kind(&d)]))
    }

    /// Validates `unit` using item `index` on `target` from `dest`.
    fn plan_item(
        &self,
        unit: &Unit,
        dest: Pos,
        index: usize,
        target: UnitId,
    ) -> Result<Step, CommandError> {
        let items = if unit.faction == Faction::Player {
            &self.pack.items
        } else {
            &unit.consumables
        };
        let item = items.get(index).ok_or(CommandError::NoItem {
            unit: unit.id,
            index,
        })?;
        let effect = match self.tables.items.get(item) {
            None => return Err(CommandError::UnknownItem(item.clone())),
            Some(ItemDef::Consumable(c)) => c.effect,
            Some(_) => return Err(CommandError::NotConsumable(item.clone())),
        };
        if target != unit.id {
            let other = self.living(target)?;
            if unit.faction.is_hostile_to(other.faction) || Pos::manhattan(dest, other.pos) != 1 {
                return Err(CommandError::BadItemTarget(target));
            }
        }
        Ok(Step::UseItem {
            index,
            item: item.clone(),
            effect,
            target,
        })
    }

    /// The class of `unit`.
    fn class_of(&self, unit: &Unit) -> Result<&ClassDef, CommandError> {
        self.tables
            .classes
            .get(&unit.class)
            .ok_or_else(|| CommandError::UnknownClass(unit.class.clone()))
    }

    /// `unit` as the combat maths sees it, standing on `pos`, with the
    /// weapon in `slot` (`None`: the equipped one).
    fn combatant(
        &self,
        unit: &Unit,
        pos: Pos,
        slot: Option<usize>,
    ) -> Result<CombatantInput<'_>, CommandError> {
        let class = self.class_of(unit)?;
        let tile = *self
            .map
            .tiles
            .get(pos)
            .ok_or(CommandError::OffMap(unit.id))?;
        let terrain = self
            .tables
            .terrain
            .get(tile)
            .ok_or(CommandError::UnknownTerrain(pos))?;
        Ok(unit.combat_input(
            class,
            &self.tables.classes,
            &self.tables.items,
            slot,
            terrain,
        ))
    }

    /// Whether `unit` may seize from `dest`.
    fn check_seize(&self, unit: &Unit, dest: Pos) -> Result<(), CommandError> {
        match self.objective {
            Objective::Seize { pos, by_lord, .. }
                if pos == dest && unit.faction == Faction::Player && (unit.is_lord || !by_lord) =>
            {
                Ok(())
            }
            _ => Err(CommandError::CannotSeize),
        }
    }

    /// The living unit `id`, or why there is none.
    fn living(&self, id: UnitId) -> Result<&Unit, CommandError> {
        if let Some(unit) = self.unit(id) {
            Ok(unit)
        } else if self.fallen.iter().any(|u| u.id == id) {
            Err(CommandError::UnitFallen(id))
        } else {
            Err(CommandError::UnknownUnit(id))
        }
    }

    fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    /// Carries out a validated `Act`: `path` ends at the unit's `dest`.
    fn act(&mut self, id: UnitId, path: Vec<Pos>, step: Step, events: &mut Vec<Event>) {
        let dest = path.last().copied();
        if let Some(dest) = dest
            && path.len() > 1
        {
            if let Some(u) = self.unit_mut(id) {
                u.pos = dest;
            }
            events.push(Event::UnitMoved { unit: id, path });
        }
        let mut seized = false;
        match step {
            Step::Wait => {}
            Step::Attack {
                target,
                slot,
                forecast,
                kinds,
            } => {
                if self
                    .unit(id)
                    .is_some_and(|u| u.loadout.equipped != Some(slot))
                {
                    self.equip(id, slot, events);
                }
                self.fight(id, target, forecast, kinds, events);
            }
            Step::UseItem {
                index,
                item,
                effect,
                target,
            } => self.use_item(id, index, item, effect, target, events),
            Step::Seize => {
                if let Some(pos) = dest {
                    events.push(Event::Seized { unit: id, pos });
                }
                seized = true;
            }
        }
        if let Some(u) = self.unit_mut(id) {
            u.acted = true;
            events.push(Event::UnitActed { unit: id });
        }
        let outcome = if seized {
            Some(Outcome::Victory)
        } else {
            self.judge()
        };
        if let Some(outcome) = outcome {
            self.finish(outcome, events);
        }
    }

    /// Equips the weapon in `slot` of unit `id` (already validated).
    fn equip(&mut self, id: UnitId, slot: usize, events: &mut Vec<Event>) {
        if let Some(u) = self.unit_mut(id) {
            u.loadout.equipped = Some(slot);
            events.push(Event::Equipped { unit: id, slot });
        }
    }

    /// Uses up item `index` of unit `id`'s source (validated) on `target`.
    fn use_item(
        &mut self,
        id: UnitId,
        index: usize,
        item: ItemId,
        effect: ConsumableEffect,
        target: UnitId,
        events: &mut Vec<Event>,
    ) {
        let Some(user) = self.unit_mut(id) else {
            return;
        };
        if user.faction == Faction::Player {
            self.pack.items.remove(index);
        } else {
            user.consumables.remove(index);
        }
        events.push(Event::ItemUsed {
            unit: id,
            item,
            target,
        });
        if let Some(t) = self.unit_mut(target) {
            let max = t.stats.hp;
            let healed = match effect {
                ConsumableEffect::Heal(amount) => t.hp.saturating_add(amount.max(0)).min(max),
                ConsumableEffect::HealFull => max,
            }
            .max(t.hp);
            let amount = healed - t.hp;
            t.hp = healed;
            events.push(Event::Healed { target, amount });
        }
    }

    /// Plays out a combat, gives weapon EXP and removes whoever fell.
    fn fight(
        &mut self,
        attacker: UnitId,
        defender: UnitId,
        forecast: Forecast,
        kinds: [Option<WeaponKind>; 2],
        events: &mut Vec<Event>,
    ) {
        let hp = |s: &Self, id| {
            s.unit(id)
                .map_or(CombatHp { current: 0, max: 0 }, |u| CombatHp {
                    current: u.hp,
                    max: u.stats.hp,
                })
        };
        let (a, d) = (hp(self, attacker), hp(self, defender));
        let outcome = resolve(
            &self.tables.items.combat_rules(),
            &forecast,
            a,
            d,
            &mut self.rng,
        );
        for (id, left) in [
            (attacker, outcome.attacker_hp),
            (defender, outcome.defender_hp),
        ] {
            if let Some(u) = self.unit_mut(id) {
                u.hp = left;
            }
        }
        let exp = weapon_exp(&self.tables.items, &outcome, a.current, d.current);
        events.push(Event::CombatResolved {
            attacker,
            defender,
            forecast,
            outcome,
        });
        for ((id, kind), amount) in [attacker, defender].into_iter().zip(kinds).zip(exp) {
            if let Some(kind) = kind {
                self.gain_weapon_exp(id, kind, amount, events);
            }
        }
        for id in [defender, attacker] {
            if let Some(i) = self.units.iter().position(|u| u.id == id && u.hp <= 0) {
                self.fallen.push(self.units.remove(i));
                events.push(Event::UnitFell { unit: id });
            }
        }
    }

    /// Gives unit `id`, if still standing, `amount` weapon EXP in `kind`.
    fn gain_weapon_exp(
        &mut self,
        id: UnitId,
        kind: WeaponKind,
        amount: u32,
        events: &mut Vec<Event>,
    ) {
        if amount == 0 {
            return;
        }
        let tables = self.tables.clone();
        let Some(unit) = self.unit_mut(id).filter(|u| u.hp > 0) else {
            return;
        };
        let Some(max) = tables
            .classes
            .get(&unit.class)
            .and_then(|c| c.weapon(kind))
            .map(|w| w.max)
        else {
            return;
        };
        let rank_up = unit.gain_weapon_exp(kind, amount, max, &tables.items.rules);
        events.push(Event::WeaponExpGained {
            unit: id,
            kind,
            amount,
        });
        if let Some(rank) = rank_up {
            events.push(Event::WeaponRankUp {
                unit: id,
                kind,
                rank,
            });
        }
    }

    /// Ends the current phase and starts the next one that isn't skipped,
    /// ending turns on the way. A turn can't pass without a phase starting:
    /// the Player phase always has units (no player units is a defeat), so
    /// at most [`Phase::ALL`]`.len()` slots are visited.
    fn end_phase(&mut self, events: &mut Vec<Event>) {
        for _ in Phase::ALL {
            if self.phase == Phase::Other {
                if let Some(outcome) = self.end_of_turn() {
                    self.finish(outcome, events);
                    return;
                }
                self.turn += 1;
            }
            self.phase = self.phase.next();
            if self.start_phase(events) {
                return;
            }
        }
    }

    /// Starts the current phase (see the module docs). Returns `false`, with
    /// no events, if it is skipped: no units of its factions, even after
    /// arrivals.
    fn start_phase(&mut self, events: &mut Vec<Event>) -> bool {
        let phase = self.phase;
        for u in self
            .units
            .iter_mut()
            .filter(|u| Phase::of(u.faction) == phase)
        {
            u.acted = false;
        }
        let arrived = self.arrive();
        if !self.units.iter().any(|u| Phase::of(u.faction) == phase) {
            return false;
        }
        if !arrived.is_empty() {
            events.push(Event::UnitsArrived { units: arrived });
        }
        events.push(Event::PhaseStarted {
            turn: self.turn,
            phase,
        });
        true
    }

    /// Places the current phase's due reinforcements whose tiles are free,
    /// already done. Returns their ids.
    fn arrive(&mut self) -> Vec<UnitId> {
        let mut arrived = Vec::new();
        let mut waiting = Vec::new();
        for r in std::mem::take(&mut self.pending) {
            let due = r.turn <= self.turn && Phase::of(r.unit.faction) == self.phase;
            if due && !self.units.iter().any(|u| u.pos == r.unit.pos) {
                let mut unit = r.unit;
                unit.acted = true;
                arrived.push(unit.id);
                self.units.push(unit);
            } else {
                waiting.push(r);
            }
        }
        self.pending = waiting;
        arrived
    }

    /// The outcome decided by the units alone: defeat first, then the
    /// objective. Turn-based results are [`Self::end_of_turn`]'s.
    fn judge(&self) -> Option<Outcome> {
        let lord_fell = self
            .fallen
            .iter()
            .any(|u| u.faction == Faction::Player && u.is_lord);
        let side_left = |f| self.units.iter().any(|u| u.faction == f);
        if lord_fell || !side_left(Faction::Player) {
            return Some(Outcome::Defeat);
        }
        let won = match self.objective {
            Objective::Rout { .. } => !side_left(Faction::Enemy),
            Objective::DefeatUnit { unit, .. } => self.fallen.iter().any(|u| u.id == unit),
            Objective::Seize { .. } | Objective::Survive { .. } => false,
        };
        won.then_some(Outcome::Victory)
    }

    /// The outcome decided as the current turn's last phase ends.
    fn end_of_turn(&self) -> Option<Outcome> {
        match self.objective {
            Objective::Survive { turns } => (self.turn == turns).then_some(Outcome::Victory),
            other => (other.turn_limit() == Some(self.turn)).then_some(Outcome::Defeat),
        }
    }

    fn finish(&mut self, outcome: Outcome, events: &mut Vec<Event>) {
        self.outcome = Some(outcome);
        events.push(Event::BattleEnded { outcome });
    }
}

/// Weapon EXP earned by the attacker and the defender in `outcome`, given
/// their HP going in. No Combat Arts yet (0312).
fn weapon_exp(
    items: &ItemTable,
    outcome: &CombatOutcome,
    attacker_hp: StatValue,
    defender_hp: StatValue,
) -> [u32; 2] {
    // HP before each strike: [attacker, defender].
    let mut hp = [attacker_hp, defender_hp];
    let mut struck = [0usize; 2];
    let mut hits = [0usize; 2];
    let mut dealt: [StatValue; 2] = [0, 0];
    for s in &outcome.strikes {
        let (me, target) = match s.by {
            Side::Attacker => (0, 1),
            Side::Defender => (1, 0),
        };
        struck[me] += 1;
        hits[me] += usize::from(s.hit);
        if !s.healed {
            dealt[me] += (hp[target] - s.target_hp_after).max(0);
        }
        hp[target] = s.target_hp_after;
    }
    [0, 1].map(|i| items.rules.weapon_exp(struck[i], hits[i], dealt[i], false))
}

#[cfg(test)]
mod tests;
