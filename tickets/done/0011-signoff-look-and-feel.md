---
id: "0011"
title: "Sign-off: look and feel (glyphs, colours, unit markers, portraits)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: ["0203"]
nick_input: decision
completed: 2026-09-25
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

- [x] Nick saw a real in-game screenshot and the mockups.
- [x] Nick answered Q1–Q4.
- [x] `docs/design/look-and-feel.md` written; palette/ADR/skill updated as needed.
- [x] No leftover debug code outside a `debug` feature.
- [x] Ticket archived.

## Completion notes

- **Nick's decisions** are in `docs/design/look-and-feel.md`:
  - palette **D "Earthy painterly"**, with per-terrain backgrounds;
  - units drawn as **name initials** in faction colour (lowercase once acted)
    with a **thin HP bar** under the tile;
  - a **pulsing bracket** cursor, `►◄` on a selected unit;
  - an FE-style **path line with one arrowhead**;
  - **32×32 shaded pixel-art portraits** in conversations only.
- **How the mockups were made:** rendered by a throwaway tool in the session
  scratchpad, using the game's real font atlas and `GlyphBuffer`, at 2× (the
  same pixels the game draws). Nick saw `docs/screenshots/0203-glyph-sampler.png`
  (the real game) plus about 20 mockups over several rounds. Q1–Q3 each needed
  more options than the ticket listed:
  - colour moods D–G were added;
  - class-symbol unit styles were tried and rejected;
  - four more cursor styles were compared;
  - the full battle screen was used to judge initials.

  The final renders are committed as `docs/screenshots/0011-*.png`. No
  debug code was added to the game, so nothing needs a `debug` feature.
- **Deviations from the ticket:**
  - The mockups were rendered outside the game, not in a debug screen.
  - **Q4:** Nick rejected both the line-art and block-shaded portrait styles.
    The result is half-block pixel art, which contradicts ADR-0012. It is
    recorded in **ADR-0018**, which supersedes ADR-0012; that ADR also adds
    sub-cell overlays for the HP bar and path line.
- **Palette:** `palette.ron` now holds D's values, plus `<terrain>_bg` and
  `path` names. `black` stays `#000000`, as the console background (a test
  pins it). `hp_low` was nudged away from `enemy` so snapshot colour names stay
  unambiguous. The sampler and sample-box snapshots changed accordingly.
- **Downstream tickets updated:**
  - 0401: unit labels, HP bar and the overlay layer;
  - 0402: cursor brackets, no panel portrait;
  - 0403: path line and arrowhead;
  - 0404: full-body art moved to 0413;
  - 0703: pixel-grid portrait format, exact mirroring;
  - 0704: layout for 32×16-cell portraits;
  - 0706: shading guidance.
- **Follow-up tickets:** 0806 colour themes (C/D/E/G, values recorded in the
  design doc), 0413 combat scene with full-body art (Nick said "probably"; a
  decision ticket), 1002 custom 16×16 class icons vs initials (post–Chapter 1).
- **For Nick:** the portrait face in the screenshots is only a style sample.
  Real characters get drawn and iterated one by one in 0706. There is no
  portrait in the battle side panel; you dropped it.
