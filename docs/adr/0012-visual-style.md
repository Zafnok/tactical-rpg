# ADR-0012: Visual style — cells, tiles, colour, portraits

- **Status:** Accepted (look-and-feel details pending Nick's sign-off, ticket 0011)
- **Date:** 2026-09-25

## Context

ASCII does not mean low clarity: Dwarf Fortress, Brogue and Caves of Qud show
that coloured glyphs with deliberate backgrounds read very well. We need rules
so every ticket draws in the same style.

## Decision

### Console and cells

- The game draws a **fixed logical console**; starting size **100 × 32 cells**,
  defined as constants in `ui` so it can be tuned.
- A cell is **8 × 16 logical pixels** (classic terminal proportions) so text
  reads naturally.
- The console is scaled by the **largest integer factor** that fits the window
  (crisp pixels), centred with letterboxing. 100×32 cells = 800×512 px →
  2× = 1600×1024, fitting a 1080p screen.

### Map tiles are two cells wide

A map tile is **2 cells wide × 1 cell tall = 16 × 16 px**, i.e. square. This
keeps distances on the map visually honest (a tactics requirement) while text
stays in narrow cells. Two glyphs per tile also allow richer terrain:
`♣♣` forest, `^^` mountain, `≈≈` water, `..` plain, `▓▓` wall.

Units occupy one tile. Default unit rendering: glyph 1 = a class letter or
symbol, glyph 2 = a status marker (e.g. `·` ready, `ˇ` has acted, `!` low HP) —
final choice confirmed in ticket 0011.

### Colour

- Every cell has 24-bit RGB **foreground and background**.
- Colours are **named in a palette file** (`assets/data/palette.ron`); code
  refers to names (`player`, `enemy`, `move_range`), never raw RGB.
- Faction colours: player **blue**, enemy **red**, ally **green**, neutral
  **yellow**.
- Overlays use **background tints**: movement range = blue bg, attack range =
  red bg, heal/support range = green bg, danger zone = dark red bg. The terrain
  glyph stays visible on top.
- Units that have acted are **desaturated/dimmed**.
- **Never colour alone:** every state conveyed by colour also has a glyph or
  pattern difference, for colour-blind players. A colour-blind palette variant
  is a later ticket.

### UI chrome

- Panels and menus use box-drawing characters (single line `┌─┐` for panels,
  double line `╔═╗` for focus/modal).
- Standard layout for the battle screen: map viewport on the left, a side panel
  (~30 cells) on the right for tile/unit info, a 2-row message/help bar at the
  bottom showing the currently valid keys.

### Portraits and dialogue

- Portraits are ASCII art, **24 × 12 cells** (192 × 192 px, square), coloured
  per glyph, with **named expressions** (at minimum: `neutral`, `happy`,
  `angry`, `sad`, `surprised`).
- Dialogue shows **two portraits at once**, left and right. The speaker is at
  full brightness with their name plate; the listener is dimmed. A 3-line text
  box sits beneath, with typewriter reveal.
- Portrait art rules live in the `ascii-art` skill.

### Font

A bitmap font covering ASCII, CP437 box-drawing/blocks/symbols and Latin-1,
with a licence allowing commercial redistribution (OFL, CC0, CC-BY, MIT).
Selected in ticket 0203; the licence file ships in `assets/fonts/`.

## Consequences

- Every screen ticket uses palette names and the layout above.
- Map code must convert tile coordinates ↔ cell coordinates (`x_cell = 2 * x_tile`).
- If Nick prefers another look in ticket 0011, the result is a superseding ADR
  and mostly data/constant changes.

## Alternatives considered

- **One square cell (e.g. 16×16) for everything, DF-style** — maps look great,
  but text is wide and awkward, and dialogue fits far fewer words per line.
- **Graphical tileset** — not ASCII; out of scope by request.
