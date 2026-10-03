//! Tests of the `.dlg` parser, validator and printer.

use proptest::prelude::*;

use super::*;

fn id(s: &str) -> CharacterId {
    CharacterId(s.into())
}

fn say(speaker: &str, expression: Option<&str>, text: &str) -> Step {
    Step::Say {
        speaker: id(speaker),
        expression: expression.map(Into::into),
        text: text.into(),
        line: LineId::default(),
    }
}

fn place(side: Side, character: &str, expression: &str) -> Step {
    Step::Place {
        side,
        character: id(character),
        expression: expression.into(),
    }
}

fn narrate(text: &str) -> Step {
    Step::Narrate {
        text: text.into(),
        line: LineId::default(),
    }
}

/// `steps` without their line ids, to compare with steps built by hand.
fn bare(steps: &[Step]) -> Vec<Step> {
    let mut steps = steps.to_vec();
    for step in &mut steps {
        match step {
            Step::Say { line, .. } | Step::Narrate { line, .. } => *line = LineId::default(),
            Step::Choice { options } => {
                for o in options {
                    o.line = LineId::default();
                    o.steps = bare(&o.steps);
                }
            }
            _ => {}
        }
    }
    steps
}

fn music(cue: &str) -> Step {
    Step::Music(MusicLine::Cue(cue.into()))
}

fn characters() -> CharacterTable {
    crate::character::load(None, None, None).unwrap_or_default()
}

fn names() -> Names {
    crate::names::load().unwrap_or_default()
}

/// Every error for one file `t.dlg`, checked against the embedded
/// (placeholder) characters and the names table.
fn errors(src: &str) -> Vec<String> {
    from_sources(
        &[("t.dlg", src)],
        Some(&characters()),
        None,
        Some(&names()),
        None,
    )
    .err()
    .unwrap_or_default()
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// The scenes of `src`, parsed without checks, and without their line
/// ids (to compare with scenes built by hand).
fn scenes(src: &str) -> Vec<Scene> {
    let (parsed, errors) = parse_dlg("t.dlg", src);
    assert_eq!(errors, []);
    let bare = |s: Scene| Scene {
        steps: bare(&s.steps),
        ..s
    };
    parsed.into_iter().map(|p| bare(p.scene)).collect()
}

/// A valid scene around `body`: the lord on the left, the knight on the
/// right.
fn scene(body: &str) -> String {
    format!("@scene s\n@left test_lord neutral\n@right test_knight neutral\n{body}\n@end\n")
}

#[test]
fn parses_every_line_type() {
    let src = "\
# a comment
@scene ch01

@caption Village of Heth, dusk
  # an indented comment
@left  ana neutral
@right bors angry
bors: You're late.
ana[happy]: Better late: than... well.
>   The rain had not stopped.
@right clear
@end
";
    assert_eq!(
        scenes(src),
        [Scene {
            id: "ch01".into(),
            steps: vec![
                Step::Caption {
                    text: "Village of Heth, dusk".into()
                },
                place(Side::Left, "ana", "neutral"),
                place(Side::Right, "bors", "angry"),
                say("bors", None, "You're late."),
                say("ana", Some("happy"), "Better late: than... well."),
                narrate("The rain had not stopped."),
                Step::Clear { side: Side::Right },
            ],
        }]
    );
}

#[test]
fn records_the_line_of_every_step() {
    let (parsed, _) = parse_dlg("t.dlg", "\n@scene a\n\n> one\n  more\nx: two\n@end\n");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].line, 2);
    assert_eq!(parsed[0].step_lines, [4, 6]);
    assert_eq!(parsed[0].file, "t.dlg");
}

#[test]
fn continuation_lines_join_with_one_space() {
    let src = "@scene a\nx: One\n  two   \n     three\n> Four\n  five\n\n  six\n@end\n";
    assert_eq!(
        scenes(src)[0].steps,
        [say("x", None, "One two three"), narrate("Four five six")]
    );
}

#[test]
fn several_scenes_per_file_and_crlf() {
    let src = "@scene a\r\n> One\r\n@end\r\n\r\n@scene b\r\n> Two\r\n  more\r\n@end";
    assert_eq!(
        scenes(src),
        [
            Scene {
                id: "a".into(),
                steps: vec![narrate("One")]
            },
            Scene {
                id: "b".into(),
                steps: vec![narrate("Two more")]
            },
        ]
    );
}

