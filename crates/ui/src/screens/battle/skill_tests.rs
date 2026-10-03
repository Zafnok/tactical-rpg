//! Tests of skills in the battle UI (ticket 0412): the skills block and the
//! timed effects on the info screen, the `Skill` menu with its target mode,
//! and the marker on a unit under an effect (combat actives in the attack
//! flow: `art_tests.rs`). The numbers are always `core`'s. Non-attack
//! actives show their uses left this battle (ticket 0316).

use insta::assert_snapshot;
use trpg_core::{BattleState, Command, Objective, Phase, Pos, SkillId, UnitAction, UnitId};

use super::BattleScreen;
use super::mode::{MenuEntry, Mode};
use super::testing::{battle_with, quick_units};
use crate::FrameInput;
use crate::color::Rgb;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::map_view::{RangeKind, UnitView};
use crate::screen::tests::ctx;
use crate::screen::{Ctx, Screen};

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

fn step(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
    s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
}

fn wait(s: &mut BattleScreen, c: &mut Ctx, seconds: f32) {
    s.update(c, &FrameInput::new(vec![], seconds, vec![]));
}

fn render(s: &BattleScreen, c: &Ctx) -> GlyphBuffer {
    let stale = Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(c, &mut buf);
    buf
}

/// Cells `x..x + w` of row `y`, trimmed on the right.
fn text(buf: &GlyphBuffer, x: i32, y: i32, w: i32) -> String {
    (x..x + w)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// The Quick Battle's units on a rout battle, `edit` applied to them first.
fn battle(c: &Ctx, edit: impl FnOnce(&mut Vec<trpg_core::Unit>)) -> BattleState {
    let (map, mut units) = quick_units(c);
    edit(&mut units);
    battle_with(c, map, units, Objective::Rout { turn_limit: None })
}

/// `unit` learns `skill`.
fn learn(c: &Ctx, unit: &mut trpg_core::Unit, skill: &str) {
    assert!(unit.learn_skill(&SkillId::new(skill), &c.content.skills));
}

/// The skirmish, with the lord knowing Keen Edge and Sword Focus 1: the
/// lord at (6, 2) beside the raider (7, 1) and the brigand (8, 2).
fn keen_skirmish(c: &Ctx) -> BattleState {
    battle(c, |units| {
        units[0].pos = p(6, 2);
        units[2].pos = p(8, 4);
        learn(c, &mut units[0], "keen_edge");
        learn(c, &mut units[0], "sword_focus_1");
    })
}

/// The lord selected, moved to (7, 2) and its action menu open.
fn lord_menu(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = BattleScreen::new(state);
    step(
        &mut s,
        c,
        &[Action::Confirm, Action::CursorRight, Action::Confirm],
    );
    wait(&mut s, c, 0.5);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    s
}

/// The left cell of the two-letter label `label` (any case) on the map.
fn shown_unit(s: &BattleScreen, c: &Ctx, label: &str) -> UnitView {
    let scene = s.scene(c);
    let unit = scene
        .units
        .iter()
        .find(|u| u.label.eq_ignore_ascii_case(label));
    unit.expect("the unit is on the map").clone()
}

/// The action menu's focus moved to `entry`.
fn focus_entry(s: &mut BattleScreen, c: &mut Ctx, entry: MenuEntry) {
    for _ in 0..8 {
        let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
            panic!("{:?}", s.mode());
        };
        if entries.get(menu.focus()) == Some(&entry) {
            return;
        }
        step(s, c, &[Action::CursorUp]);
    }
    panic!("no {entry:?} in the menu");
}

/// The knight (unit 2, a Guard, knowing Brace) selected and its action menu
/// open where it stands, with its weapon's durability set to `durability`.
fn knight_menu(c: &mut Ctx, durability: u32) -> BattleScreen {
    let state = battle(c, |units| {
        let slot = units[1].loadout.equipped_slot().unwrap();
        units[1].loadout.weapons[slot]
            .as_mut()
            .unwrap()
            .durability_left = durability;
    });
    knight_menu_in(c, state)
}

