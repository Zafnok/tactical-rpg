//! `cargo xtask test-tileset`: writes `assets/tilesets/test.png` and
//! `test.ron`, the public test tileset for the sprite map skin (ticket
//! 0433, ADR-0038).
//!
//! Tiles are 24×24, so nothing can quietly assume 16×16. Each terrain's
//! tile is its `bg` colour with its two glyphs stamped in its `fg` colour,
//! centred; each class's picture is a grey disc with the first two letters
//! of its name in white (the skin shows the unit's side itself). It is
//! generated from our own data and the font atlas, not art, and the same
//! data always gives the same files.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use trpg_content::bundle::{self, display_path};
use trpg_content::font::ATLAS_PNG_PATH;
use trpg_content::{FontAtlasDef, PaletteDef, TerrainDef, class};

use crate::font_atlas::{decode_png, encode_png};

/// The tileset's id.
pub const ID: &str = "test";
/// The image's path in the bundle.
pub const PNG_PATH: &str = "tilesets/test.png";
/// The tileset file's path in the bundle.
pub const RON_PATH: &str = "tilesets/test.ron";
/// A tile's (and a unit picture's) side, in pixels.
pub const TILE: u32 = 24;
/// Tiles (and unit pictures) per row of the image.
pub const COLUMNS: u32 = 8;
/// The disc under a class's letters (RGBA).
pub const DISC: [u8; 4] = [96, 96, 104, 255];
/// The class letters' colour (RGBA).
pub const LETTERS: [u8; 4] = [255, 255, 255, 255];
/// The letters of the picture of a unit with no class picture.
pub const FALLBACK_LETTERS: [char; 2] = ['?', '?'];

/// A terrain's string id, two glyphs, and `fg` and `bg` colours.
pub type TerrainLook = (String, [char; 2], [u8; 3], [u8; 3]);

/// What the tileset is made from.
#[derive(Debug, Clone)]
pub struct Sources {
    /// Each terrain's look, in `terrain.ron`'s order.
    pub terrain: Vec<TerrainLook>,
    /// Each class's id and the two letters on its picture, by id.
    pub classes: Vec<(String, [char; 2])>,
    /// The font atlas's layout.
    pub font: FontAtlasDef,
    /// The atlas image: width, then RGBA8 pixels.
    pub atlas: (u32, Vec<u8>),
}

impl Sources {
    /// The embedded terrain, classes, palette and font atlas.
    pub fn embedded() -> Result<Self, String> {
        let joined = |e: Vec<trpg_content::ContentError>| {
            e.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        };
        let palette = PaletteDef::load().map_err(joined)?;
        let terrain = TerrainDef::load(Some(&palette)).map_err(joined)?;
        let classes = class::load(Some(&terrain.rules.movement_types)).map_err(joined)?;
        let font = FontAtlasDef::load().map_err(joined)?;
        let png = bundle::bytes(ATLAS_PNG_PATH).ok_or("the font atlas image is missing")?;
        let (width, _, rgba) = decode_png(png)?;
        let color = |name: &str| {
            palette
                .get(name)
                .ok_or_else(|| format!("no palette colour {name}"))
        };
        let terrain = terrain
            .display
            .terrains
            .iter()
            .map(|t| Ok((t.id.clone(), t.glyphs, color(&t.fg)?, color(&t.bg)?)))
            .collect::<Result<_, String>>()?;
        let classes = classes
            .classes
            .values()
            .map(|c| (c.id.0.clone(), initials(&c.name)))
            .collect();
        Ok(Self {
            terrain,
            classes,
            font,
            atlas: (width, rgba),
        })
    }
}

/// The first two letters of `name` (a blank for a missing one).
fn initials(name: &str) -> [char; 2] {
    let mut letters = name.chars().filter(|c| !c.is_whitespace());
    [letters.next().unwrap_or(' '), letters.next().unwrap_or(' ')]
}

/// `(column, row)` of picture `index` in a grid [`COLUMNS`] wide.
fn cell(index: usize) -> (u32, u32) {
    let i = u32::try_from(index).unwrap_or(u32::MAX);
    (i % COLUMNS, i / COLUMNS)
}

/// Rows a grid of `count` pictures takes.
fn rows(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX).div_ceil(COLUMNS)
}

