//! Scripted tests of saving (ticket 0802) through the real game: the map
//! menu's `Suspend` and the title's `Continue`; "Save your progress?" after
//! a victory and the title's `Load Game`; saves that can't be read. The
//! storage is the Harness's `MemoryStorage`, carried from one launch to the
//! next.

use insta::assert_snapshot;
use trpg_core::{
    BattleState, Campaign, Command, GameMode, Pos, SAVE_VERSION, SaveFile, UnitAction, UnitId,
};
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;
use trpg_ui::save::{SUSPEND_KEY, SaveError, slot_key};
use trpg_ui::{Ctx, Game, MemoryStorage, Storage};

/// The lead's unit in the test battle (its first slot).
const LEAD: UnitId = UnitId(1);
/// The seize tile.
const FORT: Pos = Pos::new(5, 5);

/// A launch with the right-handed layout picked, on `storage`. With
/// `chained`, the test chapter is followed by the `quick` chapter.
fn launch(storage: Box<dyn Storage>, chained: bool) -> Harness {
    let mut storage = storage;
    if let Err(e) = storage.write("layout", Layout::RightHanded.name()) {
        panic!("saving the layout: {e}");
    }
    let mut ctx = Ctx::embedded()
        .unwrap_or_else(|e| panic!("{e}"))
        .with_storage(storage);
    if chained && let Some(test) = ctx.content.chapters.get_mut("test") {
        test.next = Some("quick".into());
    }
    Harness::from_game(Game::start(ctx))
}

fn first_launch(chained: bool) -> Harness {
    launch(Box::new(MemoryStorage::new()), chained)
}

/// Ends the run and launches again on its storage.
fn relaunch(h: Harness, chained: bool) -> Harness {
    launch(h.into_storage(), chained)
}

/// From the title: New Game in Classic with the default lead, through the
/// intro scene to the battle.
fn to_battle(h: &mut Harness) {
    assert_eq!(h.screens(), ["title"]);
    // New Game is focused, with or without Load Game enabled.
    assert!(!shows(h, "Continue"));
    h.keys("f f Up f");
    skip_scene(h);
    assert_eq!(h.screens(), ["title", "battle"]);
}

/// Skips the scene on screen: Cancel, then Confirm on "Skip scene?".
fn skip_scene(h: &mut Harness) {
    assert_eq!(h.top_screen(), "dialogue");
    h.keys("d f");
}

/// Wins the test battle and plays on to "Save your progress?".
fn win(h: &mut Harness) {
    send(
        h,
        &Command::Act {
            unit: LEAD,
            dest: FORT,
            action: UnitAction::Seize,
        },
    );
    // The VICTORY banner, then the victory scene.
    h.keys("f");
    skip_scene(h);
    assert_eq!(h.screens(), ["title", "save_prompt"]);
}

fn send(h: &mut Harness, cmd: &Command) {
    let battle = h.flow_mut().and_then(|f| f.battle_mut());
    battle.unwrap_or_else(|| panic!("no battle")).send(cmd);
}

fn wait_at(h: &mut Harness, unit: UnitId, dest: Pos) {
    let action = UnitAction::Wait;
    send(h, &Command::Act { unit, dest, action });
}

/// The battle on screen.
fn battle(h: &Harness) -> BattleState {
    let battle = h.flow().and_then(|f| f.battle());
    battle
        .unwrap_or_else(|| panic!("no battle"))
        .state()
        .clone()
}

/// The battle on screen and its history, serialised.
fn battle_text(h: &Harness) -> (String, String) {
    let battle = h.flow().and_then(|f| f.battle());
    let battle = battle.unwrap_or_else(|| panic!("no battle"));
    let state = ron::to_string(battle.state());
    let history = ron::to_string(battle.history());
    (
        state.unwrap_or_else(|e| panic!("{e}")),
        history.unwrap_or_else(|e| panic!("{e}")),
    )
}

/// How many commands the battle on screen has in its history.
fn moves(h: &Harness) -> usize {
    let battle = h.flow().and_then(|f| f.battle());
    battle.map_or(0, |b| b.history().len())
}

/// Rewind charges left in the battle on screen.
fn charges(h: &Harness) -> u8 {
    let battle = h.flow().and_then(|f| f.battle());
    battle.map_or(0, |b| b.history().charges_left())
}

fn campaign(h: &Harness) -> Campaign {
    let campaign = h.flow().and_then(|f| f.campaign());
    campaign.unwrap_or_else(|| panic!("no campaign")).clone()
}

