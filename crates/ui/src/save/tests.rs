use trpg_content::new_campaign;
use trpg_core::{BattleHistory, BattleState, LeadGender, LeadProfile, SAVE_VERSION};

use super::*;
use crate::screen::Ctx;
use crate::screen::tests::ctx;
use crate::storage::MemoryStorage;

fn campaign(c: &Ctx) -> Campaign {
    let lead = LeadProfile::new("Mara", LeadGender::Female);
    let mut campaign = new_campaign(&c.content, GameMode::Casual, lead);
    campaign.playtime_s = 3725;
    campaign
}

/// A suspend save of the test chapter's battle, just started.
fn suspended(c: &Ctx) -> SaveFile {
    let campaign = campaign(c);
    let def = &c.content.battles[&c.content.chapters[&campaign.chapter].battle];
    let (state, _) = BattleState::new(campaign.battle_setup(def, &c.content.tables()));
    SaveFile::suspended(campaign, BattleHistory::new(state))
}

/// `save` as text, written by `version`.
fn written_by(save: &SaveFile, version: u32) -> String {
    let mut save = save.clone();
    save.version = version;
    ron::to_string(&save).unwrap()
}

#[test]
fn slot_keys_are_valid_storage_keys() {
    assert_eq!(SLOTS, 30);
    assert_eq!(slot_key(1), "slot_01");
    assert_eq!(slot_key(30), "slot_30");
    for n in 1..=SLOTS {
        assert!(crate::storage::is_valid_key(&slot_key(n)));
    }
    assert!(crate::storage::is_valid_key(SUSPEND_KEY));
}

#[test]
fn a_save_round_trips_through_text() {
    let c = ctx();
    let save = SaveFile::chapter_cleared(campaign(&c));
    let text = encode(&save).unwrap();
    assert_eq!(decode(&text), Ok(save));
    // A battle comes back without its content tables (ADR-0020), so it is
    // compared as text.
    let save = suspended(&c);
    let text = encode(&save).unwrap();
    let back = decode(&text).unwrap();
    assert_eq!(back.campaign, save.campaign);
    assert!(matches!(back.point, SavePoint::Battle(_)));
    assert_eq!(encode(&back).unwrap(), text);
}

#[test]
fn another_versions_save_is_incompatible_and_junk_is_corrupt() {
    let c = ctx();
    let save = SaveFile::chapter_cleared(campaign(&c));
    for version in [SAVE_VERSION - 1, SAVE_VERSION + 1] {
        let text = written_by(&save, version);
        assert_eq!(decode(&text), Err(SaveError::Incompatible));
    }
    // Another version whose fields this build can't parse at all.
    let old = "SaveFile(version:0,army:[\"lord\"])";
    assert_eq!(decode(old), Err(SaveError::Incompatible));
    // This version, but not a save.
    let text = format!("SaveFile(version:{SAVE_VERSION},army:[])");
    assert_eq!(decode(&text), Err(SaveError::Corrupt));
    for junk in ["", "hello", "(", "SaveFile(version:\"one\")", "[1,2,3]"] {
        assert_eq!(decode(junk), Err(SaveError::Corrupt), "{junk:?}");
    }
    // Cut short.
    let text = encode(&save).unwrap();
    assert_eq!(decode(&text[..text.len() / 2]), Err(SaveError::Corrupt));
}

#[test]
fn the_errors_say_what_the_player_reads() {
    assert_eq!(
        SaveError::Incompatible.to_string(),
        "This save is from an incompatible version"
    );
    assert_eq!(SaveError::Corrupt.to_string(), "This save can't be read");
    assert_eq!(SaveError::Missing.to_string(), "There is no save here");
    let storage = SaveError::Storage(StorageError::Backend("disk full".into()));
    assert_eq!(storage.to_string(), "Save storage error: disk full");
}