/// An RGBA8 image being drawn.
struct Canvas {
    width: u32,
    rgba: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32) -> Self {
        let len = usize::try_from(width * height * 4).unwrap_or(0);
        Self {
            width,
            rgba: vec![0; len],
        }
    }

    fn set(&mut self, x: u32, y: u32, color: [u8; 4]) {
        let at = usize::try_from((y * self.width + x) * 4).unwrap_or(usize::MAX);
        if let Some(px) = self.rgba.get_mut(at..at + 4) {
            px.copy_from_slice(&color);
        }
    }

    fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let at = usize::try_from((y * self.width + x) * 4).unwrap_or(usize::MAX);
        self.rgba
            .get(at..at + 4)
            .map_or([0; 4], |p| [p[0], p[1], p[2], p[3]])
    }

    /// Stamps `glyphs` from the font atlas in `color`, centred in the tile
    /// whose top-left pixel is `(x, y)`: each glyph pixel blends towards
    /// `color` by its coverage. What is under the glyphs (a tile, or a
    /// disc) is solid, and stays so.
    fn stamp(&mut self, src: &Sources, glyphs: [char; 2], color: [u8; 3], (x, y): (u32, u32)) {
        let (cw, ch) = (src.font.cell_w, src.font.cell_h);
        let left = x + (TILE - 2 * cw) / 2;
        let top = y + (TILE - ch) / 2;
        for (i, glyph) in (0..).zip(glyphs) {
            let Some(r) = src.font.glyph_rect(glyph) else {
                continue;
            };
            for gy in 0..r.h {
                for gx in 0..r.w {
                    let at = usize::try_from(((r.y + gy) * src.atlas.0 + r.x + gx) * 4 + 3);
                    let cover = at.ok().and_then(|at| src.atlas.1.get(at)).copied();
                    let cover = u32::from(cover.unwrap_or(0));
                    if cover == 0 {
                        continue;
                    }
                    let (px, py) = (left + i * cw + gx, top + gy);
                    let under = self.get(px, py);
                    let out = [
                        blend(under[0], color[0], cover),
                        blend(under[1], color[1], cover),
                        blend(under[2], color[2], cover),
                        under[3],
                    ];
                    self.set(px, py, out);
                }
            }
        }
    }
}

/// Channel `under` blended towards `over` by `cover` (0 = none of it,
/// 255 = all of it), rounded.
fn blend(under: u8, over: u8, cover: u32) -> u8 {
    let cover = cover.min(255);
    let v = (u32::from(under) * (255 - cover) + u32::from(over) * cover + 127) / 255;
    u8::try_from(v).unwrap_or(u8::MAX)
}

/// Whether pixel `(x, y)` of a picture is on its disc: a circle 22 px
/// across, centred.
fn on_disc(x: u32, y: u32) -> bool {
    let d = |v: u32| i64::from(2 * v + 1) - i64::from(TILE);
    d(x) * d(x) + d(y) * d(y) <= 22 * 22
}

/// The tileset image: width, height and RGBA8 pixels. Terrain tiles in
/// `terrain.ron`'s order, [`COLUMNS`] to a row; then, from the next row,
/// the class pictures by id and the fallback picture.
pub fn image(src: &Sources) -> (u32, u32, Vec<u8>) {
    let terrain_rows = rows(src.terrain.len());
    let unit_rows = rows(src.classes.len() + 1);
    let (width, height) = (COLUMNS * TILE, (terrain_rows + unit_rows) * TILE);
    let mut canvas = Canvas::new(width, height);
    for (i, (_, glyphs, fg, bg)) in src.terrain.iter().enumerate() {
        let (cx, cy) = cell(i);
        let (x, y) = (cx * TILE, cy * TILE);
        for py in y..y + TILE {
            for px in x..x + TILE {
                canvas.set(px, py, [bg[0], bg[1], bg[2], 255]);
            }
        }
        canvas.stamp(src, *glyphs, *fg, (x, y));
    }
    let letters = src.classes.iter().map(|(_, l)| *l);
    for (i, letters) in letters.chain([FALLBACK_LETTERS]).enumerate() {
        let (cx, cy) = cell(i);
        let (x, y) = (cx * TILE, (terrain_rows + cy) * TILE);
        for py in 0..TILE {
            for px in 0..TILE {
                if on_disc(px, py) {
                    canvas.set(x + px, y + py, DISC);
                }
            }
        }
        let white = [LETTERS[0], LETTERS[1], LETTERS[2]];
        canvas.stamp(src, letters, white, (x, y));
    }
    (width, height, canvas.rgba)
}

/// The tileset file for [`image`].
pub fn ron(src: &Sources) -> String {
    let at = |i| {
        let (x, y) = cell(i);
        format!("({x}, {y})")
    };
    let mut out = String::from(
        "// Generated by `cargo xtask test-tileset` from terrain.ron, classes.ron,\n\
         // the palette and the font atlas. Do not edit; see assets/tilesets/README.md.\n\
         (\n",
    );
    // Writing to a `String` can't fail.
    let _ = writeln!(out, "    id: \"{ID}\",\n    image: \"{PNG_PATH}\",");
    let _ = writeln!(out, "    tile_px: ({TILE}, {TILE}),\n    terrain: {{");
    for (i, (id, ..)) in src.terrain.iter().enumerate() {
        let _ = writeln!(out, "        \"{id}\": {},", at(i));
    }
    let _ = writeln!(out, "    }},\n    unit_px: ({TILE}, {TILE}),");
    out.push_str("    units: (\n        characters: {},\n        classes: {\n");
    for (i, (id, _)) in src.classes.iter().enumerate() {
        let _ = writeln!(out, "            \"{id}\": {},", at(i));
    }
    let fallback = at(src.classes.len());
    let _ = writeln!(out, "        }},\n        fallback: {fallback},\n    ),");
    let top = rows(src.terrain.len()) * TILE;
    let _ = writeln!(out, "    units_origin_px: (0, {top}),\n)");
    out
}

