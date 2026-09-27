---
id: "0402"
title: Virtual cursor, camera follow, hover info panel, unit cycling
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: medium
status: done
blocked_by: ["0401"]
nick_input: sign-off
completed: 2026-09-27
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

- [x] Holding `Right` moves the cursor with the repeat timing from the keymap.
- [x] Cursor never leaves the map; camera scrolls at the 3-tile margin.
- [x] Panel shows correct terrain and unit data.
- [x] Harness tests below pass; snapshots committed.
- [x] Quick Battle is on the title menu of the Pages build and absent from release builds (step 7).

## Tests required

- Harness: `"Right Right Right"` → cursor x+3; `hold("Right", 1.0)` → expected position from repeat maths; clamp at edges; `s`/`a` (Next/PrevUnit) cycling order and wrap; camera origin after long moves.
- Snapshot: hover over a unit on forest; hover over empty plain.

## Completion notes

- **Cursor** (`ui::screens::battle::cursor`): `Cursor { pos, blink_t }`, one
  tile per arrow press, held keys repeat with the keymap timing (170 ms,
  then 55 ms), clamped to the map. `[` `]` in the cells either side of the
  tile, `cursor` colour, brightness `0.5 + 0.5·(1 + cos)/2` over a 1 s
  period (never off). The camera follows with the 3-tile margin.
- **Side panel** (`ui::screens::battle::panel`): terrain name, `DEF +n  AVO
  +n`, `Heals n% HP` only when non-zero; then for a unit: name (faction
  colour), `Class  Lv n`, `HP cur/max` with a 10-cell bar (`█` filled in
  `hp_high/mid/low`, same thresholds as the map bar via the shared
  `units::hp_fill`, `░` empty), faction name. No portrait.
- **Next/Prev unit** (`s`/`a`, `l`/`;`): the acting side's units that
  haven't acted, in reading order `(y, x)`, starting from the cursor's
  tile, wrapping.
- **Help bar** by context: ready unit `f select · e info · s next unit · d
  back`; other unit `arrows move · e info · s next unit · d back`; empty
  tile `arrows move · s next unit · d back`. `d` still leaves the battle
  until the map menu (0405).
- **Start:** cursor on the first player lord (else first player unit, else
  map centre); camera centred on it.
- **Step 7:** new `debug-tools` cargo feature, `cargo xtask web --release
  --debug-tools`, used only by `pages.yml` (ADR-0023). `release.yml` and
  CI's wasm check are unchanged, so shipped builds have no Quick Battle.
  The Pages link can only be checked after merge (Pages deploys from
  `main`); the feature build was checked locally
  (`cargo build -p trpg-app --target wasm32-unknown-unknown --release
  --features debug-tools`).

*Claude's starting rules* (UI details the design docs don't settle; say if
any feel wrong):

- Moving the cursor restarts its pulse at full brightness, so it's easy to
  follow while moving.
- Next/Prev unit starts from wherever the cursor is (so from an empty tile,
  Next goes to the next ready unit below/right of it), not from a remembered
  "current unit".
- The panel's HP bar is 10 cells of `█`/`░`; the faction is shown as a word
  (`Player`, `Enemy`, `Ally`, `Neutral`) under the HP.


