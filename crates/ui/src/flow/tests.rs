use trpg_core::{LeadGender, LeadProfile};

use super::*;
use crate::input::Action::Cancel;
use crate::screen::tests::ctx;

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
