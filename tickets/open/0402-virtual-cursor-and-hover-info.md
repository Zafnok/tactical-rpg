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

> **Heads-up from 0401:** the title's `Quick Battle` item exists only when
> `Ctx::debug_tools` is on, which is `cfg!(debug_assertions)`. The Pages site
> is a **release** build (`cargo xtask web --release`), so right now Nick
> can't reach Quick Battle from the Pages link. Step 7 fixes this before
> asking for sign-off. Later sign-offs (0404, 0408, 0410, 0414, 0502) all rely
> on it.

## Scope

**In:** cursor state and drawing, movement/clamp, camera follow, cursor
blink, side panel hover info (terrain + unit summary), `NextUnit`/`PrevUnit`,
context help bar.

**Out:** selecting units (0403), full info screen (0405).

## Implementation steps

0. **Where things are (from 0401):** the screen is `ui::screens::battle`
   (`BattleScreen`, and `quick_battle` builds the debug scene). Submodules:
   `layout` (map viewport `x 0..70`, `y 0..30`, side panel `x 70..100`,
   help row 31), `camera` (`Camera::follow(target, w, h, margin)`,
   `Camera::centred_on`, `tile_to_cell`, the only place tiles become cells)
   and `units` (labels, HP bars). `BattleScreen::follow(pos)` already wraps
   the camera with the 3-tile margin. The help line is `BattleScreen::help`.
   The side panel is drawn as an empty box.
1. `Cursor { pos: Pos, blink_t: f32 }` in `BattleScreen`. `CursorLeft/…` move 1
   tile, clamped to map bounds (no jump actions: `docs/design/controls.md`). Camera follows (0401).
2. Draw per `docs/design/look-and-feel.md` / ADR-0018: `[` and `]` in the
   cells either side of the tile, fg `cursor`, brightness pulsing between 100%
   and ~50% (sine of `blink_t`, period ~1 s; never fully off). The tile itself
   is not recoloured.
3. Side panel (top → bottom):
   - **Terrain:** name, `DEF +n`, `AVO +n`, heal % if any.
   - **Unit under cursor** (if any): name, class, `Lv n`, HP `cur/max` with a
     10-cell bar coloured `hp_high/mid/low` (thresholds 2/3 and 1/3, same as the
     map HP bar), faction label.
   - **No portrait** in the panel (Nick, 0011): stats only, keep it uncluttered.
4. `NextUnit`/`PrevUnit`: cycle through the acting faction's units that haven't
   acted, ordered by `(y, x)`; wraps; moves cursor + camera.
5. Help bar reflects context (e.g. over own ready unit: `f select · e info · s next unit`, key names read from the keymap).
6. On battle start, cursor starts on the first player lord (or first player unit).
   The camera currently starts `centred_on` the first player unit
   (`BattleScreen::new`). Centre it on the cursor's start instead.
7. **Make Quick Battle reachable from the Pages build** (see the heads-up
   above). Pick a way that keeps it out of the shipped game, e.g. a cargo
   feature (`debug-tools`) that turns `Ctx::debug_tools` on, enabled only by
   the Pages workflow and never by `release.yml` or itch/Steam builds. If
   the choice adds a build flavour, record it in an ADR. Check the Pages link
   shows `Quick Battle` before asking Nick to sign off.

## Acceptance criteria

- [ ] Holding `Right` moves the cursor with the repeat timing from the keymap.
- [ ] Cursor never leaves the map; camera scrolls at the 3-tile margin.
- [ ] Panel shows correct terrain and unit data.
- [ ] Harness tests below pass; snapshots committed.
- [ ] Quick Battle is on the title menu of the Pages build and absent from release builds (step 7).

## Tests required

- Harness: `"Right Right Right"` → cursor x+3; `hold("Right", 1.0)` → expected position from repeat maths; clamp at edges; `s`/`a` (Next/PrevUnit) cycling order and wrap; camera origin after long moves.
- Snapshot: hover over a unit on forest; hover over empty plain.

## Completion notes

