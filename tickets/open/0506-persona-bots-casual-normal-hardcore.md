---
id: "0506"
title: "Persona bots: Casual, Normal and Hardcore players, checked against 0033's targets"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0033", "0505", "0803"]
nick_input: sign-off
completed:
---

# 0506 — Persona bots: Casual, Normal, Hardcore

## Context

Third step of the automated playtesting bots. Nick's player types and their
targets are in `docs/design/playtest-bots.md` (0033). The runner, measures
and `PlayerBot` trait exist (0505); legal moves and luck reseeding are in
`core` (0504).

**Approach: search, not training.** Each persona is the same search bot with
different settings, not a trained network (training is 0507). This follows
two lines of work on AI playtesting:
- **Skill = thinking budget + mistakes.** A bot that considers fewer
  options, looks less far ahead and picks with some randomness plays like a
  weaker player. King used search-based bots like this to predict Candy
  Crush level difficulty.
- **Player type = what the bot cares about** ("procedural personas",
  Holmgård, Liapis, Togelius and Yannakakis): the same search with a
  different scoring of outcomes. Nick's types differ mostly here (Casual
  only cares about winning; Normal protects everyone and spends items; Hardcore
  wants few turns and few items).

**Don't tune the bots to pass.** If each bot's settings are adjusted until
Chapter 1 meets its targets, the check proves nothing. Settings are chosen
from the persona descriptions here, fixed, and recorded; only 0508
(comparison with Nick's real play) may change them, and never per battle.

## Nick input

**Sign-off:** Nick reads the Chapter 1 report (one table per bot, target met
or missed) and says whether it matches how Chapter 1 felt when he played it.
Sign-off never blocks the PR.

## Scope

**In:**
- `SearchBot` in `trpg-bots` with a `Persona` settings struct; three presets
  from `docs/design/playtest-bots.md`.
- Per-persona scoring of outcomes.
- Target checks in the 0505 report; exit code per 0033's Q9 answer.
- If 0033 chose to block on misses (Q9 B or C), a GitHub workflow that runs
  it (see step 6).

**Out (do not do):**
- Neural networks or training (0507).
- Rewind (never, Nick) and restarts unless 0033's Q3 chose restarts.
- Changing Chapter 1's balance. Misses are reported to Nick; any retuning is
  its own ticket.
- Tuning persona settings to make Chapter 1 pass (see Context).

## Implementation steps

1. Read `docs/design/playtest-bots.md`. Every rule there (modes, what
   "clear" means, targets, reinforcement knowledge, what a miss does) is
   followed as written; if something needed is missing, stop and say so
   (CLAUDE.md rule 3).
2. **`SearchBot`** (`crates/bots/src/search.rs`), one player-phase command
   at a time:
   - Candidates: `legal_commands(state)` for the Player phase, minus
     `Command::Equip` (attacks already equip their slot). `Talk` is free
     (doesn't end the action), so the bot issues each available talk at
     most once per battle, before its other choices. Always include
     `EndPhase`.
   - For each candidate (capped at `persona.max_candidates`, after a cheap
     ordering: attacks by the forecast's expected damage, then the rest in
     `legal_commands` order), run `persona.samples` rollouts on copies of the
     state, each copy reseeded with a seed from the bot's own RNG
     (`reseed_luck`, 0504): apply the candidate, finish the player phase with
     the baseline bot, then play the enemy phase with the enemy AI, up to
     `persona.lookahead_phases` phases.
   - Score each rollout end state with the persona's scoring (step 3);
     average per candidate. Pick by softmax with `persona.temperature`
     (0 = always the best), drawing from the bot's own RNG.
   - **Reinforcements** per 0033 Q8: when a persona must not know them,
     rollouts are run on a copy whose not-yet-arrived reinforcements are
     removed. Add a `core` method for this only if none exists; keep it
     documented as bot-only like `reseed_luck`.
3. **Scoring** (`crates/bots/src/persona.rs`): a weighted sum over the
   rollout end state and the measures so far: battle won/lost, lord alive,
   player units fallen, player HP lost, enemies defeated, objective progress
   (distance of the nearest eligible unit to a seize tile; boss HP), turns
   used, items used. Weights per persona, derived from the persona text:
   - Casual: winning and the lord's safety dominate; fallen units cost
     little (they retreat in Casual mode); items free.
   - Normal: any fallen unit costs heavily; HP lost costs; items and turns
     cost nothing.
   - Hardcore: fallen units cost heavily; turns and items cost.
   Search settings per persona (starting values, fixed after this ticket):
   Casual small budget, 1 phase ahead, some randomness; Normal medium
   budget, 2 phases, little randomness; Hardcore large budget, 2–3 phases, no
   randomness. They are dev tooling, not game content: put them in
   `crates/bots/personas.ron` (not `assets/`), loaded by the xtask.
4. **Runner**: `cargo xtask playtest <battle> --bot casual|normal|hardcore|all`
   uses each persona's game mode from 0033 Q2. The report adds, per target,
   the value, the target, and ✓ / ✗. `all` prints the three tables, then one
   summary line per persona.
5. **Speed**: report the wall time per try. If 100 tries of all three bots
   on Chapter 1 take more than 15 minutes on one machine, lower the default
   `--runs` for `all` and say so in the report header; don't cut the search
   quality.
6. **Blocking** per 0033 Q9. If misses block anything, add
   `.github/workflows/playtest.yml` running `cargo xtask playtest <each
   battle in assets/battles> --bot all` on pushes to `main` and on PRs that
   touch `assets/` or `crates/core/`, failing per the rule. Follow ADR-0008's
   workflow conventions and ADR-0014's docs-only skip.
7. **Docs**: extend `docs/playtesting.md` (0505) with the personas, their
   scoring and settings, and how to read ✓ / ✗.

## Acceptance criteria

- [ ] `cargo xtask playtest ch01 --bot all` prints three tables with ✓ / ✗
      per target from `docs/design/playtest-bots.md`, and exits per 0033 Q9.
- [ ] Same seed → identical report (except speed lines), for each persona.
- [ ] Unit test `hardcore_never_attacks_into_a_forecast_it_scores_worse`:
      given a scripted state where one attack is clearly better (kills with
      100% hit) and another is clearly worse, Hardcore with temperature 0
      picks the better one.
- [ ] Unit test `planning_never_peeks_at_real_luck`: on a state whose real
      next rolls make a 50% attack miss, the bot's choice is the same as on
      a copy of that state whose real rolls make it hit (the bot only sees
      reseeded copies).
- [ ] Unit test `casual_accepts_retreats_normal_does_not`: on a state where
      the only winning line loses a non-lord unit, Casual takes it and Normal
      prefers a slower line that keeps everyone.
- [ ] Persona settings are recorded in `crates/bots/personas.ron` and
      `docs/playtesting.md`; the PR description states they weren't tuned to
      Chapter 1's results.
- [ ] The Chapter 1 report is pasted into Completion notes for Nick.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the three tests above plus scoring per persona.
- Property: for random battles (`arb_setup`-style from 0504's tests), every
  persona's commands are accepted (no `CommandError`) to the end or turn cap.
- Snapshot / integration: the Chapter 1 report's determinism.

## Completion notes

