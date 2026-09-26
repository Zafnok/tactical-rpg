//! Data schemas, parsers and validation for game content. See ADR-0004 and
//! ADR-0005. All content comes from the embedded asset bundle ([`bundle`]);
//! [`load_embedded`] loads and validates everything, reporting every error.

pub mod bundle;
pub mod error;
pub mod font;
pub mod keymap;
pub mod map;
pub mod palette;
pub mod ron_loader;
pub mod terrain;

use std::collections::BTreeMap;

pub use error::{ContentError, ContentErrors};
pub use font::FontAtlasDef;
pub use keymap::{Action, Bindings, Chord, Key, KeymapDef, Layout, RepeatDef};
pub use map::{MapDef, MapLegend};
pub use palette::PaletteDef;
pub use terrain::{TerrainDef, TerrainDisplay, TerrainDisplayTable};

/// All validated game content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// Named colours (ADR-0012).
    pub palette: PaletteDef,
    /// Key bindings for every layout, and repeat timings (ADR-0015).
    pub keymap: KeymapDef,
    /// Font atlas layout; the image is `bundle::bytes(font::ATLAS_PNG_PATH)`.
    pub font: FontAtlasDef,
    /// Terrain rules and looks.
    pub terrain: TerrainDef,
    /// Battle maps by id (file stem).
    pub maps: BTreeMap<String, MapDef>,
}

/// Loads and validates every content type from the embedded bundle. Runs all
/// loaders and returns every error found, not just the first. Checks that
/// depend on another file (terrain colours, map legends) are skipped when that
/// file failed, so one broken file doesn't flood the report.
pub fn load_embedded() -> Result<Content, ContentErrors> {
    let palette = PaletteDef::load();
    let terrain = TerrainDef::load(palette.as_ref().ok());
    let maps = match &terrain {
        Ok(t) => map::load_all(&t.display),
        Err(_) => Ok(BTreeMap::new()),
    };
    assemble(
        palette,
        KeymapDef::load(),
        FontAtlasDef::load(),
        terrain,
        maps,
    )
}

/// Takes a loader's value, or moves its errors into `errors` and returns a
/// default.
fn take<T: Default>(result: Result<T, Vec<ContentError>>, errors: &mut Vec<ContentError>) -> T {
    result.unwrap_or_else(|e| {
        errors.extend(e);
        T::default()
    })
}

/// Combines each loader's result into [`Content`], collecting the errors of
/// every loader that failed.
fn assemble(
    palette: Result<PaletteDef, Vec<ContentError>>,
    keymap: Result<KeymapDef, Vec<ContentError>>,
    font: Result<FontAtlasDef, Vec<ContentError>>,
    terrain: Result<TerrainDef, Vec<ContentError>>,
    maps: Result<BTreeMap<String, MapDef>, Vec<ContentError>>,
) -> Result<Content, ContentErrors> {
    let mut errors = Vec::new();
    let content = Content {
        palette: take(palette, &mut errors),
        keymap: take(keymap, &mut errors),
        font: take(font, &mut errors),
        terrain: take(terrain, &mut errors),
        maps: take(maps, &mut errors),
    };
    if errors.is_empty() {
        Ok(content)
    } else {
        Err(ContentErrors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_terrain() -> Result<TerrainDef, Vec<ContentError>> {
        TerrainDef::load(PaletteDef::load().ok().as_ref())
    }

    fn ok_maps() -> Result<BTreeMap<String, MapDef>, Vec<ContentError>> {
        map::load_all(&ok_terrain().unwrap_or_default().display)
    }

    #[test]
    fn assemble_ok_keeps_content() {
        let content = assemble(
            PaletteDef::load(),
            KeymapDef::load(),
            FontAtlasDef::load(),
            ok_terrain(),
            ok_maps(),
        )
        .ok();
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
        assert_eq!(
            content.as_ref().map(|c| &c.terrain),
            ok_terrain().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.maps), ok_maps().ok().as_ref());
        assert!(content.is_some_and(|c| c.maps.contains_key("test_small")));
    }

    #[test]
    fn assemble_reports_loader_errors() {
        let e = |f: &str| vec![ContentError::new(f, "bad")];
        assert_eq!(
            assemble(
                Err(e("p")),
                Err(e("k")),
                Err(e("f")),
                Err(e("t")),
                Err(e("m"))
            ),
            Err(ContentErrors(
                ["p", "k", "f", "t", "m"]
                    .iter()
                    .flat_map(|f| e(f))
                    .collect()
            ))
        );
        let only = |i: usize| {
            assemble(
                if i == 0 {
                    Err(e("p"))
                } else {
                    PaletteDef::load()
                },
                if i == 1 {
                    Err(e("k"))
                } else {
                    KeymapDef::load()
                },
                if i == 2 {
                    Err(e("f"))
                } else {
                    FontAtlasDef::load()
                },
                if i == 3 { Err(e("t")) } else { ok_terrain() },
                if i == 4 { Err(e("m")) } else { ok_maps() },
            )
        };
        for (i, f) in ["p", "k", "f", "t", "m"].iter().enumerate() {
            assert_eq!(only(i), Err(ContentErrors(e(f))));
        }
    }

    #[test]
    fn embedded_content_loads() {
        let content = load_embedded();
        assert!(content.is_ok(), "{content:?}");
        let content = content.ok();
        assert_eq!(
            content.as_ref().map(|c| &c.terrain),
            ok_terrain().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.maps), ok_maps().ok().as_ref());
    }
}