#[test]
fn a_valid_scene_loads() {
    let src = scene("test_lord[sad]: Hi.\n@left clear\n@left test_archer happy\ntest_archer: Yo.");
    let table = from_sources(
        &[("t.dlg", src.as_str())],
        Some(&characters()),
        None,
        None,
        None,
    );
    assert_eq!(
        table.map(|t| t.get("s").map(|s| s.steps.len())),
        Ok(Some(6))
    );
}

/// Characters aren't checked without the character table.
#[test]
fn without_characters_ids_are_not_checked() {
    let src = "@scene a\n@left nobody neutral\nnobody: Hi.\n@end\n";
    assert!(from_sources(&[("t.dlg", src)], None, None, None, None).is_ok());
    assert_eq!(
        errors(src),
        [
            "t.dlg:2: unknown character \"nobody\"",
            "t.dlg:3: unknown character \"nobody\"",
        ]
    );
}

// --- Syntax errors ---------------------------------------------------------

#[test]
fn unknown_directive() {
    assert_eq!(
        errors(&scene("@center test_lord neutral\n> x")),
        ["t.dlg:4: unknown directive \"@center\""]
    );
}

#[test]
fn directive_arguments() {
    assert_eq!(
        errors("@scene\n> x\n@end\n@scene a b\n> x\n@end x\n"),
        [
            "t.dlg:1: @scene needs one scene id",
            "t.dlg:4: @scene needs one scene id",
            "t.dlg:6: @end takes nothing after it",
        ]
    );
    assert_eq!(
        errors(&scene(
            "@caption\n@left test_lord\n@right a b c\n@left clear now\n> x"
        )),
        [
            "t.dlg:4: @caption has no text",
            "t.dlg:5: @left needs a character id and an expression, or \"clear\"",
            "t.dlg:6: @right needs a character id and an expression, or \"clear\"",
            "t.dlg:7: @left needs a character id and an expression, or \"clear\"",
        ]
    );
}

#[test]
fn invalid_ids() {
    assert_eq!(
        errors("@scene Ch-1\n@left Bors Angry\nBors[Mad]: Hi.\n@end\n"),
        [
            "t.dlg:1: \"Ch-1\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:2: \"Bors\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:2: \"Angry\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:2: unknown character \"Bors\"",
            "t.dlg:2: unknown expression \"Angry\"; use one of neutral, happy, angry, sad, surprised",
            "t.dlg:3: \"Bors\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:3: \"Mad\" is not a valid id; use lowercase letters, digits and _",
            "t.dlg:3: unknown character \"Bors\"",
            "t.dlg:3: unknown expression \"Mad\"; use one of neutral, happy, angry, sad, surprised",
        ]
    );
}

#[test]
fn unrecognised_lines() {
    assert_eq!(
        errors(&scene("Hello there: friend\nJust some words\n> x")),
        [
            "t.dlg:4: unrecognised line; expected \"@directive\", \"> narration\", \"speaker: text\" or \"# comment\"",
            "t.dlg:5: unrecognised line; expected \"@directive\", \"> narration\", \"speaker: text\" or \"# comment\"",
        ]
    );
}

#[test]
fn empty_text() {
    assert_eq!(
        errors(&scene(">\ntest_lord:   \ntest_lord[sad]:\n> x")),
        [
            "t.dlg:4: narration has no text",
            "t.dlg:5: \"test_lord:\" has no text",
            "t.dlg:6: \"test_lord[sad]:\" has no text",
        ]
    );
}

#[test]
fn misplaced_indented_lines() {
    assert_eq!(
        errors(&scene("  stray\n@caption c\n  more\n> x\n  @left clear")),
        [
            "t.dlg:4: indented line doesn't continue a speech or narration line",
            "t.dlg:6: indented line doesn't continue a speech or narration line",
            "t.dlg:8: directives can't be indented",
        ]
    );
    // A speech outside a scene can't be continued either.
    assert_eq!(
        errors("x: hi\n  more\n"),
        [
            "t.dlg:1: line is outside a scene; start one with @scene <id>",
            "t.dlg:2: indented line doesn't continue a speech or narration line",
        ]
    );
}

