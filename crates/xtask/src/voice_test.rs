//! `cargo xtask voice-test-clips`: makes the clips that test voice playback
//! (ticket 0238, `docs/voice.md`): one short phrase of tones (not speech)
//! per entry of `crates/app/tests/voice/en/voice.ron`, written as OGG
//! Vorbis where the game looks for that entry's clip.
//!
//! The tones come from the sound tool ([`mod@crate::sfx`]); `ffmpeg` encodes
//! them (MSYS2: `pacman -S mingw-w64-x86_64-ffmpeg`), as it does the music
//! (`assets-src/audio/import.py`). Real clips are made by `cargo xtask
//! voice` (ticket 0721) and never live in this repository (ADR-0046).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use trpg_content::voice::{self, MANIFEST_FILE};

use crate::sfx;

/// The test voice folder of the script's language, relative to the repo
/// root.
pub const DIR: &str = "crates/app/tests/voice/en";

/// Where `ffmpeg` is when it isn't on the `PATH`.
const MSYS_FFMPEG: &str = "C:/msys64/mingw64/bin/ffmpeg.exe";

/// The clips to write: each file (relative to [`DIR`]) and its samples, in
/// manifest order. Fails if the test manifest is missing or invalid.
pub fn plan(root: &Path) -> Result<Vec<(String, Vec<i16>)>, String> {
    let content = trpg_content::load_embedded().map_err(|e| e.to_string())?;
    let path = format!("{DIR}/{MANIFEST_FILE}");
    let source =
        fs::read_to_string(root.join(&path)).map_err(|e| format!("reading {path}: {e}"))?;
    let manifest = voice::from_source(&path, &source, &content.dialogue).map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    })?;
    Ok(manifest
        .clips
        .iter()
        .enumerate()
        .map(|(i, clip)| {
            (
                voice::clip_file(&clip.line, clip.variant),
                sfx::test_phrase(i),
            )
        })
        .collect())
}

/// The `ffmpeg` to run: MSYS2's where it is installed, else the one on the
/// `PATH`.
///
/// Not covered by mutation testing (`#[mutants::skip]`), like [`encode`]:
/// the two only find and spawn an external tool that isn't installed
/// where the tests run (as `web.rs` does for `wasm-opt`). What they make
/// is checked: a test compares the committed clips with [`plan`].
#[mutants::skip]
fn ffmpeg() -> &'static str {
    if Path::new(MSYS_FFMPEG).is_file() {
        MSYS_FFMPEG
    } else {
        "ffmpeg"
    }
}

/// Encodes the WAV at `wav` as OGG Vorbis at `ogg`: mono, 44.1 kHz
/// (ADR-0026 §4), without the encoder's name and date so the same input
/// gives the same file.
///
/// Not covered by mutation testing: see [`ffmpeg`].
#[mutants::skip]
fn encode(wav: &Path, ogg: &Path) -> Result<(), String> {
    let output = Command::new(ffmpeg())
        .args(["-hide_banner", "-nostats", "-loglevel", "error", "-y", "-i"])
        .arg(wav)
        .args(["-c:a", "libvorbis", "-q:a", "4", "-ac", "1", "-ar", "44100"])
        .args(["-map_metadata", "-1", "-fflags", "+bitexact"])
        .args(["-flags:a", "+bitexact"])
        .arg(ogg)
        .output()
        .map_err(|e| format!("running ffmpeg (is it installed?): {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let said = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "ffmpeg failed on {}: {}",
            ogg.display(),
            said.trim()
        ))
    }
}

/// Runs the command: writes every clip of [`plan`] under `<root>/`[`DIR`].
pub fn run(root: &Path) -> Result<String, String> {
    let clips = plan(root)?;
    let dir = root.join(DIR);
    let scratch = std::env::temp_dir().join("trpg-voice-test-clips");
    fs::create_dir_all(&scratch).map_err(|e| format!("creating {}: {e}", scratch.display()))?;
    for (i, (file, samples)) in clips.iter().enumerate() {
        let wav = scratch.join(format!("{i}.wav"));
        fs::write(&wav, sfx::wav_bytes(samples))
            .map_err(|e| format!("writing {}: {e}", wav.display()))?;
        let ogg: PathBuf = dir.join(file);
        if let Some(parent) = ogg.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("creating {}: {e}", parent.display()))?;
        }
        encode(&wav, &ogg)?;
    }
    // Scratch files in the system's temporary folder: failing to tidy up
    // is not a failure of the command.
    let _ = fs::remove_dir_all(&scratch);
    let files: Vec<&str> = clips.iter().map(|(file, _)| file.as_str()).collect();
    Ok(format!(
        "voice-test-clips: wrote {} clips under {DIR}: {}",
        files.len(),
        files.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use trpg_content::audio::{SAMPLE_RATE, ogg_length_ms};

    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn the_plan_has_a_clip_per_manifest_entry_in_the_test_scene() {
        let clips = plan(&root()).unwrap();
        let files: Vec<&str> = clips.iter().map(|(file, _)| file.as_str()).collect();
        assert_eq!(
            files,
            [
                "test/test_21833604.ogg",
                "test/test_7de7b254.ogg",
                "test/test_01439969.ogg",
            ]
        );
        // Each is a different phrase, so a test can tell which one plays.
        assert_ne!(clips[0].1, clips[1].1);
        assert_ne!(clips[1].1, clips[2].1);
    }

    /// The committed clips are the planned ones: each file is there and as
    /// long as its phrase (Vorbis keeps the length to the sample).
    #[test]
    fn the_committed_clips_match_the_plan() {
        for (file, samples) in plan(&root()).unwrap() {
            let path = root().join(DIR).join(&file);
            let bytes = fs::read(&path).unwrap_or_default();
            let ms = u64::try_from(samples.len()).unwrap() * 1000 / u64::from(SAMPLE_RATE);
            let length = ogg_length_ms(&bytes).map(u64::from);
            assert!(
                length.is_some_and(|l| l.abs_diff(ms) <= 1),
                "{file}: {length:?} ms, planned {ms} ms; rerun `cargo xtask voice-test-clips`"
            );
        }
    }

    #[test]
    fn a_missing_manifest_is_an_error_naming_it() {
        let nowhere = std::env::temp_dir().join("trpg-voice-test-nowhere");
        let error = plan(&nowhere).unwrap_err();
        assert!(
            error.starts_with("reading crates/app/tests/voice/en/voice.ron"),
            "{error}"
        );
        assert_eq!(run(&nowhere).unwrap_err(), error);
    }
}
