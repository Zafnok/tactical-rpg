//! Tests of line ids (ADR-0045 §3, ticket 0717).

use super::super::line_id::{fnv1a64, line_hash};
use super::*;

/// `(id, speaker, text)` of every line of the one scene in `src`.
fn lines(src: &str) -> Vec<(String, String, String)> {
    let (parsed, errors) = parse_dlg("t.dlg", src);
    assert_eq!(errors, []);
    let lines = parsed[0].scene.lines();
    lines
        .iter()
        .map(|l| (l.id.to_string(), l.speaker.to_owned(), l.text.to_owned()))
        .collect()
}

/// The line ids of the one scene in `src`.
fn ids(src: &str) -> Vec<String> {
    lines(src).into_iter().map(|(id, ..)| id).collect()
}

/// The published FNV-1a 64 test vectors.
#[test]
fn fnv1a64_matches_its_test_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
}

/// Ids are stored in translation and voice files, so these strings must
/// never change. (Computed by hand: the low 32 bits of FNV-1a 64 over
/// `speaker`, a newline, `text`.)
#[test]
fn the_hash_of_known_lines_is_fixed() {
    assert_eq!(line_hash("a", "b"), "0415e670");
    assert_eq!(line_hash("test_lord", "Hello."), "9e73f1df");
    assert_eq!(line_hash(NARRATION_SPEAKER, "Hello."), "10c29f83");
    assert_eq!(line_hash(REPLY_SPEAKER, "Hello."), "bc2c818f");
}

#[test]
fn speech_narration_replies_and_reactions_get_ids() {
    let src = scene(
        "test_lord: Hello.\n> Hello.\n@choice\n* wry: Hello.\n  test_knight: Charming.\n\
         * blunt: Move.\n  > He moves.\n@endchoice\ntest_lord: Good.",
    );
    let line = |id: &str, speaker: &str, text: &str| (id.into(), speaker.into(), text.into());
    assert_eq!(
        lines(&src),
        [
            line("s_9e73f1df", "test_lord", "Hello."),
            line("s_10c29f83", ">", "Hello."),
            line("s_bc2c818f", "*", "Hello."),
            line("s_3bb6f82c", "test_knight", "Charming."),
            line("s_a971fd9e", "*", "Move."),
            line("s_62a513e2", ">", "He moves."),
            line("s_ce2602c2", "test_lord", "Good."),
        ]
    );
    assert_eq!(errors(&src), [] as [&str; 0]);
}

#[test]
fn an_id_shows_as_its_text() {
    let (parsed, _) = parse_dlg("t.dlg", "@scene ab\n> Hello.\n@end\n");
    let scene = &parsed[0].scene;
    let Some(Step::Narrate { line, .. }) = scene.steps.first() else {
        panic!("no narration: {scene:?}");
    };
    assert_eq!(line.as_str(), "ab_10c29f83");
    assert_eq!(line.to_string(), "ab_10c29f83");
}

/// The id is of the scene, the speaker and the text as written: tokens
/// unexpanded, continuation lines joined with one space; the expression
/// and a reply's tone are not part of it.
#[test]
fn the_id_comes_from_the_scene_the_speaker_and_the_text_as_written() {
    let id = |src: &str| ids(src).remove(0);
    let base = id("@scene s\ntest_lord: One {lead} two.\n@end\n");
    assert_eq!(
        base,
        format!("s_{}", line_hash("test_lord", "One {lead} two."))
    );
    assert_eq!(
        id("@scene s\ntest_lord[happy]: One {lead}\n    two.\n@end\n"),
        base
    );
    assert_ne!(id("@scene t\ntest_lord: One {lead} two.\n@end\n"), base);
    assert_ne!(id("@scene s\ntest_knight: One {lead} two.\n@end\n"), base);
    assert_ne!(id("@scene s\n> One {lead} two.\n@end\n"), base);
    assert_eq!(
        ids("@scene s\n@choice\n* wry: Hm.\n* blunt: No.\n@endchoice\n@end\n"),
        ids("@scene s\n@choice\n* earnest: Hm.\n* wry: No.\n@endchoice\n@end\n")
    );
}

/// A scene with no `@end` (an error) still gets its ids, continuation
/// lines included.
#[test]
fn scenes_without_an_end_get_ids_too() {
    let (parsed, _) = parse_dlg("t.dlg", "@scene a\n> One\n  two\n@scene b\n> Three\n  four");
    let ids: Vec<Vec<String>> = parsed
        .iter()
        .map(|p| p.scene.lines().iter().map(|l| l.id.to_string()).collect())
        .collect();
    assert_eq!(
        ids,
        [
            [format!("a_{}", line_hash(">", "One two"))],
            [format!("b_{}", line_hash(">", "Three four"))]
        ]
    );
}

