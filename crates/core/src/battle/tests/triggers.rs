//! Dialogue triggers and Talk (ticket 0705). Units here are named `c<id>`
//! ([`named`]); test units deal their weapon's might with every strike and
//! each side strikes once (see the parent module).

use super::*;
use crate::unit::CharacterId;

fn c(id: u32) -> CharacterId {
    CharacterId(format!("c{id}"))
}

/// `u` as the named character `c<id>`.
fn named(u: Unit) -> Unit {
    Unit {
        character: Some(c(u.id.0)),
        ..u
    }
}

/// [`cast`] with every unit named: lord 1 at (0,0), unit 2 at (0,2),
/// enemies 3 at (7,0) and 4 at (7,2).
fn cast_named() -> Vec<Unit> {
    cast().into_iter().map(named).collect()
}

/// Lord 1 at (0,0) and unit 2 at (0,2), enemies 3 at (2,0) and 4 at (2,2),
/// all named: the lord fights unit 3 from (1,0), unit 2 from (2,1).
fn near() -> Vec<Unit> {
    vec![
        named(lord(1, p(0, 0))),
        named(unit(2, Faction::Player, p(0, 2))),
        named(unit(3, Faction::Enemy, p(2, 0))),
        named(unit(4, Faction::Enemy, p(2, 2))),
    ]
}

fn trigger(when: TriggerWhen, scene: &str, once: bool) -> Trigger {
    Trigger {
        when,
        scene: scene.into(),
        once,
    }
}

fn scene(name: &str) -> Event {
    Event::SceneTriggered { scene: name.into() }
}

/// A battle of `units` with `triggers` (in Classic).
fn triggered(units: Vec<Unit>, triggers: Vec<Trigger>) -> BattleState {
    start(BattleSetup {
        triggers,
        ..setup(units)
    })
}

/// The scenes in `events`, in order.
fn scenes(events: &[Event]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::SceneTriggered { scene } => Some(scene.as_str()),
            _ => None,
        })
        .collect()
}

fn combat_start(unit: u32, against: Option<u32>, per_opponent: bool) -> TriggerWhen {
    TriggerWhen::CombatStart {
        unit: c(unit),
        against: against.map(c),
        per_opponent,
    }
}

fn talk(a: u32, b: u32, recruit: bool) -> TriggerWhen {
    TriggerWhen::Talk {
        a: c(a),
        b: c(b),
        recruit,
    }
}

// ---- Turn start ------------------------------------------------------------

#[test]
fn a_turn_start_scene_follows_its_phase_start_the_first_one_included() {
    let turn = |turn, phase| TriggerWhen::TurnStart { turn, phase };
    let setup = BattleSetup {
        triggers: vec![
            trigger(turn(1, Phase::Player), "opening", true),
            trigger(turn(2, Phase::Enemy), "enemy_2", true),
            trigger(turn(2, Phase::Enemy), "enemy_2b", false),
            trigger(turn(3, Phase::Player), "never", true),
        ],
        ..setup(cast_named())
    };
    let (mut s, events) = BattleState::new(setup);
    assert_eq!(events, [started(1, Phase::Player), scene("opening")]);
    assert_eq!(end(&mut s), [started(1, Phase::Enemy)]);
    assert_eq!(end(&mut s), [started(2, Phase::Player)]);
    // Two at once: in list order.
    assert_eq!(
        end(&mut s),
        [
            started(2, Phase::Enemy),
            scene("enemy_2"),
            scene("enemy_2b")
        ]
    );
    assert!(s.has_fired(1));
    assert!(!s.has_fired(2), "not once: never recorded");
    assert!(!s.has_fired(3));
}

// ---- Areas -----------------------------------------------------------------

#[test]
fn entering_an_area_fires_after_the_move_that_ends_in_it() {
    // (1,0), (2,0), (1,1) and (2,1).
    let area = TileRect {
        x: 1,
        y: 0,
        w: 2,
        h: 2,
    };
    let enters = |who, scene, once| trigger(TriggerWhen::UnitEntersArea { who, area }, scene, once);
    let mut s = triggered(
        cast_named(),
        vec![
            enters(Who::Character(c(1)), "lord_in", true),
            enters(Who::Faction(Faction::Player), "player_in", false),
            enters(Who::Faction(Faction::Enemy), "enemy_in", true),
        ],
    );
    // Passing through doesn't count.
    let events = act(&mut s, 1, p(3, 0), UnitAction::Wait);
    assert!(scenes(&events).is_empty(), "{events:?}");
    // Unit 2 ends in it: the player one, right after the move.
    let events = act(&mut s, 2, p(1, 1), UnitAction::Wait);
    assert_eq!(
        events[1..],
        [scene("player_in"), Event::UnitActed { unit: UnitId(2) }]
    );
    end(&mut s);
    end(&mut s);
    // The lord: its own, and the player one again (not once).
    let events = act(&mut s, 1, p(2, 0), UnitAction::Wait);
    assert_eq!(scenes(&events), ["lord_in", "player_in"]);
    // Staying put in it is no move: nothing.
    let events = act(&mut s, 2, p(1, 1), UnitAction::Wait);
    assert!(scenes(&events).is_empty());
    assert!(!s.has_fired(2));
}

