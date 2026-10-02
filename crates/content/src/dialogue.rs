//! Dialogue scripts (`assets/dialogue/*.dlg`): scenes of portraits, speech
//! and narration. The format is documented in `assets/dialogue/README.md`
//! (ADR-0005). [`parse_dlg`] turns a file into scenes, [`check_scene`] and
//! [`check_duplicates`] validate them, and [`print_scene`] writes a scene
//! back out.

mod check;
mod parse;

use std::collections::BTreeMap;

use trpg_core::CharacterId;

pub use check::{check_duplicates, check_scene};
pub use parse::{ChoiceLines, OptionLines, ParsedScene, parse_dlg};
pub(crate) use parse::{char_problem, is_id};

use crate::audio::AudioManifest;
use crate::bundle;
use crate::character::CharacterTable;
use crate::error::ContentError;
use crate::names::Names;
use crate::portrait::PortraitTable;

/// Directory of dialogue files inside the asset bundle.
pub const DIALOGUE_DIR: &str = "dialogue";
/// File extension of dialogue files.
pub const DIALOGUE_EXTENSION: &str = ".dlg";
/// Longest text of one speech or narration line, in characters (two text
/// boxes of about 3 × 70; ADR-0011).
pub const MAX_TEXT_LEN: usize = 200;
/// Longest text of a reply choice, in characters: it must fit the menu.
pub const MAX_OPTION_LEN: usize = 60;
/// Most text steps (speech or narration) in one reply's reaction, so the
/// scene rejoins quickly.
pub const MAX_REACTION_TEXTS: usize = 4;
/// Longest line the lead may speak outside reply choices
/// (`docs/design/setting-and-tone.md`, "Rules for writing the lead").
pub const MAX_LEAD_LINE_LEN: usize = 40;
/// The expressions every portrait has: the ones a script may use for a
/// character without a portrait. A character with one may use exactly its
/// portrait's expressions.
pub const STANDARD_EXPRESSIONS: [&str; 5] = ["neutral", "happy", "angry", "sad", "surprised"];

/// A side of the dialogue screen, where one portrait stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The left portrait.
    Left,
    /// The right portrait.
    Right,
}

impl Side {
    /// The side as written in a script: `left` or `right`.
    pub fn name(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }

    /// The other side.
    #[must_use]
    pub fn other(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// One step of a scene, in script order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// `@caption <text>`: a location/time caption, shown until the next one.
    Caption {
        /// The caption.
        text: String,
    },
    /// `@left <id> <expression>`: a character enters (or replaces whoever
    /// is) on `side`.
    Place {
        /// Where.
        side: Side,
        /// Who.
        character: CharacterId,
        /// With which expression.
        expression: String,
    },
    /// `@left clear`: the portrait on `side` leaves.
    Clear {
        /// Which side empties.
        side: Side,
    },
    /// `id: text` or `id[expression]: text`: an on-screen character speaks,
    /// first changing expression if one is given.
    Say {
        /// Who speaks.
        speaker: CharacterId,
        /// The speaker's new expression, if it changes.
        expression: Option<String>,
        /// What they say.
        text: String,
    },
    /// `> text`: narration, no speaker.
    Narrate {
        /// The narration.
        text: String,
    },
    /// `@choice` … `@endchoice`: the lead's reply choices (2–3), each with
    /// its own reaction steps. Every option rejoins the scene after the
    /// block.
    Choice {
        /// The options, in menu order.
        options: Vec<ChoiceOption>,
    },
    /// `@music <cue>` or `@music stop`: the music changes when playback
    /// reaches this line, and stays so after the scene.
    Music(MusicLine),
}

/// What a `@music` line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MusicLine {
    /// `@music <cue>`: switch to this music cue of the audio manifest.
    Cue(String),
    /// `@music stop`: fade the music out.
    Stop,
}

impl MusicLine {
    /// What follows `@music` for [`MusicLine::Stop`].
    pub const STOP: &'static str = "stop";

    /// What follows `@music` in a script: the cue id, or `stop`.
    pub fn arg(&self) -> &str {
        match self {
            MusicLine::Cue(cue) => cue,
            MusicLine::Stop => Self::STOP,
        }
    }
}

/// One reply the lead can pick: `* <tone>: <text>`, then its reaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption {
    /// The reply's tone (`earnest`, `wry`…), for writers; not shown.
    pub tone: String,
    /// What the lead says, shown in the menu.
    pub text: String,
    /// The reaction: steps played after picking this option (no choices).
    pub steps: Vec<Step>,
}

impl Step {
    /// The text of a text step (one text box): a `Say` or `Narrate`.
    pub fn text(&self) -> Option<&str> {
        match self {
            Step::Say { text, .. } | Step::Narrate { text } => Some(text),
            Step::Caption { .. }
            | Step::Place { .. }
            | Step::Clear { .. }
            | Step::Choice { .. }
            | Step::Music(_) => None,
        }
    }
}

