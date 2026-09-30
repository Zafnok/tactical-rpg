//! Tests of the playtest bots' support in `core` (ticket 0504): the list of
//! legal commands ([`legal_commands`]), [`BattleState::check`] and luck
//! reseeding ([`BattleState::reseed_luck`]).

use std::collections::BTreeSet;

use super::*;
use crate::ai::{AiWeights, next_command};

/// Plays `choices` from `setup`, each picking from the legal commands,
/// until the battle ends or the choices run out. Before each pick, `each`
/// sees the state and its legal commands.
fn random_play(
    setup: BattleSetup,
    choices: &[u16],
    mut each: impl FnMut(&BattleState, &[Command]) -> Result<(), TestCaseError>,
) -> Result<BattleState, TestCaseError> {
    let (mut s, _) = BattleState::new(setup);
    for &choice in choices {
        let legal = legal_commands(&s);
        each(&s, &legal)?;
        if s.outcome().is_some() {
            prop_assert!(legal.is_empty(), "{:?}", legal);
            break;
        }
        let cmd = &legal[usize::from(choice) % legal.len()];
        prop_assert_eq!(s.check(cmd), Ok(()));
        let applied = s.apply(cmd);
        prop_assert!(applied.is_ok(), "{:?} refused: {:?}", cmd, applied);
    }
    Ok(s)
}

/// Every `Act` action worth trying for ready `unit` on any tile, far more
/// than [`legal_commands`] tries: every unit as a target, every slot, every
/// skill and art the unit has in every role, every spell cast on every unit
/// and tile, every item, and single shop transactions.
fn every_action(s: &BattleState, unit: &Unit) -> Vec<UnitAction> {
    let skills: Vec<Option<SkillId>> = std::iter::once(None)
        .chain(
            unit.usable_skills(s.classes(), s.skills())
                .iter()
                .map(|d| Some(d.id.clone())),
        )
        .collect();
    let arts: BTreeSet<ArtId> = (0..WEAPON_SLOTS)
        .flat_map(|slot| unit.arts_for(slot, s.classes(), s.items(), s.arts()))
        .map(|a| a.id.clone())
        .collect();
    let arts: Vec<Option<ArtId>> = std::iter::once(None)
        .chain(arts.into_iter().map(Some))
        .collect();
    let targets: Vec<UnitId> = s.units().iter().map(|u| u.id).collect();
    let mut out = vec![UnitAction::Wait, UnitAction::Seize, UnitAction::Open];
    for &target in &targets {
        for slot in 0..=WEAPON_SLOTS {
            for active in &skills {
                for art in &arts {
                    out.push(UnitAction::Attack {
                        target,
                        slot,
                        active: active.clone(),
                        art: art.clone(),
                    });
                }
            }
        }
        for spell in s.spells().spells.keys() {
            for active in &skills {
                out.push(UnitAction::Cast {
                    spell: spell.clone(),
                    target: CastTarget::Unit(target),
                    active: active.clone(),
                });
            }
        }
        for pack_index in 0..=s.pack().items.len() {
            out.push(UnitAction::UseItem { pack_index, target });
        }
    }
    for spell in &unit.learned {
        for pos in s.map().tiles.positions() {
            out.push(UnitAction::Cast {
                spell: spell.clone(),
                target: CastTarget::Tile(pos),
                active: None,
            });
        }
    }
    for skill in skills.iter().flatten() {
        let on = std::iter::once(None).chain(targets.iter().copied().map(Some));
        out.extend(on.map(|target| UnitAction::UseSkill {
            skill: skill.clone(),
            target,
        }));
    }
    let mut txns: Vec<ShopTxn> = s
        .items()
        .items
        .keys()
        .map(|item| ShopTxn::Buy { item: item.clone() })
        .collect();
    let sells = (0..=WEAPON_SLOTS)
        .map(SellFrom::Weapon)
        .chain([SellFrom::Armour, SellFrom::Accessory])
        .chain((0..=s.pack().items.len()).map(SellFrom::Pack));
    txns.extend(sells.map(|from| ShopTxn::Sell { from }));
    txns.extend((0..=WEAPON_SLOTS).map(|slot| ShopTxn::Repair { slot }));
    out.extend(txns.into_iter().map(|t| UnitAction::Shop { txns: vec![t] }));
    out
}

