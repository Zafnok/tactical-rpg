//! Checks on parsed scenes: who is on screen, known characters and
//! expressions, text length, lead and name tokens, names written out, lead
//! lines, reply choices, music cues, line ids that clash, and scene ids
//! unique across files.

use std::collections::BTreeMap;

use trpg_core::CharacterId;
use trpg_core::lead::{self, LEAD_ID, PORTRAIT_FEMALE, PORTRAIT_MALE, Part};

use super::line_id::line_hash;
use super::parse::{ChoiceLines, ParsedScene};
use super::{
    ChoiceOption, MAX_LEAD_LINE_LEN, MAX_OPTION_LEN, MAX_REACTION_TEXTS, MAX_TEXT_LEN, MusicLine,
    NARRATION_SPEAKER, REPLY_SPEAKER, STANDARD_EXPRESSIONS, Side, Step,
};
use crate::audio::AudioManifest;
use crate::character::CharacterTable;
use crate::error::ContentError;
use crate::names::{Names, is_name_id, name_token};
use crate::portrait::{Portrait, PortraitTable};

/// Who stands on the left and right.
type Screen<'a> = [Option<&'a CharacterId>; 2];

/// Checks one scene, replaying it to know who is on screen at each line.
/// Character ids are checked against `characters`, expressions against
/// `portraits`, name tokens and names written out against `names`, and
/// `@music` cues against `audio`, when given.
pub fn check_scene(
    parsed: &ParsedScene,
    characters: Option<&CharacterTable>,
    portraits: Option<&PortraitTable>,
    names: Option<&Names>,
    audio: Option<&AudioManifest>,
) -> Vec<ContentError> {
    let mut c = Checker {
        file: &parsed.file,
        characters,
        portraits,
        names,
        audio,
        hashes: BTreeMap::new(),
        errors: Vec::new(),
    };
    let mut state = State::default();
    let mut choices = parsed.choice_lines.iter();
    for (step, &line) in parsed.scene.steps.iter().zip(&parsed.step_lines) {
        match step {
            Step::Choice { options } => {
                if let Some(lines) = choices.next() {
                    c.choice(line, options, lines, &mut state);
                }
            }
            _ => c.step(step, line, &mut state),
        }
    }
    if !parsed.scene.steps.iter().any(has_text) {
        c.err(
            parsed.line,
            format!("scene \"{}\" has no speech or narration", parsed.scene.id),
        );
    }
    c.errors
}

/// Whether `step` is, or contains, a speech or narration line.
fn has_text(step: &Step) -> bool {
    match step {
        Step::Choice { options } => options.iter().any(|o| o.steps.iter().any(has_text)),
        _ => step.text().is_some(),
    }
}

/// What the replay knows at a point of the scene.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct State<'a> {
    screen: Screen<'a>,
    caption: Option<&'a str>,
}

struct Checker<'a> {
    file: &'a str,
    characters: Option<&'a CharacterTable>,
    portraits: Option<&'a PortraitTable>,
    names: Option<&'a Names>,
    audio: Option<&'a AudioManifest>,
    /// The first line of the scene with each line-id hash: its source
    /// line, speaker and text.
    hashes: BTreeMap<String, (u32, String, String)>,
    errors: Vec<ContentError>,
}

fn slot(side: Side) -> usize {
    match side {
        Side::Left => 0,
        Side::Right => 1,
    }
}

impl<'a> Checker<'a> {
    fn err(&mut self, line: u32, message: String) {
        self.errors
            .push(ContentError::new(self.file, message).at(line, None));
    }

    fn known(&self, id: &CharacterId) -> bool {
        self.characters.is_none_or(|t| t.can_speak(id))
    }

    /// Reports a line (`speaker` saying `text`) whose id hash an earlier,
    /// different line of the scene has: their ids would depend on their
    /// order. A repeat of the same line is fine (it gets `_2`).
    fn line_id(&mut self, line: u32, speaker: &str, text: &str) {
        let hash = line_hash(speaker, text);
        match self.hashes.get(&hash) {
            None => {
                self.hashes
                    .insert(hash, (line, speaker.to_owned(), text.to_owned()));
            }
            Some((_, s, t)) if s == speaker && t == text => {}
            Some((first, _, t)) => {
                let message = format!(
                    "this line and line {first} (\"{t}\") get the same line id hash \
                     {hash}; reword one of them"
                );
                self.err(line, message);
            }
        }
    }

