//! The audio manifest (`assets/audio/audio.ron`, ADR-0026): every sound and
//! music cue the game can play, the music pools, and the credits for every
//! third-party work. Screens name cues; `app` plays the files.
//!
//! Sound files are embedded (under `assets/audio/`). Music files are not:
//! they live in the top-level `music/` folder shipped next to the game, so
//! the bundle can't check them. [`load`] checks the embedded sounds only;
//! a test checks the music folder with [`from_source`].

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the manifest inside the asset bundle.
pub const AUDIO_PATH: &str = "audio/audio.ron";

/// Bundle directory that sound file names are relative to.
pub const SOUND_DIR: &str = "audio";

/// Folder, next to the game (and in the repo root), that music file names
/// are relative to. Not embedded (ADR-0026).
pub const MUSIC_DIR: &str = "music";

/// Licenses a credit may have (ADR-0013). `Own` is work we made.
pub const LICENSES: [&str; 3] = ["CC0-1.0", "CC-BY-4.0", OWN];

/// The license of work we made ourselves.
pub const OWN: &str = "Own";

/// Highest cue volume, in percent of the file's own loudness.
pub const MAX_VOLUME: u8 = 100;

/// The validated audio manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AudioManifest {
    /// How long a music track fades out before the next one starts, in
    /// milliseconds (`audio.md`: 0.5 s, *tunable*).
    pub music_fade_ms: u32,
    /// Sound cues by id.
    pub sounds: BTreeMap<String, SoundCue>,
    /// Music cues by id.
    pub music: BTreeMap<String, MusicCue>,
    /// Music pools by id: the music cues one is picked from.
    pub pools: BTreeMap<String, Vec<String>>,
    /// Credits by id.
    pub credits: BTreeMap<String, Credit>,
}

/// A sound effect: one of its files plays each time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundCue {
    /// The variants, relative to `assets/audio/`; one is picked at random
    /// each time the cue plays. At least one.
    pub files: Vec<String>,
    /// Volume in percent (0–100).
    pub volume: u8,
    /// Who made it.
    pub credit: CreditRef,
}

/// A music track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicCue {
    /// The track, relative to the `music/` folder.
    pub file: String,
    /// Volume in percent (0–100).
    pub volume: u8,
    /// Whether it loops (the default) or plays once.
    pub looped: bool,
    /// Who made it.
    pub credit: CreditRef,
}

fn yes() -> bool {
    true
}

/// Who made a sound or a track.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum CreditRef {
    /// We made it.
    Own,
    /// A third-party work: the id of its entry in `credits`.
    Credit(String),
}

