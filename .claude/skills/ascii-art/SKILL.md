---
name: ascii-art
description: Author visual content — character portraits (32×32 shaded pixel art in half-block cells), map terrain glyphs, UI mockups, title screen art — following the project's visual style (ADR-0018, docs/design/look-and-feel.md). Use for portrait files, map files, and any mockup shown to Nick.
---

# ASCII art

Read `docs/adr/0018-visual-style-v2.md` and `docs/design/look-and-feel.md`
(Nick's decisions, with screenshots in `docs/screenshots/0011-*.png`). Key
facts: cells are 8×16 px (twice as tall as wide), map tiles are 2 cells wide,
every cell has fg + bg colour from named palette entries, and the palette is
mood D "Earthy painterly".

## Cell aspect ratio

Because cells are tall, a shape that looks square in a text editor looks
**tall** in game. For glyph art (title screens, UI), make shapes about twice as
many columns as rows. Portraits avoid the problem: they are pixel grids (below).

## Portraits

Nick rejected both line-art ASCII and block-shaded ASCII faces as unexpressive.
Portraits are **shaded pixel art**:

- Canvas: a **32×32 grid of colour keys**, one per pixel, `.` = transparent.
  The game draws it as 32×16 cells of `▀` (fg = top pixel, bg = bottom pixel),
  so pixels are square. See the format in ticket 0703 / `assets/portraits/README.md`.
- Composition: head and shoulders; face in the upper-middle (hair from row
  ~1, brows ~12, eyes ~13–14, mouth ~20, chin ~23, shoulders from ~26); a dark
  outline (`K`-like key) around hair and face.
- **Shading:** light from the **upper left**. Mid-tone down the right side of the
  face and jaw; shadow under the fringe, nose, lower lip and chin; soft highlight
  on the left cheek and nose bridge; darker neck under the chin. Keep it
  subtle, and **lighter still on young characters**. Nick said heavy shade
  lines read as wrinkles and age the face.
- **Expressive and human:** Nick wants clear emotion. Expressions change only
  brows, eyes and mouth pixels so they read as the same person. Required:
  `neutral`, `happy`, `angry`, `sad`, `surprised`. Useful moves: one brow
  raised (confident), eyes squeezed into arcs + open smile with teeth (happy),
  brows slammed down and in + bared teeth (angry), brows up in the middle +
  downturned mouth + a tear (sad).
- Distinguish characters by **silhouette first** (hair shape, headgear,
  collar, weapon over the shoulder), colour second.
- Keep to a small key set per portrait: outline, hair ×3 (base, highlight,
  shadow), skin ×3–4 (base, mid, shadow, highlight), eyes, mouth ×2, clothes
  ×3–4. Keys map to palette names.
- **No mini-portraits.** Portraits appear only in conversations (Nick dropped
  the battle-panel mini portrait: at 16×16 it looked like a meme face).
- After authoring, render it (the portrait viewer from ticket 0703, or a
  snapshot) and **look at it**. Don't commit art you haven't seen rendered.
  Nick iterates character by character, so expect revisions.

## Map terrain

- Two glyphs per tile. Terrain must be readable **without** colour (different
  glyph shapes), and distinct **with** colour: glyph in `<terrain>`,
  background in `<terrain>_bg`.
- Busy textures (`♣♣`, `≈≈`, `^^`) for costly terrain, calm ones (`..`) for
  open ground, so the eye reads the map's flow.
- Units (two-letter name labels in faction colour, thin HP bar under the tile)
  must stand out over terrain: units use bright fg; terrain uses mid/dark fg.

## Mockups for Nick

Nick judges **rendered images**, not ASCII in chat. Render mockups with the
game's real font atlas at in-game size (a throwaway tool outside the repo is
fine: `trpg-content` gives the atlas and `trpg-ui` gives `GlyphBuffer`; blit
cells to a PNG at 2×). Look at every render yourself before sending it, and fix
overlaps, cut-off text and invented details (no stats or rules that aren't in
`docs/design/`). Offer genuinely different options, then iterate on his
comments. He often asks for more options or a combination.
