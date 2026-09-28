//! Tests of the battle screen around the move loop (ticket 0405): the
//! danger zone, the info screen, the map menu, ending the turn, auto-end,
//! and the phase and outcome banners.

use insta::assert_snapshot;
use trpg_content::FontAtlasDef;
use trpg_core::{Objective, Outcome, UnitAction};

use super::banner::{BannerKind, PHASE_BANNER_S};
use super::testing::{battle_with, skirmish, wait};
use super::*;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::screen::tests::ctx;

fn quick() -> BattleScreen {
    BattleScreen::new(quick_battle(&ctx().content).unwrap())
}

fn render(screen: &BattleScreen, c: &Ctx) -> GlyphBuffer {
    let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank);
    screen.draw(c, &mut buf);
    buf
}

/// One frame of `dt` seconds with `actions`; the screen's answer.
fn frame(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], dt: f32) -> String {
    format!(
        "{:?}",
        s.update(c, &FrameInput::new(actions.to_vec(), dt, vec![]))
    )
}

/// One instant frame with `actions`.
fn press(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) -> String {
    frame(s, c, actions, 0.0)
}

/// The text of row `y`, trimmed.
fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(CONSOLE_W))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Whether some row of `buf` contains `text`.
fn shows(buf: &GlyphBuffer, text: &str) -> bool {
    (0..i32::from(CONSOLE_H)).any(|y| row(buf, y).contains(text))
}

/// Every glyph of `buf` is in the game's font.
fn assert_in_font(buf: &GlyphBuffer) {
    let font = FontAtlasDef::load().unwrap_or_default();
    for y in 0..i32::from(CONSOLE_H) {
        for x in 0..i32::from(CONSOLE_W) {
            let g = buf.get(x, y).unwrap().glyph;
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
    }
}

/// Player unit `index` of the screen's battle waits where it stands.
fn wait_unit(s: &mut BattleScreen, index: usize) {
    let u = &s.state().units()[index];
    let cmd = Command::Act {
        unit: u.id,
        dest: u.pos,
        action: UnitAction::Wait,
    };
    s.apply(&cmd);
}

fn phase_banner(s: &BattleScreen) -> Option<(Phase, u32)> {
    match s.banner()?.kind {
        BannerKind::Phase { phase, turn } => Some((phase, turn)),
        BannerKind::Outcome(_) => None,
    }
}

// Danger zone.

#[test]
fn the_danger_zone_is_the_cores_danger_zone_tinted_under_the_ranges() {
    let mut c = ctx();
    let mut s = quick();
    let plain = render(&s, &c);
    assert_eq!(s.danger(), None);
    press(&mut s, &mut c, &[Action::DangerZone]);
    let state = s.state();
    let expected = danger_zone(
        state.map(),
        state.terrain(),
        state.classes(),
        state.units(),
        Faction::Player,
        |u| u.attack_ranges(state.classes(), state.items(), state.spells()),
    )
    .unwrap();
    assert!(!expected.is_empty());
    assert_eq!(s.danger(), Some(&expected));
    // Every map tile is tinted `danger_zone` exactly if it is in the zone.
    let buf = render(&s, &c);
    let danger = c.palette.get(UiColor::DangerZone);
    let tiles = &s.state().map().tiles;
    for y in 0..i32::from(tiles.height()) {
        for x in 0..i32::from(tiles.width()) {
            let pos = Pos::new(x, y);
            let (cx, cy) = tile_to_cell(pos, &s.camera()).unwrap();
            let was = plain.get(cx, cy).unwrap().bg;
            let want = if expected.contains(pos) {
                was.lerp(danger, OVERLAY_BLEND)
            } else {
                was
            };
            assert_eq!(buf.get(cx, cy).unwrap().bg, want, "{pos:?}");
        }
    }
    assert!(
        s.status(&c).starts_with("w danger zone: ON"),
        "{}",
        s.status(&c)
    );
    // A selected unit's ranges go over it.
    press(&mut s, &mut c, &[Action::Confirm]);
    let buf = render(&s, &c);
    let Mode::Selected(sel) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let both = sel.moves.iter().find(|&p| expected.contains(p)).unwrap();
    let (cx, cy) = tile_to_cell(both, &s.camera()).unwrap();
    let under = plain.get(cx, cy).unwrap().bg.lerp(danger, OVERLAY_BLEND);
    let blue = c.palette.get(UiColor::MoveRange);
    assert_eq!(buf.get(cx, cy).unwrap().bg, under.lerp(blue, OVERLAY_BLEND));
    // The toggle works while a unit is selected, too.
    press(&mut s, &mut c, &[Action::DangerZone]);
    assert_eq!(s.danger(), None);
    assert!(s.status(&c).starts_with("w danger zone: OFF"));
}

#[test]
fn the_danger_zone_follows_the_battle_and_stays_on_across_turns() {
    let mut c = ctx();
    let mut s = BattleScreen::new(skirmish(&c, 1));
    press(&mut s, &mut c, &[Action::DangerZone]);
    let before = s.danger().cloned().unwrap();
    // The lord kills the first brigand: the zone shrinks at once.
    s.apply(&Command::Act {
        unit: UnitId(1),
        dest: Pos::new(7, 2),
        action: UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: None,
            art: None,
        },
    });
    assert!(s.state().unit(UnitId(4)).is_none());
    let after = s.danger().cloned().unwrap();
    assert_ne!(after, before);
    assert_eq!(after, danger_tiles(s.state()));
    // Through the next turn: still shown.
    frame(&mut s, &mut c, &[], 30.0);
    press(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    for _ in 0..3 {
        frame(&mut s, &mut c, &[], 1.0);
    }
    assert_eq!(s.state().turn(), 2);
    assert_eq!(s.danger(), Some(&danger_tiles(s.state())));
}

/// The danger zone of the Quick Battle's three enemies.
#[test]
fn danger_zone_snapshot() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::DangerZone]);
    let buf = render(&s, &c);
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

