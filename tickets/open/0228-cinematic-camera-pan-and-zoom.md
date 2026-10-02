---
id: "0228"
title: "Cinematic camera: show a large glyph scene through a panning, zooming window"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0036"]
nick_input: answer-first
completed:
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

How zoom should *look* (crisp whole steps, a smooth change, or pan only) is
Nick's choice in 0036 (Q3). This ticket builds what he picked.

## Nick input

**Answer first:** ticket 0036 (`docs/design/title-screen.md`, *Intro
cinematic*, the zoom rule).

## Scope

**In:**
- A way for a screen to show a second glyph picture (a "scene", any size in
  cells, e.g. a whole map with its units and overlays) through a window on
  the console, at a pixel offset and a zoom, with ordinary console cells
  (text, menus, the logo) drawn on top of it.
- Pixel-smooth panning: the offset is in scene pixels, not cells.
- Zoom as 0036 decided: at least whole steps 1×, 2×, 3×; in-between sizes
  only if Nick chose smooth zoom.
- Snapshots and the `Harness` stay deterministic.
- An ADR (`write-adr`): it extends ADR-0016/ADR-0018's drawing model.

**Out (do not do):**
- The cinematic's file format, shots or timing (0817).
- Using it in the battle screen. The battle camera stays as it is.
- Rotation, blur, or any effect beyond offset and zoom.

## Implementation steps

1. **ADR first.** Recommended design (change it if you find a better one,
   and say why in the ADR):
   - `trpg_ui::glyph_buffer::Backdrop { scene: Rc<GlyphBuffer>, clip: Rect
     /* console cells */, origin_px: (f32, f32) /* scene pixel at the clip's
     top-left */, zoom: f32 }` and `GlyphBuffer::set_backdrop(Backdrop)` /
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
   clip_px: (u32, u32), centre: (f32, f32), zoom: f32) -> (f32, f32)`: the
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
   pan and Confirm steps through the zooms. Follow the `keyboard-input`
   skill for its help line. It is how the result is checked by eye on
   native and on the web.
6. Look at it on native and in the web build at window scales 1 and 2:
   glyphs crisp at 2× and 3×, no seams between cells while panning.

## Acceptance criteria

- [ ] Unit: `cinema::view` centres, clamps at every edge, and centres a
      scene smaller than the window.
- [ ] Snapshot: the debug tool at 1×, 2× and 3× (header line and scene).
- [ ] A screen with a backdrop and a text box over it draws the box's cells
      and shows the scene only through see-through cells (unit test on the
      draw plan or snapshot).
- [ ] Existing snapshots are unchanged (no backdrop means no header line).
- [ ] Checked by eye on native and web: the completion notes say what was
      looked at.
- [ ] The zooms offered match 0036's answer.
- [ ] The ADR is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `cinema::view`; `Backdrop` and see-through cells in `GlyphBuffer`.
- Property: for any centre and allowed zoom, the window stays inside the
  scene when the scaled scene is at least as big as the window.
- Snapshot / integration: the debug tool through the Harness.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
