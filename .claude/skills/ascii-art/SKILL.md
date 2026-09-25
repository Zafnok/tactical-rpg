---
name: ascii-art
description: Author ASCII visual content — character portraits, map terrain glyphs, UI mockups, title screen art — following the project's visual style (ADR-0012). Use for portrait files, map files, and any mockup shown to Nick.
---

# ASCII art

Read `docs/adr/0012-visual-style.md`. Key facts: cells are 8×16 px (twice as
tall as wide), map tiles are 2 cells wide, every cell has fg + bg colour from
named palette entries.

## Cell aspect ratio

Because cells are tall, a shape that looks square in a text editor with a
square-ish font looks **tall** in game. To draw a circle-ish face, make it about
twice as many columns as rows. Portraits are 24 columns × 12 rows and should
read as square.

## Portraits

- Canvas: exactly 24×12. Pad with spaces; keep trailing spaces (the repo's
  `.editorconfig` preserves them under `assets/`).
- Composition: head and shoulders, face in the upper-middle, eyes on row 4–5.
  Leave one blank column on each side so the frame doesn't touch the art.
- Distinguish characters by **silhouette first** (hair shape, headgear, collar,
  weapon hilt over shoulder), colour second.
- Expressions change as few cells as possible (eyes, brows, mouth) so they read
  as the same person. Required: `neutral`, `happy`, `angry`, `sad`, `surprised`.
- Useful glyphs: `` / \ | _ - ( ) ' ` , . `` for lines; `░ ▒ ▓ █ ▀ ▄ ▌ ▐` for shading
  and fills; `o O 0 @ * ^ v ~` for features. Only use glyphs present in the
  font chosen in ticket 0203 (see `assets/fonts/README.md` once it exists).
- Colour layer: same 24×12 grid of single-character colour keys defined in the
  file's legend (e.g. `s` = skin, `h` = hair, `a` = armour, `.` = default).
  Keep to 4–6 colours per portrait; use background colour sparingly.
- After authoring, render it (the portrait viewer from ticket 0703, or a snapshot
  test) and look at it — don't commit art you haven't seen rendered.

## Map terrain

- Two glyphs per tile. Terrain must be readable **without** colour (different
  glyph shapes), and distinct **with** colour.
- Busy textures (`♣♣`, `≈≈`) for costly terrain, calm ones (`..`, `  `) for
  open ground, so the eye reads the map's flow.
- Units must always stand out over terrain: units use bright fg; terrain uses
  mid/dark fg.

## Mockups for Nick

When asking Nick about anything visual, show an ASCII mockup in a fenced block,
at real proportions (remember tiles are 2 characters wide), and describe the
colours in words next to it, since chat can't show them. Where possible, also
render it in-game and share a screenshot.
