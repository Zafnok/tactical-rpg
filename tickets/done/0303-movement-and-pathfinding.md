---
id: "0303"
title: Movement range, pathfinding and attack/threat ranges
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: done
blocked_by: ["0302"]
nick_input: none
completed: 2026-09-26
---

# 0303 — Movement range, pathfinding, attack and threat ranges

## Context

Powers the blue movement overlay, red attack overlay, path arrow, enemy danger
zone and the AI. Pure functions in `trpg-core`. Must be exhaustively tested:
bugs here are the kind players notice instantly.

## Nick input

None.

## Scope

**In:** `core::movement` — reachable tiles, path reconstruction, path cost
validation, attack tiles, threat area.

**Out:** Canto/split movement, flying-over-units, ZOC (only if the design docs
from 0002 demand them — then implement them here and say so).

## Implementation steps

1. `reachable(map, terrain, units: &[Unit], mover: UnitId) -> Reach`:
   - Dijkstra from the mover's position with budget = class move points;
     step cost = terrain `move_cost[movement_type]` of the tile entered;
     `None` = impassable.
   - Can pass through units that are **not hostile** to the mover; cannot
     enter tiles with hostile units.
   - Can't **end** on a tile occupied by another unit (can still pass through
     friends). `Reach` distinguishes `passable` tiles (with cost) and `stoppable`
     tiles (subset where the unit can end its move; always includes start).
   - Deterministic: ties broken by `Dir` order (0301) and a stable priority queue
     key `(cost, y, x)`.
   - `Reach` stores `cost` and `prev` per tile (`Grid<Option<…>>`).
2. `Reach::path_to(dest) -> Option<Vec<Pos>>` start→dest inclusive.
3. `path_cost(map, terrain, units, mover, path) -> Result<u32, PathError>` —
   validates a UI-drawn path (adjacent steps, passable, no hostile tiles,
   within budget). The UI (0403) uses this for the "arrow follows cursor" path.
4. `attack_tiles(reach, min_range, max_range) -> TileSet`: all tiles at
   Manhattan distance in `[min, max]` from any `stoppable` tile, excluding
   stoppable tiles themselves (FE overlay semantics). Ranges come from the
   unit's usable weapons (take the union; 0306 will supply real weapons — for
   now take `(min, max)` as parameters).
5. `threat_area(map, terrain, units, unit, ranges) -> TileSet` =
   `stoppable ∪ attack_tiles` for that unit; `danger_zone(…, faction)` =
   union over all units hostile to `faction`.
6. `TileSet`: a bitset over the grid (`Vec<u64>` or `Grid<bool>`), with iteration in row-major order.
7. Document the rules at the top of `movement.rs` in a doc comment.

## Acceptance criteria

- [x] All property tests below pass with ≥ 1000 cases each.
- [x] Hand-made scenario tests (ASCII-drawn in the test source) for: wall
      blocking, forest cost, water impassable for foot, passing through an ally,
      blocked by an enemy, can't stop on ally, flying ignores terrain costs (if the
      terrain data says flying cost is 1 everywhere).
- [x] Performance: `reachable` on a 64×64 open map with move 10 in < 1 ms in release (a simple timing test marked `#[ignore]` or a note with measurement).

## Tests required

- Property (random maps + unit placements):
  - every passable tile's cost ≤ move points;
  - `path_to(t)` is contiguous, starts at the unit, ends at `t`, sums to `cost(t)`;
  - no path enters a hostile unit's tile or an impassable tile;
  - `stoppable ⊆ passable`, and no stoppable tile (except start) holds a unit;
  - monotonic: increasing move never removes a tile;
  - `path_cost(path_to(t)) == cost(t)`.
- Unit: the scenario list above; `attack_tiles` for ranges 1, 2, 1–2.

## Completion notes

- Added `trpg_core::movement` (`crates/core/src/movement.rs`, rules in its
  module doc; tests in `movement/tests.rs`): `reachable` → `Reach` (cost +
  prev grids, stoppable `TileSet`, `path_to`), `path_cost`, `attack_tiles`,
  `threat_area`, `danger_zone`, and the `TileSet` bitset. Added
  `Grid::filled` and `Grid::get_mut` to `geom`.
- **Design check:** `turn-structure.md` rules out Canto and split movement;
  no design doc asks for zones of control or flying over units, so none are
  implemented.
- **Deviations:**
  - The functions also take `&ClassTable`, because a unit's movement type
    lives on its class. They return `Result<_, MoveError>` (unknown unit,
    unknown class, unit off the map) instead of panicking.
  - The budget is the unit's `stats.mov`, which the class sets (equal to the
    class's move points today), so later Mov modifiers work without changes.
    A negative Mov counts as 0.
  - `threat_area` with no weapon ranges is empty: a unit that can't attack
    threatens nothing (instead of showing its move range as danger).
  - `path_cost` doesn't check that the path *ends* on a stoppable tile; the
    arrow may point at any passable tile and the UI decides.
  - The ticket's "water impassable for foot" scenario follows `terrain.md`:
    **sea** is impassable for foot; **rivers** can be waded on foot (cost 5).
    Both are tested, plus mounted/armored/flying costs.
- **Tests:** ASCII scenario tests for every listed case; 8 property tests at
  1000 cases each, including a brute-force (Bellman–Ford) oracle for costs
  and a brute-force check of `attack_tiles`.
- **Performance:** `reachable` on a 64×64 open map with Mov 10 takes about
  30 µs in release (`cargo test -p trpg-core --release -- --ignored
  reachable_is_fast`), well under 1 ms.
- Nothing for Nick to see yet: the overlays are drawn by 0403.
