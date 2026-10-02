---
name: ascii-art
description: Author visual content — map terrain glyphs, UI mockups, title screen art, and the small edits allowed on bought portraits and battle art — following the project's visual style (ADR-0018, ADR-0032, docs/design/look-and-feel.md). Use for map files, portrait edits, and any mockup shown to Nick.
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
many columns as rows. Bought portraits and battle images are PNGs with
square pixels, drawn as sprite items (0229, 0711; ADR-0038), so the problem
doesn't apply to them.

## Portraits and battle art: bought, not drawn

Nick doesn't want Claude-drawn character art (ticket 0021). Dialogue faces
and combat-screen battle images are **bought**: Mega Tiles' Tiny Tales
packs (`docs/design/look-and-feel.md`, *Portraits and battle art*;
ADR-0032). Claude **never draws** a portrait, a face or a battle image.

- **Allowed edits** on a bought image, and nothing more: recolour hair,
  clothes or eyes (swap a few exact colours), and small pixel edits (a scar,
  spectacles, a missing expression such as `surprised` made from an
  existing face). New hairstyles, removing beards, new clothes or bodies are
  redrawing: not allowed.
- **Characters no bought face fits:** Mega Tiles' Character Generator (if
  bought and its licence allows it), then small edits (0706). Anything else
  goes to Nick.
- **Bought files never go in this repo** (0110). Mockups made from store
  previews stay in the scratchpad.
- Required expressions stay `neutral`, `happy`, `angry`, `sad`,
  `surprised`, mapped from the pack's 8 per character.
- **No mini-portraits.** Portraits appear only in conversations (Nick dropped
  the battle-panel mini portrait: at 16×16 it looked like a meme face).
- After any edit, render it in the dialogue screen and **look at it**.
  Don't commit art you haven't seen rendered.

The old 32×32 text portraits (`assets/portraits/*.portrait`) are
placeholders only, until 0711 and 0706 replace them.

## Map terrain

These are the rules of the **glyph skin**, the game's default map look
(ADR-0038). Another skin (a tileset of sprites, 0433) has its own data and
doesn't change these files.

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