/// `state` after the knight braced `times` times, one turn each: the
/// start of the player's next phase.
fn braced(mut state: BattleState, times: usize) -> BattleState {
    let brace = Command::Act {
        unit: UnitId(2),
        dest: state.unit(UnitId(2)).unwrap().pos,
        action: UnitAction::UseSkill {
            skill: SkillId::new("brace"),
            target: None,
        },
    };
    for _ in 0..times {
        state.apply(&brace).unwrap();
        state.apply(&Command::EndPhase).unwrap();
        while state.phase() != Phase::Player {
            state.apply(&Command::EndPhase).unwrap();
        }
    }
    state
}

/// The uses the knight has left of `skill`.
fn knight_uses(state: &BattleState, skill: &str) -> u8 {
    let knight = state.unit(UnitId(2)).unwrap();
    knight.skill_uses.uses_left(&SkillId::new(skill))
}

/// The knight selected in `state` and its action menu open where it stands.
fn knight_menu_in(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = BattleScreen::new(state);
    // The cursor starts on the lord; walk it to the knight.
    step(&mut s, c, &[Action::CursorRight, Action::CursorDown]);
    step(&mut s, c, &[Action::Confirm, Action::Confirm]);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    s
}

/// Whether the action menu's `entry` is enabled.
fn entry_enabled(s: &BattleScreen, entry: MenuEntry) -> Option<bool> {
    let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let i = entries.iter().position(|&e| e == entry)?;
    Some(menu.items()[i].enabled)
}

/// The Def shown on the info screen of unit `id` of `state`.
fn shown_def(c: &mut Ctx, state: &BattleState, id: UnitId) -> i32 {
    let mut s = BattleScreen::new(state.clone());
    let at = state.unit(id).unwrap().pos;
    s.cursor.jump(at);
    step(&mut s, c, &[Action::Info]);
    assert!(matches!(s.mode(), Mode::Info { .. }), "{:?}", s.mode());
    let buf = render(&s, c);
    // The Def row: stats are listed from row 7 (Str, Mag, Dex, Spd, Def...).
    let row = text(&buf, 29, 11, 8);
    assert!(row.starts_with("Def"), "{row}");
    row[3..].trim().parse().expect("a number")
}

#[test]
fn brace_from_the_menu_raises_def_on_the_info_screen_until_the_next_own_phase() {
    let mut c = ctx();
    let mut s = knight_menu(&mut c, 20);
    let before = shown_def(&mut c, s.state(), UnitId(2));
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    // Skill list: Brace, with its 3 uses a battle all left.
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::SkillMenu { menu, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(menu.items().len(), 1);
    let buf = render(&s, &c);
    let line = (0..30)
        .map(|y| text(&buf, 0, y, 70))
        .find(|l| l.contains("Brace"));
    let line = line.expect("the list shows Brace");
    assert!(line.contains("Brace     3/3"), "{line}");
    assert!(!line.contains("dur") && !line.contains("Wpn"), "{line}");
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Idle { .. }), "{:?}", s.mode());
    let after = shown_def(&mut c, s.state(), UnitId(2));
    assert_eq!(after, before + 5);
    // One use is spent; the knight's weapon paid nothing.
    assert_eq!(knight_uses(s.state(), "brace"), 2);
    let knight = s.state().unit(UnitId(2)).unwrap();
    let slot = knight.loadout.equipped_slot().unwrap();
    assert_eq!(knight.loadout.weapon(slot).unwrap().durability_left, 20);
    // Player, enemy and other phases pass: at the start of the knight's own
    // phase Brace has ended.
    let mut next = s.state().clone();
    next.apply(&Command::EndPhase).unwrap();
    while next.phase() != Phase::Player {
        next.apply(&Command::EndPhase).unwrap();
    }
    assert_eq!(shown_def(&mut c, &next, UnitId(2)), before);
}

