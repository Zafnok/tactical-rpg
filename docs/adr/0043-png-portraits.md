# ADR-0043: Portraits are PNG files named by a sidecar, drawn as one sprite at a whole scale

- **Status:** Accepted
- **Date:** 2026-10-02
- **Related tickets:** 0711, 0021, 0039, 0231, 0232, 0413, 0703, 0704, 0706
- **Supersedes in part:** ADR-0018 (*Portraits*: the 32×32 grid of colour
  keys, the half-block drawing, the palette legend). ADR-0018's other
  portrait rules stand: the five required expressions, portraits only in
  conversations, dimming toward the background, exact mirroring.
- **Follows:** ADR-0032 (portraits are bought art), ADR-0038 (a picture is a
  sprite item), ADR-0040 (bought files live in the private repository)

## Context

- Nick bought Mega Tiles' Tiny Tales art (0021) and chose the **busts at
  4×, filling the frame** for dialogue (0039, `look-and-feel.md`, *Dialogue
  portraits*). A bought bust is 80×80 pixels; its middle 64 columns and
  bottom 64 rows at 4× are exactly the dialogue frame's 256×256 console
  pixels (32×16 cells).
- ADR-0018's portraits are text: a 32×32 grid of palette keys, drawn as
  half-block cells, 8×8 screen pixels per portrait pixel. Cells can't hold
  a 64×64 picture with free colours.
- ADR-0038 gave the frame sprite items (0231) and says no picture is drawn
  as per-pixel cells or rectangles again; the text portraits "go when 0711
  lands".
- 0231's image table (`Content::images`) already holds the size of every
  PNG in the bundle, read from its header. `content` decodes no pixels.
- The bought files can't be in this repository (ADR-0032, ADR-0040), and
  every gate must pass without them.
- Combat pictures (0413) need the same "fit a picture in a box" drawing.

## Decision

### 1. A portrait is a sidecar file and its images

`assets/portraits/<id>.ron`, one per character (the lead has `lead_m` and
`lead_f`, as before):

```ron
(
    character: "knight_a",
    expressions: {
        "neutral": "knight_a/neutral.png",
        "happy": "knight_a/smile.png",
        "angry": "knight_a/stern.png",
        "sad": "knight_a/sad.png",
        "surprised": "knight_a/surprise.png",
        "sly": "knight_a/sly.png",
    },
)
```

- `character` is the file stem.
- `expressions` maps **our** expression names to image files, relative to
  `assets/portraits/`. By convention a character's images are in
  `assets/portraits/<id>/`. Two names may share one image.
- The five required expressions must be mapped. More are allowed, and
  dialogue may name them.
- Images are PNG, RGBA, at their native pixel size. Their colours are their
  own: portraits no longer use `palette.ron`, and colour themes (0806)
  don't recolour them.

`trpg_content::portrait` loads the sidecars after the image table and keeps
`Content::portraits` keyed by character id. A `Portrait` is its character
and its `Expression`s (name, `ImageId`, size), the required five first in
their fixed order, then the others by name.

**Checked by the loader**, all at once, each with its file (and line, for
an expression):

- a RON syntax error or an unknown field;
- `character` ≠ the file stem;
- a required expression that isn't mapped;
- an expression whose file isn't a PNG image in the bundle under
  `portraits/` (a file named `.png` that can't be read is also reported by
  the image table, under its own name);
- an image wider or taller than 256 pixels: it must fit the frame at a
  scale of at least 1.

### 2. It is drawn as one sprite item, at the largest whole scale that fits

`trpg_ui::portrait::draw_portrait(buf, (x, y), portrait, expr, dim,
mirror)` adds **one** `Sprite` and changes no cell:

- The frame is the 32×16 cells from `(x, y)`: 256×256 console pixels.
- `dest` is the whole image at scale `min(256 / width, 256 / height)`
  (whole-number division), centred in the frame; a leftover odd pixel goes
  to the right or the bottom. A 64×64 cut bust is 4× and fills the frame; a
  48×48 face is 5× with an 8-pixel margin; the 32×32 placeholders are 8×.
- `layer` is `Under`: the picture lies over the cells' backgrounds and
  under their glyphs. The caller clears the frame first (every screen
  already does), so a transparent pixel shows the frame's background.
- `dim` becomes `opacity = round(255 × (1 − dim))`. The renderer blends
  the picture over what is behind it, which is ADR-0018's "lerp toward the
  background".
