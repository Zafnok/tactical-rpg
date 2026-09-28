//! The browsing cursor (ADR-0018, `docs/design/look-and-feel.md`): which
//! tile it is on, how it moves (one tile per step, never off the map; no
//! jump keys, `docs/design/controls.md`) and how it is drawn (ADR-0024): thin
//! corner marks around the tile, or a glow on it, pulsing between full and
//! about half brightness, never off.

use std::f32::consts::TAU;

use trpg_core::Pos;

use super::layout::{MAP_VIEW, TILE_W_CELLS};
use super::units::HP_BAR_H;
use crate::color::{Palette, UiColor};
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{GlyphBuffer, Layer, Overlay, Rect};
use crate::input::Action;

/// One full bright → dim → bright pulse, in seconds. *Tunable.*
pub const BLINK_PERIOD: f32 = 1.0;

/// The dimmest the cursor gets, as a fraction of full brightness. *Tunable.*
pub const BLINK_MIN: f32 = 0.5;

/// How strongly the tile glow tints the tile at full brightness (`0` = not
/// at all, `1` = solid `cursor` colour). *Tunable.*
pub const GLOW_MAX: f32 = 0.3;

/// How the cursor is drawn (`docs/design/look-and-feel.md`): corner marks by
/// default; bigger corners and the tile glow are accessibility options.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum CursorStyle {
    /// 3 px corner marks.
    #[default]
    Corners,
    /// 4 px corner marks.
    LargeCorners,
    /// The tile's background tinted towards the cursor colour.
    TileGlow,
}

/// The cursor: a map tile and the time into its pulse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    /// The tile under the cursor; always on the map.
    pub pos: Pos,
    /// Seconds into the current pulse, in `0..BLINK_PERIOD`.
    pub blink_t: f32,
}

impl Cursor {
    /// A cursor on `pos`, at full brightness.
    pub const fn new(pos: Pos) -> Self {
        Self { pos, blink_t: 0.0 }
    }

    /// Advances the pulse by `dt` seconds (negative or non-finite `dt`
    /// counts as zero).
    pub fn tick(&mut self, dt: f32) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.blink_t = (self.blink_t + dt) % BLINK_PERIOD;
    }

    /// Current brightness: `1` at the start of a pulse, down to
    /// [`BLINK_MIN`] halfway through, then back up (a cosine).
    pub fn brightness(&self) -> f32 {
        pulse(self.blink_t, BLINK_PERIOD)
    }

    /// Moves one tile for a `Cursor…` action, staying on a `map_w × map_h`
    /// map. Returns whether the cursor moved; a move restarts the pulse at
    /// full brightness, so the cursor is easy to follow. Other actions do
    /// nothing.
    pub fn step(&mut self, action: Action, map_w: u16, map_h: u16) -> bool {
        let (dx, dy) = match action {
            Action::CursorLeft => (-1, 0),
            Action::CursorRight => (1, 0),
            Action::CursorUp => (0, -1),
            Action::CursorDown => (0, 1),
            _ => return false,
        };
        let clamp = |v: i32, len: u16| v.clamp(0, (i32::from(len) - 1).max(0));
        let to = Pos::new(clamp(self.pos.x + dx, map_w), clamp(self.pos.y + dy, map_h));
        if to == self.pos {
            return false;
        }
        self.pos = to;
        self.blink_t = 0.0;
        true
    }

    /// Jumps to `pos` (e.g. the next unit), restarting the pulse.
    pub fn jump(&mut self, pos: Pos) {
        *self = Self::new(pos);
    }
}

/// Brightness `t` seconds into a `period`-long pulse: `1` at the start,
/// [`BLINK_MIN`] halfway, back to `1` at the end (a cosine).
fn pulse(t: f32, period: f32) -> f32 {
    let wave = (TAU * t / period).cos();
    BLINK_MIN + (1.0 - BLINK_MIN) * (1.0 + wave) / 2.0
}