#[test]
fn a_move_after_an_attack_can_enter_an_area() {
    let mut archer = armed(named(unit(1, Faction::Player, p(4, 3))), 1);
    archer.class = skill_class("swoop");
    let enemy = named(unit(3, Faction::Enemy, p(4, 2)));
    let area = TileRect {
        x: 0,
        y: 4,
        w: 8,
        h: 1,
    };
    let when = TriggerWhen::UnitEntersArea {
        who: Who::Character(c(1)),
        area,
    };
    let mut s = triggered(
        vec![lord(9, p(0, 0)), archer, enemy],
        vec![trigger(when, "row_4", true)],
    );
    let attack = UnitAction::Attack {
        target: UnitId(3),
        slot: 0,
        active: Some(SkillId::new("swoop")),
        art: None,
    };
    let events = act(&mut s, 1, p(4, 3), attack);
    assert!(scenes(&events).is_empty());
    let events = s
        .apply(&Command::MoveAfter {
            unit: UnitId(1),
            to: Some(p(4, 4)),
        })
        .unwrap();
    assert_eq!(scenes(&events), ["row_4"]);
}

// ---- Combat start ------------------------------------------------------------

#[test]
fn a_combat_start_scene_plays_before_the_combat_attacked_or_attacking() {
    let mut s = triggered(
        near(),
        vec![trigger(combat_start(3, None, false), "engage", true)],
    );
    let events = act(&mut s, 1, p(1, 0), attack(3));
    let at = events
        .iter()
        .position(|e| matches!(e, Event::CombatResolved { .. }))
        .unwrap();
    assert_eq!(events[at - 1], scene("engage"));
    assert_eq!(scenes(&events), ["engage"]);
    // Once: not when unit 3 attacks back in its phase, nor against unit 2.
    end(&mut s);
    let events = act(&mut s, 3, p(2, 0), attack(1));
    assert!(scenes(&events).is_empty());
}

#[test]
fn per_opponent_fires_once_against_each_unit() {
    let mut s = triggered(
        near(),
        vec![trigger(combat_start(3, None, true), "engage", true)],
    );
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["engage"]);
    assert_eq!(scenes(&act(&mut s, 2, p(2, 1), attack(3))), ["engage"]);
    end(&mut s);
    assert!(scenes(&act(&mut s, 3, p(2, 0), attack(1))).is_empty());
}

#[test]
fn a_repeating_combat_start_fires_every_combat() {
    let mut s = triggered(
        near(),
        vec![trigger(combat_start(1, None, false), "again", false)],
    );
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["again"]);
    end(&mut s);
    assert_eq!(scenes(&act(&mut s, 3, p(2, 0), attack(1))), ["again"]);
}

#[test]
fn a_line_for_one_opponent_replaces_the_default_for_that_pair() {
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, None, false), "default", true),
            trigger(combat_start(3, Some(1), false), "vs_lord", true),
        ],
    );
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["vs_lord"]);
    assert_eq!(scenes(&act(&mut s, 2, p(2, 1), attack(3))), ["default"]);
    // Both spent; the lord never gets the default either.
    end(&mut s);
    assert!(scenes(&act(&mut s, 3, p(2, 0), attack(1))).is_empty());
}

#[test]
fn both_sides_lines_play_the_attackers_first() {
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, None, false), "enemy_line", true),
            trigger(combat_start(1, None, false), "lord_line", true),
        ],
    );
    let events = act(&mut s, 1, p(1, 0), attack(3));
    assert_eq!(scenes(&events), ["lord_line", "enemy_line"]);
}

// ---- Falling -----------------------------------------------------------------

#[test]
fn a_death_quote_plays_just_before_the_fall() {
    let units = vec![
        armed(named(lord(1, p(0, 0))), 10),
        named(unit(2, Faction::Player, p(0, 2))),
        named(unit(3, Faction::Enemy, p(2, 0))),
        named(unit(4, Faction::Enemy, p(7, 2))),
    ];
    let fell = |unit| TriggerWhen::UnitFell {
        unit: c(unit),
        mode: None,
    };
    let mut s = triggered(units, vec![trigger(fell(3), "last_words", true)]);
    let events = act(&mut s, 1, p(1, 0), attack(3));
    let at = events
        .iter()
        .position(|e| *e == Event::UnitFell { unit: UnitId(3) })
        .unwrap();
    assert_eq!(events[at - 1], scene("last_words"));
    assert_eq!(scenes(&events), ["last_words"]);
}

