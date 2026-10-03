//! Tilesets (ADR-0038): what a sprite map skin paints the battle map with.
//! `assets/tilesets/<id>.ron` names one image and, in it, a tile for every
//! terrain and a picture for units by character and by class (format in
//! `assets/tilesets/README.md`).
//!
//! Only looks hang off the game's ids here: nothing in a tileset changes a
//! rule.

use std::collections::BTreeMap;

use serde::Deserialize;
use trpg_core::{CharacterId, ClassId, ClassTable, TerrainId};

use crate::bundle;
use crate::character::CharacterTable;
use crate::error::ContentError;
use crate::image::{ImageId, ImageInfo, ImageTable};
use crate::ron_loader::parse_ron;
use crate::terrain::TerrainDisplayTable;

/// The directory of tileset files inside the bundle.
pub const TILESETS_DIR: &str = "tilesets";

/// The extension of a tileset file.
const TILESET_EXTENSION: &str = ".ron";

/// The smallest and largest side of a map tile or a unit picture, in
/// pixels.
pub const SIDE_PX: std::ops::RangeInclusive<u32> = 8..=64;

/// A rectangle of an image, in image pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageRect {
    /// Left edge.
    pub x: u32,
    /// Top edge.
    pub y: u32,
    /// Width.
    pub w: u32,
    /// Height.
    pub h: u32,
}

/// A picture: a rectangle of an image. A unit's picture need not come from
/// its tileset's image, nor be as wide as it is tall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Picture {
    /// The image.
    pub image: ImageId,
    /// Where in it.
    pub rect: ImageRect,
}

/// A validated tileset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tileset {
    /// Its id: the file's stem.
    pub id: String,
    /// The image its tiles are in.
    pub image: ImageId,
    /// A map tile's size, across × down, in pixels: in the image, and on
    /// screen.
    pub tile_px: (u32, u32),
    /// Each terrain's tile, in [`image`](Self::image). Every terrain has
    /// one.
    pub terrain: BTreeMap<TerrainId, ImageRect>,
    /// Pictures of named characters.
    pub characters: BTreeMap<CharacterId, Picture>,
    /// Pictures of classes.
    pub classes: BTreeMap<ClassId, Picture>,
    /// The picture of any unit with neither.
    pub fallback: Picture,
}

impl Tileset {
    /// The tile of terrain `id`, if the tileset has one.
    pub fn tile(&self, id: TerrainId) -> Option<ImageRect> {
        self.terrain.get(&id).copied()
    }

    /// The picture of a unit of `class`, the named `character` if any:
    /// the character's, else the class's, else the fallback.
    pub fn unit_picture(&self, character: Option<&CharacterId>, class: &ClassId) -> Picture {
        character
            .and_then(|c| self.characters.get(c))
            .or_else(|| self.classes.get(class))
            .copied()
            .unwrap_or(self.fallback)
    }
}

/// A tileset file as written.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TilesetFile {
    id: String,
    image: String,
    tile_px: (u32, u32),
    terrain: BTreeMap<String, (u32, u32)>,
    unit_px: (u32, u32),
    units: UnitsFile,
    units_origin_px: (u32, u32),
}

/// A tileset file's `units` table.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitsFile {
    characters: BTreeMap<String, (u32, u32)>,
    classes: BTreeMap<String, (u32, u32)>,
    fallback: (u32, u32),
}

/// What a tileset is checked against.
#[derive(Debug, Clone, Copy)]
pub struct TilesetRefs<'a> {
    /// The bundle's images.
    pub images: &'a ImageTable,
    /// Every terrain.
    pub terrain: &'a TerrainDisplayTable,
    /// Every class.
    pub classes: &'a ClassTable,
    /// Every named character.
    pub characters: &'a CharacterTable,
}

/// Loads every tileset in the bundle by id, reporting every problem in
/// every file.
pub fn load_all(refs: &TilesetRefs) -> Result<BTreeMap<String, Tileset>, Vec<ContentError>> {
    let mut tilesets = BTreeMap::new();
    let mut errors = Vec::new();
    for path in bundle::files_in(TILESETS_DIR) {
        let Some(stem) = path
            .strip_prefix(TILESETS_DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(TILESET_EXTENSION))
        else {
            continue;
        };
        let file = bundle::display_path(path);
        let result = bundle::file(path)
            .ok_or_else(|| vec![ContentError::new(&file, "file is not valid UTF-8")])
            .and_then(|source| parse_tileset(&file, stem, source, refs));
        match result {
            Ok(tileset) => {
                tilesets.insert(stem.to_owned(), tileset);
            }
            Err(e) => errors.extend(e),
        }
    }
    if errors.is_empty() {
        Ok(tilesets)
    } else {
        Err(errors)
    }
}

