//! The movement path (ADR-0018, `docs/design/look-and-feel.md`): how it
//! follows the cursor. A map skin draws it ([`crate::map_view`]).

use trpg_core::{Pos, Reach};

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

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_core::{BattleMap, BattleState, Grid, UnitId, path_cost, reachable};

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::{quick_battle, testing};

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
}
