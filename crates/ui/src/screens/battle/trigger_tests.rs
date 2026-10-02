//! Tests of the battle screen's dialogue triggers (ticket 0705): scenes
//! queued with the banners, scenes inside a combat's playback (the rogue's
//! line before the combat, its last words before it fades) and `Talk`.

use insta::assert_snapshot;
use trpg_content::character_unit;
use trpg_core::{
    BattleSetup, CharacterId, Faction, Objective, Phase, Pos, Trigger, TriggerWhen, Unit, UnitId,
};

use super::mode::{Effect, MenuEntry, Mode, Selection, menu_entries, step};
use super::testing::{quick_units, setup, through_ai_phases};
use super::*;
use crate::harness::Harness;
use crate::screen::tests::ctx;

/// The Quick Battle's rogue: an enemy the lord can talk to, which joins if
/// defeated.
const ROGUE: &str = "test_rogue";

/// Where the rogue stands in these tests: right of the lord at (3, 5).
const ROGUE_AT: Pos = Pos::new(4, 5);

/// The rogue's tile's left cell (`test_small` is centred: tile `(x, y)`
/// starts at cell `(2 × (x + 10), y + 11)`).
const ROGUE_CELL: (i32, i32) = (28, 16);

/// The Quick Battle's first six units (without its elemental and its mage,
/// which stand where these tests put the rogue and its like), with the
/// rogue (unit 7, an enemy with `rogue_hp` HP) next to the lord, and the
/// Quick Battle's triggers.
fn rogue_setup(c: &Ctx, rogue_hp: StatValue) -> BattleSetup {
    let (map, mut units) = quick_units(c);
    units.truncate(6);
    let def = &c.content.characters.characters[&CharacterId(ROGUE.into())];
    let classes = &c.content.classes;
    let mut rogue = character_unit(
        def,
        UnitId(7),
        classes,
        &c.content.items,
        Faction::Enemy,
        ROGUE_AT,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    rogue.hp = rogue_hp;
    units.push(rogue);
    BattleSetup {
        triggers: c.content.battles[QUICK_BATTLE].triggers.clone(),

        ..setup(c, map, units, Objective::Rout { turn_limit: None })
    }
}

fn rogue_battle(c: &Ctx, rogue_hp: StatValue) -> BattleState {
    BattleState::new(rogue_setup(c, rogue_hp)).0
}

/// One frame of `dt` seconds with `actions`; the screen's answer.
fn frame(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], dt: f32) -> String {
    format!(
        "{:?}",
        s.update(c, &FrameInput::new(actions.to_vec(), dt, vec![]))
    )
}

// ---- The scene queue -----------------------------------------------------------

/// A turn-start trigger for `turn`'s player phase, on the test scene.
fn turn_start(turn: u32) -> Trigger {
    Trigger {
        when: TriggerWhen::TurnStart {
            turn,
            phase: Phase::Player,
        },
        scene: "test_turn_3".into(),
        once: true,
    }
}

#[test]
fn a_scene_at_the_battles_start_plays_after_the_banner_and_tips_wait() {
    let mut c = ctx();
    let (map, units) = quick_units(&c);
    let (state, events) = BattleState::new(BattleSetup {
        triggers: vec![turn_start(1)],
        ..setup(&c, map, units, Objective::Rout { turn_limit: None })
    });
    let mut s = BattleScreen::start(state, &events);
    // The events' order: the phase starts (its banner), then the scene.
    assert!(s.banner().is_some());
    assert_eq!(s.queued_scene(), None);
    assert_eq!(frame(&mut s, &mut c, &[], 0.0), "None");
    assert_eq!(s.shown_tip(), None);
    assert_eq!(
        frame(&mut s, &mut c, &[Action::Confirm], 0.0),
        "Push(dialogue)"
    );
    assert_eq!(s.banner(), None);
    assert_eq!(s.shown_tip(), None);
    assert_eq!(s.queued_scene(), None);
    assert_eq!(frame(&mut s, &mut c, &[], 0.0), "None");
    // Without events, `start` is `new`.
    let q = quick_battle(&c.content).unwrap();
    let s = BattleScreen::start(q, &[]);
    assert_eq!(s.queued_scene(), None);
    assert_eq!(s.banner(), None);
}

