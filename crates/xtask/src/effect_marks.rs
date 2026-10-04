//! `cargo xtask effect-marks`: writes `assets/images/effect_marks.png`, the
//! arrows a sprite map skin puts on a unit under a timed effect (ticket
//! 0436, ADR-0049; `docs/design/look-and-feel.md`).
//!
//! 14×7: an **up arrow** (a bonus) then a **down arrow** (a penalty), each
//! a 5×5 arrow in its palette colour (`effect_bonus`, `effect_penalty`)
//! with a 1-pixel edge in the palette's `black`. It is our own picture,
//! made from the palette, and the same palette always gives the same
//! pixels: rerun the command after changing those colours.

use std::fs;
use std::path::Path;

use trpg_content::PaletteDef;
use trpg_content::bundle::display_path;
use trpg_content::image::EFFECT_MARKS_PATH;

use crate::font_atlas::encode_png;

/// The side of one arrow's box in pixels: the arrow and its edge.
pub const BOX: u32 = 7;
/// The up arrow, row by row (`#` = arrow); the down arrow is it upside
/// down.
pub const ARROW: [&str; 5] = ["..#..", ".###.", "#####", ".###.", ".###."];
/// The palette names of the up and the down arrow's colours.
pub const COLORS: [&str; 2] = ["effect_bonus", "effect_penalty"];
/// The palette name of the edge's colour.
pub const EDGE: &str = "black";

/// The colours the image is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colors {
    /// The up arrow's and the down arrow's.
    pub arrows: [[u8; 3]; 2],
    /// The edge's.
    pub edge: [u8; 3],
}

impl Colors {
    /// The embedded palette's.
    pub fn embedded() -> Result<Self, String> {
        let palette = PaletteDef::load().map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })?;
        let color = |name: &str| {
            palette
                .get(name)
                .ok_or_else(|| format!("no palette colour {name}"))
        };
        Ok(Self {
            arrows: [color(COLORS[0])?, color(COLORS[1])?],
            edge: color(EDGE)?,
        })
    }
}

/// Whether pixel `(x, y)` of an arrow's box is on the arrow; `down` for
/// the down arrow. Outside the box: no.
fn on_arrow(x: i64, y: i64, down: bool) -> bool {
    let row = if down { 5 - y } else { y - 1 };
    let cell = |v: i64| usize::try_from(v).ok();
    let at = cell(row).zip(cell(x - 1));
    at.and_then(|(row, column)| ARROW.get(row)?.as_bytes().get(column).copied()) == Some(b'#')
}

/// The colour of pixel `(x, y)` of the image.
fn pixel(colors: &Colors, x: u32, y: u32) -> [u8; 4] {
    let down = x >= BOX;
    let (bx, by) = (i64::from(x % BOX), i64::from(y));
    let solid = |[r, g, b]: [u8; 3]| [r, g, b, 255];
    if on_arrow(bx, by, down) {
        return solid(colors.arrows[usize::from(down)]);
    }
    let beside = |v: i64| v - 1..=v + 1;
    let near = beside(by).any(|ny| beside(bx).any(|nx| on_arrow(nx, ny, down)));
    if near { solid(colors.edge) } else { [0; 4] }
}

/// The image's RGBA8 pixels, row-major.
pub fn pixels(colors: &Colors) -> Vec<u8> {
    (0..BOX)
        .flat_map(|y| (0..2 * BOX).flat_map(move |x| pixel(colors, x, y)))
        .collect()
}

