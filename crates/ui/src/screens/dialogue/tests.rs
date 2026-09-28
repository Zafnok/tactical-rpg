//! Tests of the dialogue screen: reveal timing, paging, skipping and the
//! layout. Scripted runs and snapshots are in `crates/ui/tests/dialogue.rs`.

use trpg_content::Step;
use trpg_core::CharacterId;

use super::*;
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
    }
}

fn place(side: Side, character: &str) -> Step {
    Step::Place {
        side,
        character: CharacterId(character.into()),
        expression: "neutral".into(),
    }
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

#[test]
fn reveals_at_the_text_speed() {
    let mut c = ctx();
    let mut s = DialogueScreen::new(two_speakers("Hello there, knight."));
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
    let mut s = DialogueScreen::new(two_speakers(&"word ".repeat(50)));
    s.update(&mut c, &frame(&[], 0.1, true));
    assert!((s.shown - 36.0).abs() < 1e-3, "{}", s.shown);
}

#[test]
fn confirm_reveals_then_advances() {
    let mut c = ctx();
    let mut s = DialogueScreen::new(two_speakers("First."));
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
fn long_text_pages_three_lines_at_a_time() {
    let mut c = ctx();
    // 10 lines of 90 characters: four pages.
    let line = format!("{} ", "x".repeat(90));
    let mut s = DialogueScreen::new(two_speakers(&line.repeat(10)));
    let mut pages = vec![];
    while s.page_lines() != ["Second."] {
        pages.push(s.page_lines().len());
        press(&mut s, &mut c, Action::Confirm);
        press(&mut s, &mut c, Action::Confirm);
    }
    assert_eq!(pages, [3, 3, 3, 1]);
}

#[test]
fn text_filling_whole_pages_has_no_empty_page_after() {
    let mut c = ctx();
    let line = format!("{} ", "x".repeat(90));
    let mut s = DialogueScreen::new(two_speakers(&line.repeat(6)));
    let mut pages = vec![];
    while s.page_lines() != ["Second."] {
        pages.push(s.page_lines().len());
        press(&mut s, &mut c, Action::Confirm);
        press(&mut s, &mut c, Action::Confirm);
    }
    assert_eq!(pages, [3, 3]);
}

#[test]
fn two_lines_of_narration_start_at_the_top() {
    let c = ctx();
    let text = format!("{} {}", "a".repeat(60), "b".repeat(60));
    let mut s = DialogueScreen::new(scene(vec![Step::Narrate { text }]));
    s.shown = 120.0;
    let buf = draw(&s, &c);
    assert!(row(&buf, TEXT_Y).contains(&"a".repeat(60)));
    assert!(row(&buf, TEXT_Y + 1).contains(&"b".repeat(60)));
}

#[test]
fn cancel_asks_before_skipping() {
    let mut c = ctx();
    let mut s = DialogueScreen::new(two_speakers("First."));
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
    let mut s = DialogueScreen::new(scene(vec![place(Side::Left, "test_lord")]));
    assert!(matches!(
        s.update(&mut c, &frame(&[], 0.0, false)),
        Transition::Pop
    ));
}

#[test]
fn speaker_frame_is_double_and_bright() {
    let mut c = ctx();
    let mut s = DialogueScreen::new(two_speakers("Hi."));
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

#[test]
fn portraits_are_dimmed_and_mirrored_like_the_renderer() {
    let c = ctx();
    let s = DialogueScreen::new(two_speakers("Hi."));
    let buf = draw(&s, &c);
    let bg = c.palette.get(UiColor::PanelBg);
    let mut expected = GlyphBuffer::new(34, 18, Cell::new(' ', bg, bg));
    let knight = &c.content.portraits["test_knight"];
    draw_portrait(
        &mut expected,
        &c.palette,
        (1, 1),
        knight,
        "neutral",
        LISTENER_DIM,
        true,
    );
    for y in 1..17 {
        for x in 1..33 {
            assert_eq!(
                buf.get(RIGHT_X + x, FRAME_Y + y),
                expected.get(x, y),
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn narration_dims_both_and_centres_the_text() {
    let c = ctx();
    let s = DialogueScreen::new(scene(vec![
        place(Side::Left, "test_lord"),
        place(Side::Right, "test_knight"),
        Step::Narrate {
            text: "Rain.".into(),
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
    let s = DialogueScreen::new(scene(vec![
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
    let s = DialogueScreen::overlay(two_speakers("Hi."));
    assert!(s.is_overlay());
    let buf = draw(&s, &c);
    // Between the frames the screen below shows; below the text box (the
    // battle's key help) is blanked.
    assert_eq!(buf.get(50, 5).map(|c| c.glyph), Some('x'));
    assert_eq!(buf.get(50, 20).map(|c| c.glyph), Some('x'));
    for y in 28..32 {
        assert_eq!(row(&buf, y).trim(), "", "row {y}");
    }
    let full = draw(&DialogueScreen::new(two_speakers("Hi.")), &c);
    assert_eq!(full.get(50, 5).map(|c| c.glyph), Some(' '));
}

#[test]
fn the_arrow_blinks_once_revealed() {
    let mut c = ctx();
    let mut s = DialogueScreen::new(two_speakers("Hi."));
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
    let s = DialogueScreen::new(scene(vec![
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
    let s = DialogueScreen::new(scene(vec![
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
fn a_character_without_an_entry_is_named_by_id() {
    let c = ctx();
    let s = DialogueScreen::new(scene(vec![
        place(Side::Left, "stranger"),
        say("stranger", "Who, me?"),
    ]));
    let buf = draw(&s, &c);
    assert!(row(&buf, PLATE_Y).contains("stranger"));
    assert!(row(&buf, TEXT_BOX.y).starts_with("┌── stranger ─"));
}
