//! Where a scene's tiles go on the console, in pixels: what every map skin
//! works out from its own tile size, and the shared drawing (the path)
//! works from.

use trpg_core::Pos;

use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{PxRect, Rect};

/// `cells` in console pixels.
pub fn px_rect(cells: Rect) -> PxRect {
    let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
    Rect::new(cells.x * cw, cells.y * ch, cells.w * cw, cells.h * ch)
}

/// The tiles of a scene laid out on the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// The map tile in the top-left corner.
    pub origin: Pos,
    /// The top-left pixel of that tile.
    pub corner: (i32, i32),
    /// A tile's size in pixels, across × down.
    pub tile: (i32, i32),
    /// Tiles drawn, across × down.
    pub tiles: (i32, i32),
}

impl Grid {
    /// Where `tile` is, as tiles right of and below the origin, if drawn.
    pub fn offset(&self, tile: Pos) -> Option<(i32, i32)> {
        let dx = tile.x.checked_sub(self.origin.x)?;
        let dy = tile.y.checked_sub(self.origin.y)?;
        let inside = (0..self.tiles.0).contains(&dx) && (0..self.tiles.1).contains(&dy);
        inside.then_some((dx, dy))
    }

    /// The top-left pixel of `tile`, drawn or not (it may lie outside the
    /// grid).
    pub fn tile_px(&self, tile: Pos) -> (i32, i32) {
        let (dx, dy) = (tile.x - self.origin.x, tile.y - self.origin.y);
        (
            self.corner.0 + self.tile.0 * dx,
            self.corner.1 + self.tile.1 * dy,
        )
    }

    /// The pixels of the tile `dx` right of and `dy` below the origin.
    pub fn rect_at(&self, dx: i32, dy: i32) -> PxRect {
        let (w, h) = self.tile;
        Rect::new(self.corner.0 + w * dx, self.corner.1 + h * dy, w, h)
    }

    /// The pixels of `tile`, or `None` if it isn't drawn.
    pub fn rect(&self, tile: Pos) -> Option<PxRect> {
        let (dx, dy) = self.offset(tile)?;
        Some(self.rect_at(dx, dy))
    }

    /// The pixels every drawn tile covers.
    pub fn bounds(&self) -> PxRect {
        let (w, h) = self.tile;
        let (x, y) = self.corner;
        Rect::new(x, y, w * self.tiles.0, h * self.tiles.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn tiles_are_laid_from_the_corner_in_their_size() {
        let g = Grid {
            origin: p(5, -2),
            corner: (4, 8),
            tile: (24, 20),
            tiles: (3, 2),
        };
        assert_eq!(g.offset(p(5, -2)), Some((0, 0)));
        assert_eq!(g.offset(p(7, -1)), Some((2, 1)));
        for out in [
            p(8, -2),
            p(4, -2),
            p(5, 0),
            p(5, -3),
            p(i32::MIN, 0),
            p(0, i32::MIN),
        ] {
            assert_eq!(g.offset(out), None, "{out:?}");
            assert_eq!(g.rect(out), None, "{out:?}");
        }
        assert_eq!(g.tile_px(p(5, -2)), (4, 8));
        assert_eq!(g.tile_px(p(7, -1)), (52, 28));
        // Outside the grid too.
        assert_eq!(g.tile_px(p(4, -3)), (-20, -12));
        assert_eq!(g.rect(p(6, -1)), Some(Rect::new(28, 28, 24, 20)));
        assert_eq!(g.rect_at(2, 0), Rect::new(52, 8, 24, 20));
        assert_eq!(g.bounds(), Rect::new(4, 8, 72, 40));
    }
}