#[test]
fn a_turn_start_scene_waits_for_its_phase_banner() {
    let mut c = ctx();
    let (map, units) = quick_units(&c);
    let state = BattleState::new(BattleSetup {
        triggers: vec![turn_start(2)],
        ..setup(&c, map, units, Objective::Rout { turn_limit: None })
    })
    .0;
    let mut s = BattleScreen::new(state);
    s.apply(&Command::EndPhase);
    // The enemy phase's banner closes, the enemies act, the phase ends.
    assert_eq!(frame(&mut s, &mut c, &[Action::Confirm], 0.0), "None");
    through_ai_phases(&mut s, &mut c, 30.0);
    assert!(matches!(
        s.banner().map(|b| b.kind),
        Some(banner::BannerKind::Phase {
            phase: Phase::Player,
            turn: 2
        })
    ));
    assert_eq!(s.queued_scene(), None, "behind the banner");
    assert_eq!(s.help(&c), "f skip");
    // Closing it plays the scene at once.
    assert_eq!(
        frame(&mut s, &mut c, &[Action::Confirm], 0.0),
        "Push(dialogue)"
    );
    assert_eq!(s.banner(), None);
    assert_eq!(frame(&mut s, &mut c, &[], 0.0), "None");
}

// ---- Talk ----------------------------------------------------------------------

/// The lord selected, kept where it stands: its action menu.
fn lord_menu(state: &BattleState) -> Mode {
    let sel = Selection::new(state, UnitId(1)).unwrap();
    step(Mode::Selected(sel), Action::Confirm, Pos::new(3, 5), state).0
}

#[test]
fn talk_is_offered_next_to_someone_to_talk_to_and_picks_a_target() {
    let c = ctx();
    let s = rogue_battle(&c, 22);
    let menu = lord_menu(&s);
    let Mode::ActionMenu { entries, menu, .. } = &menu else {
        panic!("{menu:?}");
    };
    assert_eq!(
        entries,
        &[
            MenuEntry::Attack,
            MenuEntry::Talk,
            MenuEntry::Skill,
            MenuEntry::Item,
            MenuEntry::Equip,
            MenuEntry::Wait
        ]
    );
    assert_eq!(menu.focus(), 0, "Attack stays the default");
    assert_eq!(MenuEntry::Talk.label(), "Talk");
    // Talk: the rogue, the cursor on it.
    let (mode, effect) = step(lord_menu(&s), Action::CursorDown, Pos::new(3, 5), &s);
    let (mode, effect2) = step(mode, Action::Confirm, Pos::new(3, 5), &s);
    assert_eq!((effect, effect2), (Effect::None, Effect::Cursor(ROGUE_AT)));
    assert!(matches!(&mode, Mode::TalkTarget { targets, index: 0, .. } if targets == &[UnitId(7)]));
    assert_eq!(mode.drawn_pos(UnitId(1)), Some(Pos::new(3, 5)));
    assert_eq!(mode.selection().map(|s| s.unit), Some(UnitId(1)));
    // Cycling one target stays on it; other keys do nothing.
    let (mode, effect) = step(mode, Action::NextUnit, ROGUE_AT, &s);
    assert_eq!(effect, Effect::Cursor(ROGUE_AT));
    let (mode, effect) = step(mode, Action::Info, ROGUE_AT, &s);
    assert_eq!(effect, Effect::Cursor(ROGUE_AT));
    // Cancel: the menu, on Talk, the cursor back on the lord.
    let (back, effect) = step(mode.clone(), Action::Cancel, ROGUE_AT, &s);
    assert_eq!(effect, Effect::Cursor(Pos::new(3, 5)));
    let Mode::ActionMenu { entries, menu, .. } = &back else {
        panic!("{back:?}");
    };
    assert_eq!(entries[menu.focus()], MenuEntry::Talk);
    // Confirm: the talk, which keeps the unit's move and action open.
    let sel = mode.selection().cloned().unwrap();
    let (after, effect) = step(mode, Action::Confirm, ROGUE_AT, &s);
    assert_eq!(after, Mode::default());
    assert_eq!(
        effect,
        Effect::ApplyStay(
            Command::Talk {
                unit: UnitId(1),
                dest: Pos::new(3, 5),
                target: UnitId(7),
            },
            Box::new(sel)
        )
    );
    // Nobody to talk to from elsewhere, or for the knight.
    let mut sel = Selection::new(&s, UnitId(1)).unwrap();
    sel.path = vec![Pos::new(3, 5), Pos::new(3, 4)];
    assert!(!menu_entries(&sel, &s).contains(&MenuEntry::Talk));
    let knight = Selection::new(&s, UnitId(2)).unwrap();
    assert!(!menu_entries(&knight, &s).contains(&MenuEntry::Talk));
}

