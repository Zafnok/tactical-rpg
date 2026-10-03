use proptest::prelude::*;

use super::*;
use crate::battle::tests::{arb_setup, attack, cast, p, setup};
use crate::legal_commands;
use crate::{Pos, UnitAction, UnitId};

fn ron(state: &BattleState) -> String {
    ron::to_string(state).unwrap()
}

/// Unit 2 attacks enemy 4 (moved next to it), then the player phase ends.
fn fought() -> (BattleHistory, BattleState, BattleState) {
    let mut units = cast();
    units[3].pos = p(3, 2);
    let (mut s, _) = BattleState::new(setup(units));
    let mut h = BattleHistory::new(s.clone());
    let before_attack = s.clone();
    let hit = Command::Act {
        unit: UnitId(2),
        dest: p(2, 2),
        action: attack(4),
    };
    s.apply(&hit).unwrap();
    h.push(hit);
    s.apply(&Command::EndPhase).unwrap();
    h.push(Command::EndPhase);
    (h, before_attack, s)
}

#[test]
fn a_new_history_has_the_battles_charges_and_no_commands() {
    let (s, _) = BattleState::new(setup(cast()));
    let h = BattleHistory::new(s.clone());
    assert_eq!(h.charges_left(), 3);
    assert!(h.is_empty());
    assert_eq!(h.len(), 0);
    assert_eq!(h.state_at(0), s);
    assert!(h.replay().is_empty());
}

#[test]
fn rewinding_restores_exactly_the_state_before_the_attack() {
    let (mut h, before_attack, now) = fought();
    assert_eq!(h.len(), 2);
    assert!(!h.is_empty());
    assert_eq!(ron(&h.state_at(2)), ron(&now));
    let back = h.rewind_to(0).unwrap();
    assert_eq!(ron(&back), ron(&before_attack));
    assert_eq!(back, before_attack);
    assert!(h.is_empty());
    assert_eq!(h.charges_left(), 2);
}

#[test]
fn a_rewind_costs_one_charge_however_far_back() {
    let (mut h, _, _) = fought();
    h.rewind_to(1).unwrap();
    assert_eq!(h.charges_left(), 2);
    assert_eq!(h.commands().len(), 1);
    let (mut h, _, _) = fought();
    h.rewind_to(0).unwrap();
    assert_eq!(h.charges_left(), 2);
}

#[test]
fn doing_the_same_thing_after_a_rewind_gives_the_same_result() {
    let (mut h, _, _) = fought();
    let replayed = h.replay();
    let mut s = h.rewind_to(0).unwrap();
    let again = s.apply(&replayed[0].command).unwrap();
    assert_eq!(again, replayed[0].events);
    assert_eq!(s, replayed[1].before);
}

/// A bot planning on reseeded copies (ADR-0033) leaves the real battle's
/// luck alone: the real attack still gives what it gave before, and so
/// what a rewind repeats.
#[test]
fn the_real_battle_keeps_its_luck() {
    let (h, before_attack, _) = fought();
    let replayed = h.replay();
    let mut real = before_attack.clone();
    for seed in 0..16 {
        let mut copy = real.clone();
        copy.reseed_luck(seed);
        copy.apply(&replayed[0].command).unwrap();
    }
    assert_eq!(real, before_attack);
    assert_eq!(
        real.apply(&replayed[0].command).unwrap(),
        replayed[0].events
    );
}

#[test]
fn rewinds_are_refused_without_charges_or_a_point() {
    let (mut h, _, now) = fought();
    assert_eq!(h.rewind_to(2), Err(RewindError::NoSuchPoint(2)));
    for _ in 0..3 {
        h.push(Command::EndPhase);
        h.rewind_to(2).unwrap();
    }
    assert_eq!(h.charges_left(), 0);
    assert_eq!(h.rewind_to(0), Err(RewindError::NoCharges));
    // Nothing changed.
    assert_eq!(h.len(), 2);
    assert_eq!(h.state_at(2), now);
    assert_eq!(RewindError::NoCharges.to_string(), "no rewind charges left");
    assert_eq!(
        RewindError::NoSuchPoint(4).to_string(),
        "no earlier point 4 to rewind to"
    );
}

#[test]
fn replay_lists_each_command_with_the_state_before_it_and_its_events() {
    let (h, before_attack, _) = fought();
    let replayed = h.replay();
    assert_eq!(replayed.len(), 2);
    assert_eq!(replayed[0].before, before_attack);
    assert!(matches!(
        replayed[0].command,
        Command::Act {
            action: UnitAction::Attack { .. },
            ..
        }
    ));
    assert!(
        replayed[0]
            .events
            .iter()
            .any(|e| matches!(e, Event::CombatResolved { .. }))
    );
    assert_eq!(replayed[1].before, h.state_at(1));
    assert_eq!(replayed[1].command, Command::EndPhase);
}

#[test]
fn a_saved_history_needs_its_tables_back() {
    let (h, _, now) = fought();
    let text = ron::to_string(&h).unwrap();
    let mut loaded: BattleHistory = ron::from_str(&text).unwrap();
    assert_eq!(loaded.charges_left(), 3);
    assert_eq!(loaded.commands(), h.commands());
    let s = setup(cast());
    loaded.restore_tables(&crate::GameTables {
        terrain: s.terrain,
        classes: s.classes,
        items: s.items,
        spells: s.spells,
        skills: s.skills,
        arts: s.arts,
        supports: s.supports,
    });
    // Only the unit's pos differs from `fought`'s setup, not the tables.
    let replayed = loaded.state_at(2);
    assert_eq!(replayed.units(), now.units());
    assert_eq!(
        replayed.unit(UnitId(4)).map(|u| u.pos),
        Some(Pos::new(3, 2))
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_iters: 256,
        ..ProptestConfig::with_cases(128)
    })]

    #[test]
    fn state_at_n_equals_applying_the_first_n_commands(
        setup in arb_setup(),
        choices in prop::collection::vec(any::<u16>(), 0..60),
        pick in any::<prop::sample::Index>(),
    ) {
        let (mut s, _) = BattleState::new(setup);
        let mut h = BattleHistory::new(s.clone());
        let mut states = vec![s.clone()];
        for choice in choices {
            if s.outcome().is_some() {
                break;
            }
            let legal = legal_commands(&s);
            let cmd = legal[usize::from(choice) % legal.len()].clone();
            prop_assert!(s.apply(&cmd).is_ok());
            h.push(cmd);
            states.push(s.clone());
        }
        let n = pick.index(states.len());
        prop_assert_eq!(ron(&h.state_at(n)), ron(&states[n]));
        let replayed = h.replay();
        for (r, before) in replayed.iter().zip(&states) {
            prop_assert_eq!(&r.before, before);
        }
        if n < h.len() {
            let charges = h.charges_left();
            prop_assert_eq!(h.rewind_to(n).map(|s| ron(&s)), Ok(ron(&states[n])));
            prop_assert_eq!(h.len(), n);
            prop_assert_eq!(h.charges_left(), charges.saturating_sub(1));
        }
    }
}
