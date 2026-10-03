//! Tests of the AI phase playback (ticket 0502): the enemies act one by one
//! through the battle screen, only Confirm (faster) and Cancel (skip a
//! fight) do anything meanwhile, and the battle ends up exactly where
//! applying the AI's commands directly takes it.

use insta::assert_snapshot;
use trpg_core::{BattleState, Command, Phase, UnitId, next_command};

use super::ai_phase::PACING;
use super::testing::through_ai_phases;
use super::*;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::harness::{FRAME_DT, Harness};
use crate::screen::tests::ctx;

fn quick() -> BattleScreen {
    BattleScreen::new(quick_battle(&ctx().content).unwrap())
}

/// One frame of `dt` seconds with `actions`, Confirm held if `held`.
fn frame(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action], dt: f32, held: bool) {
    let held = if held { vec![Action::Confirm] } else { vec![] };
    s.update(c, &FrameInput::new(actions.to_vec(), dt, held));
}

fn press(s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]) {
    frame(s, c, actions, 0.0, false);
}

fn render(s: &BattleScreen, c: &Ctx) -> GlyphBuffer {
    let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank);
    s.draw(c, &mut buf);
    buf
}

/// The glyphs of every row of `buf`.
fn glyphs(buf: &GlyphBuffer) -> Vec<String> {
    (0..i32::from(CONSOLE_H))
        .map(|y| {
            (0..i32::from(CONSOLE_W))
                .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
                .collect()
        })
        .collect()
}

/// `state` after the rest of its AI phases, the AI's commands applied
/// directly (no screen): each [`next_command`], or `EndPhase` when it has
/// none, until the player phase is back or the battle is over.
fn played_directly(c: &Ctx, mut state: BattleState) -> (BattleState, Vec<Command>) {
    let mut sent = Vec::new();
    while state.phase() != Phase::Player && state.outcome().is_none() {
        let cmd = next_command(&state, &c.content.ai).unwrap_or(Command::EndPhase);
        state.apply(&cmd).unwrap();
        sent.push(cmd);
    }
    (state, sent)
}

/// The Quick Battle with the turn ended, the enemy banner closed: the
/// first enemy action starts.
fn enemy_phase(c: &mut Ctx) -> BattleScreen {
    let mut s = quick();
    press(&mut s, c, &[Action::EndTurn, Action::EndTurn]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    press(&mut s, c, &[Action::Confirm]);
    s
}

#[test]
fn nothing_acts_under_the_banner() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    let at_start = s.state().clone();
    frame(&mut s, &mut c, &[], 0.5, false);
    assert!(s.banner().is_some());
    assert_eq!(s.state(), &at_start);
    assert_eq!(s.mode(), &Mode::default());
}

#[test]
fn enemies_act_one_by_one_then_the_player_has_turn_two() {
    let mut c = ctx();
    let mut s = enemy_phase(&mut c);
    let enemies: Vec<UnitId> = s
        .state()
        .units()
        .iter()
        .filter(|u| u.faction == Faction::Enemy)
        .map(|u| u.id)
        .collect();
    // Each action shows on its own: its unit, then the next one's.
    let mut shown = Vec::new();
    for _ in 0..10_000 {
        if !s.ai_phase() {
            break;
        }
        if let Mode::AiAction(a) = s.mode()
            && shown.last() != Some(&a.unit())
        {
            // Only this unit has acted since the last one.
            shown.push(a.unit());
            let acted = s.state().units().iter().filter(|u| u.acted).count();
            assert_eq!(acted, shown.len(), "{shown:?}");
        }
        frame(&mut s, &mut c, &[], FRAME_DT, false);
    }
    shown.sort();
    assert_eq!(shown, enemies, "every enemy, one at a time");
    assert_eq!((s.state().turn(), s.state().phase()), (2, Phase::Player));
    let banner = s.banner().map(|b| b.kind);
    assert_eq!(
        banner,
        Some(banner::BannerKind::Phase {
            phase: Phase::Player,
            turn: 2
        })
    );
}

#[test]
fn an_action_pans_marks_the_unit_then_walks() {
    let mut c = ctx();
    let mut s = enemy_phase(&mut c);
    let Mode::AiAction(a) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let (unit, start) = (a.unit(), a.start());
    // The battle already holds the action; the map shows it as it was.
    let before = s.shown_units();
    let drawn = before.iter().find(|(u, _)| u.id == unit).unwrap();
    assert_eq!(drawn.0.pos, start);
    assert_ne!(s.state().unit(unit).unwrap().pos, start, "it moves");
    assert_eq!(s.cursor().pos, start);
    assert_eq!(s.help(&c), "hold f fast");
    // The cursor marks it (no pan: the small map is all in view), then it
    // walks (and the cursor goes).
    assert!((a.walk_start() - (PACING.highlight)).abs() < 1e-6);
    frame(&mut s, &mut c, &[], PACING.highlight / 2.0, false);
    let Mode::AiAction(a) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert!(a.shows_cursor());
    assert_eq!(s.scene(&c).cursor_tile(), Some(start));
    frame(&mut s, &mut c, &[], PACING.highlight, false);
    let Mode::AiAction(a) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert!(a.walking());
    // The cursor is gone.
    assert_eq!(s.scene(&c).cursor_tile(), None);
    // Then its combat plays, as the player's do.
    for _ in 0..100 {
        if !matches!(s.mode(), Mode::AiAction(_)) {
            break;
        }
        frame(&mut s, &mut c, &[], FRAME_DT, false);
    }
    assert!(matches!(s.mode(), Mode::Combat(_)), "{:?}", s.mode());
    assert_eq!(s.help(&c), "d skip · hold f fast");
}