/// Draws `cursor` in `style` on the tile whose left cell is `(x, y)`,
/// clipped to the map viewport.
///
/// Corner marks are 1 px `Over` overlays that stay clear of every letter
/// (ADR-0024): their vertical arms sit in the pixel column just left of the
/// tile and in the tile's last pixel column, which the font never inks; their
/// horizontal arms sit on the tile's top row and on the row just above the
/// HP bar, outside the letters' rows. So a neighbour's initials always show.
pub fn draw_cursor(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    cursor: &Cursor,
    style: CursorStyle,
    x: i32,
    y: i32,
) {
    let color = palette.get(UiColor::Cursor);
    let arm = match style {
        CursorStyle::Corners => 3,
        CursorStyle::LargeCorners => 4,
        CursorStyle::TileGlow => {
            let tile = Rect::new(x, y, TILE_W_CELLS, 1);
            if let Some(tile) = tile.intersect(&MAP_VIEW) {
                buf.blend_bg(tile, color, GLOW_MAX * cursor.brightness());
            }
            return;
        }
    };
    let fg = color.scale(cursor.brightness());
    let view = px_rect(MAP_VIEW);
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

/// `cells` in console pixels.
pub(super) fn px_rect(cells: Rect) -> Rect {
    let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
    Rect::new(cells.x * cw, cells.y * ch, cells.w * cw, cells.h * ch)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::color::Rgb;
    use crate::color::tests::game_palette;
    use crate::glyph_buffer::Cell;

    const MOVES: [Action; 4] = [
        Action::CursorLeft,
        Action::CursorRight,
        Action::CursorUp,
        Action::CursorDown,
    ];

    #[test]
    fn steps_one_tile_and_clamps_at_the_edges() {
        let mut c = Cursor::new(Pos::new(1, 1));
        c.blink_t = 0.3;
        assert!(c.step(Action::CursorLeft, 3, 3));
        assert_eq!((c.pos, c.blink_t), (Pos::new(0, 1), 0.0));
        c.blink_t = 0.3;
        assert!(!c.step(Action::CursorLeft, 3, 3));
        assert_eq!((c.pos, c.blink_t), (Pos::new(0, 1), 0.3));
        assert!(c.step(Action::CursorUp, 3, 3));
        assert!(!c.step(Action::CursorUp, 3, 3));
        assert_eq!(c.pos, Pos::new(0, 0));
        assert!(c.step(Action::CursorRight, 3, 3));
        assert!(c.step(Action::CursorRight, 3, 3));
        assert!(!c.step(Action::CursorRight, 3, 3));
        assert!(c.step(Action::CursorDown, 3, 3));
        assert!(c.step(Action::CursorDown, 3, 3));
        assert!(!c.step(Action::CursorDown, 3, 3));
        assert_eq!(c.pos, Pos::new(2, 2));
        assert!(!c.step(Action::Confirm, 3, 3));
        assert!(!c.step(Action::NextUnit, 3, 3));
        assert_eq!(c.pos, Pos::new(2, 2));
    }

    #[test]
    fn jump_moves_and_restarts_the_pulse() {
        let mut c = Cursor::new(Pos::new(1, 1));
        c.tick(0.4);
        c.jump(Pos::new(5, 6));
        assert_eq!(c, Cursor::new(Pos::new(5, 6)));
    }

    #[test]
    fn pulses_between_full_and_half_brightness() {
        let mut c = Cursor::new(Pos::new(0, 0));
        assert!((c.brightness() - 1.0).abs() < 1e-6);
        c.tick(0.25);
        assert!((c.blink_t - 0.25).abs() < 1e-6);
        assert!((c.brightness() - 0.75).abs() < 1e-6);
        c.tick(0.25);
        assert!((c.brightness() - 0.5).abs() < 1e-6);
        c.tick(0.5);
        assert!(c.blink_t < 1e-6, "wraps after a period: {}", c.blink_t);
        c.tick(0.25);
        c.tick(-1.0);
        c.tick(f32::NAN);
        c.tick(f32::INFINITY);
        assert!((c.blink_t - 0.25).abs() < 1e-6, "{}", c.blink_t);
    }

    #[test]
    fn pulse_scales_with_its_period() {
        assert!((pulse(0.0, 2.0) - 1.0).abs() < 1e-6);
        assert!((pulse(0.5, 2.0) - 0.75).abs() < 1e-6);
        assert!((pulse(1.0, 2.0) - BLINK_MIN).abs() < 1e-6);
        assert!((pulse(2.0, 2.0) - 1.0).abs() < 1e-6);
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
        draw_cursor(&mut buf, &p, &c, CursorStyle::Corners, 10, 4);
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
        draw_cursor(
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
        draw_cursor(&mut buf, &p, &c, CursorStyle::Corners, 10, 4);
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
        draw_cursor(&mut buf, &p, &c, CursorStyle::TileGlow, 10, 4);
        let bright = bg.lerp(cursor, GLOW_MAX);
        let cell = |b: &GlyphBuffer, x| *b.get(x, 4).unwrap();
        assert_eq!(cell(&buf, 10), Cell::new('.', Rgb::new(1, 1, 1), bright));
        assert_eq!(cell(&buf, 11), Cell::new('.', Rgb::new(1, 1, 1), bright));
        assert_eq!(cell(&buf, 9).bg, bg);
        assert_eq!(cell(&buf, 12).bg, bg);
        assert!(buf.overlays().is_empty());
        c.tick(0.5);
        let mut dim = dots();
        draw_cursor(&mut dim, &p, &c, CursorStyle::TileGlow, 10, 4);
        assert_eq!(cell(&dim, 10).bg, bg.lerp(cursor, GLOW_MAX * 0.5));
    }

    #[test]
    fn px_rect_scales_cells_to_pixels() {
        assert_eq!(px_rect(Rect::new(3, 2, 5, 4)), Rect::new(24, 32, 40, 64));
        assert_eq!(px_rect(MAP_VIEW), Rect::new(0, 0, 560, 480));
    }

    #[test]
    fn nothing_is_drawn_outside_the_map_viewport() {
        let p = game_palette();
        let c = Cursor::new(Pos::new(0, 0));
        let view = px_rect(MAP_VIEW);
        for style in [CursorStyle::Corners, CursorStyle::LargeCorners] {
            for (x, y) in [(0, 0), (68, 29), (0, 29), (68, 0)] {
                let mut buf = dots();
                draw_cursor(&mut buf, &p, &c, style, x, y);
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
        draw_cursor(&mut buf, &p, &c, CursorStyle::Corners, 0, 0);
        assert_eq!(
            rects(&buf)[..2],
            [Rect::new(0, 0, 2, 1), Rect::new(13, 0, 3, 1)]
        );
        // Off the viewport entirely (the help bar rows): nothing at all.
        for style in [CursorStyle::Corners, CursorStyle::TileGlow] {
            let mut buf = dots();
            let before = buf.clone();
            draw_cursor(&mut buf, &p, &c, style, 10, 30);
            assert_eq!(buf, before);
        }
    }

    proptest! {
        #[test]
        fn never_leaves_the_map(
            w in 1u16..20,
            h in 1u16..20,
            steps in prop::collection::vec(0usize..4, 0..60),
        ) {
            let mut c = Cursor::new(Pos::new(0, 0));
            for s in steps {
                c.step(MOVES[s], w, h);
                prop_assert!((0..i32::from(w)).contains(&c.pos.x));
                prop_assert!((0..i32::from(h)).contains(&c.pos.y));
            }
        }

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
            draw_cursor(&mut buf, &p, &c, style, x, y);
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

        #[test]
        fn brightness_stays_between_min_and_full(t in -10.0f32..10.0) {
            let mut c = Cursor::new(Pos::new(0, 0));
            c.tick(t);
            let b = c.brightness();
            prop_assert!((BLINK_MIN - 1e-6..=1.0 + 1e-6).contains(&b), "{b}");
        }
    }
}