#[test]
fn saves_are_written_to_and_read_from_storage() {
    let c = ctx();
    let mut storage = MemoryStorage::new();
    assert_eq!(read(&storage, SUSPEND_KEY), Ok(None));
    assert!(!has_suspend(&storage));
    assert!(!any_slot(&storage));
    let save = suspended(&c);
    write(&mut storage, SUSPEND_KEY, &save).unwrap();
    assert!(has_suspend(&storage));
    assert!(!any_slot(&storage));
    let back = read(&storage, SUSPEND_KEY).unwrap().unwrap();
    assert_eq!(encode(&back), encode(&save));
    // The last slot counts as a slot; junk in one does too.
    storage.write(&slot_key(SLOTS), "junk").unwrap();
    assert!(any_slot(&storage));
    assert_eq!(read(&storage, &slot_key(SLOTS)), Err(SaveError::Corrupt));
    // A key that isn't one is a storage error, not a panic.
    let bad = read(&storage, "No Such Key");
    assert!(matches!(bad, Err(SaveError::Storage(_))), "{bad:?}");
    let bad = write(&mut storage, "No Such Key", &suspended(&c));
    assert!(matches!(bad, Err(SaveError::Storage(_))), "{bad:?}");
    assert!(!has_suspend(&MemoryStorage::new()));
}

#[test]
fn a_slot_shows_the_chapter_it_goes_on_with() {
    let mut c = ctx();
    let game = campaign(&c);
    assert_eq!(game.chapter, "test");
    let title = c.content.chapters["test"].title.clone();
    // No chapter after it yet.
    assert_eq!(
        next_chapter_title(&c.content, &game),
        format!("{title} (cleared)")
    );
    // The chapter after it.
    c.content.chapters.get_mut("test").unwrap().next = Some("quick".into());
    let next = c.content.chapters["quick"].title.clone();
    assert_ne!(next, title);
    assert_eq!(next_chapter_title(&c.content, &game), next);
    // A next chapter the game doesn't have: as if there were none.
    c.content.chapters.get_mut("test").unwrap().next = Some("gone".into());
    assert_eq!(
        next_chapter_title(&c.content, &game),
        format!("{title} (cleared)")
    );
    // A cleared chapter the game no longer has: its id.
    let mut lost = game;
    lost.chapter = "old_ch".into();
    assert_eq!(next_chapter_title(&c.content, &lost), "old_ch");
}

#[test]
fn slots_are_empty_saved_or_unreadable() {
    let c = ctx();
    let mut storage = MemoryStorage::new();
    let game = campaign(&c);
    let save = SaveFile::chapter_cleared(game.clone());
    write(&mut storage, &slot_key(2), &save).unwrap();
    storage.write(&slot_key(3), "junk").unwrap();
    let old = written_by(&save, SAVE_VERSION + 1);
    storage.write(&slot_key(4), &old).unwrap();
    // A battle save is never written to a slot.
    write(&mut storage, &slot_key(5), &suspended(&c)).unwrap();

    let all = slots(&storage, &c.content);
    assert_eq!(all.len(), SLOTS);
    assert_eq!(all[0], Slot::Empty);
    let Slot::Saved(summary) = &all[1] else {
        panic!("{:?}", all[1]);
    };
    assert_eq!(summary.chapter, next_chapter_title(&c.content, &game));
    assert_eq!(summary.mode, GameMode::Casual);
    assert_eq!(summary.roster, game.roster.len());
    assert!(summary.roster > 0);
    assert_eq!(summary.playtime_s, 3725);
    assert_eq!(summary.save, save);
    assert_eq!(all[2], Slot::Unreadable(SaveError::Corrupt));
    assert_eq!(all[3], Slot::Unreadable(SaveError::Incompatible));
    assert_eq!(all[4], Slot::Unreadable(SaveError::Corrupt));
    assert!(all[5..].iter().all(|s| *s == Slot::Empty));
    assert_eq!(slot(&storage, &c.content, 2), all[1]);
}

#[test]
fn playtime_reads_as_hours_minutes_seconds() {
    assert_eq!(playtime_text(0), "0:00:00");
    assert_eq!(playtime_text(59), "0:00:59");
    assert_eq!(playtime_text(60), "0:01:00");
    assert_eq!(playtime_text(3725), "1:02:05");
    assert_eq!(playtime_text(3600 * 123 + 59 * 60 + 59), "123:59:59");
}
