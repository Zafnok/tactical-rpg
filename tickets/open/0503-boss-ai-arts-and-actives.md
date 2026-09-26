---
id: "0503"
title: "Boss AI: choosing Combat Arts and active skills"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0501", "0311", "0312"]
nick_input: none
completed:
---

# 0503 — Boss AI uses arts and actives

## Context

Nick decided (ticket 0014, `docs/design/combat-arts.md` → *Enemies: bosses
only*) that among enemies **only bosses** use Combat Arts and active skills.
They follow the player's rules and pay with their own weapon's durability.
0501's AI never uses them; this ticket lets it do so for units with
`boss: true` in chapter data (0801). The AI stays pure `core` code issuing
ordinary `Command`s ([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

None. How dangerous bosses feel is judged in the playtest (0804).

## Scope

**In:**
- In 0501's attack scoring, for boss units, also score each usable art and
  combat active (0312/0311 `usable_arts`, usable actives) with the same
  `forecast`-based score.
- Non-combat actives for bosses (e.g. Brace): use one instead of waiting when
  the boss can't attack this turn and it is in a hostile unit's threat area.
- Deterministic tie-breaks.

**Out (do not do):**
- Ordinary enemies and Other-phase units using arts or actives (never).
- UI (0413 shows the names during playback).

## Implementation steps

1. In `core::ai`, when the acting unit is a boss, extend the candidate list
   `(dest, target, weapon)` with `(dest, target, weapon, Option<ArtOrActive>)`.
   Score with the same formula, plus a durability penalty
   `w_dur * cost` (new weight in `ai.ron`, *tunable*) so bosses don't burn
   durability on attacks that barely change the outcome.
2. Tie-break: plain attack before arts, then by art/skill id.
3. Non-combat active rule (step "In" above), as the `Stationary`/`Guard`
   behaviours' fallback.
4. `debug_assert!` that every emitted command is valid, as in 0501.

## Acceptance criteria

- [ ] A boss with Guard Break in reach of a unit it can only kill without a counter picks Guard Break (test).
- [ ] A boss never picks an art it can't pay for; an ordinary enemy never picks one (tests).
- [ ] Same state → same command (determinism test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above.
- Property: extend 0501's random-state AI test with a boss; every emitted command is valid.

## Completion notes
