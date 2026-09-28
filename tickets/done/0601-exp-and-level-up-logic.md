---
id: "0601"
title: EXP gain and level-up rules
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: done
blocked_by: ["0005", "0305", "0306"]
nick_input: answer-first
completed: 2026-09-28
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
2. `growth(unit, class, stat)` = `class growth + 20 if talent` (it can exceed 100: `growth / 100` sure points plus a roll for 1 more).
3. `level_up(unit, class, min_gains_table, rng) -> StatGains`: the exact
   6-step procedure in `progression.md`. Always 7 `roll()` calls, then safety
   net picks via `roll_below(total)`. Add `roll_below(n)` to the RNG trait if
   it is missing (and to `ScriptedRng`). Capped or 0% stats never gain; gains never pass the cap.
4. `grant_exp(unit, amount, rng) -> Vec<Event>`: adds EXP; crossing 100 →
   `LeveledUp { unit, new_level, gains }` (current HP rises with HP gains); at
   the level cap (a data value; placeholder 99, `progression.md`) EXP is `--` and no EXP is gained.
5. `grant_class_points(unit, amount) -> Vec<Event>`: to the current class
   record only; class level = `1 + cp / cp_per_class_level[tier]` (a per-tier data table), capped at 10 (data);
   crossing a class level emits `ClassLeveledUp`, learns class spells for that
   class level (0309's `refresh_spells`), and at 10 emits
   `ClassMastered` + `SkillLearned` for each of the class's passives (its
   active becomes permanent; 0311's `usable_skills` reads mastery from
   `class_records`).
6. Hook into `apply()` after combat / heal / tile cast / non-combat active
   resolution, for player-faction units only: EXP and CP per the tables.
   Ally-faction units' EXP goes into the battle's `exp_pool`, which is split
   at battle end among the eligible player units (`progression.md`), with
   `ExpGained` events per unit (possibly several `LeveledUp`).
   Document the RNG consumption order.
7. Update replay tests (event logs now include EXP/CP events).

## Acceptance criteria

- [x] EXP formula matches the design doc's example table (all 8 rows).
- [x] Growth procedure matches the design; seeded tests with `ScriptedRng` prove each branch (gain, no gain, capped, 0% growth, talent +20, growth > 100 giving +1 and +2, a +2 clamped by the cap, safety net with 0 and 1 natural gains at tier 1 and tier 3, net limited by the number of eligible stats), including the worked example in `progression.md`.
- [x] Level cap respected; class level cap and mastery respected; CP go to the current class only.
- [x] Ally EXP pool: split evenly with the remainder dropped; capped, dead and undeployed units excluded (tests).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: as above.
- Property: after any number of level-ups, no stat gains past its class cap (stats already above the cap after a reclass never grow); each stat gains at most `ceil(growth / 100)` per level; gains per level ≥ `min(min_gains[tier], eligible stats)`; total level ≤ cap; class level ≤ 10.
- Statistical (seeded): with a `min_gains` of 0, average gains over 10k level-ups ≈ growth rates (±2%, including one growth above 100%); with the real table, each stat's average is ≥ its growth rate − 2%.

## Completion notes

- New `core::progression`: the EXP formulas, `growth`, `level_up` (the exact
  6-step procedure: always 7 `roll()`s, then a `roll_below` per net pick),
  `apply_gains`, `grant_exp` and `grant_class_points`, with the events
  `ExpGained`, `LeveledUp`, `ClassPointsGained`, `ClassLeveledUp`,
  `ClassMastered`, `SkillLearned` and `SpellLearned`.
- `RandomSource::roll_below(n)` added (unbiased rejection sampling in
  `SimRng`; a separate scripted list in `ScriptedRng`).
- `Unit` gained `talent: Option<StatKind>` (from the character data; generic
  units have none).
- `BattleState::apply` awards EXP and CP to player units after every combat
  (a Line Pierce strike is its own combat), heal, tile cast and non-combat
  active. Ally units' EXP goes into `exp_pool`, shared on a win just before
  `BattleEnded`. RNG order is documented in the `battle` module docs.
- The replay test now has levels on (HP/Res growths), so EXP, level ups and
  class levels are part of what must replay and survive a save.

**Deviations:** `exp_for_combat` returns `u32` (like `Unit::exp`), not `u8`.
Level ups and class level ups only *add* spells instead of calling
`refresh_spells` (which would also drop spells given by chapter data);
`refresh_spells` stays for promotion/reclass (0603). `level_up` takes the
minimum gains as a number; a tier missing from a tier table uses the highest
tier listed (content validation already requires every tier in use).

**Claude's starting rules** (the design docs were silent; Nick may veto):
- A defender that **can't counter** (out of range or unarmed) took no part in
  the combat and gets no EXP or CP (reading "attacked or countered"
  literally).
- A unit that **falls** in a combat gets nothing from it.
- Only the actions in the EXP table give EXP: **items, shops, chests,
  seizing and waiting give none**.
- **Sanctuary** (an active that heals) gives the active-skill award (20),
  not the heal-spell award (24). A **Shove** that kills by collision gives
  only the active-skill award, not a kill award.
- An award that reaches the **level cap** is cut to what reaching it takes;
  the rest is lost and EXP shows `--`.

No follow-up tickets.
