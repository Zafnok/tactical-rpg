//! Tests of the player's settings in battle (ticket 0805): the animation
//! speed, the enemy phase speed and combat animations off. Whatever the
//! speeds, the battle ends up the same.

use trpg_core::Phase;

use super::testing::{skirmish, through_ai_phases};
use super::*;
use crate::harness::{FRAME_DT, Harness};
use crate::screen::tests::ctx;
use crate::settings::{AnimSpeed, EnemyPhaseSpeed, Settings};

/// The skirmish in the harness with the settings changed by `change`, and
/// the lord's sword attack on the brigand just confirmed.
fn fight(change: impl FnOnce(&mut Settings)) -> Harness {
    let state = skirmish(&ctx(), 20);
    let mut h = Harness::with_screen(Box::new(BattleScreen::new(state)));
    h.ctx_mut().change_settings(change).unwrap();
    // The lord one step right, Attack, the iron sword, the brigand.
    h.keys("f Right f").wait(0.5);
    h.keys("f f Right f");
    h
}

fn battle(h: &Harness) -> &BattleScreen {
    h.battle().expect("a battle")
}

fn in_combat(h: &Harness) -> bool {
    matches!(battle(h).mode(), Mode::Combat(_))
}

/// Frames the fight's playback takes from its confirm.
fn frames_of_fight(h: &mut Harness) -> usize {
    let mut frames = 0;
    while in_combat(h) {
        h.wait(FRAME_DT);
        frames += 1;
        assert!(frames < 2_000, "the playback never ended");
    }
    frames
}

#[test]
fn fast_animations_play_a_fight_twice_as_fast() {
    let mut normal = fight(|_| {});
    assert!(in_combat(&normal));
    let slow = frames_of_fight(&mut normal);
    assert!(slow > 60, "{slow} frames");
    let mut fast = fight(|s| s.anim_speed = AnimSpeed::Fast);
    let quick = frames_of_fight(&mut fast);
    assert!(quick.abs_diff(slow / 2) <= 3, "{quick} against {slow}");
    // The same battle either way.
    assert_eq!(battle(&fast).state(), battle(&normal).state());
}

#[test]
fn the_enemy_phase_speed_leaves_the_players_fights_alone() {
    let slow = frames_of_fight(&mut fight(|_| {}));
    let mut h = fight(|s| s.enemy_phase_speed = EnemyPhaseSpeed::Fast);
    assert_eq!(frames_of_fight(&mut h), slow);
}

#[test]
fn with_combat_animations_off_a_fight_is_over_at_once() {
    let mut normal = fight(|_| {});
    frames_of_fight(&mut normal);
    let h = fight(|s| s.combat_animations = false);
    // The frame that confirmed the attack skipped its playback.
    assert!(!in_combat(&h), "{:?}", battle(&h).mode());
    assert_eq!(battle(&h).state(), battle(&normal).state());
    assert!(battle(&h).state().units()[0].acted);
}

/// Frames the Quick Battle's first AI phases take with the settings
/// changed by `change`, and the battle after them.
fn ai_phases(change: impl FnOnce(&mut Settings)) -> (usize, BattleScreen) {
    let mut c = ctx();
    c.change_settings(change).unwrap();
    let mut s = BattleScreen::new(quick_battle(&c.content).unwrap());
    let frame = |s: &mut BattleScreen, c: &mut Ctx, actions: &[Action]| {
        s.update(c, &FrameInput::new(actions.to_vec(), 0.0, vec![]));
    };
    frame(&mut s, &mut c, &[Action::EndTurn, Action::EndTurn]);
    assert_eq!(s.state().phase(), Phase::Enemy);
    // The banner closed: the first enemy acts.
    frame(&mut s, &mut c, &[Action::Confirm]);
    let frames = through_ai_phases(&mut s, &mut c, 0.05);
    (frames, s)
}

#[test]
fn the_enemy_phase_plays_faster_at_fast_and_faster_still_with_fast_animations() {
    let (normal, at_normal) = ai_phases(|_| {});
    let (fast, at_fast) = ai_phases(|s| s.enemy_phase_speed = EnemyPhaseSpeed::Fast);
    let (both, at_both) = ai_phases(|s| {
        s.enemy_phase_speed = EnemyPhaseSpeed::Fast;
        s.anim_speed = AnimSpeed::Fast;
    });
    let (anim, _) = ai_phases(|s| s.anim_speed = AnimSpeed::Fast);
    // Banners between the phases keep their time, so not quite half.
    assert!(fast * 10 < normal * 7, "{fast} against {normal}");
    assert!(both * 10 < fast * 8, "{both} against {fast}");
    assert!(fast.abs_diff(anim) <= 2, "{fast} against {anim}");
    // The same battle whatever the speed.
    assert_eq!(at_fast.state(), at_normal.state());
    assert_eq!(at_both.state(), at_normal.state());
}

#[test]
fn with_combat_animations_off_the_enemies_fights_are_skipped_too() {
    let (normal, at_normal) = ai_phases(|_| {});
    let (off, at_off) = ai_phases(|s| s.combat_animations = false);
    assert!(off < normal, "{off} against {normal}");
    assert_eq!(at_off.state(), at_normal.state());
}

/// The settings are read every frame: a change made on the Options screen
/// over the battle holds as soon as the battle is back.
#[test]
fn a_setting_changed_mid_battle_holds_from_the_next_frame() {
    let mut c = ctx();
    let mut s = BattleScreen::new(quick_battle(&c.content).unwrap());
    let idle = FrameInput::new(vec![], 0.0, vec![]);
    s.update(&mut c, &idle);
    assert!(!s.auto_end());
    c.change_settings(|s| s.auto_end_turn = true).unwrap();
    s.update(&mut c, &idle);
    assert!(s.auto_end());
    assert!(s.status(&c).ends_with("auto-end: ON"));
}
