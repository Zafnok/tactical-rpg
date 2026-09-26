# ADR-0018: Visual style v2 (after Nick's look sign-off)

- **Status:** Accepted
- **Date:** 2026-09-25
- **Related tickets:** 0011, 0401, 0402, 0403, 0703, 0704, 0706
- **Supersedes:** ADR-0012

## Context

ADR-0012 set the visual rules before Nick had seen anything. In ticket 0011 he
judged real renders and decided the look (`docs/design/look-and-feel.md`).
Three of his decisions don't fit ADR-0012:

1. **Units** are name initials with a thin HP bar under the tile, not
   "class letter + status marker".
2. **The path** is a line through tile centres that ends in an arrowhead, and
   the HP bar is 2 px tall. Neither can be drawn with whole glyph cells.
3. **Portraits** are 32×32 shaded pixel art, not 24×12 ASCII art. Line-art and
   block-shading ASCII were both rejected.

This ADR restates ADR-0012's rules that still hold and replaces the rest, so
screen tickets read one document.

## Decision

### Console, cells and tiles (unchanged from ADR-0012)

- A fixed logical console of **100 × 32 cells**, each **8 × 16 px**. It's
  integer-scaled and letterboxed (details in ADR-0016).
- A map tile is **2 cells wide × 1 tall = 16 × 16 px**. `x_cell = 2 * x_tile`,
  in exactly one helper (`tile_to_cell`, 0401).

### Colour

- Every cell has 24-bit fg and bg colours. Code uses **palette names**, never raw
  RGB. The palette is `assets/data/palette.ron`, currently mood D from the
  design doc.
- Terrain has two names: `<terrain>` for the glyph colour and `<terrain>_bg`
  for the background tint. Overlays (`move_range`, `attack_range`,
  `heal_range`, `danger_zone`) blend over the terrain background at about 75%,
  a constant in `ui`.
- Faction colours are unchanged: player blue, enemy red, ally green, neutral
  yellow.
- **Never colour alone** (unchanged): every colour-coded state also has a shape
  or glyph difference. Acted = lowercase letters. HP = bar *length*.
- Dimming ("acted", listener portrait) **lerps toward the background colour**,
  not toward black, so it also works for the light theme E planned in 0806.
- Colour themes will be alternative palette files selected in options (0806).
  Code keeps using names, so a theme is a data swap.

### Sub-cell overlays

`GlyphBuffer` gains an overlay list: `Overlay { rect: PxRect, color: Rgb,
layer: Under | Over }`, in **console pixels** (the 800×512 logical space before
scaling). `Under` draws after cell backgrounds and before glyphs. `Over` draws
after glyphs. `blit` offsets overlays, and clipping follows the buffer bounds.
The app renderer draws them as scaled rectangles, so they stay crisp at any
integer scale.

- Uses: the unit HP bar (`Over`, bottom 2 px of the tile) and the movement path
  line (`Under`, 3 px wide through tile centres, starting at the unit tile's
  edge), plus the arrowhead (`Over`, stacked 1-px rects forming a triangle).
- Snapshot text (ADR-0007) lists overlays after the glyph grid, one per line
  (`over  x,y wxh  hp_mid`), so tests still review them as text.
- Only axis-aligned rectangles. Anything fancier is art, and art goes in
  portraits.

### Units on the map

- Two cells: the unit's **map label** (two letters). By default it's the first
  two letters of the character's name, or of the class name for generic
  enemies. Content can override it, and a duplicate label within one faction
  on a map is a content validation error.
- Fg = faction colour. Bg = terrain background (or overlay). No faction backing.
- Acted: label lowercased and dimmed.
- HP bar: `Over` overlay at the tile's bottom 2 px. Width = `round(16 × hp/max)`,
  colour `hp_high` (> 2/3), `hp_mid` (> 1/3) or `hp_low`, and the unfilled part
  drawn in `black`.

### Cursor and path

- Browsing: `[` `]` in the cells either side of the tile (overwriting the
  neighbouring tiles' inner glyphs while shown), fg `cursor`, brightness pulsing
  between 100% and ~50% (never off).
- Selected unit, cursor on it: `►` `◄` in the same cells.
- Cursor away from a selected unit: path line + arrowhead overlays in `path`
  colour. No cursor frame is drawn on the destination.

### Portraits

- A portrait expression is a **32×32 grid of colour keys** (`.` =
  transparent), one key per pixel. It's drawn as **32×16 cells**: each cell is
  `▀` with fg = the top pixel and bg = the bottom pixel. `▄` or a space is used
  when a pixel is transparent. Pixels are square (8×8 screen px).
- The key legend maps keys to palette names (skin, hair, metal… colours are
  added to `palette.ron` as portraits need them). There's no glyph layer; the
  art is all colour.
- Mirroring is exact: reverse each row. `draw_portrait` supports it, so a
  listener on the right can face the speaker.
- Required expressions (unchanged): `neutral`, `happy`, `angry`, `sad`,
  `surprised`.
- Portraits appear **only in conversations**, not in the battle side panel. The
  combat screen uses full-body art (0413).

### UI chrome and layout (unchanged from ADR-0012)

- Single-line boxes `┌─┐` for panels, double-line `╔═╗` for focus/modal (a
  selected unit's panel, the speaker's portrait frame).
- Battle screen: map viewport left (70 cells), side panel right (30 cells),
  2-row help bar at the bottom. The panel shows the tile and unit under the
  cursor.
- Dialogue: two portraits (speaker bright, listener dimmed), name plates, a
  3-line text box with typewriter reveal. Positions are in 0704, sized for
  32×16-cell portraits.

### Font (unchanged)

Terminus 8×16 per ADR-0016. The half blocks `▀ ▄` are already in the atlas.

## Consequences

- Units, cursor and path are fully specified. 0401–0403 follow this ADR and the
  design doc, not ADR-0012's placeholders.
- `GlyphBuffer`, `blit`, the snapshot format and the app renderer each gain
  overlay support. That's one small, well-contained change, done in 0401.
- Portrait files are pixel grids, so they're easier to author and review than
  glyph art, but they're larger (32 lines per expression). The portrait tickets
  (0703/0706) change format accordingly.
- Portraits are less "pure ASCII" than the original plan. Nick chose this
  knowingly for readable, expressive faces.
- The mockups for 0011 were rendered by a throwaway tool outside the repo, with
  the real font atlas. No debug code was added to the game.

## Alternatives considered

- **Keep ADR-0012 unchanged and draw the HP bar with glyphs (`▁`, partial
  blocks in the tile's cells)** — lower-block glyphs replace the unit's letters,
  and a bar under the label needs a second row. Sub-cell rects are simpler and
  look like what Nick approved.
- **Portraits as glyph art with a colour layer (ADR-0012)** — Nick rejected both
  line-art and block-shaded samples as unexpressive.
- **Portraits as PNG images** — leaves the glyph pipeline and needs an image
  decoder and new asset rules. Half-block pixel grids get the same look within
  the existing renderer.
- **Quadrant blocks (`▖▗▘▝…`) for 2×2 pixels per cell** — only two colours per
  cell, so colour boundaries fringe. Half-blocks give full colour per pixel.
