//! The glyph skin: the battle map drawn with coloured glyphs (ADR-0018).

pub mod cursor;
pub mod path;
pub mod units;

use trpg_core::Pos;

use crate::screens::battle::camera::Camera;
use crate::screens::battle::layout::{MAP_VIEW, TILE_W_CELLS, VIEW_TILES_H, VIEW_TILES_W};

/// The console cell of the left glyph of `tile`, or `None` if the tile is
/// outside the viewport. The only place the "two cells per tile" rule lives
/// (ADR-0018).
pub fn tile_to_cell(tile: Pos, camera: &Camera) -> Option<(i32, i32)> {
    let dx = tile.x.checked_sub(camera.origin.x)?;
    let dy = tile.y.checked_sub(camera.origin.y)?;
    let inside = (0..VIEW_TILES_W).contains(&dx) && (0..VIEW_TILES_H).contains(&dy);
    inside.then(|| (MAP_VIEW.x + TILE_W_CELLS * dx, MAP_VIEW.y + dy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam(x: i32, y: i32) -> Camera {
        Camera {
            origin: Pos::new(x, y),
        }
    }

    #[test]
    fn tile_to_cell_maps_two_cells_per_tile() {
        let c = cam(5, 2);
        assert_eq!(tile_to_cell(Pos::new(5, 2), &c), Some((0, 0)));
        assert_eq!(tile_to_cell(Pos::new(6, 3), &c), Some((2, 1)));
        assert_eq!(tile_to_cell(Pos::new(39, 31), &c), Some((68, 29)));
        assert_eq!(tile_to_cell(Pos::new(40, 2), &c), None);
        assert_eq!(tile_to_cell(Pos::new(5, 32), &c), None);
        assert_eq!(tile_to_cell(Pos::new(4, 2), &c), None);
        assert_eq!(tile_to_cell(Pos::new(5, 1), &c), None);
        let small = cam(-10, -11);
        assert_eq!(tile_to_cell(Pos::new(0, 0), &small), Some((20, 11)));
        assert_eq!(tile_to_cell(Pos::new(i32::MIN, 0), &c), None);
        assert_eq!(tile_to_cell(Pos::new(0, i32::MIN), &c), None);
    }
}
