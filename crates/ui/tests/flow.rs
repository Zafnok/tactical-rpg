//! Scripted tests of the game flow (ticket 0801) through the real game:
//! New Game → mode → lead → the test chapter (`assets/chapters/test.ron`:
//! an intro scene, a battle where the lead seizes the fort at (5, 5)
//! within 3 turns, a victory scene) → "To be continued" → title. Battles
//! are won and lost with scripted commands.

use insta::assert_snapshot;
use trpg_core::{
    BattleState, Command, GameMode, LeadGender, LeadProfile, Outcome, Pos, UnitAction, UnitId,
};
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// The lead's unit in the test battle (its first slot).
const LEAD: UnitId = UnitId(1);
/// The seize tile.
const FORT: Pos = Pos::new(5, 5);

fn title() -> Harness {
    Harness::with_layout(Layout::RightHanded)
}

/// From the title: New Game, the mode (`Down` for Casual), then the lead
/// screen's `keys`, which must end on Start.
fn new_game(casual: bool, lead_keys: &str) -> Harness {
    let mut h = title();
    h.keys("f");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys(if casual { "Down f" } else { "f" });
    assert_eq!(h.top_screen(), "lead_select");
    h.keys(lead_keys);
    h
}

/// Skips the scene on screen: Cancel, then Confirm on "Skip scene?".
fn skip_scene(h: &mut Harness) {
    assert_eq!(h.top_screen(), "dialogue");
    h.keys("d f");
}

/// New Game in Classic with the default lead, through the intro scene to
/// the battle, its notes still up.
fn to_notes() -> Harness {
    let mut h = new_game(false, "Up f");
    skip_scene(&mut h);
    assert_eq!(h.screens(), ["title", "battle"]);
    assert!(notes_open(&h));
    h
}

/// [`to_notes`], and the battle notes closed.
fn to_battle() -> Harness {
    let mut h = to_notes();
    h.keys("f");
    assert!(!notes_open(&h));
    h
}

/// Whether the battle on screen shows its notes box.
fn notes_open(h: &Harness) -> bool {
    let battle = h.flow().and_then(|f| f.battle());
    battle.is_some_and(trpg_ui::screens::battle::BattleScreen::notes_open)
}

/// Sends `cmd` to the battle on screen.
fn send(h: &mut Harness, cmd: &Command) {
    let battle = h.flow_mut().and_then(|f| f.battle_mut());
    battle.unwrap_or_else(|| panic!("no battle")).send(cmd);
}

/// The battle on screen.
fn battle(h: &Harness) -> BattleState {
    let battle = h.flow().and_then(|f| f.battle());
    battle
        .unwrap_or_else(|| panic!("no battle"))
        .state()
        .clone()
}

/// Rewind charges left in the battle on screen.
fn charges(h: &Harness) -> u8 {
    let battle = h.flow().and_then(|f| f.battle());
    battle.map_or(0, |b| b.history().charges_left())
}

fn seize(h: &mut Harness) {
    send(
        h,
        &Command::Act {
            unit: LEAD,
            dest: FORT,
            action: UnitAction::Seize,
        },
    );
}

/// Presses Confirm until the top screen is `name` (banners close one per
/// press).
fn confirm_until(h: &mut Harness, name: &str) {
    for _ in 0..20 {
        if h.top_screen() == name {
            return;
        }
        h.keys("f");
    }
    panic!("never reached {name}: {:?}", h.screens());
}