/// One scene: `@scene <id>` … `@end`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scene {
    /// Scene id, unique across every dialogue file.
    pub id: String,
    /// Its steps, in order.
    pub steps: Vec<Step>,
}

/// Every scene of every dialogue file, by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DialogueTable {
    /// Scenes by id.
    pub scenes: BTreeMap<String, Scene>,
}

impl DialogueTable {
    /// The scene with id `id`.
    pub fn get(&self, id: &str) -> Option<&Scene> {
        self.scenes.get(id)
    }
}

/// Loads and validates every `*.dlg` file in the bundle. Character ids are
/// checked against `characters`, expressions against the characters'
/// `portraits`, name tokens and names written out against `names`, and
/// `@music` cues against `audio`, when given (each is skipped if its files
/// failed to load). Reports every error of every file.
pub fn load(
    characters: Option<&CharacterTable>,
    portraits: Option<&PortraitTable>,
    names: Option<&Names>,
    audio: Option<&AudioManifest>,
) -> Result<DialogueTable, Vec<ContentError>> {
    let files: Vec<(String, Option<&str>)> = bundle::files_in(DIALOGUE_DIR)
        .into_iter()
        .filter(|path| path.ends_with(DIALOGUE_EXTENSION))
        .map(|path| (bundle::display_path(path), bundle::file(path)))
        .collect();
    load_files(&files, characters, portraits, names, audio)
}

/// Like [`from_sources`], for files given as `(file name, source)` where a
/// `None` source is a file that isn't valid UTF-8 (an error).
fn load_files(
    files: &[(String, Option<&str>)],
    characters: Option<&CharacterTable>,
    portraits: Option<&PortraitTable>,
    names: Option<&Names>,
    audio: Option<&AudioManifest>,
) -> Result<DialogueTable, Vec<ContentError>> {
    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for (file, source) in files {
        match source {
            Some(source) => sources.push((file, *source)),
            None => errors.push(ContentError::new(file, "file is not valid UTF-8")),
        }
    }
    match from_sources(&sources, characters, portraits, names, audio) {
        Ok(table) if errors.is_empty() => Ok(table),
        Ok(_) => Err(errors),
        Err(e) => {
            errors.extend(e);
            Err(errors)
        }
    }
}

/// Parses and validates dialogue files given as `(file name, source)`
/// pairs. Reports every problem, ordered by file and line.
pub fn from_sources<F: AsRef<str>>(
    files: &[(F, &str)],
    characters: Option<&CharacterTable>,
    portraits: Option<&PortraitTable>,
    names: Option<&Names>,
    audio: Option<&AudioManifest>,
) -> Result<DialogueTable, Vec<ContentError>> {
    let mut scenes = Vec::new();
    let mut errors = Vec::new();
    for (file, source) in files {
        let (parsed, e) = parse_dlg(file.as_ref(), source);
        errors.extend(e);
        scenes.extend(parsed);
    }
    for s in &scenes {
        errors.extend(check_scene(s, characters, portraits, names, audio));
    }
    errors.extend(check_duplicates(&scenes));
    if errors.is_empty() {
        let scenes = scenes
            .into_iter()
            .map(|p| (p.scene.id.clone(), p.scene))
            .collect();
        Ok(DialogueTable { scenes })
    } else {
        errors.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
        Err(errors)
    }
}

/// Writes `scene` in `.dlg` format (one line per step, reactions indented
/// under their option), ending with `@end` and a newline. Parsing the
/// result gives `scene` back.
pub fn print_scene(scene: &Scene) -> String {
    let mut out = format!("@scene {}\n", scene.id);
    for step in &scene.steps {
        print_step(&mut out, step, "");
    }
    out.push_str("@end\n");
    out
}

/// Appends `step`'s line(s) to `out`, each starting with `indent`.
fn print_step(out: &mut String, step: &Step, indent: &str) {
    let line = match step {
        Step::Caption { text } => format!("@caption {text}"),
        Step::Place {
            side,
            character,
            expression,
        } => format!("@{} {} {expression}", side.name(), character.0),
        Step::Clear { side } => format!("@{} clear", side.name()),
        Step::Say {
            speaker,
            expression: Some(e),
            text,
        } => format!("{}[{e}]: {text}", speaker.0),
        Step::Say {
            speaker,
            expression: None,
            text,
        } => format!("{}: {text}", speaker.0),
        Step::Narrate { text } => format!("> {text}"),
        Step::Music(music) => format!("@music {}", music.arg()),
        Step::Choice { options } => {
            out.push_str("@choice\n");
            for o in options {
                for part in ["* ", &o.tone, ": ", &o.text, "\n"] {
                    out.push_str(part);
                }
                for s in &o.steps {
                    print_step(out, s, "  ");
                }
            }
            "@endchoice".to_owned()
        }
    };
    out.push_str(indent);
    out.push_str(&line);
    out.push('\n');
}

#[cfg(test)]
mod tests;
