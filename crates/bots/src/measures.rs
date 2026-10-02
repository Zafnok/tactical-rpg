//! What happened in one try of a battle.

use std::collections::BTreeMap;

use trpg_core::{BattleState, Event, Faction, ItemId, Outcome, Phase, Turn, UnitId};

/// The measures of one try (`docs/design/playtest-bots.md`): built from the
/// battle's start with [`BattleMeasures::new`], then told every applied
/// command's events with [`BattleMeasures::observe`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleMeasures {
    /// How the battle ended; `None` while it is on (a try stopped by the
    /// turn cap keeps `None`).
    pub outcome: Option<Outcome>,
    /// The last turn started.
    pub turns: Turn,
    /// Each player unit that fell (died in Classic, retreated in Casual) and
    /// the turn it fell on, in the order they fell.
    pub player_fallen: Vec<(UnitId, Turn)>,
    /// Consumables used by player units, counted per item.
    pub items_used: BTreeMap<ItemId, u32>,
    /// Commands applied, every side's.
    pub commands: u32,
    /// Commands applied in the Player phase.
    pub player_commands: u32,
    /// The phase the next command is issued in.
    phase: Phase,
}

impl BattleMeasures {
    /// Measures of a try that starts at `state`.
    pub fn new(state: &BattleState) -> Self {
        Self {
            outcome: state.outcome(),
            turns: state.turn(),
            player_fallen: Vec::new(),
            items_used: BTreeMap::new(),
            commands: 0,
            player_commands: 0,
            phase: state.phase(),
        }
    }

    /// Counts one applied command: `events` are what it gave and
    /// `state_after` is the battle after it.
    pub fn observe(&mut self, state_after: &BattleState, events: &[Event]) {
        self.commands += 1;
        if self.phase == Phase::Player {
            self.player_commands += 1;
        }
        for event in events {
            match event {
                Event::PhaseStarted { turn, phase } => {
                    self.turns = self.turns.max(*turn);
                    self.phase = *phase;
                }
                Event::UnitFell { unit } => {
                    let fallen = state_after.fallen().iter().find(|u| u.id == *unit);
                    if fallen.is_some_and(|u| u.faction == Faction::Player) {
                        self.player_fallen.push((*unit, self.turns));
                    }
                }
                Event::ItemUsed { unit, item, .. } => {
                    if is_player(state_after, *unit) {
                        *self.items_used.entry(item.clone()).or_default() += 1;
                    }
                }
                Event::BattleEnded { outcome } => self.outcome = Some(*outcome),
                _ => {}
            }
        }
    }

    /// Every consumable used by player units.
    pub fn items_total(&self) -> u32 {
        self.items_used.values().sum()
    }
}

/// Whether `unit`, standing or fallen, is a player unit.
fn is_player(state: &BattleState, unit: UnitId) -> bool {
    let fallen = || state.fallen().iter().find(|u| u.id == unit);
    state
        .unit(unit)
        .or_else(fallen)
        .is_some_and(|u| u.faction == Faction::Player)
}

#[cfg(test)]
mod tests {
    use trpg_core::{Command, UnitAction};

    use super::*;
    use crate::testkit::{Scripted, attack, enemy, player, rigged, start, weak};

    /// Two turns of a battle on the test map's plain row: on turn 2 player
    /// 2 fells enemy 4, then enemy 3 fells player 1.
    fn two_falls() -> (Scripted, BattleMeasures) {
        let mut battle = Scripted::new(start(vec![
            weak(player(1, 5)),
            rigged(player(2, 2)),
            rigged(enemy(3, 6)),
            weak(enemy(4, 1)),
        ]));
        let start = battle.measures.clone();
        battle.next_player_phase();
        battle.act(2, 2, attack(4));
        battle.end_phase();
        battle.act(3, 6, attack(1));
        (battle, start)
    }

    #[test]
    fn measures_start_from_the_state() {
        let (_, start) = two_falls();
        assert_eq!(start.outcome, None);
        assert_eq!(start.turns, 1);
        assert_eq!(start.player_fallen, vec![]);
        assert_eq!(start.items_total(), 0);
        assert_eq!((start.commands, start.player_commands), (0, 0));
    }

    #[test]
    fn measures_count_fallen_player_units_only() {
        let (battle, _) = two_falls();
        let fallen: Vec<UnitId> = battle.state.fallen().iter().map(|u| u.id).collect();
        assert_eq!(fallen, [UnitId(4), UnitId(1)]);
        assert_eq!(battle.measures.player_fallen, [(UnitId(1), 2)]);
        assert_eq!(battle.measures.outcome, None);
    }

