---
id: "0011"
title: "Sign-off: look and feel (glyphs, colours, unit markers, portraits)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0203"]
nick_input: decision
completed:
---

# 0011 — Sign-off: look and feel

## Context

ADR-0012 sets the visual rules (8×16 cells, 2-cell-wide square map tiles,
named palette, two-portrait dialogue). Nick said ASCII shouldn't mean poor
clarity; he should see and approve the look before we build many screens on
it. By the time this runs, ticket 0203 has a working font and a glyph/palette
sampler screen to screenshot. Use the `ask-nick` and `ascii-art` skills.

## Nick input

**Decision** — pick options and comment on screenshots.

## Preparation (before asking)

1. Run the game's glyph/palette sampler (from 0203) and take a screenshot.
2. Build three **static mockups** of a small battle scene (~12×8 tiles, 3 player
   units, 3 enemies, one forest patch, water, a mountain, a move-range overlay,
   the cursor, and the side panel). Render them for real if the sampler makes
   that easy (a temporary debug screen is fine and must be removed or kept behind
   a `debug` feature flag); otherwise ASCII in chat with colour described.

## Questions to ask

### Q1. Colour mood

**A. Earthy & muted (Dwarf Fortress classic)** — browns, dull greens, soft reds.
**B. Vivid & high-contrast (Brogue)** — deep blacks, glowing lights, saturated overlays.
**C. Rich & painterly (Caves of Qud)** — distinct hues per material, teal/amber accents.

### Q2. How units look on the map (tile = 2 glyphs)

**A. Class letter + status marker** — `S·` (Swordfighter, ready), `Lˇ` (Lancer,
already acted). Colour = faction.
**B. Symbol + class letter** — `@S`, `@L` (roguelike `@` for people).
**C. Name initials** — `Al`, `Ce`; class shown in the side panel.

### Q3. Cursor style

**A. Bright inverted tile** (swaps fg/bg). **B. Blinking bracket frame** drawn in
the neighbouring half-cells `[S·]`. **C. Coloured background pulse.**

### Q4. Portrait style

Show one sample portrait (24×12) in two styles: **A. line-art** (`/ \ | _`),
**B. shaded blocks** (`░▒▓█`). Nick picks or mixes.

## What to record

`docs/design/look-and-feel.md`: Nick's choices and comments; the resulting
palette values (update `assets/data/palette.ron` created by 0201); unit
marker, cursor and portrait conventions. If anything contradicts ADR-0012,
write a superseding ADR (`write-adr`). Update the `ascii-art` skill if the
conventions changed.

## Acceptance criteria

- [ ] Nick saw a real in-game screenshot and the mockups.
- [ ] Nick answered Q1–Q4.
- [ ] `docs/design/look-and-feel.md` written; palette/ADR/skill updated as needed.
- [ ] No leftover debug code outside a `debug` feature.
- [ ] Ticket archived.

## Completion notes

