//! Tests of the dialogue screen: reveal timing, paging, skipping and the
//! layout. Scripted runs and snapshots are in `crates/ui/tests/it/dialogue.rs`.

use trpg_content::Step;
use trpg_core::CharacterId;

use super::*;
use crate::audio::AudioRequest;
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::Sprite;
use crate::portrait::dim_opacity;
use crate::screen::tests::ctx;

/// One frame of `dt` seconds with `actions`, Confirm held if `held`.
fn frame(actions: &[Action], dt: f32, held: bool) -> FrameInput {
    let held = if held { vec![Action::Confirm] } else { vec![] };
    FrameInput::new(actions.to_vec(), dt, held)
}

fn press(s: &mut DialogueScreen, c: &mut Ctx, action: Action) -> Transition {
    s.update(c, &frame(&[action], 0.0, false))
}

fn say(speaker: &str, text: &str) -> Step {
    Step::Say {
        speaker: CharacterId(speaker.into()),
        expression: None,
        text: text.into(),
        line: trpg_content::LineId::default(),
    }
}

fn place(side: Side, character: &str) -> Step {
    Step::Place {
        side,
        character: CharacterId(character.into()),
        expression: "neutral".into(),
    }
}

/// `scene` full-screen, with the default lead.
fn full(scene: Scene) -> DialogueScreen {
    DialogueScreen::new(scene, ctx().lead, ctx().content.names)
}

/// `scene` as an overlay, with the default lead.
fn over(scene: Scene) -> DialogueScreen {
    DialogueScreen::overlay(scene, ctx().lead, ctx().content.names)
}

fn scene(steps: Vec<Step>) -> Scene {
    Scene {
        id: "t".into(),
        steps,
    }
}

/// Lord on the left speaking to the knight on the right.
fn two_speakers(text: &str) -> Scene {
    scene(vec![
        place(Side::Left, "test_lord"),
        place(Side::Right, "test_knight"),
        say("test_lord", text),
        say("test_knight", "Second."),
    ])
}

fn draw(s: &DialogueScreen, c: &Ctx) -> GlyphBuffer {
    let p = &c.palette;
    let mut buf = GlyphBuffer::new(
        100,
        32,
        Cell::new('x', p.get(UiColor::Text), p.get(UiColor::Black)),
    );
    s.draw(c, &mut buf);
    buf
}

fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// Line counts of each page of the first box of a [`two_speakers`] scene,
/// pressing Confirm twice per page until the second box. Gives up after
/// 20 pages, so a paging bug fails the test instead of hanging it.
fn page_sizes(s: &mut DialogueScreen, c: &mut Ctx) -> Vec<usize> {
    let mut pages = vec![];
    for _ in 0..20 {
        if s.page_lines() == ["Second."] {
            return pages;
        }
        pages.push(s.page_lines().len());
        press(s, c, Action::Confirm);
        press(s, c, Action::Confirm);
    }
    panic!("never reached the second box; pages so far: {pages:?}");
}

#[test]
fn reveals_at_the_text_speed() {
    let mut c = ctx();
    let mut s = full(two_speakers("Hello there, knight."));
    assert_eq!(s.name(), "dialogue");
    assert!(!s.is_overlay());
    s.update(&mut c, &frame(&[], 0.1, false));
    assert!((s.shown - 6.0).abs() < 1e-3, "{}", s.shown);
    assert!(row(&draw(&s, &c), TEXT_Y).contains(" Hello "));
    assert!(!row(&draw(&s, &c), TEXT_Y).contains("Hello t"));
    c.text_speed = 10.0;
    s.update(&mut c, &frame(&[], 0.1, false));
    assert!((s.shown - 7.0).abs() < 1e-3, "{}", s.shown);
    // Never past the end.
    s.update(&mut c, &frame(&[], 10.0, false));
    assert!(s.is_revealed());
    assert!((s.shown - 20.0).abs() < 1e-3);
}

#[test]
fn holding_confirm_fast_forwards() {
    let mut c = ctx();
    let mut s = full(two_speakers(&"word ".repeat(50)));
    s.update(&mut c, &frame(&[], 0.1, true));
    assert!((s.shown - 36.0).abs() < 1e-3, "{}", s.shown);
}