#[test]
fn a_skill_menu_entry_needs_a_usable_non_combat_active() {
    let mut c = ctx();
    // The Guard's Brace needs no weapon: a broken one changes nothing.
    let s = knight_menu(&mut c, 0);
    assert_eq!(entry_enabled(&s, MenuEntry::Skill), Some(true));
    // With a use left the entry is on; after its 3 uses it is dimmed (the
    // cursor skips it).
    let state = braced(battle(&c, |_| {}), 2);
    assert_eq!(knight_uses(&state, "brace"), 1);
    let s = knight_menu_in(&mut c, state);
    assert_eq!(entry_enabled(&s, MenuEntry::Skill), Some(true));
    let state = braced(battle(&c, |_| {}), 3);
    assert_eq!(knight_uses(&state, "brace"), 0);
    let mut s = knight_menu_in(&mut c, state);
    assert_eq!(entry_enabled(&s, MenuEntry::Skill), Some(false));
    step(&mut s, &mut c, &[Action::Cancel]);
    // A unit that knows no non-combat active has no entry at all: the
    // archer's active (Vault) is a combat one.
    let state = battle(&c, |_| {});
    let mut s = BattleScreen::new(state);
    step(&mut s, &mut c, &[Action::CursorLeft, Action::CursorUp]);
    step(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    assert_eq!(entry_enabled(&s, MenuEntry::Skill), None);
}

#[test]
fn cancelling_out_of_the_skill_menu_leaves_the_battle_unchanged() {
    let mut c = ctx();
    let mut s = knight_menu(&mut c, 20);
    let before = s.state().clone();
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm, Action::Cancel]);
    assert!(
        matches!(s.mode(), Mode::ActionMenu { .. }),
        "{:?}",
        s.mode()
    );
    assert_eq!(s.state(), &before);
    // Back on `Skill`, not the default entry.
    let Mode::ActionMenu { menu, entries, .. } = s.mode() else {
        unreachable!()
    };
    assert_eq!(entries[menu.focus()], MenuEntry::Skill);
}

#[test]
fn a_unit_under_an_effect_shows_the_effect_colour_behind_its_glyphs() {
    let mut c = ctx();
    let mut s = knight_menu(&mut c, 20);
    // Knight at (4, 6): without an effect the map shows none (each skin
    // marks one its own way: the glyph skin with the effect colour behind
    // its letters).
    assert!(!shown_unit(&s, &c, "Kn").has_effect);
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    assert!(shown_unit(&s, &c, "Kn").has_effect);
}

#[test]
fn the_info_screen_lists_skills_with_markers_costs_and_timed_effects() {
    let mut c = ctx();
    let mut s = knight_menu(&mut c, 20);
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    let mut info = BattleScreen::new(s.state().clone());
    info.cursor.jump(p(4, 6));
    step(&mut info, &mut c, &[Action::Info]);
    let buf = render(&info, &c);
    let rows: Vec<String> = (0..30).map(|y| text(&buf, 29, y, 26)).collect();
    let find = |needle: &str| rows.iter().any(|r| r.contains(needle));
    assert!(find("A Brace"), "{rows:?}");
    // One of its 3 uses is spent.
    assert!(find("2/3"), "{rows:?}");
    assert!(!find("dur"), "{rows:?}");
    assert!(find("self: Def +5 Res +5"), "{rows:?}");
    let effects: Vec<String> = (0..30).map(|y| text(&buf, 37, y, 18)).collect();
    assert!(effects.iter().any(|r| r == "Effects"), "{effects:?}");
    assert!(effects.iter().any(|r| r == "Def +5 Res +5"), "{effects:?}");
    assert!(
        effects.iter().any(|r| r == "until Player phase"),
        "{effects:?}"
    );
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

#[test]
fn a_learned_passive_is_marked_p_with_its_effect() {
    let mut c = ctx();
    let mut s = BattleScreen::new(keen_skirmish(&c));
    // The lord is under the cursor at the start.
    step(&mut s, &mut c, &[Action::Info]);
    let buf = render(&s, &c);
    let rows: Vec<String> = (0..30).map(|y| text(&buf, 29, y, 26)).collect();
    let find = |needle: &str| rows.iter().any(|r| r.contains(needle));
    assert!(find("P Sword Focus 1"), "{rows:?}");
    assert!(find("+10 crit with Sword"), "{rows:?}");
    assert!(find("A Keen Edge"), "{rows:?}");
    assert!(find("Sword: +30 hit +10 crit"), "{rows:?}");
}

#[test]
fn skill_menu_snapshot() {
    let mut c = ctx();
    let mut s = knight_menu(&mut c, 20);
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm]);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

/// The knight also knowing Fortify, after its 3 Braces: the skill list
/// opened from its action menu.
fn spent_brace_menu(c: &mut Ctx) -> BattleScreen {
    let state = battle(c, |units| learn(c, &mut units[1], "fortify"));
    let mut s = knight_menu_in(c, braced(state, 3));
    focus_entry(&mut s, c, MenuEntry::Skill);
    step(&mut s, c, &[Action::Confirm]);
    s
}

