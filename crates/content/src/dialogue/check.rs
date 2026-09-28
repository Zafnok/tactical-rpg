//! Checks on parsed scenes: who is on screen, known characters and
//! expressions, text length, and scene ids unique across files.

use std::collections::BTreeMap;

use trpg_core::CharacterId;

use super::parse::ParsedScene;
use super::{MAX_TEXT_LEN, STANDARD_EXPRESSIONS, Side, Step};
use crate::character::CharacterTable;
use crate::error::ContentError;

/// Checks one scene, replaying it to know who is on screen at each line.
/// Character ids are checked against `characters` when given.
pub fn check_scene(parsed: &ParsedScene, characters: Option<&CharacterTable>) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let mut err = |line: u32, message: String| {
        errors.push(ContentError::new(&parsed.file, message).at(line, None));
    };
    let known = |id: &CharacterId| characters.is_none_or(|t| t.characters.contains_key(id));
    let mut screen: [Option<&CharacterId>; 2] = [None, None];
    let slot = |side: Side| match side {
        Side::Left => 0,
        Side::Right => 1,
    };
    for (step, &line) in parsed.scene.steps.iter().zip(&parsed.step_lines) {
        if let Some(text) = step.text() {
            let len = text.chars().count();
            if len > MAX_TEXT_LEN {
                err(
                    line,
                    format!("text is {len} characters; the limit is {MAX_TEXT_LEN}"),
                );
            }
        }
        match step {
            Step::Place {
                side,
                character,
                expression,
            } => {
                if !known(character) {
                    err(line, format!("unknown character \"{}\"", character.0));
                }
                if let Some(message) = expression_problem(expression) {
                    err(line, message);
                }
                if screen[slot(side.other())] == Some(character) {
                    err(
                        line,
                        format!(
                            "\"{}\" is already on the {}; a character can't be on both sides",
                            character.0,
                            side.other().name()
                        ),
                    );
                }
                screen[slot(*side)] = Some(character);
            }
            Step::Clear { side } => screen[slot(*side)] = None,
            Step::Say {
                speaker,
                expression,
                ..
            } => {
                if !known(speaker) {
                    err(line, format!("unknown character \"{}\"", speaker.0));
                } else if !screen.contains(&Some(speaker)) {
                    err(
                        line,
                        format!(
                            "\"{}\" speaks but is not on screen; place them with @left or @right first",
                            speaker.0
                        ),
                    );
                }
                if let Some(message) = expression.as_deref().and_then(expression_problem) {
                    err(line, message);
                }
            }
            Step::Caption { .. } | Step::Narrate { .. } => {}
        }
    }
    if parsed.scene.steps.iter().all(|s| s.text().is_none()) {
        err(
            parsed.line,
            format!("scene \"{}\" has no speech or narration", parsed.scene.id),
        );
    }
    errors
}

/// What is wrong with `expression`, if anything. Until portraits exist
/// (ticket 0703) only the [`STANDARD_EXPRESSIONS`] are allowed.
fn expression_problem(expression: &str) -> Option<String> {
    (!STANDARD_EXPRESSIONS.contains(&expression)).then(|| {
        format!(
            "unknown expression \"{expression}\"; use one of {}",
            STANDARD_EXPRESSIONS.join(", ")
        )
    })
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
