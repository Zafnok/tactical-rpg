//! The movement path arrow (ADR-0018, `docs/design/look-and-feel.md`): how
//! it follows the cursor, and how it is drawn: a 3-px line through tile
//! centres, under the glyphs, from the edge of the unit's tile, ending in a
//! single arrowhead over the destination tile.

use trpg_core::{Pos, Reach};

use super::camera::Camera;
use super::layout::{MAP_VIEW, TILE_W_CELLS};
use crate::color::Rgb;
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{Layer, Overlay, PxRect, Rect};

/// A tile's size in pixels (square).
pub const TILE_PX: i32 = 16;

/// The path line's thickness, in pixels.
pub const LINE_W: i32 = 3;

/// Offset of the line from a tile's left (or top) edge: pixels 7..=9 of 16.
const LINE_OFFSET: i32 = 7;

/// The arrowhead's columns (or rows), from its base (11 px across) to its
/// tip (1 px).
const ARROW_LEN: i32 = 6;

/// Offset of the arrowhead's base from the tile's edge behind it.
const ARROW_BASE: i32 = 5;

/// The path after the cursor moves to `to`. `path` runs from the unit's
/// tile (never empty); `valid` says whether a path is legal (in a battle:
/// [`trpg_core::path_cost`] is `Ok`).
///
/// - `to` already on the path: the path is cut back to it.
/// - `to` next to the path's end and the longer path is legal: `to` is
///   appended, so the player can steer a route.
/// - Otherwise the cheapest path to `to` ([`Reach::path_to`]), or the path
///   unchanged if `to` can't be reached.
pub fn steer(path: &[Pos], to: Pos, reach: &Reach, valid: impl Fn(&[Pos]) -> bool) -> Vec<Pos> {
    if let Some(i) = path.iter().position(|&p| p == to) {
        return path[..=i].to_vec();
    }
    if path.last().is_some_and(|&end| Pos::manhattan(end, to) == 1) {
        let mut longer = path.to_vec();
        longer.push(to);
        if valid(&longer) {
            return longer;
        }
    }
    reach.path_to(to).unwrap_or_else(|| path.to_vec())
}

/// The top-left pixel of `tile` on the console with `camera` (it may lie
/// outside the viewport).
fn tile_px(tile: Pos, camera: Camera) -> (i32, i32) {
    let cx = MAP_VIEW.x + TILE_W_CELLS * (tile.x - camera.origin.x);
    let cy = MAP_VIEW.y + (tile.y - camera.origin.y);
    (cx * i32::from(CELL_W_PX), cy * i32::from(CELL_H_PX))
}

/// The map viewport in pixels: path overlays are clipped to it.
fn view_px() -> PxRect {
    let (w, h) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
    Rect::new(
        MAP_VIEW.x * w,
        MAP_VIEW.y * h,
        MAP_VIEW.w * w,
        MAP_VIEW.h * h,
    )
}

