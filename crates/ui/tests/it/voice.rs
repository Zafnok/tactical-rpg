//! Voice clips through the Harness (ticket 0238, ADR-0046): a line is
//! asked for only when it has a clip that may be played and voices are on.
//! No game screen plays a voice yet (0720), so the "Voice test" debug tool
//! stands in for one.

use trpg_content::{LineId, Variant};
use trpg_ui::AudioRequest;
use trpg_ui::debug::TEST_SCENE;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

/// `(id, text)` of the test scene's first four lines, none of which has a
/// token.
fn lines(h: &Harness) -> Vec<(LineId, String)> {
    let scene = &h.game().ctx().content.dialogue.scenes[TEST_SCENE];
    let lines = scene.lines();
    let pair = |l: &trpg_content::dialogue::Line| (l.id.clone(), l.text.to_owned());
    lines.iter().take(4).map(pair).collect()
}

/// A manifest with one clip per `(line, spoken)`.
fn manifest(clips: &[(&LineId, &str)]) -> String {
    let clip = |(line, spoken): &(&LineId, &str)| {
        format!(
            "(line: \"{line}\", spoken: \"{spoken}\", voice: \"v\", \
             made_by: Generated(tool: \"t\", model: \"m\", date: \"2026-10-03\")),\n"
        )
    };
    format!(
        "(clips: [\n{}])",
        clips.iter().map(clip).collect::<String>()
    )
}

/// The debug menu with "Voice test" focused (the last tool but one).
fn on_voice_test() -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("F2 Up Up");
    assert_eq!(h.top_screen(), "debug_menu");
    h
}

fn play(line: &LineId) -> AudioRequest {
    AudioRequest::PlayVoice {
        line: line.clone(),
        variant: Variant::None,
    }
}

/// The voice requests of the run so far.
fn voice_requests(h: &Harness) -> Vec<AudioRequest> {
    let voice = |r: &AudioRequest| !matches!(r, AudioRequest::PlaySound { .. });
    h.audio_requests().into_iter().filter(voice).collect()
}

/// Acceptance: a line with a clip queues `PlayVoice`; a line without one,
/// with a stale one, or with voices off, queues nothing.
#[test]
fn only_lines_with_a_playable_clip_are_asked_for() {
    let mut h = on_voice_test();
    let lines = lines(&h);
    // No manifest: the tool has nothing to say.
    h.clear_audio().keys("f");
    assert!(h.voices().is_empty());
    // The first line has a clip, the second a stale one, the third none,
    // the fourth a clip.
    let source = manifest(&[
        (&lines[0].0, &lines[0].1),
        (&lines[1].0, "What it said before a rewrite."),
        (&lines[3].0, &lines[3].1),
    ]);
    h.ctx_mut().set_voice_manifest(&source);
    assert!(h.ctx_mut().take_warnings().is_empty());
    h.clear_audio().keys("f");
    let preload = AudioRequest::PreloadVoices {
        lines: vec![
            (lines[0].0.clone(), Variant::None),
            (lines[3].0.clone(), Variant::None),
        ],
    };
    assert_eq!(voice_requests(&h), [preload, play(&lines[0].0)]);
    h.clear_audio().keys("f");
    assert_eq!(voice_requests(&h), [play(&lines[3].0)]);
    assert_eq!(h.voices(), [lines[3].0.to_string()]);
    // Asked for directly, as a screen would: the stale and the missing
    // clip stay silent.
    h.clear_audio();
    for (line, _) in &lines {
        h.ctx_mut().play_voice(line);
    }
    h.wait(0.0);
    assert_eq!(h.voices(), [lines[0].0.to_string(), lines[3].0.to_string()]);
    // Voices off: nothing at all.
    h.ctx_mut().voices_on = false;
    h.clear_audio().keys("f");
    h.ctx_mut().play_voice(&lines[0].0);
    h.wait(0.0);
    assert!(voice_requests(&h).is_empty());
    assert_eq!(h.sounds(), ["menu_select"]);
}

#[test]
fn stop_voice_reaches_the_frame_output() {
    let mut h = on_voice_test();
    h.clear_audio();
    h.ctx_mut().stop_voice();
    h.wait(0.0);
    assert_eq!(voice_requests(&h), [AudioRequest::StopVoice]);
    assert!(
        h.music_commands().is_empty(),
        "voices leave the music alone"
    );
}

/// Each frame carries the voice volume for `app`.
#[test]
fn frames_carry_the_voice_volume() {
    let mut ctx = trpg_ui::Ctx::embedded().unwrap();
    ctx.voice_volume = 5;
    let mut game = trpg_ui::Game::start(ctx);
    let volume = game.frame(&[], 0.0).voice_volume;
    assert!((volume - 0.5).abs() < f32::EPSILON, "{volume}");
}
