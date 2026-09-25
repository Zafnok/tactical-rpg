---
id: "0402"
title: Virtual cursor, camera follow, hover info panel, unit cycling
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0401"]
nick_input: sign-off
completed:
---

# 0402 — Virtual cursor and hover info

## Context

The core of Nick's request: a keyboard-driven virtual cursor with vim-style
movement ([ADR-0015](../../docs/adr/0015-input-actions-and-keymap-layouts.md), keys per `docs/design/controls.md`).
Must feel snappy.

## Nick input

**Sign-off:** after merge, Nick tries the Quick Battle (debug build or Pages
link) and says whether cursor speed and key layout feel right. Feedback becomes
`tuning` tickets.

## Scope

**In:** cursor state and drawing, movement/clamp, camera follow, cursor
blink, side panel hover info (terrain + unit summary), `NextUnit`/`PrevUnit`,
context help bar.

**Out:** selecting units (0403), full info screen (0405).

## Implementation steps

1. `Cursor { pos: Pos, blink_t: f32 }` in `BattleScreen`. `CursorLeft/…` move 1
   tile, clamped to map bounds (no jump actions: `docs/design/controls.md`). Camera follows (0401).
2. Draw per `look-and-feel.md` if present; default: tile background replaced by
   `cursor` palette colour, pulsing brightness (sine of `blink_t`, period ~1 s),
   glyphs kept readable (use `text_highlight` fg if contrast too low).
3. Side panel (top → bottom):
   - **Terrain:** name, `DEF +n`, `AVO +n`, heal % if any.
   - **Unit under cursor** (if any): name, class, `Lv n`, HP `cur/max` with a
     10-cell bar coloured `hp_high/mid/low` (thresholds 60% / 30%), faction label.
4. `NextUnit`/`PrevUnit`: cycle through the acting faction's units that haven't
   acted, ordered by `(y, x)`; wraps; moves cursor + camera.
5. Help bar reflects context (e.g. over own ready unit: `f select · e info · s next unit`, key names read from the keymap).
6. On battle start, cursor starts on the first player lord (or first player unit).

## Acceptance criteria

- [ ] Holding `Right` moves the cursor with the repeat timing from the keymap.
- [ ] Cursor never leaves the map; camera scrolls at the 3-tile margin.
- [ ] Panel shows correct terrain and unit data.
- [ ] Harness tests below pass; snapshots committed.

## Tests required

- Harness: `"Right Right Right"` → cursor x+3; `hold("Right", 1.0)` → expected position from repeat maths; clamp at edges; `s`/`a` (Next/PrevUnit) cycling order and wrap; camera origin after long moves.
- Snapshot: hover over a unit on forest; hover over empty plain.

## Completion notes

