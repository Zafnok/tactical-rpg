//! Where the parts of the battle screen go on the 100 × 32 console
//! (ADR-0018, `docs/design/look-and-feel.md`).

use crate::glyph_buffer::Rect;

/// The map viewport, in cells: the left 70 columns, rows `0..30`. How many
/// tiles it shows is the map skin's to say (ADR-0038).
pub const MAP_VIEW: Rect = Rect::new(0, 0, 70, 30);

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
        assert_eq!(MAP_VIEW.x + MAP_VIEW.w, SIDE_PANEL.x);
        assert_eq!(SIDE_PANEL.x + SIDE_PANEL.w, i32::from(CONSOLE_W));
        assert_eq!(MAP_VIEW.h, SIDE_PANEL.h);
        assert_eq!(HELP_BAR.y, MAP_VIEW.h);
        assert_eq!(HELP_BAR.y + HELP_BAR.h, i32::from(CONSOLE_H));
        assert_eq!(HELP_BAR.w, i32::from(CONSOLE_W));
        assert_eq!(HELP_ROW, 31);
    }
}