#[test]
fn talk_cycles_between_several_targets() {
    let c = ctx();
    let mut setup = rogue_setup(&c, 22);
    // Two more rogue-like enemies, below and left of the lord, each with
    // its own talk: three targets, in unit order.
    for (id, pos, who) in [
        (8, Pos::new(3, 6), "extra_1"),
        (9, Pos::new(2, 5), "extra_2"),
    ] {
        let mut other: Unit = setup.units[6].clone();
        other.id = UnitId(id);
        other.pos = pos;
        other.character = Some(CharacterId(who.into()));
        setup.units.push(other);
        setup.triggers.push(Trigger {
            when: TriggerWhen::Talk {
                a: CharacterId("test_lord".into()),
                b: CharacterId(who.into()),
            },
            scene: "test_talk".into(),
            once: true,
        });
    }
    let s = BattleState::new(setup).0;
    let (mode, _) = step(lord_menu(&s), Action::CursorDown, Pos::new(3, 5), &s);
    let (mode, _) = step(mode, Action::Confirm, Pos::new(3, 5), &s);
    let at = |effect| match effect {
        Effect::Cursor(p) => p,
        other => panic!("{other:?}"),
    };
    let (mode, effect) = step(mode, Action::CursorRight, ROGUE_AT, &s);
    assert_eq!(at(effect), Pos::new(3, 6));
    let (mode, effect) = step(mode, Action::PrevUnit, Pos::new(3, 6), &s);
    assert_eq!(at(effect), ROGUE_AT);
    // Back from the first: the last.
    let (mode, effect) = step(mode, Action::CursorUp, ROGUE_AT, &s);
    assert_eq!(at(effect), Pos::new(2, 5));
    // On from the last: the first.
    let (_, effect) = step(mode, Action::NextUnit, Pos::new(2, 5), &s);
    assert_eq!(at(effect), ROGUE_AT);
}

#[test]
fn talking_plays_the_scene_and_keeps_the_units_turn() {
    let mut c = ctx();
    let mut s = BattleScreen::new(rogue_battle(&c, 22));
    s.cursor.jump(Pos::new(3, 5));
    // Select the lord, stay, Talk, the rogue.
    for a in [Action::Confirm, Action::Confirm, Action::CursorDown] {
        frame(&mut s, &mut c, &[a], 0.0);
    }
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(
        matches!(s.mode(), Mode::TalkTarget { .. }),
        "{:?}",
        s.mode()
    );
    assert_eq!(s.help(&c), "arrows next target · f talk · d back");
    assert_eq!(
        frame(&mut s, &mut c, &[Action::Confirm], 0.0),
        "Push(dialogue)"
    );
    // Nobody changes sides; the lord hasn't used its turn: its menu is
    // open again, with no Talk (it has been said).
    assert_eq!(s.state().unit(UnitId(7)).unwrap().faction, Faction::Enemy);
    assert!(s.state().recruited().is_empty());
    assert!(!s.state().unit(UnitId(1)).unwrap().acted);
    assert_eq!(s.history().len(), 1);
    let Mode::ActionMenu { entries, menu, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert!(!entries.contains(&MenuEntry::Talk));
    assert_eq!(entries[menu.focus()], MenuEntry::Attack);
    // Then it acts as usual.
    s.apply(&Command::Act {
        unit: UnitId(1),
        dest: Pos::new(3, 5),
        action: trpg_core::UnitAction::Wait,
    });
    assert!(s.state().unit(UnitId(1)).unwrap().acted);
    assert_eq!(s.history().len(), 2);
}

// ---- In a combat: harness ------------------------------------------------------

/// The rogue battle in the harness, with the lord attacking the rogue with
/// its iron sword (select, stay, Attack, the first weapon, the rogue).
fn lord_attacks_rogue(rogue_hp: StatValue) -> Harness {
    let c = ctx();
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(rogue_battle(&c, rogue_hp))));
    // The cursor starts on the lord.
    h.keys("f f f f f");
    h
}