#[test]
fn confirm_reveals_then_advances() {
    let mut c = ctx();
    let mut s = full(two_speakers("First."));
    assert!(!s.is_revealed());
    assert!(matches!(
        press(&mut s, &mut c, Action::Confirm),
        Transition::None
    ));
    assert!(s.is_revealed());
    assert_eq!(s.page_lines(), ["First."]);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(s.page_lines(), ["Second."]);
    assert!(!s.is_revealed(), "the next box starts hidden");
    press(&mut s, &mut c, Action::Confirm);
    let end = press(&mut s, &mut c, Action::Confirm);
    assert!(matches!(end, Transition::Pop));
    assert!(s.player().is_finished());
}

#[test]
fn end_turn_advances_like_confirm() {
    let mut c = ctx();
    let mut s = full(two_speakers("First."));
    press(&mut s, &mut c, Action::EndTurn);
    assert!(s.is_revealed());
    press(&mut s, &mut c, Action::EndTurn);
    assert_eq!(s.page_lines(), ["Second."]);
    // Held, it fast-forwards.
    let held = FrameInput::new(vec![], 0.05, vec![Action::EndTurn]);
    s.update(&mut c, &held);
    assert!(s.is_revealed(), "{}", s.shown);
    // It answers yes to the skip question.
    press(&mut s, &mut c, Action::Cancel);
    assert!(matches!(
        press(&mut s, &mut c, Action::EndTurn),
        Transition::Pop
    ));
}

#[test]
fn long_text_pages_three_lines_at_a_time() {
    let mut c = ctx();
    // 10 lines of 90 characters: four pages.
    let line = format!("{} ", "x".repeat(90));
    let mut s = full(two_speakers(&line.repeat(10)));
    let pages = page_sizes(&mut s, &mut c);
    assert_eq!(pages, [3, 3, 3, 1]);
}

#[test]
fn text_filling_whole_pages_has_no_empty_page_after() {
    let mut c = ctx();
    let line = format!("{} ", "x".repeat(90));
    let mut s = full(two_speakers(&line.repeat(6)));
    let pages = page_sizes(&mut s, &mut c);
    assert_eq!(pages, [3, 3]);
}

#[test]
fn two_lines_of_narration_start_at_the_top() {
    let c = ctx();
    let text = format!("{} {}", "a".repeat(60), "b".repeat(60));
    let mut s = full(scene(vec![Step::Narrate {
        text,
        line: trpg_content::LineId::default(),
    }]));
    s.shown = 120.0;
    let buf = draw(&s, &c);
    assert!(row(&buf, TEXT_Y).contains(&"a".repeat(60)));
    assert!(row(&buf, TEXT_Y + 1).contains(&"b".repeat(60)));
}

#[test]
fn cancel_asks_before_skipping() {
    let mut c = ctx();
    let mut s = full(two_speakers("First."));
    press(&mut s, &mut c, Action::Cancel);
    assert!(s.is_asking_skip());
    let buf = draw(&s, &c);
    assert!(row(&buf, TEXT_Y).contains("Skip scene?"));
    assert!(row(&buf, TEXT_Y + 1).contains("f yes / d no"));
    // Time stands still while asking.
    s.update(&mut c, &frame(&[], 1.0, false));
    assert!(!s.is_revealed());
    // No: back to the scene where it was.
    press(&mut s, &mut c, Action::Cancel);
    assert!(!s.is_asking_skip());
    assert_eq!(s.page_lines(), ["First."]);
    // Yes: the scene ends.
    press(&mut s, &mut c, Action::Cancel);
    assert!(matches!(
        press(&mut s, &mut c, Action::Confirm),
        Transition::Pop
    ));
}

#[test]
fn a_scene_without_text_pops_at_once() {
    let mut c = ctx();
    let mut s = full(scene(vec![place(Side::Left, "test_lord")]));
    assert!(matches!(
        s.update(&mut c, &frame(&[], 0.0, false)),
        Transition::Pop
    ));
}

