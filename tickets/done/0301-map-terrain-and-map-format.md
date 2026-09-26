---
id: "0301"
title: Battle map model, terrain table and the .map file format
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: done
blocked_by: ["0001", "0201"]
nick_input: none
completed: 2026-09-26
---

# 0301 — Battle map, terrain, `.map` format

## Context

The grid everything happens on. Rules parts (move costs, defence/avoid
bonuses) go in `trpg-core`; display parts (glyphs, colours) and file parsing in
`trpg-content` ([ADR-0004](../../docs/adr/0004-crate-architecture.md),
[ADR-0005](../../docs/adr/0005-data-driven-content.md)). Terrain combat
bonuses come from `docs/design/stats-and-combat.md` (ticket 0001).

## Nick input

None (terrain numbers come from 0001).

## Scope

**In:** `core::geom` (`Pos`, `Grid<T>`), `core::terrain`, `core::map`,
`assets/data/terrain.ron`, `.map` parser/validator/printer in `content`, one
small test map.

**Out:** units (0302), pathfinding (0303), drawing the map (0401).

## Implementation steps

1. `core::geom`: `Pos { x: i32, y: i32 }` (signed so neighbour maths can go
   negative before bounds checks), `Pos::manhattan(a, b)`, `Dir` (4 dirs, fixed
   order Right, Down, Left, Up — used for deterministic iteration).
   `Grid<T> { width: u16, height: u16, cells: Vec<T> }` with `get(pos) -> Option<&T>`,
   `in_bounds`, `neighbors4(pos) -> impl Iterator<Item = Pos>` (in bounds, fixed order).
2. `core::terrain`:
   - `MovementTypeId(u8)`, `TerrainId(u16)`.
   - `TerrainRules { name: String, move_cost: Vec<Option<u8>> /* indexed by MovementTypeId; None = impassable */, defense: i8, avoid: i8, heal_percent: u8 }`.
   - `TerrainTable { movement_types: Vec<String>, terrains: Vec<TerrainRules> }` with lookup by id.
3. `core::map::BattleMap { name: String, tiles: Grid<TerrainId> }`.
4. `assets/data/terrain.ron`: `movement_types: ["foot", "mounted", "armored", "flying"]`
   (adjust if `docs/design/progression.md` exists and says otherwise), and
   terrains at least: `plain, road, forest, thicket (impassable woods), mountain,
   peak, water, sea, wall, fort, bridge, floor, door`. Each has: `id`,
   `name`, `glyphs: "..", fg: "<palette name>", bg: "<palette name>"`, costs per
   movement type, `defense`, `avoid`, `heal_percent`. **Use the terrain numbers
   from `docs/design/stats-and-combat.md`**; for terrain types it doesn't list,
   copy the closest FE value and add a `// TUNABLE` comment.
5. `content`: load `terrain.ron` → `(core::TerrainTable, TerrainDisplayTable)`;
   validate palette names exist (against `PaletteDef`), glyph strings are exactly
   2 chars, cost vectors match movement types, ids unique.
6. `.map` format (document it in `assets/maps/README.md`):
   ```
   (
     name: "Test Field",
     legend: { '.': "plain", 'T': "forest", '^': "mountain", '~': "water", '#': "wall", 'F': "fort", '=': "bridge" },
   )
   ---
   ....TT..~~..
   ..^^TT..==..
   ```
   RON header, a line `---`, then one character per tile (the 2-glyph look is
   rendering, not file format). Parser returns `BattleMap` + errors with exact
   line/column: unknown legend char, ragged rows, unknown terrain id in legend,
   empty map, size over 64×64.
   Also `print_map(&BattleMap, legend) -> String` for round-trip tests.
7. `assets/maps/test_small.map` (~12×8, uses every legend entry). Add maps and
   terrain to `Content` and the all-assets test.

## Acceptance criteria

- [x] Terrain values match `docs/design/stats-and-combat.md`.
- [x] Parser reports all errors with correct line/column (tests for each error kind).
- [x] `test_small.map` loads; all-assets test passes.
- [x] `core` has no knowledge of glyphs/colours/files.

## Tests required

- Unit: `Grid` bounds and neighbours ordering; each parser error; terrain validation errors.
- Property: for random grids over a legend, `parse(print(map)) == map`; `neighbors4` never yields out-of-bounds positions.

## Completion notes

- **core**: `geom` (`Pos`, `Dir` in fixed order Right/Down/Left/Up, `Grid<T>`
  with private fields so `cells.len() == width * height` always holds; built
  with `Grid::from_cells`), `terrain` (`MovementTypeId`, `TerrainId`,
  `TerrainRules`, `TerrainTable` with `get` / `movement_type` / `move_cost`),
  `map::BattleMap`. No glyphs, colours or files in `core`.
- **assets/data/terrain.ron**: movement types `foot, mounted, armored, flying`
  (as in `progression.md`). The 13 terrains the ticket lists (`fort` is the
  only building). Values pinned to `terrain.md` by
  `embedded_terrain_matches_design_doc` and
  `embedded_move_costs_match_design_doc`.
- **Nick revisited the player-facing choices** (recorded in the new
  `docs/design/terrain.md`): heavy FE GBA-style movement costs, but mountains
  cost 3 on foot and 5 mounted (horses can climb them now); foot units can
  wade rivers at 5; fliers pay 3 on peaks. The costs are pinned by
  `embedded_move_costs_match_design_doc`. Terrain Def/Avoid: FE GBA style
  (unchanged numbers), fliers get no terrain bonus. **Healing tiles are
  deferred** (Nick wants to design them with capturing later), so
  `heal_percent` is 0 on every terrain. Building tiles: only `fort` for the
  first playtest; village / gate / throne are deferred (maybe with
  capturing later), and 0308, 0409, 0501, 0803 and `weapons-and-items.md`
  were updated to match. The terrain tables moved from `stats-and-combat.md`
  into `terrain.md`.
- **Deviation, cost format**: `move_cost` is a map keyed by movement type name
  (`{ "foot": Some(2), ..., "flying": None }`) rather than a bare list, so the
  file is readable without counting positions. The loader still produces the
  `Vec<Option<u8>>` indexed by `MovementTypeId`, and reports missing and
  unknown movement types. A cost of `Some(0)` is rejected (would break
  pathfinding); `heal_percent` over 100 is rejected; glyphs must be in the
  font atlas's required set.
- **Palette**: added placeholder terrain colours `thicket, peak, sea, wood,
  floor, fort` (Nick tunes the look in 0011). The debug palette sampler
  snapshot changed only by listing them.
- **.map format** documented in `assets/maps/README.md`. `parse_map` reports
  every error with line/column; also rejects a header without `---` and an
  empty first row; tolerates CRLF and trailing blank lines.
- **Deviation**: `print_map` returns `Option<String>` (`None` if a tile's
  terrain has no legend character) instead of `String`. The parser returns a
  `MapDef { map, legend }` so a map can be printed back with its own legend.
- `Content` now has `terrain` and `maps` (keyed by file stem). Terrain colour
  checks are skipped when the palette failed to load, and maps are skipped
  when terrain failed, so one broken file doesn't produce a flood of errors.
- Follow-ups: 0017 (decide: fliers as a tier 3+ class line, from Nick).
- For Nick: nothing visible yet; the map is first drawn in 0401.