// Info screen.

#[test]
fn info_opens_on_any_unit_cycles_its_faction_and_closes_on_the_last_one() {
    let mut c = ctx();
    let mut s = quick();
    let info = |s: &BattleScreen| match s.mode() {
        Mode::Info { unit } => Some(unit.0),
        _ => None,
    };
    // On the lord.
    press(&mut s, &mut c, &[Action::Info]);
    assert_eq!(info(&s), Some(1));
    assert_eq!(s.help(&c), "s next unit · a previous · d close");
    // Players in reading order: archer (2, 4), lord (3, 5), knight (4, 6).
    let mut visit = |a| {
        press(&mut s, &mut c, &[a]);
        info(&s)
    };
    assert_eq!(visit(Action::CursorDown), Some(2));
    assert_eq!(visit(Action::NextUnit), Some(3));
    assert_eq!(visit(Action::PrevUnit), Some(2));
    assert_eq!(visit(Action::CursorUp), Some(1));
    assert_eq!(visit(Action::CursorUp), Some(3));
    // Other keys do nothing; Cancel closes it on the archer.
    assert_eq!(visit(Action::CursorLeft), Some(3));
    assert_eq!(visit(Action::Cancel), None);
    assert_eq!(s.cursor().pos, Pos::new(2, 4));
    // An enemy: the enemies, in reading order: raider (7, 1), brigands
    // (8, 2) and (7, 4). Info closes it too.
    s.cursor.jump(Pos::new(8, 2));
    press(&mut s, &mut c, &[Action::Info, Action::NextUnit]);
    assert_eq!(info(&s), Some(5));
    press(&mut s, &mut c, &[Action::Info]);
    assert_eq!((info(&s), s.cursor().pos), (None, Pos::new(7, 4)));
    // Nothing on an empty tile.
    s.cursor.jump(Pos::new(6, 5));
    press(&mut s, &mut c, &[Action::Info]);
    assert_eq!(s.mode(), &Mode::default());
}