fn stored(h: &Harness, key: &str) -> Option<String> {
    let read = h.game().ctx().storage.read(key);
    read.unwrap_or_else(|e| panic!("{e}"))
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

/// The map menu's `Suspend` (Units, Objective, (Options), Suspend), and
/// Confirm on "Suspend the battle and return to the title?".
fn suspend(h: &mut Harness) {
    h.keys("d Down Down f");
    assert!(shows(h, "Suspend the battle and return to the title?"));
    assert_eq!(h.screens(), ["title", "battle"], "not before the answer");
    h.keys("f");
}

/// A battle with a rewind spent and two moves made, suspended.
fn suspended_battle() -> (Harness, (String, String)) {
    let mut h = first_launch(false);
    to_battle(&mut h);
    wait_at(&mut h, LEAD, Pos::new(4, 5));
    h.keys("r f f");
    assert_eq!(charges(&h), 2);
    wait_at(&mut h, LEAD, Pos::new(4, 5));
    wait_at(&mut h, UnitId(2), Pos::new(4, 6));
    assert_eq!(moves(&h), 2);
    h.wait(3.0);
    let before = battle_text(&h);
    assert_eq!(stored(&h, SUSPEND_KEY), None);
    suspend(&mut h);
    (h, before)
}

/// Acceptance: suspend mid-battle → Continue → the battle is the same to
/// the byte (rewind points and charges too) → the suspend save is gone.
#[test]
fn suspend_then_continue_restores_the_battle_exactly() {
    let (mut h, before) = suspended_battle();
    // Back at the title, which now offers Continue, focused.
    assert_eq!(h.screens(), ["title"]);
    assert!(stored(&h, SUSPEND_KEY).is_some());
    assert!(shows(&h, "Continue"));
    assert_snapshot!(h.snapshot());
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle_text(&h), before);
    assert_eq!(charges(&h), 2);
    assert_eq!(stored(&h, SUSPEND_KEY), None, "a suspend save loads once");
    let played = campaign(&h);
    assert_eq!(played.chapter, "test");
    assert_eq!(played.playtime_s, 3, "playtime carries on");
    assert_eq!(h.game().ctx().lead, played.lead);
    // Both moves can still be rewound.
    assert_eq!(moves(&h), 2);
    h.keys("r f f");
    assert_eq!(charges(&h), 1);
    assert_eq!(moves(&h), 1);
}

/// Acceptance (the web's "reload the page"): the suspend save outlives the
/// launch, and Continue works in the next one.
#[test]
fn continue_works_in_the_next_launch() {
    let (h, before) = suspended_battle();
    let mut h = relaunch(h, false);
    assert_eq!(h.screens(), ["title"]);
    assert!(shows(&h, "Continue"));
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle_text(&h), before);
    assert_eq!(charges(&h), 2);
    assert_eq!(stored(&h, SUSPEND_KEY), None);
    // Suspended a second time, it can be continued a second time.
    suspend(&mut h);
    assert_eq!(h.screens(), ["title"]);
    assert!(stored(&h, SUSPEND_KEY).is_some());
    h.keys("f");
    assert_eq!(battle_text(&h), before);
    // Once continued, leaving for the title leaves no Continue behind.
    for _ in 0..6 {
        send(&mut h, &Command::EndPhase);
    }
    for _ in 0..20 {
        if h.top_screen() == "game_over" {
            break;
        }
        h.keys("f");
    }
    assert_eq!(h.top_screen(), "game_over");
    h.keys("Down f");
    assert_eq!(h.screens(), ["title"]);
    assert!(!shows(&h, "Continue"));
}

