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

fn combat_start(unit: u32, against: Option<u32>) -> TriggerWhen {
    TriggerWhen::CombatStart {
        unit: c(unit),
        against: against.map(c),
    }
}

fn talk(a: u32, b: u32) -> TriggerWhen {
    TriggerWhen::Talk { a: c(a), b: c(b) }
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
    let mut s = triggered(near(), vec![trigger(combat_start(3, None), "engage", true)]);
    let events = act(&mut s, 1, p(1, 0), attack(3));
    let at = events
        .iter()
        .position(|e| matches!(e, Event::CombatResolved { .. }))
        .unwrap();
    assert_eq!(events[at - 1], scene("engage"));
    assert_eq!(scenes(&events), ["engage"]);
    // Once per battle: not against unit 2, nor when unit 3 attacks back.
    assert!(scenes(&act(&mut s, 2, p(2, 1), attack(3))).is_empty());
    end(&mut s);
    let events = act(&mut s, 3, p(2, 0), attack(1));
    assert!(scenes(&events).is_empty());
}

#[test]
fn a_repeating_combat_start_fires_every_combat() {
    let mut s = triggered(near(), vec![trigger(combat_start(1, None), "again", false)]);
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["again"]);
    end(&mut s);
    assert_eq!(scenes(&act(&mut s, 3, p(2, 0), attack(1))), ["again"]);
}

#[test]
fn a_line_for_one_opponent_replaces_the_default_for_that_pair() {
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, None), "default", true),
            trigger(combat_start(3, Some(1)), "vs_lord", true),
        ],
    );
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["vs_lord"]);
    assert_eq!(scenes(&act(&mut s, 2, p(2, 1), attack(3))), ["default"]);
    // Both spent; the lord never gets the default either.
    end(&mut s);
    assert!(scenes(&act(&mut s, 3, p(2, 0), attack(1))).is_empty());
}

#[test]
fn the_default_line_never_plays_to_a_character_with_its_own_line() {
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, None), "default", true),
            trigger(combat_start(3, Some(1)), "vs_lord", true),
        ],
    );
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["vs_lord"]);
    end(&mut s);
    // The lord again: its line has played, and the default stays unplayed.
    assert!(scenes(&act(&mut s, 3, p(2, 0), attack(1))).is_empty());
    assert!(!s.has_fired(0));
}

#[test]
fn a_fight_plays_one_scene_the_first_listed() {
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, None), "enemy_line", true),
            trigger(combat_start(1, None), "lord_line", true),
        ],
    );
    let events = act(&mut s, 1, p(1, 0), attack(3));
    assert_eq!(scenes(&events), ["enemy_line"]);
    // The lord's plays in its next fight.
    end(&mut s);
    assert_eq!(scenes(&act(&mut s, 3, p(2, 0), attack(1))), ["lord_line"]);
}

#[test]
fn only_the_fighters_own_pair_scene_plays() {
    // Unit 3 has scenes with unit 2 (listed first) and with the lord: the
    // lord's fight plays the lord's.
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, Some(2)), "3_and_2", true),
            trigger(combat_start(3, Some(1)), "3_and_lord", true),
        ],
    );
    assert_eq!(scenes(&act(&mut s, 1, p(1, 0), attack(3))), ["3_and_lord"]);
    assert_eq!(scenes(&act(&mut s, 2, p(2, 1), attack(3))), ["3_and_2"]);
}

#[test]
fn a_pairs_scene_plays_whoever_attacks_and_whoever_lists_it() {
    // Written as the lord's line against unit 3: unit 3 attacking the lord
    // plays it too, and not unit 3's general line.
    let mut s = triggered(
        near(),
        vec![
            trigger(combat_start(3, None), "general", true),
            trigger(combat_start(1, Some(3)), "lord_and_3", true),
        ],
    );
    act(&mut s, 1, p(0, 0), UnitAction::Wait);
    act(&mut s, 2, p(0, 2), UnitAction::Wait);
    end(&mut s);
    let events = act(&mut s, 3, p(1, 0), attack(1));
    assert_eq!(scenes(&events), ["lord_and_3"]);
    // A fight where neither fighter has a line plays nothing.
    let events = act(&mut s, 4, p(1, 2), attack(2));
    assert!(scenes(&events).is_empty(), "unit 4 has no line");
}