#[test]
fn lines_outside_scenes() {
    assert_eq!(
        errors("> before\n@scene a\n> x\n@end\n@left test_lord neutral\n@end\n"),
        [
            "t.dlg:1: line is outside a scene; start one with @scene <id>",
            "t.dlg:5: line is outside a scene; start one with @scene <id>",
            "t.dlg:6: @end without an open @scene",
        ]
    );
}

#[test]
fn missing_end() {
    assert_eq!(
        errors("@scene a\n> x\n@scene b\n> y\n"),
        [
            "t.dlg:1: scene \"a\" has no @end",
            "t.dlg:3: scene \"b\" has no @end",
        ]
    );
}

#[test]
fn non_ascii_text() {
    assert_eq!(
        errors(&scene(
            "test_lord: \u{2018}Hi\u{2019} \u{201C}you\u{201D}\u{2026}\n> A\u{2014}B\u{2013}C\n  x\u{e9}\n@caption Heth\u{a0}x\ntest_knight[sad]: a\tb"
        )),
        [
            "t.dlg:4:12: non-ASCII punctuation '\u{2018}'; use ' instead",
            "t.dlg:4:15: non-ASCII punctuation '\u{2019}'; use ' instead",
            "t.dlg:4:17: non-ASCII punctuation '\u{201C}'; use \" instead",
            "t.dlg:4:21: non-ASCII punctuation '\u{201D}'; use \" instead",
            "t.dlg:4:22: non-ASCII punctuation '\u{2026}'; use ... instead",
            "t.dlg:5:4: non-ASCII punctuation '\u{2014}'; use -- instead",
            "t.dlg:5:6: non-ASCII punctuation '\u{2013}'; use - instead",
            "t.dlg:6:4: unsupported character '\u{e9}'; dialogue text is plain ASCII",
            "t.dlg:7:14: unsupported character '\\u{a0}'; dialogue text is plain ASCII",
            "t.dlg:8:20: unsupported character '\\t'; dialogue text is plain ASCII",
        ]
    );
}

// --- Scene checks ----------------------------------------------------------

#[test]
fn unknown_characters() {
    assert_eq!(
        errors(&scene("@left bors neutral\nbors: Hi.\nmira: Hey.")),
        [
            "t.dlg:4: unknown character \"bors\"",
            "t.dlg:5: unknown character \"bors\"",
            "t.dlg:6: unknown character \"mira\"",
        ]
    );
}

#[test]
fn speakers_who_are_not_units_may_be_placed_and_speak() {
    let src = "@scene s\n@left retainer neutral\n@right rival angry\n\
               retainer: Hold.\nrival[happy]: No.\n@end\n";
    assert_eq!(errors(src), [] as [&str; 0]);
    let characters = characters();
    assert!(!characters.speakers.is_empty());
    let (parsed, parse_errors) = parse_dlg("t.dlg", src);
    assert_eq!(parse_errors, []);
    assert_eq!(parsed.len(), 1);
    for p in &parsed {
        assert_eq!(
            check_scene(p, Some(&characters), None, Some(&names()), None),
            []
        );
    }
    // Someone in neither list still fails, next to a speaker.
    assert_eq!(
        errors("@scene s\n@left retainer neutral\n@right mira neutral\nmira: Hey.\n@end\n"),
        [
            "t.dlg:3: unknown character \"mira\"",
            "t.dlg:4: unknown character \"mira\"",
        ]
    );
}

#[test]
fn speaker_not_on_screen() {
    assert_eq!(
        errors(&scene(
            "test_archer: Hi.\n@right clear\ntest_knight[sad]: Bye.\ntest_lord: Ok."
        )),
        [
            "t.dlg:4: \"test_archer\" speaks but is not on screen; place them with @left or @right first",
            "t.dlg:6: \"test_knight\" speaks but is not on screen; place them with @left or @right first",
        ]
    );
}