    /// Checks one step (not a choice) on `line` and applies it to `state`.
    fn step<'s>(&mut self, step: &'s Step, line: u32, state: &mut State<'s>) {
        if let Some(text) = step.text() {
            self.tokens(line, text);
            self.literal_names(line, text);
            let len = self.len(text);
            if len > MAX_TEXT_LEN {
                self.err(
                    line,
                    format!("text is {len} characters; the limit is {MAX_TEXT_LEN}"),
                );
            }
        }
        match step {
            Step::Caption { text } => {
                self.tokens(line, text);
                self.literal_names(line, text);
                state.caption = Some(text);
            }
            Step::Place {
                side,
                character,
                expression,
            } => {
                if !self.known(character) {
                    self.err(line, format!("unknown character \"{}\"", character.0));
                }
                if let Some(message) = self.expression_problem(character, expression) {
                    self.err(line, message);
                }
                if state.screen[slot(side.other())] == Some(character) {
                    self.err(
                        line,
                        format!(
                            "\"{}\" is already on the {}; a character can't be on both sides",
                            character.0,
                            side.other().name()
                        ),
                    );
                }
                state.screen[slot(*side)] = Some(character);
            }
            Step::Clear { side } => state.screen[slot(*side)] = None,
            Step::Say {
                speaker,
                expression,
                text,
                ..
            } => {
                self.line_id(line, &speaker.0, text);
                if !self.known(speaker) {
                    self.err(line, format!("unknown character \"{}\"", speaker.0));
                } else if !state.screen.contains(&Some(speaker)) {
                    self.err(
                        line,
                        format!(
                            "\"{}\" speaks but is not on screen; place them with @left or @right first",
                            speaker.0
                        ),
                    );
                }
                let problem = expression
                    .as_deref()
                    .and_then(|e| self.expression_problem(speaker, e));
                if let Some(message) = problem {
                    self.err(line, message);
                }
                let len = self.len(text);
                if speaker.0 == LEAD_ID && len > MAX_LEAD_LINE_LEN {
                    self.err(
                        line,
                        format!(
                            "the lead's line is {len} characters; the limit is {MAX_LEAD_LINE_LEN}: \
                             the lead speaks in short, neutral lines and otherwise through reply \
                             choices (docs/design/setting-and-tone.md, \"Rules for writing the lead\")"
                        ),
                    );
                }
            }
            Step::Music(MusicLine::Cue(cue)) => {
                if let Some(message) = self.audio.and_then(|a| music_problem(a, cue)) {
                    self.err(line, message);
                }
            }
            Step::Narrate { text, .. } => self.line_id(line, NARRATION_SPEAKER, text),
            Step::Choice { .. } | Step::Music(MusicLine::Stop) => {}
        }
    }

    /// Checks a `@choice` block on `line` and applies it to `state`: every
    /// option must leave the screen as the first one does.
    fn choice<'s>(
        &mut self,
        line: u32,
        options: &'s [ChoiceOption],
        lines: &ChoiceLines,
        state: &mut State<'s>,
    ) {
        let n = options.len();
        if !(2..=3).contains(&n) {
            self.err(line, format!("@choice has {n} options; it needs 2 or 3"));
        }
        let mut first: Option<State<'s>> = None;
        for (option, option_lines) in options.iter().zip(&lines.options) {
            let at = option_lines.line;
            self.line_id(at, REPLY_SPEAKER, &option.text);
            self.tokens(at, &option.text);
            self.literal_names(at, &option.text);
            let len = self.len(&option.text);
            if len > MAX_OPTION_LEN {
                self.err(
                    at,
                    format!(
                        "option text is {len} characters; the limit is {MAX_OPTION_LEN}, so it fits the menu"
                    ),
                );
            }
            let texts = option.steps.iter().filter(|s| s.text().is_some()).count();
            if texts > MAX_REACTION_TEXTS {
                self.err(
                    at,
                    format!(
                        "reaction has {texts} speech or narration lines; the limit is \
                         {MAX_REACTION_TEXTS}, so the scene rejoins quickly"
                    ),
                );
            }
            let mut after = *state;
            for (step, &l) in option.steps.iter().zip(&option_lines.step_lines) {
                self.step(step, l, &mut after);
            }
            match first {
                None => first = Some(after),
                Some(f) if f.screen != after.screen => self.err(
                    at,
                    format!(
                        "after this reaction {}; after the first option's {}; every option \
                         must leave the same characters on screen",
                        describe(after.screen),
                        describe(f.screen)
                    ),
                ),
                Some(f) if f.caption != after.caption => self.err(
                    at,
                    "this reaction leaves a different caption from the first option's; every \
                     option must leave the same caption"
                        .to_owned(),
                ),
                Some(_) => {}
            }
        }
        if let Some(f) = first {
            *state = f;
        }
    }

    /// Characters `text` can take once every token is filled in, at most:
    /// lead tokens at their longest ([`lead::longest`]) and every name token
    /// as the longest name in the table, so a rename can't push the line
    /// over its limit. Unknown tokens count as written.
    fn len(&self, text: &str) -> usize {
        lead::split_tokens(text)
            .map(|part| match part {
                Part::Text(t) | Part::Unclosed(t) => t.chars().count(),
                Part::Token(t) => match (name_token(t), self.names) {
                    (Some(_), Some(names)) => names.longest(),
                    _ => lead::longest(t).unwrap_or(t.chars().count() + 2),
                },
            })
            .sum()
    }

    /// Reports a display name from the names table written out in `text`
    /// instead of its token, which a rename would leave behind.
    fn literal_names(&mut self, line: u32, text: &str) {
        let Some(names) = self.names else {
            return;
        };
        let Some((id, form)) = names.literal_in(text) else {
            return;
        };
        let message = if id == LEAD_ID {
            format!(
                "\"{form}\" is the lead's default name; write {{lead}} for the name the player chose"
            )
        } else {
            let name = names.get(id).unwrap_or(form);
            format!(
                "\"{form}\" is written out; write {{n:{id}}} (\"{name}\") so a rename reaches this line"
            )
        };
        self.err(line, message);
    }

    /// Checks name token `{token}`: its id must be in the names table, and
    /// not the lead's (the player names the lead).
    fn name_token(&mut self, line: u32, token: &str) {
        let id = name_token(token).map_or("", |t| t.id);
        if id == LEAD_ID {
            self.err(
                line,
                format!(
                    "{{{token}}} is only the lead's default name; write {{lead}} for the name the player chose"
                ),
            );
        } else if !is_name_id(id) || self.names.is_some_and(|n| n.get(id).is_none()) {
            self.err(
                line,
                format!(
                    "unknown name id \"{id}\" in {{{token}}}; name ids are listed in assets/data/names.ron"
                ),
            );
        }
    }

    /// Reports every `{...}` in `text` that isn't a lead token or a name
    /// token with a known id.
    fn tokens(&mut self, line: u32, text: &str) {
        for part in lead::split_tokens(text) {
            match part {
                Part::Token(t) if name_token(t).is_some() => self.name_token(line, t),
                Part::Token(t) if !lead::is_token(t) => self.err(
                    line,
                    format!(
                        "unknown token \"{{{t}}}\"; use {{lead}}, {{they}}, {{them}}, {{their}}, \
                         {{theirs}}, {{themself}} (capitalised: {{They}}...) or a name, {{n:<id>}}"
                    ),
                ),
                Part::Unclosed(_) => {
                    self.err(line, "\"{\" has no closing \"}\"".to_owned());
                }
                Part::Text(_) | Part::Token(_) => {}
            }
        }
    }

    /// The portraits that `character` may show: the lead has one per
    /// gender, anyone else at most their own.
    fn portraits_of(&self, character: &CharacterId) -> Vec<&'a Portrait> {
        let Some(portraits) = self.portraits else {
            return Vec::new();
        };
        if character.0 == LEAD_ID {
            [PORTRAIT_MALE, PORTRAIT_FEMALE]
                .iter()
                .filter_map(|id| portraits.get(*id))
                .collect()
        } else {
            portraits.get(&character.0).into_iter().collect()
        }
    }

    /// What is wrong with `character` showing `expression`, if anything: it
    /// must be one of the character's portraits' expressions when they
    /// have any (the lead: both portraits'), else one of the
    /// [`STANDARD_EXPRESSIONS`].
    fn expression_problem(&self, character: &CharacterId, expression: &str) -> Option<String> {
        let portraits = self.portraits_of(character);
        if let Some(portrait) = portraits
            .iter()
            .find(|p| p.expression(expression).is_none())
        {
            let names: Vec<&str> = portrait
                .expressions
                .iter()
                .map(|e| e.name.as_str())
                .collect();
            return Some(format!(
                "\"{}\"'s portrait has no expression \"{expression}\"; use one of {}",
                portrait.character,
                names.join(", ")
            ));
        }
        (portraits.is_empty() && !STANDARD_EXPRESSIONS.contains(&expression)).then(|| {
            format!(
                "unknown expression \"{expression}\"; use one of {}",
                STANDARD_EXPRESSIONS.join(", ")
            )
        })
    }
}

