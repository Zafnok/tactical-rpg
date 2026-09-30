//! Tests of reply choices (`@choice`), lead tokens and lead lines (0708).

use super::*;

fn option(tone: &str, text: &str, steps: Vec<Step>) -> ChoiceOption {
    ChoiceOption {
        tone: tone.into(),
        text: text.into(),
        steps,
    }
}

/// A valid two-option choice (reactions by the knight), for `scene`.
const CHOICE: &str = "\
@choice
* earnest: We do this properly.
  test_knight[surprised]: ...Huh. Fine.
* blunt: Move.
  test_knight[angry]: Charming.
  > He moves.
@endchoice";

#[test]
fn parses_choice_blocks() {
    let src = scene(&format!("{CHOICE}\ntest_lord: Good."));
    let (parsed, errs) = parse_dlg("t.dlg", &src);
    assert_eq!(errs, []);
    assert_eq!(
        parsed[0].scene.steps[2..],
        [
            Step::Choice {
                options: vec![
                    option(
                        "earnest",
                        "We do this properly.",
                        vec![say("test_knight", Some("surprised"), "...Huh. Fine.")]
                    ),
                    option(
                        "blunt",
                        "Move.",
                        vec![
                            say("test_knight", Some("angry"), "Charming."),
                            narrate("He moves."),
                        ]
                    ),
                ]
            },
            say("test_lord", None, "Good."),
        ]
    );
    assert_eq!(parsed[0].step_lines, [2, 3, 4, 11]);
    assert_eq!(
        parsed[0].choice_lines,
        [ChoiceLines {
            options: vec![
                OptionLines {
                    line: 5,
                    step_lines: vec![6]
                },
                OptionLines {
                    line: 7,
                    step_lines: vec![8, 9]
                },
            ]
        }]
    );
    assert_eq!(errors(&src), Vec::<String>::new());
}

#[test]
fn reactions_take_directives_and_continuations() {
    let src = scene(
        "@choice
* a: One.
  @right clear
  @right test_archer happy
  test_archer: Long
    and longer.
* b: Two.
  @right test_archer sad
  > Quiet.
@endchoice",
    );
    assert_eq!(errors(&src), Vec::<String>::new());
    let steps = &scenes(&src)[0].steps;
    let Some(Step::Choice { options }) = steps.get(2) else {
        panic!("no choice: {steps:?}");
    };
    assert_eq!(
        options[0].steps,
        [
            Step::Clear { side: Side::Right },
            place(Side::Right, "test_archer", "happy"),
            say("test_archer", None, "Long and longer."),
        ]
    );
}

#[test]
fn choice_syntax_errors() {
    assert_eq!(
        errors(&scene("> x\n@choice\n* a: One.\n* b: Two.")),
        ["t.dlg:5: @choice has no @endchoice"]
    );
    // At the end of the file, with no @end either.
    assert_eq!(
        errors("@scene s\n> x\n@choice\n* a: One.\n* b: Two.\n"),
        [
            "t.dlg:1: scene \"s\" has no @end",
            "t.dlg:3: @choice has no @endchoice",
        ]
    );
    // A new scene ends it too.
    assert_eq!(
        errors("@scene s\n> x\n@choice\n* a: One.\n* b: Two.\n@scene t\n> y\n@end\n"),
        [
            "t.dlg:1: scene \"s\" has no @end",
            "t.dlg:3: @choice has no @endchoice",
        ]
    );
    assert_eq!(
        errors(&scene(
            "> x\n@choice\n* a: One.\n  @choice\n* b: Two.\n@choice x\n@endchoice"
        )),
        [
            "t.dlg:7: @choice can't be nested inside another @choice",
            "t.dlg:9: @choice takes nothing after it",
            "t.dlg:9: @choice can't be nested inside another @choice",
        ]
    );
    assert_eq!(
        errors(&scene(
            "> x\n@endchoice\n* a: One.\n@choice\n* a: A.\n* b: B.\n@endchoice y"
        )),
        [
            "t.dlg:5: @endchoice without an open @choice",
            "t.dlg:6: an option (\"* tone: text\") must be inside a @choice block",
            "t.dlg:10: @endchoice takes nothing after it",
        ]
    );
    let bad_option = "an option needs a tone and text: \"* tone: text\"";
    assert_eq!(
        errors(&scene(
            "> x\n@choice\n  > early\n*a: x\n* : x\n* a:\n* a x\n* A: x\n * b: x\n> y\n@endchoice\n> z"
        )),
        [
            "t.dlg:5: @choice has 1 options; it needs 2 or 3".to_owned(),
            "t.dlg:6: a reaction line needs an option above it: \"* tone: text\"".to_owned(),
            format!("t.dlg:7: {bad_option}"),
            format!("t.dlg:8: {bad_option}"),
            format!("t.dlg:9: {bad_option}"),
            format!("t.dlg:10: {bad_option}"),
            "t.dlg:11: \"A\" is not a valid id; use lowercase letters, digits and _".to_owned(),
            "t.dlg:12: reaction lines are indented by two spaces".to_owned(),
            "t.dlg:13: inside @choice, a line is an option (\"* tone: text\"), an indented reaction line or @endchoice".to_owned(),
        ]
    );
    assert_eq!(
        errors("@choice\n@endchoice\n"),
        [
            "t.dlg:1: line is outside a scene; start one with @scene <id>",
            "t.dlg:2: @endchoice without an open @choice",
        ]
    );
    assert_eq!(
        errors(&scene("> x\n@choice\n* a: Café.\n* b: B.\n@endchoice")),
        ["t.dlg:6:9: unsupported character 'é'; dialogue text is plain ASCII"]
    );
}

