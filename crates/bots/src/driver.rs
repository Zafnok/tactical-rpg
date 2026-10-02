//! One try of a battle.

use std::fmt;

use trpg_core::{AiWeights, BattleState, Command, CommandError, Phase, Turn, next_command};

use crate::{BattleMeasures, PlayerBot};

/// A command the battle refused during a try: a bug in the bot (or in the
/// AI), never skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayError {
    /// The refused command.
    pub command: Command,
    /// The turn it was issued on.
    pub turn: Turn,
    /// The phase it was issued in.
    pub phase: Phase,
    /// Why it was refused.
    pub error: CommandError,
}

impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "turn {}, {:?} phase: {:?} refused: {}",
            self.turn, self.phase, self.command, self.error
        )
    }
}

impl std::error::Error for PlayError {}

/// Plays `state` to the battle's end, or until turn `turn_cap` has passed:
/// `bot` plays the Player phase, and every other phase is played as the
/// game plays it, the AI's command ([`next_command`] with `enemy_weights`)
/// or else the phase's end.
///
/// A try stopped by the cap has no outcome, and its
/// [`turns`](BattleMeasures::turns) is `turn_cap + 1`, the turn that was
/// starting.
///
/// The bot never rewinds (ADR-0033): no `BattleHistory` is built.
pub fn play_battle(
    mut state: BattleState,
    bot: &mut dyn PlayerBot,
    enemy_weights: &AiWeights,
    turn_cap: Turn,
) -> Result<BattleMeasures, Box<PlayError>> {
    let mut measures = BattleMeasures::new(&state);
    while state.outcome().is_none() && state.turn() <= turn_cap {
        let phase = state.phase();
        let command = if phase == Phase::Player {
            bot.choose(&state)
        } else {
            next_command(&state, enemy_weights).unwrap_or(Command::EndPhase)
        };
        match state.apply(&command) {
            Ok(events) => measures.observe(&state, &events),
            Err(error) => {
                return Err(Box::new(PlayError {
                    command,
                    turn: state.turn(),
                    phase,
                    error,
                }));
            }
        }
    }
    Ok(measures)
}

#[cfg(test)]
mod tests {
    use trpg_core::{AiBehavior, Outcome, UnitAction, UnitId};

    use super::*;
    use crate::BaselineBot;
    use crate::testkit::{content, enemy, player, rigged, start, weak};

    /// Ends every Player phase at once. It gives up (panics) when asked for
    /// more phases than any test here plays, so a driver that never stops
    /// fails a test instead of hanging it.
    #[derive(Default)]
    struct Passive {
        phases: u32,
    }

    impl PlayerBot for Passive {
        fn choose(&mut self, _: &BattleState) -> Command {
            self.phases += 1;
            assert!(self.phases <= 100, "the try never stops");
            Command::EndPhase
        }
    }

    /// Orders a unit that isn't in the battle.
    struct Broken;

    impl PlayerBot for Broken {
        fn choose(&mut self, state: &BattleState) -> Command {
            if state.turn() < 2 {
                return Command::EndPhase;
            }
            Command::Act {
                unit: UnitId(99),
                dest: trpg_core::Pos::new(0, 0),
                action: UnitAction::Wait,
            }
        }
    }

    fn stationary(mut unit: trpg_core::Unit) -> trpg_core::Unit {
        unit.ai = AiBehavior::Stationary;
        unit
    }

    #[test]
    fn baseline_bot_wins_a_battle_it_cant_lose() {
        let state = start(vec![rigged(player(1, 5)), weak(stationary(enemy(2, 0)))]);
        let weights = content().ai;
        let mut bot = BaselineBot::new(weights);
        let measures = play_battle(state, &mut bot, &weights, 60).unwrap();
        assert_eq!(measures.outcome, Some(Outcome::Victory));
        assert_eq!(measures.player_fallen, vec![]);
        // Five tiles to walk at 5 a turn: it strikes on turn 2 at the latest.
        assert!(measures.turns <= 2, "{measures:?}");
        assert!(measures.player_commands >= 1);
    }

    #[test]
    fn the_enemy_phase_is_played_by_the_ai() {
        // The player does nothing; the enemy walks over and fells them.
        let state = start(vec![weak(player(1, 8)), rigged(enemy(2, 0))]);
        let measures = play_battle(state, &mut Passive::default(), &content().ai, 60).unwrap();
        assert_eq!(measures.outcome, Some(Outcome::Defeat));
        assert_eq!(measures.player_fallen.len(), 1);
        assert_eq!(measures.player_fallen[0].0, UnitId(1));
        // One EndPhase per turn from the bot.
        assert_eq!(measures.player_commands, measures.turns);
        assert!(measures.commands > measures.player_commands);
    }

    #[test]
    fn a_try_stops_when_the_turn_cap_has_passed() {
        // Nobody moves: the battle never ends.
        let state = start(vec![player(1, 8), stationary(enemy(2, 0))]);
        let measures = play_battle(state, &mut Passive::default(), &content().ai, 3).unwrap();
        assert_eq!(measures.outcome, None);
        // Turns 1 to 3 are played in full; turn 4 only starts.
        assert_eq!(measures.turns, 4);
        assert_eq!(measures.player_commands, 3);
    }

    #[test]
    fn a_turn_cap_of_zero_plays_nothing() {
        let state = start(vec![player(1, 8), stationary(enemy(2, 0))]);
        let measures =
            play_battle(state.clone(), &mut Passive::default(), &content().ai, 0).unwrap();
        assert_eq!(measures, BattleMeasures::new(&state));
    }

    #[test]
    fn a_battle_already_over_plays_nothing() {
        let mut state = start(vec![rigged(player(1, 5)), weak(enemy(2, 6))]);
        let attack = Command::Act {
            unit: UnitId(1),
            dest: trpg_core::Pos::new(5, 4),
            action: crate::testkit::attack(2),
        };
        state.apply(&attack).unwrap();
        let measures = play_battle(state, &mut Broken, &content().ai, 60).unwrap();
        assert_eq!(measures.outcome, Some(Outcome::Victory));
        assert_eq!(measures.commands, 0);
    }

    #[test]
    fn a_refused_command_ends_the_try_with_the_command_and_turn() {
        let state = start(vec![player(1, 8), stationary(enemy(2, 0))]);
        let error = play_battle(state, &mut Broken, &content().ai, 60).unwrap_err();
        assert_eq!(error.turn, 2);
        assert_eq!(error.phase, Phase::Player);
        assert_eq!(error.error, CommandError::UnknownUnit(UnitId(99)));
        assert!(matches!(
            error.command,
            Command::Act {
                unit: UnitId(99),
                ..
            }
        ));
        let text = error.to_string();
        assert!(text.starts_with("turn 2, Player phase: Act {"), "{text}");
        assert!(text.contains("refused: "), "{text}");
    }
}