/// The line between the centres of adjacent tiles `a` and `b`; with
/// `from_edge`, only the part outside `a`'s tile.
fn segment(a: Pos, b: Pos, camera: Camera, from_edge: bool) -> PxRect {
    let ((ax, ay), (bx, by)) = (tile_px(a, camera), tile_px(b, camera));
    let (mut x0, mut y0) = (ax.min(bx) + LINE_OFFSET, ay.min(by) + LINE_OFFSET);
    let (mut x1, mut y1) = (
        ax.max(bx) + LINE_OFFSET + LINE_W,
        ay.max(by) + LINE_OFFSET + LINE_W,
    );
    if from_edge {
        match (b.x - a.x, b.y - a.y) {
            (1, _) => x0 = ax + TILE_PX,
            (-1, _) => x1 = ax,
            (_, 1) => y0 = ay + TILE_PX,
            _ => y1 = ay,
        }
    }
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

/// The arrowhead on tile `to`, pointing away from the adjacent tile `from`:
/// [`ARROW_LEN`] stacked 1-px rects, 11 px across at the base down to 1 at
/// the tip, centred on the line.
fn arrowhead(from: Pos, to: Pos, camera: Camera) -> Vec<PxRect> {
    let (px, py) = tile_px(to, camera);
    let mid = LINE_OFFSET + LINE_W / 2;
    (0..ARROW_LEN)
        .map(|i| {
            let half = ARROW_LEN - 1 - i;
            let across = 2 * half + 1;
            let near = ARROW_BASE + i;
            let far = TILE_PX - 1 - ARROW_BASE - i;
            match (to.x - from.x, to.y - from.y) {
                (1, _) => Rect::new(px + near, py + mid - half, 1, across),
                (-1, _) => Rect::new(px + far, py + mid - half, 1, across),
                (_, 1) => Rect::new(px + mid - half, py + near, across, 1),
                _ => Rect::new(px + mid - half, py + far, across, 1),
            }
        })
        .collect()
}

/// The overlays drawing `path` (the unit's tile first) in `color`: nothing
/// for a path of one tile. Clipped to the map viewport.
pub fn path_overlays(path: &[Pos], camera: Camera, color: Rgb) -> Vec<Overlay> {
    let view = view_px();
    let mut out = Vec::new();
    let mut add = |rect: PxRect, layer| {
        if let Some(rect) = rect.intersect(&view) {
            out.push(Overlay::new(rect, color, layer));
        }
    };
    for (i, pair) in path.windows(2).enumerate() {
        add(segment(pair[0], pair[1], camera, i == 0), Layer::Under);
    }
    if let [.., from, to] = path {
        for rect in arrowhead(*from, *to, camera) {
            add(rect, Layer::Over);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_core::{BattleMap, BattleState, Grid, UnitId, path_cost, reachable};

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::{quick_battle, testing};

    const RED: Rgb = Rgb::new(255, 0, 0);

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// The Quick Battle's units on `map`: the lord is unit 1, Mov 5.
    fn battle(map: BattleMap) -> BattleState {
        let c = ctx();
        let units = quick_battle(&c.content).unwrap().units().to_vec();
        testing::battle(&c, map, units)
    }

    fn quick_map() -> BattleMap {
        quick_battle(&ctx().content).unwrap().map().clone()
    }

    /// `path` steered to each of `cursor` in turn, for unit 1 of `s`.
    fn steered(s: &BattleState, cursor: &[Pos]) -> Vec<Pos> {
        let id = UnitId(1);
        let reach = reachable(s.map(), s.terrain(), s.classes(), s.units(), id).unwrap();
        let valid = |path: &[Pos]| {
            path_cost(s.map(), s.terrain(), s.classes(), s.units(), id, path).is_ok()
        };
        let mut path = vec![reach.origin()];
        for &to in cursor {
            path = steer(&path, to, &reach, valid);
        }
        path
    }

    #[test]
    fn steering_appends_cuts_back_and_falls_back_to_the_cheapest_path() {
        let plain = ctx().content.terrain.display.id_of("plain").unwrap();
        let s = battle(BattleMap::new("Open", Grid::filled(14, 8, plain)));
        let lord = p(3, 5);
        // Down, right, right, up: a detour the cheapest path wouldn't take.
        let walk = [p(3, 6), p(4, 6), p(5, 6), p(5, 5)];
        // (4, 6) holds the knight: passable, so the path goes through it.
        assert_eq!(
            steered(&s, &walk),
            [lord, p(3, 6), p(4, 6), p(5, 6), p(5, 5)]
        );
        // Back onto the path: cut back to that tile.
        let mut back = walk.to_vec();
        back.push(p(4, 6));
        assert_eq!(steered(&s, &back), [lord, p(3, 6), p(4, 6)]);
        // Back onto the unit: just its tile.
        back.push(p(3, 6));
        back.push(lord);
        assert_eq!(steered(&s, &back), [lord]);
        // Too long to extend (Mov 5): the cheapest path instead.
        let far = [p(3, 4), p(3, 3), p(4, 3), p(5, 3), p(6, 3), p(6, 4)];
        let reach = reachable(s.map(), s.terrain(), s.classes(), s.units(), UnitId(1)).unwrap();
        assert_eq!(reach.budget(), 5);
        let cheapest = reach.path_to(p(6, 4)).unwrap();
        assert_eq!(cheapest.len(), 5);
        assert_eq!(steered(&s, &far), cheapest);
        // Unreachable (7 tiles away): the path stays as it was.
        assert_eq!(steered(&s, &[p(4, 5), p(10, 5)]), [lord, p(4, 5)]);
    }

    proptest! {
        /// Whatever the cursor does, the path stays legal, and it ends on
        /// the cursor whenever the cursor is on a tile the unit can reach.
        #[test]
        fn steered_paths_are_legal_and_follow_the_cursor(
            moves in prop::collection::vec(0..4usize, 0..40),
        ) {
            let s = battle(quick_map());
            let id = UnitId(1);
            let reach = reachable(s.map(), s.terrain(), s.classes(), s.units(), id).unwrap();
            let cost = |path: &[Pos]| {
                path_cost(s.map(), s.terrain(), s.classes(), s.units(), id, path)
            };
            let (w, h) = (i32::from(s.map().tiles.width()), i32::from(s.map().tiles.height()));
            let mut cursor = reach.origin();
            let mut path = vec![cursor];
            for m in moves {
                let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][m];
                cursor = p((cursor.x + dx).clamp(0, w - 1), (cursor.y + dy).clamp(0, h - 1));
                path = steer(&path, cursor, &reach, |q| cost(q).is_ok());
                prop_assert!(cost(&path).is_ok(), "{path:?}");
                if reach.is_passable(cursor) {
                    prop_assert_eq!(path.last(), Some(&cursor));
                }
            }
        }
    }

    /// The overlays of `path` with the camera at the origin, as
    /// `(layer, x, y, w, h)`.
    fn rects(path: &[Pos]) -> Vec<(Layer, i32, i32, i32, i32)> {
        path_overlays(path, Camera::default(), RED)
            .iter()
            .map(|o| (o.layer, o.rect.x, o.rect.y, o.rect.w, o.rect.h))
            .collect()
    }

    #[test]
    fn a_one_tile_path_draws_nothing() {
        assert!(rects(&[p(2, 2)]).is_empty());
    }

    #[test]
    fn the_line_starts_at_the_unit_tile_edge_and_ends_in_one_arrowhead() {
        // Right from (1, 1) (pixels 16..32 × 16..32) to (2, 1), then down.
        let r = rects(&[p(1, 1), p(2, 1), p(2, 2)]);
        let under: Vec<_> = r.iter().filter(|o| o.0 == Layer::Under).collect();
        assert_eq!(
            under,
            [
                // From the tile's right edge (x 32) to (2, 1)'s centre band.
                &(Layer::Under, 32, 23, 10, 3),
                // (2, 1)'s centre band down to (2, 2)'s.
                &(Layer::Under, 39, 23, 3, 19),
            ]
        );
        // The arrowhead points down on (2, 2) (pixels 32..48 × 32..48):
        // rows 37..=42, 11 px wide down to 1, centred on x 40.
        let over: Vec<_> = r.iter().filter(|o| o.0 == Layer::Over).copied().collect();
        let expect: Vec<_> = (0..6)
            .map(|i| (Layer::Over, 40 - (5 - i), 37 + i, 2 * (5 - i) + 1, 1))
            .collect();
        assert_eq!(over, expect);
    }

    #[test]
    fn arrowheads_point_the_way_of_the_last_step() {
        let tips = |path: &[Pos]| {
            let over: Vec<_> = rects(path)
                .into_iter()
                .filter(|o| o.0 == Layer::Over)
                .collect();
            // The 1-px tip is the last rect.
            let (_, x, y, w, h) = over[over.len() - 1];
            assert_eq!((w, h), (1, 1));
            (x, y)
        };
        // Tile (1, 1) spans pixels 16..32 on both axes; its centre is 24.
        assert_eq!(tips(&[p(0, 1), p(1, 1)]), (16 + 10, 24));
        assert_eq!(tips(&[p(2, 1), p(1, 1)]), (16 + 5, 24));
        assert_eq!(tips(&[p(1, 0), p(1, 1)]), (24, 16 + 10));
        assert_eq!(tips(&[p(1, 2), p(1, 1)]), (24, 16 + 5));
        // Each line starts at the unit tile's edge facing the move.
        let first = |path: &[Pos]| rects(path)[0];
        assert_eq!(first(&[p(1, 1), p(0, 1)]), (Layer::Under, 7, 23, 9, 3));
        assert_eq!(first(&[p(1, 1), p(1, 0)]), (Layer::Under, 23, 7, 3, 9));
        assert_eq!(first(&[p(1, 1), p(1, 2)]), (Layer::Under, 23, 32, 3, 10));
    }

    #[test]
    fn overlays_are_clipped_to_the_map_view() {
        // The viewport is 35 tiles wide: tile 35 is under the side panel.
        let r = rects(&[p(33, 0), p(34, 0), p(35, 0)]);
        let right = MAP_VIEW.w * i32::from(CELL_W_PX);
        assert!(r.iter().all(|o| o.1 + o.3 <= right), "{r:?}");
        assert_eq!(r.len(), 2, "no arrowhead off the view: {r:?}");
    }
}
