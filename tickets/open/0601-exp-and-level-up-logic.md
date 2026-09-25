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

Implements `docs/design/progression.md` (0005) exactly: unit EXP (character
level, never resets), class-driven growth rolls plus the talent stat, the
per-tier safety net ("blessed N"), caps, level cap, and **class points /
class levels / mastery**. Integrated into battle command application so every
EXP and CP change is an `Event`.

## Nick input

**Answer first:** 0005 (done).

## Scope

**In:** `core::progression` (pure functions), `ExpGained` / `LeveledUp` /
`ClassPointsGained` / `ClassLeveledUp` / `ClassMastered` / `SkillLearned` /
`SpellLearned` events, integration into `BattleState::apply` for
player-faction units.

**Out:** level-up UI (0602), promotion and reclass (0603), skill *effects*
(0311; this ticket only records that a skill was learned).

## Implementation steps

1. `exp_for_combat(unit_level, target_level, outcome: {NoDamage, Damaged, Killed}, target_is_boss) -> u8`,
   `exp_for_heal()`, `exp_for_tile_cast()`, `exp_for_active_skill()` exactly
   per the table in `progression.md`.
2. `growth(unit, class, stat)` = `min(100, class growth + 20 if talent)`.
3. `level_up(unit, class, min_gains_table, rng) -> StatGains`: the exact
   6-step procedure in `progression.md`. Always 7 `roll()` calls, then safety
   net picks via `roll_below(total)`. Add `roll_below(n)` to the RNG trait if
   it is missing (and to `ScriptedRng`). Capped or 0% stats never gain.
4. `grant_exp(unit, amount, rng) -> Vec<Event>`: adds EXP; crossing 100 →
   `LeveledUp { unit, new_level, gains }` (current HP rises with HP gains); at
   the level cap (40, data) EXP is `--` and no EXP is gained.
5. `grant_class_points(unit, amount) -> Vec<Event>`: to the current class
   record only; class level = `1 + cp / (15 × tier)` capped at 10 (data);
   crossing a class level emits `ClassLeveledUp`, learns class spells for that
   class level (0309's `learn_new_spells`, if it exists), and at 10 emits
   `ClassMastered` + `SkillLearned` for the class's active.
6. Hook into `apply()` after combat / heal / tile cast / non-combat active
   resolution, for player-faction units only: EXP and CP per the tables.
   Document the RNG consumption order.
7. Update replay tests (event logs now include EXP/CP events).

## Acceptance criteria

- [ ] EXP formula matches the design doc's example table (all 8 rows).
- [ ] Growth procedure matches the design; seeded tests with `ScriptedRng` prove each branch (gain, no gain, capped, 0% growth, talent +20, safety net with 0 and 1 natural gains at tier 1 and tier 3, net limited by the number of eligible stats), including the worked example in `progression.md`.
- [ ] Level cap respected; class level cap and mastery respected; CP go to the current class only.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: as above.
- Property: after any number of level-ups, no stat gains past its class cap (stats already above the cap after a reclass never grow); each stat gains at most +1 per level; gains per level ≥ `min(min_gains[tier], eligible stats)`; total level ≤ cap; class level ≤ 10.
- Statistical (seeded): with a `min_gains` of 0, average gains over 10k level-ups ≈ growth rates (±2%); with the real table, each stat's average is ≥ its growth rate − 2%.

## Completion notes

