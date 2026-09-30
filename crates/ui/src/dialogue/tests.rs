//! Tests of the dialogue player: the embedded test scene box by box, and
//! every text step shown once, in order, for random scenes.

use proptest::prelude::*;

use std::borrow::Cow;

use trpg_content::ChoiceOption;

use super::*;

fn id(s: &str) -> CharacterId {
    CharacterId(s.into())
}

/// A lead named Ellery, male.
fn lead() -> LeadProfile {
    LeadProfile::new("Ellery", trpg_core::LeadGender::Male)
}

/// Plays `scene` with [`lead`].
fn play(scene: Scene) -> DialoguePlayer {
    DialoguePlayer::new(scene, lead())
}

/// More text boxes than any scene in these tests has.
const MAX_BOXES: usize = 100;

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
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    left: Option<(String, String)>,
    right: Option<(String, String)>,
    speaker: Option<Side>,
    text: Option<String>,
    caption: Option<String>,
    narration: bool,
    choices: Option<Vec<String>>,
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
        choices: v.choices.map(|c| c.into_iter().map(Into::into).collect()),
    }
}

fn portrait(character: &str, expression: &str) -> (String, String) {
    (character.into(), expression.into())
}

/// Views from now until the player finishes, picking reply `pick` at each
/// choice (the choice itself is one view). Bounded, so a player that never
/// finishes fails instead of hanging.
fn walk(player: &mut DialoguePlayer, pick: usize) -> Vec<Shown> {
    let mut views = Vec::new();
    for _ in 0..MAX_BOXES {
        if player.is_finished() {
            break;
        }
        views.push(shown(player.current()));
        if player.is_choosing() {
            player.choose(pick);
        } else {
            player.advance();
        }
    }
    assert!(player.is_finished());
    views.push(shown(player.current()));
    views
}

const CAPTION: &str = "Village of Heth, dusk";

fn says(
    left: Option<(String, String)>,
    right: Option<(String, String)>,
    speaker: Side,
    text: &str,
) -> Shown {
    Shown {
        left,
        right,
        speaker: Some(speaker),
        text: Some(text.into()),
        caption: Some(CAPTION.into()),
        narration: false,
        choices: None,
    }
}

fn narrates(left: Option<(String, String)>, right: Option<(String, String)>, text: &str) -> Shown {
    Shown {
        left,
        right,
        speaker: None,
        text: Some(text.into()),
        caption: Some(CAPTION.into()),
        narration: true,
        choices: None,
    }
}

#[test]
fn walks_the_test_scene() {
    let mut player = play(test_scene());
    assert_eq!(player.scene_id(), "test");
    let views = walk(&mut player, 0);
    let lord = |e| Some(portrait("test_lord", e));
    let knight = Some(portrait("test_knight", "angry"));
    let archer = |e| Some(portrait("test_archer", e));
    let ellery = Some(portrait("lead", "neutral"));
    let came_back = says(
        ellery.clone(),
        archer("surprised"),
        Side::Right,
        "Ellery! You came back for us.",
    );
    assert_eq!(
        views,
        [
            narrates(None, None, "The rain had not stopped for three days."),
            says(lord("neutral"), knight.clone(), Side::Right, "You're late."),
            says(
                lord("happy"),
                knight.clone(),
                Side::Left,
                "Better late than... well."
            ),
            says(
                lord("happy"),
                knight,
                Side::Right,
                "Than never. Say it. I've heard it from you often enough."
            ),
            narrates(lord("happy"), None, "The knight stamps off into the rain."),
            says(
                lord("happy"),
                archer("surprised"),
                Side::Right,
                "Was that about me?"
            ),
            says(
                lord("sad"),
                archer("surprised"),
                Side::Left,
                "It's always about you."
            ),
            came_back.clone(),
            // The choice: the archer's line stays up under the replies.
            Shown {
                choices: Some(vec![
                    "I said I would. I keep my word.".into(),
                    "Someone has to carry your arrows.".into(),
                    "Get moving. We're not safe here.".into(),
                ]),
                ..came_back
            },
            says(
                ellery.clone(),
                archer("happy"),
                Side::Right,
                "You do. It's why we follow you."
            ),
            // Rejoined: the script sets the archer's expression.
            says(ellery.clone(), archer("neutral"), Side::Left, "Let's move."),
            narrates(
                ellery.clone(),
                archer("neutral"),
                "Ellery tightens his grip on the sword. He won't lose anyone today."
            ),
            // Finished: the last portraits stay, no text.
            Shown {
                left: ellery,
                right: archer("neutral"),
                speaker: None,
                text: None,
                caption: Some(CAPTION.into()),
                narration: false,
                choices: None,
            },
        ]
    );
}