#[test]
fn repeats_of_a_line_in_a_scene_are_numbered() {
    let src = scene(
        "test_lord: Go.\ntest_knight: Go.\ntest_lord: Go.\n@choice\n* wry: Go.\n  test_lord: Go.\n\
         * blunt: No.\n@endchoice",
    );
    let lord = format!("s_{}", line_hash("test_lord", "Go."));
    assert_eq!(
        ids(&src),
        [
            lord.clone(),
            format!("s_{}", line_hash("test_knight", "Go.")),
            format!("{lord}_2"),
            format!("s_{}", line_hash("*", "Go.")),
            format!("{lord}_3"),
            format!("s_{}", line_hash("*", "No.")),
        ]
    );
    assert_eq!(errors(&src), [] as [&str; 0]);
}

#[test]
fn inserting_or_moving_a_line_leaves_the_other_ids_unchanged() {
    let before = ids(&scene("test_lord: One.\ntest_knight: Two.\n> Three."));
    let inserted = ids(&scene(
        "> New.\ntest_lord: One.\ntest_knight: Two.\n> Three.",
    ));
    assert_eq!(inserted[1..], before[..]);
    let moved = ids(&scene("> Three.\ntest_lord: One.\ntest_knight: Two."));
    assert_eq!(
        moved.iter().collect::<Vec<_>>(),
        [&before[2], &before[0], &before[1]]
    );
}

#[test]
fn changing_one_word_changes_that_lines_id_only() {
    let before = ids(&scene("test_lord: One.\ntest_knight: Two words.\n> Three."));
    let after = ids(&scene("test_lord: One.\ntest_knight: Two birds.\n> Three."));
    assert_eq!(after[0], before[0]);
    assert_ne!(after[1], before[1]);
    assert_eq!(after[2], before[2]);
}

/// Two different lines whose hashes collide (found by search) would swap
/// ids when reordered, so they are an error naming both.
#[test]
fn different_lines_with_the_same_hash_are_an_error() {
    assert_eq!(line_hash("test_lord", "Count 52392."), "88faeb87");
    assert_eq!(line_hash("test_lord", "Count 148280."), "88faeb87");
    assert_eq!(
        errors(&scene("test_lord: Count 52392.\ntest_lord: Count 148280.")),
        [
            "t.dlg:5: this line and line 4 (\"Count 52392.\") get the same line id hash \
             88faeb87; reword one of them"
        ]
    );
    // In a reply's reaction too.
    let src = scene(
        "@choice\n* wry: Hm.\n  test_lord: Count 52392.\n* blunt: No.\n  test_lord: Count 148280.\n\
         @endchoice",
    );
    assert_eq!(
        errors(&src),
        [
            "t.dlg:8: this line and line 6 (\"Count 52392.\") get the same line id hash \
             88faeb87; reword one of them"
        ]
    );
}

#[test]
fn a_scene_built_by_hand_gets_ids_when_asked() {
    let mut scene = Scene {
        id: "s".into(),
        steps: vec![narrate("Hello."), say("test_lord", None, "Hello.")],
    };
    assert!(scene.lines().iter().all(|l| l.id.as_str().is_empty()));
    scene.assign_line_ids();
    let ids: Vec<&str> = scene.lines().iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["s_10c29f83", "s_9e73f1df"]);
}

proptest! {
    /// Inserting a new line anywhere at the top of a scene never changes
    /// another line's id.
    #[test]
    fn inserting_a_line_never_changes_another_id(
        scene in arb_scene(),
        text in arb_text(),
        at in any::<proptest::sample::Index>(),
    ) {
        let before: Vec<String> = scene.lines().iter().map(|l| l.id.to_string()).collect();
        let repeat = |l: &Line| l.speaker == NARRATION_SPEAKER && l.text == text;
        prop_assume!(!scene.lines().iter().any(repeat));
        let mut after = scene.clone();
        after.steps.insert(at.index(scene.steps.len() + 1), narrate(&text));
        after.assign_line_ids();
        let new = format!("{}_{}", scene.id, line_hash(NARRATION_SPEAKER, &text));
        let rest: Vec<String> = after
            .lines()
            .iter()
            .map(|l| l.id.to_string())
            .filter(|id| *id != new)
            .collect();
        prop_assert_eq!(rest, before);
    }
}
