//! What plays the Player phase.

use trpg_core::{AiWeights, BattleState, Command, next_command};

/// A player made of code. [`play_battle`](crate::play_battle) asks it for
/// every command of the Player phase, the phase's end included.
///
/// A bot is built for one try, with a `seed: u64` for any randomness of its
/// own and for reseeding the copies it plans on
/// ([`BattleState::reseed_luck`], ADR-0033): it must never learn the real
/// battle's luck. The same seed and the same states must give the same
/// commands.
pub trait PlayerBot {
    /// The next command in `state`, which is in the Player phase and not
    /// over. It must be one the battle accepts
    /// ([`trpg_core::legal_commands`]); a refused command ends the try with
    /// a [`PlayError`](crate::PlayError).
    fn choose(&mut self, state: &BattleState) -> Command;
}

/// The baseline: the enemy AI ([`next_command`]) playing the player's
/// side, ending the phase when it has nothing left to do. It never uses
/// items and has no randomness of its own, so it takes no seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaselineBot {
    weights: AiWeights,
}

impl BaselineBot {
    /// A baseline bot scoring its attacks with `weights`.
    pub fn new(weights: AiWeights) -> Self {
        Self { weights }
    }
}

impl PlayerBot for BaselineBot {
    fn choose(&mut self, state: &BattleState) -> Command {
        next_command(state, &self.weights).unwrap_or(Command::EndPhase)
    }
}