#[test]
fn same_character_on_both_sides() {
    assert_eq!(
        errors(&scene(
            "@right test_lord happy\n@left test_knight neutral\n> x"
        )),
        ["t.dlg:4: \"test_lord\" is already on the left; a character can't be on both sides",]
    );
    // Replacing yourself on the same side, or crossing over after leaving,
    // is fine.
    assert_eq!(
        errors(&scene(
            "@left test_lord sad\n@left clear\n@right test_lord happy\n> x"
        )),
        Vec::<String>::new()
    );
}

/// Every error for `t.dlg`, with expressions checked against `portraits`.
fn errors_with(src: &str, portraits: &PortraitTable) -> Vec<String> {
    from_sources(
        &[("t.dlg", src)],
        Some(&characters()),
        Some(portraits),
        None,
        None,
    )
    .err()
    .unwrap_or_default()
    .iter()
    .map(ToString::to_string)
    .collect()
}

#[test]
fn expressions_come_from_the_portrait() {
    let images = crate::ImageTable::load().unwrap_or_default();
    let mut portraits = crate::portrait::load_all(&images).unwrap_or_default();
    // The lord's portrait gains a "smug" expression and loses "sad".
    let lord = portraits.get_mut("test_lord").map(|p| &mut p.expressions);
    if let Some(expressions) = lord {
        expressions.retain(|e| e.name != "sad");
        let smug = expressions
            .first()
            .cloned()
            .map(|e| crate::portrait::Expression {
                name: "smug".into(),
                ..e
            });
        expressions.extend(smug);
    }
    let src = scene(
        "@left test_lord smug
test_lord[sad]: Hi.
test_knight[smug]: Hi.",
    );
    assert_eq!(
        errors_with(&src, &portraits),
        [
            "t.dlg:5: \"test_lord\"'s portrait has no expression \"sad\"; use one of neutral, happy, angry, surprised, smug",
            "t.dlg:6: \"test_knight\"'s portrait has no expression \"smug\"; use one of neutral, happy, angry, sad, surprised",
        ]
    );
    // Characters without a portrait keep the standard five.
    portraits.clear();
    assert_eq!(
        errors_with(&src, &portraits),
        [
            "t.dlg:4: unknown expression \"smug\"; use one of neutral, happy, angry, sad, surprised",
            "t.dlg:6: unknown expression \"smug\"; use one of neutral, happy, angry, sad, surprised",
        ]
    );
    assert_eq!(STANDARD_EXPRESSIONS, crate::portrait::REQUIRED_EXPRESSIONS);
}

#[test]
fn unknown_expressions() {
    assert_eq!(
        errors(&scene("@left test_lord smug\ntest_knight[furious]: Hi.")),
        [
            "t.dlg:4: unknown expression \"smug\"; use one of neutral, happy, angry, sad, surprised",
            "t.dlg:5: unknown expression \"furious\"; use one of neutral, happy, angry, sad, surprised",
        ]
    );
    for e in STANDARD_EXPRESSIONS {
        assert_eq!(
            errors(&scene(&format!("test_lord[{e}]: Hi."))),
            Vec::<String>::new()
        );
    }
}

#[test]
fn text_length_limit() {
    let at_limit = "a".repeat(MAX_TEXT_LEN);
    assert_eq!(
        errors(&scene(&format!("test_lord: {at_limit}\n> {at_limit}"))),
        Vec::<String>::new()
    );
    // Continuation lines count, joined with a space.
    let half = "b".repeat(MAX_TEXT_LEN / 2);
    assert_eq!(
        errors(&scene(&format!(
            "test_lord: {at_limit}x\n> {half}\n  {half}"
        ))),
        [
            "t.dlg:4: text is 201 characters; the limit is 200",
            "t.dlg:5: text is 201 characters; the limit is 200",
        ]
    );
}

#[test]
fn scene_without_text() {
    assert_eq!(
        errors(
            "@scene a\n@caption Only a caption\n@left test_lord neutral\n@end\n@scene b\n@end\n"
        ),
        [
            "t.dlg:1: scene \"a\" has no speech or narration",
            "t.dlg:5: scene \"b\" has no speech or narration",
        ]
    );
}

#[test]
fn duplicate_scene_ids_across_files() {
    let a = "@scene one\n> x\n@end\n@scene two\n> x\n@end\n";
    let b = "\n@scene two\n> y\n@end\n@scene one\n> y\n@end\n";
    let errs: Vec<String> = from_sources(&[("a.dlg", a), ("b.dlg", b)], None, None, None, None)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errs,
        [
            "b.dlg:2: duplicate scene id \"two\"; first used at a.dlg:4",
            "b.dlg:5: duplicate scene id \"one\"; first used at a.dlg:1",
        ]
    );
}