/// Every command in a wide search `s` accepts: every `Act` of
/// [`every_action`] on every tile each unit can stop on, every equip,
/// talk and move after, and ending the phase (but for the multi-transaction
/// shop visits). A unit is ready if it may wait where it stands.
fn brute_force(s: &BattleState) -> BTreeSet<String> {
    let mut cmds = vec![Command::EndPhase];
    let tiles: Vec<Pos> = s.map().tiles.positions().collect();
    let first = s.units().first().map(|u| u.id);
    for to in std::iter::once(None).chain(tiles.iter().copied().map(Some)) {
        let unit = s.pending_move().map(|m| m.unit).or(first);
        cmds.extend(unit.map(|unit| Command::MoveAfter { unit, to }));
    }
    for u in s.units() {
        let equips = (0..=WEAPON_SLOTS)
            .map(Equipped::Weapon)
            .chain(s.spells().spells.keys().cloned().map(Equipped::Spell));
        cmds.extend(equips.map(|equipped| Command::Equip {
            unit: u.id,
            equipped,
        }));
        for &dest in &tiles {
            for t in s.units() {
                cmds.push(Command::Talk {
                    unit: u.id,
                    dest,
                    target: t.id,
                });
            }
        }
    }
    let mut accepted: BTreeSet<String> = cmds
        .into_iter()
        .filter(|c| s.check(c).is_ok())
        .map(|c| format!("{c:?}"))
        .collect();
    let wait_here = |u: &Unit| Command::Act {
        unit: u.id,
        dest: u.pos,
        action: UnitAction::Wait,
    };
    for u in s.units().iter().filter(|u| s.check(&wait_here(u)).is_ok()) {
        let reach = reachable(s.map(), s.terrain(), s.classes(), s.units(), u.id).unwrap();
        let actions = every_action(s, u);
        for dest in reach.stoppable().iter() {
            let path = reach.path_to(dest).unwrap();
            for action in &actions {
                if s.check_action(u, dest, &path, action).is_ok() {
                    let cmd = Command::Act {
                        unit: u.id,
                        dest,
                        action: action.clone(),
                    };
                    accepted.insert(format!("{cmd:?}"));
                }
            }
        }
    }
    accepted
}

/// `cmds` as a set, without the listed shop visits of two transactions.
fn single_visits(cmds: &[Command]) -> BTreeSet<String> {
    cmds.iter()
        .filter(|c| {
            !matches!(c, Command::Act { action: UnitAction::Shop { txns }, .. } if txns.len() > 1)
        })
        .map(|c| format!("{c:?}"))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_iters: 256,
        ..ProptestConfig::with_cases(128)
    })]

    #[test]
    fn every_legal_command_is_accepted(
        setup in arb_setup(),
        choices in prop::collection::vec(any::<u16>(), 200),
    ) {
        random_play(setup, &choices, |s, legal| {
            // Ending the phase is always legal, and first, unless the
            // battle is over or a unit waits to move after its attack.
            let free = s.outcome().is_none() && s.pending_move().is_none();
            prop_assert_eq!(free, legal.first() == Some(&Command::EndPhase));
            for cmd in legal {
                prop_assert_eq!(s.check(cmd), Ok(()), "{:?}", cmd);
            }
            Ok(())
        })?;
    }

    #[test]
    fn ai_commands_are_legal(
        setup in arb_setup(),
        choices in prop::collection::vec(any::<u16>(), 0..120),
    ) {
        random_play(setup, &choices, |s, legal| {
            if let Some(cmd) = next_command(s, &AiWeights::STARTING) {
                prop_assert!(legal.contains(&cmd), "{:?} not in {:?}", cmd, legal);
            }
            Ok(())
        })?;
    }
}

proptest! {
    // Each case searches far wider than `legal_commands` does.
    #![proptest_config(ProptestConfig {
        max_shrink_iters: 64,
        ..ProptestConfig::with_cases(48)
    })]

    #[test]
    fn legal_commands_are_every_accepted_command(
        setup in arb_setup(),
        choices in prop::collection::vec(any::<u16>(), 0..40),
    ) {
        let s = random_play(setup, &choices, |_, _| Ok(()))?;
        let legal = legal_commands(&s);
        // The one visit of two transactions listed: two of the shop's
        // first item, wherever a unit may buy one.
        for cmd in &legal {
            if let Command::Act { unit, dest, action: UnitAction::Shop { txns } } = cmd
                && let [ShopTxn::Buy { item }] = txns.as_slice()
                && s.map().shop(*dest).and_then(|shop| shop.stock.first()) == Some(item)
            {
                let two = Command::Act {
                    unit: *unit,
                    dest: *dest,
                    action: UnitAction::Shop { txns: vec![txns[0].clone(); 2] },
                };
                prop_assert_eq!(legal.contains(&two), s.check(&two).is_ok(), "{:?}", two);
            }
        }
        let listed = single_visits(&legal);
        let unique: BTreeSet<String> = legal.iter().map(|c| format!("{c:?}")).collect();
        prop_assert_eq!(unique.len(), legal.len(), "a command is listed twice");
        prop_assert_eq!(listed, brute_force(&s));
    }
}

