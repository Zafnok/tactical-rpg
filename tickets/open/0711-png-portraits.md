---
id: "0711"
title: PNG portraits drawn as pixel overlays (48×48 bought faces)
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0021"]
nick_input: sign-off
completed:
---

# 0711 — PNG portraits drawn as pixel overlays

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
colours at that grain, so these portraits are drawn with the pixel overlays
the cursor already uses (ADR-0024: `GlyphBuffer::add_overlay`, `Overlay`,
`Layer`).

The store's face sets are RPG Maker MV/MZ files: a **576×288 sheet, a 4×2
grid of 144×144 cells**, each face drawn at 3× (so 48×48 underneath). Check
this on the bought files before writing the importer; the store pages don't
state it.

Keep the overlay drawing general (any PNG size, any whole scale): 0413
reuses it for the big still battle images.

## Nick input

**Sign-off:** a screenshot of the test scene (F2 → Play test scene) with two
bought portraits, speaker and dimmed listener, next to one with the old art.

## Scope

**In:**
- Portraits as **PNG files** (RGBA, native pixel size, 48×48 for Tiny
  Tales), drawn at the largest whole scale that fits 256×256 console px and
  centred in the frame, decoded at load with the `png`
  crate (MIT/Apache, already in `Cargo.lock`). Colours come from the image
  itself, not `palette.ron`.
- A sidecar per character, `assets/portraits/<id>.ron`, mapping our
  expression names to image files, e.g.
  `(character: "knight_a", expressions: { "neutral": "knight_a/neutral.png",
  "happy": "knight_a/laughing.png", … })`. The five required expressions
  must be mapped; extra expressions are allowed, as today.
- Drawing: one overlay per horizontal run of same-coloured pixels, each
  one scale step tall (5 px for a 48×48 face). Transparent pixels (alpha < 128) draw nothing. `dim` lerps each
  colour toward the frame's background (as ADR-0018 says), and `mirror`
  flips each row.
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
  whole-number scale to fit the frame, drawn as overlays.

**Out (do not do):** picking faces for the cast (0706); the private assets
repo (0110); theme recolouring of portraits.

## Implementation steps

1. ADR first: format, drawing and the snapshot representation of overlays
   (they already appear in `GlyphBuffer::to_snapshot`).
2. `trpg_content::portrait`: the PNG and sidecar loader and validator.
   `Content::portraits` stays keyed by character id.
3. `trpg_ui::portrait::draw_portrait`: same signature, overlay drawing for
   PNG portraits. Clipping is as today.
4. Portrait viewer (0703) and dialogue screen (0704) keep working unchanged.
   Check both.
5. The importer xtask, with tests on a small fixture face set (a made-up
   576×288 image, never a bought one).
6. Update `assets/portraits/README.md` and the `ascii-art` skill's portrait
   notes.

## Acceptance criteria

- [ ] A 48×48 PNG portrait loads, validates and is drawn at 5 px per pixel,
      centred in the 32×16-cell area of a dialogue frame (a test checks the
      overlay bounds).
- [ ] Dimmed and mirrored drawing tested against the source pixels.
- [ ] Every loader error has a test with its message.
- [ ] Dialogue snapshots updated and looked at, and a rendered PNG screenshot
      sent to Nick.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: decode and validate, the sidecar mapping, run merging, dim, mirror,
  clipping.
- Property: drawing covers exactly the opaque pixels (no gaps or overlaps).
- Snapshot: portrait viewer and dialogue screen with a PNG portrait.

## Completion notes

