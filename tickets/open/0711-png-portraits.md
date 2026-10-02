---
id: "0711"
title: PNG portraits drawn as sprite items (48×48 bought faces)
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0021", "0110", "0231"]
nick_input: sign-off
completed:
---

# 0711 — PNG portraits drawn as sprite items

## Context

Nick chose bought portraits (0021): the face set faces from Mega Tiles'
Tiny Tales packs, **48×48 pixel art, 8 expressions per character**
(`docs/design/look-and-feel.md`, *Portraits and battle art*; ADR-0032).
Today portraits are 32×32 text
grids of palette keys, drawn as half-block cells: 2 pixels per cell, so 8×8
screen pixels per portrait pixel (ADR-0018, `trpg_content::portrait`,
`trpg_ui::portrait::draw_portrait`).

The dialogue frame is **32×16 cells = 256×256 console px**. A 48×48 face
drawn at the largest whole scale that fits, **5×5 console px per image
pixel**, takes 240×240 px, centred in that frame with an 8 px margin. So the
dialogue layout (0704) doesn't change. Cells can't hold pixels of different
colours at that grain, so each portrait is drawn as **one sprite item**
(0231, ADR-0038: `GlyphBuffer::add_sprite`, `Sprite`), which `app` draws
from the PNG as a texture.

**Changed 2026-10-01 (ADR-0038):** this ticket used to draw each face as one
solid rectangle per run of same-coloured pixels (over a thousand overlays a
face, each a line in every snapshot). Pictures are now sprite items, so
0231 was added to `blocked_by`, and `content` no longer decodes pixels: it
reads each PNG's size from 0231's image table.

The store's face sets are RPG Maker MV/MZ files: a **576×288 sheet, a 4×2
grid of 144×144 cells**, each face drawn at 3× (so 48×48 underneath). Check
this on the bought files before writing the importer; the store pages don't
state it.

Keep the drawing general (any PNG size, any whole scale): 0413 reuses it
for the big still battle images.

**Needs 0110 first** (added to `blocked_by` 2026-10-01): the importer reads
the bought zips and writes into `assets-private/`, the face-set layout must
be checked on the bought files, and the sign-off shows two bought
portraits. All of that needs Nick's purchase and the private assets folder
from 0110.

## Nick input

**Sign-off:** a screenshot of the test scene (F2 → Play test scene) with two
bought portraits, speaker and dimmed listener, next to one with the old art.

## Scope

**In:**
- Portraits as **PNG files** (RGBA, native pixel size, 48×48 for Tiny
  Tales), drawn at the largest whole scale that fits 256×256 console px and
  centred in the frame. `content` reads only each file's size (0231's
  `Content::images`); `app` decodes and draws it. Colours come from the
  image itself, not `palette.ron`.
- A sidecar per character, `assets/portraits/<id>.ron`, mapping our
  expression names to image files, e.g.
  `(character: "knight_a", expressions: { "neutral": "knight_a/neutral.png",
  "happy": "knight_a/laughing.png", … })`. The five required expressions
  must be mapped; extra expressions are allowed, as today.
- Drawing: one `Sprite` per portrait, `dest` = the image at its whole
  scale, centred in the frame. Transparent pixels show the frame's
  background. `dim` becomes the sprite's `opacity` (255 × (1 − dim)), which
  fades every pixel toward the background behind it, as ADR-0018 says, and
  `mirror` is `flip_x`. No per-pixel rectangles or cells (ADR-0038).
- Keep the text `.portrait` format working for the placeholders until 0706
  replaces them, or convert the two placeholders to PNG and remove the text
  format. Choose one and record it in the ADR.
- `cargo xtask portrait-import <zip-or-dir> <faceset-file> <character-id>`:
  slices the chosen 4×2 face set into its 8 faces, reduces each 3×-drawn
  144×144 cell to its 48×48 pixels (error if a cell isn't made of exact 3×3
  blocks), writes them into `assets-private/portraits/<id>/`, and writes a
  sidecar stub to fill in.
- Validation in the loader (all errors at once, with file names): size must
  fit 256×256 console px at a scale of at least 1; missing required expression; unreadable PNG; sidecar
  `character` ≠ file stem.
- ADR (`write-adr`) superseding ADR-0018's portrait section: PNG portraits,
  whole-number scale to fit the frame, drawn as sprite items.

**Out (do not do):** picking faces for the cast (0706); the private assets
repo (0110); theme recolouring of portraits.

## Implementation steps

1. ADR first: the format and the drawing (a sprite is already one line in
   `GlyphBuffer::to_snapshot`, from 0231).
2. `trpg_content::portrait`: the sidecar loader and validator, checking each
   named PNG against `Content::images`. `Content::portraits` stays keyed by
   character id.
3. `trpg_ui::portrait::draw_portrait`: same signature, one sprite item for
   a PNG portrait. Clipping is the sprite's `clip`, as 0231 defines it.
4. Portrait viewer (0703) and dialogue screen (0704) keep working unchanged.
   Check both.
5. The importer xtask, with tests on a small fixture face set (a made-up
   576×288 image, never a bought one).
6. Update `assets/portraits/README.md` and the `ascii-art` skill's portrait
   notes.

## Acceptance criteria

- [ ] A 48×48 PNG portrait loads, validates and is drawn at 5 px per pixel,
      centred in the 32×16-cell area of a dialogue frame (a test checks the
      sprite's `dest`).
- [ ] Dimmed and mirrored drawing tested on the sprite's `opacity` and
      `flip_x`, and looked at in a rendered frame.
- [ ] Every loader error has a test with its message.
- [ ] Dialogue snapshots updated and looked at, and a rendered PNG screenshot
      sent to Nick.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: validate, the sidecar mapping, the whole-scale and centring maths,
  dim, mirror, clipping.
- Property: for any image size that fits, `dest` lies inside the frame and
  its scale is a whole number.
- Snapshot: portrait viewer and dialogue screen with a PNG portrait.

## Completion notes

