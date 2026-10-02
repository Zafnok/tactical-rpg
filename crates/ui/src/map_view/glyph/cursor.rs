//! How the glyph skin draws the cursor (ADR-0024): thin corner marks
//! around the tile, or a glow on it.

use super::units::HP_BAR_H;
use super::{Layout, TILE_W_CELLS, px_rect};
use crate::color::{Palette, UiColor};
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{GlyphBuffer, Layer, Overlay, Rect};
use crate::map_view::scene::{CursorStyle, CursorView};

/// How strongly the tile glow tints the tile at full brightness (`0` = not
/// at all, `1` = solid `cursor` colour). *Tunable.*
pub const GLOW_MAX: f32 = 0.3;

/// Draws `cursor` in its style on the tile whose left cell is `(x, y)`,
/// clipped to the tiles of `layout`.
///
/// Corner marks are 1 px `Over` overlays that stay clear of every letter
/// (ADR-0024): their vertical arms sit in the pixel column just left of the
/// tile and in the tile's last pixel column, which the font never inks; their
/// horizontal arms sit on the tile's top row and on the row just above the
/// HP bar, outside the letters' rows. So a neighbour's initials always show.
pub fn draw_cursor(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    cursor: &CursorView,
    layout: &Layout,
    x: i32,
    y: i32,
) {
    let color = palette.get(UiColor::Cursor);
    let view = layout.cells();
    let arm = match cursor.style {
        CursorStyle::Corners => 3,
        CursorStyle::LargeCorners => 4,
        CursorStyle::TileGlow => {
            let tile = Rect::new(x, y, TILE_W_CELLS, 1);
            if let Some(tile) = tile.intersect(&view) {
                buf.blend_bg(tile, color, GLOW_MAX * cursor.brightness);
            }
            return;
        }
    };
    let fg = color.scale(cursor.brightness);
    let view = px_rect(view);
    for r in corner_arms(x, y, arm) {
        if let Some(r) = r.intersect(&view) {
            buf.add_overlay(Overlay::new(r, fg, Layer::Over));
        }
    }
}