/// Runs the command: writes the tileset under repo root `root`.
pub fn run(root: &Path) -> Result<String, String> {
    let src = Sources::embedded()?;
    let (width, height, rgba) = image(&src);
    let png = encode_png(width, height, &rgba)?;
    let png_shown = display_path(PNG_PATH);
    let ron_shown = display_path(RON_PATH);
    for (shown, bytes) in [(&png_shown, png), (&ron_shown, ron(&src).into_bytes())] {
        let path = root.join(shown);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
        }
        fs::write(&path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))?;
    }
    Ok(format!(
        "test-tileset: wrote {png_shown} ({width}×{height}) and {ron_shown}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sources() -> Sources {
        Sources::embedded().unwrap()
    }

    /// Pixel `(x, y)` of an image `width` wide.
    fn px(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let at = usize::try_from((y * width + x) * 4).unwrap();
        [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
    }

    #[test]
    fn pictures_go_eight_to_a_row() {
        assert_eq!(cell(0), (0, 0));
        assert_eq!(cell(7), (7, 0));
        assert_eq!(cell(8), (0, 1));
        assert_eq!(cell(19), (3, 2));
        assert_eq!((rows(0), rows(1), rows(8), rows(9)), (0, 1, 1, 2));
        assert_eq!(initials("Fire Elemental"), ['F', 'i']);
        assert_eq!(initials("I"), ['I', ' ']);
        assert_eq!(initials(" X y"), ['X', 'y']);
    }

    #[test]
    fn a_canvas_keeps_each_pixel_where_it_was_set() {
        let mut canvas = Canvas::new(5, 4);
        assert_eq!(canvas.rgba.len(), 5 * 4 * 4);
        canvas.set(3, 2, [1, 2, 3, 4]);
        canvas.set(4, 3, [5, 6, 7, 8]);
        assert_eq!(canvas.get(3, 2), [1, 2, 3, 4]);
        assert_eq!(canvas.get(4, 3), [5, 6, 7, 8]);
        assert_eq!(canvas.get(0, 0), [0; 4]);
        // Row-major RGBA: pixel (3, 2) is the 14th.
        assert_eq!(canvas.rgba[13 * 4..14 * 4], [1, 2, 3, 4]);
        // Off the canvas: nothing set, nothing read.
        canvas.set(0, 4, [9; 4]);
        assert_eq!(canvas.get(0, 4), [0; 4]);
    }

    #[test]
    fn glyph_pixels_blend_by_their_coverage() {
        assert_eq!(blend(200, 100, 255), 100);
        assert_eq!(blend(200, 100, 0), 200);
        assert_eq!(blend(200, 100, 51), 180);
        assert_eq!(blend(0, 255, 128), 128);
        assert_eq!(blend(255, 0, 128), 127);
        assert_eq!(blend(200, 100, 999), 100);
    }

    #[test]
    fn the_fallback_picture_gets_a_row_of_its_own_when_the_classes_fill_theirs() {
        let mut src = sources();
        src.classes.truncate(8);
        let terrain_rows = rows(src.terrain.len());
        let (w, h, rgba) = image(&src);
        assert_eq!((w, h), (192, (terrain_rows + 2) * TILE));
        // The fallback: first in the second row of pictures.
        let y = (terrain_rows + 1) * TILE;
        assert_eq!(px(&rgba, w, 1, y + 12), DISC);
        assert!(ron(&src).contains("        fallback: (0, 1),\n"));
    }

    #[test]
    fn the_disc_is_22_px_across_and_centred() {
        let row: Vec<bool> = (0..TILE).map(|x| on_disc(x, 12)).collect();
        assert_eq!(row.iter().filter(|&&b| b).count(), 22);
        assert!(!row[0] && row[1] && row[22] && !row[23]);
        assert!(!on_disc(0, 0) && !on_disc(23, 23) && !on_disc(2, 2));
        assert!(on_disc(11, 1) && on_disc(12, 22) && !on_disc(12, 0));
    }

    #[test]
    fn terrain_tiles_are_their_colours_with_their_glyphs_centred() {
        let src = sources();
        let (w, h, rgba) = image(&src);
        let terrain_rows = rows(src.terrain.len());
        let unit_rows = rows(src.classes.len() + 1);
        assert_eq!((w, h), (192, (terrain_rows + unit_rows) * TILE));
        let (_, _, fg, bg) = &src.terrain[0];
        let solid = |c: &[u8; 3]| [c[0], c[1], c[2], 255];
        // The corners and the 4 px margin around the glyphs: background.
        for (x, y) in [(0, 0), (23, 23), (3, 12), (20, 12), (12, 3), (12, 20)] {
            assert_eq!(px(&rgba, w, x, y), solid(bg), "({x}, {y})");
        }
        // Somewhere in the glyphs (cells 4..20 × 4..20): the glyph colour.
        let inked = (4..20)
            .flat_map(|y| (4..20).map(move |x| (x, y)))
            .filter(|&(x, y)| px(&rgba, w, x, y) == solid(fg))
            .count();
        assert!(inked > 0, "no glyph pixels in {:?}", src.terrain[0]);
        // The second terrain's tile is the next one along.
        let (_, _, _, bg2) = &src.terrain[1];
        assert_eq!(px(&rgba, w, TILE, 0), solid(bg2));
    }

    #[test]
    fn class_pictures_are_discs_with_letters_after_the_terrain() {
        let src = sources();
        let (w, _, rgba) = image(&src);
        let top = rows(src.terrain.len()) * TILE;
        // A corner is clear; the disc's edge is the disc colour.
        assert_eq!(px(&rgba, w, 0, top), [0; 4]);
        assert_eq!(px(&rgba, w, 1, top + 12), DISC);
        let letters = (4..20)
            .flat_map(|y| (4..20).map(move |x| (x, y)))
            .filter(|&(x, y)| px(&rgba, w, x, top + y) == LETTERS)
            .count();
        assert!(letters > 0);
        // The fallback comes right after the last class.
        let (fx, fy) = cell(src.classes.len());
        let (x, y) = (fx * TILE, top + fy * TILE);
        assert_eq!(px(&rgba, w, x + 1, y + 12), DISC);
    }

    #[test]
    fn the_file_names_every_terrain_and_class() {
        let src = sources();
        let text = ron(&src);
        assert!(text.contains("    id: \"test\",\n    image: \"tilesets/test.png\",\n"));
        assert!(text.contains("    tile_px: (24, 24),\n"));
        assert!(text.contains("        \"plain\": (0, 0),\n"));
        // Classes by id.
        assert!(text.contains("            \"arcanist\": (0, 0),\n"));
        for (id, ..) in &src.terrain {
            assert!(text.contains(&format!("        \"{id}\": ")), "{id}");
        }
        for (id, _) in &src.classes {
            assert!(text.contains(&format!("            \"{id}\": ")), "{id}");
        }
        let top = rows(src.terrain.len()) * TILE;
        assert!(text.ends_with(&format!("    units_origin_px: (0, {top}),\n)\n")));
    }

    #[test]
    fn committed_files_are_what_the_tool_makes() {
        let src = sources();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let png = fs::read(root.join(display_path(PNG_PATH))).unwrap();
        assert_eq!(
            decode_png(&png).unwrap(),
            image(&src),
            "rerun `cargo xtask test-tileset`"
        );
        let text = fs::read_to_string(root.join(display_path(RON_PATH))).unwrap();
        assert_eq!(
            text.replace("\r\n", "\n"),
            ron(&src),
            "rerun `cargo xtask test-tileset`"
        );
    }

    #[test]
    fn run_writes_both_files_and_names_them() {
        let root = std::env::temp_dir().join(format!("xtask-test-tileset-{}", std::process::id()));
        let summary = run(&root).unwrap();
        let src = sources();
        let (w, h, rgba) = image(&src);
        assert_eq!(
            summary,
            format!(
                "test-tileset: wrote assets/tilesets/test.png ({w}×{h}) and assets/tilesets/test.ron"
            )
        );
        let written = fs::read(root.join("assets/tilesets/test.png")).unwrap();
        assert_eq!(decode_png(&written).unwrap(), (w, h, rgba));
        let text = fs::read_to_string(root.join("assets/tilesets/test.ron")).unwrap();
        assert_eq!(text, ron(&src));
        // A root that can't hold directories fails with the path.
        let file = root.join("file");
        fs::write(&file, "x").unwrap();
        let err = run(&file).unwrap_err();
        assert!(err.starts_with("creating "), "{err}");
        // The image's own path taken by a directory fails the write.
        let blocked = root.join("blocked");
        fs::create_dir_all(blocked.join("assets/tilesets/test.png")).unwrap();
        let err = run(&blocked).unwrap_err();
        assert!(err.starts_with("writing "), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }
}
