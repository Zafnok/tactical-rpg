---
id: "0711"
title: PNG portraits drawn as sprite items (bought busts at 4×)
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0021", "0110", "0116", "0231", "0232"]
nick_input: sign-off
completed:
---

# 0711 — PNG portraits drawn as sprite items

## Context

**Changed 2026-10-02 (ticket 0039):** Nick chose the **busts at 4×,
filling the frame**, over the 48×48 faces at 5× (`look-and-feel.md`,
*Dialogue portraits*; mockups H4 and H5 in the bought-art folder's
`spike-renders/`). A bought bust is 80×80. The importer cuts it to its
**middle 64 columns and bottom 64 rows** (x 8..72, y 16..80) and writes
that 64×64 file; drawn at the largest whole scale that fits, 4×, it is
exactly the frame's 256×256 console px, with no margin. Where the text
below says a 48×48 face at 5×, read a 64×64 cut bust at 4×; the drawing
rule itself (largest whole scale that fits, centred) is unchanged.

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

**Checked on the bought files (2026-10-02, ticket 0038).** Each Heroes
pack has `Facesets/Original`, `Facesets/2X` and `Facesets/3X`. The 576×288
sheet is the **3X** copy. `Original` already holds what we want, so the
importer needs no 3× reduction:

- `Facesets/Original/Portraits/<Hero>_<Expression>.png`: one **48×48** file
  per expression. The 8 expressions are `Neutral`, `Smile`, `Stern`, `Sad`,
  `Surprise` (two files in Heroes 2 are spelled `Surprised`), `Thinking`,
  `Sly`, `Unique`.
- `Facesets/Original/<Hero>.png`: the same 8 as a **192×96** sheet, 4×2.
- `Facesets/Original/Busts/Bust_<Hero>_<Expression>.png`: an **80×80**
  bust in the same 8 expressions (320×160 as a sheet). **This is what
  dialogue uses** (ticket 0039), cut to 64×64.
- The Character Generator exports the same two layouts (a 192×96 face
  sheet with "Crop Face" on, a 320×160 bust sheet with it off), at 1×.

So `portrait-import` takes either a folder of single 80×80 bust files or
a 320×160 bust sheet (4×2) at 1×, and cuts each bust to 64×64. A bust
that sits off-centre may be cut a few columns to one side instead: an
optional `--shift-x <pixels>` (−8..=8) moves the cut. It still accepts
48×48 faces (written as they are), and keeps the 3×-sheet path only if it
costs nothing. The heroes' files are already sorted per character in the
private assets repository (ADR-0040; `cargo xtask private-assets
--library`): `assets-private/library/tiny-tales/characters/heroes/
<Name>/face_<expression>.png` and `bust_<expression>.png`.

Keep the drawing general (any PNG size, any whole scale): 0413 reuses it
for the big still battle images.

**Needs 0110 first** (added to `blocked_by` 2026-10-01): the importer reads
the bought files and writes into `assets-private/game/`, the face-set layout must
be checked on the bought files, and the sign-off shows two bought
portraits. All of that needs Nick's purchase and the private assets folder
from 0110, uploaded by Nick (**0116**, added to `blocked_by` 2026-10-02:
until then `cargo xtask private-assets` can't fetch it in a worktree).

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
- `cargo xtask portrait-import <bust-folder-or-sheet> <character-id>
  [--shift-x <pixels>]`: takes a folder of single 80×80 bust files or a
  320×160 bust sheet (4×2) at 1×, cuts each bust to its middle 64 columns
  and bottom 64 rows (x 8..72, y 16..80; `--shift-x`, −8..=8, moves the
  cut sideways), writes the 64×64 files into
  `assets-private/game/portraits/<id>/`, and writes a sidecar stub to fill
  in. 48×48 faces are accepted and written as they are; the 3× sheet path
  only if it costs nothing (see *Context*). Error, with the file name, on
  any other size.
- The sign-off screenshot is made with `cargo xtask frame-png` (0232).
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

- [ ] A 64×64 PNG portrait loads, validates and is drawn at 4 px per pixel,
      filling the 32×16-cell area of a dialogue frame; a 48×48 one is drawn
      at 5 px per pixel, centred (tests check the sprite's `dest`).
- [ ] The importer cuts an 80×80 bust to its middle 64 columns and bottom
      64 rows, and `--shift-x` moves the cut (tests on a made-up image).
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