/// Acceptance: New Game on the test chapter → skip the scenes → win with
/// scripted commands → the victory scene → "To be continued" → title.
#[test]
fn new_game_plays_the_test_chapter_to_the_end() {
    let mut h = to_battle();
    let chapter = h.flow().and_then(|f| f.chapter()).map(|c| c.id.clone());
    assert_eq!(chapter.as_deref(), Some("test"));
    assert_eq!(charges(&h), 3, "a Normal map");
    seize(&mut h);
    assert_eq!(battle(&h).outcome(), Some(Outcome::Victory));
    // The VICTORY banner, then the victory scene.
    h.keys("f");
    assert_eq!(h.screens(), ["title", "dialogue"]);
    let flow = h.flow().unwrap_or_else(|| panic!("no flow"));
    let campaign = flow.campaign().unwrap_or_else(|| panic!("no campaign"));
    assert_eq!(campaign.gold, 500, "the battle's clear gold");
    // The two unused potions went to the stock.
    assert_eq!(campaign.stock.count(&trpg_core::ItemId::new("potion")), 2);
    let rewards = flow.rewards().unwrap_or_else(|| panic!("no rewards"));
    assert_eq!(rewards.unused_charges, 3);
    assert_eq!(rewards.bonus_exp, 21);
    skip_scene(&mut h);
    assert_eq!(h.screens(), ["title", "to_be_continued"]);
    assert_snapshot!(h.snapshot());
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
}

/// Acceptance: New Game → the female lead, her name typed → the intro shows her name and
/// pronouns (0708 tokens).
#[test]
fn the_intro_speaks_of_the_lead_the_player_made() {
    // Female; the name box: "Ellery" deleted, "Ma" typed, Enter; Start.
    let mut h = new_game(
        true,
        "Right Down f Backspace Backspace Backspace Backspace Backspace Backspace",
    );
    h.type_text("Ma");
    h.keys("Enter Down f");
    let flow = h.flow().unwrap_or_else(|| panic!("no flow"));
    let campaign = flow.campaign().unwrap_or_else(|| panic!("no campaign"));
    assert_eq!(campaign.mode, GameMode::Casual);
    assert_eq!(campaign.lead, LeadProfile::new("Ma", LeadGender::Female));
    assert_eq!(campaign.roster[0].name, "Ma");
    assert_eq!(h.game().ctx().lead, campaign.lead);
    assert_eq!(h.top_screen(), "dialogue");
    // First box: the knight names her. Second: narration with her
    // pronoun.
    h.keys("f");
    assert!(shows(&h, "Ma! The fort is just ahead."), "{}", h.snapshot());
    h.keys("f f");
    assert!(
        shows(&h, "Ma looks over the field. She can see the fort"),
        "{}",
        h.snapshot()
    );
    assert_snapshot!(h.snapshot());
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

/// Acceptance: a defeat → Game Over → Retry starts the battle again,
/// identical to its start, with every rewind charge back.
#[test]
fn retry_after_a_defeat_restarts_the_battle() {
    let mut h = to_battle();
    let start = battle(&h);
    // A move, rewound (a charge spent), then the turn limit runs out.
    send(
        &mut h,
        &Command::Act {
            unit: LEAD,
            dest: Pos::new(4, 5),
            action: UnitAction::Wait,
        },
    );
    h.keys("r f f");
    assert_eq!(charges(&h), 2);
    for _ in 0..6 {
        send(&mut h, &Command::EndPhase);
    }
    assert_eq!(battle(&h).outcome(), Some(Outcome::Defeat));
    confirm_until(&mut h, "game_over");
    assert_eq!(h.screens(), ["title", "game_over"]);
    assert_snapshot!(h.snapshot());
    // Retry: the battle from its start, its notes first.
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle(&h), start);
    assert_eq!(charges(&h), 3);
    assert!(notes_open(&h));
    // Lose again, and go back to the title.
    for _ in 0..6 {
        send(&mut h, &Command::EndPhase);
    }
    confirm_until(&mut h, "game_over");
    h.keys("Down f");
    assert_eq!(h.screens(), ["title"]);
}