#[test]
fn every_error_is_reported_in_line_order() {
    let src = "@scene a\n@left nobody neutral\n@caption \u{2019}\n> x\n";
    assert_eq!(
        errors(src),
        [
            "t.dlg:1: scene \"a\" has no @end",
            "t.dlg:2: unknown character \"nobody\"",
            "t.dlg:3:10: non-ASCII punctuation '\u{2019}'; use ' instead",
        ]
    );
}

#[test]
fn files_that_are_not_utf8() {
    let ok = "@scene a
> x
@end
";
    let bad = "@scene b
@end
";
    let load = |files: &[(&str, Option<&str>)]| {
        let files: Vec<(String, Option<&str>)> =
            files.iter().map(|&(f, s)| (f.to_owned(), s)).collect();
        load_files(&files, None, None, None, None)
            .map_err(|e| e.iter().map(ToString::to_string).collect::<Vec<_>>())
    };
    assert_eq!(load(&[("a.dlg", Some(ok))]).map(|t| t.scenes.len()), Ok(1));
    assert_eq!(
        load(&[("a.dlg", Some(ok)), ("x.dlg", None)]).map(|t| t.scenes.len()),
        Err(vec!["x.dlg: file is not valid UTF-8".to_owned()])
    );
    assert_eq!(
        load(&[("x.dlg", None), ("b.dlg", Some(bad))]).map(|t| t.scenes.len()),
        Err(vec![
            "x.dlg: file is not valid UTF-8".to_owned(),
            "b.dlg:1: scene \"b\" has no speech or narration".to_owned(),
        ])
    );
}

mod choice;
mod line_ids;
mod names;

// --- Embedded files ---------------------------------------------------------

#[test]
fn embedded_test_scene_loads() {
    let table = load(Some(&characters()), None, Some(&names()), None);
    assert!(table.is_ok(), "{table:?}");
    let steps = table
        .ok()
        .and_then(|t| t.get("test").map(|s| bare(&s.steps)));
    assert_eq!(
        steps.as_ref().map(|s| s[..4].to_vec()),
        Some(vec![
            Step::Caption {
                text: "Village of Heth, dusk".into()
            },
            narrate("The rain had not stopped for three days."),
            place(Side::Left, "test_lord", "neutral"),
            place(Side::Right, "test_knight", "angry"),
        ])
    );
    assert_eq!(
        steps.and_then(|s| s.get(6).cloned()),
        Some(say(
            "test_knight",
            None,
            "Than never. Say it. I've heard it from you often enough."
        ))
    );
}

/// The full example in `assets/dialogue/README.md` parses and passes every
/// check except the character ids (its cast isn't in `characters.ron`). Its
/// `@music` lines name real music cues.
#[test]
fn readme_example_is_valid() {
    let readme = bundle::file("dialogue/README.md").unwrap_or_default();
    let example = readme.split("```").nth(1).unwrap_or_default();
    let audio = crate::audio::load().ok();
    assert!(audio.is_some());
    let table = from_sources(&[("README.md", example)], None, None, None, audio.as_ref());
    assert!(table.is_ok(), "{table:?}");
    let table = table.unwrap_or_default();
    let steps = &table.scenes["ch01_opening"].steps;
    assert!(steps.contains(&music("talk_calm")));
    assert!(steps.contains(&Step::Music(MusicLine::Stop)));
    assert_eq!(
        table.scenes.keys().collect::<Vec<_>>(),
        ["ch01_after_battle", "ch01_opening"]
    );
}

#[test]
fn side_names() {
    assert_eq!(Side::Left.name(), "left");
    assert_eq!(Side::Right.name(), "right");
    assert_eq!(Side::Left.other(), Side::Right);
    assert_eq!(Side::Right.other(), Side::Left);
}

