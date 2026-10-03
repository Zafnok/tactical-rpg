//! Voice clips (ADR-0046, `docs/voice.md`): the manifest
//! `voice/<lang>/voice.ron` that lists every clip by dialogue line id, and
//! the cast `voice/<lang>/cast.ron`. The `voice/` folder ships beside the
//! game and is not embedded, so nothing here reads a file: `app` reads the
//! text and these functions parse and validate it.
//!
//! A clip records the words it says (`spoken`). When the line says
//! something else today (a name in `names.ron` changed), the clip is
//! **stale** and is left out of the [`Playable`] set.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::lead::{self, LeadGender, LeadProfile, Part};

use crate::character::CharacterTable;
use crate::dialogue::{DialogueTable, LineId, NARRATION_SPEAKER};
use crate::error::ContentError;
use crate::names::Names;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Folder, next to the game, that holds one folder of clips per language.
/// Not embedded (ADR-0046 rule 4).
pub const VOICE_DIR: &str = "voice";
/// The manifest's file name inside a language's folder.
pub const MANIFEST_FILE: &str = "voice.ron";
/// The cast's file name inside a language's folder.
pub const CAST_FILE: &str = "cast.ron";
/// The language the script is written in: the only voice folder read until
/// the player can pick a language (ADR-0045 §5).
pub const SOURCE_LANG: &str = "en";

/// Which recording of a line a clip is: lines whose words change with the
/// lead's gender (`{they}`, `{their}`…) have one for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize)]
pub enum Variant {
    /// The line's only clip: `<line id>.ogg`.
    #[default]
    None,
    /// Said of a male lead: `<line id>.m.ogg`.
    M,
    /// Said of a female lead: `<line id>.f.ogg`.
    F,
}

impl Variant {
    /// The variant said of a lead of `gender`.
    pub const fn of(gender: LeadGender) -> Self {
        match gender {
            LeadGender::Male => Self::M,
            LeadGender::Female => Self::F,
        }
    }

    /// The lead's gender this variant is said of.
    pub const fn gender(self) -> Option<LeadGender> {
        match self {
            Self::None => None,
            Self::M => Some(LeadGender::Male),
            Self::F => Some(LeadGender::Female),
        }
    }

    /// What goes between the line id and `.ogg` in the file name.
    const fn suffix(self) -> &'static str {
        match self {
            Self::None => "",
            Self::M => ".m",
            Self::F => ".f",
        }
    }
}

/// Who or what made a clip (ADR-0046 rules 3 and 9: the credits say which
/// voices are generated).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MadeBy {
    /// Made by a text-to-speech tool.
    Generated {
        /// The tool.
        tool: String,
        /// The model or voice engine version.
        model: String,
        /// The day it was made, `YYYY-MM-DD`.
        date: String,
    },
    /// Recorded by a person.
    Recorded {
        /// The actor, as they want to be credited.
        actor: String,
    },
}

/// One clip of the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceClip {
    /// The dialogue line it says.
    pub line: LineId,
    /// Which recording of the line it is.
    pub variant: Variant,
    /// The words it says: the line's text with every token filled in, as
    /// it was when the clip was made.
    pub spoken: String,
    /// The cast voice that says it (a value of the [`Cast`]).
    pub voice: String,
    /// Who or what made it.
    pub made_by: MadeBy,
}

/// The validated voice manifest of one language.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoiceManifest {
    /// Every clip, in file order.
    pub clips: Vec<VoiceClip>,
}