/// `Restart Battle` after Continue starts the battle as it first started,
/// every rewind charge back.
#[test]
fn a_continued_battle_restarts_from_its_first_turn() {
    let mut fresh = first_launch(false);
    to_battle(&mut fresh);
    let start = battle(&fresh);
    let (mut h, _) = suspended_battle();
    h.keys("f");
    assert_ne!(battle(&h), start);
    // The map menu: Units, Objective, (Options), Suspend, Restart Battle.
    h.keys("d Down Down Down f f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle(&h), start);
    assert_eq!(charges(&h), 3);
    // And is won as any other.
    win(&mut h);
}

/// Acceptance: save after victory → Load → the next chapter starts with
/// the same roster.
#[test]
fn save_after_victory_then_load_starts_the_next_chapter() {
    let mut h = first_launch(true);
    // Nothing saved yet: Load Game can't be chosen (Down skips it).
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "battle"], "Quick Battle");
    let mut h = relaunch(h, true);
    to_battle(&mut h);
    h.wait(5.0);
    win(&mut h);
    assert_snapshot!("save_prompt", h.snapshot());
    let won = campaign(&h);
    assert_eq!(won.chapter, "test", "the chapter just cleared");
    assert_eq!(won.gold, 500);
    // Yes → the slots, on the first empty one → saved there.
    h.keys("f");
    assert_eq!(h.screens(), ["title", "save_slots"]);
    assert_snapshot!("save_slots_empty", h.snapshot());
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"], "the next chapter");
    assert_eq!(campaign(&h).chapter, "quick");
    let saved = stored(&h, &slot_key(1)).unwrap_or_else(|| panic!("slot 1 is empty"));
    let mut saved: SaveFile = ron::from_str(&saved).unwrap_or_else(|e| panic!("{e}"));
    // (Saved a few frames after `won` was read.)
    assert_eq!(saved.campaign.playtime_s, 5);
    saved.campaign.playtime_s = won.playtime_s;
    assert_eq!(saved, SaveFile::chapter_cleared(won.clone()));

    // The next launch: Load Game (now enabled) → the slot → the next
    // chapter, with the army that won the last one.
    let mut h = relaunch(h, true);
    assert!(!shows(&h, "Continue"));
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "save_slots"]);
    assert!(shows(&h, "Quick Battle"), "the chapter it goes on with");
    assert!(shows(&h, "0:00:05"));
    assert_snapshot!("load_slots", h.snapshot());
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let loaded = campaign(&h);
    assert_eq!(loaded.chapter, "quick");
    assert_eq!(loaded.roster, won.roster);
    assert_eq!(loaded.gold, 500);
    assert_eq!(loaded.mode, GameMode::Classic);
    assert_eq!(loaded.stock, won.stock);
    assert_eq!(loaded.playtime_s, 5, "playtime carries on");
    assert_eq!(h.game().ctx().lead, won.lead);
    let chapter = h.flow().and_then(|f| f.chapter()).map(|c| c.id.clone());
    assert_eq!(chapter.as_deref(), Some("quick"));
    // Only the knight has a slot in the Quick Battle.
    let players: Vec<String> = battle(&h)
        .units()
        .iter()
        .filter(|u| u.faction == trpg_core::Faction::Player)
        .map(|u| u.name.clone())
        .collect();
    assert_eq!(players, ["Test Knight"]);
}

/// "No" saves nothing; backing out of the slots asks again.
#[test]
fn the_save_prompt_can_be_declined_and_the_slots_backed_out_of() {
    let mut h = first_launch(false);
    to_battle(&mut h);
    win(&mut h);
    // Yes, then back out of the slots: the question again.
    h.keys("f");
    assert_eq!(h.top_screen(), "save_slots");
    h.keys("d");
    assert_eq!(h.screens(), ["title", "save_prompt"]);
    assert_eq!(stored(&h, &slot_key(1)), None);
    // No: on with the game (the last chapter: "To be continued").
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "to_be_continued"]);
    assert_eq!(stored(&h, &slot_key(1)), None);
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);

    // Again, saving into slot 2; then once more in the same run.
    to_battle(&mut h);
    win(&mut h);
    h.keys("f Down f");
    assert_eq!(h.screens(), ["title", "to_be_continued"]);
    assert_eq!(stored(&h, &slot_key(1)), None);
    assert!(stored(&h, &slot_key(2)).is_some());
    h.keys("f");
    // Loading it goes on after the cleared chapter: nothing yet.
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "save_slots"]);
    assert!(shows(&h, "Test Chapter (cleared)"));
    h.keys("f");
    assert_eq!(h.screens(), ["title", "to_be_continued"]);
    assert_eq!(campaign(&h).chapter, "test");
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
}

/// Saving again opens on the first empty slot, not the one used before
/// (Nick, PR #141); a slot with a save says it will be overwritten and
/// asks first.
#[test]
fn saving_again_opens_on_an_empty_slot_and_asks_before_overwriting() {
    let mut h = first_launch(true);
    to_battle(&mut h);
    win(&mut h);
    // Yes, the first empty slot (1).
    h.keys("f f");
    assert!(stored(&h, &slot_key(1)).is_some());
    assert_eq!(h.screens(), ["title", "battle"]);
    // The next launch: Load Game opens on that save; back to the title.
    let mut h = relaunch(h, true);
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "save_slots"]);
    assert!(shows(&h, "arrows choose · f load · d back"));
    h.keys("d");
    assert_eq!(h.screens(), ["title"]);
    // Another game won: the slots open on slot 2, the first empty one.
    h.keys("Up");
    to_battle(&mut h);
    win(&mut h);
    let first = stored(&h, &slot_key(1));
    h.keys("f");
    assert!(shows(&h, "f save here"), "{}", h.snapshot());
    // Up to the full slot: it says so, and asks.
    h.keys("Up");
    assert!(shows(&h, "f overwrite"), "{}", h.snapshot());
    h.keys("f");
    assert!(shows(&h, "Overwrite slot 01?"), "{}", h.snapshot());
    assert_eq!(stored(&h, &slot_key(1)), first);
    // No: back to the list. Down to the empty slot and save there.
    h.keys("d Down f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(stored(&h, &slot_key(1)), first);
    assert!(stored(&h, &slot_key(2)).is_some());
}

