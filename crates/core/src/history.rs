//! Turn rewind (ticket 0307, `docs/design/death-and-difficulty.md`): a
//! battle's first state and every command applied since, so any earlier
//! point can be rebuilt by replaying.
//!
//! # Rules
//!
//! - **Points.** Point `n` is the battle after its first `n` commands
//!   ([`BattleHistory::state_at`]); point 0 is the state the battle started
//!   in. Rewinding to point `n` undoes command `n` and every one after it,
//!   the enemy's included.
//! - **Charges.** A battle starts with [`BattleState::rewind_charges`]
//!   charges (from the map's difficulty tier, 0801). A rewind costs one
//!   however far back it goes; with none left it is refused. Charges don't
//!   carry between battles, and restarting a battle starts a fresh history
//!   with every charge back.
//! - **Luck is part of the state.** The battle's [`SimRng`](crate::SimRng)
//!   is in [`BattleState`], so replaying rebuilds the exact same state:
//!   **rewinding and then doing the same thing again gives the same
//!   result** (like Echoes' Turnwheel). Doing something different consumes
//!   the RNG differently and so changes what follows naturally.
//! - **Saving.** The history is serialisable for the suspend save (0802).
//!   Like any saved [`BattleState`] (ADR-0020), the first state comes back
//!   without its content tables: the loader must call
//!   [`BattleHistory::restore_tables`] before replaying.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    ArtTable, BattleState, ClassTable, Command, Event, ItemTable, SkillTable, SpellTable,
    TerrainTable,
};

/// A battle's first state, the commands applied since and the rewind
/// charges left.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleHistory {
    initial: BattleState,
    commands: Vec<Command>,
    charges_left: u8,
}

/// One past command, as shown by the rewind screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replayed {
    /// The battle just before the command (the point a rewind to it goes
    /// back to).
    pub before: BattleState,
    /// The command.
    pub command: Command,
    /// What it did.
    pub events: Vec<Event>,
}

/// Why a rewind was refused. Nothing changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewindError {
    /// No charges left.
    NoCharges,
    /// There is no earlier point `n` (it must be below the number of
    /// commands).
    NoSuchPoint(usize),
}

impl fmt::Display for RewindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCharges => f.write_str("no rewind charges left"),
            Self::NoSuchPoint(n) => write!(f, "no earlier point {n} to rewind to"),
        }
    }
}

impl std::error::Error for RewindError {}

impl BattleHistory {
    /// A history starting at `initial` (a battle just started), with its
    /// [`rewind_charges`](BattleState::rewind_charges).
    pub fn new(initial: BattleState) -> Self {
        Self {
            charges_left: initial.rewind_charges(),
            initial,
            commands: Vec::new(),
        }
    }

    /// Records `cmd`, which was just applied to the battle successfully.
    /// (A refused command changes nothing, so recording one by mistake
    /// replays as nothing too.)
    pub fn push(&mut self, cmd: Command) {
        self.commands.push(cmd);
    }

    /// The commands applied so far, oldest first.
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// How many commands were applied (the latest point).
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether no command was applied yet (nothing to rewind).
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Rewind charges left.
    pub fn charges_left(&self) -> u8 {
        self.charges_left
    }

    /// The battle after its first `n` commands (all of them if `n` is past
    /// the end).
    pub fn state_at(&self, n: usize) -> BattleState {
        let mut state = self.initial.clone();
        for cmd in self.commands.iter().take(n) {
            // A refused command changes nothing, as when it was first sent.
            let _ = state.apply(cmd);
        }
        state
    }

    /// Every command replayed in order, with the battle just before it and
    /// its events: one replay for the whole list.
    pub fn replay(&self) -> Vec<Replayed> {
        let mut state = self.initial.clone();
        self.commands
            .iter()
            .map(|cmd| {
                let before = state.clone();
                let events = state.apply(cmd).unwrap_or_default();
                Replayed {
                    before,
                    command: cmd.clone(),
                    events,
                }
            })
            .collect()
    }

    /// Goes back to point `n` (before command `n`), forgetting that command
    /// and every later one, for one charge. Returns the battle at that
    /// point.
    pub fn rewind_to(&mut self, n: usize) -> Result<BattleState, RewindError> {
        if n >= self.commands.len() {
            return Err(RewindError::NoSuchPoint(n));
        }
        if self.charges_left == 0 {
            return Err(RewindError::NoCharges);
        }
        self.charges_left -= 1;
        self.commands.truncate(n);
        Ok(self.state_at(n))
    }

    /// Reattaches the content tables to the first state after
    /// deserialising ([`BattleState::restore_tables`]).
    pub fn restore_tables(
        &mut self,
        terrain: Arc<TerrainTable>,
        classes: Arc<ClassTable>,
        items: Arc<ItemTable>,
        spells: Arc<SpellTable>,
        skills: Arc<SkillTable>,
        arts: Arc<ArtTable>,
    ) {
        self.initial
            .restore_tables(terrain, classes, items, spells, skills, arts);
    }
}

#[cfg(test)]
mod tests;