/// The eight 1 px arms of the corner marks around the tile whose left cell
/// is `(x, y)`, each `arm` px long, in console pixels.
fn corner_arms(x: i32, y: i32, arm: i32) -> [Rect; 8] {
    let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
    // Left arms in the left neighbour's last (always blank) pixel column,
    // right arms in this tile's last one; top arms on the tile's first row,
    // bottom arms on the row above the 2 px HP bar.
    let left = x * cw - 1;
    let right = (x + TILE_W_CELLS) * cw - 1;
    let top = y * ch;
    let bottom = top + ch - HP_BAR_H - 1;
    let across = |x0, y0| Rect::new(x0, y0, arm, 1);
    let down = |x0, y0| Rect::new(x0, y0, 1, arm);
    [
        across(left, top),
        down(left, top),
        across(right - arm + 1, top),
        down(right, top),
        across(left, bottom),
        down(left, bottom - arm + 1),
        across(right - arm + 1, bottom),
        down(right, bottom - arm + 1),
    ]
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_core::Pos;

    use super::*;
    use crate::color::Rgb;
    use crate::color::tests::game_palette;
    use crate::glyph_buffer::Cell;
    use crate::map_view::glyph::tests::layout_at;
    use crate::screens::battle::cursor::Cursor;
    use crate::screens::battle::layout::MAP_VIEW;

    /// Draws `cursor` in `style` on the tile whose left cell is `(x, y)` of
    /// the battle screen's map area.
    fn draw(
        buf: &mut GlyphBuffer,
        palette: &Palette,
        cursor: &Cursor,
        style: CursorStyle,
        x: i32,
        y: i32,
    ) {
        let view = CursorView {
            pos: cursor.pos,
            brightness: cursor.brightness(),
            style,
        };
        draw_cursor(buf, palette, &view, &layout_at(Pos::new(0, 0)), x, y);
    }

    /// A 100 × 32 buffer of `.` on a dark background.
    fn dots() -> GlyphBuffer {
        GlyphBuffer::new(
            100,
            32,
            Cell::new('.', Rgb::new(1, 1, 1), Rgb::new(9, 9, 9)),
        )
    }

    fn rects(buf: &GlyphBuffer) -> Vec<Rect> {
        buf.overlays().iter().map(|o| o.rect).collect()
    }

    #[test]
    fn corner_marks_frame_the_tile_outside_the_letters() {
        let p = game_palette();
        let c = Cursor::new(Pos::new(0, 0));
        let mut buf = dots();
        let before = buf.clone();
        // Tile at cells (10, 4)-(11, 4): pixels x 80..96, y 64..80.
        draw(&mut buf, &p, &c, CursorStyle::Corners, 10, 4);
        let r = Rect::new;
        assert_eq!(
            rects(&buf),
            [
                r(79, 64, 3, 1),
                r(79, 64, 1, 3),
                r(93, 64, 3, 1),
                r(95, 64, 1, 3),
                r(79, 77, 3, 1),
                r(79, 75, 1, 3),
                r(93, 77, 3, 1),
                r(95, 75, 1, 3),
            ]
        );
        let cursor = p.get(UiColor::Cursor);
        for o in buf.overlays() {
            assert_eq!((o.color, o.layer), (cursor, Layer::Over));
        }
        for (cx, cy) in (0..100).flat_map(|cx| (0..32).map(move |cy| (cx, cy))) {
            assert_eq!(buf.get(cx, cy), before.get(cx, cy), "cell ({cx}, {cy})");
        }
    }

    #[test]
    fn large_corners_have_longer_arms() {
        let p = game_palette();
        let mut buf = dots();
        draw(
            &mut buf,
            &p,
            &Cursor::new(Pos::new(0, 0)),
            CursorStyle::LargeCorners,
            10,
            4,
        );
        let r = Rect::new;
        assert_eq!(
            rects(&buf),
            [
                r(79, 64, 4, 1),
                r(79, 64, 1, 4),
                r(92, 64, 4, 1),
                r(95, 64, 1, 4),
                r(79, 77, 4, 1),
                r(79, 74, 1, 4),
                r(92, 77, 4, 1),
                r(95, 74, 1, 4),
            ]
        );
    }

    #[test]
    fn corner_marks_pulse() {
        let p = game_palette();
        let mut c = Cursor::new(Pos::new(0, 0));
        c.tick(0.5);
        let mut buf = dots();
        draw(&mut buf, &p, &c, CursorStyle::Corners, 10, 4);
        let dim = p.get(UiColor::Cursor).scale(0.5);
        assert!(buf.overlays().iter().all(|o| o.color == dim));
    }

    #[test]
    fn tile_glow_tints_only_the_tile_and_pulses() {
        let p = game_palette();
        let cursor = p.get(UiColor::Cursor);
        let mut c = Cursor::new(Pos::new(0, 0));
        let mut buf = dots();
        let bg = Rgb::new(9, 9, 9);
        draw(&mut buf, &p, &c, CursorStyle::TileGlow, 10, 4);
        let bright = bg.lerp(cursor, GLOW_MAX);
        let cell = |b: &GlyphBuffer, x| *b.get(x, 4).unwrap();
        assert_eq!(cell(&buf, 10), Cell::new('.', Rgb::new(1, 1, 1), bright));
        assert_eq!(cell(&buf, 11), Cell::new('.', Rgb::new(1, 1, 1), bright));
        assert_eq!(cell(&buf, 9).bg, bg);
        assert_eq!(cell(&buf, 12).bg, bg);
        assert!(buf.overlays().is_empty());
        c.tick(0.5);
        let mut dim = dots();
        draw(&mut dim, &p, &c, CursorStyle::TileGlow, 10, 4);
        assert_eq!(cell(&dim, 10).bg, bg.lerp(cursor, GLOW_MAX * 0.5));
    }

    #[test]
    fn nothing_is_drawn_outside_the_map_viewport() {
        let p = game_palette();
        let c = Cursor::new(Pos::new(0, 0));
        let view = px_rect(MAP_VIEW);
        for style in [CursorStyle::Corners, CursorStyle::LargeCorners] {
            for (x, y) in [(0, 0), (68, 29), (0, 29), (68, 0)] {
                let mut buf = dots();
                draw(&mut buf, &p, &c, style, x, y);
                assert!(!buf.overlays().is_empty());
                for r in rects(&buf) {
                    assert_eq!(
                        r.intersect(&view),
                        Some(r),
                        "{style:?} at ({x}, {y}): {r:?}"
                    );
                }
            }
        }
        // At the left edge the left arms' column is off the viewport: the
        // vertical ones go, the horizontal ones lose a pixel.
        let mut buf = dots();
        draw(&mut buf, &p, &c, CursorStyle::Corners, 0, 0);
        assert_eq!(
            rects(&buf)[..2],
            [Rect::new(0, 0, 2, 1), Rect::new(13, 0, 3, 1)]
        );
        // Off the viewport entirely (the help bar rows): nothing at all.
        for style in [CursorStyle::Corners, CursorStyle::TileGlow] {
            let mut buf = dots();
            let before = buf.clone();
            draw(&mut buf, &p, &c, style, 10, 30);
            assert_eq!(buf, before);
        }
    }

    proptest! {
        /// Whatever the style, pulse and position, the cursor changes no
        /// glyph or glyph colour, and its overlays stay out of the letters:
        /// in the tile's rows 2..=11 (where letters are inked) only the
        /// pixel column left of the tile and the tile's last one are used.
        #[test]
        fn never_covers_letters(
            style in prop::sample::select(vec![
                CursorStyle::Corners,
                CursorStyle::LargeCorners,
                CursorStyle::TileGlow,
            ]),
            t in 0.0f32..1.0,
            tx in 0i32..35,
            ty in 0i32..30,
        ) {
            let p = game_palette();
            let mut c = Cursor::new(Pos::new(0, 0));
            c.tick(t);
            let mut buf = dots();
            let before = buf.clone();
            let (x, y) = (tx * TILE_W_CELLS, ty);
            draw(&mut buf, &p, &c, style, x, y);
            for cy in 0..32 {
                for cx in 0..100 {
                    let (a, b) = (before.get(cx, cy).unwrap(), buf.get(cx, cy).unwrap());
                    prop_assert_eq!((a.glyph, a.fg), (b.glyph, b.fg));
                }
            }
            let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
            for r in rects(&buf) {
                for py in r.y..r.y + r.h {
                    for px in r.x..r.x + r.w {
                        let (rx, ry) = (px - x * cw, py - y * ch);
                        prop_assert!((-1..=15).contains(&rx) && (0..=13).contains(&ry));
                        prop_assert!(
                            !(2..=11).contains(&ry) || rx == -1 || rx == 15,
                            "pixel ({}, {}) of the tile", rx, ry
                        );
                    }
                }
            }
        }
    }
}