#[test]
fn speaker_frame_is_double_and_bright() {
    let mut c = ctx();
    let mut s = full(two_speakers("Hi."));
    let buf = draw(&s, &c);
    let glyph = |b: &GlyphBuffer, x, y| b.get(x, y).map(|c| c.glyph);
    assert_eq!(glyph(&buf, LEFT_X, FRAME_Y), Some('╔'));
    assert_eq!(glyph(&buf, RIGHT_X, FRAME_Y), Some('┌'));
    assert_eq!(
        buf.get(LEFT_X, FRAME_Y).map(|c| c.fg),
        Some(c.palette.get(UiColor::PanelBorderFocus))
    );
    assert!(row(&buf, PLATE_Y).contains("Test Lord"));
    assert!(row(&buf, PLATE_Y).contains("Test Knight"));
    // The speaker's name is on the text box's top border.
    assert!(row(&buf, TEXT_BOX.y).starts_with("┌── Test Lord ─"));
    // Now the knight speaks: the frames swap.
    press(&mut s, &mut c, Action::Confirm);
    press(&mut s, &mut c, Action::Confirm);
    let buf = draw(&s, &c);
    assert_eq!(glyph(&buf, LEFT_X, FRAME_Y), Some('┌'));
    assert_eq!(glyph(&buf, RIGHT_X, FRAME_Y), Some('╔'));
    assert!(row(&buf, TEXT_BOX.y).starts_with("┌── Test Knight ─"));
}

/// The sprite of the portrait in the frame at column `x`.
fn portrait_sprite(buf: &GlyphBuffer, x: i32) -> Option<Sprite> {
    let left = (x + 1) * i32::from(CELL_W_PX);
    let inside = |s: &Sprite| (left..left + 256).contains(&s.dest.x);
    buf.sprites().into_iter().find(inside)
}

#[test]
fn the_speaker_is_bright_and_the_listener_dimmed_and_mirrored() {
    let c = ctx();
    let s = full(two_speakers("Hi."));
    let buf = draw(&s, &c);
    assert_eq!(buf.sprites().len(), 2);
    // The placeholders are 32×32: 8× fills the 256×256 px inside the frame.
    let top = (FRAME_Y + 1) * i32::from(CELL_H_PX);
    let listener = portrait_sprite(&buf, RIGHT_X).unwrap();
    assert_eq!(listener.image.path(), "portraits/test_knight/neutral.png");
    assert_eq!(listener.dest, Rect::new((RIGHT_X + 1) * 8, top, 256, 256));
    assert_eq!(listener.clip, listener.dest);
    assert!(listener.flip_x);
    assert_eq!(listener.opacity, dim_opacity(LISTENER_DIM));
    assert!(listener.opacity < 255);
    let speaker = portrait_sprite(&buf, LEFT_X).unwrap();
    assert_eq!(speaker.image.path(), "portraits/test_lord/neutral.png");
    assert_eq!(speaker.dest, Rect::new((LEFT_X + 1) * 8, top, 256, 256));
    assert!(!speaker.flip_x);
    assert_eq!(speaker.opacity, 255);
}

#[test]
fn narration_dims_both_and_centres_the_text() {
    let c = ctx();
    let s = full(scene(vec![
        place(Side::Left, "test_lord"),
        place(Side::Right, "test_knight"),
        Step::Narrate {
            text: "Rain.".into(),
            line: trpg_content::LineId::default(),
        },
    ]));
    let mut s = s;
    s.shown = 5.0;
    let buf = draw(&s, &c);
    assert_eq!(buf.get(LEFT_X, FRAME_Y).map(|c| c.glyph), Some('┌'));
    assert_eq!(buf.get(RIGHT_X, FRAME_Y).map(|c| c.glyph), Some('┌'));
    // No name on the border; one line, centred in the middle row.
    assert!(row(&buf, TEXT_BOX.y).starts_with("┌────"));
    let middle = row(&buf, TEXT_Y + 1);
    let x = middle.find("Rain.").map(|b| middle[..b].chars().count());
    assert_eq!(x, Some(47));
    assert_eq!(
        buf.get(47, TEXT_Y + 1).map(|c| c.fg),
        Some(c.palette.get(UiColor::TextDim))
    );
}

#[test]
fn an_empty_side_draws_nothing() {
    let c = ctx();
    let s = full(scene(vec![
        place(Side::Left, "test_lord"),
        say("test_lord", "Alone."),
    ]));
    let buf = draw(&s, &c);
    let black = c.palette.get(UiColor::Black);
    for y in 0..=PLATE_Y {
        for x in RIGHT_X..RIGHT_X + FRAME.0 {
            assert_eq!(buf.get(x, y).map(|c| (c.glyph, c.bg)), Some((' ', black)));
        }
    }
}