/// `text` as the suspend save and in slot 1 of an otherwise empty storage.
fn storage_with(text: &str) -> Box<dyn Storage> {
    let mut storage = MemoryStorage::new();
    for key in [SUSPEND_KEY.to_owned(), slot_key(1)] {
        if let Err(e) = storage.write(&key, text) {
            panic!("{e}");
        }
    }
    Box::new(storage)
}

/// Acceptance: corrupt and wrong-version saves show a message, never
/// panic, and are left where they are.
#[test]
fn saves_that_cant_be_read_show_a_message() {
    // A real save, as another version wrote it.
    let (h, _) = suspended_battle();
    let real = stored(&h, SUSPEND_KEY).unwrap_or_else(|| panic!("no suspend save"));
    let mut other: SaveFile = ron::from_str(&real).unwrap_or_else(|e| panic!("{e}"));
    other.version = SAVE_VERSION + 1;
    let other = ron::to_string(&other).unwrap_or_else(|e| panic!("{e}"));
    let half: String = real.chars().take(real.chars().count() / 2).collect();

    for (text, error) in [
        (other.as_str(), SaveError::Incompatible),
        ("SaveFile(version: 0, old: true)", SaveError::Incompatible),
        ("not a save at all", SaveError::Corrupt),
        (half.as_str(), SaveError::Corrupt),
        ("", SaveError::Corrupt),
    ] {
        let message = error.to_string();
        let mut h = launch(storage_with(text), false);
        // Continue is offered; choosing it says why it can't.
        assert!(shows(&h, "Continue"));
        assert!(!shows(&h, &message));
        h.keys("f");
        assert_eq!(h.screens(), ["title"], "{text:?}");
        assert!(shows(&h, &message), "{}", h.snapshot());
        assert_eq!(stored(&h, SUSPEND_KEY).as_deref(), Some(text));
        // The next key clears the message.
        h.keys("Down");
        assert!(!shows(&h, &message));
        // Load Game: the slot says why too, and can't be loaded.
        h.keys("Down f");
        assert_eq!(h.screens(), ["title", "save_slots"]);
        assert!(shows(&h, &message), "{}", h.snapshot());
        h.keys("Up Down f");
        assert_eq!(h.screens(), ["title", "save_slots"]);
        assert_eq!(stored(&h, &slot_key(1)).as_deref(), Some(text));
        h.keys("d");
        assert_eq!(h.screens(), ["title"]);
    }
}

/// The title with a suspend save that can't be read, after Continue.
#[test]
fn the_title_says_why_it_cant_continue() {
    let mut h = launch(storage_with("SaveFile(version: 0)"), false);
    h.keys("f");
    assert_snapshot!(h.snapshot());
}

/// A suspend save of a chapter the game doesn't have (any more) can't be
/// continued, and isn't deleted.
#[test]
fn a_suspend_save_of_an_unknown_chapter_cant_be_continued() {
    let (h, _) = suspended_battle();
    let real = stored(&h, SUSPEND_KEY).unwrap_or_else(|| panic!("no suspend save"));
    let mut save: SaveFile = ron::from_str(&real).unwrap_or_else(|e| panic!("{e}"));
    save.campaign.chapter = "gone".into();
    let text = ron::to_string(&save).unwrap_or_else(|e| panic!("{e}"));
    let mut h = launch(storage_with(&text), false);
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
    assert!(shows(&h, &SaveError::Corrupt.to_string()));
    assert_eq!(stored(&h, SUSPEND_KEY), Some(text.clone()));
    // A chapter save in the suspend key isn't a battle to continue.
    save.campaign.chapter = "test".into();
    let cleared = SaveFile::chapter_cleared(save.campaign);
    let text = ron::to_string(&cleared).unwrap_or_else(|e| panic!("{e}"));
    let mut h = launch(storage_with(&text), false);
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
    assert!(shows(&h, &SaveError::Corrupt.to_string()));
    // In a slot it is a save like any other.
    h.keys("Down Down f");
    assert_eq!(h.screens(), ["title", "save_slots"]);
    h.keys("f");
    assert_eq!(h.screens(), ["title", "to_be_continued"]);
}

