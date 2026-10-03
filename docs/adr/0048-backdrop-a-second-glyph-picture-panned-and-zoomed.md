# ADR-0048: A backdrop: a second glyph picture behind the console, panned by the pixel and zoomed in whole steps

- **Status:** Accepted
- **Date:** 2026-10-03
- **Related tickets:** 0228, 0036, 0817, 1010, 0231, 0232, 0433, 0439
- **Extends:** ADR-0003 and ADR-0016 (what a frame holds and how `app`
  draws it), ADR-0018 (pixel-placed items), ADR-0038 (a scene's sprites
  pan and zoom with it), ADR-0007 (what a snapshot shows)

## Context

- The title's intro cinematic (0036, `docs/design/title-screen.md`, *Intro
  cinematic*) pans and zooms over a battlefield and, later, the overworld
  (1010), with text and the logo on top.
- A frame is one 100×32-cell `GlyphBuffer` drawn at a whole window scale
  (ADR-0003, ADR-0016). Nothing can be drawn bigger than one glyph per
  cell, and the only camera (the battle's) moves a whole tile, 16 pixels,
  at a time.
- Nick chose how zoom looks (0036): **whole steps only (2×, 3×, 4×), always
  crisp, one size per shot.** No gradual zoom.
- A scene must be able to hold sprites (0231, 0433): a sprite-skinned map
  has to pan and zoom like a glyph one (ADR-0038).
- Snapshots and the `Harness` must stay deterministic (ADR-0007), and
  `cargo xtask frame-png` (0232) must show what `app` shows.
- The workspace forbids `unsafe`, and macroquad's scissor is only reachable
  through `unsafe { get_internal_gl() }`.

## Decision

### 1. A frame may hold one backdrop

`GlyphBuffer::set_backdrop(scene, clip, origin_px, zoom)` stores a
`Backdrop` (`crates/ui/src/glyph_buffer/backdrop.rs`):

| Field | Meaning |
| ----- | ------- |
| `scene: Rc<GlyphBuffer>` | The picture: any size in cells, with its own rectangles and sprites. Built with the same drawing calls as a frame (a map skin paints into it) |
| `clip: Rect` | The window, in console cells. Clipped to the buffer; an empty one sets no backdrop |
| `origin_px: (f32, f32)` | The scene pixel at the window's top-left. Fractions pan smoothly. It may lie outside the scene |
| `zoom: u8` | Console pixels per scene pixel: 1 to `MAX_ZOOM` (4). Other values are clamped. Never a fraction, and nothing animates it |

At most one per frame. Clearing the frame (a solid `fill_rect` over the
whole buffer, which `Game` does before every draw) removes it, so a screen
sets it every frame it wants it. A backdrop the scene has of its own is
not drawn.

### 2. The scene shows only through see-through cells

`Cell` has a `see_through` flag. `Cell::see_through(glyph, fg)` is a cell
with **no background**: the backdrop shows behind its glyph, and a space is
a plain hole. Every other cell is solid and draws over the scene as it
always did, so text boxes, menus and help bars need no changes: draw them
after marking the window.

- `print_fg` over a see-through cell keeps it see-through (text straight
  on the scene: a caption, the logo). Anything that sets a whole cell
  (`print`, `set`, `fill_rect`, `draw_box`, `blit`) makes it solid.
- A see-through cell with no backdrop behind it (none set, or outside
  `clip`) shows the console's clear colour.
- The console's own rectangles and sprites are drawn over the scene, as
  over any cell.

### 3. Drawing order and pixel rule (every renderer)

1. The clear colour.
2. The scene, inside `clip` only: its cell backgrounds, `Under` items,
   glyphs, `Over` items, each scene pixel `zoom × scale` window pixels big
   (`scale` = the window's whole scale, ADR-0003).
3. The console: backgrounds of **solid** cells (all of them when there is
   a backdrop, so a cell in the clear colour hides the scene), `Under`
   items, glyphs, `Over` items.

The scene's top-left is placed at `Backdrop::scene_offset(scale)`:
`−origin_px × zoom × scale`, **rounded to whole window pixels**. So a
scene pixel always covers a whole block of window pixels, glyphs never
smear, and cells never show seams; a pan moves in steps of one window
pixel (a quarter of a scene pixel at 2× in a 2× window). Scene pixels the
window shows beyond the scene's edge are the clear colour. Sampling is
nearest-pixel, as for everything else.

This differs from the order the ticket sketched (console backgrounds, then
the scene): drawing the scene first and every solid background after it
needs no per-cell clipping of the scene.

### 4. `app` clips with a camera, not a scissor

`Renderer::draw_backdrop` sets a `Camera2D` whose viewport is the window
in physical pixels and whose view is exactly that rectangle, one unit per
physical pixel; what falls outside a camera's view isn't drawn. It is
built by hand (`target`, `zoom`), because `Camera2D::from_display_rect`
flips the picture when drawing to the screen. `set_default_camera()`
follows. No `unsafe`.

`cargo xtask frame-png`'s `Painter` draws the scene into a picture the size
of the window and copies it in. It and `app` share
`Backdrop::scene_offset`, and were compared pixel for pixel on the web
build at window scales 1 and 2 (0228's completion notes).

### 5. Framing a shot is a pure function

`trpg_ui::cinema::view(scene_px, clip_px, centre, zoom) -> origin_px` puts
`centre` in the middle of the window, stops the window at the scene's
edges, and centres a scene smaller than the window. The cinematic's shots
(0817) move `centre` over time and call it; the debug menu's "Scene camera"
does the same from held keys.

### 6. Snapshots

- A see-through cell gets a colour key of its own; its legend line reads
  `bg:see-through`. In the colour rows a hole can't be mistaken for a
  blank cell.
- A frame with a backdrop ends with
  `--- backdrop: clip x,y wxh  origin x,y  zoom N ---` (the origin to a
  tenth of a pixel) and then the scene's own snapshot.
- A frame without a backdrop is unchanged.

## Consequences

- A screen pans and zooms any glyph or sprite picture with three calls:
  build the scene, mark the window, `set_backdrop`. 0817 and 1010 build on
  it.
- `GlyphBuffer` is no longer `Eq` (the origin is a float); it is still
  `PartialEq`, which is all tests used.
- Every `Cell` carries one more flag. `Cell { glyph, fg, bg }` literals no
  longer compile: use `Cell::new`.
- The whole scene is drawn every frame, clipped by the GPU. That is fine
  for a Chapter 1 map (48×16 cells); an overworld of tens of thousands of
  cells may need the cells outside the window skipped. Do that in
  `draw_layers` when it is measured to matter.
- The battle screen is untouched. Zooming the battle map (0439) is a
  different thing: a skin choice, not a backdrop.
- Rotation, blur, fractional or animated zoom are out. A later ADR would
  be needed, and Nick has ruled smooth zoom out.

## Alternatives considered

- **Draw the scene's cells straight into the console buffer, shifted.**
  Cells are 8×16 pixels: a pan would jump 8 pixels sideways, and no zoom
  is possible. This is what the battle camera does.
- **Render the scene to a texture, then draw the texture scaled.** One
  draw call and trivial clipping, but the texture has to be made in `app`
  from a second buffer anyway, render targets differ between native and
  WebGL1, and the Harness and `frame-png` would need a second
  implementation of the same thing. Drawing the same cells with a bigger
  scale reuses the code that is already tested.
- **Turn the scene into console items in `ui`** (a rectangle per cell
  background, a "glyph item" per glyph). No change in `app`'s order, but
  it needs a new item kind anyway, thousands of items per frame, and
  snapshots listing every one of them.
- **A list of see-through cells or rectangles beside the cells**, instead
  of a flag on `Cell`. Later writes (a text box over the window) would
  have to remember to remove entries; with the flag, replacing a cell
  makes it solid for free.
- **`Rc<GlyphBuffer>` versus a copy.** The scene is big and usually the
  same from frame to frame (only the origin moves); sharing it makes
  setting the backdrop free.
- **macroquad's scissor.** It needs `unsafe`. A viewport camera gives the
  same clip without it.
- **Fixed-point origin (to keep `Eq`).** The finest step that matters
  depends on zoom × window scale, so any fixed unit is either too coarse
  or arbitrary. A float, rounded once at draw time, is simpler.
