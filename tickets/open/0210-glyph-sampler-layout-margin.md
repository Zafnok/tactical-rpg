---
id: "0210"
title: Give the glyph sampler layout margin instead of an exact fit
type: bug
milestone: M1 Engine
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0210 — Give the glyph sampler layout margin instead of an exact fit

## Context

Ticket 0209 fixed the debug glyph sampler (`crates/ui/src/debug.rs`,
`glyph_sampler`) so its two demo panels (`sample_panels`) fit on screen for
the current 51-colour palette (`assets/data/palette.ron`). That fix removed
the blank separator rows between the glyph grid, the palette swatches and
the sample sentence, and computed the palette section's start row
(`palette_top`) from the actual glyph count instead of a fixed constant.

With the current palette, `panels_top` lands the demo panels' `top` row at
exactly `CONSOLE_H - 4` — the minimum height `sample_panels` needs. There is
no margin left: the next colour ticket 0310 or any other adds to
`assets/data/palette.ron` will push `panels_top` past `CONSOLE_H - 4` again
and reproduce the same bug (panels clipped or not drawn), the same way
ticket 0209 itself was caused by ticket 0310 adding six colours.

## Nick input

None.

## Scope

**In:**
- Change the swatch layout in `crates/ui/src/debug.rs` (`glyph_sampler`) so
  it has real margin below the demo panels for at least a few more palette
  colours (e.g. up to 60 total), not just the current 51 — for example more
  `SWATCH_COLUMNS` with a narrower per-column name field (truncating overly
  long colour names such as `panel_border_focus` if needed to make the
  columns fit), or laying `sample_panels` beside the palette swatches
  instead of below them.
- Update `sampler_snapshot` and the panel-position unit test added in ticket
  0209 (`debug::tests::demo_panels_fit_below_the_embedded_palette`) to
  reflect the new layout headroom.

**Out (do not do):**
- Changing the palette, the font or any game screen.

## Implementation steps

1. Pick a layout with real margin (see Scope) and implement it in
   `crates/ui/src/debug.rs`.
2. Re-check `sampler_snapshot` by reading the `.snap.new`, then accept it.
3. Tighten `demo_panels_fit_below_the_embedded_palette` (or add a second
   test) so it fails loudly well before the layout is actually full, e.g.
   asserting there are at least N rows of margin for the embedded palette's
   current colour count, not just that it currently fits.

## Acceptance criteria

- [ ] The sampler snapshot still shows both panels with their borders and
      all three content rows.
- [ ] A unit test asserts the layout has margin (not just that it currently
      fits) for palettes somewhat larger than today's 51 colours.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the margin assertion above.
- Snapshot: `sampler_snapshot` updated if the layout changes.

## Completion notes
