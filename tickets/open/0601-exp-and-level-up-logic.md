---
id: "0601"
title: EXP gain and level-up rules
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0005", "0305", "0306"]
nick_input: answer-first
completed:
---

# 0601 — EXP and level-up rules

## Context

Implements `docs/design/progression.md` (0005) exactly: EXP formula, growth
rolls (with the safety net if chosen), caps and level cap. Integrated into
battle command application so every EXP change is an `Event`.

## Nick input

**Answer first:** 0005.

## Scope

**In:** `core::progression` (pure functions), `ExpGained`/`LeveledUp` events,
integration into `BattleState::apply` for player-faction units.

**Out:** level-up UI (0602), promotion (0603).

## Implementation steps

1. `exp_for_combat(attacker_level, defender_level, killed: bool, …) -> u8` and
   `exp_for_heal(…)` exactly per design; clamp to design min/max.
2. `level_up(unit, class, rng) -> StatGains`: roll each stat per the design's
   procedure; apply caps (a capped stat can't gain); apply the safety net rule;
   return gains.
3. `grant_exp(unit, amount, rng) -> Vec<Event>`: adds EXP; for each 100
   crossed: `LeveledUp { unit, new_level, gains }`; at level cap, EXP stays at
   the design's value (FE: `--`).
4. Hook into `apply()` after combat/heal resolution, for player-side units only
   (unless design says otherwise). RNG consumption order documented.
5. Update replay tests (event logs now include EXP events).

## Acceptance criteria

- [ ] EXP formula matches a table of ≥ 6 cases derived from the design doc.
- [ ] Growth procedure matches the design; seeded tests with `ScriptedRng` prove each branch (gain, no gain, capped, safety net).
- [ ] Level cap respected.

## Tests required

- Unit: as above.
- Property: after any number of level-ups, every stat ≤ cap; per-level gain per stat ≤ design max; total level ≤ cap.
- Statistical (seeded): average gains over 10k level-ups ≈ growth rates (±2%).

## Completion notes