#[test]
fn option_count() {
    let one = "@choice\n* a: A.\n  > r\n@endchoice\n> x";
    assert_eq!(
        errors(&scene(one)),
        ["t.dlg:4: @choice has 1 options; it needs 2 or 3"]
    );
    let four = "@choice\n* a: A.\n* b: B.\n* c: C.\n* d: D.\n@endchoice\n> x";
    assert_eq!(
        errors(&scene(four)),
        ["t.dlg:4: @choice has 4 options; it needs 2 or 3"]
    );
    let three = "@choice\n* a: A.\n* b: B.\n* c: C.\n@endchoice\n> x";
    assert_eq!(errors(&scene(three)), Vec::<String>::new());
}

#[test]
fn option_text_limit() {
    let at_limit = "a".repeat(MAX_OPTION_LEN);
    let ok = format!("@choice\n* a: {at_limit}\n* b: B.\n@endchoice\n> x");
    assert_eq!(errors(&scene(&ok)), Vec::<String>::new());
    // {lead} counts as the longest name (12).
    let long = format!(
        "@choice\n* a: {}{{lead}}\n* b: B.\n@endchoice\n> x",
        "a".repeat(49)
    );
    assert_eq!(
        errors(&scene(&long)),
        ["t.dlg:5: option text is 61 characters; the limit is 60, so it fits the menu"]
    );
}

#[test]
fn reaction_limit() {
    let four = "  > 1\n  > 2\n  > 3\n  > 4";
    let ok = format!("@choice\n* a: A.\n{four}\n* b: B.\n@endchoice");
    assert_eq!(errors(&scene(&ok)), Vec::<String>::new());
    let five = format!("@choice\n* a: A.\n* b: B.\n{four}\n  > 5\n@endchoice");
    assert_eq!(
        errors(&scene(&five)),
        [
            "t.dlg:6: reaction has 5 speech or narration lines; the limit is 4, so the scene rejoins quickly"
        ]
    );
}

#[test]
fn every_option_leaves_the_same_screen() {
    let src = scene(
        "@choice
* a: A.
  @right clear
  @right test_archer happy
  > r
* b: B.
  > r
* c: C.
  @left clear
  @right test_archer sad
@endchoice
test_archer: Who's there?",
    );
    assert_eq!(
        errors(&src),
        [
            "t.dlg:9: after this reaction test_lord is on the left and test_knight on the right; after the first option's test_lord is on the left and test_archer on the right; every option must leave the same characters on screen",
            "t.dlg:11: after this reaction nobody is on the left and test_archer on the right; after the first option's test_lord is on the left and test_archer on the right; every option must leave the same characters on screen",
        ]
    );
    // Expressions may differ: the scene takes back the old ones.
    let src = scene(
        "@choice\n* a: A.\n  test_knight[happy]: Ha.\n* b: B.\n  test_knight[sad]: Oh.\n@endchoice",
    );
    assert_eq!(errors(&src), Vec::<String>::new());
    let caption = scene("@choice\n* a: A.\n  @caption Later\n* b: B.\n@endchoice\n> x");
    assert_eq!(
        errors(&caption),
        [
            "t.dlg:7: this reaction leaves a different caption from the first option's; every option must leave the same caption"
        ]
    );
}

