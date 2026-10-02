//! Tests of the battle notes on the battle screen (ticket 0411), on the
//! test chapter's battle (`assets/battles/test.ron`): a note about its
//! brigand (unit 3) and one about no unit.

use trpg_content::{TipTrigger, new_campaign};
use trpg_core::{Command, UnitId};

use super::notes::{BLINK_S, NOTES_TITLE};
use super::*;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::screen::tests::ctx;

/// The brigand the first note is about.
const BRIGAND: UnitId = UnitId(3);

/// The test battle as the flow starts it, and its start events.
fn test_battle(c: &Ctx) -> (BattleState, Vec<Event>) {
    let lead = LeadProfile::new(DEFAULT_NAME, LeadGender::Male);
    let campaign = new_campaign(&c.content, GameMode::Classic, lead);
    let def = &c.content.battles["test"];
    BattleState::new(campaign.battle_setup(def, &c.content.tables()))
}

fn started(c: &Ctx) -> BattleScreen {
    let (state, events) = test_battle(c);
    BattleScreen::start(state, &events)
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

fn shows(s: &BattleScreen, c: &Ctx, text: &str) -> bool {
    let buf = render(s, c);
    (0..i32::from(CONSOLE_H)).any(|y| {
        let row: String = (0..i32::from(CONSOLE_W))
            .map(|x| buf.get(x, y).map_or(' ', |cell| cell.glyph))
            .collect();
        row.contains(text)
    })
}

/// The colours (text, background) of the left cell of `unit`'s tile.
fn colours(s: &BattleScreen, c: &Ctx, unit: UnitId) -> (Rgb, Rgb) {
    let pos = s.state().unit(unit).unwrap().pos;
    let (x, y) = super::testing::tile_cell(s, c, pos).unwrap();
    let buf = render(s, c);
    let cell = buf.get(x, y).unwrap();
    (cell.fg, cell.bg)
}

#[test]
fn the_notes_open_only_for_a_battle_just_started_that_has_some() {
    let c = ctx();
    let (state, events) = test_battle(&c);
    assert_eq!(state.battle_notes().len(), 2);
    assert_eq!(state.battle_notes()[0].units, [BRIGAND]);
    assert!(BattleScreen::start(state.clone(), &events).notes_open());
    // A screen on a battle already under way (a debug view, a test).
    assert!(!BattleScreen::new(state).notes_open());
    // No notes: no box.
    let quick = quick_battle(&c.content).unwrap();
    assert!(quick.battle_notes().is_empty());
    let s = BattleScreen::start(quick, &[]);
    assert!(!s.notes_open());
    assert!(!shows(&s, &c, NOTES_TITLE));
}

#[test]
fn a_battle_decided_at_its_start_skips_the_notes() {
    let c = ctx();
    let lead = LeadProfile::new(DEFAULT_NAME, LeadGender::Male);
    let campaign = new_campaign(&c.content, GameMode::Classic, lead);
    let def = &c.content.battles["test"];
    let mut setup = campaign.battle_setup(def, &c.content.tables());
    setup.objective = trpg_core::Objective::Rout { turn_limit: None };
    setup.units.retain(|u| u.faction == Faction::Player);
    let (state, events) = BattleState::new(setup);
    assert!(state.outcome().is_some());
    assert!(!state.battle_notes().is_empty());
    let s = BattleScreen::start(state, &events);
    assert!(!s.notes_open());
    assert!(!shows(&s, &c, NOTES_TITLE));
}

#[test]
fn only_confirm_closes_the_notes() {
    let mut c = ctx();
    let mut s = started(&c);
    assert!(shows(&s, &c, NOTES_TITLE));
    assert_eq!(s.help(&c), "f close");
    let cursor = s.cursor().pos;
    for action in [
        Action::Cancel,
        Action::CursorRight,
        Action::Info,
        Action::EndTurn,
        Action::Rewind,
        Action::DangerZone,
        Action::ToggleAutoEnd,
        Action::NextUnit,
    ] {
        frame(&mut s, &mut c, &[action], 0.0);
        assert!(s.notes_open(), "{action:?}");
        assert_eq!(s.mode(), &Mode::default(), "{action:?}");
    }
    assert_eq!(s.cursor().pos, cursor);
    assert!(s.danger().is_none() && !s.auto_end() && s.rewind().is_none());
    // Confirm closes the box and does nothing else (the cursor is on the
    // lead: a second Confirm would select it).
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(!s.notes_open());
    assert_eq!(s.mode(), &Mode::default());
    assert!(!shows(&s, &c, NOTES_TITLE));
    assert_ne!(s.help(&c), "f close");
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(matches!(s.mode(), Mode::Selected(_)), "{:?}", s.mode());
}

#[test]
fn a_turn_one_scene_plays_after_the_notes() {
    let c = ctx();
    let (state, mut events) = test_battle(&c);
    events.push(Event::SceneTriggered {
        scene: "test_intro".into(),
    });
    let mut s = BattleScreen::start(state, &events);
    let mut c = c;
    assert!(s.notes_open());
    assert_eq!(s.queued_scene(), None);
    assert_eq!(frame(&mut s, &mut c, &[], 1.0), "None");
    assert!(s.notes_open());
    // Closing the notes starts the scene.
    let pushed = frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(pushed.starts_with("Push"), "{pushed}");
    assert!(!s.notes_open());
}

#[test]
fn the_start_tip_waits_for_the_notes() {
    let mut c = ctx();
    c.tips_enabled = true;
    let mut s = started(&c);
    frame(&mut s, &mut c, &[], 0.0);
    assert!(s.notes_open());
    assert_eq!(s.shown_tip(), None);
    // The Confirm that closes the notes doesn't also close the tip.
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(!s.notes_open());
    assert_eq!(s.shown_tip(), Some(TipTrigger::FirstBattleStart));
}

#[test]
fn the_noted_unit_blinks_while_the_notes_are_up() {
    let mut c = ctx();
    let mut s = started(&c);
    let lead = UnitId(1);
    let inverted = colours(&s, &c, BRIGAND);
    let lead_plain = colours(&s, &c, lead);
    // On for BLINK_S, off for BLINK_S, on again.
    frame(&mut s, &mut c, &[], BLINK_S * 0.9);
    assert_eq!(colours(&s, &c, BRIGAND), inverted);
    frame(&mut s, &mut c, &[], BLINK_S * 0.2);
    let plain = colours(&s, &c, BRIGAND);
    assert_eq!(plain, (inverted.1, inverted.0), "the colours swap");
    assert_ne!(plain, inverted);
    frame(&mut s, &mut c, &[], BLINK_S);
    assert_eq!(colours(&s, &c, BRIGAND), inverted);
    // A unit no note is about never blinks.
    assert_eq!(colours(&s, &c, lead), lead_plain);
    frame(&mut s, &mut c, &[], BLINK_S);
    assert_eq!(colours(&s, &c, BRIGAND), plain);
    assert_eq!(colours(&s, &c, lead), lead_plain);
    // Closed: plain, whatever the time.
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert_eq!(colours(&s, &c, BRIGAND), plain);
    frame(&mut s, &mut c, &[], BLINK_S * 2.0);
    assert_eq!(colours(&s, &c, BRIGAND), plain);
    assert_eq!(colours(&s, &c, lead), lead_plain);
    // The Objective page lists the notes: the blink starts over, on.
    frame(
        &mut s,
        &mut c,
        &[Action::Cancel, Action::CursorDown, Action::Confirm],
        0.0,
    );
    assert_eq!(s.mode(), &Mode::Objective);
    assert!(shows(&s, &c, notes::NOTES_HEADING));
    assert_eq!(colours(&s, &c, BRIGAND), inverted);
    frame(&mut s, &mut c, &[], BLINK_S);
    assert_eq!(colours(&s, &c, BRIGAND), plain);
    frame(&mut s, &mut c, &[Action::Cancel], BLINK_S);
    assert_eq!(colours(&s, &c, BRIGAND), plain);
}

#[test]
fn a_command_behind_the_notes_waits_for_them() {
    let mut c = ctx();
    let mut s = started(&c);
    s.send(&Command::EndPhase);
    frame(&mut s, &mut c, &[], 0.5);
    assert!(s.notes_open());
    assert!(s.banner().is_none());
    assert!(s.ai_phase(), "the AI waits too");
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(s.banner().is_some());
}

/// The map scene (ADR-0038) while the notes are up: the unit a note is
/// about is highlighted in the on half of its blink, and no other unit is.
#[test]
fn the_scene_highlights_the_noted_unit_as_it_blinks() {
    let mut c = ctx();
    let mut s = started(&c);
    assert!(s.notes_open());
    let highlighted = |s: &BattleScreen, c: &Ctx| -> Vec<UnitId> {
        let units = s.scene(c).units;
        let lit = units.iter().filter(|u| u.highlight);
        lit.map(|u| u.id).collect()
    };
    assert_eq!(highlighted(&s, &c), [BRIGAND]);
    // Off for the second half of the blink, then on again.
    frame(&mut s, &mut c, &[], BLINK_S * 1.1);
    assert!(highlighted(&s, &c).is_empty());
    frame(&mut s, &mut c, &[], BLINK_S);
    assert_eq!(highlighted(&s, &c), [BRIGAND]);
    // The notes closed: no unit is picked out.
    frame(&mut s, &mut c, &[Action::Confirm], 0.0);
    assert!(!s.notes_open());
    assert!(highlighted(&s, &c).is_empty());
}