/// Runs the command: writes the image under repo root `root`.
pub fn run(root: &Path) -> Result<String, String> {
    let colors = Colors::embedded()?;
    let shown = display_path(EFFECT_MARKS_PATH);
    let path = root.join(&shown);
    let png = encode_png(2 * BOX, BOX, &pixels(&colors))?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    fs::write(&path, png).map_err(|e| format!("writing {}: {e}", path.display()))?;
    Ok(format!("effect-marks: wrote {shown} ({}×{BOX})", 2 * BOX))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_atlas::decode_png;

    const COLORS: Colors = Colors {
        arrows: [[1, 2, 3], [4, 5, 6]],
        edge: [7, 8, 9],
    };

    /// The image as text: `B` up arrow (bonus), `P` down arrow (penalty), `e` edge, `.` clear.
    fn drawn() -> Vec<String> {
        let rows = (0..BOX).map(|y| {
            let row = (0..2 * BOX).map(|x| match pixel(&COLORS, x, y) {
                [1, 2, 3, 255] => 'B',
                [4, 5, 6, 255] => 'P',
                [7, 8, 9, 255] => 'e',
                [0, 0, 0, 0] => '.',
                other => panic!("({x}, {y}): {other:?}"),
            });
            row.collect()
        });
        rows.collect()
    }

    #[test]
    fn an_up_arrow_then_a_down_arrow_each_with_a_one_pixel_edge() {
        assert_eq!(
            drawn(),
            [
                "..eee...eeeee.",
                ".eeBee..ePPPe.",
                "eeBBBeeeePPPee",
                "eBBBBBeePPPPPe",
                "eeBBBeeeePPPee",
                ".eBBBe..eePee.",
                ".eeeee...eee..",
            ]
        );
    }

    #[test]
    fn the_arrow_is_five_wide_and_only_inside_its_box() {
        for down in [false, true] {
            let on = |x, y| on_arrow(x, y, down);
            assert!((1..=5).all(|x| on(x, 3)), "the widest row, {down}");
            assert!(!on(0, 3) && !on(6, 3) && !on(-1, 3) && !on(3, -1) && !on(3, 7));
            let count = (0..7).flat_map(|y| (0..7).map(move |x| (x, y)));
            assert_eq!(count.filter(|&(x, y)| on(x, y)).count(), 15);
        }
        // The point: at the top going up, at the bottom going down.
        assert!(on_arrow(3, 1, false) && !on_arrow(2, 1, false));
        assert!(on_arrow(3, 5, true) && !on_arrow(2, 5, true));
        assert!(on_arrow(2, 5, false) && on_arrow(2, 1, true));
    }

    #[test]
    fn pixels_are_row_major() {
        let rgba = pixels(&COLORS);
        assert_eq!(rgba.len(), 14 * 7 * 4);
        let at = |x: usize, y: usize| &rgba[(y * 14 + x) * 4..][..4];
        assert_eq!(at(3, 1), [1, 2, 3, 255]);
        assert_eq!(at(10, 5), [4, 5, 6, 255]);
        assert_eq!(at(0, 0), [0, 0, 0, 0]);
        assert_eq!(at(2, 0), [7, 8, 9, 255]);
    }

    #[test]
    fn the_colours_are_the_palettes() {
        let colors = Colors::embedded().unwrap();
        let palette = PaletteDef::load().unwrap();
        assert_eq!(colors.arrows[0], palette.get("effect_bonus").unwrap());
        assert_eq!(colors.arrows[1], palette.get("effect_penalty").unwrap());
        assert_eq!(colors.edge, palette.get("black").unwrap());
        assert_ne!(colors.arrows[0], colors.arrows[1]);
    }

    #[test]
    fn committed_image_is_what_the_tool_makes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let committed = fs::read(root.join(display_path(EFFECT_MARKS_PATH))).unwrap();
        assert_eq!(
            decode_png(&committed).unwrap(),
            (14, 7, pixels(&Colors::embedded().unwrap())),
            "rerun `cargo xtask effect-marks`"
        );
    }

    #[test]
    fn run_writes_the_image_and_names_it() {
        let root = std::env::temp_dir().join(format!("xtask-effect-marks-{}", std::process::id()));
        let summary = run(&root).unwrap();
        assert_eq!(
            summary,
            "effect-marks: wrote assets/images/effect_marks.png (14×7)"
        );
        let written = fs::read(root.join("assets/images/effect_marks.png")).unwrap();
        let colors = Colors::embedded().unwrap();
        assert_eq!(decode_png(&written).unwrap(), (14, 7, pixels(&colors)));
        // A root that can't hold directories fails with the path.
        let file = root.join("file");
        fs::write(&file, "x").unwrap();
        let err = run(&file).unwrap_err();
        assert!(err.starts_with("creating "), "{err}");
        // The image's own path taken by a directory fails the write.
        let blocked = root.join("blocked");
        fs::create_dir_all(blocked.join("assets/images/effect_marks.png")).unwrap();
        let err = run(&blocked).unwrap_err();
        assert!(err.starts_with("writing "), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }
}
