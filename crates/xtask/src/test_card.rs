//! `cargo xtask test-card`: writes `assets/images/test_card.png`, the
//! public test image for sprite items (ticket 0231, ADR-0038).
//!
//! 16×16: a one-pixel white border around four coloured quadrants (red and
//! green above blue and yellow), so a flip, a crop and a scale each show.
//! It is generated, not art, and the same code always writes the same
//! pixels.

use std::fs;
use std::path::Path;

use trpg_content::bundle::display_path;
use trpg_content::image::TEST_CARD_PATH;

use crate::font_atlas::encode_png;

/// Width and height of the card in pixels.
pub const SIZE: u32 = 16;
/// The border colour (RGBA).
pub const BORDER: [u8; 4] = [255, 255, 255, 255];
/// The quadrants' colours (RGBA): top-left, top-right, bottom-left,
/// bottom-right.
pub const QUADRANTS: [[u8; 4]; 4] = [
    [204, 51, 51, 255],
    [51, 170, 68, 255],
    [51, 102, 204, 255],
    [230, 190, 50, 255],
];

/// The colour of pixel `(x, y)`.
fn pixel(x: u32, y: u32) -> [u8; 4] {
    let edge = |v: u32| v == 0 || v == SIZE - 1;
    if edge(x) || edge(y) {
        return BORDER;
    }
    let half = SIZE / 2;
    QUADRANTS[usize::from(y >= half) * 2 + usize::from(x >= half)]
}

/// The card's RGBA8 pixels, row-major.
pub fn pixels() -> Vec<u8> {
    (0..SIZE)
        .flat_map(|y| (0..SIZE).flat_map(move |x| pixel(x, y)))
        .collect()
}

/// Runs the command: writes the card under repo root `root`.
pub fn run(root: &Path) -> Result<String, String> {
    let shown = display_path(TEST_CARD_PATH);
    let path = root.join(&shown);
    let png = encode_png(SIZE, SIZE, &pixels())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    fs::write(&path, png).map_err(|e| format!("writing {}: {e}", path.display()))?;
    Ok(format!("test-card: wrote {shown} ({SIZE}×{SIZE})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font_atlas::decode_png;

    #[test]
    fn border_and_quadrants() {
        let [red, green, blue, yellow] = QUADRANTS;
        for i in 0..SIZE {
            for edge in [(i, 0), (i, 15), (0, i), (15, i)] {
                assert_eq!(pixel(edge.0, edge.1), BORDER, "{edge:?}");
            }
        }
        // Each quadrant's corners: just inside the border and at the middle.
        for (colour, xs, ys) in [
            (red, [1, 7], [1, 7]),
            (green, [8, 14], [1, 7]),
            (blue, [1, 7], [8, 14]),
            (yellow, [8, 14], [8, 14]),
        ] {
            for x in xs {
                for y in ys {
                    assert_eq!(pixel(x, y), colour, "({x}, {y})");
                }
            }
        }
        let mut distinct = QUADRANTS.to_vec();
        distinct.push(BORDER);
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 5);
    }

    #[test]
    fn pixels_are_row_major() {
        let rgba = pixels();
        assert_eq!(rgba.len(), 16 * 16 * 4);
        let at = |x: usize, y: usize| &rgba[(y * 16 + x) * 4..][..4];
        assert_eq!(at(0, 0), BORDER);
        assert_eq!(at(14, 1), QUADRANTS[1]);
        assert_eq!(at(1, 14), QUADRANTS[2]);
    }

    #[test]
    fn committed_card_is_what_the_tool_makes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = root.join(display_path(TEST_CARD_PATH));
        let committed = fs::read(&path).unwrap();
        assert_eq!(
            decode_png(&committed).unwrap(),
            (SIZE, SIZE, pixels()),
            "rerun `cargo xtask test-card`"
        );
    }

    #[test]
    fn run_writes_the_card_and_names_it() {
        let root = std::env::temp_dir().join(format!("xtask-test-card-{}", std::process::id()));
        let summary = run(&root).unwrap();
        assert_eq!(
            summary,
            "test-card: wrote assets/images/test_card.png (16×16)"
        );
        let written = fs::read(root.join("assets/images/test_card.png")).unwrap();
        assert_eq!(decode_png(&written).unwrap(), (16, 16, pixels()));
        // A root that can't hold directories fails with the path.
        let file = root.join("file");
        fs::write(&file, "x").unwrap();
        let err = run(&file).unwrap_err();
        assert!(err.starts_with("creating "), "{err}");
        // The card's own path taken by a directory fails the write.
        let blocked = root.join("blocked");
        fs::create_dir_all(blocked.join("assets/images/test_card.png")).unwrap();
        let err = run(&blocked).unwrap_err();
        assert!(err.starts_with("writing "), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }
}
