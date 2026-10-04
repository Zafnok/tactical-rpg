---
id: "0228"
title: "Cinematic camera: show a large glyph scene through a panning, zooming window"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: ["0036"]
nick_input: answer-first
completed: 2026-10-03
---

# 0228 — Cinematic camera: pan and zoom over a glyph scene

## Context

The title's intro cinematic (ticket 0036) pans and zooms over a battlefield
and, later, the overworld. The renderer can't do that today:

- `app` draws one 100×32-cell `GlyphBuffer` at a whole-number window scale
  (`crates/app/src/render.rs`, `Renderer::draw`; ADR-0016, ADR-0003).
- The battle camera moves a whole tile at a time (`screens/battle/camera.rs`),
  which is 16 px: too jerky for a slow pan.
- Nothing can be drawn bigger than one glyph per cell, so there is no zoom.
  A 24×16-tile map (Chapter 1) is only 384×256 px of the 800×512 px
  console, so close-ups need 2× or 3×.

How zoom should *look* was Nick's choice in 0036: **whole steps only (2×,
3×, 4×), always crisp, and a shot keeps one size while it is on screen**;
no gradual zoom (`docs/design/title-screen.md`, *Intro cinematic*). So
`zoom` here is a whole number, and nothing in this ticket animates it.

## Nick input

**Answer first:** ticket 0036 (`docs/design/title-screen.md`, *Intro
cinematic*, the zoom rule). Answered 2026-10-03: whole steps.

## Scope

**In:**
- A way for a screen to show a second glyph picture (a "scene", any size in
  cells, e.g. a whole map with its units and overlays) through a window on
  the console, at a pixel offset and a zoom, with ordinary console cells
  (text, menus, the logo) drawn on top of it.
- Pixel-smooth panning: the offset is in scene pixels, not cells.
- Zoom as 0036 decided: whole steps 1×, 2×, 3× and 4× only. No in-between
  sizes (Nick ruled out smooth zoom).
- Snapshots and the `Harness` stay deterministic.
- An ADR (`write-adr`): it extends ADR-0016/ADR-0018's drawing model.

**Out (do not do):**
- The cinematic's file format, shots or timing (0817).
- Using it in the battle screen. The battle camera stays as it is.
- Rotation, blur, or any effect beyond offset and zoom.
- Fractional or animated zoom.

## Implementation steps

1. **ADR first.** Recommended design (change it if you find a better one,
   and say why in the ADR):
   - `trpg_ui::glyph_buffer::Backdrop { scene: Rc<GlyphBuffer>, clip: Rect
     /* console cells */, origin_px: (f32, f32) /* scene pixel at the clip's
     top-left */, zoom: u8 /* 1..=4 */ }` and `GlyphBuffer::set_backdrop(Backdrop)` /
     `backdrop()`. At most one per frame; `clear`/refill removes it.
   - Console cells inside `clip` that should let the scene show are marked
     see-through (e.g. `Cell::see_through()`, a flag on `Cell`). Every
     other cell draws over the scene as today, so text boxes and menus
     need no changes.
   - `Renderer::draw` order: console backgrounds → the scene (its cell
     backgrounds, under-overlays, glyphs, over-overlays, scaled by `zoom`,
     shifted by `origin_px`, clipped to `clip`) → console under-overlays,
     glyphs, over-overlays, skipping see-through cells. Nearest-neighbour
     sampling (the atlas texture already uses it). After scaling, snap the
     scene's offset to whole window pixels so glyphs don't smear.
   - Scene pixels outside the scene (the window runs past its edge) show
     the console's clear colour.
   - The scene's **sprite items** (0231, ADR-0038) scale, shift and clip
     with it like its rectangles, so a sprite-skinned map (0433) or a
     picture in a scene pans and zooms too. If 0231 isn't done yet, add a
     line to 0231's steps saying the backdrop must draw them; if 0232 is
     done, its PNG renderer draws backdrops as well.
2. A pure helper for shots, `trpg_ui::cinema::view(scene_px: (u32, u32),
   clip_px: (u32, u32), centre: (f32, f32), zoom: u8) -> (f32, f32)`: the
   `origin_px` that puts `centre` in the middle of the window, clamped so
   the window never leaves the scene (centred on an axis where the scaled
   scene is smaller than the window).
3. Snapshots (`crates/ui/src/snapshot.rs`, `GlyphBuffer::to_snapshot`):
   when a backdrop is set, add a header line with `clip`, `origin_px`
   (rounded to 0.1) and `zoom`, then the scene's own snapshot, so a test
   pins both the picture and where the window is. See-through cells get a
   marker that can't be confused with a space.
4. `app`: implement the draw order. Clip with macroquad's scissor (check it
   uses physical pixels with `high_dpi`, as `Renderer::draw` does).
5. A debug tool (F2 menu, `crates/ui/src/debug.rs`, `TOOLS`): "Scene
   camera", showing `test_small.map` as a backdrop that the cursor actions
   pan and Confirm steps through the zooms 1× to 4×. Follow the `keyboard-input`
   skill for its help line. It is how the result is checked by eye on
   native and on the web.