    #[test]
    fn measures_ignore_a_fall_of_a_unit_the_state_doesnt_know() {
        let (battle, _) = two_falls();
        let mut measures = battle.measures.clone();
        measures.observe(&battle.state, &[Event::UnitFell { unit: UnitId(99) }]);
        assert_eq!(measures.player_fallen, battle.measures.player_fallen);
    }

    #[test]
    fn measures_count_items_used_by_player_units() {
        let mut battle = Scripted::new(start(vec![player(1, 5), player(2, 4), enemy(3, 0)]));
        let potion = ItemId::new("potion");
        let elixir = ItemId::new("elixir");
        let use_on = |target| UnitAction::UseItem {
            pack_index: 0,
            target: UnitId(target),
        };
        battle.act(1, 5, use_on(1));
        assert_eq!(battle.measures.items_used, [(potion.clone(), 1)].into());
        battle.act(2, 4, use_on(1));
        battle.next_player_phase();
        battle.act(1, 5, use_on(2));
        let used = [(elixir.clone(), 1), (potion.clone(), 2)];
        assert_eq!(battle.measures.items_used, used.into());
        assert_eq!(battle.measures.items_total(), 3);

        // Only player units' items count (the battle lets no one else use
        // one, so the events are made up).
        let by = |unit, target| Event::ItemUsed {
            unit: UnitId(unit),
            item: potion.clone(),
            target: UnitId(target),
        };
        let mut measures = battle.measures.clone();
        measures.observe(&battle.state, &[by(3, 1), by(99, 1)]);
        assert_eq!(measures.items_total(), 3);
        measures.observe(&battle.state, &[by(2, 3)]);
        assert_eq!(measures.items_used[&potion], 3);
    }

    #[test]
    fn measures_count_an_item_used_by_a_player_unit_that_then_fell() {
        let (battle, _) = two_falls();
        let mut measures = battle.measures.clone();
        let by = |unit| Event::ItemUsed {
            unit: UnitId(unit),
            item: ItemId::new("potion"),
            target: UnitId(unit),
        };
        // Player 1 and enemy 4 have both fallen.
        measures.observe(&battle.state, &[by(4)]);
        assert_eq!(measures.items_total(), 0);
        measures.observe(&battle.state, &[by(1)]);
        assert_eq!(measures.items_total(), 1);
    }

    #[test]
    fn measures_turns_is_the_last_turn_started() {
        let mut battle = Scripted::new(start(vec![player(1, 5), enemy(3, 0)]));
        assert_eq!(battle.measures.turns, 1);
        battle.act(1, 5, UnitAction::Wait);
        assert_eq!(battle.measures.turns, 1);
        battle.next_player_phase();
        battle.next_player_phase();
        assert_eq!(battle.state.turn(), 3);
        assert_eq!(battle.measures.turns, 3);
        // A turn never goes back.
        let old = Event::PhaseStarted {
            turn: 2,
            phase: Phase::Enemy,
        };
        battle.measures.observe(&battle.state, &[old]);
        assert_eq!(battle.measures.turns, 3);
    }

    #[test]
    fn measures_count_commands_and_the_player_phases_commands() {
        let mut battle = Scripted::new(start(vec![player(1, 5), enemy(3, 0)]));
        battle.act(1, 5, UnitAction::Wait);
        assert_eq!(
            (battle.measures.commands, battle.measures.player_commands),
            (1, 1)
        );
        // Ending the Player phase is the player's command; what follows is
        // not.
        battle.end_phase();
        assert_eq!(battle.state.phase(), Phase::Enemy);
        battle.act(3, 0, UnitAction::Wait);
        battle.next_player_phase();
        let enemy_side = battle.measures.commands - battle.measures.player_commands;
        assert_eq!(battle.measures.player_commands, 2);
        assert_eq!(enemy_side, 2);
        battle.step(&Command::EndPhase);
        assert_eq!(battle.measures.player_commands, 3);
    }

    #[test]
    fn measures_record_the_outcome() {
        // The last enemy falls: victory.
        let mut battle = Scripted::new(start(vec![rigged(player(1, 5)), weak(enemy(2, 6))]));
        battle.act(1, 5, attack(2));
        assert_eq!(battle.state.outcome(), Some(Outcome::Victory));
        assert_eq!(battle.measures.outcome, Some(Outcome::Victory));
        assert_eq!(battle.measures.player_fallen, vec![]);

        // The last player unit falls: defeat, on turn 1.
        let mut battle = Scripted::new(start(vec![weak(player(1, 5)), rigged(enemy(2, 6))]));
        battle.end_phase();
        battle.act(2, 6, attack(1));
        assert_eq!(battle.measures.outcome, Some(Outcome::Defeat));
        assert_eq!(battle.measures.player_fallen, [(UnitId(1), 1)]);
    }
}