// ---- Half HP -------------------------------------------------------------------

#[test]
fn the_half_hp_line_plays_after_the_combat_that_brings_it_to_half() {
    // Unit 3 has 10 HP; the lord deals 3 a strike, it strikes back for 3.
    let half = |unit| TriggerWhen::HalfHp { unit: c(unit) };
    let mut s = triggered(
        near(),
        vec![
            trigger(half(3), "half", true),
            trigger(half(1), "lord_half", true),
        ],
    );
    // 10 → 7: nothing.
    assert!(scenes(&act(&mut s, 1, p(1, 0), attack(3))).is_empty());
    // 7 → 4: right after the combat.
    let events = act(&mut s, 2, p(2, 1), attack(3));
    let at = events
        .iter()
        .position(|e| matches!(e, Event::CombatResolved { .. }))
        .unwrap();
    assert_eq!(events[at + 1], scene("half"));
    assert_eq!(scenes(&events), ["half"]);
    // Once: 4 → 1 plays nothing; the lord (10 → 4 by now) gets its own.
    end(&mut s);
    let events = act(&mut s, 3, p(2, 0), attack(1));
    assert_eq!(scenes(&events), ["lord_half"]);
}

#[test]
fn a_blow_that_defeats_plays_no_half_hp_line() {
    let units = vec![
        armed(named(lord(1, p(0, 0))), 10),
        named(unit(3, Faction::Enemy, p(2, 0))),
        named(unit(4, Faction::Enemy, p(7, 2))),
    ];
    let half = TriggerWhen::HalfHp { unit: c(3) };
    let mut s = triggered(units, vec![trigger(half, "half", true)]);
    assert!(scenes(&act(&mut s, 1, p(1, 0), attack(3))).is_empty());
}

// ---- Falling -----------------------------------------------------------------

fn fell(unit: u32, mode: Option<GameMode>, recruit: bool) -> TriggerWhen {
    TriggerWhen::UnitFell {
        unit: c(unit),
        mode,
        recruit,
    }
}

/// The lord (might 10: fells in one blow) at (0,0), unit 2 at (0,2),
/// enemies 3 at (2,0) and 4 at (7,2).
fn strong() -> Vec<Unit> {
    vec![
        armed(named(lord(1, p(0, 0))), 10),
        named(unit(2, Faction::Player, p(0, 2))),
        named(unit(3, Faction::Enemy, p(2, 0))),
        named(unit(4, Faction::Enemy, p(7, 2))),
    ]
}

#[test]
fn a_death_quote_plays_just_before_the_fall() {
    let mut s = triggered(
        strong(),
        vec![trigger(fell(3, None, false), "last_words", true)],
    );
    let events = act(&mut s, 1, p(1, 0), attack(3));
    let at = events
        .iter()
        .position(|e| *e == Event::UnitFell { unit: UnitId(3) })
        .unwrap();
    assert_eq!(events[at - 1], scene("last_words"));
    assert_eq!(scenes(&events), ["last_words"]);
    assert!(s.recruited().is_empty());
}

#[test]
fn an_enemy_that_joins_if_defeated_is_recruited_as_it_falls() {
    let mut s = triggered(strong(), vec![trigger(fell(3, None, true), "yields", true)]);
    let events = act(&mut s, 1, p(1, 0), attack(3));
    let at = events
        .iter()
        .position(|e| *e == Event::UnitFell { unit: UnitId(3) })
        .unwrap();
    assert_eq!(events[at - 1], scene("yields"));
    assert_eq!(events[at + 1], Event::UnitRecruited { unit: UnitId(3) });
    let ids: Vec<UnitId> = s.recruited().iter().map(|u| u.id).collect();
    assert_eq!(ids, [UnitId(3)]);
    // It fell: off the map, among the fallen too.
    assert!(s.unit(UnitId(3)).is_none());
    assert!(s.fallen().iter().any(|u| u.id == UnitId(3)));
}

