//! Tests of the voice manifest (ADR-0046, ticket 0238).

use trpg_core::lead::LeadGender::{Female, Male};

use super::*;
use crate::dialogue::from_sources;

/// A scene `s` with a plain line, a line with a name, a line that changes
/// with the lead's gender, a line with the lead's name, and narration.
const SCRIPT: &str = "\
@scene s
@left test_lord neutral
test_lord: Hello.
test_lord: {N:king} rides for {n:keep} at dawn.
test_lord: {They} left {their} sword with {n:king}.
test_lord: Welcome back, {lead}.
> Hello.
@end
";

fn dialogue() -> DialogueTable {
    from_sources(&[("t.dlg", SCRIPT)], None, None, None, None).unwrap()
}

fn names_with(king: &str) -> Names {
    let names = [("king", king), ("keep", "Harrow Keep")];
    Names {
        names: names.map(|(k, v)| (k.to_owned(), v.to_owned())).into(),
    }
}

fn names() -> Names {
    names_with("the king")
}

/// The id of the line of [`SCRIPT`] written `text`, said by `speaker`.
fn id(speaker: &str, text: &str) -> String {
    let dialogue = dialogue();
    let lines = dialogue.scenes["s"].lines();
    let line = lines
        .iter()
        .find(|l| l.speaker == speaker && l.text == text);
    line.unwrap().id.to_string()
}

fn hello() -> String {
    id("test_lord", "Hello.")
}

fn rides() -> String {
    id("test_lord", "{N:king} rides for {n:keep} at dawn.")
}

fn sword() -> String {
    id("test_lord", "{They} left {their} sword with {n:king}.")
}

fn welcome() -> String {
    id("test_lord", "Welcome back, {lead}.")
}

/// One manifest entry.
fn clip(line: &str, variant: &str, spoken: &str) -> String {
    format!(
        "(line: \"{line}\", {variant} spoken: \"{spoken}\", voice: \"lord_a\", \
         made_by: Generated(tool: \"t\", model: \"m\", date: \"2026-10-03\")),\n"
    )
}

fn manifest_source(clips: &[String]) -> String {
    format!("(clips: [\n{}])", clips.concat())
}

fn manifest(clips: &[String]) -> VoiceManifest {
    from_source("voice.ron", &manifest_source(clips), &dialogue()).unwrap()
}

