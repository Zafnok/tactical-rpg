//! The browsing cursor (ADR-0018, `docs/design/look-and-feel.md`): which
//! tile it is on, how it moves (one tile per step, never off the map; no
//! jump keys, `docs/design/controls.md`) and how it is drawn: `[` and `]` in
//! the cells either side of the tile, pulsing between full and about half
//! brightness, never off.

use std::f32::consts::TAU;

use trpg_core::Pos;

use super::layout::{MAP_VIEW, TILE_W_CELLS};
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;

/// One full bright → dim → bright pulse, in seconds. *Tunable.*
pub const BLINK_PERIOD: f32 = 1.0;

/// The dimmest the cursor gets, as a fraction of full brightness. *Tunable.*
pub const BLINK_MIN: f32 = 0.5;

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

/// The browsing cursor's marks, either side of the tile.
pub const BRACKETS: [char; 2] = ['[', ']'];

/// The marks while the cursor is on the selected unit.
pub const ARROWS: [char; 2] = ['►', '◄'];

/// Draws `cursor`'s `marks` (e.g. [`BRACKETS`]) either side of the tile
/// whose left cell is `(x, y)`, over the neighbouring cells' glyphs and
/// keeping their background. Marks outside the map viewport are not drawn.
pub fn draw_cursor(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    cursor: &Cursor,
    marks: [char; 2],
    (x, y): (i32, i32),
) {
    let fg = palette.get(UiColor::Cursor).scale(cursor.brightness());
    let [left, right] = marks;
    for (glyph, cx) in [(left, x - 1), (right, x + TILE_W_CELLS)] {
        if !MAP_VIEW.contains(cx, y) {
            continue;
        }
        if let Some(&cell) = buf.get(cx, y) {
            buf.set(cx, y, Cell { glyph, fg, ..cell });
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::color::Rgb;
    use crate::color::tests::game_palette;

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

    #[test]
    fn draws_brackets_either_side_keeping_the_background() {
        let p = game_palette();
        let bg = Rgb::new(9, 9, 9);
        let mut buf = GlyphBuffer::new(100, 32, Cell::new('.', Rgb::new(1, 1, 1), bg));
        let mut c = Cursor::new(Pos::new(0, 0));
        draw_cursor(&mut buf, &p, &c, BRACKETS, (10, 4));
        let cursor = p.get(UiColor::Cursor);
        assert_eq!(*buf.get(9, 4).unwrap(), Cell::new('[', cursor, bg));
        assert_eq!(*buf.get(12, 4).unwrap(), Cell::new(']', cursor, bg));
        assert_eq!(buf.get(10, 4).unwrap().glyph, '.');
        assert_eq!(buf.get(11, 4).unwrap().glyph, '.');
        c.tick(0.5);
        draw_cursor(&mut buf, &p, &c, BRACKETS, (10, 4));
        assert_eq!(buf.get(9, 4).unwrap().fg, cursor.scale(0.5));
        // At the viewport's edges: the bracket outside it is left out.
        draw_cursor(&mut buf, &p, &c, BRACKETS, (68, 7));
        assert_eq!(buf.get(67, 7).unwrap().glyph, '[');
        assert_eq!(buf.get(70, 7).unwrap().glyph, '.');
        draw_cursor(&mut buf, &p, &c, BRACKETS, (0, 8));
        assert_eq!(buf.get(2, 8).unwrap().glyph, ']');
        draw_cursor(&mut buf, &p, &c, BRACKETS, (10, 30));
        assert_eq!(buf.get(9, 30).unwrap().glyph, '.');
        // On a selected unit: arrows instead.
        draw_cursor(&mut buf, &p, &c, ARROWS, (20, 4));
        assert_eq!(buf.get(19, 4).unwrap().glyph, '►');
        assert_eq!(buf.get(22, 4).unwrap().glyph, '◄');
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

        #[test]
        fn brightness_stays_between_min_and_full(t in -10.0f32..10.0) {
            let mut c = Cursor::new(Pos::new(0, 0));
            c.tick(t);
            let b = c.brightness();
            prop_assert!((BLINK_MIN - 1e-6..=1.0 + 1e-6).contains(&b), "{b}");
        }
    }
}
