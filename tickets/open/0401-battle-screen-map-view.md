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
[ADR-0012](../../docs/adr/0012-visual-style.md) — and
`docs/design/look-and-feel.md` **if 0011 is done** (it overrides ADR defaults
for unit markers, cursor and palette).

## Nick input

None.

## Scope

**In:** `ui::battle::BattleScreen` (draw only + holding a `BattleState`),
`Camera`, layout constants, drawing terrain and units, a debug "Quick Battle"
entry on the title menu (debug builds only) that loads `test_small.map` with
placeholder units.

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
4. Terrain drawing from the terrain display table (2 glyphs, fg, bg).
5. Unit drawing: default glyph 1 = the class's `map_glyph` (add `map_glyph: char`
   to class display data in `classes.ron` and its loader), glyph 2 = status marker
   (`·` ready, `ˇ` acted, `!` HP ≤ 25%). Fg = faction colour; acted units dimmed
   (`Rgb::scale`). Background = terrain bg. Follow `look-and-feel.md` if it exists.
6. `BattleScreen::new(state, content)`; `draw` renders map + empty side panel box
   + help bar text `arrows move · f select · d back · e info` (key names read from the keymap).
7. Title menu: in debug builds add `Quick Battle` → pushes a `BattleScreen` built
   from `test_small.map` + placeholder characters (`d` pops back for now).

## Acceptance criteria

- [ ] Quick Battle shows the map with units, correct 2-glyph tiles and colours.
- [ ] Camera tests cover: small map centred, large map clamped at all four edges, margin respected.
- [ ] Snapshot of the Quick Battle screen committed.

## Tests required

- Unit: `Camera::follow`, `tile_to_cell`.
- Snapshot: battle screen (small map); a large generated map with camera scrolled to bottom-right.

## Completion notes