#[test]
fn a_fallen_player_unit_plays_its_line_for_the_mode() {
    let lines = |mode| {
        let units = vec![
            named(lord(1, p(6, 0))),
            named(unit(2, Faction::Player, p(0, 2))),
            armed(named(unit(3, Faction::Enemy, p(7, 0))), 10),
        ];
        let fell = |m| TriggerWhen::UnitFell {
            unit: c(1),
            mode: Some(m),
        };
        let mut s = start(BattleSetup {
            triggers: vec![
                trigger(fell(GameMode::Classic), "death", true),
                trigger(fell(GameMode::Casual), "retreat", true),
            ],
            mode,
            ..setup(units)
        });
        assert_eq!(s.mode(), mode);
        end(&mut s);
        let events = act(&mut s, 3, p(7, 0), attack(1));
        // The line, the fall, then the defeat (the lord fell).
        let tail = &events[events.len() - 4..];
        assert_eq!(tail[1], Event::UnitFell { unit: UnitId(1) });
        assert_eq!(tail[3], ended(Outcome::Defeat));
        match &tail[0] {
            Event::SceneTriggered { scene } => scene.clone(),
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(lines(GameMode::Classic), "death");
    assert_eq!(lines(GameMode::Casual), "retreat");
    assert_eq!(GameMode::default(), GameMode::Classic);
}

// ---- Talk ----------------------------------------------------------------------

/// Lord 1 at (0,0), unit 2 at (0,2); enemy 3 at (2,0) (in reach), enemy 4
/// at (7,2). The lord can recruit unit 3; unit 2 can talk to the lord.
fn talkers() -> BattleState {
    let mut units = cast_named();
    units[2].pos = p(2, 0);
    triggered(
        units,
        vec![
            trigger(talk(1, 3, true), "recruit", true),
            trigger(talk(2, 1, false), "chat", false),
        ],
    )
}

fn talk_to(target: u32) -> UnitAction {
    UnitAction::Talk {
        target: UnitId(target),
    }
}

#[test]
fn talk_targets_are_adjacent_units_with_an_unfired_talk() {
    let s = talkers();
    assert_eq!(s.talk_targets(UnitId(1), p(1, 0)), [UnitId(3)]);
    assert_eq!(s.talk_targets(UnitId(1), p(2, 1)), [UnitId(3)]);
    // Too far, or the other way round.
    assert!(s.talk_targets(UnitId(1), p(0, 1)).is_empty());
    assert!(s.talk_targets(UnitId(3), p(1, 0)).is_empty());
    assert_eq!(s.talk_targets(UnitId(2), p(0, 1)), [UnitId(1)]);
    assert!(s.talk_targets(UnitId(9), p(0, 1)).is_empty());
}

#[test]
fn talk_is_refused_without_a_trigger_or_out_of_reach() {
    let mut s = talkers();
    refused_act(
        &mut s,
        1,
        p(0, 1),
        talk_to(2),
        CommandError::CannotTalk(UnitId(2)),
    );
    refused_act(
        &mut s,
        1,
        p(0, 1),
        talk_to(3),
        CommandError::OutOfRange {
            target: UnitId(3),
            distance: 3,
        },
    );
    refused_act(
        &mut s,
        1,
        p(1, 0),
        talk_to(1),
        CommandError::CannotTalk(UnitId(1)),
    );
    refused_act(
        &mut s,
        1,
        p(1, 0),
        talk_to(8),
        CommandError::UnknownUnit(UnitId(8)),
    );
    assert_eq!(
        CommandError::CannotTalk(UnitId(2)).to_string(),
        "nothing to say to unit 2"
    );
}

#[test]
fn talk_recruits_and_the_recruit_acts_next_player_phase() {
    let mut s = talkers();
    let events = act(&mut s, 1, p(1, 0), talk_to(3));
    assert_eq!(
        events,
        [
            Event::UnitMoved {
                unit: UnitId(1),
                path: vec![p(0, 0), p(1, 0)],
            },
            scene("recruit"),
            Event::UnitRecruited { unit: UnitId(3) },
            Event::UnitActed { unit: UnitId(1) },
        ]
    );
    let recruit = s.unit(UnitId(3)).unwrap();
    assert_eq!((recruit.faction, recruit.acted), (Faction::Player, true));
    assert!(s.has_fired(0));
    // Once: nothing more to say.
    assert!(s.talk_targets(UnitId(1), p(1, 0)).is_empty());
    // Not this phase…
    refused_act(
        &mut s,
        3,
        p(3, 0),
        UnitAction::Wait,
        CommandError::AlreadyActed(UnitId(3)),
    );
    // …but the next player phase, as a player unit.
    end(&mut s);
    let events = end(&mut s);
    assert_eq!(events, [started(2, Phase::Player)]);
    act(&mut s, 3, p(3, 0), UnitAction::Wait);
    assert_eq!(s.outcome(), None, "enemy 4 is still there");
}

#[test]
fn talk_without_recruiting_ends_the_action_and_may_repeat() {
    let mut s = talkers();
    let events = act(&mut s, 2, p(0, 1), talk_to(1));
    assert_eq!(scenes(&events), ["chat"]);
    assert_eq!(s.unit(UnitId(1)).unwrap().faction, Faction::Player);
    assert!(s.unit(UnitId(2)).unwrap().acted);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::UnitRecruited { .. } | Event::ExpGained { .. }))
    );
    assert!(!s.has_fired(1));
    end(&mut s);
    end(&mut s);
    assert_eq!(scenes(&act(&mut s, 2, p(0, 1), talk_to(1))), ["chat"]);
}

