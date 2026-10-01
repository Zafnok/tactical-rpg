---
id: "0511"
title: "Boss and green AI: weigh debuffs, stances, moves after attacking and drain"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0503", "0804"]
nick_input: none
completed:
---

# 0511 — Boss AI weighs effects the forecast doesn't show

## Context

Ticket 0503 lets bosses and combat green units choose Combat Arts and
actives. It scores each one from the attack's forecast (plus Line Pierce's
strike), minus a penalty for the durability it costs
(`crates/core/src/ai.rs`, module docs → *Arts and actives*). An effect the
forecast's numbers don't show scores nothing, so with the penalty the AI
never picks an art or active **for** it:

- a **debuff** on the target (Pinning Shot's Mov −3, Pressure Point's
  Spd −3; `ArtEffect::on_first_hit`),
- a **stance** on later turns (Sidestep's avoid +20; `ArtEffect::stance`, an
  active's `Stance` rider). Its bonus in the attack's own combat is already
  in the forecast,
- a **move after the attack** (Vault, Swoop; `ActiveEffect::Strike::post_move`),
- **drain** (`ActiveEffect::Strike::drain`).

So a boss archer never uses Pinning Shot, and a gauntlet boss uses Sidestep
only when its avoid already pays off in the combat itself. Whether that
makes bosses feel flat is judged in the Chapter 1 playtest (0804); do this
ticket only if Nick's playtest notes ask for it (hence `blocked_by` 0804).

Design: `docs/design/combat-arts.md` (the arts and what they do; *Enemies:
bosses only; combat green units too*).

## Nick input

None. The numbers are *tunable* AI weights (`assets/data/ai.ron`).

## Scope

**In:**
- A score for each of the four effects above, added to the technique's
  score in `Planner::best_attack` (`crates/core/src/ai.rs`), with new
  weights in `AiWeights` / `assets/data/ai.ron`.

**Out (do not do):**
- Ordinary enemies or non-combat green units using arts or actives (never).
- Changing how the plain attack is scored, or the weights 0501 and 0503 set.
- Non-combat actives (0503's rule for a unit holding its tile stays).
- UI.

## Implementation steps

1. `BattleState::attack_forecast` (`crates/core/src/battle.rs`) returns the
   forecast and the pierce. Extend what it returns (a small `pub(crate)`
   struct) with what the AI needs from the validated `AttackStep`: the
   art's `on_first_hit` debuff and `stance`, the active's `stance`,
   `post_move` and `drain`.
2. In `core::ai`, add to a technique's score, each with its own weight in
   `AiWeights` (all *tunable*):
   - **debuff:** `debuff × amount × P(at least one of the attacker's strikes
     hits)`, only if the target survives (use the `Odds` walk; add the
     chance to `Odds`);
   - **stance:** `stance × (Def + Res + avoid / 10 of its bonuses)`, only if
     the unit's tile after the attack is in the danger zone
     (`Planner::danger`) and the unit doesn't already have the effect;
   - **move after:** `post_move` if the attacking tile is in the danger zone
     (it can step away);
   - **drain:** `damage × min(E[HP dealt] / 2, HP the attacker is missing)`.
3. Document the rules in the module docs and the weights in `ai.ron`; update
   the `ai.ron` loader tests in `crates/content/src/ai.rs`.

## Acceptance criteria

- [ ] A boss archer in range of a unit it can't kill uses Pinning Shot when the debuff weight makes it worth its durability, and a plain shot when the weight is 0 (tests).
- [ ] A boss that will be attacked next turn prefers a stance art over a plain attack of the same numbers; one out of every hostile unit's reach doesn't (tests).
- [ ] With every new weight at 0 the AI chooses exactly as after 0503 (test).
- [ ] Same state → same command (determinism test); every emitted command is valid (0501's property test still passes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above, as ASCII scenes in `crates/core/src/ai/tests.rs`.
- Property: 0501's random-state AI test keeps passing with the new weights.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
