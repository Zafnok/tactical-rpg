use std::collections::BTreeMap;

use proptest::prelude::*;

use super::*;
use crate::battle::tests::{arb_setup, arb_unit, cast, item, setup};
use crate::battle::{BattleState, Command, GameMode};
use crate::item::Stock;
use crate::lead::{LeadGender, LeadProfile};
use crate::legal_commands;
use crate::unit::{CharacterId, Unit};

fn campaign() -> Campaign {
    Campaign::new_game(
        GameMode::Casual,
        LeadProfile::new("Mara", LeadGender::Female),
        "ch01",
        cast(),
        100,
        Stock::default(),
    )
}

#[test]
fn a_chapter_save_is_of_this_version_and_holds_no_battle() {
    let save = SaveFile::chapter_cleared(campaign());
    assert_eq!(save.version, SAVE_VERSION);
    assert_eq!(save.campaign, campaign());
    assert_eq!(save.point, SavePoint::ChapterCleared);
}

#[test]
fn a_suspend_save_holds_the_battles_history() {
    let (mut s, _) = BattleState::new(setup(cast()));
    let mut history = BattleHistory::new(s.clone());
    s.apply(&Command::EndPhase).unwrap();
    history.push(Command::EndPhase);
    let save = SaveFile::suspended(campaign(), history.clone());
    assert_eq!(save.version, SAVE_VERSION);
    assert_eq!(save.campaign, campaign());
    assert_eq!(save.point, SavePoint::Battle(Box::new(history)));
}

#[test]
fn the_header_reads_the_version_of_any_save() {
    let mut save = SaveFile::chapter_cleared(campaign());
    let text = ron::to_string(&save).unwrap();
    let header: SaveHeader = ron::from_str(&text).unwrap();
    assert_eq!(header.version, SAVE_VERSION);
    assert!(header.is_current());
    for version in [SAVE_VERSION - 1, SAVE_VERSION + 1] {
        save.version = version;
        let text = ron::to_string(&save).unwrap();
        let header: SaveHeader = ron::from_str(&text).unwrap();
        assert_eq!(header.version, version);
        assert!(!header.is_current());
    }
    // A save from a build whose other fields have changed shape.
    let other = "SaveFile(version:7,campaign:(era:\"old\"),elsewhere:[1,2])";
    assert_eq!(
        ron::from_str::<SaveHeader>(other).map(|h| (h.version, h.is_current())),
        Ok((7, false))
    );
    assert!(ron::from_str::<SaveFile>(other).is_err());
    assert!(ron::from_str::<SaveHeader>("not a save").is_err());
}

prop_compose! {
    fn arb_campaign()(
        roster in prop::collection::vec(arb_unit(), 0..=6),
        casual in any::<bool>(),
        female in any::<bool>(),
        name in "[A-Za-z' -]{1,12}",
        chapter in "[a-z0-9_]{0,8}",
        gold in any::<u32>(),
        playtime_s in any::<u64>(),
        flags in prop::collection::btree_map("[a-z_]{1,8}", any::<bool>(), 0..=4),
        potions in 0u32..=5,
        spare in prop::option::of(arb_unit()),
    ) -> Campaign {
        let roster: Vec<Unit> = roster
            .into_iter()
            .enumerate()
            .map(|(i, u)| Unit { character: Some(CharacterId(format!("c{i}"))), ..u })
            .collect();
        let mut stock = Stock::default();
        if potions > 0 {
            stock.items = BTreeMap::from([(item("potion"), potions)]);
        }
        // A used weapon in the stock keeps its wear.
        stock.weapons = spare
            .iter()
            .flat_map(|u| u.loadout.weapons.iter().flatten().cloned())
            .collect();
        let gender = if female { LeadGender::Female } else { LeadGender::Male };
        let mode = if casual { GameMode::Casual } else { GameMode::Classic };
        let mut campaign =
            Campaign::new_game(mode, LeadProfile::new(name, gender), chapter, roster, gold, stock);
        campaign.flags = flags;
        campaign.playtime_s = playtime_s;
        campaign
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        max_shrink_iters: 256,
        ..ProptestConfig::with_cases(96)
    })]

    #[test]
    fn a_chapter_save_round_trips_through_ron(campaign in arb_campaign()) {
        let save = SaveFile::chapter_cleared(campaign);
        let text = ron::to_string(&save).unwrap();
        let back: SaveFile = ron::from_str(&text).unwrap();
        prop_assert_eq!(&back, &save);
        prop_assert_eq!(ron::to_string(&back).unwrap(), text);
    }

    /// A battle played some way, suspended and read back, is the same
    /// battle to the byte, and goes on the same way.
    #[test]
    fn a_suspend_save_round_trips_through_ron(
        campaign in arb_campaign(),
        setup in arb_setup(),
        choices in prop::collection::vec(any::<u16>(), 0..40),
        more in prop::collection::vec(any::<u16>(), 0..10),
    ) {
        let tables = (
            setup.terrain.clone(),
            setup.classes.clone(),
            setup.items.clone(),
            setup.spells.clone(),
            setup.skills.clone(),
            setup.arts.clone(),
        );
        let (mut s, _) = BattleState::new(setup);
        let mut history = BattleHistory::new(s.clone());
        for choice in choices {
            if s.outcome().is_some() {
                break;
            }
            let legal = legal_commands(&s);
            let cmd = legal[usize::from(choice) % legal.len()].clone();
            prop_assert!(s.apply(&cmd).is_ok());
            history.push(cmd);
        }
        let save = SaveFile::suspended(campaign, history.clone());
        let text = ron::to_string(&save).unwrap();
        let back: SaveFile = ron::from_str(&text).unwrap();
        prop_assert_eq!(ron::to_string(&back).unwrap(), text);
        prop_assert_eq!(&back.campaign, &save.campaign);
        let SavePoint::Battle(mut loaded) = back.point else {
            return Err(TestCaseError::fail("not a battle save"));
        };
        let (terrain, classes, items, spells, skills, arts) = tables;
        loaded.restore_tables(&crate::GameTables {
            terrain,
            classes,
            items,
            spells,
            skills,
            arts,
            supports: std::sync::Arc::default(),
        });
        prop_assert_eq!(loaded.charges_left(), history.charges_left());
        prop_assert_eq!(loaded.commands(), history.commands());
        let mut resumed = loaded.state_at(loaded.len());
        prop_assert_eq!(ron::to_string(&resumed).unwrap(), ron::to_string(&s).unwrap());
        // The same commands after the save do the same things.
        for choice in more {
            if s.outcome().is_some() {
                break;
            }
            let legal = legal_commands(&s);
            let cmd = legal[usize::from(choice) % legal.len()].clone();
            prop_assert_eq!(resumed.apply(&cmd), s.apply(&cmd));
        }
        prop_assert_eq!(ron::to_string(&resumed).unwrap(), ron::to_string(&s).unwrap());
    }
}
