//! Scripted tests of the dialogue screen through the real game (ADR-0007
//! layer 4): the test scene (`assets/dialogue/test.dlg`) opened from the
//! debug menu, full-screen and over the battle map, and reply choices
//! (0708).

use insta::assert_snapshot;
use trpg_content::{ChoiceOption, MusicLine, Scene, Side, Step};
use trpg_core::{CharacterId, LeadGender, LeadProfile};
use trpg_ui::audio::{AudioRequest, MusicCommand};
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;
use trpg_ui::screens::DialogueScreen;

/// Text boxes in the test scene, picking the first reply.
const TEST_BOXES: usize = 11;
/// Reply choices in the test scene: one press each picks the first reply.
const TEST_CHOICES: usize = 1;
/// Presses of Confirm that play the test scene through: two per text box
/// (reveal, move on), one per choice.
const TEST_PRESSES: usize = 2 * TEST_BOXES + TEST_CHOICES;

/// At the title, then the debug menu's "Play test scene".
fn full_screen() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F2 Down Down f");
    assert_eq!(h.screens(), ["title", "debug_menu", "dialogue"]);
    h
}

/// In Quick Battle (its `PLAYER PHASE` banner closed), then the debug
/// menu's "Play test scene (overlay)".
fn over_the_map() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down f f F2 Down Down Down f");
    assert_eq!(h.screens(), ["title", "battle", "dialogue"]);
    h
}

/// Presses `f` until the dialogue closes; returns how many presses it took.
fn play_to_the_end(h: &mut Harness) -> usize {
    for presses in 1..=100 {
        h.keys("f");
        if h.top_screen() != "dialogue" {
            return presses;
        }
    }
    panic!("the scene never ended");
}

