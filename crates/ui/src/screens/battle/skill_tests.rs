//! Tests of skills in the battle UI (ticket 0412): the skills block and the
//! timed effects on the info screen, the combat active cycled in the attack
//! forecast, the `Skill` menu with its target mode, and the marker on a unit
//! under an effect. The numbers are always `core`'s.

use insta::assert_snapshot;
use trpg_core::{BattleState, Command, Objective, Pos, SkillId, UnitAction, UnitId};

use super::BattleScreen;
use super::attack::attack_with;
use super::forecast::{HP_ROW, LEFT_X, SKILL_ROW};
use super::mode::{MenuEntry, Mode};
use super::testing::{battle_with, quick_units};
use super::units::EFFECT_BLEND;
use crate::FrameInput;
use crate::color::{Rgb, UiColor};
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
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

/// The side panel's row `y`, inside its border.
fn panel(buf: &GlyphBuffer, y: i32) -> String {
    text(buf, 71, y, 28).trim().to_owned()
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

/// The lord targeting the brigand (unit 4) with its iron sword.
fn lord_on_brigand(c: &mut Ctx, state: BattleState) -> BattleScreen {
    let mut s = lord_menu(c, state);
    // Attack, then the iron sword; the raider is first, the brigand next.
    step(
        &mut s,
        c,
        &[Action::Confirm, Action::Confirm, Action::CursorRight],
    );
    let Mode::Targeting(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(t.target(), UnitId(4));
    s
}

/// The left cell of the two-letter label `label` (any case) on the map.
fn label_cell(buf: &GlyphBuffer, label: &str) -> (i32, i32) {
    (0..30)
        .flat_map(|y| (0..70).map(move |x| (x, y)))
        .find(|&(x, y)| text(buf, x, y, 2).eq_ignore_ascii_case(label))
        .expect("the label is drawn")
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
    // Skill list: Brace, at the cost of 3 of the weapon's 20.
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
    assert!(line.contains("3 dur") && line.contains("20/"), "{line}");
    step(&mut s, &mut c, &[Action::Confirm]);
    assert!(matches!(s.mode(), Mode::Idle { .. }), "{:?}", s.mode());
    let after = shown_def(&mut c, s.state(), UnitId(2));
    assert_eq!(after, before + 5);
    // The knight's weapon paid 3 durability.
    let knight = s.state().unit(UnitId(2)).unwrap();
    let slot = knight.loadout.equipped_slot().unwrap();
    assert_eq!(knight.loadout.weapon(slot).unwrap().durability_left, 17);
    // Player, enemy and other phases pass: at the start of the knight's own
    // phase Brace has ended.
    let mut next = s.state().clone();
    for _ in 0..3 {
        next.apply(&Command::EndPhase).unwrap();
    }
    assert_eq!(shown_def(&mut c, &next, UnitId(2)), before);
}

#[test]
fn a_skill_menu_entry_needs_a_usable_non_combat_active() {
    let mut c = ctx();
    // The Guard's Brace, weapon at 3: it can be paid (3 dur).
    let s = knight_menu(&mut c, 3);
    assert_eq!(entry_enabled(&s, MenuEntry::Skill), Some(true));
    // At 2 it can't: the entry is dimmed, and choosing it opens nothing.
    let mut s = knight_menu(&mut c, 2);
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
    // Knight at (4, 6): without an effect its glyphs sit on the terrain.
    let cell_of = |buf: &GlyphBuffer| {
        let mut found = None;
        for y in 0..30 {
            for x in 0..70 {
                if text(buf, x, y, 2).eq_ignore_ascii_case("Kn") {
                    found = Some((x, y));
                }
            }
        }
        found.expect("the knight is drawn")
    };
    let plain = render(&s, &c);
    let (x, y) = cell_of(&plain);
    let terrain_bg = plain.get(x, y).unwrap().bg;
    focus_entry(&mut s, &mut c, MenuEntry::Skill);
    step(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    let buf = render(&s, &c);
    let (x, y) = cell_of(&buf);
    let effect = c.palette.get(UiColor::Effect);
    assert_eq!(
        buf.get(x, y).unwrap().bg,
        terrain_bg.lerp(effect, EFFECT_BLEND)
    );
    assert_ne!(buf.get(x, y).unwrap().bg, terrain_bg);
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
    assert!(find("3 dur"), "{rows:?}");
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

#[test]
fn s_cycles_the_combat_actives_and_the_forecast_follows_core() {
    let mut c = ctx();
    let state = keen_skirmish(&c);
    let plain = state
        .preview_attack(UnitId(1), p(7, 2), &attack_with(UnitId(4), 0, None))
        .unwrap();
    let keen = state
        .preview_attack(
            UnitId(1),
            p(7, 2),
            &attack_with(UnitId(4), 0, Some(SkillId::new("keen_edge"))),
        )
        .unwrap();
    assert_ne!(plain.forecast, keen.forecast);
    let mut s = lord_on_brigand(&mut c, state);
    let numbers = |s: &BattleScreen, c: &Ctx| {
        let buf = render(s, c);
        let at = |dy| text(&buf, LEFT_X + 5, HP_ROW + dy, 3);
        (at(1), at(2), panel(&buf, SKILL_ROW))
    };
    let shown = |p: &trpg_core::AttackPreview| {
        (
            p.forecast.attacker.hit.to_string(),
            p.forecast.attacker.crit.to_string(),
        )
    };
    let (hit, crit, line) = numbers(&s, &c);
    assert_eq!((hit, crit), shown(&plain));
    assert_eq!(line, "");
    // One press: Keen Edge, its numbers and `(20 → 17)` under the title.
    step(&mut s, &mut c, &[Action::CursorDown]);
    let (hit, crit, line) = numbers(&s, &c);
    assert_eq!((hit, crit), shown(&keen));
    let (from, to) = keen.durability.unwrap();
    assert_eq!(to, from - 3);
    assert_eq!(line, format!("Keen Edge ({from} → {to})"));
    let Mode::Targeting(t) = s.mode() else {
        unreachable!()
    };
    assert_eq!(t.preview, keen);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
    // Another: none again.
    step(&mut s, &mut c, &[Action::CursorDown]);
    let (hit, crit, line) = numbers(&s, &c);
    assert_eq!((hit, crit), shown(&plain));
    assert_eq!(line, "");
}

#[test]
fn the_chosen_active_is_used_and_paid_for_when_attacking() {
    let mut c = ctx();
    let state = keen_skirmish(&c);
    let mut s = lord_on_brigand(&mut c, state);
    let before = s
        .state()
        .unit(UnitId(1))
        .unwrap()
        .loadout
        .weapon(0)
        .unwrap()
        .durability_left;
    step(&mut s, &mut c, &[Action::CursorDown, Action::Confirm]);
    let after = s
        .state()
        .unit(UnitId(1))
        .unwrap()
        .loadout
        .weapon(0)
        .unwrap()
        .durability_left;
    assert_eq!(after, before - 3);
}

#[test]
fn an_active_the_weapon_cannot_pay_for_is_skipped() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        units[0].pos = p(6, 2);
        learn(&c, &mut units[0], "keen_edge");
        units[0].loadout.weapons[0]
            .as_mut()
            .unwrap()
            .durability_left = 2;
    });
    let mut s = lord_on_brigand(&mut c, state);
    let Mode::Targeting(t) = s.mode() else {
        unreachable!()
    };
    let first = t.target();
    assert!(t.actives(s.state()).is_empty());
    // With nothing to choose, Down cycles the targets like Right.
    step(&mut s, &mut c, &[Action::CursorDown]);
    let Mode::Targeting(t) = s.mode() else {
        unreachable!()
    };
    assert_eq!(t.active, None);
    assert_ne!(t.target(), first);
}

#[test]
fn the_help_line_names_the_skill_key_only_when_an_active_can_be_chosen() {
    let mut c = ctx();
    let state = keen_skirmish(&c);
    let with = lord_on_brigand(&mut c, state);
    assert!(with.help(&c).contains("skill"), "{}", with.help(&c));
    // A lord without Keen Edge (Inspire isn't a combat active).
    let state = battle(&c, |units| units[0].pos = p(6, 2));
    let without = lord_on_brigand(&mut c, state);
    assert!(!without.help(&c).contains("skill"), "{}", without.help(&c));
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
    let raider_bg = |s: &BattleScreen, c: &Ctx| {
        let buf = render(s, c);
        let (x, y) = label_cell(&buf, "Ra");
        buf.get(x, y).unwrap().bg
    };
    let plain_bg = raider_bg(&s, &c);
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
    assert_ne!(raider_bg(&s, &c), plain_bg);
    // Left goes back around to the brigand, and right to the raider.
    step(&mut s, &mut c, &[Action::CursorLeft]);
    let Mode::SkillTarget(t) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(t.target(), UnitId(4));
    step(&mut s, &mut c, &[Action::CursorRight]);
    let buf = render(&s, &c);
    let message = text(&buf, 1, 30, 60);
    assert!(message.starts_with("Shove on Raider"), "{message}");
    assert!(message.contains("→"), "{message}");
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
fn a_cycle_through_two_actives_returns_to_none() {
    let mut c = ctx();
    let state = battle(&c, |units| {
        units[0].pos = p(6, 2);
        learn(&c, &mut units[0], "keen_edge");
        learn(&c, &mut units[0], "blade_flurry");
    });
    let mut s = lord_on_brigand(&mut c, state);
    let active = |s: &BattleScreen| {
        let Mode::Targeting(t) = s.mode() else {
            panic!("{:?}", s.mode());
        };
        t.active.clone().map(|a| a.0)
    };
    assert_eq!(active(&s), None);
    // Down goes forward: the first active in the unit's skill order first.
    let Mode::Targeting(t) = s.mode() else {
        unreachable!()
    };
    let order: Vec<String> = t.actives(s.state()).into_iter().map(|a| a.0).collect();
    assert_eq!(order.len(), 2);
    let mut seen = vec![];
    for _ in 0..3 {
        step(&mut s, &mut c, &[Action::CursorDown]);
        seen.push(active(&s));
    }
    assert_eq!(seen.len(), 3);
    assert!(seen[0].is_some() && seen[1].is_some() && seen[2].is_none());
    assert_eq!(seen[0].as_deref(), Some(order[0].as_str()));
    assert_eq!(seen[1].as_deref(), Some(order[1].as_str()));
    // Up goes back around the ring: none, then the last active.
    step(&mut s, &mut c, &[Action::CursorUp]);
    assert_eq!(active(&s), seen[1]);
    step(&mut s, &mut c, &[Action::CursorUp]);
    assert_eq!(active(&s), seen[0]);
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