/// Parses tileset `source` (errors attributed to `file`, whose stem is
/// `stem`) and checks it against `refs`, reporting every problem.
pub fn parse_tileset(
    file: &str,
    stem: &str,
    source: &str,
    refs: &TilesetRefs,
) -> Result<Tileset, Vec<ContentError>> {
    let def: TilesetFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut problems = own_problems(&def, stem);
    let image = refs.images.id(&def.image);
    let info = image.and_then(|id| refs.images.info(id));
    if image.is_none() {
        problems.push(format!(
            "image \"{}\" is not a PNG in the asset bundle",
            def.image
        ));
    }
    let mut check = |what: String, rect: ImageRect| {
        if let Some(info) = info
            && !inside(rect, info)
        {
            let ImageRect { x, y, w, h } = rect;
            let (iw, ih) = (info.width, info.height);
            problems.push(format!(
                "{what}: its {w}×{h} px at ({x}, {y}) lies outside the {iw}×{ih} px image"
            ));
        }
    };
    let mut terrain = BTreeMap::new();
    let mut unknown = Vec::new();
    for (name, &at) in &def.terrain {
        let rect = grid_rect(at, def.tile_px, (0, 0));
        check(format!("terrain \"{name}\""), rect);
        match refs.terrain.id_of(name) {
            Some(id) => {
                terrain.insert(id, rect);
            }
            None => unknown.push(format!("terrain \"{name}\" is not in terrain.ron")),
        }
    }
    for (name, &at) in &def.units.classes {
        check(format!("class \"{name}\""), def.unit_rect(at));
        if refs.classes.get(&ClassId(name.clone())).is_none() {
            unknown.push(format!("class \"{name}\" is not in classes.ron"));
        }
    }
    for (name, &at) in &def.units.characters {
        check(format!("character \"{name}\""), def.unit_rect(at));
        let id = CharacterId(name.clone());
        if !refs.characters.characters.contains_key(&id) {
            unknown.push(format!("character \"{name}\" is not in characters.ron"));
        }
    }
    check("fallback".to_owned(), def.unit_rect(def.units.fallback));
    problems.extend(unknown);
    for t in &refs.terrain.terrains {
        if !def.terrain.contains_key(&t.id) {
            problems.push(format!("terrain \"{}\" has no tile", t.id));
        }
    }
    match image {
        Some(image) if problems.is_empty() => Ok(def.into_tileset(image, terrain)),
        _ => Err(problems
            .into_iter()
            .map(|m| ContentError::new(file, m))
            .collect()),
    }
}

/// The problems of a tileset file `stem` on its own: its id and sizes.
fn own_problems(def: &TilesetFile, stem: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if def.id != stem {
        problems.push(format!(
            "id \"{}\" must be the file's name, \"{stem}\"",
            def.id
        ));
    }
    for (name, (w, h)) in [("tile_px", def.tile_px), ("unit_px", def.unit_px)] {
        if !SIDE_PX.contains(&w) || !SIDE_PX.contains(&h) {
            let (lo, hi) = (SIDE_PX.start(), SIDE_PX.end());
            problems.push(format!(
                "{name} is ({w}, {h}); each side must be {lo} to {hi} px"
            ));
        }
    }
    problems
}

impl TilesetFile {
    /// The rectangle of unit picture `at`.
    fn unit_rect(&self, at: (u32, u32)) -> ImageRect {
        grid_rect(at, self.unit_px, self.units_origin_px)
    }

    /// The checked file as a [`Tileset`] of `image`, with its `terrain`
    /// tiles resolved.
    fn into_tileset(self, image: ImageId, terrain: BTreeMap<TerrainId, ImageRect>) -> Tileset {
        let picture = |at| Picture {
            image,
            rect: self.unit_rect(at),
        };
        let characters = self.units.characters.iter();
        let classes = self.units.classes.iter();
        Tileset {
            characters: characters
                .map(|(name, &at)| (CharacterId(name.clone()), picture(at)))
                .collect(),
            classes: classes
                .map(|(name, &at)| (ClassId(name.clone()), picture(at)))
                .collect(),
            fallback: picture(self.units.fallback),
            id: self.id,
            image,
            tile_px: self.tile_px,
            terrain,
        }
    }
}

/// The rectangle of cell `(column, row)` of a grid of `size` cells that
/// starts at pixel `origin`. Saturates rather than overflow (the result
/// then lies outside any image).
fn grid_rect((column, row): (u32, u32), (w, h): (u32, u32), (ox, oy): (u32, u32)) -> ImageRect {
    ImageRect {
        x: ox.saturating_add(column.saturating_mul(w)),
        y: oy.saturating_add(row.saturating_mul(h)),
        w,
        h,
    }
}