/// Each reply plays its own reaction, then every one shows the same view
/// from the rejoin on.
#[test]
fn every_reply_rejoins_the_same_way() {
    let mut player = play(test_scene());
    player.skip_to_choice();
    assert!(player.is_choosing());
    let reactions = [
        vec!["You do. It's why we follow you."],
        vec![
            "I carry my own arrows, thank you.",
            "The archer's ears go red anyway.",
        ],
        vec!["Right. Moving."],
    ];
    let mut rejoined = Vec::new();
    for (pick, reaction) in reactions.iter().enumerate() {
        let mut p = player.clone();
        let views = walk(&mut p, pick);
        let texts: Vec<Option<&str>> = views.iter().map(|v| v.text.as_deref()).collect();
        let n = reaction.len();
        // The choice view, the reaction, then the rest of the scene.
        assert_eq!(
            texts[1..=n],
            reaction.iter().map(|&t| Some(t)).collect::<Vec<_>>()[..]
        );
        assert_eq!(texts[n + 1], Some("Let's move."));
        rejoined.push(views[n + 1..].to_vec());
    }
    assert_eq!(rejoined[0], rejoined[1]);
    assert_eq!(rejoined[0], rejoined[2]);
}

#[test]
fn advancing_a_finished_scene_does_nothing() {
    let mut player = play(Scene {
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
    let player = play(Scene {
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
            choices: None,
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
    let mut player = play(Scene {
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

fn option(text: &str, steps: Vec<Step>) -> ChoiceOption {
    ChoiceOption {
        tone: "t".into(),
        text: text.into(),
        steps,
    }
}

fn narration(text: &str) -> Step {
    Step::Narrate { text: text.into() }
}

#[test]
fn a_choice_waits_for_a_reply() {
    let mut player = play(Scene {
        id: "s".into(),
        steps: vec![
            Step::Choice {
                options: vec![
                    option("{They} goes.", vec![narration("a")]),
                    option("{lead} stays.", vec![]),
                ],
            },
            narration("after"),
        ],
    });
    // A choice with no text box before it shows no text.
    let v = player.current();
    assert_eq!(v.text, None);
    assert_eq!(
        v.choices,
        Some(vec![Cow::from("He goes."), Cow::from("Ellery stays.")])
    );
    assert!(player.is_choosing() && !player.is_finished());
    let before = player.clone();
    player.advance();
    player.skip_to_choice();
    player.choose(2);
    assert_eq!(player, before);
    // An empty reaction goes straight on.
    player.choose(1);
    assert!(!player.is_choosing());
    assert_eq!(player.current().text.as_deref(), Some("after"));
    // Choosing with no choice open does nothing.
    let before = player.clone();
    player.choose(0);
    assert_eq!(player, before);
    player.skip_to_choice();
    assert!(player.is_finished());
}

#[test]
fn a_choice_without_replies_is_passed_over() {
    let player = play(Scene {
        id: "s".into(),
        steps: vec![Step::Choice { options: vec![] }, narration("after")],
    });
    assert_eq!(player.current().text.as_deref(), Some("after"));
}

/// At the rejoin, portraits stay as the reaction left them: the script,
/// not the player, decides any change.
#[test]
fn the_reaction_leaves_the_portraits_as_they_are() {
    let place = |side, who: &str, expression: &str| Step::Place {
        side,
        character: id(who),
        expression: expression.into(),
    };
    let mut player = play(Scene {
        id: "s".into(),
        steps: vec![
            place(Side::Left, "a", "sad"),
            place(Side::Right, "b", "angry"),
            narration("before"),
            Step::Choice {
                options: vec![option(
                    "x",
                    vec![
                        place(Side::Left, "b", "happy"),
                        place(Side::Right, "c", "surprised"),
                        narration("reaction"),
                    ],
                )],
            },
            narration("after"),
        ],
    });
    player.advance();
    player.choose(0);
    let during = shown(player.current());
    player.advance();
    let v = shown(player.current());
    assert_eq!(v.text.as_deref(), Some("after"));
    assert_eq!(
        (v.left.clone(), v.right.clone()),
        (
            Some(portrait("b", "happy")),
            Some(portrait("c", "surprised"))
        )
    );
    assert_eq!((during.left, during.right), (v.left, v.right));
}

/// Skipping stops at each choice, and at the end.
#[test]
fn skipping_stops_at_choices() {
    let mut player = play(test_scene());
    player.skip_to_choice();
    assert!(player.is_choosing());
    assert_eq!(
        player.current().text.as_deref(),
        Some("Ellery! You came back for us.")
    );
    player.skip_to_choice();
    assert!(player.is_choosing());
    player.choose(2);
    player.skip_to_choice();
    assert!(player.is_finished());
}

#[test]
fn tokens_follow_the_lead() {
    let scene = Scene {
        id: "s".into(),
        steps: vec![
            Step::Caption {
                text: "{lead}'s camp".into(),
            },
            narration("{They} fed {themself}."),
        ],
    };
    let player = DialoguePlayer::new(
        scene.clone(),
        LeadProfile::new("Isolde", trpg_core::LeadGender::Female),
    );
    let v = player.current();
    assert_eq!(v.caption.as_deref(), Some("Isolde's camp"));
    assert_eq!(v.text.as_deref(), Some("She fed herself."));
    // The scene itself is unchanged.
    assert_eq!(player.scene, scene);
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

/// A scene step, or a choice of up to 3 replies whose reactions are
/// simple steps.
fn arb_step_or_choice() -> impl Strategy<Value = Step> {
    let reply = ("[a-z]{1,6}", proptest::collection::vec(arb_step(), 0..4))
        .prop_map(|(text, steps)| option(&text, steps));
    prop_oneof![
        3 => arb_step(),
        1 => proptest::collection::vec(reply, 1..4).prop_map(|options| Step::Choice { options }),
    ]
}

/// The text boxes of `steps` in order, playing reply `picks[k] % n` at
/// the k-th choice.
fn expected_texts(steps: &[Step], picks: &[usize]) -> Vec<String> {
    let mut picks = picks.iter().cycle();
    let mut out = Vec::new();
    for step in steps {
        match step {
            Step::Choice { options } => {
                let pick = picks.next().copied().unwrap_or(0) % options.len();
                out.extend(
                    options[pick]
                        .steps
                        .iter()
                        .filter_map(Step::text)
                        .map(str::to_owned),
                );
            }
            _ => out.extend(step.text().map(str::to_owned)),
        }
    }
    out
}

proptest! {
    /// Whatever replies are picked, every text box outside the choices is
    /// shown exactly once, in order, with the picked reactions between.
    #[test]
    fn every_reply_path_visits_the_rest_once_in_order(
        steps in proptest::collection::vec(arb_step_or_choice(), 0..16),
        picks in proptest::collection::vec(0..3usize, 1..6),
    ) {
        let expected = expected_texts(&steps, &picks);
        let choices = steps.iter().filter(|s| matches!(s, Step::Choice { .. })).count();
        let mut player = play(Scene { id: "s".into(), steps });
        let mut picked = picks.iter().cycle();
        let mut seen = Vec::new();
        let mut asked = 0;
        for _ in 0..MAX_BOXES {
            if player.is_finished() {
                break;
            }
            let v = player.current();
            if let Some(options) = v.choices {
                let n = options.len();
                asked += 1;
                let pick = picked.next().copied().unwrap_or(0) % n;
                player.choose(pick);
            } else {
                seen.push(v.text.unwrap_or_default().into_owned());
                player.advance();
            }
        }
        prop_assert!(player.is_finished());
        prop_assert_eq!(asked, choices);
        prop_assert_eq!(seen, expected);
    }

    /// Advancing until finished shows every speech and narration exactly
    /// once, in script order.
    #[test]
    fn visits_every_text_step_once_in_order(steps in proptest::collection::vec(arb_step(), 0..20)) {
        let expected: Vec<(String, bool)> = steps
            .iter()
            .filter_map(|s| s.text().map(|t| (t.to_owned(), matches!(s, Step::Narrate { .. }))))
            .collect();
        let mut player = play(Scene { id: "s".into(), steps });
        let mut seen = Vec::new();
        for _ in 0..MAX_BOXES {
            if player.is_finished() {
                break;
            }
            let v = player.current();
            prop_assert!(v.text.is_some());
            seen.push((v.text.unwrap_or_default().into_owned(), v.narration));
            player.advance();
        }
        prop_assert!(player.is_finished());
        prop_assert_eq!(player.current().text, None);
        prop_assert_eq!(seen, expected);
    }
}
