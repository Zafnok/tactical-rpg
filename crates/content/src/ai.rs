//! The AI's numbers (`assets/data/ai.ron`, ticket 0501). The rules that use
//! them are in `trpg_core::ai`.

use trpg_core::AiWeights;

use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the AI file inside the asset bundle.
pub const AI_PATH: &str = "data/ai.ron";

/// Loads and validates the embedded AI file.
pub fn load() -> Result<AiWeights, Vec<ContentError>> {
    let display = bundle::display_path(AI_PATH);
    let source = bundle::file(AI_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Parses and validates AI `source`, attributing errors to `file`: both
/// percentages must be at most 100. Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<AiWeights, Vec<ContentError>> {
    let weights: AiWeights = parse_ron(file, source).map_err(|e| vec![e])?;
    let errors: Vec<ContentError> = [
        ("self_heal_below", weights.self_heal_below),
        ("likely_kill", weights.likely_kill),
    ]
    .into_iter()
    .filter(|&(_, percent)| percent > 100)
    .map(|(name, percent)| {
        let e = ContentError::new(file, format!("{name} is {percent}%: at most 100"));
        match line_of(source, &format!("{name}:")) {
            Some(line) => e.at(line, None),
            None => e,
        }
    })
    .collect();
    if errors.is_empty() {
        Ok(weights)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "(damage: 1, kill: 2, lord: 3, risk: 4, terrain: 5, \
                         self_heal_below: 100, likely_kill: 0)";

    #[test]
    fn valid_source_loads() {
        let expected = AiWeights {
            damage: 1,
            kill: 2,
            lord: 3,
            risk: 4,
            terrain: 5,
            self_heal_below: 100,
            likely_kill: 0,
        };
        assert_eq!(from_source("ai.ron", VALID), Ok(expected));
    }

    #[test]
    fn percentages_over_100_are_refused_with_their_line() {
        let src = "(\n damage: 1, kill: 2, lord: 3, risk: 4, terrain: 5,\n \
                   self_heal_below: 101,\n likely_kill: 200,\n)";
        let errors: Vec<String> = from_source("ai.ron", src)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            errors,
            [
                "ai.ron:3: self_heal_below is 101%: at most 100",
                "ai.ron:4: likely_kill is 200%: at most 100",
            ]
        );
        let one_line = "(damage: 1, kill: 2, lord: 3, risk: 4, terrain: 5, \
                        self_heal_below: 101, likely_kill: 0)";
        assert_eq!(from_source("ai.ron", one_line).map_err(|e| e.len()), Err(1));
    }

    #[test]
    fn syntax_errors_and_unknown_fields_are_refused() {
        assert!(from_source("ai.ron", "(").is_err());
        let extra = VALID.replace("damage: 1", "damage: 1, bravery: 9");
        assert!(from_source("ai.ron", &extra).is_err());
        let missing = VALID.replace("lord: 3, ", "");
        assert!(from_source("ai.ron", &missing).is_err());
    }

    #[test]
    fn embedded_file_holds_the_starting_values() {
        assert_eq!(load(), Ok(AiWeights::default()));
    }
}
