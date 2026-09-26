//! Data schemas, parsers and validation for game content. See ADR-0004 and
//! ADR-0005. All content comes from the embedded asset bundle ([`bundle`]);
//! [`load_embedded`] loads and validates everything, reporting every error.

pub mod bundle;
pub mod error;
pub mod font;
pub mod keymap;
pub mod palette;
pub mod ron_loader;

pub use error::{ContentError, ContentErrors};
pub use font::FontAtlasDef;
pub use keymap::{Action, Bindings, Chord, Key, KeymapDef, Layout, RepeatDef};
pub use palette::PaletteDef;

/// All validated game content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// Named colours (ADR-0012).
    pub palette: PaletteDef,
    /// Key bindings for every layout, and repeat timings (ADR-0015).
    pub keymap: KeymapDef,
    /// Font atlas layout; the image is `bundle::bytes(font::ATLAS_PNG_PATH)`.
    pub font: FontAtlasDef,
}

/// Loads and validates every content type from the embedded bundle. Runs all
/// loaders and returns every error found, not just the first.
pub fn load_embedded() -> Result<Content, ContentErrors> {
    assemble(PaletteDef::load(), KeymapDef::load(), FontAtlasDef::load())
}

/// Combines each loader's result into [`Content`], collecting the errors of
/// every loader that failed.
fn assemble(
    palette: Result<PaletteDef, Vec<ContentError>>,
    keymap: Result<KeymapDef, Vec<ContentError>>,
    font: Result<FontAtlasDef, Vec<ContentError>>,
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
    let font = font.unwrap_or_else(|e| {
        errors.extend(e);
        FontAtlasDef::default()
    });
    if errors.is_empty() {
        Ok(Content {
            palette,
            keymap,
            font,
        })
    } else {
        Err(ContentErrors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_ok_keeps_content() {
        let content = assemble(PaletteDef::load(), KeymapDef::load(), FontAtlasDef::load()).ok();
        assert_eq!(
            content.as_ref().map(|c| &c.palette),
            PaletteDef::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.keymap),
            KeymapDef::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.font),
            FontAtlasDef::load().ok().as_ref()
        );
    }

    #[test]
    fn assemble_reports_loader_errors() {
        let errs = vec![ContentError::new("a", "one"), ContentError::new("b", "two")];
        let keymap_errs = vec![ContentError::new("k", "three")];
        let font_errs = vec![ContentError::new("f", "four")];
        let mut all = errs.clone();
        all.extend(keymap_errs.clone());
        all.extend(font_errs.clone());
        assert_eq!(
            assemble(Err(errs), Err(keymap_errs), Err(font_errs)),
            Err(ContentErrors(all))
        );
        let keymap_only = vec![ContentError::new("k", "only")];
        assert_eq!(
            assemble(
                PaletteDef::load(),
                Err(keymap_only.clone()),
                FontAtlasDef::load()
            ),
            Err(ContentErrors(keymap_only))
        );
        let font_only = vec![ContentError::new("f", "only")];
        assert_eq!(
            assemble(
                PaletteDef::load(),
                KeymapDef::load(),
                Err(font_only.clone())
            ),
            Err(ContentErrors(font_only))
        );
    }
}