/// What is wrong with `@music <cue>`, if anything: `cue` must be a music
/// cue of the audio manifest (not a sound, nor a music pool).
fn music_problem(audio: &AudioManifest, cue: &str) -> Option<String> {
    if audio.music.contains_key(cue) {
        return None;
    }
    let what = if audio.sounds.contains_key(cue) {
        "a sound, not a music cue"
    } else if audio.pools.contains_key(cue) {
        "a music pool, not a music cue"
    } else {
        "not a music cue"
    };
    Some(format!(
        "\"{cue}\" is {what}; music cues are listed in assets/audio/audio.ron"
    ))
}

/// Who is on `screen`, in words: `test_lord is on the left and nobody on
/// the right`.
fn describe(screen: Screen) -> String {
    fn name(c: Option<&CharacterId>) -> &str {
        c.map_or("nobody", |c| c.0.as_str())
    }
    format!(
        "{} is on the left and {} on the right",
        name(screen[0]),
        name(screen[1])
    )
}

/// One error for each scene whose id an earlier scene (in any file)
/// already has.
pub fn check_duplicates(scenes: &[ParsedScene]) -> Vec<ContentError> {
    let mut first: BTreeMap<&str, &ParsedScene> = BTreeMap::new();
    let mut errors = Vec::new();
    for s in scenes {
        if let Some(f) = first.get(s.scene.id.as_str()) {
            errors.push(
                ContentError::new(
                    &s.file,
                    format!(
                        "duplicate scene id \"{}\"; first used at {}:{}",
                        s.scene.id, f.file, f.line
                    ),
                )
                .at(s.line, None),
            );
        } else {
            first.insert(&s.scene.id, s);
        }
    }
    errors
}
