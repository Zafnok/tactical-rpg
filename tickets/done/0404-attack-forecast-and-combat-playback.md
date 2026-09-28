---
id: "0404"
title: Attack targeting, combat forecast panel, combat playback
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: done
blocked_by: ["0403", "0304", "0306"]
nick_input: sign-off
completed: 2026-09-27
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

**Out:** full-body art of the two combatants on the combat screen (Nick
wants it, 0011; designed in 0413); EXP/level up display (0602), dialogue death quotes (0705), sound.

## Implementation steps

1. Action menu `Attack` enabled when any hostile is in range from `dest` with
   any usable weapon. If several weapons can reach, first show a weapon list
   (Menu) with each weapon's stats.
2. **Targeting:** cursor snaps between valid targets (cursor keys and `NextUnit`/`PrevUnit` both
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

- [x] Forecast numbers equal `core::combat::forecast` output (test compares panel text to a forecast computed in the test).
- [x] Playback shows every strike in order; final HPs match state.
- [x] Skipping and fast-forward work; no input is lost afterwards.
- [x] Snapshots + Harness tests below.

## Tests required

- Harness: select → move → Attack → target → confirm → wait until playback ends → defender HP matches `BattleState`; kill case shows fall and removes unit.
- Snapshot: forecast panel (with doubling and with no counter); playback mid-strike (use `wait()` to reach a deterministic frame).
- Unit: target ordering, playback timeline positions for given `dt`s.

## Completion notes

**Forecast layout: Nick chose a different one** (three rounds of renders,
recorded in `docs/design/look-and-feel.md` → *Attack forecast*, screenshot
`docs/screenshots/0404-forecast.png`). It replaces step 3's panel:

- Mixed-case labels (`Hit`, `Crit`), an HP bar per side shading what each
  would lose if every strike hit.
- Strikes listed **in the order they happen**, numbered down the middle: the
  attacker's on the left (`8 dmg →`), counters on the right (`← 9 dmg`), then
  a total per side (`17 ×2`). This replaces `7+8 ×2`.
- A pixel-drawn red **skull** on the strike that kills if every strike before
  it hit. Strikes after it are dimmed.
- `no counter` in the target's column, `--` for its Hit and Crit. `!` after
  `dmg` when effective, and `(broken)` under a broken weapon's name.
- 0414's step 2 now points to this layout. The Combat Arts mockup is too
  wide to sit beside it in the side panel, so 0414 lays the two out together.

**What was built:**

- **Core:** `combat::strike_order` (now shared by `resolve`) and
  `combat::if_all_hit`, which gives a `StrikePlan`: every strike as if it hit
  without a crit, `kills()`, and strikes after a fall. `AttackPreview` gained
  `plan`, computed from both units' real HP. The UI computes nothing itself.
  Tests: unit tests and a property test that the plan equals `resolve` with
  sure hits, and that the order equals `resolve`'s when nobody falls.
- **Attack flow** (`ui/src/screens/battle/attack.rs`, `mode.rs`): `Attack` is
  enabled when a weapon can attack someone from the destination. Every check
  is `BattleState::preview_attack`. Several weapons open a weapon list
  (`Iron Sword  Mt  5  Hit  90  Crit  0  Rng 1`, focused on the equipped
  weapon); one weapon goes straight to targeting. In targeting, the cursor
  jumps between targets in `(y, x)` order: right, down and next unit go
  forward; left, up and previous unit go back. Target tiles are tinted red.
  Confirm attacks. Cancel goes back to the weapon list (or the action menu,
  on `Attack`) with the cursor back on the unit.
- **Forecast panel** (`forecast.rs`) replaces the side panel while
  targeting.
- **Combat playback** (`playback.rs`): a box at the top of the map with both
  names, HP numbers and bars. Per strike, the striker's name flashes, then
  `HIT -7` / `MISS` / `CRITICAL! -21` shows under the target while its HP
  drains at 30 HP/s. The map, the side panel and the box all show the played
  HP. A unit that falls fades into its tile over 0.5 s with
  `<Name> has fallen.` on the message line. Hold Confirm = ×4; a tap (a
  press released within 0.2 s) skips to the end. Then browsing again, or
  the move after an attack (Vault). Timings are one const struct
  (`TIMINGS`). Death-quote hook for 0705: the fallen units are
  `Playback::falls()`, and a quote beat goes before each `Beat::Fall`.
- `Menu::focused` (menu widget).

**Deviations:**

- **Playback is a mode of the battle screen (`Mode::Combat`), not a pushed
  Screen.** A pushed screen can't make the map below it show the played HP
  or a fading unit, and its `Pop` would drop that frame's keys. See
  ADR-0025 (new). 0502 will play the AI's attacks the same way.
- Cancel in targeting goes back to the weapon list when there was one
  (the ticket said the action menu).
- Absorb strikes (0410) show as `+9 hp` in the forecast and `HEAL +9` in the
  playback, as placeholders until 0410 does affinity display.
- Only weapons attack here. Attack spells come with 0410, arts and combat
  actives with 0412/0414 (`art: None, active: None`).
- **Quick Battle: the second brigand starts at (7, 4) instead of (12, 3).**
  Before, no player unit could reach an enemy on turn 1, and there is no
  End Turn yet (0405), so the sign-off fight couldn't happen. Now the lord,
  the knight and the archer can each reach it. This changed five snapshots
  (only that brigand moved).

**Tests:** an end-to-end fight through the real Quick Battle
(`tests/battle.rs`); harness tests in `screens/battle/attack_tests.rs` (weapon list
and target order; forecast text against `combat::forecast` computed in the
test; after the playback, the defender's HP is the battle's; a kill fades
with its message and removes the unit; a tap skips and the next keys work;
hold is ×4 and leaves the same battle). Snapshots: forecast with a double
and a counter, forecast with no counter and a kill, playback mid-strike.
Unit tests for the target order, the timeline positions for given `dt`s,
drain, fade, messages, tap and hold, the box, the skull, the fade drawing
and the new mode steps.

**For Nick when playing:** in Quick Battle the brigand in the middle of
the map is in reach. Try the lord (two swords, so it asks which; it strikes
twice and the brigand hits back) and the archer from two tiles away (no
counter). Tap `f` during a fight to skip it, hold `f` to speed it up. There
is no End Turn yet (0405), so for another try, go back to the title and
start again. Pacing starting values (all tunable, in `TIMINGS`): 0.3 s
intro, 0.3 s name flash, result shown at least 0.5 s, 0.25 s between
strikes, 0.5 s fade, 0.4 s at the end.

**Rules decided where the design was silent** (all presentation, no
gameplay rule):
- *Claude's starting rule:* the kill mark assumes every strike hits **and no
  crits** (Nick said "assuming all hit").
- *Claude's starting rule:* the totals count every strike in the forecast,
  including those after a kill (dimmed).
- *Claude's starting rule:* the playback timings above.

Follow-up tickets: 0417 (acted units keep their uppercase label, dimmed
only; Nick's feedback after playing this build).
