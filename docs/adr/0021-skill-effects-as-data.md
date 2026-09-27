# ADR-0021: Skill effects are data, gathered into combat modifiers

- **Status:** Accepted
- **Date:** 2026-09-27
- **Related tickets:** 0311, 0312, 0412, 0503, 1001

## Context

Ticket 0311 gives the ~48 class skills of `progression.md` their effects.
They are placeholder content that Nick will tune after the playtest, and
more tiers (1001) and Combat Arts (0312) follow with the same kinds of
effect: bonuses to hit, crit, might, avoid or attack speed, extra strikes,
stat bonuses that last until a phase starts, a move after attacking, heals
and pushes. Balance changes must not need code changes (ADR-0005), and the
combat formulas of 0304 are covered by worked-example and property tests
that must keep passing.

## Decision

1. **Effects are a closed set of data variants** in `core::skill`
   (`PassiveEffect`, `ActiveEffect`, `Condition`, `WeaponReq`, `Area`,
   `SkillCost`), deserialised straight from `assets/data/skills.ron` by
   `trpg-content`. Adding a skill with existing kinds of effect is a data
   change; adding a new kind of effect is a new variant plus its rule and
   tests. No scripting, no per-skill code.
2. **Combat sees only numbers.** Every skill bonus that touches a combat is
   gathered by the battle into one `CombatMods` value and stat bonuses on
   each side's `CombatantInput`, before `forecast()` runs: passives whose
   conditions hold, timed effects, ally auras and the chosen combat active
   (plus its stance rider). `combat.rs` knows nothing about skills; with
   default mods its formulas give exactly the old results. So the forecast
   and the resolution always agree, and the UI shows skill effects by
   showing the forecast.
3. **Costs go through one helper pair**: `check_cost` (validation, no
   change) and `pay_cost` (pays and returns the events, with an `ItemBroke`
   to emit after the action). Combat Arts (0312) use them too.
4. **Timed effects live on the unit** (`Unit::effects`, saved with the
   battle, ADR-0020) and are keyed by their source skill: the same effect
   refreshes, never stacks. They end at the start of a given phase.
5. **Moves after an attack** are part of the attack command
   (`UnitAction::Attack::then_move`), validated with the rest of the command,
   so an `Act` stays one atomic, replayable command (0305's hook).

## Consequences

- Retuning a skill, or adding one of an existing shape, is a one-line data
  edit, validated at load (costs, ranks, families, class references).
- The combat maths stays small and testable on its own; skills are tested
  against the forecast, with the real data in `trpg-content`'s
  `tests/skills.rs`.
- A new kind of effect (e.g. Line Pierce's extra strike in 0312) needs a new
  variant and a hook in the battle; the closed set can grow, and each growth
  is reviewed by tests rather than by reading scripts.
- `UnitAction::Attack` and `UnitAction::Cast` gained fields, so every
  command literal names them (`active: None`, `then_move: None`).
- Choosing the move after an attack happens before the combat's rolls: the
  UI (0403/0412) must ask for it with the attack, not after the animation.

## Alternatives considered

- **Per-skill Rust code (a trait object or a match on the id)** — every
  retune would be a code change, and 0013's rescale would touch code.
- **A small scripting language in the data** — far more power than the
  placeholder skills need, and much harder to test and validate.
- **Skill checks inside `combat.rs`** — the combat maths would need the unit,
  the map and the phase, mixing layers and breaking its standalone tests.
- **A separate "move after" command after the attack** — the unit would sit
  in a half-acted state between two commands, which every other command,
  save and replay would have to handle.
