//! A map skin: how a [`MapScene`] looks (ADR-0038).

use trpg_core::Pos;

use super::scene::MapScene;
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{GlyphBuffer, PxRect, Rect};
use crate::screen::Ctx;

/// Paints battle maps. The tile size is the skin's: screens ask it how many
/// tiles fit and where a tile is. A skin never changes the game: only the
/// frame, and how many tiles are on screen.
pub trait MapSkin: std::fmt::Debug {
    /// A stable `snake_case` name for the kind of skin, e.g. `"glyph"`.
    fn name(&self) -> &'static str;

    /// The id of the tileset it paints from, if it paints from one.
    fn tileset_id(&self) -> Option<&str> {
        None
    }

    /// How many tiles, across × down, fit in a map area of `area` cells.
    fn view_tiles(&self, area: Rect) -> (i32, i32);

    /// Paints `scene` into the cells of `area`. Touches nothing outside it.
    fn paint(&self, ctx: &Ctx, scene: &MapScene, area: Rect, buf: &mut GlyphBuffer);

    /// The console pixels of `tile` when `scene` is painted into `area`, or
    /// `None` if it isn't visible. Menus and popups go beside it.
    fn tile_px(&self, scene: &MapScene, area: Rect, tile: Pos) -> Option<PxRect>;

    /// The cells [`tile_px`](Self::tile_px) touches.
    fn tile_cells(&self, scene: &MapScene, area: Rect, tile: Pos) -> Option<Rect> {
        let px = self.tile_px(scene, area, tile)?;
        let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
        let (x, y) = (px.x.div_euclid(cw), px.y.div_euclid(ch));
        let right = (px.x + px.w + cw - 1).div_euclid(cw);
        let bottom = (px.y + px.h + ch - 1).div_euclid(ch);
        Some(Rect::new(x, y, right - x, bottom - y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A skin whose every tile is the pixel rectangle it was built with.
    #[derive(Debug)]
    struct Fixed(Option<PxRect>);

    impl MapSkin for Fixed {
        fn name(&self) -> &'static str {
            "fixed"
        }
        fn view_tiles(&self, _: Rect) -> (i32, i32) {
            (1, 1)
        }
        fn paint(&self, _: &Ctx, _: &MapScene, _: Rect, _: &mut GlyphBuffer) {}
        fn tile_px(&self, _: &MapScene, _: Rect, _: Pos) -> Option<PxRect> {
            self.0
        }
    }

    fn cells(px: Option<PxRect>) -> Option<Rect> {
        let scene = MapScene::new(Pos::new(0, 0), (1, 1));
        Fixed(px).tile_cells(&scene, Rect::new(0, 0, 2, 1), Pos::new(0, 0))
    }

    #[test]
    fn tile_cells_are_the_cells_the_tile_touches() {
        let r = Rect::new;
        // Cells are 8 × 16 px. A glyph tile: two whole cells.
        assert_eq!(cells(Some(r(160, 176, 16, 16))), Some(r(20, 11, 2, 1)));
        // One pixel into the next cell, each way.
        assert_eq!(cells(Some(r(160, 176, 17, 17))), Some(r(20, 11, 3, 2)));
        assert_eq!(cells(Some(r(159, 175, 16, 16))), Some(r(19, 10, 3, 2)));
        // A 32 px tile: four cells by two.
        assert_eq!(cells(Some(r(32, 32, 32, 32))), Some(r(4, 2, 4, 2)));
        // Left of and above the console.
        assert_eq!(cells(Some(r(-16, -16, 16, 16))), Some(r(-2, -1, 2, 1)));
        assert_eq!(cells(Some(r(-1, -1, 2, 2))), Some(r(-1, -1, 2, 2)));
        assert_eq!(cells(None), None);
        assert_eq!(Fixed(None).tileset_id(), None);
    }
}