fn errors(clips: &[String]) -> Vec<String> {
    from_source("voice.ron", &manifest_source(clips), &dialogue())
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// A manifest with a fresh clip for every line of [`SCRIPT`] that can
/// have one.
fn full() -> Vec<String> {
    vec![
        clip(&hello(), "", "Hello."),
        clip(&rides(), "", "The king rides for Harrow Keep at dawn."),
        clip(&sword(), "variant: M,", "He left his sword with the king."),
        clip(&sword(), "variant: F,", "She left her sword with the king."),
        clip(&id(">", "Hello."), "", "Hello."),
    ]
}

#[test]
fn spoken_text_fills_names_and_pronouns() {
    let n = names();
    assert_eq!(spoken_text("Hello.", &n, Male).as_deref(), Some("Hello."));
    assert_eq!(
        spoken_text("{N:king} rides for {n:keep}.", &n, Female).as_deref(),
        Some("The king rides for Harrow Keep.")
    );
    let text = "{They} left {their} sword; it is {theirs}, {they} said to {themself}.";
    assert_eq!(
        spoken_text(text, &n, Male).as_deref(),
        Some("He left his sword; it is his, he said to himself.")
    );
    assert_eq!(
        spoken_text(text, &n, Female).as_deref(),
        Some("She left her sword; it is hers, she said to herself.")
    );
    // Unknown tokens and stray braces stay as written.
    assert_eq!(
        spoken_text("{n:nobody} and {what} and {open", &n, Male).as_deref(),
        Some("{n:nobody} and {what} and {open")
    );
}

#[test]
fn a_line_with_the_leads_name_has_no_spoken_text() {
    for gender in [Male, Female] {
        assert_eq!(spoken_text("Welcome back, {lead}.", &names(), gender), None);
        assert_eq!(spoken_text("{lead}! {They} came.", &names(), gender), None);
    }
}

#[test]
fn only_pronoun_tokens_change_with_gender() {
    assert!(changes_with_gender("{They} left."));
    assert!(changes_with_gender("It is {theirs}."));
    assert!(!changes_with_gender("Hello."));
    assert!(!changes_with_gender("{N:king} rides for {n:keep}."));
    assert!(!changes_with_gender("Welcome back, {lead}."));
    assert!(!changes_with_gender("{what} {open"));
}

#[test]
fn a_clip_file_is_under_its_scene_with_its_variant() {
    let line = LineId::new("ch1_gate_1a2b3c4d");
    assert_eq!(
        clip_file(&line, Variant::None),
        "ch1_gate/ch1_gate_1a2b3c4d.ogg"
    );
    assert_eq!(
        clip_file(&line, Variant::M),
        "ch1_gate/ch1_gate_1a2b3c4d.m.ogg"
    );
    assert_eq!(
        clip_file(&line, Variant::F),
        "ch1_gate/ch1_gate_1a2b3c4d.f.ogg"
    );
    let repeat = LineId::new("ch1_gate_1a2b3c4d_2");
    assert_eq!(
        clip_file(&repeat, Variant::None),
        "ch1_gate/ch1_gate_1a2b3c4d_2.ogg"
    );
}

#[test]
fn variants_and_genders_match() {
    assert_eq!(Variant::of(Male), Variant::M);
    assert_eq!(Variant::of(Female), Variant::F);
    assert_eq!(Variant::M.gender(), Some(Male));
    assert_eq!(Variant::F.gender(), Some(Female));
    assert_eq!(Variant::None.gender(), None);
    assert_eq!(Variant::default(), Variant::None);
}

#[test]
fn a_valid_manifest_parses() {
    let recorded = format!(
        "(line: \"{}\", spoken: \"Hello.\", voice: \"narrator\", \
         made_by: Recorded(actor: \"A. Person\")),\n",
        id(">", "Hello.")
    );
    let m = manifest(&[
        clip(&hello(), "", "Hello."),
        clip(&sword(), "variant: M,", "He left his sword with the king."),
        clip(&sword(), "variant: F,", "She left her sword with the king."),
        recorded,
    ]);
    assert_eq!(m.clips.len(), 4);
    let first = &m.clips[0];
    assert_eq!(first.line.as_str(), hello());
    assert_eq!(first.variant, Variant::None);
    assert_eq!(first.spoken, "Hello.");
    assert_eq!(first.voice, "lord_a");
    let generated = MadeBy::Generated {
        tool: "t".into(),
        model: "m".into(),
        date: "2026-10-03".into(),
    };
    assert_eq!(first.made_by, generated);
    assert_eq!(m.clips[1].variant, Variant::M);
    assert_eq!(m.clips[2].variant, Variant::F);
    let actor = "A. Person".into();
    assert_eq!(m.clips[3].made_by, MadeBy::Recorded { actor });
    assert_eq!(manifest(&[]), VoiceManifest::default());
}

#[test]
fn a_clip_of_an_unknown_line_is_an_error() {
    let clips = [clip(&hello(), "", "Hello."), clip("s_00000000", "", "Hi.")];
    assert_eq!(
        errors(&clips),
        ["voice.ron:3: no dialogue line has the id \"s_00000000\""]
    );
}

#[test]
fn the_same_variant_twice_is_an_error() {
    let hello = hello();
    let clips = [clip(&hello, "", "Hello."), clip(&hello, "", "Hello.")];
    assert_eq!(
        errors(&clips),
        [format!("voice.ron:2: line \"{hello}\" has two None clips")]
    );
    let sword = sword();
    let clips = [
        clip(&sword, "variant: M,", "x"),
        clip(&sword, "variant: F,", "x"),
        clip(&sword, "variant: F,", "x"),
    ];
    assert_eq!(
        errors(&clips),
        [format!("voice.ron:2: line \"{sword}\" has two F clips")]
    );
}

#[test]
fn m_and_f_come_as_a_pair() {
    let sword = sword();
    for variant in ["variant: M,", "variant: F,"] {
        assert_eq!(
            errors(&[clip(&sword, variant, "x")]),
            [format!(
                "voice.ron:2: line \"{sword}\" has an M or an F clip without the other"
            )]
        );
    }
    let all = [
        clip(&sword, "", "x"),
        clip(&sword, "variant: M,", "x"),
        clip(&sword, "variant: F,", "x"),
    ];
    assert_eq!(
        errors(&all),
        [format!(
            "voice.ron:2: line \"{sword}\" has a clip for both genders and M or F clips; \
             keep one kind"
        )]
    );
    // With only one of the pair beside it, both problems are reported.
    assert_eq!(errors(&all[..2]).len(), 2);
    assert!(errors(&all[1..]).is_empty());
}

#[test]
fn empty_words_or_voice_are_errors() {
    let hello = hello();
    let no_voice = format!(
        "(line: \"{hello}\", spoken: \" \", voice: \"\", made_by: Recorded(actor: \"a\")),\n"
    );
    assert_eq!(
        errors(&[no_voice]),
        [
            format!("voice.ron:2: the clip of line \"{hello}\" has no spoken text"),
            format!("voice.ron:2: the clip of line \"{hello}\" has no voice"),
        ]
    );
}

#[test]
fn a_file_that_is_not_a_manifest_is_one_error_with_its_place() {
    let d = dialogue();
    let broken = from_source("voice.ron", "(clips: [\n  nonsense,\n])", &d).unwrap_err();
    assert_eq!(broken.len(), 1);
    assert_eq!(broken[0].line, Some(2));
    let extra = "(clips: [], cast: 1)";
    assert!(from_source("voice.ron", extra, &d).is_err());
    assert!(from_source("voice.ron", "", &d).is_err());
}

/// Acceptance (0238): renaming a name used in a clip's line makes that
/// clip stale, and only that clip.
#[test]
fn renaming_a_name_makes_the_clips_that_say_it_stale() {
    let d = dialogue();
    let m = manifest(&full());
    let stale = |names: &Names| -> Vec<bool> {
        m.clips
            .iter()
            .map(|c| VoiceManifest::is_stale(c, &d, names))
            .collect()
    };
    assert_eq!(stale(&names()), [false; 5]);
    assert_eq!(m.playable(&d, &names()).len(), 5);
    // `king` is in the second line and both clips of the third.
    let renamed = names_with("the queen");
    assert_eq!(stale(&renamed), [false, true, true, true, false]);
    let playable = m.playable(&d, &renamed);
    assert_eq!(playable.len(), 2);
    let line = |id: String| LineId::new(id);
    assert_eq!(
        playable.variant_for(&line(hello()), Male),
        Some(Variant::None)
    );
    assert_eq!(playable.variant_for(&line(rides()), Male), None);
    assert_eq!(playable.variant_for(&line(sword()), Female), None);
}

#[test]
fn the_playable_clip_of_a_line_follows_the_leads_gender() {
    let d = dialogue();
    let playable = manifest(&full()).playable(&d, &names());
    assert!(!playable.is_empty());
    let sword = LineId::new(sword());
    assert_eq!(playable.variant_for(&sword, Male), Some(Variant::M));
    assert_eq!(playable.variant_for(&sword, Female), Some(Variant::F));
    let hello = LineId::new(hello());
    for gender in [Male, Female] {
        assert_eq!(playable.variant_for(&hello, gender), Some(Variant::None));
        let unknown = LineId::new("s_00000000");
        assert_eq!(playable.variant_for(&unknown, gender), None);
        let unvoiced = LineId::new(welcome());
        assert_eq!(playable.variant_for(&unvoiced, gender), None);
    }
    assert!(Playable::default().is_empty());
    assert_eq!(Playable::default().len(), 0);
}

#[test]
fn each_gendered_clip_is_checked_against_its_own_words() {
    let d = dialogue();
    // The F clip says the M words: only it is stale.
    let m = manifest(&[
        clip(&sword(), "variant: M,", "He left his sword with the king."),
        clip(&sword(), "variant: F,", "He left his sword with the king."),
    ]);
    let playable = m.playable(&d, &names());
    let sword = LineId::new(sword());
    assert_eq!(playable.variant_for(&sword, Male), Some(Variant::M));
    assert_eq!(playable.variant_for(&sword, Female), None);
}

#[test]
fn clips_that_cannot_say_their_line_are_stale() {
    let d = dialogue();
    let n = names();
    // One clip for both genders of a line that changes with the gender.
    let both = manifest(&[clip(&sword(), "", "He left his sword with the king.")]);
    assert!(VoiceManifest::is_stale(&both.clips[0], &d, &n));
    assert!(both.playable(&d, &n).is_empty());
    // A line with the lead's name, whatever the clip says.
    let named = manifest(&[clip(&welcome(), "", "Welcome back, Ellery.")]);
    assert!(VoiceManifest::is_stale(&named.clips[0], &d, &n));
    assert!(named.playable(&d, &n).is_empty());
    // A clip whose line is gone (the manifest was read with an older script).
    let mut gone = manifest(&[clip(&hello(), "", "Hello.")]);
    gone.clips[0].line = LineId::new("s_00000000");
    assert!(VoiceManifest::is_stale(&gone.clips[0], &d, &n));
    assert!(gone.playable(&d, &n).is_empty());
    // Reworded speech: a different text under the same clip.
    let reworded = manifest(&[clip(&hello(), "", "Hello there.")]);
    assert!(VoiceManifest::is_stale(&reworded.clips[0], &d, &n));
}

#[test]
fn a_cast_maps_speakers_to_voices() {
    let characters = crate::character::load(None, None, None).ok();
    let source = "(voices: {\n\"test_lord\": \"lord_a\",\n\">\": \"narrator\",\n})";
    let cast = cast_from_source("cast.ron", source, characters.as_ref()).unwrap();
    assert_eq!(cast.voices["test_lord"], "lord_a");
    assert_eq!(cast.voices[NARRATION_SPEAKER], "narrator");
    // Without the character table, any speaker passes.
    let anyone = "(voices: {\"nobody\": \"x\"})";
    assert!(cast_from_source("cast.ron", anyone, None).is_ok());
    assert!(cast_from_source("cast.ron", "(voices: 3)", None).is_err());
}

#[test]
fn a_cast_with_an_unknown_speaker_or_an_empty_voice_is_an_error() {
    let characters = crate::character::load(None, None, None).ok();
    assert!(characters.is_some());
    let source = "(voices: {\n\"nobody\": \"x\",\n\"test_lord\": \" \",\n})";
    let errors: Vec<String> = cast_from_source("cast.ron", source, characters.as_ref())
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errors,
        [
            "cast.ron:2: \"nobody\" is not a character that speaks",
            "cast.ron:3: \"test_lord\" has no voice",
        ]
    );
}