#[test]
fn the_overlay_leaves_the_rest_of_the_screen() {
    let c = ctx();
    let s = over(two_speakers("Hi."));
    assert!(s.is_overlay());
    let buf = draw(&s, &c);
    // Between the frames the screen below shows; below the text box (the
    // battle's key help) is blanked.
    assert_eq!(buf.get(50, 5).map(|c| c.glyph), Some('x'));
    assert_eq!(buf.get(50, 20).map(|c| c.glyph), Some('x'));
    for y in 28..32 {
        assert_eq!(row(&buf, y).trim(), "", "row {y}");
    }
    let full = draw(&full(two_speakers("Hi.")), &c);
    assert_eq!(full.get(50, 5).map(|c| c.glyph), Some(' '));
}

#[test]
fn the_arrow_blinks_once_revealed() {
    let mut c = ctx();
    let mut s = full(two_speakers("Hi."));
    let arrow_row = TEXT_BOX.y + TEXT_BOX.h - 2;
    assert!(!row(&draw(&s, &c), arrow_row).contains('▼'));
    press(&mut s, &mut c, Action::Confirm);
    assert!(row(&draw(&s, &c), arrow_row).ends_with("f ▼  │"));
    // Hidden from exactly half a second.
    s.update(&mut c, &frame(&[], 0.5, false));
    assert!(!row(&draw(&s, &c), arrow_row).contains('▼'));
    s.update(&mut c, &frame(&[], 0.1, false));
    assert!(!row(&draw(&s, &c), arrow_row).contains('▼'));
    assert!(row(&draw(&s, &c), arrow_row).contains(" f "));
    s.update(&mut c, &frame(&[], 0.5, false));
    assert!(row(&draw(&s, &c), arrow_row).contains('▼'));
}

#[test]
fn the_caption_is_top_centre() {
    let c = ctx();
    let s = full(scene(vec![
        Step::Caption {
            text: "Heth".into(),
        },
        say("nobody", "Hm."),
    ]));
    assert_eq!(row(&draw(&s, &c), 0).trim(), "Heth");
    assert_eq!(row(&draw(&s, &c), 0).find("Heth"), Some(48));
}

#[test]
fn a_character_without_a_portrait_gets_an_empty_frame() {
    let c = ctx();
    let s = full(scene(vec![
        place(Side::Right, "test_archer"),
        say("test_archer", "Me?"),
    ]));
    let buf = draw(&s, &c);
    assert_eq!(buf.get(RIGHT_X, FRAME_Y).map(|c| c.glyph), Some('╔'));
    let inside: String = row(&buf, FRAME_Y + 8).chars().skip(66).take(32).collect();
    assert_eq!(inside.trim(), "");
    assert!(row(&buf, PLATE_Y).contains("Test Archer"));
}

#[test]
fn a_speaker_who_is_not_a_unit_is_named_by_the_names_table() {
    let c = ctx();
    let name = c
        .content
        .names
        .get("retainer")
        .unwrap_or_default()
        .to_owned();
    assert!(!name.is_empty() && name != "retainer", "{name}");
    let s = full(scene(vec![
        place(Side::Left, "retainer"),
        say("retainer", "My lord."),
    ]));
    let buf = draw(&s, &c);
    assert!(row(&buf, PLATE_Y).contains(&name));
    assert!(!row(&buf, PLATE_Y).contains("retainer"));
    assert!(row(&buf, TEXT_BOX.y).starts_with(&format!("┌── {name} ─")));
}

#[test]
fn a_character_without_an_entry_is_named_by_id() {
    let c = ctx();
    let s = full(scene(vec![
        place(Side::Left, "stranger"),
        say("stranger", "Who, me?"),
    ]));
    let buf = draw(&s, &c);
    assert!(row(&buf, PLATE_Y).contains("stranger"));
    assert!(row(&buf, TEXT_BOX.y).starts_with("┌── stranger ─"));
}

// --- Reply choices (0708) ---------------------------------------------------