#[test]
fn info_shows_stats_with_caps_and_the_loadout() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::Info]);
    let buf = render(&s, &c);
    let lord = s.state().unit(UnitId(1)).unwrap().clone();
    let class = s.state().classes().get(&lord.class).unwrap().clone();
    let text = |x: i32, y: i32, n: i32| -> String {
        (x..x + n).map(|x| buf.get(x, y).unwrap().glyph).collect()
    };
    assert_eq!(text(29, 1, 9), "Test Lord");
    let str_line = format!("Str {:>3}/{}", lord.stats.str, class.caps.str);
    assert_eq!(text(29, 8, 20).trim(), str_line);
    // The cap is dim.
    let dim = c.palette.get(UiColor::TextDim);
    assert_eq!(buf.get(36, 8).unwrap().fg, dim);
    assert!(shows(&buf, "Weapons"));
    assert!(shows(&buf, "Weapon ranks"));
    assert!(shows(&buf, "portrait"));
    let equipped = lord.loadout.equipped_slot().unwrap();
    let def = &lord.loadout.weapon(equipped).unwrap().def;
    let name = &s.state().items().weapon(def).unwrap().name;
    assert!(shows(&buf, &format!("E {name}")));
    assert_in_font(&buf);
}

/// The lord's info screen.
#[test]
fn info_screen_snapshot() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::Info]);
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

// Map menu.

#[test]
fn the_map_menu_opens_from_cancel_an_empty_tile_or_the_menu_key() {
    let mut c = ctx();
    let is_menu = |s: &BattleScreen| matches!(s.mode(), Mode::MapMenu { .. });
    for (at, key) in [
        (Pos::new(3, 5), Action::Cancel),
        (Pos::new(6, 5), Action::Confirm),
        (Pos::new(3, 5), Action::Menu),
    ] {
        let mut s = quick();
        s.cursor.jump(at);
        press(&mut s, &mut c, &[key]);
        assert!(is_menu(&s), "{key:?}");
        assert_eq!(s.help(&c), "arrows choose · f confirm · d back");
        press(&mut s, &mut c, &[Action::Cancel]);
        assert_eq!(s.mode(), &Mode::default());
    }
}

#[test]
fn units_jumps_the_cursor_to_the_chosen_unit() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::Cancel, Action::Confirm]);
    assert!(matches!(s.mode(), Mode::UnitList { .. }), "{:?}", s.mode());
    let buf = render(&s, &c);
    assert!(shows(&buf, "Test Archer  HP 17/17  ready"));
    // Cancel: back to the menu, on Units.
    press(&mut s, &mut c, &[Action::Cancel]);
    let Mode::MapMenu { menu, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(menu.focus(), 0);
    // The archer, third.
    press(
        &mut s,
        &mut c,
        &[
            Action::Confirm,
            Action::CursorDown,
            Action::CursorDown,
            Action::Confirm,
        ],
    );
    assert_eq!(s.mode(), &Mode::default());
    assert_eq!(s.cursor().pos, Pos::new(2, 4));
}

#[test]
fn objective_shows_the_goal_and_the_turn() {
    let mut c = ctx();
    let mut s = quick();
    press(
        &mut s,
        &mut c,
        &[Action::Cancel, Action::CursorDown, Action::Confirm],
    );
    assert_eq!(s.mode(), &Mode::Objective);
    assert_eq!(s.help(&c), "d back");
    let buf = render(&s, &c);
    assert!(shows(&buf, "Rout the enemy"));
    assert!(shows(&buf, "Turn 1"));
    // Other keys do nothing; Cancel goes back to the menu, on Objective.
    press(&mut s, &mut c, &[Action::CursorDown, Action::Cancel]);
    let Mode::MapMenu { menu, .. } = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_eq!(menu.focus(), 1);
    // Down skips the disabled Options and Suspend to End Turn.
    press(&mut s, &mut c, &[Action::CursorDown, Action::Confirm]);
    assert_eq!(s.mode(), &Mode::EndTurnPrompt { ready: 3 });
}

/// The map menu beside the cursor on the lord, focused on `Units`.
#[test]
fn map_menu_snapshot() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::Cancel]);
    let buf = render(&s, &c);
    assert_in_font(&buf);
    assert_snapshot!(buf.to_snapshot(&c.palette));
}

// Ending the turn.