- `mirror` is `flip_x`.
- Clipping is the sprite's `clip` (0231): `dest` never moves or rescales.

`fit_whole_scale(size, frame)` is the general part (any image size, any
frame): 0413 uses it for the combat pictures.

### 3. The text format is removed; the placeholders became PNG files

The four placeholders (`test_lord`, `test_knight`, `lead_m`, `lead_f`) were
converted once, pixel for pixel, to 32×32 PNG files with a sidecar each.
At 8× they look exactly as before. The `.portrait` parser, its palette
legend, the 26 portrait-only palette colours and the debug sampler's rule
that hid them are deleted. The placeholders stay in this repository for
good: they are what every gate and every clone without the bought art
shows (ADR-0040).

### 4. Bought busts come in through `cargo xtask portrait-import`

```
cargo xtask portrait-import <bust-folder-or-sheet> <character-id> [--shift-x <pixels>]
```

- A **folder** of single files: its `bust_<expression>.png` files (not
  `bust_sheet.png`); if it has none, its `face_…` files; else every PNG.
  The expression is what follows the file name's last `_`, in lower case.
- A **sheet**: 320×160 (busts) or 192×96 (faces), 4×2, in the order
  `neutral smile sad sly thinking stern surprise unique` (checked against
  the single files of all 16 bought heroes).
- An 80×80 bust is cut to x 8..72, y 16..80. `--shift-x` (−8..=8) moves
  the cut sideways for a bust that sits off-centre. A 48×48 face is
  written whole. Any other size is an error that names the file, and
  nothing is written.
- It writes `assets-private/game/portraits/<id>/<expression>.png` and, only
  if there is none yet, a sidecar `assets-private/game/portraits/<id>.ron`
  that the game loads as it is: each required expression points at the
  closest bought one by name (`happy` → `smile`, `angry` → `stern`,
  `surprised` → `surprise`), and every bought expression is also listed
  under its own name. Ticket 0706 corrects the mapping per character.

The private files replace the public ones at the same path (ADR-0040), so
a bought portrait for a character is its sidecar and image folder in
`assets-private/game/portraits/`.

## Consequences

- A portrait costs one line in a snapshot instead of 16 rows of half
  blocks; dialogue snapshots show which image, where, flipped and how dim.
- Portrait tests assert on the sprite (`dest`, `flip_x`, `opacity`), not on
  cells. Whether a picture *looks* right is checked by eye in a rendered
  frame (`cargo xtask frame-png`).
- `content` cannot check an image's pixels (it never could decode them): a
  wrong picture in the right size passes the loader.
- A portrait's pixel is 4, 5 or 8 console pixels wide, no longer always 8:
  bought and placeholder art look different in scale, which is the point.
- Portraits no longer follow the palette or a colour theme.
- The 3×-size RPG Maker sheets (576×288) are not read by the importer: the
  bought packs have the 1× files, so nothing needs them.
- Screens that draw over a portrait's cells with `fill_rect` or `blit` cut
  the sprite (0231), as they cut overlays; nothing special is needed.

## Alternatives considered

- **Keep the text format beside the PNG one until 0706** — two loaders,
  two drawing paths and two kinds of snapshot for four placeholder files.
  ADR-0038 already says the text portraits go with this ticket.
- **Generate the placeholder PNGs with an `xtask` (as the test card is)**
  — they are hand-made art, not a formula; the text files were their only
  source and are gone. They are ordinary committed assets.
- **Name images by convention (`<id>/<expression>.png`), no sidecar** —
  the bought packs have 8 expressions with other names (`smile`, `stern`),
  and 0706 must choose which stands for each of ours without renaming or
  copying bought files.
- **Store the cut busts at 4× (256×256)** — sixteen times the bytes in the
  binary for no gain; nearest-pixel scaling at a whole scale is exact.
- **Cut the bust at draw time (keep the 80×80 file, give the sprite a
  `src`)** — the cut (and a per-character shift) would have to live in the
  sidecar and be checked by the loader; cutting once in the importer keeps
  the game's rule to "draw the whole image".
- **Fractional scales to fill the frame with any size** — blurs or
  unevenly doubles pixels; `app` samples the nearest pixel (0231).
- **`Over` layer** — text printed on those cells later would be hidden by
  the picture; under the glyphs, cells keep the last word.
