---
id: "0404"
title: Attack targeting, combat forecast panel, combat playback
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0403", "0304", "0306"]
nick_input: sign-off
completed:
---

# 0404 — Attack targeting, forecast, combat playback

## Context

Makes combat visible: pick a target, read the forecast, watch the exchange.
Numbers come from `core::combat::forecast` (0304); outcomes from applied
`Event`s — the UI never computes results itself
([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

**Sign-off:** Nick watches a few fights (Quick Battle) and comments on
readability and pacing.

## Scope

**In:** `Attack` in the action menu, targeting mode, forecast panel, weapon
choice (if the unit has several usable weapons), `CombatPlayback` overlay
driven by `CombatResolved`/`UnitFell` events, fall animation.

**Out:** EXP/level up display (0602), dialogue death quotes (0705), sound.

## Implementation steps

1. Action menu `Attack` enabled when any hostile is in range from `dest` with
   any usable weapon. If several weapons can reach, first show a weapon list
   (Menu) with each weapon's stats.
2. **Targeting:** cursor snaps between valid targets (`hjkl` and `Tab` both
   cycle, ordered by `(y, x)`); `Cancel` returns to action menu.
3. **Forecast panel** (replaces side panel while targeting), both sides:
   ```
   ┌──────── FORECAST ────────┐
   │ Ana            Brigand   │
   │ Iron Sword     Iron Axe  │
   │ HP   18          HP   22 │
   │ DMG  7+8 ×2      DMG   4 │
   │ HIT  87          HIT  61 │
   │ CRIT  3          CRIT  0 │
   └──────────────────────────┘
   ```
   `×N` when the side strikes N times; for swords the follow-up damage is
   shown (`7+8` = first strike 7, each follow-up 8); an effectiveness marker
   (e.g. `!` after DMG, highlight colour) when the weapon is effective against
   the target; `(broken)` after a broken weapon's name; `--` when a side
   can't counter. There is no weapon triangle (`weapons-and-items.md`). Confirm → apply `Act { Attack }`.
4. **CombatPlayback** overlay (a Screen pushed with the events): top-centre
   box with both combatants' names, HP bars and numbers. For each `Strike`:
   attacker's name flashes, then `HIT -7` / `MISS` / `CRITICAL! -21` in
   highlight colour, HP bar drains at ~30 HP/s, pause 0.25 s between strikes.
   Hold Confirm = ×4 speed; a tap skips to the end. Timings in a const struct
   (tunable). Pops itself when done.
5. **Fall:** on `UnitFell`, the unit's tile glyphs fade (lerp to terrain over
   0.5 s) with a `<Name> has fallen.` message line; leave a hook
   (`on_unit_fell` callback/event list) for death quotes in 0705.
6. After playback, return to `Idle` (unit acted).

## Acceptance criteria

- [ ] Forecast numbers equal `core::combat::forecast` output (test compares panel text to a forecast computed in the test).
- [ ] Playback shows every strike in order; final HPs match state.
- [ ] Skipping and fast-forward work; no input is lost afterwards.
- [ ] Snapshots + Harness tests below.

## Tests required

- Harness: select → move → Attack → target → confirm → wait until playback ends → defender HP matches `BattleState`; kill case shows fall and removes unit.
- Snapshot: forecast panel (with doubling and with no counter); playback mid-strike (use `wait()` to reach a deterministic frame).
- Unit: target ordering, playback timeline positions for given `dt`s.

## Completion notes