/// The debug Quick Battle suspends and continues like any battle.
#[test]
fn the_quick_battle_suspends_too() {
    let mut h = first_launch(false);
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let archer = battle(&h).unit(UnitId(3)).map(|u| u.pos);
    wait_at(
        &mut h,
        UnitId(3),
        archer.unwrap_or_else(|| panic!("no unit 3")),
    );
    assert_eq!(moves(&h), 1);
    let before = battle_text(&h);
    suspend(&mut h);
    assert_eq!(h.screens(), ["title"]);
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle_text(&h), before);
    assert_eq!(campaign(&h).chapter, "quick");
}

/// A storage whose writes or deletes of the suspend save can be made to
/// fail.
#[derive(Debug, Default)]
struct Flaky {
    inner: MemoryStorage,
    fail_suspend_write: bool,
    fail_delete: bool,
}

impl Storage for Flaky {
    fn read(&self, key: &str) -> Result<Option<String>, trpg_ui::StorageError> {
        self.inner.read(key)
    }
    fn write(&mut self, key: &str, value: &str) -> Result<(), trpg_ui::StorageError> {
        if self.fail_suspend_write && key == SUSPEND_KEY {
            return Err(trpg_ui::StorageError::Backend("disk full".into()));
        }
        self.inner.write(key, value)
    }
    fn delete(&mut self, key: &str) -> Result<(), trpg_ui::StorageError> {
        if self.fail_delete {
            return Err(trpg_ui::StorageError::Backend("locked".into()));
        }
        self.inner.delete(key)
    }
    fn list(&self) -> Result<Vec<String>, trpg_ui::StorageError> {
        self.inner.list()
    }
}

/// A suspend save that can't be written: the battle goes on, saying why.
#[test]
fn a_failed_suspend_stays_in_the_battle() {
    let storage = Flaky {
        fail_suspend_write: true,
        ..Flaky::default()
    };
    let mut h = launch(Box::new(storage), false);
    to_battle(&mut h);
    wait_at(&mut h, UnitId(2), Pos::new(4, 6));
    assert_eq!(moves(&h), 1);
    let before = battle_text(&h);
    suspend(&mut h);
    assert_eq!(h.screens(), ["title", "battle"]);
    assert!(
        shows(&h, "Save storage error: disk full"),
        "{}",
        h.snapshot()
    );
    assert_eq!(battle_text(&h), before);
    assert_eq!(stored(&h, SUSPEND_KEY), None);
    // The battle is still played: the map menu opens again, and the
    // message goes away.
    h.wait(2.0);
    assert!(!shows(&h, "Save storage error"));
    h.keys("d");
    assert!(shows(&h, "Suspend"));
    h.keys("d");
    win(&mut h);
}

/// A suspend save that can't be deleted is still continued.
#[test]
fn continue_goes_on_even_if_the_save_cant_be_deleted() {
    let (h, before) = suspended_battle();
    let mut storage = Flaky {
        fail_delete: true,
        ..Flaky::default()
    };
    let old = h.into_storage();
    let text = old.read(SUSPEND_KEY).unwrap_or_else(|e| panic!("{e}"));
    let text = text.unwrap_or_else(|| panic!("no suspend save"));
    if let Err(e) = storage.inner.write(SUSPEND_KEY, &text) {
        panic!("{e}");
    }
    let mut h = launch(Box::new(storage), false);
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle_text(&h), before);
}

/// Suspending brings the title's music back; Continue starts the battle's
/// again (0807).
#[test]
fn the_music_follows_suspend_and_continue() {
    let mut h = first_launch(false);
    if let Some(test) = h.ctx_mut().content.battles.get_mut("test") {
        test.music = trpg_core::BattleMusic::Cue("battle_easy".into());
    }
    let music = |h: &Harness| -> Vec<String> {
        h.audio_requests()
            .iter()
            .filter(|r| !matches!(r, trpg_ui::AudioRequest::PlaySound { .. }))
            .map(|r| r.cue().unwrap_or("-").to_owned())
            .collect()
    };
    to_battle(&mut h);
    assert_eq!(music(&h), ["title", "battle_easy"]);
    suspend(&mut h);
    h.wait(0.1);
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(music(&h), ["title", "battle_easy", "title"]);
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(music(&h), ["title", "battle_easy", "title", "battle_easy"]);
}
