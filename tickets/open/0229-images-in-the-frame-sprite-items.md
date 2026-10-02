---
id: "0229"
title: "Images in the frame: sprite items drawn from asset files"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0229 — Images in the frame: sprite items

## Context

Nick wants graphics to be replaceable (2026-10-01): glyphs today, sprites
from image files later, without an engine rewrite. ADR-0038 sets the rules.
The first gap it found: a frame can't hold a picture. A `GlyphBuffer`
(`crates/ui/src/glyph_buffer.rs`) is cells plus solid-colour rectangles
(`Overlay`), and `app` (`crates/app/src/render.rs`, `Renderer::draw`) draws
only those. Ticket 0711 was going to draw each bought face as over a
thousand small rectangles.

This ticket adds the picture item everything else uses: bought portraits
(0711), combat pictures (0413), a sprite map (0433), the headless PNG
renderer (0230). It has no dependencies and is on the Chapter 1 critical
path through 0711.

## Nick input

None.

## Scope

**In:**
- `Sprite` items in `GlyphBuffer`, with the same clipping, cutting and
  `blit` rules overlays have.
- An image table in `trpg-content`: every PNG under `assets/` except the
  font atlas, by bundle path, with its size.
- `app` draws sprite items from textures.
- Snapshot lines for sprites.
- One public test image and a debug tool that shows it.

**Out (do not do):**
- Portraits or combat pictures (0711, 0413), map tiles (0433).
- Animation frames, rotation, colour tinting, flashing. A later ticket that
  needs one adds it to `Sprite` (0413 may need a flash).
- Decoding PNG pixels in `content` or `ui`. Only `app` (and `xtask`)
  decode.
- Renaming `GlyphBuffer`.

## Implementation steps

1. **Read ADR-0038.** If the design below changes, amend that ADR's
   section 2 in this PR (it is the decision this ticket implements).
2. **`trpg_content::image`** (`crates/content/src/image.rs`):
   `ImageTable { images: BTreeMap<String, ImageInfo> }`,
   `ImageInfo { width: u32, height: u32 }`, key = bundle path with forward
   slashes (e.g. `"images/test_card.png"`). `ImageTable::load()` walks the
   bundle for `*.png` (add a recursive lister to `bundle.rs` beside
   `files_in`), skips `font::ATLAS_PNG_PATH`, and reads each size with
   `font::png_size` (move that helper to `image.rs` and re-export it).
   Errors (all at once, with file names): not a PNG, zero size, either side
   over 4096. Add `Content::images` and the all-assets test.
3. **`Sprite`** in `glyph_buffer.rs`:
   ```rust
   pub struct Sprite {
       pub image: ImageId,      // interned bundle path; see below
       pub src: PxRect,         // in image pixels
       pub dest: PxRect,        // in console pixels
       pub clip: PxRect,        // the part of `dest` that is drawn
       pub layer: Layer,
       pub flip_x: bool,
       pub opacity: u8,         // 255 = solid; lower lets what's under it show
   }
   ```
   `ImageId` is a cheap `Copy + Eq + Hash` handle: an index into
   `ImageTable` (add `ImageTable::id(path) -> Option<ImageId>` and
   `path(id) -> &str`). `Sprite::new(image, src, dest, layer)` sets
   `clip = dest`, no flip, opacity 255.
4. **One drawing order.** Rectangles and sprites in a layer are drawn in
   the order they were added. Store them in one list
   (`enum Item { Rect(Overlay), Sprite(Sprite) }`, `GlyphBuffer::items()`),
   and keep `overlays()` returning only the rectangles (as a `Vec`), so the
   existing tests that read overlays stand. Add `sprites()` likewise and
   `add_sprite(Sprite)`.
5. **The same rules as overlays:**
   - `add_sprite` clips `clip` to `pixel_bounds()`; a sprite wholly outside
     is dropped. `src` and `dest` are never changed by clipping.
   - `fill_rect` and `blit` cut sprites as they cut overlays
     (`cut_overlays`): a sprite under replaced cells is split into up to
     four sprites with the same `src`/`dest` and smaller `clip`s.
   - `blit` brings the source buffer's sprites along, `dest` and `clip`
     offset, `clip` clipped.
   - `GlyphBuffer::dim(rect, factor)` is for cells; it does not touch
     items. Document that on `dim`. (Whole-frame fades are 0813's.)
6. **Snapshot** (`crates/ui/src/snapshot.rs`): the `--- overlays ---`
   section lists items in drawing order. A sprite line:
   `over  sprite 8,8 240x240  images/test_card.png 0,0 48x48` followed by
   ` flip`, ` opacity=128`, ` clip=8,8 120x240` only when not the default.
   `to_snapshot` needs the image paths: take them from an `&ImageTable`
   argument or store the path beside the id; pick one and keep
   `Harness::snapshot()`'s signature. Snapshots without sprites must not
   change.
7. **`app`** (`render.rs`): `Renderer::new` also takes the image table and
   uploads a `Texture2D` (`FilterMode::Nearest`) per image from
   `trpg_content::bundle::bytes`. `Renderer::draw` draws each layer's items
   in order; for a sprite, `draw_texture_ex` with the part of `src` that
   maps to `clip` (compute it in `f32`; nearest sampling keeps it exact at
   whole scales), `flip_x`, and colour `WHITE` with alpha `opacity`. A
   missing texture draws a magenta rectangle and logs once, like a missing
   glyph.
8. **Test image:** `assets/images/test_card.png`, 16×16, made by a new
   `cargo xtask test-card` (four coloured quadrants and a one-pixel border,
   so a flip, a crop and a scale are each visible). It is generated, not
   art; add it to `THIRD_PARTY_ASSETS.md` only if that file lists our own
   generated assets (check how the font atlas is listed).
9. **Debug tool** (`crates/ui/src/debug.rs`, the tools list): "Sprite
   test": the card at 1×, 3× and 5×, flipped, at half opacity over a
   panel, and half-covered by a text box drawn after it (shows the cut).
   Help line per the `keyboard-input` skill.
10. Look at the debug tool on native and in the web build (browser pane) at
    window scales 1 and 2. Update `crates/ui/README.md` (the frame's
    items) and `assets/README` or `assets/images/README.md` (what goes in
    `images/`).

## Acceptance criteria

- [ ] Unit: `add_sprite` clips and drops; `fill_rect` over half a sprite
      leaves sprites whose `clip`s cover exactly the rest; `blit` offsets
      `dest` and `clip`.
- [ ] Property: after any mix of `add_sprite`, `add_overlay`, `fill_rect`
      and `blit`, every item's visible rectangle lies inside
      `pixel_bounds()`, and no sprite `clip` overlaps a cell filled after
      it was added.
- [ ] Every existing snapshot is unchanged.
- [ ] Snapshot of the "Sprite test" tool, with one sprite line per sprite.
- [ ] Content test: a non-PNG file named `.png` and an oversize image each
      give their error; the embedded table lists `images/test_card.png`
      16×16.
- [ ] The tool was looked at on native and web; Completion notes say what
      was seen.
- [ ] `trpg-ui` and `trpg-content` still decode no image
      (`cargo tree -p trpg-ui -e normal` has no `png` or `image` crate).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the sprite rules above; `ImageTable` loading and errors; snapshot
  line format (every optional suffix).
- Property: as listed.
- Snapshot / integration: the debug tool through the Harness.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
