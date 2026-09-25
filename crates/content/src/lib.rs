//! Data schemas, parsers and validation for game content. See ADR-0004 and
//! ADR-0005. All content comes from the embedded asset bundle ([`bundle`]);
//! [`load_embedded`] loads and validates everything, reporting every error.

pub mod bundle;
pub mod error;
pub mod keymap;
pub mod palette;
pub mod ron_loader;

pub use error::{ContentError, ContentErrors};
pub use keymap::{Action, Chord, Key, KeymapDef, RepeatDef};
pub use palette::PaletteDef;

/// All validated game content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// Named colours (ADR-0012).
    pub palette: PaletteDef,
    /// Default key bindings and repeat timings (ADR-0006).
    pub keymap: KeymapDef,
}

/// Loads and validates every content type from the embedded bundle. Runs all
/// loaders and returns every error found, not just the first.
pub fn load_embedded() -> Result<Content, ContentErrors> {
    assemble(PaletteDef::load(), KeymapDef::load())
}

/// Combines each loader's result into [`Content`], collecting the errors of
/// every loader that failed.
fn assemble(
    palette: Result<PaletteDef, Vec<ContentError>>,
    keymap: Result<KeymapDef, Vec<ContentError>>,
) -> Result<Content, ContentErrors> {
    let mut errors = Vec::new();
    let palette = palette.unwrap_or_else(|e| {
        errors.extend(e);
        PaletteDef::default()
    });
    let keymap = keymap.unwrap_or_else(|e| {
        errors.extend(e);
        KeymapDef::default()
    });
    if errors.is_empty() {
        Ok(Content { palette, keymap })
    } else {
        Err(ContentErrors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_ok_keeps_content() {
        let content = assemble(PaletteDef::load(), KeymapDef::load()).ok();
        assert_eq!(
            content.as_ref().map(|c| &c.palette),
            PaletteDef::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.keymap),
            KeymapDef::load().ok().as_ref()
        );
    }

    #[test]
    fn assemble_reports_loader_errors() {
        let errs = vec![ContentError::new("a", "one"), ContentError::new("b", "two")];
        let keymap_errs = vec![ContentError::new("k", "three")];
        let mut all = errs.clone();
        all.extend(keymap_errs.clone());
        assert_eq!(
            assemble(Err(errs), Err(keymap_errs)),
            Err(ContentErrors(all))
        );
        let keymap_only = vec![ContentError::new("k", "only")];
        assert_eq!(
            assemble(PaletteDef::load(), Err(keymap_only.clone())),
            Err(ContentErrors(keymap_only))
        );
    }
}
