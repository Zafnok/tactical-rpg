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
pub use parse::{ParsedScene, parse_dlg};

use crate::bundle;
use crate::character::CharacterTable;
use crate::error::ContentError;

/// Directory of dialogue files inside the asset bundle.
pub const DIALOGUE_DIR: &str = "dialogue";
/// File extension of dialogue files.
pub const DIALOGUE_EXTENSION: &str = ".dlg";
/// Longest text of one speech or narration line, in characters (two text
/// boxes of about 3 × 70; ADR-0011).
pub const MAX_TEXT_LEN: usize = 200;
/// The expressions every portrait has. Until portraits exist (ticket 0703)
/// these are the only expressions a script may use.
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
}

impl Step {
    /// The text of a text step (one text box): a `Say` or `Narrate`.
    pub fn text(&self) -> Option<&str> {
        match self {
            Step::Say { text, .. } | Step::Narrate { text } => Some(text),
            Step::Caption { .. } | Step::Place { .. } | Step::Clear { .. } => None,
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
/// checked against `characters` when given (skipped if the character file
/// failed to load). Reports every error of every file.
pub fn load(characters: Option<&CharacterTable>) -> Result<DialogueTable, Vec<ContentError>> {
    let files: Vec<(String, Option<&str>)> = bundle::files_in(DIALOGUE_DIR)
        .into_iter()
        .filter(|path| path.ends_with(DIALOGUE_EXTENSION))
        .map(|path| (bundle::display_path(path), bundle::file(path)))
        .collect();
    load_files(&files, characters)
}

/// Like [`from_sources`], for files given as `(file name, source)` where a
/// `None` source is a file that isn't valid UTF-8 (an error).
fn load_files(
    files: &[(String, Option<&str>)],
    characters: Option<&CharacterTable>,
) -> Result<DialogueTable, Vec<ContentError>> {
    let mut sources = Vec::new();
    let mut errors = Vec::new();
    for (file, source) in files {
        match source {
            Some(source) => sources.push((file, *source)),
            None => errors.push(ContentError::new(file, "file is not valid UTF-8")),
        }
    }
    match from_sources(&sources, characters) {
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
) -> Result<DialogueTable, Vec<ContentError>> {
    let mut scenes = Vec::new();
    let mut errors = Vec::new();
    for (file, source) in files {
        let (parsed, e) = parse_dlg(file.as_ref(), source);
        errors.extend(e);
        scenes.extend(parsed);
    }
    for s in &scenes {
        errors.extend(check_scene(s, characters));
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

/// Writes `scene` in `.dlg` format (one line per step), ending with `@end`
/// and a newline. Parsing the result gives `scene` back.
pub fn print_scene(scene: &Scene) -> String {
    let mut out = format!("@scene {}\n", scene.id);
    for step in &scene.steps {
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
        };
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("@end\n");
    out
}

#[cfg(test)]
mod tests;