#[test]
fn unknown_tokens() {
    let src = scene(
        "test_lord: {lead}, {they} {Them} {themself}.
> {nope} and {he} and {Lead}.
@caption {x} Heth
@choice
* a: {bogus}
* b: {
@endchoice",
    );
    let hint = "use {lead}, {they}, {them}, {their}, {theirs}, {themself} (capitalised: {They}...) or a name, {n:<id>}";
    assert_eq!(
        errors(&src),
        [
            format!("t.dlg:5: unknown token \"{{nope}}\"; {hint}"),
            format!("t.dlg:5: unknown token \"{{he}}\"; {hint}"),
            format!("t.dlg:5: unknown token \"{{Lead}}\"; {hint}"),
            format!("t.dlg:6: unknown token \"{{x}}\"; {hint}"),
            format!("t.dlg:8: unknown token \"{{bogus}}\"; {hint}"),
            "t.dlg:9: \"{\" has no closing \"}\"".to_owned(),
        ]
    );
}

#[test]
fn text_length_counts_the_longest_substitution() {
    // 188 + {lead} (12) = 200: at the limit; {they} (3) more is 203.
    let base = "a".repeat(188);
    assert_eq!(
        errors(&scene(&format!(
            "> {base}{{lead}}\n> {base}{{lead}}{{they}}"
        ))),
        ["t.dlg:5: text is 203 characters; the limit is 200"]
    );
}

#[test]
fn the_lead_speaks_in_short_lines() {
    let body = |text: &str| format!("@right clear\n@right lead neutral\nlead: {text}");
    let at_limit = "a".repeat(MAX_LEAD_LINE_LEN);
    assert_eq!(errors(&scene(&body(&at_limit))), Vec::<String>::new());
    assert_eq!(errors(&scene(&body("Let's move."))), Vec::<String>::new());
    let message = "the lead's line is 41 characters; the limit is 40: the lead speaks in short, \
                   neutral lines and otherwise through reply choices \
                   (docs/design/setting-and-tone.md, \"Rules for writing the lead\")";
    assert_eq!(
        errors(&scene(&body(&format!("{at_limit}a")))),
        [format!("t.dlg:6: {message}")]
    );
    // {lead} counts as 12: 29 + 12 = 41.
    let with_name = format!("{}{{lead}}", "a".repeat(29));
    assert_eq!(
        errors(&scene(&body(&with_name))),
        [format!("t.dlg:6: {message}")]
    );
    // Other speakers' lines aren't limited.
    assert_eq!(
        errors(&scene(&format!("test_lord: {at_limit}a"))),
        Vec::<String>::new()
    );
}

#[test]
fn the_leads_expressions_come_from_both_portraits() {
    let palette = crate::palette::PaletteDef::load().unwrap_or_default();
    let mut portraits = crate::portrait::load_all(&palette).unwrap_or_default();
    assert!(portraits.contains_key("lead_m") && portraits.contains_key("lead_f"));
    let src = scene("@right clear\n@right lead neutral\nlead[sad]: Hm.");
    assert_eq!(errors_with(&src, &portraits), Vec::<String>::new());
    // One portrait lacks "sad": the lead can't use it.
    if let Some(p) = portraits.get_mut("lead_f") {
        p.expressions.retain(|e| e.name != "sad");
    }
    assert_eq!(
        errors_with(&src, &portraits),
        [
            "t.dlg:6: \"lead_f\"'s portrait has no expression \"sad\"; use one of neutral, happy, angry, surprised"
        ]
    );
    // With neither portrait, the standard five.
    portraits.retain(|id, _| !id.starts_with("lead"));
    assert_eq!(errors_with(&src, &portraits), Vec::<String>::new());
}

#[test]
fn a_scene_whose_only_text_is_in_a_reaction_has_text() {
    let src = "@scene s\n@choice\n* a: A.\n  > r\n* b: B.\n@endchoice\n@end\n";
    assert_eq!(errors(src), Vec::<String>::new());
    let src = "@scene s\n@choice\n* a: A.\n* b: B.\n@endchoice\n@end\n";
    assert_eq!(
        errors(src),
        ["t.dlg:1: scene \"s\" has no speech or narration"]
    );
}

#[test]
fn choice_step_has_no_text() {
    let choice = Step::Choice {
        options: vec![option("a", "A.", vec![narrate("x")])],
    };
    assert_eq!(choice.text(), None);
}

#[test]
fn prints_choices() {
    let s = Scene {
        id: "s".into(),
        steps: vec![
            Step::Choice {
                options: vec![
                    option(
                        "a",
                        "A.",
                        vec![narrate("x"), Step::Clear { side: Side::Left }],
                    ),
                    option("b", "B.", vec![]),
                ],
            },
            narrate("y"),
        ],
    };
    assert_eq!(
        print_scene(&s),
        "@scene s\n@choice\n* a: A.\n  > x\n  @left clear\n* b: B.\n@endchoice\n> y\n@end\n"
    );
}

/// The choice example in `assets/dialogue/README.md` parses and passes
/// every check except the character ids (its cast isn't in
/// `characters.ron`).
#[test]
fn readme_choice_example_is_valid() {
    let readme = bundle::file("dialogue/README.md").unwrap_or_default();
    let example = readme.split("```").nth(3).unwrap_or_default();
    let table = from_sources(&[("README.md", example)], None, None, None);
    assert!(table.is_ok(), "{table:?}");
    let scene = table.unwrap_or_default().scenes.remove("ch01_gate");
    let choice = scene.and_then(|s| {
        s.steps
            .into_iter()
            .find(|s| matches!(s, Step::Choice { .. }))
    });
    assert!(matches!(choice, Some(Step::Choice { options }) if options.len() == 3));
}