/// The acts in `cmds` of unit `id` with `action`, as `(dest, action)`.
fn acts_of(cmds: &[Command], id: u32) -> Vec<(Pos, &UnitAction)> {
    cmds.iter()
        .filter_map(|c| match c {
            Command::Act { unit, dest, action } if *unit == UnitId(id) => Some((*dest, action)),
            _ => None,
        })
        .collect()
}

#[test]
fn legal_commands_is_deterministic() {
    let near = || {
        let mut units = cast();
        units[3].pos = p(3, 2);
        start(setup(units))
    };
    let (a, b) = (near(), near());
    assert_eq!(a, b);
    let listed = legal_commands(&a);
    assert!(
        acts_of(&listed, 2)
            .iter()
            .any(|(_, action)| matches!(action, UnitAction::Attack { .. }))
    );
    assert_eq!(listed, legal_commands(&b));
    assert_eq!(listed, legal_commands(&a));
}

#[test]
fn nothing_is_legal_once_the_battle_is_over() {
    let mut s = start(setup(cast()));
    s.outcome = Some(Outcome::Victory);
    assert!(legal_commands(&s).is_empty());
}

#[test]
fn only_the_move_after_an_attack_is_legal_while_it_waits() {
    let mut s = start(setup(cast()));
    s.pending_move = Some(PendingMove {
        unit: UnitId(2),
        tiles: 1,
    });
    let to = |to| Command::MoveAfter {
        unit: UnitId(2),
        to,
    };
    assert_eq!(
        legal_commands(&s),
        [
            to(None),
            to(Some(p(1, 2))),
            to(Some(p(0, 3))),
            to(Some(p(0, 1)))
        ]
    );
}

#[test]
fn an_attack_is_listed_as_far_as_a_range_bonus_reaches() {
    // Archer 1 (a 1–2 bow, Long Shot: +1 range with bows) and enemy 3 at
    // (7,0): from (4,0), 3 tiles away, only Long Shot reaches.
    let archer = Unit {
        class: skill_class("long_shot"),
        ..carrying(lord(1, p(1, 0)), &[item("flier_bow")])
    };
    let s = start(setup(vec![archer, unit(3, Faction::Enemy, p(7, 0))]));
    let from = |dest| {
        acts_of(&legal_commands(&s), 1)
            .into_iter()
            .filter(|&(d, _)| d == dest)
            .filter_map(|(_, action)| match action {
                UnitAction::Attack { active, .. } => Some(active.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(from(p(4, 0)), [Some(SkillId::new("long_shot"))]);
    assert_eq!(from(p(3, 0)), []);
}

/// Unit 1 (hit 50, might 1) next to enemy 3: a coin flip for each strike.
fn coin_flip() -> BattleState {
    let attacker = carrying(lord(1, p(0, 0)), &[weapon_id(1, 1, 1, 50, 0)]);
    start(setup(vec![attacker, unit(3, Faction::Enemy, p(1, 0))]))
}

fn attack_3() -> Command {
    Command::Act {
        unit: UnitId(1),
        dest: p(0, 0),
        action: attack(3),
    }
}

/// Whether unit 1's first strike hit, in `events`.
fn first_strike_hit(events: &[Event]) -> bool {
    events
        .iter()
        .find_map(|e| match e {
            Event::CombatResolved {
                forecast, outcome, ..
            } => {
                assert_eq!(forecast.attacker.hit, 50);
                outcome.strikes.first().map(|x| x.hit)
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn reseeding_changes_luck_but_not_the_board() {
    let s = coin_flip();
    let reseeded = |seed| {
        let mut copy = s.clone();
        copy.reseed_luck(seed);
        copy
    };
    let (a, b) = (reseeded(1), reseeded(2));
    assert_ne!(a.rng, b.rng);
    assert_ne!(a.rng, s.rng);
    // Only the luck differs: map, units, turn and everything else match.
    for copy in [a, b] {
        assert_eq!(
            BattleState {
                rng: s.rng.clone(),
                ..copy
            },
            s
        );
    }
    let hits: BTreeSet<bool> = (0..32)
        .map(|seed| first_strike_hit(&reseeded(seed).apply(&attack_3()).unwrap()))
        .collect();
    assert_eq!(hits, BTreeSet::from([false, true]));
    for seed in 0..8 {
        let once = reseeded(seed).apply(&attack_3()).unwrap();
        assert_eq!(reseeded(seed).apply(&attack_3()).unwrap(), once);
    }
}

#[test]
fn check_agrees_with_apply_and_changes_nothing() {
    let s = coin_flip();
    let before = s.clone();
    assert_eq!(s.check(&attack_3()), Ok(()));
    assert_eq!(s.check(&Command::EndPhase), Ok(()));
    let far = Command::Act {
        unit: UnitId(1),
        dest: p(7, 4),
        action: UnitAction::Wait,
    };
    assert_eq!(s.check(&far), Err(CommandError::CannotStop(p(7, 4))));
    assert_eq!(s, before);
}
