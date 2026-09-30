---
id: "0711"
title: 64×64 PNG portraits drawn as pixel overlays
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0021"]
nick_input: sign-off
completed:
---

# 0711 — 64×64 PNG portraits

## Context

Nick chose bought 64×64 portraits (0021). Today portraits are 32×32 text
grids of palette keys, drawn as half-block cells: 2 pixels per cell, so 8×8
screen pixels per portrait pixel (ADR-0018, `trpg_content::portrait`,
`trpg_ui::portrait::draw_portrait`).

A 64×64 image drawn at **4×4 console pixels per image pixel** takes 256×256
px = **32×16 cells**, exactly the frame the dialogue screen (0704) already
uses. So the layout doesn't change. A cell can't hold 2×4 pixels of
different colours, so these portraits are drawn with the pixel overlays the
cursor already uses (ADR-0024: `GlyphBuffer::add_overlay`, `Overlay`,
`Layer`).

**Changed 2026-09-30:** 0021 may pick a different artist (one who sells
portraits and battle sprites). The 64×64 numbers here are CaptainSkolot's.
If the new portraits are another size, 0021 updates this ticket. Keep the
overlay drawing general enough that 0413 reuses it for battle sprites
(any PNG size, not only 64×64 portraits).

## Nick input

**Sign-off:** a screenshot of the test scene (F2 → Play test scene) with two
bought portraits, speaker and dimmed listener, next to one with the old art.

## Scope

**In:**
- Portraits as **PNG files** (64×64, RGBA), decoded at load with the `png`
  crate (MIT/Apache, already in `Cargo.lock`). Colours come from the image
  itself, not `palette.ron`.
- A sidecar per character, `assets/portraits/<id>.ron`, mapping our
  expression names to image files, e.g.
  `(character: "knight_a", expressions: { "neutral": "knight_a/neutral.png",
  "happy": "knight_a/laughing.png", … })`. The five required expressions
  must be mapped; extra expressions are allowed, as today.
- Drawing: one overlay per horizontal run of same-coloured pixels, each
  4 px tall. Transparent pixels (alpha < 128) draw nothing. `dim` lerps each
  colour toward the frame's background (as ADR-0018 says), and `mirror`
  flips each row.
- Keep the text `.portrait` format working for the placeholders until 0706
  replaces them, or convert the two placeholders to PNG and remove the text
  format. Choose one and record it in the ADR.
- `cargo xtask portrait-import <zip-or-dir> <character-id>`: copies the
  chosen pack's PNGs into place and writes a sidecar stub to fill in.
- Validation in the loader (all errors at once, with file names): size must
  be 64×64; missing required expression; unreadable PNG; sidecar
  `character` ≠ file stem.
- ADR (`write-adr`) superseding ADR-0018's portrait section: PNG portraits,
  4 px per pixel, drawn as overlays.

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
5. The importer xtask, with tests on a small fixture zip.
6. Update `assets/portraits/README.md` and the `ascii-art` skill's portrait
   notes.

## Acceptance criteria

- [ ] A 64×64 PNG portrait loads, validates and fills exactly the 32×16-cell
      area inside a dialogue frame (a test checks the overlay bounds).
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