/// Which voice says each speaker's lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cast {
    /// Voice ids by speaker: a character id, or
    /// [`NARRATION_SPEAKER`] for narration.
    pub voices: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    clips: Vec<RawClip>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClip {
    line: String,
    #[serde(default)]
    variant: Variant,
    spoken: String,
    voice: String,
    made_by: MadeBy,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCast {
    voices: BTreeMap<String, String>,
}

/// The words `text` (a dialogue line as written) says of a lead of
/// `gender`, with today's `names`: every name and pronoun token filled in.
/// `None` for a line with `{lead}`: the player types that name, so no clip
/// can say it.
pub fn spoken_text(text: &str, names: &Names, gender: LeadGender) -> Option<String> {
    // Only the pronouns are read: a line with the name has no spoken text.
    let lead = LeadProfile::new("", gender);
    let mut out = String::with_capacity(text.len());
    for part in lead::split_tokens(&names.substitute(text)) {
        match part {
            Part::Token(lead::NAME_TOKEN) => return None,
            Part::Token(t) => {
                if let Some(word) = lead.token(t) {
                    out.push_str(&word);
                } else {
                    out.push('{');
                    out.push_str(t);
                    out.push('}');
                }
            }
            Part::Text(t) | Part::Unclosed(t) => out.push_str(t),
        }
    }
    Some(out)
}

/// Whether the words of `text` change with the lead's gender (it has a
/// pronoun token), so it needs an [`M`](Variant::M) and an
/// [`F`](Variant::F) clip.
pub fn changes_with_gender(text: &str) -> bool {
    lead::split_tokens(text).any(|part| match part {
        Part::Token(t) => t != lead::NAME_TOKEN && lead::is_token(t),
        Part::Text(_) | Part::Unclosed(_) => false,
    })
}

/// A clip's file, relative to its language's folder:
/// `<scene id>/<line id>.ogg`, with `.m` or `.f` before `.ogg` for a
/// gendered variant (ADR-0046 rule 2).
pub fn clip_file(line: &LineId, variant: Variant) -> String {
    format!("{}/{line}{}.ogg", line.scene_id(), variant.suffix())
}

/// The text of every dialogue line, by line id.
fn line_texts(dialogue: &DialogueTable) -> BTreeMap<&str, &str> {
    dialogue
        .scenes
        .values()
        .flat_map(crate::dialogue::Scene::lines)
        .map(|line| (line.id.as_str(), line.text))
        .collect()
}

/// Parses and validates manifest `source`, attributing errors to `file`:
/// every clip's line exists in `dialogue`, no line has the same variant
/// twice, a line has either one clip or an `M` and `F` pair, and `spoken`
/// and `voice` aren't empty. Reports every problem found.
///
/// A stale clip is not an error (see [`VoiceManifest::playable`]).
pub fn from_source(
    file: &str,
    source: &str,
    dialogue: &DialogueTable,
) -> Result<VoiceManifest, Vec<ContentError>> {
    let raw: RawManifest = parse_ron(file, source).map_err(|e| vec![e])?;
    let texts = line_texts(dialogue);
    let mut errors = Vec::new();
    let mut err = |line: &str, message: String| {
        let e = ContentError::new(file, message);
        errors.push(match line_of(source, &format!("\"{line}\"")) {
            Some(l) => e.at(l, None),
            None => e,
        });
    };
    let mut variants: BTreeMap<&str, Vec<Variant>> = BTreeMap::new();
    for clip in &raw.clips {
        let id = clip.line.as_str();
        if !texts.contains_key(id) {
            err(id, format!("no dialogue line has the id \"{id}\""));
        }
        let seen = variants.entry(id).or_default();
        if seen.contains(&clip.variant) {
            err(
                id,
                format!("line \"{id}\" has two {:?} clips", clip.variant),
            );
        }
        seen.push(clip.variant);
        if clip.spoken.trim().is_empty() {
            err(id, format!("the clip of line \"{id}\" has no spoken text"));
        }
        if clip.voice.trim().is_empty() {
            err(id, format!("the clip of line \"{id}\" has no voice"));
        }
    }
    for (&id, seen) in &variants {
        let has = |v| seen.contains(&v);
        if has(Variant::M) != has(Variant::F) {
            err(
                id,
                format!("line \"{id}\" has an M or an F clip without the other"),
            );
        }
        if has(Variant::None) && (has(Variant::M) || has(Variant::F)) {
            err(
                id,
                format!(
                    "line \"{id}\" has a clip for both genders and M or F clips; keep one kind"
                ),
            );
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let clips = raw
        .clips
        .into_iter()
        .map(|c| VoiceClip {
            line: LineId::new(c.line),
            variant: c.variant,
            spoken: c.spoken,
            voice: c.voice,
            made_by: c.made_by,
        })
        .collect();
    Ok(VoiceManifest { clips })
}

/// Parses and validates cast `source`, attributing errors to `file`: no
/// voice id is empty, and every speaker is [`NARRATION_SPEAKER`] or, when
/// `characters` is given, a character that can speak.
pub fn cast_from_source(
    file: &str,
    source: &str,
    characters: Option<&CharacterTable>,
) -> Result<Cast, Vec<ContentError>> {
    let raw: RawCast = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    for (speaker, voice) in &raw.voices {
        let mut err = |message: String| {
            let e = ContentError::new(file, message);
            errors.push(match line_of(source, &format!("\"{speaker}\"")) {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        if voice.trim().is_empty() {
            err(format!("\"{speaker}\" has no voice"));
        }
        let known = speaker == NARRATION_SPEAKER
            || characters.is_none_or(|c| c.can_speak(&trpg_core::CharacterId(speaker.clone())));
        if !known {
            err(format!("\"{speaker}\" is not a character that speaks"));
        }
    }
    if errors.is_empty() {
        Ok(Cast { voices: raw.voices })
    } else {
        Err(errors)
    }
}

/// The clips that may be played: those that say what their line says
/// today.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Playable {
    clips: BTreeSet<(LineId, Variant)>,
}

impl Playable {
    /// The clip to play for `line` when the lead is of `gender`: the line's
    /// only clip, else the one said of that gender. `None` if the line has
    /// no playable clip.
    pub fn variant_for(&self, line: &LineId, gender: LeadGender) -> Option<Variant> {
        [Variant::None, Variant::of(gender)]
            .into_iter()
            .find(|&v| self.clips.contains(&(line.clone(), v)))
    }

    /// How many clips may be played.
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Whether no clip may be played.
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
}

impl VoiceManifest {
    /// Whether `clip` is stale: its `spoken` is not what its line says
    /// today (with `names`, of the lead its variant is for). A clip of a
    /// line that is gone, has `{lead}`, or changes with the lead's gender
    /// while the clip is for both, is stale too.
    pub fn is_stale(clip: &VoiceClip, dialogue: &DialogueTable, names: &Names) -> bool {
        let Some(&text) = line_texts(dialogue).get(clip.line.as_str()) else {
            return true;
        };
        stale(clip, text, names)
    }

    /// The clips that aren't stale ([`is_stale`](Self::is_stale)).
    pub fn playable(&self, dialogue: &DialogueTable, names: &Names) -> Playable {
        let texts = line_texts(dialogue);
        let clips = self
            .clips
            .iter()
            .filter(|clip| {
                texts
                    .get(clip.line.as_str())
                    .is_some_and(|text| !stale(clip, text, names))
            })
            .map(|clip| (clip.line.clone(), clip.variant))
            .collect();
        Playable { clips }
    }
}

/// Whether `clip`, of the line written `text`, is stale.
fn stale(clip: &VoiceClip, text: &str, names: &Names) -> bool {
    let gender = match clip.variant.gender() {
        Some(gender) => gender,
        None if changes_with_gender(text) => return true,
        None => LeadGender::Male,
    };
    spoken_text(text, names, gender).is_none_or(|spoken| spoken != clip.spoken)
}

#[cfg(test)]
mod tests;