#[test]
fn a_skill_with_no_uses_left_is_dimmed_with_the_reason() {
    let mut c = ctx();
    let mut s = spent_brace_menu(&mut c);
    let Mode::SkillMenu { menu, choices, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let lines: Vec<(&str, bool, Option<&str>)> = choices
        .iter()
        .map(|c| (c.skill.0.as_str(), c.usable, c.reason.as_deref()))
        .collect();
    assert_eq!(
        lines,
        [
            ("brace", false, Some("no uses left")),
            ("fortify", true, None)
        ]
    );
    let enabled: Vec<bool> = menu.items().iter().map(|i| i.enabled).collect();
    assert_eq!(enabled, [false, true]);
    let buf = render(&s, &c);
    let row = |name: &str| {
        (0..30)
            .map(|y| text(&buf, 0, y, 70))
            .find(|l| l.contains(name))
            .unwrap_or_else(|| panic!("the list shows {name}"))
    };
    let (brace, fortify) = (row("Brace"), row("Fortify"));
    assert!(brace.contains("Brace       0/3  no uses left"), "{brace}");
    assert!(fortify.contains("Fortify     2/2 "), "{fortify}");
    assert!(!fortify.contains("left"), "{fortify}");
    // The cursor skips the spent skill: it starts on Fortify, which works.
    assert_eq!(menu.focus(), 1);
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Idle { .. }), "{:?}", s.mode());
    assert_eq!(knight_uses(s.state(), "fortify"), 1);
    assert_eq!(knight_uses(s.state(), "brace"), 0);
}

