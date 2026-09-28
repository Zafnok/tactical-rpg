---
id: "0416"
title: Cursor as corner marks that never cover a neighbour's initials
type: bug
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0402"]
nick_input: sign-off
completed: 2026-09-27
---

# 0416 — Cursor as corner marks that never cover a neighbour's initials

## Context

Nick's playtest of the Quick Battle (after 0402): "the cursor overlaps units'
names when adjacent. Like if Ar is on tile 1x1 and cursor [ ] is on tile 1x2
then it just says A [ ] the r in Ar is cut off."

Cause: `draw_cursor` (`crates/ui/src/screens/battle/cursor.rs`) writes `[` and
`]` as whole glyphs into the cells either side of the tile, which are the
neighbouring tiles' inner cells (ADR-0018 "Cursor and path"). A unit label
fills both cells of its tile, so an adjacent cursor overwrites one letter.

Nick picked a new look from real renders (recorded in
`docs/design/look-and-feel.md`, "Cursor and selection"): **3×3 corner marks**
by default, with **4×4 corner marks** and a **tile glow** as accessibility
options. The corners are drawn as pixel overlays in pixel columns that the font
never inks (Terminus 8×16: letters use columns 1–6 of a cell, plus column 0 for
`M T W Y m w`; column 7 is always blank; letters use rows 2–11, the HP bar
rows 14–15). The technical change is [ADR-0024](../../docs/adr/0024-cursor-as-pixel-overlays.md).

## Nick input

**Sign-off:** play the Quick Battle and move the cursor next to and onto units;
no initial should ever be hidden.

## Scope

**In:**
- `CursorStyle { Corners, LargeCorners, TileGlow }`, default `Corners`, held in
  `Ctx` until 0805 moves it into `Settings`.
- Drawing all three styles; pulsing as today.
- Design doc, ADR-0024 (partly superseding ADR-0018), and 0805's steps gain the
  cursor style row.

**Out (do not do):**
- The options screen itself (0805).
- The selected-unit `►` `◄` arrows (0403): they aren't drawn yet. 0403 gets a
  note that they share the overlap problem.

## Implementation steps

1. `cursor.rs`: add `CursorStyle` (derive `Default`, `Copy`, `Eq`, `Debug`).
   Replace the glyph brackets in `draw_cursor(buf, palette, cursor, style, x, y)`:
   - Corner marks: tile pixel origin `(px, py) = (x·8, y·16)`. Arms are 1 px
     thick, in `cursor` colour × brightness, `Layer::Over`. Left arms in pixel
     column `px − 1` (the left neighbour's always-blank column 7), right arms in
     `px + 15` (this tile's always-blank column 7). Top arms on row `py`, bottom
     arms on row `py + 13` (just above the HP bar). Each corner: a horizontal
     arm `n` px long running inward and a vertical arm `n` px long running
     inward; `n = 3` for `Corners`, `4` for `LargeCorners`.
   - Tile glow: `blend_bg` the tile's two cells towards `cursor` by
     `GLOW_MAX × brightness` (`GLOW_MAX = 0.3`, tunable).
   - Clip everything to `MAP_VIEW` (in pixels for overlays).
2. `screen.rs`: `Ctx::cursor_style: CursorStyle` (default `Corners`).
3. `battle/mod.rs`: pass `ctx.cursor_style` to `draw_cursor`.
4. Docs: design doc, ADR-0024, ADR-0018 status line and ADR index, a cursor
   style row in 0805, a note in 0403.

## Acceptance criteria

- [x] With any style, a cursor next to or on a unit changes no glyph of any
  unit label (property test over positions, neighbours and styles).
- [x] Corner arms are 3 px (default) or 4 px long, in the pixel columns and rows
  above (unit test).
- [x] Tile glow tints only the tile's two cells, pulsing with brightness (unit test).
- [x] Nothing is drawn outside the map viewport (unit test at the edges).
- [x] Battle snapshots updated and reviewed.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: arm placement per style; glow; viewport clipping; pulse.
- Property: labels untouched for random neighbour layouts.
- Snapshot: existing battle snapshots (now list the cursor overlays).

## Completion notes

- The cursor is now drawn as 1 px corner-mark overlays (3 px arms by default,
  4 px for `LargeCorners`) or as a tile glow, per ADR-0024. None of the styles
  change any cell's glyph, so neighbouring initials (and terrain like the
  fort's `[]`, which the old brackets also overwrote) always show. The
  property test `cursor::tests::never_covers_letters` checks this for every
  style, pulse phase and tile.
- The style is `Ctx::cursor_style` (default `Corners`). There's no way to pick
  the accessibility styles in the game yet: 0805's Options menu gains a
  "Cursor" row for that (its steps are updated).
- Tests that found the cursor by its `[` glyph now find it by its overlays.
  The four battle snapshots changed only by the cursor.
- Checked in the real web build: corner marks next to and on `Kn` in the
  Quick Battle, both letters intact.
- Open for Nick (recorded in `look-and-feel.md` and ticket 0403): the
  selected-unit arrows `►Al◄` would hide a neighbour's letter the same way,
  so 0403 must ask before drawing them.
- *Claude's starting rule:* tile glow strength up to 30% at full brightness
  (`GLOW_MAX`, tunable), matching the mockup Nick saw.