#[test]
fn recruiting_the_last_enemy_wins_a_rout() {
    let units = vec![
        named(lord(1, p(0, 0))),
        named(unit(3, Faction::Enemy, p(2, 0))),
    ];
    let mut s = triggered(units, vec![trigger(talk(1, 3, true), "recruit", true)]);
    let events = act(&mut s, 1, p(1, 0), talk_to(3));
    assert_eq!(events.last(), Some(&ended(Outcome::Victory)));
}

// ---- Saving ----------------------------------------------------------------------

#[test]
fn fired_triggers_survive_a_save_and_load() {
    let mut s = talkers();
    act(&mut s, 1, p(1, 0), talk_to(3));
    let text = ron::to_string(&s).unwrap();
    let mut loaded: BattleState = ron::from_str(&text).unwrap();
    let mut units = cast_named();
    units[2].pos = p(2, 0);
    loaded.restore_tables(
        Arc::new(terrain()),
        Arc::new(classes()),
        Arc::new(items_for(&units)),
        Arc::new(spells()),
        Arc::new(skills()),
        Arc::new(test_arts()),
    );
    assert_eq!(loaded, s);
    assert!(loaded.has_fired(0));
    assert_eq!(loaded.triggers(), s.triggers());
    // Once fired, still fired: the lord has nothing more to say.
    end(&mut loaded);
    end(&mut loaded);
    assert!(loaded.talk_targets(UnitId(1), p(1, 0)).is_empty());
}

#[test]
fn triggers_read_from_ron_as_the_module_docs_write_them() {
    let text = r#"[
        (when: TurnStart(turn: 3, phase: Player), scene: "ch01_hint", once: true),
        (when: UnitEntersArea(who: Faction(Player), area: (x: 10, y: 5, w: 2, h: 2)), scene: "ch01_fort", once: true),
        (when: CombatStart(unit: "harl", against: Some("ana")), scene: "ch01_harl_ana", once: true),
        (when: CombatStart(unit: "harl"), scene: "ch01_harl", once: true),
        (when: UnitFell(unit: "harl"), scene: "ch01_harl_death", once: true),
        (when: UnitFell(unit: "tamsin", mode: Some(Casual)), scene: "ch01_tamsin_retreat", once: true),
        (when: Talk(a: "ana", b: "rook", recruit: true), scene: "ch02_rook_joins", once: true),
    ]"#;
    let triggers: Vec<Trigger> = ron::from_str(text).unwrap();
    let ch = |s: &str| CharacterId(s.into());
    assert_eq!(triggers.len(), 7);
    assert_eq!(
        triggers[3].when,
        TriggerWhen::CombatStart {
            unit: ch("harl"),
            against: None,
            per_opponent: false,
        }
    );
    assert_eq!(
        triggers[1].when,
        TriggerWhen::UnitEntersArea {
            who: Who::Faction(Faction::Player),
            area: TileRect {
                x: 10,
                y: 5,
                w: 2,
                h: 2
            },
        }
    );
    assert_eq!(
        triggers[6],
        trigger(talk_between(ch("ana"), ch("rook")), "ch02_rook_joins", true)
    );
}

fn talk_between(a: CharacterId, b: CharacterId) -> TriggerWhen {
    TriggerWhen::Talk {
        a,
        b,
        recruit: true,
    }
}

#[test]
fn a_tile_rect_holds_its_tiles_only() {
    let r = TileRect {
        x: 1,
        y: 2,
        w: 2,
        h: 1,
    };
    assert!(r.contains(p(1, 2)) && r.contains(p(2, 2)));
    for out in [p(0, 2), p(3, 2), p(1, 1), p(1, 3)] {
        assert!(!r.contains(out), "{out:?}");
    }
}
