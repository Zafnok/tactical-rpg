---
id: "0401"
title: "Battle screen layout: map viewport, camera, terrain and unit drawing"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0205", "0301", "0302"]
nick_input: none
completed:
---

# 0401 — Battle screen: map view and camera

## Context

First battle screen. Layout and look per
[ADR-0018](../../docs/adr/0018-visual-style-v2.md) and
[`docs/design/look-and-feel.md`](../../docs/design/look-and-feel.md) (decided
by Nick in 0011; screenshots in `docs/screenshots/0011-*.png`).

## Nick input

None.

## Scope

**In:** `ui::battle::BattleScreen` (draw only + holding a `BattleState`),
`Camera`, layout constants, drawing terrain and units, a debug "Quick Battle"
entry on the title menu (debug builds only) that loads `test_small.map` with
placeholder units, and sub-cell overlay support (ADR-0018) in `GlyphBuffer`,
snapshots and the app renderer (needed for unit HP bars).

**Out:** cursor and input (0402), overlays (0403), side-panel contents (0402).

## Implementation steps

1. Layout constants in `ui::battle::layout`: map viewport = cells `x 0..70`,
   `y 0..30` (35×30 tiles); side panel = `x 70..100`, `y 0..30` (single-line box);
   help bar = rows 30–31.
2. `Camera { origin: Pos }` (top-left tile of viewport): `follow(target, margin = 3)`
   keeps target ≥ margin tiles from each viewport edge, clamped to the map;
   maps smaller than the viewport are centred (origin may be negative → draw
   blank). Pure + unit-tested.
3. `tile_to_cell(tile, camera) -> Option<(x, y)>` (x = 2 × tile x offset) —
   single place where the 2-cells-per-tile rule lives.
4. Terrain drawing from the terrain display table (2 glyphs; fg = palette
   `<terrain>`, bg = `<terrain>_bg`).
5. **Overlays (ADR-0018):** add `Overlay { rect: PxRect, color: Rgb, layer:
   Under | Over }` to `GlyphBuffer` (console-pixel coordinates), carried and
   offset by `blit`, clipped to the buffer; list them in snapshot text after the
   grid; draw them in `app::render` (Under after backgrounds, Over after glyphs,
   scaled like cells).
6. Unit drawing per `look-and-feel.md`: the unit's two-letter **map label**
   (default: first two letters of the character name, or of the class name for
   generic enemies; optional `map_label` override in unit/character data;
   duplicate labels within one faction on a map = content validation error).
   Fg = faction colour; bg = terrain bg (no backing). Acted: label lowercased
   and dimmed (lerp toward the background). HP bar: `Over` overlay on the
   tile's bottom 2 px, width `round(16 × hp / max)`, `hp_high` > 2/3, `hp_mid`
   > 1/3, else `hp_low`; the rest of the bar `black`.
7. `BattleScreen::new(state, content)`; `draw` renders map + empty side panel box
   + help bar text `arrows move · f select · d back · e info` (key names read from the keymap).
8. Title menu: in debug builds add `Quick Battle` → pushes a `BattleScreen` built
   from `test_small.map` + placeholder characters (`d` pops back for now).

## Acceptance criteria

- [ ] Quick Battle shows the map with units, correct 2-glyph tiles and colours.
- [ ] Camera tests cover: small map centred, large map clamped at all four edges, margin respected.
- [ ] Snapshot of the Quick Battle screen committed.

## Tests required

- Unit: `Camera::follow`, `tile_to_cell`; overlay `blit` offset and clipping;
  map label defaults, override, lowercase when acted; HP bar width/colour at
  the thresholds (property: width always in `0..=16`).
- Snapshot: battle screen (small map, including a wounded and an acted unit so
  HP-bar overlays and lowercase labels are reviewed); a large generated map with camera scrolled to bottom-right.

## Completion notes

