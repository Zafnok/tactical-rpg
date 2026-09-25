//! Data schemas, parsers and validation for game content. See ADR-0004 and
//! ADR-0005. All content comes from the embedded asset bundle ([`bundle`]);
//! [`load_embedded`] loads and validates everything, reporting every error.

pub mod bundle;
pub mod error;
pub mod palette;
pub mod ron_loader;

pub use error::{ContentError, ContentErrors};
pub use palette::PaletteDef;

/// All validated game content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// Named colours (ADR-0012).
    pub palette: PaletteDef,
}

/// Loads and validates every content type from the embedded bundle. Runs all
/// loaders and returns every error found, not just the first.
pub fn load_embedded() -> Result<Content, ContentErrors> {
    assemble(PaletteDef::load())
}

/// Combines each loader's result into [`Content`], collecting the errors of
/// every loader that failed.
fn assemble(palette: Result<PaletteDef, Vec<ContentError>>) -> Result<Content, ContentErrors> {
    let mut errors = Vec::new();
    let palette = palette.unwrap_or_else(|e| {
        errors.extend(e);
        PaletteDef::default()
    });
    if errors.is_empty() {
        Ok(Content { palette })
    } else {
        Err(ContentErrors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_ok_keeps_content() {
        let content = assemble(PaletteDef::load()).map_err(|e| e.to_string());
        assert_eq!(
            content.map(|c| c.palette),
            PaletteDef::load().map_err(|_| String::new())
        );
    }

    #[test]
    fn assemble_reports_loader_errors() {
        let errs = vec![ContentError::new("a", "one"), ContentError::new("b", "two")];
        assert_eq!(assemble(Err(errs.clone())), Err(ContentErrors(errs)));
    }
}