/// Cells `x..x + w` of row `y`.
fn text(h: &Harness, x: i32, y: i32, w: i32) -> String {
    let buf = h.game().buffer();
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// Whether some row shows `s`.
fn shows(h: &Harness, s: &str) -> bool {
    (0..32).any(|y| text(h, 0, y, 100).contains(s))
}

/// Reads through a one-line scene just pushed: reveal, then next.
fn read_line(h: &mut Harness) {
    h.keys("f f");
}

/// Ends a one-line scene already shown in full.
fn close_line(h: &mut Harness) {
    h.keys("f");
}

#[test]
fn engaging_the_rogue_plays_its_line_over_the_map_then_the_combat() {
    // At full HP (22): the lord's attack leaves it standing at half or less.
    let mut h = lord_attacks_rogue(22);
    assert_eq!(h.screens(), ["battle", "dialogue"]);
    h.wait(1.0);
    assert!(shows(&h, "You picked the wrong fort to storm."));
    assert_snapshot!(h.snapshot());
    close_line(&mut h);
    // Back to the combat, from its start.
    assert_eq!(h.screens(), ["battle"]);
    assert!(shows(&h, "d skip · hold f fast"), "{}", h.snapshot());
    assert!(shows(&h, "Test Lord"));
    // After the strikes: its half-HP line, then the end of the combat.
    h.wait(10.0);
    assert_eq!(h.screens(), ["battle", "dialogue"]);
    h.wait(1.0);
    assert!(shows(&h, "That all you've got?"), "{}", h.snapshot());
    close_line(&mut h);
    h.wait(2.0);
    assert!(shows(&h, "d menu · Space end turn"));
    assert_eq!(h.screens(), ["battle"]);
}

#[test]
fn a_fallen_rogue_says_its_last_words_before_it_fades() {
    let mut h = lord_attacks_rogue(1);
    read_line(&mut h);
    assert_eq!(h.screens(), ["battle"]);
    // The first hit fells it: its last words, while it is still drawn.
    h.wait(2.0);
    assert_eq!(h.screens(), ["battle", "dialogue"]);
    h.wait(1.0);
    assert!(shows(
        &h,
        "Should have taken the other job... Fine. You win. I'm yours."
    ));
    assert_snapshot!(h.snapshot());
    close_line(&mut h);
    assert_eq!(h.screens(), ["battle"]);
    assert_eq!(
        text(&h, ROGUE_CELL.0, ROGUE_CELL.1, 2),
        "Ro",
        "not faded yet"
    );
    // Then it fades and the battle goes on.
    h.wait(3.0);
    assert_eq!(text(&h, ROGUE_CELL.0, ROGUE_CELL.1, 2), "..");
    assert!(shows(&h, "d menu · Space end turn"));
}

#[test]
fn skipping_a_combat_still_stops_for_its_scenes() {
    let mut h = lord_attacks_rogue(1);
    read_line(&mut h);
    // Skip: straight to the last words, then to the end.
    h.keys("d");
    assert_eq!(h.screens(), ["battle", "dialogue"]);
    h.wait(1.0);
    assert!(shows(
        &h,
        "Should have taken the other job... Fine. You win. I'm yours."
    ));
    // Shown in full already: one press ends it.
    h.keys("f");
    assert_eq!(h.screens(), ["battle"]);
    // Then the lord's EXP bar (0602), and browsing again.
    h.wait(3.0);
    assert!(shows(&h, "d menu · Space end turn"), "{}", h.snapshot());
}

#[test]
fn a_scene_waits_for_the_exp_bar() {
    let mut c = ctx();
    let (map, mut units) = quick_units(&c);
    units[0].pos = Pos::new(6, 2);
    units[0].hp = 5;
    units[1].pos = Pos::new(6, 4);
    units[1].learned.insert(trpg_core::SpellId::new("heal"));
    let state = BattleState::new(BattleSetup {
        triggers: vec![Trigger {
            when: TriggerWhen::UnitEntersArea {
                who: trpg_core::Who::Faction(Faction::Player),
                area: trpg_core::TileRect {
                    x: 6,
                    y: 3,
                    w: 1,
                    h: 1,
                },
            },
            scene: "test_fort".into(),
            once: true,
        }],
        ..setup(&c, map, units, Objective::Rout { turn_limit: None })
    })
    .0;
    let mut s = BattleScreen::new(state);
    // The knight steps onto the area and heals the lord: the scene, then
    // the knight's EXP bar.
    s.apply(&Command::Act {
        unit: UnitId(2),
        dest: Pos::new(6, 3),
        action: trpg_core::UnitAction::Cast {
            spell: trpg_core::SpellId::new("heal"),
            target: trpg_core::CastTarget::Unit(UnitId(1)),
            active: None,
        },
    });
    assert!(s.progress().is_some());
    assert_eq!(s.queued_scene(), None, "after the EXP bar");
    assert_eq!(frame(&mut s, &mut c, &[], 0.0), "None");
    let bar = super::progress::PROGRESS_TIMINGS;
    let after = frame(&mut s, &mut c, &[], bar.exp_fill + bar.exp_hold + 0.1);
    assert!(s.progress().is_none());
    assert_eq!(after, "Push(dialogue)");
}

#[test]
fn the_rewind_list_names_a_talk() {
    let c = ctx();
    let mut s = BattleScreen::new(rogue_battle(&c, 22));
    s.apply(&Command::Talk {
        unit: UnitId(1),
        dest: Pos::new(3, 5),
        target: UnitId(7),
    });
    let replayed = s.history().replay();
    assert_eq!(
        rewind::describe(&replayed[0]),
        "Turn 1 · Test Lord talked to Test Rogue"
    );
}