fn reply(text: &str, reaction: &str) -> trpg_content::ChoiceOption {
    trpg_content::ChoiceOption {
        tone: "t".into(),
        text: text.into(),
        line: trpg_content::LineId::default(),
        steps: vec![say("test_knight", reaction)],
    }
}

/// The knight asks; the lord's two replies; the knight answers after.
fn asking() -> Scene {
    scene(vec![
        place(Side::Left, "test_lord"),
        place(Side::Right, "test_knight"),
        say("test_knight", "Ready?"),
        Step::Choice {
            options: vec![reply("Yes.", "Good."), reply("{They} knows.", "Hm.")],
        },
        say("test_knight", "Go."),
    ])
}

/// The sound cues played since the last call.
fn sounds(c: &mut Ctx) -> Vec<String> {
    c.audio
        .take()
        .iter()
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect()
}

#[test]
fn a_choice_lists_its_replies_under_the_question() {
    let mut c = ctx();
    let mut s = full(asking());
    press(&mut s, &mut c, Action::Confirm);
    assert!(s.menu().is_none());
    // The press that opens the choice doesn't pick a reply, even with a
    // second press in the same frame.
    let t = s.update(
        &mut c,
        &frame(&[Action::Confirm, Action::Confirm], 0.0, false),
    );
    assert!(matches!(t, Transition::None));
    assert_eq!(s.menu().map(Menu::focus), Some(0));
    let buf = draw(&s, &c);
    // The question, fully shown, a blank row, then the replies, the
    // focused one marked; this fits the usual box, so it doesn't grow.
    assert_eq!(s.text_box(), (TEXT_BOX, TEXT_Y));
    assert!(s.is_revealed());
    assert!(row(&buf, TEXT_Y).starts_with("│   Ready?  "));
    assert_eq!(row(&buf, TEXT_Y + 1).trim_matches(['│', ' ']), "");
    assert!(row(&buf, TEXT_Y + 2).starts_with("│   > Yes.  "));
    assert!(row(&buf, TEXT_Y + 3).starts_with("│     He knows.  "));
    assert_eq!(
        buf.get(TEXT_X, TEXT_Y + 2).map(|c| c.fg),
        Some(c.palette.get(UiColor::TextHighlight))
    );
    assert_eq!(
        buf.get(TEXT_X, TEXT_Y + 3).map(|c| c.fg),
        Some(c.palette.get(UiColor::Text))
    );
    // No ▼ while choosing; the portraits' name plates stay clear.
    assert!(!row(&buf, TEXT_BOX.y + TEXT_BOX.h - 2).contains('▼'));
    assert!(row(&buf, PLATE_Y).contains("Test Lord"));
    // The focus marker follows the cursor.
    press(&mut s, &mut c, Action::CursorDown);
    let buf = draw(&s, &c);
    assert!(row(&buf, TEXT_Y + 2).starts_with("│     Yes.  "));
    assert!(row(&buf, TEXT_Y + 3).starts_with("│   > He knows.  "));
}

/// A long question and three replies: the box grows upward, its bottom
/// staying put.
#[test]
fn the_text_box_grows_to_fit_the_replies() {
    let mut c = ctx();
    let long = "word ".repeat(40);
    let mut s = full(scene(vec![
        place(Side::Left, "test_lord"),
        place(Side::Right, "test_knight"),
        say("test_knight", long.trim()),
        Step::Choice {
            options: vec![reply("A.", "a"), reply("B.", "b"), reply("C.", "c")],
        },
    ]));
    press(&mut s, &mut c, Action::Confirm);
    press(&mut s, &mut c, Action::Confirm);
    let lines = s.page_lines().len();
    assert_eq!(lines, 3);
    // 3 question rows, a blank one and 3 replies: 7 rows from row 20.
    let (rect, text_y) = s.text_box();
    assert_eq!(text_y, 20);
    assert_eq!(rect, Rect::new(0, 18, 100, 10));
    let buf = draw(&s, &c);
    assert!(row(&buf, 18).starts_with("┌── Test Knight ─"));
    assert!(row(&buf, 26).starts_with("│     C.  "));
    assert!(row(&buf, 27).starts_with("└──"));
}