#[test]
fn step_text() {
    assert_eq!(say("a", None, "hi").text(), Some("hi"));
    assert_eq!(narrate("n").text(), Some("n"));
    assert_eq!(Step::Caption { text: "c".into() }.text(), None);
    assert_eq!(place(Side::Left, "a", "sad").text(), None);
    assert_eq!(Step::Clear { side: Side::Left }.text(), None);
    assert_eq!(music("talk_calm").text(), None);
    assert_eq!(Step::Music(MusicLine::Stop).text(), None);
}

// --- Music ------------------------------------------------------------------

/// Every error for `t.dlg`, with `@music` cues checked against the
/// embedded audio manifest.
fn errors_with_audio(src: &str) -> Vec<String> {
    let audio = crate::audio::load().unwrap_or_default();
    from_sources(&[("t.dlg", src)], None, None, None, Some(&audio))
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn music_lines_parse_anywhere_in_a_scene() {
    let src = "\
@scene s
@music talk_calm
> One.
@music  stop
@choice
* wry: Hm.
  @music scene_sad
  > Two.
* blunt: No.
@endchoice
@music talk_calm
@end
";
    let (parsed, errors) = parse_dlg("t.dlg", src);
    assert_eq!(errors, []);
    assert_eq!(
        bare(&parsed[0].scene.steps),
        [
            music("talk_calm"),
            narrate("One."),
            Step::Music(MusicLine::Stop),
            Step::Choice {
                options: vec![
                    ChoiceOption {
                        tone: "wry".into(),
                        text: "Hm.".into(),
                        line: LineId::default(),
                        steps: vec![music("scene_sad"), narrate("Two.")],
                    },
                    ChoiceOption {
                        tone: "blunt".into(),
                        text: "No.".into(),
                        line: LineId::default(),
                        steps: vec![],
                    },
                ],
            },
            music("talk_calm"),
        ]
    );
    assert_eq!(parsed[0].step_lines, [2, 3, 4, 5, 11]);
    assert_eq!(parsed[0].choice_lines[0].options[0].step_lines, [7, 8]);
    assert_eq!(errors_with_audio(src), [] as [&str; 0]);
}

#[test]
fn music_needs_exactly_one_argument() {
    let needs = "@music needs one music cue id, or \"stop\"";
    assert_eq!(
        errors(&scene(
            "@music\n@music talk_calm scene_sad\n@music stop now\n@music Talk\n> x"
        )),
        [
            format!("t.dlg:4: {needs}"),
            format!("t.dlg:5: {needs}"),
            format!("t.dlg:6: {needs}"),
            "t.dlg:7: \"Talk\" is not a valid id; use lowercase letters, digits and _".to_owned(),
        ]
    );
}

/// The music example in `assets/dialogue/README.md` is valid and names
/// real music cues.
#[test]
fn readme_music_example_is_valid() {
    let readme = bundle::file("dialogue/README.md").unwrap_or_default();
    let example = readme.split("```").nth(3).unwrap_or_default();
    assert_eq!(errors_with_audio(example), [] as [&str; 0]);
    let (parsed, _) = parse_dlg("README.md", example);
    assert_eq!(parsed[0].scene.id, "ch01_bad_news");
    assert_eq!(parsed[0].scene.steps[0], music("talk_calm"));
    assert_eq!(parsed[0].scene.steps[4], music("scene_sad"));
}

#[test]
fn music_line_arguments() {
    assert_eq!(MusicLine::Cue("talk_calm".into()).arg(), "talk_calm");
    assert_eq!(MusicLine::Stop.arg(), "stop");
    assert_eq!(MusicLine::STOP, "stop");
}

#[test]
fn music_prints_as_written() {
    let scene = Scene {
        id: "s".into(),
        steps: vec![
            music("talk_calm"),
            narrate("x"),
            Step::Music(MusicLine::Stop),
        ],
    };
    let printed = print_scene(&scene);
    assert_eq!(
        printed,
        "@scene s\n@music talk_calm\n> x\n@music stop\n@end\n"
    );
    assert_eq!(scenes(&printed), [scene]);
}

/// `@music` must name a music cue of `assets/audio/audio.ron`: not an
/// unknown id, a sound or a music pool. In a reply's reaction too.
#[test]
fn music_must_be_a_music_cue() {
    let listed = "music cues are listed in assets/audio/audio.ron";
    let src = "\
@scene s
@music no_such_cue
@music menu_move
@music skirmish
@music talk_calm
@music stop
> x
@choice
* wry: Hm.
  @music heal
* blunt: No.
@endchoice
@end
";
    assert_eq!(
        errors_with_audio(src),
        [
            format!("t.dlg:2: \"no_such_cue\" is not a music cue; {listed}"),
            format!("t.dlg:3: \"menu_move\" is a sound, not a music cue; {listed}"),
            format!("t.dlg:4: \"skirmish\" is a music pool, not a music cue; {listed}"),
            format!("t.dlg:10: \"heal\" is a sound, not a music cue; {listed}"),
        ]
    );
    // Without an audio manifest (it failed to load) cues aren't checked.
    assert_eq!(errors(src), [] as [&str; 0]);
}

/// Every embedded script's `@music` cues are checked when the game's
/// content loads.
#[test]
fn the_embedded_scripts_are_checked_against_the_audio_manifest() {
    let mut audio = crate::audio::load().unwrap_or_default();
    assert!(load(None, None, None, Some(&audio)).is_ok());
    let files = [(
        "t.dlg".to_owned(),
        Some("@scene s\n@music talk_calm\n> x\n@end\n"),
    )];
    assert!(load_files(&files, None, None, None, Some(&audio)).is_ok());
    audio.music.remove("talk_calm");
    let errs = load_files(&files, None, None, None, Some(&audio)).err();
    assert_eq!(errs.map(|e| e.len()), Some(1));
}

// --- Round trip -------------------------------------------------------------

fn arb_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,8}".prop_filter("reserved", |s| s != "clear")
}