#[test]
fn skill_menu_with_no_uses_left_snapshot() {
    let mut c = ctx();
    let s = spent_brace_menu(&mut c);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn a_skill_that_only_lacks_someone_to_use_it_on_is_dimmed_without_a_reason() {
    let mut c = ctx();
    // The lord knows Shove and Brace, with nobody beside it to shove.
    let state = battle(&c, |units| {
        learn(&c, &mut units[0], "shove");
        learn(&c, &mut units[0], "brace");
    });
    let mut s = BattleScreen::new(state);
    step(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::SkillMenu { menu, choices, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let shove = choices.iter().position(|c| c.skill.0 == "shove").unwrap();
    assert_eq!(
        (choices[shove].usable, choices[shove].reason.as_deref()),
        (false, None)
    );
    assert!(!menu.items()[shove].enabled);
    let buf = render(&s, &c);
    let line = (0..30)
        .map(|y| text(&buf, 0, y, 70))
        .find(|l| l.contains("Shove"))
        .expect("the list shows Shove");
    // The lord's own Inspire makes the names 7 wide.
    assert!(line.contains("Shove       8/8 "), "{line}");
    assert!(!line.contains("left"), "{line}");
}

#[test]
fn shove_picks_an_adjacent_enemy_and_pushes_it() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        units[0].pos = p(6, 2);
        learn(&c, &mut units[0], "shove");
    });
    let mut s = lord_menu(&mut c, state);
    let before = s.state().clone();
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::SkillMenu { choices, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    // Shove needs an enemy beside: the raider at (7, 1) above and the
    // brigand at (8, 2) to the right, in reading order.
    let shove = choices.iter().find(|c| c.skill.0 == "shove").unwrap();
    assert!(shove.needs_target && shove.usable);
    assert_eq!(shove.targets, [UnitId(6), UnitId(4)]);
    // Choose it: the cursor starts on the raider, and the message names it.
    let raider_tints = |s: &BattleScreen, c: &Ctx| {
        let raider = shown_unit(s, c, "Ra");
        s.scene(c).tints_at(raider.pos)
    };
    assert!(raider_tints(&s, &c).is_empty());
    let i = choices.iter().position(|c| c.skill.0 == "shove").unwrap();
    for _ in 0..i {
        step(&mut s, &mut c, &[Action::CursorDown]);
    }
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::SkillTarget(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(t.target(), UnitId(6));
    assert_eq!(s.mode().drawn_pos(UnitId(1)), Some(p(7, 2)));
    assert_eq!(s.mode().drawn_pos(UnitId(4)), None);
    // Whoever can be shoved is tinted as in an attack.
    assert_eq!(raider_tints(&s, &c), [RangeKind::Attack]);
    // Left goes back around to the brigand, and right to the raider.
    step(&mut s, &mut c, &[Action::CursorLeft]);
    let Mode::SkillTarget(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(t.target(), UnitId(4));
    step(&mut s, &mut c, &[Action::CursorRight]);
    let buf = render(&s, &c);
    let message = text(&buf, 1, 30, 60);
    // The message says what it costs: one of the 8 uses.
    assert!(
        message.starts_with("Shove on Raider (8 → 7 uses)"),
        "{message}"
    );
    // Cancel goes back to the list; choose again, pick the brigand, confirm.
    step(&mut s, &mut c, &[Action::Cancel]);
    assert!(matches!(s.mode(), Mode::SkillMenu { .. }), "{:?}", s.mode());
    step(&mut s, &mut c, &[Action::Confirm, Action::CursorRight]);
    let Mode::SkillTarget(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(t.target(), UnitId(4));
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Idle { .. }), "{:?}", s.mode());
    let brigand = s.state().unit(UnitId(4)).unwrap();
    assert_eq!(brigand.pos, p(9, 2));
    let lord = s.state().unit(UnitId(1)).unwrap();
    assert_eq!(lord.skill_uses.uses_left(&SkillId::new("shove")), 7);
    // Exactly what the core does with the same command.
    let mut expected = before;
    let cmd = Command::Act {
        unit: UnitId(1),
        dest: p(7, 2),
        action: UnitAction::UseSkill {
            skill: SkillId::new("shove"),
            target: Some(UnitId(4)),
        },
    };
    expected.apply(&cmd).unwrap();
    assert_eq!(s.state(), &expected);
}

#[test]
fn the_info_screen_lists_at_most_two_effects_and_six_skills() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        for skill in [
            "sword_focus_1",
            "light_feet_1",
            "axe_focus_1",
            "bow_focus_1",
            "steadfast_1",
            "charge_1",
            "black_magic_1",
        ] {
            learn(&c, &mut units[0], skill);
        }
        for id in ["brace", "fortify", "keen_edge"] {
            units[0].add_effect(trpg_core::TimedEffect {
                source: trpg_core::EffectSource::Skill(SkillId::new(id)),
                mods: trpg_core::TimedMods::default(),
                until: trpg_core::Phase::Enemy,
            });
        }
    });
    let mut s = BattleScreen::new(state);
    step(&mut s, &mut c, &[Action::Info]);
    let buf = render(&s, &c);
    // Effects: three rows each from row 7; the third isn't listed.
    let name = |y| text(&buf, 37, y, 18);
    assert_eq!(name(7), "Brace");
    assert_eq!(name(10), "Fortify");
    assert_eq!(name(13), "");
    // Skills: two rows each from row 16; the seventh isn't listed.
    let rows: Vec<String> = (16..29).map(|y| text(&buf, 29, y, 26)).collect();
    let listed = rows
        .iter()
        .filter(|r| r.starts_with("P ") || r.starts_with("A "))
        .count();
    assert_eq!(listed, 6, "{rows:?}");
}

/// The map scene (ADR-0038) while choosing who to shove: the skill's
/// targets are an attack range, the user stands where it will act from and
/// the cursor is on the target.
#[test]
fn the_scene_marks_who_a_skill_can_target() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        units[0].pos = p(6, 2);
        learn(&c, &mut units[0], "shove");
    });
    let mut s = lord_menu(&mut c, state);
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm]);
    let Mode::SkillMenu { choices, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let i = choices.iter().position(|c| c.skill.0 == "shove").unwrap();
    // The skill list: no cursor, no range.
    let scene = s.scene(&c);
    assert_eq!(scene.cursor, None);
    assert!(scene.tinted(RangeKind::Attack).is_empty());
    for _ in 0..i {
        step(&mut s, &mut c, &[Action::CursorDown]);
    }
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::SkillTarget(_)), "{:?}", s.mode());
    let scene = s.scene(&c);
    // The raider above and the brigand to the right of (7, 2).
    assert_eq!(scene.tinted(RangeKind::Attack), [p(7, 1), p(8, 2)]);
    for kind in [RangeKind::Danger, RangeKind::Move, RangeKind::Heal] {
        assert!(scene.tinted(kind).is_empty(), "{kind:?}");
    }
    assert_eq!(scene.unit(UnitId(1)).map(|u| u.pos), Some(p(7, 2)));
    assert_eq!(scene.cursor.map(|c| c.pos), Some(p(7, 1)));
    assert!(scene.path.is_empty());
}
