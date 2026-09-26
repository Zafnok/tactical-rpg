//! Where the parts of the battle screen go on the 100 × 32 console
//! (ADR-0018, `docs/design/look-and-feel.md`).

use crate::glyph_buffer::Rect;

/// Cells per map tile, across (a tile is two glyphs wide, ADR-0018).
pub const TILE_W_CELLS: i32 = 2;

/// The map viewport, in cells: the left 70 columns, rows `0..30`.
pub const MAP_VIEW: Rect = Rect::new(0, 0, 70, 30);

/// Viewport width in tiles (35).
pub const VIEW_TILES_W: i32 = MAP_VIEW.w / TILE_W_CELLS;

/// Viewport height in tiles (30).
pub const VIEW_TILES_H: i32 = MAP_VIEW.h;

/// The side panel (single-line box), right of the map.
pub const SIDE_PANEL: Rect = Rect::new(70, 0, 30, 30);

/// The two-row message and key-help bar under the map and panel.
pub const HELP_BAR: Rect = Rect::new(0, 30, 100, 2);

/// Row of the key-help line (the bar's second row; the first is for
/// messages).
pub const HELP_ROW: i32 = HELP_BAR.y + 1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};

    #[test]
    fn regions_tile_the_console() {
        assert_eq!((VIEW_TILES_W, VIEW_TILES_H), (35, 30));
        assert_eq!(MAP_VIEW.x + MAP_VIEW.w, SIDE_PANEL.x);
        assert_eq!(SIDE_PANEL.x + SIDE_PANEL.w, i32::from(CONSOLE_W));
        assert_eq!(MAP_VIEW.h, SIDE_PANEL.h);
        assert_eq!(HELP_BAR.y, MAP_VIEW.h);
        assert_eq!(HELP_BAR.y + HELP_BAR.h, i32::from(CONSOLE_H));
        assert_eq!(HELP_BAR.w, i32::from(CONSOLE_W));
        assert_eq!(HELP_ROW, 31);
    }
}