#[test]
fn replies_are_picked_with_the_cursor_keys_and_confirm() {
    let mut c = ctx();
    let mut s = full(asking());
    press(&mut s, &mut c, Action::Confirm);
    press(&mut s, &mut c, Action::Confirm);
    sounds(&mut c);
    press(&mut s, &mut c, Action::CursorDown);
    assert_eq!(s.menu().map(Menu::focus), Some(1));
    press(&mut s, &mut c, Action::CursorDown);
    assert_eq!(s.menu().map(Menu::focus), Some(0));
    press(&mut s, &mut c, Action::CursorUp);
    assert_eq!(s.menu().map(Menu::focus), Some(1));
    assert_eq!(sounds(&mut c), ["menu_move"; 3]);
    // Cancel doesn't skip past a choice, and makes no sound.
    press(&mut s, &mut c, Action::Cancel);
    assert!(!s.is_asking_skip());
    assert!(sounds(&mut c).is_empty());
    // End turn confirms, like everywhere in dialogue.
    press(&mut s, &mut c, Action::EndTurn);
    assert_eq!(sounds(&mut c), ["menu_select"]);
    assert!(s.menu().is_none());
    assert_eq!(s.page_lines(), ["Hm."]);
    press(&mut s, &mut c, Action::Confirm);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(s.page_lines(), ["Go."]);
}

#[test]
fn skipping_stops_at_a_choice() {
    let mut c = ctx();
    let mut s = full(asking());
    press(&mut s, &mut c, Action::Cancel);
    let t = s.update(
        &mut c,
        &frame(&[Action::Confirm, Action::Confirm], 0.0, false),
    );
    assert!(matches!(t, Transition::None));
    assert!(!s.is_asking_skip());
    assert_eq!(s.menu().map(Menu::focus), Some(0));
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(s.page_lines(), ["Good."]);
    // After the choice, skipping ends the scene.
    press(&mut s, &mut c, Action::Cancel);
    assert!(matches!(
        press(&mut s, &mut c, Action::Confirm),
        Transition::Pop
    ));
}

#[test]
fn a_last_reply_with_nothing_after_ends_the_scene() {
    let mut c = ctx();
    let mut s = full(scene(vec![Step::Choice {
        options: vec![
            trpg_content::ChoiceOption {
                tone: "t".into(),
                text: "Bye.".into(),
                line: trpg_content::LineId::default(),
                steps: vec![],
            },
            reply("Stay.", "Ok."),
        ],
    }]));
    assert!(s.menu().is_some());
    assert!(matches!(
        press(&mut s, &mut c, Action::Confirm),
        Transition::Pop
    ));
}

#[test]
fn the_lead_shows_the_players_name_and_gendered_portrait() {
    let c = ctx();
    let lead_scene = || scene(vec![place(Side::Left, "lead"), say("lead", "Let's move.")]);
    for (gender, art) in [
        (trpg_core::LeadGender::Male, "lead_m"),
        (trpg_core::LeadGender::Female, "lead_f"),
    ] {
        let s = DialogueScreen::new(
            lead_scene(),
            LeadProfile::new("Isolde", gender),
            c.content.names.clone(),
        );
        let buf = draw(&s, &c);
        assert!(row(&buf, PLATE_Y).contains("Isolde"));
        assert!(row(&buf, TEXT_BOX.y).starts_with("┌── Isolde ─"));
        let sprite = portrait_sprite(&buf, LEFT_X).unwrap();
        assert_eq!(sprite.image.path(), format!("portraits/{art}/neutral.png"));
        assert_eq!((sprite.opacity, sprite.flip_x), (255, false));
    }
}

/// Only a press that opens a choice ends the frame's keys: two presses in
/// one frame otherwise both count.
#[test]
fn two_presses_in_one_frame_both_count() {
    let mut c = ctx();
    let mut s = full(two_speakers("First."));
    press(&mut s, &mut c, Action::Confirm);
    s.update(
        &mut c,
        &frame(&[Action::Confirm, Action::Confirm], 0.0, false),
    );
    assert_eq!(s.page_lines(), ["Second."]);
    assert!(s.is_revealed());
}

