//! The baseline bot plays every shipped battle file (ticket 0505).

use trpg_bots::{BaselineBot, play_battle};
use trpg_content::battle_campaign;
use trpg_core::{BattleState, GameMode, LeadGender, LeadProfile};

#[test]
fn baseline_bot_plays_a_battle_to_the_end() {
    let content = trpg_content::load_embedded().unwrap();
    let tables = content.tables();
    assert!(!content.battles.is_empty());
    for (id, def) in &content.battles {
        let lead = LeadProfile::new("Bot", LeadGender::Male);
        let campaign = battle_campaign(&content, def, GameMode::Classic, lead);
        let mut finished = 0;
        for seed in 1..=5 {
            let mut setup = campaign.battle_setup(def, &tables);
            setup.seed = seed;
            let (state, _) = BattleState::new(setup);
            let mut bot = BaselineBot::new(content.ai);
            let measures = play_battle(state, &mut bot, &content.ai, 60)
                .unwrap_or_else(|e| panic!("{id}, seed {seed}: {e}"));
            assert!(measures.player_commands > 0, "{id}, seed {seed}");
            finished += u32::from(measures.outcome.is_some());
        }
        assert!(finished > 0, "{id}: no try ended in 60 turns");
    }
}
