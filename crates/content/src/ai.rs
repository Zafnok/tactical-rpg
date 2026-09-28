//! The AI's numbers (`assets/data/ai.ron`, ticket 0501). The rules that use
//! them are in `trpg_core::ai`.

use trpg_core::AiWeights;

use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;

/// Path of the AI file inside the asset bundle.
pub const AI_PATH: &str = "data/ai.ron";

/// Loads the embedded AI file.
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

/// Parses AI `source`, attributing errors to `file`. Every weight must be
/// given, as a whole number from 0 up.
pub fn from_source(file: &str, source: &str) -> Result<AiWeights, Vec<ContentError>> {
    parse_ron(file, source).map_err(|e| vec![e])
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "(damage: 1, kill: 2, lord: 3, risk: 4, terrain: 5)";

    #[test]
    fn valid_source_loads() {
        let expected = AiWeights {
            damage: 1,
            kill: 2,
            lord: 3,
            risk: 4,
            terrain: 5,
        };
        assert_eq!(from_source("ai.ron", VALID), Ok(expected));
    }

    #[test]
    fn bad_sources_are_refused_with_their_position() {
        assert!(from_source("ai.ron", "(").is_err());
        let extra = VALID.replace("damage: 1", "damage: 1, bravery: 9");
        assert!(from_source("ai.ron", &extra).is_err());
        let missing = VALID.replace("lord: 3, ", "");
        assert!(from_source("ai.ron", &missing).is_err());
        let negative = "(
 damage: 1, kill: 2, lord: 3,
 risk: -4, terrain: 5)";
        let errors = from_source("ai.ron", negative).err().unwrap_or_default();
        assert_eq!(errors.first().map(|e| e.line), Some(Some(3)));
    }

    #[test]
    fn embedded_file_holds_the_starting_values() {
        assert_eq!(load(), Ok(AiWeights::STARTING));
    }
}