/// One third-party work, for the credits screen (`audio.md` rule 3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Credit {
    /// The work's title.
    pub title: String,
    /// Its author, as they want to be credited.
    pub author: String,
    /// The page it came from.
    pub source: String,
    /// One of [`LICENSES`].
    pub license: String,
    /// Free tags (`music`, `sfx`, the cue, a mood, a place …).
    pub tags: Vec<String>,
    /// Which file or version we use, e.g. "V2 file".
    pub note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    music_fade_ms: u32,
    sounds: Vec<RawSound>,
    music: Vec<RawMusic>,
    pools: Vec<RawPool>,
    credits: Vec<RawCredit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSound {
    id: String,
    files: Vec<String>,
    volume: u8,
    credit: CreditRef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMusic {
    id: String,
    file: String,
    volume: u8,
    #[serde(default = "yes")]
    looped: bool,
    credit: CreditRef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCredit {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    source: String,
    license: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPool {
    id: String,
    music: Vec<String>,
}

/// A file's contents, or `None` if it's missing.
pub type Bytes = Option<Cow<'static, [u8]>>;

/// How to read the referenced files. `music` is `None` where the music
/// folder can't be seen (the game itself: content does no file I/O).
pub struct Files<'a> {
    /// Reads a bundle path (e.g. `audio/sfx/x.wav`).
    pub sound: &'a dyn Fn(&str) -> Bytes,
    /// Reads a file in the music folder.
    pub music: Option<&'a dyn Fn(&str) -> Bytes>,
}

/// The sample rate every file must have: quad-snd (native) plays at
/// 44.1 kHz and resamples anything else crudely (ADR-0026).
pub const SAMPLE_RATE: u32 = 44_100;

/// Why `bytes` (a file named `name`) isn't something the game can play:
/// its content must match its extension (OGG Vorbis for `.ogg`, PCM or
/// float WAV for `.wav`), with one or two channels at [`SAMPLE_RATE`].
/// quad-snd panics on native, and never finishes loading on the web, on a
/// file it can't read, so this runs before any file ships.
pub fn format_problem(name: &str, bytes: &[u8]) -> Option<String> {
    let format = if has_extension(name, &["wav"]) {
        wav_format(bytes).ok_or("not a PCM WAV file")
    } else {
        vorbis_format(bytes).ok_or("not an OGG Vorbis file")
    };
    match format {
        Err(why) => Some(why.to_owned()),
        Ok((channels, _)) if !(1..=2).contains(&channels) => {
            Some(format!("{channels} channels; must be mono or stereo"))
        }
        Ok((_, rate)) if rate != SAMPLE_RATE => {
            Some(format!("{rate} Hz; must be {SAMPLE_RATE} Hz"))
        }
        Ok(_) => None,
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// A WAV's (channels, sample rate), from its `fmt ` chunk.
fn wav_format(bytes: &[u8]) -> Option<(u16, u32)> {
    if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    loop {
        let id = bytes.get(at..at + 4)?;
        let size = usize::try_from(u32_at(bytes, at + 4)?).ok()?;
        if id == b"fmt " {
            // 1 = integer PCM, 3 = float: what quad-snd's decoder reads.
            let encoding = u16_at(bytes, at + 8)?;
            return [1, 3]
                .contains(&encoding)
                .then_some((u16_at(bytes, at + 10)?, u32_at(bytes, at + 12)?));
        }
        at = at.checked_add(size.checked_add(8 + size % 2)?)?;
    }
}

/// An OGG Vorbis file's (channels, sample rate), from the identification
/// header in its first page.
fn vorbis_format(bytes: &[u8]) -> Option<(u16, u32)> {
    if bytes.get(..4)? != b"OggS" {
        return None;
    }
    let packet = 27 + usize::from(*bytes.get(26)?);
    if bytes.get(packet..packet + 7)? != b"\x01vorbis" {
        return None;
    }
    let channels = u16::from(*bytes.get(packet + 11)?);
    Some((channels, u32_at(bytes, packet + 12)?))
}

/// The bundle path of a sound file named in the manifest.
pub fn sound_path(file: &str) -> String {
    format!("{SOUND_DIR}/{file}")
}

/// Loads and validates the embedded manifest. Music files aren't checked
/// (see the module docs).
pub fn load() -> Result<AudioManifest, Vec<ContentError>> {
    let display = bundle::display_path(AUDIO_PATH);
    let source = bundle::file(AUDIO_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    let in_bundle = |path: &str| bundle::bytes(path).map(Cow::Borrowed);
    from_source(
        &display,
        source,
        &Files {
            sound: &in_bundle,
            music: None,
        },
    )
}

/// Parses and validates manifest `source`, attributing errors to `file`.
pub fn from_source(
    file: &str,
    source: &str,
    files: &Files<'_>,
) -> Result<AudioManifest, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut check = Check {
        file,
        source,
        files,
        errors: Vec::new(),
        cue_ids: BTreeSet::new(),
        manifest: AudioManifest {
            music_fade_ms: raw.music_fade_ms,
            ..AudioManifest::default()
        },
    };
    for c in raw.credits {
        check.credit(c);
    }
    for s in raw.sounds {
        check.sound(s);
    }
    for m in raw.music {
        check.music(m);
    }
    for p in raw.pools {
        check.pool(p);
    }
    if check.errors.is_empty() {
        Ok(check.manifest)
    } else {
        Err(check.errors)
    }
}

/// Validation state: the manifest built so far and the problems found.
struct Check<'a> {
    file: &'a str,
    source: &'a str,
    files: &'a Files<'a>,
    errors: Vec<ContentError>,
    /// Sound, music and pool ids seen (one namespace).
    cue_ids: BTreeSet<String>,
    manifest: AudioManifest,
}

impl Check<'_> {
    /// Records a problem with entry `id`, at the line that names it.
    fn err(&mut self, id: &str, message: String) {
        let e = ContentError::new(self.file, message);
        self.errors
            .push(match line_of(self.source, &format!("id: \"{id}\"")) {
                Some(l) => e.at(l, None),
                None => e,
            });
    }

    fn new_cue(&mut self, id: &str) {
        if id.is_empty() {
            self.err(id, "a cue id is empty".into());
        }
        if !self.cue_ids.insert(id.to_owned()) {
            self.err(id, format!("duplicate cue id \"{id}\""));
        }
    }

    fn volume(&mut self, id: &str, volume: u8) {
        if volume > MAX_VOLUME {
            self.err(
                id,
                format!("\"{id}\": volume {volume} is over {MAX_VOLUME} (percent)"),
            );
        }
    }

    fn credit_ref(&mut self, id: &str, credit: &CreditRef) {
        if let CreditRef::Credit(c) = credit
            && !self.manifest.credits.contains_key(c)
        {
            self.err(id, format!("\"{id}\" names unknown credit \"{c}\""));
        }
    }

    fn credit(&mut self, c: RawCredit) {
        let id = c.id;
        let entry = Credit {
            title: c.title,
            author: c.author,
            source: c.source,
            license: c.license,
            tags: c.tags,
            note: c.note,
        };
        if self.manifest.credits.contains_key(&id) {
            self.err(&id, format!("duplicate credit id \"{id}\""));
        }
        for problem in credit_problems(&entry) {
            self.err(&id, format!("credit \"{id}\": {problem}"));
        }
        self.manifest.credits.insert(id, entry);
    }

    fn sound(&mut self, s: RawSound) {
        let id = s.id;
        self.new_cue(&id);
        if s.files.is_empty() {
            self.err(&id, format!("sound \"{id}\" has no files"));
        }
        for f in &s.files {
            let path = bundle::display_path(&sound_path(f));
            if has_extension(f, &["ogg", "wav"]) {
                match (self.files.sound)(&sound_path(f)) {
                    None => self.err(&id, format!("sound \"{id}\": file {path} not found")),
                    Some(bytes) => {
                        if let Some(why) = format_problem(f, &bytes) {
                            self.err(&id, format!("sound \"{id}\": {path}: {why}"));
                        }
                    }
                }
            } else {
                self.err(&id, format!("sound \"{id}\": \"{f}\" must be .ogg or .wav"));
            }
        }
        self.volume(&id, s.volume);
        self.credit_ref(&id, &s.credit);
        let entry = SoundCue {
            files: s.files,
            volume: s.volume,
            credit: s.credit,
        };
        self.manifest.sounds.insert(id, entry);
    }

    fn music(&mut self, m: RawMusic) {
        let id = m.id;
        self.new_cue(&id);
        let f = &m.file;
        if !has_extension(f, &["ogg"]) {
            self.err(&id, format!("music \"{id}\": \"{f}\" must be .ogg"));
        } else if let Some(read) = self.files.music {
            match read(f) {
                None => self.err(
                    &id,
                    format!("music \"{id}\": file {MUSIC_DIR}/{f} not found"),
                ),
                Some(bytes) => {
                    if let Some(why) = format_problem(f, &bytes) {
                        self.err(&id, format!("music \"{id}\": {MUSIC_DIR}/{f}: {why}"));
                    }
                }
            }
        }
        self.volume(&id, m.volume);
        self.credit_ref(&id, &m.credit);
        let entry = MusicCue {
            file: m.file,
            volume: m.volume,
            looped: m.looped,
            credit: m.credit,
        };
        self.manifest.music.insert(id, entry);
    }

    fn pool(&mut self, RawPool { id, music }: RawPool) {
        self.new_cue(&id);
        if music.is_empty() {
            self.err(&id, format!("pool \"{id}\" is empty"));
        }
        for m in &music {
            if self.manifest.sounds.contains_key(m) {
                let why = format!("pool \"{id}\" names \"{m}\", a sound; pools hold music cues");
                self.err(&id, why);
            } else if !self.manifest.music.contains_key(m) {
                self.err(
                    &id,
                    format!("pool \"{id}\" names unknown music cue \"{m}\""),
                );
            }
        }
        self.manifest.pools.insert(id, music);
    }
}

/// What's wrong with a credit: an unknown license, or (for third-party
/// work) a missing title, author or source link.
fn credit_problems(c: &Credit) -> Vec<String> {
    let mut problems = Vec::new();
    if !LICENSES.contains(&c.license.as_str()) {
        problems.push(format!(
            "license \"{}\" is not allowed (ADR-0013: {})",
            c.license,
            LICENSES.join(", ")
        ));
    }
    if c.license != OWN {
        for (field, value) in [("title", &c.title), ("author", &c.author)] {
            if value.trim().is_empty() {
                problems.push(format!("{field} is missing"));
            }
        }
        if !c.source.starts_with("https://") && !c.source.starts_with("http://") {
            problems.push("source must be a web link (http:// or https://)".into());
        }
    }
    problems
}

fn has_extension(file: &str, allowed: &[&str]) -> bool {
    file.rsplit_once('.')
        .is_some_and(|(stem, ext)| !stem.is_empty() && allowed.contains(&ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "(music_fade_ms: 500, sounds: [], music: [], pools: [], credits: [])";

    const FULL: &str = r#"(
    music_fade_ms: 250,
    sounds: [
        (id: "menu_move", files: ["sfx/menu_move.wav"], volume: 100, credit: Own),
        (id: "step_foot", files: ["sfx/step_1.ogg", "sfx/step_2.ogg"], volume: 60, credit: Credit("steps")),
    ],
    music: [
        (id: "title", file: "title.ogg", volume: 80, credit: Credit("sunrise")),
        (id: "sting", file: "sting.ogg", volume: 100, looped: false, credit: Own),
    ],
    pools: [
        (id: "skirmish", music: ["title", "sting"]),
    ],
    credits: [
        (id: "sunrise", title: "New Sunrise", author: "nene",
         source: "https://opengameart.org/content/new-sunrise", license: "CC0-1.0",
         tags: ["music", "title"], note: "V1 file"),
        (id: "steps", title: "Footsteps", author: "someone",
         source: "https://freesound.org/s/1/", license: "CC-BY-4.0"),
        (id: "ours", license: "Own"),
    ],
)"#;

    /// The start of a WAV file: a `LIST` chunk (odd-sized, so padded), then
    /// `fmt `.
    fn wav(channels: u16, rate: u32, encoding: u16) -> Vec<u8> {
        let mut w = b"RIFF\0\0\0\0WAVELIST\x05\0\0\0abcde\0fmt \x10\0\0\0".to_vec();
        w.extend_from_slice(&encoding.to_le_bytes());
        w.extend_from_slice(&channels.to_le_bytes());
        w.extend_from_slice(&rate.to_le_bytes());
        w.extend_from_slice(&[0; 8]);
        w
    }

    /// The first page of an OGG Vorbis file (two lacing values).
    fn ogg(channels: u8, rate: u32) -> Vec<u8> {
        let mut o = b"OggS".to_vec();
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&[2, 30, 0]);
        o.extend_from_slice(b"\x01vorbis\0\0\0\0");
        o.push(channels);
        o.extend_from_slice(&rate.to_le_bytes());
        o
    }

    /// A good file for `name`'s extension.
    fn good(name: &str) -> Vec<u8> {
        if has_extension(name, &["wav"]) {
            wav(2, SAMPLE_RATE, 1)
        } else {
            ogg(2, SAMPLE_RATE)
        }
    }

    fn all_files() -> Files<'static> {
        Files {
            sound: &|p: &str| Some(Cow::Owned(good(p))),
            music: Some(&|f: &str| Some(Cow::Owned(good(f)))),
        }
    }

    fn errors(source: &str) -> Vec<String> {
        errors_with(source, &all_files())
    }

    fn errors_with(source: &str, files: &Files<'_>) -> Vec<String> {
        match from_source("a.ron", source, files) {
            Ok(_) => vec![],
            Err(e) => e.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn the_empty_manifest_is_valid() {
        let m = from_source("a.ron", EMPTY, &all_files());
        assert_eq!(
            m,
            Ok(AudioManifest {
                music_fade_ms: 500,
                ..AudioManifest::default()
            })
        );
    }

    #[test]
    fn a_full_manifest_loads() {
        let m = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert_eq!(m.music_fade_ms, 250);
        assert_eq!(
            m.sounds
                .get("step_foot")
                .map(|s| (&s.files, s.volume, &s.credit)),
            Some((
                &vec!["sfx/step_1.ogg".to_owned(), "sfx/step_2.ogg".to_owned()],
                60,
                &CreditRef::Credit("steps".into())
            ))
        );
        assert_eq!(
            m.sounds.get("menu_move").map(|s| &s.credit),
            Some(&CreditRef::Own)
        );
        let title = m.music.get("title");
        assert_eq!(
            title.map(|t| (t.file.as_str(), t.volume)),
            Some(("title.ogg", 80))
        );
        assert_eq!(
            title.map(|t| t.looped),
            Some(true),
            "music loops by default"
        );
        assert_eq!(m.music.get("sting").map(|t| t.looped), Some(false));
        assert_eq!(
            m.pools.get("skirmish"),
            Some(&vec!["title".to_owned(), "sting".to_owned()])
        );
        let sunrise = m.credits.get("sunrise").cloned().unwrap_or_default();
        assert_eq!(
            (sunrise.title.as_str(), sunrise.author.as_str()),
            ("New Sunrise", "nene")
        );
        assert_eq!(sunrise.license, "CC0-1.0");
        assert_eq!(sunrise.tags, ["music", "title"]);
        assert_eq!(sunrise.note, "V1 file");
        assert_eq!(m.credits.get("ours").map(|c| c.title.as_str()), Some(""));
    }

    #[test]
    fn the_embedded_manifest_is_valid() {
        let m = load();
        assert!(m.is_ok(), "{m:?}");
        // audio.md's starting fade (TUNABLE): 0.5 s.
        assert_eq!(m.map(|m| m.music_fade_ms), Ok(500));
    }

    /// Music files aren't embedded, so the game can't check them; this
    /// test checks the repo's `music/` folder instead (ADR-0026).
    #[test]
    fn every_music_file_is_in_the_music_folder() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../music");
        assert!(dir.is_dir(), "the {MUSIC_DIR}/ folder is missing");
        let exists = |f: &str| std::fs::read(dir.join(f)).ok().map(Cow::Owned);
        let in_bundle = |path: &str| bundle::bytes(path).map(Cow::Borrowed);
        let source = bundle::file(AUDIO_PATH).unwrap_or_default();
        let files = Files {
            sound: &in_bundle,
            music: Some(&exists),
        };
        assert_eq!(errors_with(source, &files), Vec::<String>::new());
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let dup = FULL.replace(r#"(id: "sting""#, r#"(id: "menu_move""#);
        assert_eq!(
            errors(&dup.replace(r#""title", "sting""#, r#""title""#)),
            ["a.ron:4: duplicate cue id \"menu_move\""]
        );
        let pool = FULL.replace(r#"(id: "skirmish""#, r#"(id: "title""#);
        assert_eq!(errors(&pool), ["a.ron:8: duplicate cue id \"title\""]);
        let credit = FULL.replace(r#"(id: "ours""#, r#"(id: "steps""#);
        assert_eq!(errors(&credit), ["a.ron:18: duplicate credit id \"steps\""]);
        let empty = FULL.replace(r#"(id: "sting""#, r#"(id: """#);
        assert_eq!(
            errors(&empty.replace(r#""title", "sting""#, r#""title""#)),
            ["a.ron:9: a cue id is empty"]
        );
    }

    #[test]
    fn missing_files_are_refused() {
        let no_step_2 = |p: &str| (p != "audio/sfx/step_2.ogg").then(|| Cow::Owned(good(p)));
        let no_title = |f: &str| (f != "title.ogg").then(|| Cow::Owned(good(f)));
        let files = Files {
            sound: &no_step_2,
            music: Some(&no_title),
        };
        assert_eq!(
            errors_with(FULL, &files),
            [
                "a.ron:5: sound \"step_foot\": file assets/audio/sfx/step_2.ogg not found",
                "a.ron:8: music \"title\": file music/title.ogg not found",
            ]
        );
        // The game itself can't see the music folder and skips that check.
        let files = Files {
            sound: &|p: &str| Some(Cow::Owned(good(p))),
            music: None,
        };
        assert_eq!(errors_with(FULL, &files), Vec::<String>::new());
    }

    #[test]
    fn unplayable_files_are_refused() {
        let swapped = |name: &str| {
            let bytes = if has_extension(name, &["wav"]) {
                ogg(2, SAMPLE_RATE)
            } else {
                wav(2, SAMPLE_RATE, 1)
            };
            Some(Cow::Owned(bytes))
        };
        let files = Files {
            sound: &swapped,
            music: Some(&swapped),
        };
        assert_eq!(
            errors_with(FULL, &files),
            [
                "a.ron:4: sound \"menu_move\": assets/audio/sfx/menu_move.wav: not a PCM WAV file",
                "a.ron:5: sound \"step_foot\": assets/audio/sfx/step_1.ogg: not an OGG Vorbis file",
                "a.ron:5: sound \"step_foot\": assets/audio/sfx/step_2.ogg: not an OGG Vorbis file",
                "a.ron:8: music \"title\": music/title.ogg: not an OGG Vorbis file",
                "a.ron:9: music \"sting\": music/sting.ogg: not an OGG Vorbis file",
            ]
        );
    }

    #[test]
    fn file_headers_must_be_44_1_khz_mono_or_stereo() {
        let ok = |name, bytes: Vec<u8>| format_problem(name, &bytes);
        assert_eq!(ok("a.wav", wav(1, 44_100, 1)), None);
        assert_eq!(ok("a.wav", wav(2, 44_100, 3)), None, "float WAV");
        assert_eq!(ok("a.ogg", ogg(1, 44_100)), None);
        assert_eq!(ok("a.ogg", ogg(2, 44_100)), None);
        assert_eq!(
            ok("a.wav", wav(2, 48_000, 1)).as_deref(),
            Some("48000 Hz; must be 44100 Hz")
        );
        assert_eq!(
            ok("a.ogg", ogg(2, 22_050)).as_deref(),
            Some("22050 Hz; must be 44100 Hz")
        );
        assert_eq!(
            ok("a.ogg", ogg(6, 44_100)).as_deref(),
            Some("6 channels; must be mono or stereo")
        );
        assert_eq!(
            ok("a.wav", wav(0, 44_100, 1)).as_deref(),
            Some("0 channels; must be mono or stereo")
        );
        let not_wav = Some("not a PCM WAV file");
        assert_eq!(ok("a.wav", wav(2, 44_100, 2)).as_deref(), not_wav, "ADPCM");
        assert_eq!(
            ok("a.wav", b"RIFF\0\0\0\0WAVE".to_vec()).as_deref(),
            not_wav
        );
        assert_eq!(
            ok("a.wav", b"RIFX\0\0\0\0WAVE".to_vec()).as_deref(),
            not_wav
        );
        assert_eq!(
            ok("a.wav", b"RIFF\0\0\0\0AVI ".to_vec()).as_deref(),
            not_wav
        );
        // A good WAV with one of its two magic words wrong.
        let mut rifx = wav(2, 44_100, 1);
        rifx[3] = b'X';
        assert_eq!(ok("a.wav", rifx).as_deref(), not_wav);
        let mut avi = wav(2, 44_100, 1);
        avi[8..12].copy_from_slice(b"AVI ");
        assert_eq!(ok("a.wav", avi).as_deref(), not_wav);
        let mut huge = wav(2, 44_100, 1);
        huge[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(ok("a.wav", huge).as_deref(), not_wav);
        assert_eq!(ok("a.wav", Vec::new()).as_deref(), not_wav);
        let not_ogg = Some("not an OGG Vorbis file");
        let mut opus = ogg(2, 44_100);
        opus[30..36].copy_from_slice(b"opus!!");
        assert_eq!(ok("a.ogg", opus).as_deref(), not_ogg);
        assert_eq!(ok("a.ogg", b"OggS".to_vec()).as_deref(), not_ogg);
        assert_eq!(ok("a.ogg", b"ID3\x03".to_vec()).as_deref(), not_ogg);
        let short = ogg(2, 44_100)[..40].to_vec();
        assert_eq!(ok("a.ogg", short).as_deref(), not_ogg);
    }

    #[test]
    fn file_formats_are_checked() {
        let mp3 = FULL.replace("title.ogg", "title.mp3");
        assert_eq!(
            errors(&mp3),
            ["a.ron:8: music \"title\": \"title.mp3\" must be .ogg"]
        );
        let wav_music = FULL.replace("title.ogg", "title.wav");
        assert_eq!(errors(&wav_music).len(), 1);
        let flac = FULL.replace("menu_move.wav", "menu_move.flac");
        assert_eq!(
            errors(&flac),
            ["a.ron:4: sound \"menu_move\": \"sfx/menu_move.flac\" must be .ogg or .wav"]
        );
        assert_eq!(errors(&FULL.replace("sfx/menu_move.wav", ".wav")).len(), 1);
        assert_eq!(errors(&FULL.replace("sfx/menu_move.wav", "wav")).len(), 1);
        let none = FULL.replace(r#"["sfx/menu_move.wav"]"#, "[]");
        assert_eq!(errors(&none), ["a.ron:4: sound \"menu_move\" has no files"]);
    }

    #[test]
    fn unknown_licenses_are_refused() {
        let nc = FULL.replace("\"CC-BY-4.0\"", "\"CC-BY-NC-4.0\"");
        assert_eq!(
            errors(&nc),
            [
                "a.ron:18: credit \"steps\": license \"CC-BY-NC-4.0\" is not allowed \
                 (ADR-0013: CC0-1.0, CC-BY-4.0, Own)"
            ]
        );
    }

    #[test]
    fn third_party_credits_need_title_author_and_link() {
        let bare = FULL.replace(
            r#"title: "Footsteps", author: "someone","#,
            r#"title: " ", author: "","#,
        );
        let bare = bare.replace("https://freesound.org/s/1/", "freesound 1");
        assert_eq!(
            errors(&bare),
            [
                "a.ron:18: credit \"steps\": title is missing",
                "a.ron:18: credit \"steps\": author is missing",
                "a.ron:18: credit \"steps\": source must be a web link (http:// or https://)",
            ]
        );
        let http = FULL.replace("https://freesound", "http://freesound");
        assert_eq!(errors(&http), Vec::<String>::new());
    }

    #[test]
    fn credits_must_exist() {
        let unknown = FULL.replace(r#"Credit("steps")"#, r#"Credit("stepz")"#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:5: \"step_foot\" names unknown credit \"stepz\""]
        );
        let unknown = FULL.replace(r#"Credit("sunrise")"#, r#"Credit("dawn")"#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:8: \"title\" names unknown credit \"dawn\""]
        );
    }

    #[test]
    fn pools_hold_only_music_cues() {
        let sound = FULL.replace(r#""title", "sting""#, r#""title", "menu_move""#);
        assert_eq!(
            errors(&sound),
            ["a.ron:12: pool \"skirmish\" names \"menu_move\", a sound; pools hold music cues"]
        );
        let unknown = FULL.replace(r#""title", "sting""#, r#""title", "nope""#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:12: pool \"skirmish\" names unknown music cue \"nope\""]
        );
        let empty = FULL.replace(r#""title", "sting""#, "");
        assert_eq!(errors(&empty), ["a.ron:12: pool \"skirmish\" is empty"]);
    }

    #[test]
    fn volumes_are_percentages() {
        assert_eq!(
            errors(&FULL.replace("volume: 60", "volume: 100")),
            Vec::<String>::new()
        );
        assert_eq!(
            errors(&FULL.replace("volume: 60", "volume: 101")),
            ["a.ron:5: \"step_foot\": volume 101 is over 100 (percent)"]
        );
        assert_eq!(
            errors(&FULL.replace("volume: 80", "volume: 101")),
            ["a.ron:8: \"title\": volume 101 is over 100 (percent)"]
        );
    }

    #[test]
    fn syntax_and_unknown_fields_are_refused() {
        assert_eq!(errors("(").len(), 1);
        assert_eq!(
            errors(&FULL.replace("volume: 80", "volume: 80, loud: true")).len(),
            1
        );
        assert_eq!(errors(&EMPTY.replace("pools: [], ", "")).len(), 1);
    }

    #[test]
    fn sound_paths_are_under_the_audio_directory() {
        assert_eq!(sound_path("sfx/a.wav"), "audio/sfx/a.wav");
    }
}
