use trpg_core::{Command, LeadGender, LeadProfile, Pos, StatKind, UnitAction, UnitId};

use super::*;
use crate::game::Game;
use crate::harness::Harness;
use crate::input::Action::Cancel;
use crate::screen::tests::ctx;
use crate::screens::battle::progress::stat_row;

/// Playtime counts the seconds since the campaign began, carrying on
/// from what it had (a loaded campaign, 0802).
#[test]
fn playtime_carries_on_from_the_campaigns() {
    let mut c = ctx();
    c.clock_s = 10.0;
    let lead = LeadProfile::new("Mara", LeadGender::Female);
    let mut campaign = trpg_content::new_campaign(&c.content, GameMode::Casual, lead.clone());
    campaign.playtime_s = 100;
    let mut flow = FlowScreen::new_game();
    flow.begin(&mut c, campaign);
    assert_eq!(c.lead, lead);
    c.clock_s = 12.5;
    flow.count_playtime(&c);
    assert_eq!(flow.campaign().map(|c| c.playtime_s), Some(102));
}

fn update(flow: &mut FlowScreen, c: &mut Ctx, actions: &[crate::input::Action]) {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    flow.update(c, &input);
}

/// Nick (0408): after a defeat, `Retry Battle` goes back to Preparations
/// with the loadouts and pack as the player left them.
#[test]
fn retry_goes_back_to_preparations_as_they_were_left() {
    use crate::input::Action::{Confirm, CursorLeft, CursorRight};
    let mut c = ctx();
    let mut flow = FlowScreen::quick_battle(&mut c).unwrap_or_else(|| panic!("no quick battle"));
    assert_eq!(flow.name(), "preparations");
    assert!(flow.battle().is_none());
    // Pack an Elixir, then Fight!.
    update(&mut flow, &mut c, &[CursorRight, Confirm, Confirm]);
    update(&mut flow, &mut c, &[Cancel, CursorRight, Confirm]);
    assert_eq!(flow.name(), "battle");
    assert!(flow.preparations().is_none());
    let elixir = [trpg_core::ItemId::new("elixir")];
    let pack = |f: &FlowScreen| f.battle().map(|b| b.state().pack().items.clone());
    assert_eq!(pack(&flow).as_deref(), Some(&elixir[..]));
    // A defeat's Game Over, then Retry.
    flow.stage = Stage::GameOver(GameOverScreen::new());
    update(&mut flow, &mut c, &[Confirm]);
    assert_eq!(flow.name(), "preparations");
    let prep = flow.preparations().unwrap_or_else(|| panic!("no prep"));
    assert_eq!(prep.setup().pack.items, elixir);
    assert_eq!(prep.setup().stock.count(&elixir[0]), 1);
    // Fight! again: the same battle.
    update(&mut flow, &mut c, &[CursorLeft, Confirm]);
    assert_eq!(pack(&flow).as_deref(), Some(&elixir[..]));
}

/// A battle without Preparations starts straight away, with its default
/// pack.
#[test]
fn a_battle_without_preparations_starts_at_once() {
    let mut c = ctx();
    let lead = LeadProfile::new("Mara", LeadGender::Female);
    let mut campaign = trpg_content::new_campaign(&c.content, GameMode::Casual, lead);
    campaign.chapter = "test".into();
    let mut flow = FlowScreen::new_game();
    flow.begin(&mut c, campaign);
    // Skip the intro scene: Cancel, then Confirm.
    update(&mut flow, &mut c, &[Cancel]);
    update(&mut flow, &mut c, &[crate::input::Action::Confirm]);
    assert_eq!(flow.name(), "battle");
    let pack = flow.battle().map(|b| b.state().pack().items.len());
    assert_eq!(pack, Some(2));
}

/// Whether some row of the screen contains `text`.
fn shows(h: &Harness, text: &str) -> bool {
    let buf = h.game().buffer();
    (0..32).any(|y| {
        (0..100)
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect::<String>()
            .contains(text)
    })
}

/// Acceptance (0810): a level up caused by the rewind bonus shows 0602's
/// level-up screen after the results, with the numbers the roster got, and
/// waits for its own press before the victory scene.
#[test]
fn a_level_up_from_the_rewind_bonus_shows_the_level_up_screen() {
    let mut c = ctx();
    let lead = LeadProfile::new("Mara", LeadGender::Female);
    let mut campaign = trpg_content::new_campaign(&c.content, GameMode::Classic, lead);
    // Three unused charges (21 EXP) level her.
    campaign.roster[0].exp = 85;
    let before = campaign.roster[0].clone();
    let mut flow = FlowScreen::new_game();
    flow.begin(&mut c, campaign);
    let mut h = Harness::from_game(Game::new(c, Box::new(flow)));
    // Skip the intro, close the battle notes and turn 1's banner; the lead
    // seizes the fort.
    h.keys("d f f f");
    assert_eq!(h.top_screen(), "battle");
    let seize = Command::Act {
        unit: UnitId(1),
        dest: Pos::new(5, 5),
        action: UnitAction::Seize,
    };
    let battle = h.flow_mut().and_then(FlowScreen::battle_mut);
    battle.unwrap_or_else(|| panic!("no battle")).send(&seize);
    h.keys("f");
    assert_eq!(h.top_screen(), "results");
    let stage = h.flow().map(FlowScreen::stage);
    assert!(matches!(stage, Some(Stage::Results(_))));
    let row = "Mara          Exile    Lv 1  EXP █████████████████░░░ 85";
    assert!(shows(&h, row), "{}", h.snapshot());
    // A press fills the bars: she is level 2, and marked.
    h.keys("f");
    let row = "Mara          Exile    Lv 2  EXP █░░░░░░░░░░░░░░░░░░░  6  LEVEL UP";
    assert!(shows(&h, row), "{}", h.snapshot());
    // The next shows her level-up page; it stays until its own press.
    h.keys("f");
    assert_eq!(h.top_screen(), "results");
    assert!(shows(&h, "LEVEL UP!"), "{}", h.snapshot());
    assert!(shows(&h, "Lv 1 → 2"));
    h.wait(10.0);
    assert_eq!(h.top_screen(), "results");
    let campaign = h.flow().and_then(FlowScreen::campaign);
    let after = campaign.map(|c| c.roster[0].clone());
    let after = after.unwrap_or_else(|| panic!("no campaign"));
    assert_eq!((after.level, after.exp), (2, 6));
    // The page's numbers are the roster's.
    for kind in StatKind::GROWABLE {
        let was = before.stats.get(kind);
        let row = stat_row(kind, was, after.stats.get(kind) - was);
        assert!(shows(&h, &row), "{row}: {}", h.snapshot());
    }
    h.keys("d");
    assert_eq!(h.top_screen(), "dialogue");
}