#[test]
fn end_turn_asks_while_units_are_ready_and_space_again_ends_it() {
    let mut c = ctx();
    let mut s = quick();
    wait_unit(&mut s, 2);
    press(&mut s, &mut c, &[Action::EndTurn]);
    assert_eq!(s.mode(), &Mode::EndTurnPrompt { ready: 2 });
    let buf = render(&s, &c);
    assert!(shows(&buf, "End turn with 2 units ready?"));
    assert!(shows(&buf, "f yes / d no"));
    assert_eq!(s.help(&c), "Space yes · f yes · d no");
    // Cancel backs out; other keys wait.
    press(&mut s, &mut c, &[Action::Info, Action::Cancel]);
    assert_eq!(s.mode(), &Mode::default());
    assert_eq!(s.state().phase(), Phase::Player);
    // Double-tap Space: the enemy phase starts, with its banner.
    press(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    assert_eq!(phase_banner(&s), Some((Phase::Enemy, 1)));
    // Confirm on the prompt works too.
    let mut s = quick();
    press(&mut s, &mut c, &[Action::EndTurn, Action::Confirm]);
    assert_eq!(s.state().phase(), Phase::Enemy);
}

#[test]
fn with_no_unit_ready_end_turn_ends_at_once() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::ToggleAutoEnd]);
    for i in 0..3 {
        wait_unit(&mut s, i);
    }
    press(&mut s, &mut c, &[]);
    assert_eq!(s.state().phase(), Phase::Player, "auto-end is off");
    press(&mut s, &mut c, &[Action::EndTurn]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    // From the map menu, too.
    let mut s = quick();
    press(&mut s, &mut c, &[Action::ToggleAutoEnd]);
    for i in 0..3 {
        wait_unit(&mut s, i);
    }
    press(
        &mut s,
        &mut c,
        &[Action::Cancel, Action::CursorUp, Action::Confirm],
    );
    assert_eq!(s.state().phase(), Phase::Enemy);
}

#[test]
fn auto_end_ends_the_phase_when_the_last_unit_acts_and_toggles() {
    let mut c = ctx();
    let mut s = quick();
    assert!(s.auto_end());
    assert!(s.status(&c).ends_with("Shift+Space auto-end: ON"));
    wait_unit(&mut s, 0);
    wait_unit(&mut s, 1);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.state().phase(), Phase::Player);
    wait_unit(&mut s, 2);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    // Off: a message for a moment, and the last unit's action doesn't end
    // the phase.
    let mut s = quick();
    press(&mut s, &mut c, &[Action::ToggleAutoEnd]);
    assert!(!s.auto_end());
    assert_eq!(s.toast(), Some("Auto-end: OFF"));
    let buf = render(&s, &c);
    assert!(row(&buf, HELP_BAR.y).starts_with("Auto-end: OFF"));
    assert!(s.status(&c).ends_with("auto-end: OFF"));
    frame(&mut s, &mut c, &[], TOAST_S / 2.0);
    assert!(s.toast().is_some());
    frame(&mut s, &mut c, &[], TOAST_S / 2.0);
    assert_eq!(s.toast(), None);
    for i in 0..3 {
        wait_unit(&mut s, i);
    }
    press(&mut s, &mut c, &[]);
    assert_eq!(s.state().phase(), Phase::Player);
    // Turned back on with everyone done: nothing until the turn is ended.
    press(&mut s, &mut c, &[Action::ToggleAutoEnd]);
    assert_eq!(s.toast(), Some("Auto-end: ON"));
    frame(&mut s, &mut c, &[], 1.0);
    assert_eq!(s.state().phase(), Phase::Player);
}

#[test]
fn auto_end_waits_for_the_combat_to_play() {
    let mut c = ctx();
    let mut state = skirmish(&c, 20);
    // Only the lord is left to act.
    wait(&mut state, 1);
    wait(&mut state, 2);
    let mut s = BattleScreen::new(state);
    s.apply(&Command::Act {
        unit: UnitId(1),
        dest: Pos::new(7, 2),
        action: UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: None,
            art: None,
        },
    });
    press(&mut s, &mut c, &[]);
    assert!(matches!(s.mode(), Mode::Combat(_)));
    assert_eq!(s.state().phase(), Phase::Player);
    frame(&mut s, &mut c, &[], 30.0);
    press(&mut s, &mut c, &[]);
    assert_eq!(s.state().phase(), Phase::Enemy);
}

// Banners.