#[test]
fn a_fallen_player_unit_plays_its_line_for_the_mode() {
    let lines = |mode| {
        let units = vec![
            named(lord(1, p(6, 0))),
            named(unit(2, Faction::Player, p(0, 2))),
            armed(named(unit(3, Faction::Enemy, p(7, 0))), 10),
        ];
        let mut s = start(BattleSetup {
            triggers: vec![
                trigger(fell(1, Some(GameMode::Classic), false), "death", true),
                trigger(fell(1, Some(GameMode::Casual), false), "retreat", true),
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
/// at (7,2). The lord and unit 3 can talk once; units 1 and 2 can chat any
/// time (listed as 2 and 1).
fn talkers() -> BattleState {
    let mut units = cast_named();
    units[2].pos = p(2, 0);
    triggered(
        units,
        vec![
            trigger(talk(1, 3), "parley", true),
            trigger(talk(2, 1), "chat", false),
        ],
    )
}

/// Unit `unit` talking to `target` from `dest`.
fn talk_cmd(unit: u32, dest: Pos, target: u32) -> Command {
    Command::Talk {
        unit: UnitId(unit),
        dest,
        target: UnitId(target),
    }
}

#[test]
fn talk_targets_are_adjacent_units_with_an_unfired_talk_either_way() {
    let s = talkers();
    assert_eq!(s.talk_targets(UnitId(1), p(1, 0)), [UnitId(3)]);
    assert_eq!(s.talk_targets(UnitId(1), p(2, 1)), [UnitId(3)]);
    // Either one may start it.
    assert_eq!(s.talk_targets(UnitId(3), p(1, 0)), [UnitId(1)]);
    assert_eq!(s.talk_targets(UnitId(2), p(0, 1)), [UnitId(1)]);
    assert_eq!(s.talk_targets(UnitId(1), p(0, 1)), [UnitId(2)]);
    // Too far, or nobody.
    assert!(s.talk_targets(UnitId(4), p(6, 2)).is_empty());
    assert!(s.talk_targets(UnitId(9), p(0, 1)).is_empty());
}

#[test]
fn talking_is_free_the_unit_stays_ready_where_it_was() {
    let mut s = talkers();
    let events = s.apply(&talk_cmd(1, p(1, 0), 3)).unwrap();
    assert_eq!(events, [scene("parley")]);
    let lord = s.unit(UnitId(1)).unwrap();
    assert_eq!((lord.pos, lord.acted), (p(0, 0), false));
    assert!(s.has_fired(0));
    // Nothing changes sides; the enemy is still an enemy on the map.
    assert_eq!(s.unit(UnitId(3)).unwrap().faction, Faction::Enemy);
    assert!(s.recruited().is_empty());
    // Once: nothing more to say to it.
    assert!(s.talk_targets(UnitId(1), p(1, 0)).is_empty());
    // The lord still moves and acts.
    let events = act(&mut s, 1, p(1, 0), attack(3));
    assert!(events.contains(&Event::UnitActed { unit: UnitId(1) }));
}

#[test]
fn a_talk_that_isnt_once_may_repeat() {
    let mut s = talkers();
    assert_eq!(s.apply(&talk_cmd(2, p(0, 1), 1)).unwrap(), [scene("chat")]);
    assert_eq!(s.apply(&talk_cmd(1, p(0, 1), 2)).unwrap(), [scene("chat")]);
    assert!(!s.has_fired(1));
    assert!(!events_gain_exp(
        &s.apply(&talk_cmd(2, p(0, 1), 1)).unwrap()
    ));
}

fn events_gain_exp(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::ExpGained { .. }))
}

#[test]
fn talk_is_refused_without_a_trigger_out_of_reach_or_not_ready() {
    let mut s = talkers();
    refused(
        &mut s,
        &talk_cmd(2, p(1, 2), 4),
        CommandError::CannotTalk(UnitId(4)),
    );
    refused(
        &mut s,
        &talk_cmd(1, p(0, 1), 3),
        CommandError::OutOfRange {
            target: UnitId(3),
            distance: 3,
        },
    );
    refused(
        &mut s,
        &talk_cmd(1, p(1, 0), 1),
        CommandError::CannotTalk(UnitId(1)),
    );
    refused(
        &mut s,
        &talk_cmd(1, p(1, 0), 8),
        CommandError::UnknownUnit(UnitId(8)),
    );
    // A tile it can't stop on (unit 2's), or can't reach.
    refused(
        &mut s,
        &talk_cmd(1, p(0, 2), 2),
        CommandError::CannotStop(p(0, 2)),
    );
    refused(
        &mut s,
        &talk_cmd(1, p(6, 0), 3),
        CommandError::CannotStop(p(6, 0)),
    );
    // Not its phase, or already acted.
    refused(
        &mut s,
        &talk_cmd(3, p(1, 0), 1),
        CommandError::NotItsPhase {
            unit: UnitId(3),
            phase: Phase::Player,
        },
    );
    act(&mut s, 2, p(0, 2), UnitAction::Wait);
    refused(
        &mut s,
        &talk_cmd(2, p(0, 1), 1),
        CommandError::AlreadyActed(UnitId(2)),
    );
    assert_eq!(
        CommandError::CannotTalk(UnitId(2)).to_string(),
        "nothing to say to unit 2"
    );
}

// ---- Recruiting --------------------------------------------------------------------

#[test]
fn recruits_survive_a_save_and_load() {
    let mut s = triggered(
        strong(),
        vec![
            trigger(fell(3, None, true), "yields", true),
            trigger(talk(1, 2), "chat", true),
        ],
    );
    s.apply(&talk_cmd(1, p(0, 1), 2)).unwrap();
    act(&mut s, 1, p(1, 0), attack(3));
    let text = ron::to_string(&s).unwrap();
    let mut loaded: BattleState = ron::from_str(&text).unwrap();
    loaded.restore_tables(&crate::GameTables {
        terrain: Arc::new(terrain()),
        classes: Arc::new(classes()),
        items: Arc::new(items_for(&strong())),
        spells: Arc::new(spells()),
        skills: Arc::new(skills()),
        arts: Arc::new(test_arts()),
        supports: Arc::default(),
    });
    assert_eq!(loaded, s);
    assert!(loaded.has_fired(0) && loaded.has_fired(1));
    assert_eq!(loaded.triggers().len(), 2);
    assert_eq!(loaded.triggers(), s.triggers());
    assert_eq!(loaded.recruited().len(), 1);
    assert_eq!(loaded.recruited(), s.recruited());
}

#[test]
fn triggers_read_from_ron_as_the_module_docs_write_them() {
    let text = r#"[
        (when: TurnStart(turn: 3, phase: Player), scene: "ch01_hint", once: true),
        (when: UnitEntersArea(who: Faction(Player), area: (x: 10, y: 5, w: 2, h: 2)), scene: "ch01_fort", once: true),
        (when: CombatStart(unit: "harl", against: Some("ana")), scene: "ch01_harl_ana", once: true),
        (when: CombatStart(unit: "harl"), scene: "ch01_harl", once: true),
        (when: HalfHp(unit: "harl"), scene: "ch01_harl_half", once: true),
        (when: UnitFell(unit: "harl"), scene: "ch01_harl_death", once: true),
        (when: UnitFell(unit: "tamsin", mode: Some(Casual)), scene: "ch01_tamsin_retreat", once: true),
        (when: UnitFell(unit: "brom", recruit: true), scene: "ch02_brom_yields", once: true),
        (when: Talk(a: "ana", b: "rook"), scene: "ch02_ana_rook", once: true),
    ]"#;
    let triggers: Vec<Trigger> = ron::from_str(text).unwrap();
    let ch = |s: &str| CharacterId(s.into());
    assert_eq!(triggers.len(), 9);
    assert_eq!(
        triggers[3].when,
        TriggerWhen::CombatStart {
            unit: ch("harl"),
            against: None,
        }
    );
    assert_eq!(triggers[4].when, TriggerWhen::HalfHp { unit: ch("harl") });
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
        triggers[5].when,
        TriggerWhen::UnitFell {
            unit: ch("harl"),
            mode: None,
            recruit: false,
        }
    );
    assert_eq!(
        triggers[7].when,
        TriggerWhen::UnitFell {
            unit: ch("brom"),
            mode: None,
            recruit: true,
        }
    );
    assert_eq!(
        triggers[8],
        trigger(
            TriggerWhen::Talk {
                a: ch("ana"),
                b: ch("rook"),
            },
            "ch02_ana_rook",
            true
        )
    );
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