/// The text of row `y`, trimmed.
fn row(h: &Harness, y: i32) -> String {
    let buf = h.game().buffer();
    (0..100)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

#[test]
fn the_full_screen_scene_plays_to_the_end() {
    let mut h = full_screen();
    // One press reveals each box, the next moves on.
    assert_eq!(play_to_the_end(&mut h), TEST_PRESSES);
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn the_overlay_scene_plays_to_the_end() {
    let mut h = over_the_map();
    assert_eq!(play_to_the_end(&mut h), TEST_PRESSES);
    assert_eq!(h.screens(), ["title", "battle"]);
}

#[test]
fn space_plays_the_scene_too() {
    let mut h = full_screen();
    for _ in 0..TEST_PRESSES {
        h.keys("Space");
    }
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn holding_space_fast_forwards() {
    let mut h = full_screen();
    h.keys("Space").hold("Space", 0.05);
    assert!(row(&h, 23).contains("You're late."));
}

#[test]
fn waiting_reveals_the_text_so_one_press_moves_on() {
    let mut h = full_screen();
    for _ in 0..TEST_BOXES + TEST_CHOICES {
        h.wait(2.0).keys("f");
    }
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn skipping_asks_first() {
    let mut h = full_screen();
    h.keys("d");
    assert!(row(&h, 23).starts_with("│   Skip scene?  "));
    assert!(row(&h, 24).starts_with("│   f yes / d no  "));
    // No: the scene goes on.
    h.keys("d");
    assert_eq!(h.top_screen(), "dialogue");
    h.keys("f f");
    assert!(row(&h, 21).contains(" Test Knight "));
    // Yes: it skips to the reply choice; after that, to the end.
    h.keys("d f");
    assert!(row(&h, 22).contains("Ellery! You came back for us."));
    h.keys("f d f");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

#[test]
fn holding_confirm_fast_forwards() {
    let mut normal = full_screen();
    normal.keys("f f").wait(0.05);
    let mut fast = full_screen();
    // The press that moves on to the knight's line stays held.
    fast.keys("f").hold("f", 0.05);
    assert!(row(&fast, 23).contains("You're late."));
    assert!(!row(&normal, 23).contains("You're late."));
}

/// The narration that opens the test scene, fully shown, under the caption.
#[test]
fn narration_snapshot() {
    let mut h = full_screen();
    h.keys("f");
    assert_snapshot!(h.snapshot());
}

/// The lord's first line, part-way through its reveal.
#[test]
fn mid_reveal_snapshot() {
    let mut h = full_screen();
    h.keys("f f f f").wait(0.1);
    assert!(row(&h, 23).starts_with("│   Better late t  "));
    assert_snapshot!(h.snapshot());
}

/// The knight's first line, fully shown, with the `▼`.
#[test]
fn fully_revealed_snapshot() {
    let mut h = full_screen();
    h.keys("f f f");
    assert_snapshot!(h.snapshot());
}

/// The lord answering, over the Quick Battle map.
#[test]
fn overlay_on_the_battle_map_snapshot() {
    let mut h = over_the_map();
    h.keys("f f f f f");
    assert_snapshot!(h.snapshot());
}

/// Reading on plays nothing; the skip prompt sounds like a menu (0425).
#[test]
fn only_the_skip_prompt_makes_sounds() {
    let mut h = full_screen();
    h.clear_audio().keys("f f Space");
    assert!(h.sounds().is_empty(), "{:?}", h.sounds());
    // Back opens the prompt, back again closes it, then yes skips.
    h.keys("d");
    assert_eq!(h.sounds(), ["menu_select"]);
    h.clear_audio().keys("d");
    assert_eq!(h.sounds(), ["menu_cancel"]);
    h.clear_audio().keys("d f");
    assert_eq!(h.sounds(), ["menu_select", "menu_select"]);
    // Skipping stopped at the reply choice; picking one sounds too.
    h.clear_audio().keys("f");
    assert_eq!(h.sounds(), ["menu_select"]);
    h.keys("d f");
    assert_eq!(h.top_screen(), "debug_menu");
}

/// Presses to reach the test scene's reply choice: two per text box
/// before it.
const TO_THE_CHOICE: usize = 2 * 8;

/// At the test scene's reply choice.
fn at_the_choice() -> Harness {
    let mut h = full_screen();
    for _ in 0..TO_THE_CHOICE {
        h.keys("f");
    }
    h
}

/// The three-reply choice of the test scene, with the default lead's name
/// in the archer's line.
#[test]
fn three_reply_choice_snapshot() {
    let h = at_the_choice();
    assert!(row(&h, 22).contains("Ellery! You came back for us."));
    assert_snapshot!(h.snapshot());
}

#[test]
fn the_cursor_keys_pick_a_reply() {
    let mut h = at_the_choice();
    h.clear_audio().keys("Down Down Up");
    assert_eq!(h.sounds(), ["menu_move"; 3]);
    h.keys("f");
    assert_eq!(
        h.sounds(),
        ["menu_move", "menu_move", "menu_move", "menu_select"]
    );
    h.keys("f");
    assert!(row(&h, 23).contains("I carry my own arrows, thank you."));
}

#[test]
fn skipping_stops_at_the_choice() {
    let mut h = full_screen();
    h.keys("d f");
    assert_eq!(h.top_screen(), "dialogue");
    assert!(row(&h, 22).contains("Ellery! You came back for us."));
    // Pick the blunt reply, then skip the rest.
    h.keys("Up f d f");
    assert_eq!(h.screens(), ["title", "debug_menu"]);
}

/// A two-reply choice, with a female lead named by the player speaking.
#[test]
fn two_reply_choice_snapshot() {
    let say = |who: &str, text: &str| Step::Say {
        speaker: CharacterId(who.into()),
        expression: None,
        text: text.into(),
    };
    let place = |side, who: &str| Step::Place {
        side,
        character: CharacterId(who.into()),
        expression: "neutral".into(),
    };
    let reply = |tone: &str, text: &str, reaction| ChoiceOption {
        tone: tone.into(),
        text: text.into(),
        steps: vec![say("test_knight", reaction)],
    };
    let scene = Scene {
        id: "two".into(),
        steps: vec![
            place(Side::Left, "lead"),
            place(Side::Right, "test_knight"),
            say("lead", "Where is {their} sword?"),
            say("test_knight", "{lead}, will you lead the charge?"),
            Step::Choice {
                options: vec![
                    reply("earnest", "I will. Stay close.", "Always."),
                    reply("wry", "Someone has to.", "Ha."),
                ],
            },
        ],
    };
    let lead = LeadProfile::new("Isolde", LeadGender::Female);
    let mut h = Harness::with_screen(Box::new(DialogueScreen::new(
        scene,
        lead,
        trpg_content::load_embedded()
            .map(|c| c.names)
            .unwrap_or_default(),
    )));
    h.keys("f f f f");
    assert!(row(&h, 23).contains("Isolde, will you lead the charge?"));
    assert_snapshot!(h.snapshot());
}

// --- Music (0710) -----------------------------------------------------------

/// A game playing a scene with these lines: narration `One.` to `Four.`,
/// with `@music village` before `Two.`, `@music talk_calm` before `Three.`
/// and nothing after it.
fn music_scene() -> Harness {
    let narrate = |text: &str| Step::Narrate { text: text.into() };
    let music = |cue: &str| Step::Music(MusicLine::Cue(cue.into()));
    let scene = Scene {
        id: "music".into(),
        steps: vec![
            narrate("One."),
            music("village"),
            narrate("Two."),
            music("talk_calm"),
            narrate("Three."),
            narrate("Four."),
        ],
    };
    let content = trpg_content::load_embedded();
    assert!(content.is_ok());
    let names = content.map(|c| c.names).unwrap_or_default();
    Harness::with_screen(Box::new(DialogueScreen::new(
        scene,
        LeadProfile::new("Ellery", LeadGender::Male),
        names,
    )))
}

/// The music requests of the run so far.
fn music_requests(h: &Harness) -> Vec<AudioRequest> {
    let music = |r: &AudioRequest| !matches!(r, AudioRequest::PlaySound { .. });
    h.audio_requests().into_iter().filter(music).collect()
}

fn play_music(cue: &str) -> AudioRequest {
    AudioRequest::PlayMusic { cue: cue.into() }
}

/// `@music` mid-scene asks for the music when the scene reaches that line,
/// not before, and the track starts.
#[test]
fn music_changes_when_the_scene_reaches_its_line() {
    let mut h = music_scene();
    h.wait(0.5);
    assert!(row(&h, 24).contains("One."));
    assert_eq!(music_requests(&h), []);
    h.keys("f").wait(0.5);
    assert!(row(&h, 24).contains("Two."));
    assert_eq!(music_requests(&h), [play_music("village")]);
    h.keys("f").wait(0.5);
    assert!(row(&h, 24).contains("Three."));
    assert_eq!(
        music_requests(&h),
        [play_music("village"), play_music("talk_calm")]
    );
    // Reading on asks for nothing more, and the mood track is what plays
    // once the village one has faded out.
    h.keys("f").wait(2.0);
    assert!(row(&h, 24).contains("Four."));
    assert_eq!(
        music_requests(&h),
        [play_music("village"), play_music("talk_calm")]
    );
    assert_eq!(h.game().music().current(), Some("talk_calm"));
    assert_eq!(h.music_clock().map(|c| c.cue.as_str()), Some("talk_calm"));
}

/// Skipping a scene with two `@music` lines asks only for the last, so the
/// music is the same as after watching it.
#[test]
fn skipping_asks_for_the_last_music_only() {
    let mut h = music_scene();
    h.keys("d f");
    assert_eq!(music_requests(&h), [play_music("talk_calm")]);
    let started: Vec<&str> = h
        .music_commands()
        .iter()
        .filter_map(|c| match c {
            MusicCommand::Start { cue, .. } => Some(cue.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(started, ["talk_calm"]);
    // The scene is over; its music plays on.
    assert_eq!(h.top_screen(), "");
    assert_eq!(h.game().music().current(), Some("talk_calm"));
}
