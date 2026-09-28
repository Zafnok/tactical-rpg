//! Drawing portraits (ADR-0018): two pixels per cell, as `▀` with the top
//! pixel in fg and the bottom pixel in bg.

use trpg_content::Portrait;

use crate::color::{Palette, Rgb};
use crate::glyph_buffer::{Cell, GlyphBuffer};

/// Upper half block: fg is the top pixel, bg the bottom one.
const UPPER: char = '▀';
/// Lower half block: used when only the bottom pixel is opaque.
const LOWER: char = '▄';

/// Draws expression `expr` of `portrait` with its top-left cell at `(x, y)`,
/// over whatever background the cells there already have: a transparent
/// pixel shows it. `dim` (0 = full colour, 1 = gone) lerps every pixel
/// toward that background; `mirror` flips it left to right. Clipped to the
/// buffer. Returns `false`, drawing nothing, when the portrait has no such
/// expression.
pub fn draw_portrait(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    (x, y): (i32, i32),
    portrait: &Portrait,
    expr: &str,
    dim: f32,
    mirror: bool,
) -> bool {
    let Some(expression) = portrait.expression(expr) else {
        return false;
    };
    for row in 0..portrait.height.div_ceil(2) {
        for col in 0..portrait.width {
            let cx = x + i32::from(col);
            let cy = y + i32::from(row);
            let Some(bg) = buf.get(cx, cy).map(|c| c.bg) else {
                continue;
            };
            let px = if mirror {
                portrait.width - 1 - col
            } else {
                col
            };
            let color = |py: u16| {
                portrait
                    .color_at(expression, px, py)
                    .and_then(|name| palette.lookup(name))
                    .map(|rgb| rgb.lerp(bg, dim))
            };
            buf.set(cx, cy, cell(color(row * 2), color(row * 2 + 1), bg));
        }
    }
    true
}

