use trpg_core::{LeadGender, LeadProfile};

use super::*;
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