/// Printable ASCII, no spaces at either end.
fn arb_text() -> impl Strategy<Value = String> {
    "[!-~]([ -~]{0,40}[!-~])?"
}

fn arb_side() -> impl Strategy<Value = Side> {
    prop_oneof![Just(Side::Left), Just(Side::Right)]
}

/// Any step but a choice.
fn arb_simple_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        arb_text().prop_map(|text| Step::Caption { text }),
        (arb_side(), arb_id(), arb_id()).prop_map(|(side, c, expression)| Step::Place {
            side,
            character: CharacterId(c),
            expression,
        }),
        arb_side().prop_map(|side| Step::Clear { side }),
        (arb_id(), proptest::option::of(arb_id()), arb_text()).prop_map(|(s, expression, text)| {
            Step::Say {
                speaker: CharacterId(s),
                expression,
                text,
                line: LineId::default(),
            }
        }),
        arb_text().prop_map(|text| Step::Narrate {
            text,
            line: LineId::default(),
        }),
        // A cue named "stop" would print as `@music stop`.
        arb_id()
            .prop_filter("reserved", |s| s != MusicLine::STOP)
            .prop_map(|cue| Step::Music(MusicLine::Cue(cue))),
        Just(Step::Music(MusicLine::Stop)),
    ]
}

/// Any step, choices (of simple steps) included.
fn arb_step() -> impl Strategy<Value = Step> {
    let option = (
        arb_id(),
        arb_text(),
        proptest::collection::vec(arb_simple_step(), 0..4),
    )
        .prop_map(|(tone, text, steps)| ChoiceOption {
            tone,
            text,
            line: LineId::default(),
            steps,
        });
    prop_oneof![
        4 => arb_simple_step(),
        1 => proptest::collection::vec(option, 0..4).prop_map(|options| Step::Choice { options }),
    ]
}

fn arb_scene() -> impl Strategy<Value = Scene> {
    (arb_id(), proptest::collection::vec(arb_step(), 0..12)).prop_map(|(id, steps)| {
        let mut scene = Scene { id, steps };
        scene.assign_line_ids();
        scene
    })
}

proptest! {
    #[test]
    fn print_parse_round_trip(scenes in proptest::collection::vec(arb_scene(), 1..4)) {
        let printed: Vec<String> = scenes.iter().map(print_scene).collect();
        let (parsed, errors) = parse_dlg("t.dlg", &printed.join("\n"));
        prop_assert_eq!(errors, []);
        let parsed: Vec<Scene> = parsed.into_iter().map(|p| p.scene).collect();
        prop_assert_eq!(parsed, scenes);
    }
}