/// Acceptance: map menu → `Restart Battle` → confirm: the same as Retry.
#[test]
fn restart_battle_from_the_map_menu() {
    let mut h = to_battle();
    let start = battle(&h);
    send(
        &mut h,
        &Command::Act {
            unit: LEAD,
            dest: Pos::new(4, 5),
            action: UnitAction::Wait,
        },
    );
    h.keys("r f f");
    assert_eq!(charges(&h), 2);
    send(
        &mut h,
        &Command::Act {
            unit: UnitId(2),
            dest: Pos::new(4, 6),
            action: UnitAction::Wait,
        },
    );
    // The map menu: Units, Objective, (Options, Suspend), Restart Battle.
    h.keys("d Down Down f");
    assert!(
        shows(&h, "Restart the battle from turn 1?"),
        "{}",
        h.snapshot()
    );
    assert_snapshot!(h.snapshot());
    // Back out to the menu, then confirm.
    h.keys("d");
    assert!(!shows(&h, "Restart the battle"));
    h.keys("f f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(battle(&h), start);
    assert_eq!(charges(&h), 3);
    assert!(notes_open(&h));
}

/// Acceptance (0411): the test battle's two notes show in a box when the
/// battle starts, before any phase banner; only Confirm closes it; the map
/// menu's `Objective` page lists them again.
#[test]
fn battle_notes_show_at_the_start_and_on_the_objective_page() {
    let mut h = to_notes();
    let start = battle(&h);
    assert_eq!(start.battle_notes().len(), 2);
    assert!(shows(&h, "BATTLE NOTES"), "{}", h.snapshot());
    assert!(shows(&h, "• Brigand: guards the road, but the fort is"));
    assert!(shows(&h, "  nearer than the fight."));
    assert!(shows(&h, "• Seize the fort within 3 turns: send your lead"));
    assert!(shows(&h, "f close"));
    assert_snapshot!(h.snapshot());
    // No key but Confirm does anything: no map menu, no cursor move, no
    // end of turn.
    let cursor = |h: &Harness| h.flow().and_then(|f| f.battle()).map(|b| b.cursor().pos);
    let at = cursor(&h);
    h.keys("d Right e Space Space r");
    assert!(notes_open(&h));
    assert_eq!(cursor(&h), at);
    assert_eq!(battle(&h), start);
    assert!(!shows(&h, "Objective"));
    h.keys("f");
    assert!(!notes_open(&h));
    assert!(!shows(&h, "BATTLE NOTES"));
    assert!(!shows(&h, "Brigand: guards"));
    assert_eq!(battle(&h), start);
    // The map menu's Objective page: the objective, then the notes.
    h.keys("d Down f");
    assert!(shows(&h, "Seize the Fort"), "{}", h.snapshot());
    assert!(shows(&h, "Turn 1/3"));
    assert!(shows(&h, "Battle notes"));
    assert!(shows(&h, "• Brigand: guards the road, but the fort is"));
    assert!(shows(&h, "• Seize the fort within 3 turns: send your lead"));
    assert!(!shows(&h, "BATTLE NOTES"));
    assert_snapshot!("objective_page_lists_the_battle_notes", h.snapshot());
    h.keys("d d");
    assert!(!shows(&h, "Battle notes"));
}

/// Acceptance (0411): the first phase banner comes after the notes, not
/// over them.
#[test]
fn the_phase_banner_waits_for_the_battle_notes() {
    let banner = |h: &Harness| {
        let battle = h.flow().and_then(|f| f.battle());
        battle.is_some_and(|b| b.banner().is_some())
    };
    let mut h = to_notes();
    assert!(!banner(&h));
    // The enemy phase begins behind the box: its banner waits.
    send(&mut h, &Command::EndPhase);
    h.keys("Left");
    assert!(notes_open(&h));
    assert!(!banner(&h));
    assert!(!shows(&h, "ENEMY PHASE"));
    h.keys("f");
    assert!(!notes_open(&h));
    assert!(banner(&h));
    assert!(shows(&h, "ENEMY PHASE"), "{}", h.snapshot());
}

/// Acceptance (0411): a battle without notes (the Quick Battle) has no
/// notes box, and nothing more on its `Objective` page.
#[test]
fn no_notes_box_for_a_battle_without_notes() {
    let mut h = title();
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert!(battle(&h).battle_notes().is_empty());
    assert!(!notes_open(&h));
    assert!(!shows(&h, "BATTLE NOTES"));
    // The Objective page: only the objective and the turn.
    h.keys("d Down f");
    assert!(shows(&h, "Rout the enemy"), "{}", h.snapshot());
    assert!(!shows(&h, "Battle notes"));
}

/// Cancel on the lead screen goes back to the mode; on the mode, to the
/// title.
#[test]
fn back_out_of_new_game() {
    let mut h = new_game(false, "d");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("d");
    assert_eq!(h.screens(), ["title"]);
}

/// The debug Quick Battle runs through the flow: a won battle with no
/// scenes goes to "To be continued".
#[test]
fn quick_battle_runs_through_the_flow() {
    let mut h = title();
    h.keys("Down f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let flow = h.flow().unwrap_or_else(|| panic!("no flow"));
    let campaign = flow.campaign().unwrap_or_else(|| panic!("no campaign"));
    assert_eq!(campaign.chapter, "quick");
    assert_eq!(campaign.mode, GameMode::Classic);
    assert_eq!(campaign.roster.len(), 3);
    assert_eq!(battle(&h).units().len(), 6);
}

/// Playtime counts from the start of the campaign.
#[test]
fn playtime_counts_while_playing() {
    let mut h = to_battle();
    h.wait(2.0);
    let played = h
        .flow()
        .and_then(|f| f.campaign())
        .map_or(0, |c| c.playtime_s);
    assert_eq!(played, 2);
}

/// A chapter with a `next` goes on to it after its victory scenes, with
/// the army it won with.
#[test]
fn the_next_chapter_follows_a_victory() {
    let mut ctx = trpg_ui::Ctx::embedded()
        .unwrap_or_else(|e| panic!("{e}"))
        .with_layout(Layout::RightHanded);
    if let Some(test) = ctx.content.chapters.get_mut("test") {
        test.next = Some("quick".into());
    }
    let mut h = Harness::from_game(trpg_ui::Game::start(ctx));
    // New Game, Classic, Start.
    h.keys("f f Up f");
    skip_scene(&mut h);
    // The battle notes, then the VICTORY banner.
    seize(&mut h);
    h.keys("f f");
    skip_scene(&mut h);
    assert_eq!(h.screens(), ["title", "battle"]);
    let flow = h.flow().unwrap_or_else(|| panic!("no flow"));
    assert_eq!(flow.chapter().map(|c| c.id.as_str()), Some("quick"));
    let campaign = flow.campaign().unwrap_or_else(|| panic!("no campaign"));
    assert_eq!(campaign.chapter, "quick");
    assert_eq!(campaign.gold, 500);
    // The Quick Battle's slots are the test lord, knight and archer: of
    // this army only the knight has one.
    let players: Vec<String> = battle(&h)
        .units()
        .iter()
        .filter(|u| u.faction == trpg_core::Faction::Player)
        .map(|u| u.name.clone())
        .collect();
    assert_eq!(players, ["Test Knight"]);
}

/// On a controller, choosing the name opens the letter grid instead of
/// the typing box (Nick: type on a keyboard, a grid on a controller).
#[test]
fn a_controller_spells_the_name_on_the_letter_grid() {
    let mut h = title();
    // New Game, Classic, down to the name, choose it.
    h.pad("South South DpadDown South");
    assert_eq!(h.top_screen(), "lead_select");
    assert!(shows(&h, "A  B  C  D"), "{}", h.snapshot());
    assert!(!shows(&h, "Type a name"));
    // East (Cancel) deletes the "y"; "B" is right of "A"; Done is up and
    // round to the left of the bottom row.
    h.pad("East DpadRight South DpadUp DpadLeft DpadLeft South");
    assert!(!shows(&h, "A  B  C  D"), "{}", h.snapshot());
    assert!(shows(&h, "EllerB Veyne"), "{}", h.snapshot());
    // The keyboard still types: the same name row opens the typing box.
    h.keys("f");
    assert!(shows(&h, "Type a name on your keyboard."));
}
