---
id: "0231"
title: "Images in the frame: sprite items drawn from asset files"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-01
---

# 0231 — Images in the frame: sprite items

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
renderer (0232). It has no dependencies and is on the Chapter 1 critical
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

- [x] Unit: `add_sprite` clips and drops; `fill_rect` over half a sprite
      leaves sprites whose `clip`s cover exactly the rest; `blit` offsets
      `dest` and `clip`.
- [x] Property: after any mix of `add_sprite`, `add_overlay`, `fill_rect`
      and `blit`, every item's visible rectangle lies inside
      `pixel_bounds()`, and no sprite `clip` overlaps a cell filled after
      it was added.
- [x] Every existing snapshot is unchanged.
- [x] Snapshot of the "Sprite test" tool, with one sprite line per sprite.
- [x] Content test: a non-PNG file named `.png` and an oversize image each
      give their error; the embedded table lists `images/test_card.png`
      16×16.
- [x] The tool was looked at on native and web; Completion notes say what
      was seen.
- [x] `trpg-ui` and `trpg-content` still decode no image
      (`cargo tree -p trpg-ui -e normal` has no `png` or `image` crate).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the sprite rules above; `ImageTable` loading and errors; snapshot
  line format (every optional suffix).
- Property: as listed.
- Snapshot / integration: the debug tool through the Harness.

## Completion notes

Done. A frame can now hold pictures: `GlyphBuffer::add_sprite(Sprite)`
puts part of an image file on the frame, `app` draws it from a texture,
and a snapshot shows it as one line. Nothing in the game uses it yet
except the new "Sprite test" debug tool (F2, last entry); 0711, 0413, 0433
and 0232 build on it.

**For Nick:** nothing to decide and no gameplay rule was added. After the
merge you can look at it on Pages: F2, then "Sprite test".
`docs/screenshots/0231-sprite-test.png` is what it should look like.

**What was built**

- `trpg_content::image`: `ImageTable` (`Content::images`), every PNG under
  `assets/` except the font atlas, by path, with its size. Errors name
  each bad file: not a PNG, an empty image, a side over 4096.
  `bundle::files_under` lists a directory and everything below it.
- `Sprite` and `Item` in `glyph_buffer.rs`. Rectangles and sprites share
  one list in drawing order (`items()`); `overlays()` and `sprites()` give
  one kind. `fill_rect` and `blit` cut sprites as they cut rectangles.
- Snapshot lines for sprites; no existing snapshot's format changed.
- `app` uploads a texture per image and draws items in order per layer; a
  sprite without a texture is a magenta rectangle, logged once.
- `cargo xtask test-card` writes `assets/images/test_card.png` (16×16).
- "Sprite test" debug tool; `crates/ui/README.md` (*What a frame holds*)
  and `assets/images/README.md`.

**What was seen** (acceptance: looked at on native and web)

- Native (Windows, debug build), window sized for scale 1 and for scale 2:
  the card at 1×, 3× and 5× is sharp with its one-pixel border; flipped
  swaps left and right; the top-right quarter shows green with the border
  on the top and right; the glyphs sit on the picture drawn under them;
  the half-opacity card shows the panel through it; the card under the
  text box keeps only its top rows and left half.
- Web build in the browser pane at scales 1, 2 and 3: the same, and pixels
  read back from the canvas matched exactly at each scale (every screen
  pixel of a card pixel one colour; half opacity over the panel reads
  116,37,35 for red, the 50% blend).
- First look found the text over the "under the glyphs" picture was white
  on the card's white border; it is black now.

**Deviations from the steps**

- **Steps 2, 3 and 6 (`ImageId`):** an `ImageId` is the interned bundle
  path itself (`ImageId::path()`), not an index into the table. Step 6
  offered "store the path beside the id"; with the path stored, the index
  added nothing, and a path can't silently point at the wrong picture if
  two tables differ. So `ImageTable::images` is keyed by `&'static str`
  (bundle paths are static), there is `ImageId::path()` instead of
  `ImageTable::path(id)`, and `ImageTable::info(id)` and `ids()` were
  added. `to_snapshot(&Palette)` and `Harness::snapshot()` keep their
  signatures. ADR-0038 section 2 already says a sprite is "named by its
  path in the asset bundle", so it needed no amendment.
- **Step 5:** `add_sprite` also clips `clip` to `dest`, and drops a sprite
  whose `src` is empty: both show nothing.
- **Step 7:** the arithmetic for "the part of `src` that maps to `clip`"
  is `Sprite::clipped_src()` in `ui`, where it is unit-tested (`app` has
  no tests and is outside mutation testing); `app` calls it. 0232's
  renderer can use it too.
- **Step 8:** no `--check` flag; a test compares the committed file's
  pixels with the tool's. Not added to `THIRD_PARTY_ASSETS.md`: that file
  lists third-party items only (our own sounds aren't in it either).
- **Step 9:** the tool also shows a crop (the top-right quarter) and a
  picture on the `Under` layer with glyphs over it, so every `Sprite`
  field is on screen. "Sprite test" is the last debug tool, so no test's
  key script for the other tools changed except one in
  `tests/key_bindings_screen.rs` that relied on "Key bindings" being last.
- **Existing tests touched:** the debug menu snapshot gained the "Sprite
  test" line; two battle tests changed `.iter()` to `.into_iter()` because
  `overlays()` now returns a `Vec`.

**Notes for later tickets**

- A sprite's `src` must lie inside its image; the frame doesn't know image
  sizes. Tickets that read rectangles from files (0711, 0433) check them
  against `Content::images` when loading.
- A see-through sprite leaves the web canvas itself slightly see-through
  there. It looks right because the page behind it is black
  (`web/index.html`); keep it black.
- 0813 / 0817 / 0228 (whole-frame effects) must handle items as well as
  cells, as ADR-0038 says; `dim` and `blend_bg` are documented as cells
  only.

No follow-up tickets.
