//! Data schemas, parsers and validation for game content. See ADR-0004 and
//! ADR-0005. All content comes from the embedded asset bundle ([`bundle`]);
//! [`load_embedded`] loads and validates everything, reporting every error.

pub mod bundle;
pub mod character;
pub mod class;
mod enums;
pub mod error;
pub mod font;
pub mod item;
pub mod keymap;
pub mod map;
pub mod palette;
pub mod ron_loader;
pub mod terrain;

use std::collections::BTreeMap;

use trpg_core::{ClassTable, ItemTable};

pub use character::{CharacterTable, GenericTemplate, character_unit, check_map_labels};
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
    /// The class tree and progression tables.
    pub classes: ClassTable,
    /// Items and item rules.
    pub items: ItemTable,
    /// Named characters and generic unit templates.
    pub characters: CharacterTable,
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
    let classes = class::load(
        terrain
            .as_ref()
            .ok()
            .map(|t| t.rules.movement_types.as_slice()),
    );
    let items = item::load();
    let maps = check_map_features(maps, items.as_ref().ok(), terrain.as_ref().ok());
    let characters = character::load(classes.as_ref().ok(), items.as_ref().ok());
    assemble(
        palette,
        KeymapDef::load(),
        FontAtlasDef::load(),
        terrain,
        maps,
        Loaded {
            classes,
            items,
            characters,
        },
    )
}

/// Adds the map feature checks ([`map::check_features`]) to the maps'
/// result. Skipped when the maps, items or terrain failed to load.
fn check_map_features(
    maps: Result<BTreeMap<String, MapDef>, Vec<ContentError>>,
    items: Option<&ItemTable>,
    terrain: Option<&TerrainDef>,
) -> Result<BTreeMap<String, MapDef>, Vec<ContentError>> {
    match (maps, items, terrain) {
        (Ok(maps), Some(items), Some(terrain)) => {
            let errors = map::check_features(&maps, items, &terrain.rules);
            if errors.is_empty() {
                Ok(maps)
            } else {
                Err(errors)
            }
        }
        (maps, ..) => maps,
    }
}

/// Loader results for the unit data (kept together to keep [`assemble`]'s
/// argument list short).
struct Loaded {
    classes: Result<ClassTable, Vec<ContentError>>,
    items: Result<ItemTable, Vec<ContentError>>,
    characters: Result<CharacterTable, Vec<ContentError>>,
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
    units: Loaded,
) -> Result<Content, ContentErrors> {
    let mut errors = Vec::new();
    let content = Content {
        palette: take(palette, &mut errors),
        keymap: take(keymap, &mut errors),
        font: take(font, &mut errors),
        terrain: take(terrain, &mut errors),
        maps: take(maps, &mut errors),
        classes: take(units.classes, &mut errors),
        items: take(units.items, &mut errors),
        characters: take(units.characters, &mut errors),
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

    fn ok_classes() -> Result<ClassTable, Vec<ContentError>> {
        class::load(Some(&ok_terrain().unwrap_or_default().rules.movement_types))
    }

    fn ok_characters() -> Result<CharacterTable, Vec<ContentError>> {
        character::load(ok_classes().ok().as_ref(), item::load().ok().as_ref())
    }

    fn ok_units() -> Loaded {
        Loaded {
            classes: ok_classes(),
            items: item::load(),
            characters: ok_characters(),
        }
    }

    #[test]
    fn assemble_ok_keeps_content() {
        let content = assemble(
            PaletteDef::load(),
            KeymapDef::load(),
            FontAtlasDef::load(),
            ok_terrain(),
            ok_maps(),
            ok_units(),
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
        assert_eq!(
            content.as_ref().map(|c| &c.classes),
            ok_classes().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.items),
            item::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.characters),
            ok_characters().ok().as_ref()
        );
        assert!(
            content
                .as_ref()
                .is_some_and(|c| c.maps.contains_key("test_small"))
        );
        assert!(
            content.is_some_and(
                |c| !c.classes.classes.is_empty() && !c.characters.characters.is_empty()
            )
        );
    }

    const NAMES: [&str; 8] = ["p", "k", "f", "t", "m", "c", "i", "u"];

    #[test]
    fn assemble_reports_loader_errors() {
        let e = |f: &str| vec![ContentError::new(f, "bad")];
        assert_eq!(
            assemble(
                Err(e("p")),
                Err(e("k")),
                Err(e("f")),
                Err(e("t")),
                Err(e("m")),
                Loaded {
                    classes: Err(e("c")),
                    items: Err(e("i")),
                    characters: Err(e("u")),
                },
            ),
            Err(ContentErrors(NAMES.iter().flat_map(|f| e(f)).collect()))
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
                Loaded {
                    classes: if i == 5 { Err(e("c")) } else { ok_classes() },
                    items: if i == 6 { Err(e("i")) } else { item::load() },
                    characters: if i == 7 { Err(e("u")) } else { ok_characters() },
                },
            )
        };
        for (i, f) in NAMES.iter().enumerate() {
            assert_eq!(only(i), Err(ContentErrors(e(f))));
        }
    }

    #[test]
    fn map_feature_checks_join_the_map_errors() {
        let terrain = ok_terrain().ok();
        let items = item::load().ok();
        let mut maps = ok_maps().unwrap_or_default();
        assert_eq!(
            check_map_features(Ok(maps.clone()), items.as_ref(), terrain.as_ref()),
            Ok(maps.clone())
        );
        if let Some(def) = maps.get_mut("test_small") {
            def.map.features.insert(
                trpg_core::Pos::new(0, 0),
                trpg_core::TileFeature::Chest(trpg_core::Loot::Item(trpg_core::ItemId::new("x"))),
            );
        }
        let errors = check_map_features(Ok(maps.clone()), items.as_ref(), terrain.as_ref());
        assert_eq!(errors.map_err(|e| e.len()), Err(1));
        // Skipped when another file failed.
        assert_eq!(
            check_map_features(Ok(maps.clone()), None, terrain.as_ref()),
            Ok(maps.clone())
        );
        assert_eq!(
            check_map_features(Ok(maps.clone()), items.as_ref(), None),
            Ok(maps)
        );
        let failed = Err(vec![ContentError::new("m", "bad")]);
        assert_eq!(
            check_map_features(failed.clone(), items.as_ref(), terrain.as_ref()),
            failed
        );
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
        assert_eq!(
            content.as_ref().map(|c| &c.classes),
            ok_classes().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.characters),
            ok_characters().ok().as_ref()
        );
    }
}