#[test]
fn phase_banners_close_after_a_second_or_on_confirm_and_the_enemy_phase_passes() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    assert_eq!(phase_banner(&s), Some((Phase::Enemy, 1)));
    assert_eq!(s.help(&c), "f skip");
    // Keys other than Confirm are ignored.
    let before = s.cursor().pos;
    press(&mut s, &mut c, &[Action::CursorRight, Action::Cancel]);
    assert_eq!(s.cursor().pos, before);
    assert_eq!(phase_banner(&s), Some((Phase::Enemy, 1)));
    frame(&mut s, &mut c, &[], PHASE_BANNER_S * 0.9);
    assert_eq!(phase_banner(&s), Some((Phase::Enemy, 1)));
    // Closed: the enemy phase ends at once (no AI yet), the player's turn
    // 2 starts.
    frame(&mut s, &mut c, &[], PHASE_BANNER_S * 0.2);
    assert_eq!(phase_banner(&s), Some((Phase::Player, 2)));
    assert_eq!((s.state().turn(), s.state().phase()), (2, Phase::Player));
    let buf = render(&s, &c);
    assert!(shows(&buf, "PLAYER PHASE"));
    assert!(shows(&buf, "Turn 2"));
    assert_in_font(&buf);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.banner(), None);
    assert_eq!(s.ready_units().len(), 3);
}

/// `PLAYER PHASE` on turn 2.
#[test]
fn phase_banner_snapshot() {
    let mut c = ctx();
    let mut s = quick();
    press(
        &mut s,
        &mut c,
        &[Action::EndTurn, Action::EndTurn, Action::Confirm],
    );
    assert_eq!(phase_banner(&s), Some((Phase::Player, 2)));
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn victory_shows_after_the_combat_and_confirm_leaves() {
    let mut c = ctx();
    let quick = quick_battle(&c.content).unwrap();
    let mut units = quick.units().to_vec();
    units.retain(|u| u.id == UnitId(1) || u.id == UnitId(4));
    units[0].pos = Pos::new(7, 2);
    units[1].hp = 1;
    let rout = Objective::Rout { turn_limit: None };
    let mut s = BattleScreen::new(battle_with(&c, quick.map().clone(), units, rout));
    s.apply(&Command::Act {
        unit: UnitId(1),
        dest: Pos::new(7, 2),
        action: UnitAction::Attack {
            target: UnitId(4),
            slot: 0,
            active: None,
            art: None,
        },
    });
    assert_eq!(s.state().outcome(), Some(Outcome::Victory));
    // Not over the combat.
    assert_eq!(s.banner(), None);
    frame(&mut s, &mut c, &[], 30.0);
    let won = s.banner().map(|b| b.kind);
    assert_eq!(won, Some(BannerKind::Outcome(Outcome::Victory)));
    // It stays until confirmed.
    assert_eq!(frame(&mut s, &mut c, &[], 10.0), "None");
    assert_eq!(s.help(&c), "f continue");
    let buf = render(&s, &c);
    assert!(shows(&buf, "VICTORY"));
    assert_in_font(&buf);
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "Pop");
}

#[test]
fn missing_the_turn_limit_is_a_defeat() {
    let mut c = ctx();
    let quick = quick_battle(&c.content).unwrap();
    let rout = Objective::Rout {
        turn_limit: Some(1),
    };
    let state = battle_with(&c, quick.map().clone(), quick.units().to_vec(), rout);
    let mut s = BattleScreen::new(state);
    press(
        &mut s,
        &mut c,
        &[Action::Cancel, Action::CursorDown, Action::Confirm],
    );
    assert!(shows(&render(&s, &c), "Turn 1/1"));
    press(&mut s, &mut c, &[Action::Cancel, Action::Cancel]);
    press(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    assert_eq!(phase_banner(&s), Some((Phase::Enemy, 1)));
    assert!(shows(&render(&s, &c), "Turn 1/1"));
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "None");
    let lost = s.banner().map(|b| b.kind);
    assert_eq!(lost, Some(BannerKind::Outcome(Outcome::Defeat)));
    assert!(shows(&render(&s, &c), "DEFEAT"));
    // The map menu can't end a turn any more; Confirm leaves.
    assert!(!map_menu::MapEntry::EndTurn.enabled(s.state()));
    assert_eq!(press(&mut s, &mut c, &[Action::Confirm]), "Pop");
}