6. Look at it on native and in the web build at window scales 1 and 2:
   glyphs crisp at 2×, 3× and 4×, no seams between cells while panning.

## Acceptance criteria

- [x] Unit: `cinema::view` centres, clamps at every edge, and centres a
      scene smaller than the window.
- [x] Snapshot: the debug tool at 1×, 2×, 3× and 4× (header line and scene).
- [x] A screen with a backdrop and a text box over it draws the box's cells
      and shows the scene only through see-through cells (unit test on the
      draw plan or snapshot).
- [x] Existing snapshots are unchanged (no backdrop means no header line).
- [x] Checked by eye on native and web: the completion notes say what was
      looked at.
- [x] The zooms offered match 0036's answer: whole numbers 1–4, nothing
      in between.
- [x] The ADR is written and listed in `docs/adr/README.md`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `cinema::view`; `Backdrop` and see-through cells in `GlyphBuffer`.
- Property: for any centre and allowed zoom, the window stays inside the
  scene when the scaled scene is at least as big as the window.
- Snapshot / integration: the debug tool through the Harness.

## Completion notes

**Done.** ADR-0048 records the design.

- **`GlyphBuffer` backdrop** (`crates/ui/src/glyph_buffer/backdrop.rs`):
  `set_backdrop(scene: Rc<GlyphBuffer>, clip, origin_px, zoom)`,
  `backdrop()`, `clear_backdrop()`. Zoom is clamped to 1..=4
  (`MAX_ZOOM`); nothing takes a fractional zoom. A solid fill of the whole
  buffer (how `Game` clears a frame) removes the backdrop.
- **See-through cells**: `Cell::see_through(glyph, fg)`, a flag on `Cell`.
  The scene shows only behind them; a see-through cell may still hold a
  glyph (text straight on the scene, for captions and the logo), and
  `print_fg` keeps it see-through.
- **`trpg_ui::cinema::view`**: centres, clamps at every edge, centres a
  smaller scene. Unit and property tests.
- **Snapshots**: `bg:see-through` legend key; a
  `--- backdrop: clip … origin … zoom N ---` line and the scene's own
  snapshot after the console's. No backdrop, no change: every existing
  snapshot is byte-for-byte the same except the debug menu's (one more
  tool).
- **`app`** (`render.rs`): clear colour → the scene in its window → the
  console. The scene's sprites and rectangles scale, shift and clip with
  its cells (0231's sprite items; checked with the test tileset skin).
- **`cargo xtask frame-png`** (0232) draws backdrops too, with hand-checked
  pixel tests.
- **Debug tool "Scene camera"** (F2 menu, before "Map skin"): the whole
  `test_small.map`, painted by the map skin in use, with a path and the
  cursor on it; held cursor keys pan at 60 scene pixels a second, Confirm
  steps 1× → 2× → 3× → 4× → 1×, Cancel closes. A caption sits on the scene
  and a text box covers the window's corner. Harness snapshots at each
  zoom.

**Deviations from the plan.**

- *Draw order.* The ticket had console backgrounds, then the scene. It is
  the scene first, then every solid console background: the same picture,
  with no per-cell clipping of the scene.
- *No scissor.* macroquad's scissor needs `unsafe`, which the workspace
  forbids. The window is clipped by a camera whose viewport is the window
  (physical pixels, so it holds with `high_dpi`).
- *See-through marker.* It is in the snapshot's colour rows and legend
  (`bg:see-through`), not the glyph rows: a see-through cell may hold a
  real glyph, so no glyph could stand for "hole".
- `GlyphBuffer` lost `Eq` (kept `PartialEq`): the origin is a float.

**Checked by eye.**

- *Web* (debug build in the browser pane, display scale 1.75): the Scene
  camera at 1×, 2×, 3× and 4×, at window scale 2 (1792×1344 canvas) and
  window scale 1 (1540×1050 canvas, console at an odd offset). At every
  zoom and both scales the canvas was **pixel-identical** to
  `frame-png`'s render of the same frame (0 pixels differing). Then
  panned at 4× right, down and left: fractional origins (107.6, 89.4),
  stopping at the bottom edge, no seams.
- *Native* (Windows, debug build, 1640×1064 window = window scale 2): 3×,
  4×, and 4× panned to origin 106.0,57.3. Glyphs sharp (each font pixel a
  6×6 or 8×8 block), the path and cursor rectangles scaled with the
  tiles, the scene cut exactly at the window's frame, the caption over the
  scene, the text box hiding it. Native at window scale 1 was not looked
  at (the window opens at scale 2 here); the web build covered scale 1
  with the same drawing code.
- Sprite skin: `frame-png` of the Scene camera with the test tileset at 2×.

**Gameplay rules decided:** none. The pan speed and layout are the debug
tool's own.

**Follow-ups:** none created. If a very large scene (the overworld, 1010)
turns out slow, skip the cells outside the window in `draw_layers`
(ADR-0048, *Consequences*).

**For Nick:** nothing to decide. To see it: F2 → "Scene camera" on the
Pages build after the merge; hold the cursor keys to pan, Confirm to step
the zoom.