/// An enemy mid-walk: the map shows everyone as before its action, it on
/// its way along its path.
#[test]
fn enemy_mid_move_snapshot() {
    let mut c = ctx();
    let mut s = enemy_phase(&mut c);
    for _ in 0..100 {
        if let Mode::AiAction(a) = s.mode()
            && a.walking()
            && a.walker_pos() != a.start()
        {
            break;
        }
        frame(&mut s, &mut c, &[], FRAME_DT, false);
    }
    let Mode::AiAction(a) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert!(a.walking() && a.walker_pos() != a.start());
    assert_snapshot!(render(&s, &c).to_snapshot(&c.palette));
}

#[test]
fn holding_confirm_is_faster_and_changes_nothing_else() {
    let mut c = ctx();
    let run = |c: &mut Ctx, held: bool| {
        let mut s = enemy_phase(c);
        let mut frames = 0;
        while s.ai_phase() {
            frame(&mut s, c, &[], FRAME_DT, held);
            frames += 1;
            assert!(frames < 10_000);
        }
        (s, frames)
    };
    let (slow, slow_frames) = run(&mut c, false);
    let (fast, fast_frames) = run(&mut c, true);
    assert_eq!(fast.state(), slow.state());
    assert_eq!(fast.history(), slow.history());
    // Everything but the fixed banner plays 4× as fast.
    assert!(
        fast_frames * 3 < slow_frames,
        "{fast_frames} vs {slow_frames}"
    );
}

#[test]
fn the_screen_ends_where_the_ai_commands_applied_directly_do() {
    let mut c = ctx();
    let start = quick();
    let mut direct = start.state().clone();
    direct.apply(&Command::EndPhase).unwrap();
    let (direct, sent) = played_directly(&c, direct);
    assert!(sent.len() > 1, "the enemies did something: {sent:?}");
    let mut s = enemy_phase(&mut c);
    through_ai_phases(&mut s, &mut c, 1.0);
    assert_eq!(s.state(), &direct);
    let mut commands = vec![Command::EndPhase];
    commands.extend(sent);
    assert_eq!(s.history().commands(), commands.as_slice());
}

#[test]
fn only_confirm_and_cancel_do_anything_while_the_ai_plays() {
    let mut c = ctx();
    let mut s = enemy_phase(&mut c);
    let (cursor, camera, mode) = (s.cursor().pos, s.camera(), s.mode().clone());
    let auto_end = s.auto_end();
    press(
        &mut s,
        &mut c,
        &[
            Action::CursorRight,
            Action::CursorDown,
            Action::Info,
            Action::NextUnit,
            Action::DangerZone,
            Action::ToggleAutoEnd,
            Action::Rewind,
            Action::EndTurn,
        ],
    );
    assert_eq!(s.cursor().pos, cursor);
    assert_eq!(s.camera(), camera);
    assert_eq!(s.mode(), &mode);
    assert_eq!(s.danger(), None);
    assert_eq!(s.auto_end(), auto_end);
    assert_eq!(s.toast(), None);
    assert!(s.rewind().is_none());
    // Confirm and Cancel don't skip the walk either.
    press(&mut s, &mut c, &[Action::Confirm, Action::Cancel]);
    assert_eq!(s.mode(), &mode);
}

#[test]
fn cancel_skips_an_enemy_fight() {
    let mut c = ctx();
    let mut s = enemy_phase(&mut c);
    for _ in 0..1000 {
        if matches!(s.mode(), Mode::Combat(_)) {
            break;
        }
        frame(&mut s, &mut c, &[], FRAME_DT, false);
    }
    let Mode::Combat(pb) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    let fighter = pb.bouts()[0].attacker.unit;
    assert!(pb.time() < pb.total());
    // Skipped at once: the next enemy's action starts.
    press(&mut s, &mut c, &[Action::Cancel]);
    let Mode::AiAction(a) = s.mode() else {
        panic!("{:?}", s.mode());
    };
    assert_ne!(a.unit(), fighter);
}

#[test]
fn the_cursor_and_camera_come_back_for_the_player_phase() {
    let mut c = ctx();
    let mut s = quick();
    press(&mut s, &mut c, &[Action::CursorRight, Action::CursorRight]);
    press(&mut s, &mut c, &[Action::CursorDown]);
    let (cursor, camera) = (s.cursor().pos, s.camera());
    press(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_ne!(s.cursor().pos, cursor, "on the acting enemy");
    through_ai_phases(&mut s, &mut c, 1.0);
    assert_eq!(s.state().phase(), Phase::Player);
    assert_eq!(s.cursor().pos, cursor);
    assert_eq!(s.camera(), camera);
}

/// Through the real game: end the turn, let time pass, and the screen
/// shows the battle the AI's commands applied directly give.
#[test]
fn harness_enemy_phase_matches_applying_the_ai_directly() {
    let c = ctx();
    let mut direct = quick().state().clone();
    direct.apply(&Command::EndPhase).unwrap();
    let (direct, _) = played_directly(&c, direct);
    let mut h = Harness::with_screen(Box::new(quick()));
    h.keys("Space Space");
    h.wait(30.0);
    let mut expected = Harness::with_screen(Box::new(BattleScreen::new(direct)));
    expected.wait(FRAME_DT);
    let shown = glyphs(h.game().buffer());
    assert!(!shown.iter().any(|r| r.contains("PHASE")), "banner gone");
    assert_eq!(shown, glyphs(expected.game().buffer()));
    // Sanity: the enemy phase did change the map.
    let untouched = Harness::with_screen(Box::new(quick()));
    assert_ne!(shown, glyphs(untouched.game().buffer()));
}
