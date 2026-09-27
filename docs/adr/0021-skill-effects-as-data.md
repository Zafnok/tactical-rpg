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
5. **Moves after an attack are their own command**, because Nick wants the
   tile chosen after seeing the combat. An attack with a post-action move
   ends with `Event::MoveAfterOffered` instead of `UnitActed`, and the
   battle records a `PendingMove` (saved with it). The only command accepted
   then is `Command::MoveAfter { unit, to }` (`to: None` stays), which
   ends the action; every other command is refused with
   `MoveAfterPending`. The state never has a half-applied command: both
   commands are atomic and replay as they are.

## Consequences

- Retuning a skill, or adding one of an existing shape, is a one-line data
  edit, validated at load (costs, ranks, families, class references).
- The combat maths stays small and testable on its own; skills are tested
  against the forecast, with the real data in `trpg-content`'s
  `tests/skills.rs`.
- A new kind of effect (e.g. Line Pierce's extra strike in 0312) needs a new
  variant and a hook in the battle; the closed set can grow, and each growth
  is reviewed by tests rather than by reading scripts.
- `UnitAction::Attack` and `UnitAction::Cast` gained an `active` field, so
  every command literal names it (`active: None`).
- The UI (0403/0412) and the AI (0501) must answer a `MoveAfterOffered`
  with a `MoveAfter` before anything else; `BattleState::move_after_tiles`
  lists the choices.

## Alternatives considered

- **Per-skill Rust code (a trait object or a match on the id)** — every
  retune would be a code change, and 0013's rescale would touch code.
- **A small scripting language in the data** — far more power than the
  placeholder skills need, and much harder to test and validate.
- **Skill checks inside `combat.rs`** — the combat maths would need the unit,
  the map and the phase, mixing layers and breaking its standalone tests.
- **The move after as a field of the attack command** — simpler (one atomic
  command, no waiting state), and it was the first version; but the tile
  had to be chosen before the rolls, and Nick wants it chosen after the
  combat plays out.