/// Whether `rect` lies inside an image of size `info`.
fn inside(rect: ImageRect, info: ImageInfo) -> bool {
    let right = u64::from(rect.x) + u64::from(rect.w);
    let bottom = u64::from(rect.y) + u64::from(rect.h);
    right <= u64::from(info.width) && bottom <= u64::from(info.height)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::image::png_header;

    /// The game's terrain, classes and characters, and one 64 × 64 image,
    /// `tilesets/t.png`.
    struct Fixture {
        content: crate::Content,
        images: ImageTable,
    }

    impl Fixture {
        fn new() -> Self {
            let header = png_header(64, 64);
            Self {
                content: crate::load_embedded().unwrap(),
                images: ImageTable::from_files([("tilesets/t.png", header.as_slice())]).unwrap(),
            }
        }

        fn refs(&self) -> TilesetRefs<'_> {
            TilesetRefs {
                images: &self.images,
                terrain: &self.content.terrain.display,
                classes: &self.content.classes,
                characters: &self.content.characters,
            }
        }

        /// Every terrain on tile (0, 0) of a 16 px grid, then `extra`.
        fn terrain(&self, extra: &str) -> String {
            let mut out = String::new();
            for t in &self.content.terrain.display.terrains {
                let _ = write!(out, "\"{}\": (0, 0), ", t.id);
            }
            out.push_str(extra);
            out
        }

        fn parse(&self, source: &str) -> Result<Tileset, Vec<ContentError>> {
            parse_tileset("assets/tilesets/t.ron", "t", source, &self.refs())
        }

        /// The messages of the errors `source` gives.
        fn errors(&self, source: &str) -> Vec<String> {
            let errors = self.parse(source).unwrap_err();
            for e in &errors {
                assert_eq!(e.file, "assets/tilesets/t.ron");
            }
            errors.into_iter().map(|e| e.message).collect()
        }
    }

    /// A tileset file with `fields` in place of the defaults' (a field
    /// written in `fields` is left out of the defaults).
    fn file(f: &Fixture, fields: &[(&str, &str)]) -> String {
        let terrain = format!("{{ {} }}", f.terrain(""));
        let defaults = [
            ("id", "\"t\"".to_owned()),
            ("image", "\"tilesets/t.png\"".to_owned()),
            ("tile_px", "(16, 16)".to_owned()),
            ("terrain", terrain),
            ("unit_px", "(16, 24)".to_owned()),
            (
                "units",
                "(characters: { \"test_lord\": (1, 0) }, classes: { \"brigand\": (0, 0) }, \
                 fallback: (2, 0))"
                    .to_owned(),
            ),
            ("units_origin_px", "(0, 16)".to_owned()),
        ];
        let body: Vec<String> = defaults
            .iter()
            .map(|(name, value)| {
                let value = fields
                    .iter()
                    .find(|(n, _)| n == name)
                    .map_or(value.as_str(), |(_, v)| v);
                format!("{name}: {value}")
            })
            .collect();
        format!("({})", body.join(",\n"))
    }

    #[test]
    fn a_tileset_resolves_its_ids_and_rectangles() {
        let f = Fixture::new();
        let t = f.parse(&file(&f, &[])).unwrap();
        let image = f.images.id("tilesets/t.png").unwrap();
        let display = &f.content.terrain.display;
        assert_eq!((t.id.as_str(), t.image, t.tile_px), ("t", image, (16, 16)));
        assert_eq!(t.terrain.len(), display.terrains.len());
        let plain = display.id_of("plain").unwrap();
        let tile = ImageRect {
            x: 0,
            y: 0,
            w: 16,
            h: 16,
        };
        assert_eq!(t.tile(plain), Some(tile));
        assert_eq!(t.tile(TerrainId(999)), None);
        // Unit pictures: 16 × 24 cells from (0, 16).
        let at = |x| Picture {
            image,
            rect: ImageRect {
                x,
                y: 16,
                w: 16,
                h: 24,
            },
        };
        let lord = CharacterId("test_lord".into());
        let brigand = ClassId("brigand".into());
        let mage = ClassId("mage".into());
        assert_eq!(t.unit_picture(Some(&lord), &brigand), at(16));
        assert_eq!(t.unit_picture(None, &brigand), at(0));
        let nobody = CharacterId("nobody".into());
        assert_eq!(t.unit_picture(Some(&nobody), &brigand), at(0));
        assert_eq!(t.unit_picture(None, &mage), at(32));
        assert_eq!(t.fallback, at(32));
    }

    #[test]
    fn grids_start_at_their_origin_and_saturate() {
        let r = grid_rect((2, 3), (24, 20), (5, 96));
        assert_eq!(
            r,
            ImageRect {
                x: 53,
                y: 156,
                w: 24,
                h: 20
            }
        );
        let huge = grid_rect((u32::MAX, 1), (24, 24), (1, 0));
        assert_eq!((huge.x, huge.y), (u32::MAX, 24));
        let info = |width, height| ImageInfo { width, height };
        let rect = |x, y, w, h| ImageRect { x, y, w, h };
        assert!(inside(rect(24, 24, 24, 24), info(48, 48)));
        assert!(!inside(rect(25, 24, 24, 24), info(48, 48)));
        assert!(!inside(rect(24, 25, 24, 24), info(48, 48)));
        assert!(!inside(huge, info(48, 48)));
    }

    #[test]
    fn a_bad_file_is_one_error_with_its_position() {
        let f = Fixture::new();
        let errors = f.parse("(id: \"t\", bogus: 1)").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, "assets/tilesets/t.ron");
        assert!(errors[0].line.is_some());
    }

    #[test]
    fn the_id_is_the_file_name() {
        let f = Fixture::new();
        assert_eq!(
            f.errors(&file(&f, &[("id", "\"other\"")])),
            ["id \"other\" must be the file's name, \"t\""]
        );
    }

    #[test]
    fn tile_and_unit_sides_are_8_to_64_px() {
        let f = Fixture::new();
        for ok in ["(8, 64)", "(64, 8)"] {
            let source = file(&f, &[("tile_px", ok), ("units_origin_px", "(0, 0)")]);
            assert!(f.parse(&source).is_ok(), "{ok}");
        }
        assert_eq!(
            f.errors(&file(&f, &[("tile_px", "(7, 16)")])),
            ["tile_px is (7, 16); each side must be 8 to 64 px"]
        );
        assert_eq!(
            f.errors(&file(&f, &[("unit_px", "(16, 7)")])),
            ["unit_px is (16, 7); each side must be 8 to 64 px"]
        );
    }

    #[test]
    fn the_image_must_be_in_the_bundle() {
        let f = Fixture::new();
        // Its rectangles can't be checked, so they aren't.
        assert_eq!(
            f.errors(&file(&f, &[("image", "\"tilesets/none.png\"")])),
            ["image \"tilesets/none.png\" is not a PNG in the asset bundle"]
        );
    }

    #[test]
    fn every_rectangle_lies_inside_the_image() {
        let f = Fixture::new();
        let terrain = f
            .terrain("")
            .replace("\"plain\": (0, 0)", "\"plain\": (4, 0)");
        let terrain = format!("{{ {terrain} }}");
        let units = "(characters: { \"test_lord\": (0, 2) }, classes: { \"brigand\": (4, 0) }, \
                     fallback: (3, 2))";
        assert_eq!(
            f.errors(&file(&f, &[("terrain", &terrain), ("units", units)])),
            [
                "terrain \"plain\": its 16×16 px at (64, 0) lies outside the 64×64 px image",
                "class \"brigand\": its 16×24 px at (64, 16) lies outside the 64×64 px image",
                "character \"test_lord\": its 16×24 px at (0, 64) lies outside the 64×64 px image",
                "fallback: its 16×24 px at (48, 64) lies outside the 64×64 px image",
            ]
        );
    }

    #[test]
    fn every_terrain_has_a_tile_and_every_name_exists() {
        let f = Fixture::new();
        let terrain = format!(
            "{{ {} }}",
            f.terrain("\"lava\": (0, 0)")
                .replace("\"forest\": (0, 0), ", "")
        );
        let units = "(characters: { \"nobody\": (0, 0) }, classes: { \"wizard\": (0, 0) }, \
                     fallback: (0, 0))";
        assert_eq!(
            f.errors(&file(&f, &[("terrain", &terrain), ("units", units)])),
            [
                "terrain \"lava\" is not in terrain.ron",
                "class \"wizard\" is not in classes.ron",
                "character \"nobody\" is not in characters.ron",
                "terrain \"forest\" has no tile",
            ]
        );
    }

    #[test]
    fn the_embedded_test_tileset_loads() {
        let f = Fixture::new();
        let tilesets = &f.content.tilesets;
        assert_eq!(tilesets.keys().collect::<Vec<_>>(), ["test"]);
        let test = &tilesets["test"];
        assert_eq!(test.tile_px, (24, 24));
        assert_eq!(test.image.path(), "tilesets/test.png");
        // Every class has a picture of its own.
        assert_eq!(test.classes.len(), f.content.classes.classes.len());
        assert!(test.characters.is_empty());
    }
}