/// The cell for a top and bottom pixel (`None` = transparent) over `bg`.
fn cell(top: Option<Rgb>, bottom: Option<Rgb>, bg: Rgb) -> Cell {
    match (top, bottom) {
        (Some(t), b) => Cell::new(UPPER, t, b.unwrap_or(bg)),
        (None, Some(b)) => Cell::new(LOWER, b, bg),
        (None, None) => Cell::new(' ', bg, bg),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use insta::assert_snapshot;
    use trpg_content::portrait::Expression;

    use super::*;
    use crate::color::UiColor;
    use crate::color::tests::game_palette;

    fn content() -> trpg_content::Content {
        trpg_content::load_embedded().unwrap()
    }

    /// A 3×4 portrait (2 cell rows): `h` = player, `s` = enemy.
    fn tiny() -> Portrait {
        let keys = "hs..s.h.h.ss";
        Portrait {
            character: "tiny".to_owned(),
            width: 3,
            height: 4,
            colors: BTreeMap::from([('h', "player".to_owned()), ('s', "enemy".to_owned())]),
            expressions: vec![Expression {
                name: "neutral".to_owned(),
                pixels: keys.chars().map(|k| (k != '.').then_some(k)).collect(),
            }],
        }
    }

    fn canvas(w: u16, h: u16) -> (Palette, GlyphBuffer, Rgb) {
        let p = game_palette();
        let bg = p.get(UiColor::PanelBg);
        let buf = GlyphBuffer::new(w, h, Cell::new('x', p.get(UiColor::Text), bg));
        (p, buf, bg)
    }

    #[test]
    fn two_pixels_per_cell() {
        let (p, mut buf, bg) = canvas(3, 2);
        assert!(draw_portrait(
            &mut buf,
            &p,
            (0, 0),
            &tiny(),
            "neutral",
            0.0,
            false
        ));
        let (h, s) = (p.get(UiColor::Player), p.get(UiColor::Enemy));
        // Rows "hs." over ".s.", then "h.h" over ".ss".
        assert_eq!(buf.get(0, 0), Some(&Cell::new('▀', h, bg)));
        assert_eq!(buf.get(1, 0), Some(&Cell::new('▀', s, s)));
        assert_eq!(buf.get(2, 0), Some(&Cell::new(' ', bg, bg)));
        assert_eq!(buf.get(0, 1), Some(&Cell::new('▀', h, bg)));
        assert_eq!(buf.get(1, 1), Some(&Cell::new('▄', s, bg)));
        assert_eq!(buf.get(2, 1), Some(&Cell::new('▀', h, s)));
    }

    #[test]
    fn mirror_reverses_each_row() {
        let (p, mut plain, _) = canvas(3, 2);
        let (_, mut mirrored, _) = canvas(3, 2);
        draw_portrait(&mut plain, &p, (0, 0), &tiny(), "neutral", 0.0, false);
        draw_portrait(&mut mirrored, &p, (0, 0), &tiny(), "neutral", 0.0, true);
        for y in 0..2 {
            for x in 0..3 {
                assert_eq!(plain.get(x, y), mirrored.get(2 - x, y));
            }
        }
    }

    #[test]
    fn dim_lerps_toward_the_background() {
        let (p, mut buf, bg) = canvas(3, 2);
        draw_portrait(&mut buf, &p, (0, 0), &tiny(), "neutral", 0.25, false);
        let (h, s) = (p.get(UiColor::Player), p.get(UiColor::Enemy));
        assert_eq!(
            buf.get(1, 0),
            Some(&Cell::new('▀', s.lerp(bg, 0.25), s.lerp(bg, 0.25)))
        );
        assert_eq!(buf.get(0, 0), Some(&Cell::new('▀', h.lerp(bg, 0.25), bg)));
        // Each cell dims toward its own background.
        let (_, mut buf, _) = canvas(3, 2);
        let other = p.get(UiColor::Black);
        buf.set(0, 0, Cell::new(' ', other, other));
        draw_portrait(&mut buf, &p, (0, 0), &tiny(), "neutral", 1.0, false);
        assert_eq!(buf.get(0, 0), Some(&Cell::new('▀', other, other)));
        assert_eq!(buf.get(1, 0), Some(&Cell::new('▀', bg, bg)));
    }

    #[test]
    fn clips_to_the_buffer() {
        let (p, full, _) = canvas(5, 4);
        let mut expected = full.clone();
        draw_portrait(&mut expected, &p, (1, 1), &tiny(), "neutral", 0.0, false);
        for (dx, dy) in [(-2, 0), (0, -1), (3, 0), (0, 3), (-3, -2), (5, 4)] {
            let mut buf = full.clone();
            assert!(draw_portrait(
                &mut buf,
                &p,
                (dx, dy),
                &tiny(),
                "neutral",
                0.0,
                false
            ));
            for y in 0..4 {
                for x in 0..5 {
                    let (sx, sy) = (x - dx + 1, y - dy + 1);
                    let inside = (0..3).contains(&(x - dx)) && (0..2).contains(&(y - dy));
                    let want = if inside {
                        expected.get(sx, sy)
                    } else {
                        full.get(x, y)
                    };
                    assert_eq!(buf.get(x, y), want, "at ({x}, {y}) drawn from ({dx}, {dy})");
                }
            }
        }
    }

    #[test]
    fn unknown_expression_draws_nothing() {
        let (p, mut buf, _) = canvas(3, 2);
        let before = buf.clone();
        assert!(!draw_portrait(
            &mut buf,
            &p,
            (0, 0),
            &tiny(),
            "bored",
            0.0,
            false
        ));
        assert_eq!(buf, before);
    }

    #[test]
    fn a_full_portrait_is_32_by_16_cells() {
        let content = content();
        let portrait = &content.portraits["test_lord"];
        let (p, mut buf, _) = canvas(34, 18);
        let before = buf.clone();
        draw_portrait(&mut buf, &p, (1, 1), portrait, "neutral", 0.0, false);
        for y in 0..18 {
            for x in 0..34 {
                let inside = (1..33).contains(&x) && (1..17).contains(&y);
                if !inside {
                    assert_eq!(buf.get(x, y), before.get(x, y), "({x}, {y})");
                }
            }
        }
        // The portrait fills its bottom row (shoulders).
        assert_ne!(buf.get(16, 16), before.get(16, 16));
    }

    /// A 32×16 snapshot of `expr` of portrait `id`.
    fn snap(id: &str, expr: &str, dim: f32, mirror: bool) -> String {
        let content = content();
        let (p, mut buf, _) = canvas(32, 16);
        assert!(draw_portrait(
            &mut buf,
            &p,
            (0, 0),
            &content.portraits[id],
            expr,
            dim,
            mirror
        ));
        buf.to_snapshot(&p)
    }

    #[test]
    fn test_lord_snapshots() {
        assert_snapshot!(
            "test_lord_neutral",
            snap("test_lord", "neutral", 0.0, false)
        );
        assert_snapshot!("test_lord_happy", snap("test_lord", "happy", 0.0, false));
        assert_snapshot!("test_lord_dimmed", snap("test_lord", "neutral", 0.5, false));
        assert_snapshot!("test_lord_mirrored", snap("test_lord", "sad", 0.0, true));
    }

    #[test]
    fn test_knight_snapshots() {
        assert_snapshot!(
            "test_knight_neutral",
            snap("test_knight", "neutral", 0.0, false)
        );
        assert_snapshot!(
            "test_knight_angry",
            snap("test_knight", "angry", 0.0, false)
        );
        assert_snapshot!(
            "test_knight_dimmed",
            snap("test_knight", "angry", 0.5, false)
        );
        assert_snapshot!(
            "test_knight_mirrored",
            snap("test_knight", "neutral", 0.0, true)
        );
    }
}