/// Replies with no line before them get no blank row above them.
#[test]
fn replies_without_a_question_start_at_the_top() {
    let c = ctx();
    let s = full(scene(vec![Step::Choice {
        options: vec![
            reply("A.", "a"),
            reply("B.", "b"),
            reply("C.", "c"),
            reply("D.", "d"),
        ],
    }]));
    assert_eq!(s.text_box(), (TEXT_BOX, TEXT_Y));
    let buf = draw(&s, &c);
    assert!(row(&buf, TEXT_Y).starts_with("│   > A.  "));
    assert!(row(&buf, TEXT_Y + 3).starts_with("│     D.  "));
}

fn music(cue: &str) -> Step {
    Step::Music(MusicLine::Cue(cue.into()))
}

fn narrate(text: &str) -> Step {
    Step::Narrate {
        text: text.into(),
        line: trpg_content::LineId::default(),
    }
}

/// The music requests made since the last call.
fn music_requests(c: &mut Ctx) -> Vec<AudioRequest> {
    let music = |r: &AudioRequest| !matches!(r, AudioRequest::PlaySound { .. });
    c.audio.take().into_iter().filter(music).collect()
}

fn play_music(cue: &str) -> AudioRequest {
    AudioRequest::PlayMusic { cue: cue.into() }
}

/// `@music` asks for the music when the scene reaches its line: the one
/// before the first text box on the first frame, a later one with the text
/// box after it, and `@music stop` fades the music out.
#[test]
fn music_changes_when_its_line_is_reached() {
    let mut c = ctx();
    let mut s = full(scene(vec![
        music("village"),
        narrate("One."),
        narrate("Two."),
        music("talk_calm"),
        narrate("Three."),
        Step::Music(MusicLine::Stop),
    ]));
    assert!(c.audio.pending().is_empty());
    s.update(&mut c, &frame(&[], 0.0, false));
    assert_eq!(music_requests(&mut c), [play_music("village")]);
    // Not again on later frames, nor when revealing or reading on to a
    // box without a @music before it.
    s.update(&mut c, &frame(&[], 0.0, false));
    press(&mut s, &mut c, Action::Confirm);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(s.page_lines(), ["Two."]);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(music_requests(&mut c), []);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(s.page_lines(), ["Three."]);
    assert_eq!(music_requests(&mut c), [play_music("talk_calm")]);
    // The line after the last text box is reached as the scene closes.
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(music_requests(&mut c), []);
    let t = press(&mut s, &mut c, Action::Confirm);
    assert!(matches!(t, Transition::Pop));
    assert_eq!(music_requests(&mut c), [AudioRequest::StopMusic]);
}

/// Skipping asks only for the last `@music` passed, and a scene without
/// one asks for nothing.
#[test]
fn skipping_asks_for_the_last_music_only() {
    let mut c = ctx();
    let mut s = full(scene(vec![
        narrate("One."),
        music("village"),
        narrate("Two."),
        music("talk_calm"),
        narrate("Three."),
    ]));
    press(&mut s, &mut c, Action::Cancel);
    let t = press(&mut s, &mut c, Action::Confirm);
    assert!(matches!(t, Transition::Pop));
    assert_eq!(music_requests(&mut c), [play_music("talk_calm")]);

    let mut s = full(two_speakers("First."));
    press(&mut s, &mut c, Action::Cancel);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(music_requests(&mut c), []);
}

/// A reply's reaction changes the music when it is picked; skipping to the
/// choice asks for the music before it.
#[test]
fn music_around_a_choice() {
    let mut c = ctx();
    let reply = |text: &str, steps| trpg_content::ChoiceOption {
        tone: "wry".into(),
        text: text.into(),
        line: trpg_content::LineId::default(),
        steps,
    };
    let mut s = full(scene(vec![
        narrate("One."),
        music("village"),
        narrate("Well?"),
        Step::Choice {
            options: vec![
                reply("Yes.", vec![music("scene_sad"), narrate("Oh.")]),
                reply("No.", vec![narrate("Good.")]),
            ],
        },
    ]));
    press(&mut s, &mut c, Action::Cancel);
    press(&mut s, &mut c, Action::Confirm);
    assert!(s.menu().is_some());
    assert_eq!(music_requests(&mut c), [play_music("village")]);
    press(&mut s, &mut c, Action::Confirm);
    assert_eq!(s.page_lines(), ["Oh."]);
    assert_eq!(music_requests(&mut c), [play_music("scene_sad")]);
}
