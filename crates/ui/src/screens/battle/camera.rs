//! The battle camera: which map tiles the viewport shows, and where a tile
//! lands on the console.

use trpg_core::Pos;

use super::layout::{MAP_VIEW, TILE_W_CELLS, VIEW_TILES_H, VIEW_TILES_W};

/// The map tile shown in the viewport's top-left corner. It can be negative
/// (or run past the map) when the map is smaller than the viewport: the map
/// is then centred and the cells around it are left blank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Camera {
    /// Top-left tile of the viewport.
    pub origin: Pos,
}

impl Camera {
    /// Default [`follow`](Self::follow) margin, in tiles.
    pub const MARGIN: i32 = 3;

    /// A camera with `target` as near the viewport's centre as the map
    /// allows, on a `map_w × map_h` map.
    pub fn centred_on(target: Pos, map_w: u16, map_h: u16) -> Self {
        let origin = Pos::new(
            clamp_axis(target.x - VIEW_TILES_W / 2, map_w, VIEW_TILES_W),
            clamp_axis(target.y - VIEW_TILES_H / 2, map_h, VIEW_TILES_H),
        );
        Self { origin }
    }

    /// Scrolls as little as possible so `target` is at least `margin` tiles
    /// from each viewport edge, without showing past the map's edges. On an
    /// axis where the map is smaller than the viewport, the map is centred.
    /// A margin over half the viewport is treated as half.
    pub fn follow(&mut self, target: Pos, map_w: u16, map_h: u16, margin: i32) {
        self.origin = Pos::new(
            follow_axis(self.origin.x, target.x, map_w, VIEW_TILES_W, margin),
            follow_axis(self.origin.y, target.y, map_h, VIEW_TILES_H, margin),
        );
    }
}

/// One axis of [`Camera::follow`].
fn follow_axis(origin: i32, target: i32, map_len: u16, view: i32, margin: i32) -> i32 {
    let margin = margin.clamp(0, (view - 1) / 2);
    let lowest = target - (view - 1 - margin);
    let highest = target - margin;
    clamp_axis(origin.clamp(lowest, highest), map_len, view)
}

/// Keeps `origin` within the map (`0..=map_len - view`), or centres a map
/// shorter than the viewport.
fn clamp_axis(origin: i32, map_len: u16, view: i32) -> i32 {
    let map_len = i32::from(map_len);
    if map_len <= view {
        -((view - map_len) / 2)
    } else {
        origin.clamp(0, map_len - view)
    }
}

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
    use proptest::prelude::*;

    use super::*;

    fn cam(x: i32, y: i32) -> Camera {
        Camera {
            origin: Pos::new(x, y),
        }
    }

    fn followed(from: Camera, target: Pos, w: u16, h: u16) -> Pos {
        let mut c = from;
        c.follow(target, w, h, Camera::MARGIN);
        c.origin
    }

    #[test]
    fn small_map_is_centred() {
        // 14 × 8: (35 − 14) / 2 = 10 blank tiles left, (30 − 8) / 2 = 11 above.
        for target in [Pos::new(0, 0), Pos::new(13, 7), Pos::new(6, 3)] {
            assert_eq!(followed(cam(0, 0), target, 14, 8), Pos::new(-10, -11));
            assert_eq!(followed(cam(5, 9), target, 14, 8), Pos::new(-10, -11));
        }
        assert_eq!(Camera::centred_on(Pos::new(0, 0), 14, 8), cam(-10, -11));
        // Exactly viewport-sized: origin 0.
        assert_eq!(
            followed(cam(3, 3), Pos::new(34, 29), 35, 30),
            Pos::new(0, 0)
        );
        // One axis small, the other large.
        assert_eq!(
            followed(cam(0, 0), Pos::new(0, 50), 34, 64),
            Pos::new(0, 24)
        );
    }

    #[test]
    fn large_map_clamps_at_every_edge() {
        let (w, h) = (64, 40);
        // Top-left.
        assert_eq!(followed(cam(10, 5), Pos::new(0, 0), w, h), Pos::new(0, 0));
        // Bottom-right: origin = size − view.
        assert_eq!(
            followed(cam(0, 0), Pos::new(63, 39), w, h),
            Pos::new(29, 10)
        );
        // Top-right and bottom-left.
        assert_eq!(followed(cam(0, 5), Pos::new(63, 0), w, h), Pos::new(29, 0));
        assert_eq!(followed(cam(20, 0), Pos::new(0, 39), w, h), Pos::new(0, 10));
        // An origin already out of range is pulled back in.
        assert_eq!(
            followed(cam(-5, 99), Pos::new(10, 20), w, h),
            Pos::new(0, 10)
        );
        assert_eq!(Camera::centred_on(Pos::new(63, 39), w, h), cam(29, 10));
        assert_eq!(Camera::centred_on(Pos::new(0, 0), w, h), cam(0, 0));
        assert_eq!(Camera::centred_on(Pos::new(32, 20), w, h), cam(15, 5));
    }

    #[test]
    fn margin_is_respected() {
        let (w, h) = (64, 64);
        let start = cam(10, 10);
        // Inside the margins: no scrolling.
        assert_eq!(followed(start, Pos::new(13, 13), w, h), Pos::new(10, 10));
        assert_eq!(followed(start, Pos::new(41, 36), w, h), Pos::new(10, 10));
        // One tile into a margin scrolls by one.
        assert_eq!(followed(start, Pos::new(12, 12), w, h), Pos::new(9, 9));
        assert_eq!(followed(start, Pos::new(42, 37), w, h), Pos::new(11, 11));
        // Far away: jumps so the target sits exactly at the margin.
        assert_eq!(followed(start, Pos::new(60, 3), w, h), Pos::new(29, 0));
        // Custom and oversized margins.
        let mut c = start;
        c.follow(Pos::new(10, 10), w, h, 0);
        assert_eq!(c.origin, Pos::new(10, 10));
        c.follow(Pos::new(30, 30), w, h, 99);
        // Clamped to half the view (17, 14): the least scroll that centres it.
        assert_eq!(c.origin, Pos::new(13, 15));
        c.follow(Pos::new(30, 30), w, h, -4);
        assert_eq!(c.origin, Pos::new(13, 15));
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

    proptest! {
        #[test]
        fn follow_keeps_target_visible_and_map_filling_the_view(
            ox in -100..100i32, oy in -100..100i32,
            tx in 0..64i32, ty in 0..64i32,
            w in 1u16..65, h in 1u16..65, margin in 0..10i32,
        ) {
            let target = Pos::new(tx.min(i32::from(w) - 1), ty.min(i32::from(h) - 1));
            let mut c = cam(ox, oy);
            c.follow(target, w, h, margin);
            prop_assert!(tile_to_cell(target, &c).is_some());
            for (o, len, view) in [(c.origin.x, w, VIEW_TILES_W), (c.origin.y, h, VIEW_TILES_H)] {
                let len = i32::from(len);
                if len > view {
                    prop_assert!(o >= 0 && o + view <= len);
                } else {
                    prop_assert_eq!(o, -((view - len) / 2));
                }
            }
        }
    }
}
