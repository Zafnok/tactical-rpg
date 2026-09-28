---
id: "0422"
title: Draw forts as battlements (╦╦) instead of brackets
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-09-28
---

# 0422 — Draw forts as battlements (╦╦) instead of brackets

## Context

Nick (2026-09-28): "not a fan of the fort glyph it looks too similar to a
cursor". The fort was `[]` in `fort` gold; the cursor is pale-gold corner
marks (`docs/design/look-and-feel.md`, ticket 0416), so a fort looked like a
second cursor. Shown rendered options (A `ΠΠ`, B `⌂⌂`, C `╦╦`, D `▟▙`,
E grey `ΠΠ`), Nick picked **C**: "I like C for the fort".

## Nick input

None (decided above).

## Scope

**In:** `assets/data/terrain.ron` fort glyphs → `╦╦` (colour unchanged);
`look-and-feel.md` records the choice; tests and snapshots that show forts.

**Out (do not do):** fort colour, other terrain glyphs, the glyph sampler's
`[]` cursor sample (`crates/ui/src/debug.rs`, that one *is* the cursor).

## Implementation steps

1. Change the glyphs in `terrain.ron`.
2. Record the decision in `docs/design/look-and-feel.md`.
3. Update `tile(&h, 30, 16)` in `crates/ui/tests/battle.rs`,
   `the_path_ends_in_an_arrowhead_with_no_cursor_frame_there`, and snapshots.

## Acceptance criteria

- [x] Forts draw `╦╦` in `fort` / `fort_bg` everywhere the map is drawn.
- [x] Snapshot diffs change only fort cells.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Existing snapshot and harness tests, updated.

## Completion notes

Data-only change plus test expectations. `╦` is in the Terminus atlas (box
drawing), so no font work.
