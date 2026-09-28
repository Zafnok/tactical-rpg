//! Tests of the dialogue player: the embedded test scene box by box, and
//! every text step shown once, in order, for random scenes.

use proptest::prelude::*;

use super::*;

fn id(s: &str) -> CharacterId {
    CharacterId(s.into())
}

/// The embedded `test` scene (`assets/dialogue/test.dlg`).
fn test_scene() -> Scene {
    let content = trpg_content::load_embedded().unwrap_or_else(|e| panic!("{e}"));
    content
        .dialogue
        .get("test")
        .cloned()
        .unwrap_or_else(|| panic!("no test scene"))
}

/// An owned copy of a [`View`], so a whole walk can be compared at once.
#[derive(Debug, PartialEq, Eq)]
struct Shown {
    left: Option<(String, String)>,
    right: Option<(String, String)>,
    speaker: Option<Side>,
    text: Option<String>,
    caption: Option<String>,
    narration: bool,
}

fn shown(v: View<'_>) -> Shown {
    let p = |p: Option<Portrait<'_>>| p.map(|p| (p.character.0.clone(), p.expression.to_owned()));
    Shown {
        left: p(v.left),
        right: p(v.right),
        speaker: v.speaker,
        text: v.text.map(Into::into),
        caption: v.caption.map(Into::into),
        narration: v.narration,
    }
}

fn portrait(character: &str, expression: &str) -> (String, String) {
    (character.into(), expression.into())
}

#[test]
fn walks_the_test_scene() {
    let caption = Some("Village of Heth, dusk".to_owned());
    let mut player = DialoguePlayer::new(test_scene());
    assert_eq!(player.scene_id(), "test");
    let mut views = Vec::new();
    while !player.is_finished() {
        views.push(shown(player.current()));
        player.advance();
    }
    views.push(shown(player.current()));
    let lord = |e| Some(portrait("test_lord", e));
    let say = |left, right, speaker, text: &str| Shown {
        left,
        right,
        speaker: Some(speaker),
        text: Some(text.into()),
        caption: caption.clone(),
        narration: false,
    };
    let narrate = |left, right, text: &str| Shown {
        left,
        right,
        speaker: None,
        text: Some(text.into()),
        caption: caption.clone(),
        narration: true,
    };
    let knight = Some(portrait("test_knight", "angry"));
    let archer = Some(portrait("test_archer", "surprised"));
    assert_eq!(
        views,
        [
            narrate(None, None, "The rain had not stopped for three days."),
            say(lord("neutral"), knight.clone(), Side::Right, "You're late."),
            say(
                lord("happy"),
                knight.clone(),
                Side::Left,
                "Better late than... well."
            ),
            say(
                lord("happy"),
                knight,
                Side::Right,
                "Than never. Say it. I've heard it from you often enough."
            ),
            narrate(lord("happy"), None, "The knight stamps off into the rain."),
            say(
                lord("happy"),
                archer.clone(),
                Side::Right,
                "Was that about me?"
            ),
            say(
                lord("sad"),
                archer.clone(),
                Side::Left,
                "It's always about you."
            ),
            // Finished: the last portraits stay, no text.
            Shown {
                left: lord("sad"),
                right: archer,
                speaker: None,
                text: None,
                caption: caption.clone(),
                narration: false,
            },
        ]
    );
}

#[test]
fn advancing_a_finished_scene_does_nothing() {
    let mut player = DialoguePlayer::new(Scene {
        id: "s".into(),
        steps: vec![Step::Narrate { text: "x".into() }],
    });
    assert!(!player.is_finished());
    player.advance();
    assert!(player.is_finished());
    let before = player.clone();
    player.advance();
    assert_eq!(player, before);
}

#[test]
fn a_scene_without_text_starts_finished() {
    let player = DialoguePlayer::new(Scene {
        id: "s".into(),
        steps: vec![
            Step::Caption { text: "c".into() },
            Step::Place {
                side: Side::Left,
                character: id("a"),
                expression: "sad".into(),
            },
        ],
    });
    assert!(player.is_finished());
    assert_eq!(
        shown(player.current()),
        Shown {
            left: Some(portrait("a", "sad")),
            right: None,
            speaker: None,
            text: None,
            caption: Some("c".into()),
            narration: false,
        }
    );
}

/// A speaker's expression change applies only to the speaker, and a
/// speaker who isn't on screen (a script the validator would reject) has no
/// side.
#[test]
fn expression_changes_and_offscreen_speakers() {
    let say = |who: &str, e: Option<&str>| Step::Say {
        speaker: id(who),
        expression: e.map(Into::into),
        text: "t".into(),
    };
    let place = |side, who: &str| Step::Place {
        side,
        character: id(who),
        expression: "neutral".into(),
    };
    let mut player = DialoguePlayer::new(Scene {
        id: "s".into(),
        steps: vec![
            place(Side::Left, "a"),
            place(Side::Right, "b"),
            say("b", Some("angry")),
            say("c", Some("sad")),
            say("a", None),
        ],
    });
    let v = shown(player.current());
    assert_eq!(
        (v.left, v.right),
        (Some(portrait("a", "neutral")), Some(portrait("b", "angry")))
    );
    assert_eq!(v.speaker, Some(Side::Right));
    player.advance();
    let v = shown(player.current());
    assert_eq!(
        (v.left, v.right),
        (Some(portrait("a", "neutral")), Some(portrait("b", "angry")))
    );
    assert_eq!((v.speaker, v.narration), (None, false));
    player.advance();
    assert_eq!(player.current().speaker, Some(Side::Left));
}

fn arb_step() -> impl Strategy<Value = Step> {
    let side = || prop_oneof![Just(Side::Left), Just(Side::Right)];
    let who = || prop_oneof![Just("a"), Just("b"), Just("c")].prop_map(id);
    let expression = || prop_oneof![Just("sad".to_owned()), Just("happy".to_owned())];
    prop_oneof![
        "[a-z]{1,6}".prop_map(|text| Step::Caption { text }),
        (side(), who(), expression()).prop_map(|(side, character, expression)| Step::Place {
            side,
            character,
            expression,
        }),
        side().prop_map(|side| Step::Clear { side }),
        (who(), proptest::option::of(expression()), "[a-z]{1,6}").prop_map(
            |(speaker, expression, text)| Step::Say {
                speaker,
                expression,
                text,
            }
        ),
        "[a-z]{1,6}".prop_map(|text| Step::Narrate { text }),
    ]
}

proptest! {
    /// Advancing until finished shows every speech and narration exactly
    /// once, in script order.
    #[test]
    fn visits_every_text_step_once_in_order(steps in proptest::collection::vec(arb_step(), 0..20)) {
        let expected: Vec<(String, bool)> = steps
            .iter()
            .filter_map(|s| s.text().map(|t| (t.to_owned(), matches!(s, Step::Narrate { .. }))))
            .collect();
        let mut player = DialoguePlayer::new(Scene { id: "s".into(), steps });
        let mut seen = Vec::new();
        while !player.is_finished() {
            let v = player.current();
            prop_assert!(v.text.is_some());
            seen.push((v.text.unwrap_or_default().to_owned(), v.narration));
            player.advance();
        }
        prop_assert_eq!(player.current().text, None);
        prop_assert_eq!(seen, expected);
    }
}
