---
id: "0209"
title: Keep the glyph sampler's demo panels on screen as the palette grows
type: bug
milestone: M1 Engine
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0209 — Keep the glyph sampler's demo panels on screen as the palette grows

## Context

The debug glyph sampler (`crates/ui/src/debug.rs`, `glyph_sampler`) lists
every glyph, then every palette colour as swatches, then a sample sentence,
then two demo panels (`sample_panels`: "Unit" and "Ranges") from the row
after the sentence down to the bottom of the console.

Ticket 0310 added six terrain-magic colours to `assets/data/palette.ron`
(`fire`, `fire_bg`, `ash`, `ash_bg`, `ice`, `ice_bg`). The swatch list grew
by two rows, so `sample_panels` now starts at `top == CONSOLE_H` and draws
nothing: the accepted snapshot
`crates/ui/src/snapshots/trpg_ui__debug__tests__sampler_snapshot.snap` no
longer shows the panels. Every future colour makes it worse.

## Nick input

None.

## Scope

**In:**
- Lay the sampler out so the glyph grid, every palette swatch, the sentence
  and both demo panels (at least 3 rows tall) fit in `CONSOLE_W × CONSOLE_H`
  with the current palette (51 colours).

**Out (do not do):**
- Changing the palette, the font or any game screen.

## Implementation steps

1. In `crates/ui/src/debug.rs`, make room: e.g. more swatch columns
   (`SWATCH_COLUMNS`, with a narrower `SWATCH_W`), fewer blank rows between
   the glyph grid and the palette, or panels beside the palette instead of
   below it. Pick whichever keeps the code simplest.
2. Re-check `sampler_snapshot` by reading the `.snap.new`, then accept it.

## Acceptance criteria

- [ ] The sampler snapshot shows both panels with their borders and all
      three content rows.
- [ ] A new unit test asserts the panels' top row is at most
      `CONSOLE_H - 4` for the embedded palette.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the panel-position assertion above.
- Snapshot: `sampler_snapshot` updated.

## Completion notes
