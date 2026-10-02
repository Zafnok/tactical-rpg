---
id: "0810"
title: "Show what a won battle earned: clear gold and the unused-rewind EXP bonus"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: done
blocked_by: ["0801", "0602"]
nick_input: decision
completed: 2026-10-01
---

# 0810 — Show what a won battle earned

## Context

When a battle is won, 0801's `Campaign::apply_result` gives the player two
rewards that nothing on screen shows today:

- **Clear gold**: the battle file's `clear_gold` (Chapter 1: 1000 gold,
  `docs/design/chapter-1.md`).
- **The unused-rewind EXP bonus**: every deployed player unit gets 7 % of one
  level's EXP per unused rewind charge
  (`docs/design/death-and-difficulty.md`). A unit can level up from it.

0801's flow goes straight from the `VICTORY` banner to the victory scenes, so
the player never learns about either reward. The playtest (0804, step 4) asks
Nick whether the rewind bonus felt right, which he can't judge if it's
invisible. Found while listing Chapter 1 gaps (2026-09-29).

How the rewards are shown is player-facing UI, so Nick decides it. Don't pick
a layout or wording as if it were decided.

## Nick input

**Decision**, with the `ask-nick` skill: show real rendered mockups (the
`ascii-art` skill, at the game's real size and palette D) of at least these
options, plus "describe your own". Before naming a game as an example, check
that the game really does it.

- **A. Message lines only:** after the banner, short boxes one after the
  other ("Got 1000 gold." then "Unused rewinds: +21 EXP to everyone.").
- **B. Results screen:** one screen listing gold earned, rewind charges left,
  and each unit's EXP gain as a filling bar (0602's EXP bar), with level-ups
  played from there.
- **C. EXP only on the map:** after the banner, each unit's EXP bar fills in
  turn on the map (0602), then one gold line.

Sub-questions: where it sits (before or after the victory scenes), and
whether it can be skipped with Cancel like combat playback (0418).

## Scope

**In:**
- The chosen presentation, as a screen or mode pushed by `ui::flow` after
  `BattleEnded` victory and before or after the victory scenes (Nick's call).
- Reading the rewards from `core` (`apply_result`'s outcome, or a
  `BattleRewards` value it returns): the UI never computes EXP or gold itself
  (ADR-0004).
- Level-ups caused by the bonus use 0602's level-up screen.
- Record Nick's answer in `docs/design/death-and-difficulty.md` (next to the
  rewind bonus rule) with his words.

**Out (do not do):**
- Changing the reward amounts (tuning tickets after the playtest).
- Level-up or EXP sounds (Nick's own work).
- Victory music (0809).

## Implementation steps

1. Run the decision with mockups; record it.
2. If `apply_result` doesn't return what the screen needs, make it return a
   `BattleRewards { gold: u32, rewind_charges_left: u8, exp: Vec<(UnitId, u32 /* before */, u32 /* gained */)>, level_ups: Vec<LevelUp> }`
   (names are a suggestion) and cover it with unit tests.
3. Build the chosen UI in `crates/ui/src/screens/`, pushed by `ui::flow`.
4. Harness test over the test chapter (`assets/chapters/test.ron`).

## Acceptance criteria

- [x] Nick's choice and words are recorded in `death-and-difficulty.md`.
- [x] Unit: `apply_result` reports gold and per-unit EXP matching the rules
      (0 unused charges → no EXP line; 3 unused → 21 % of a level each).
- [x] Harness: winning the test chapter shows the gold and the EXP bonus,
      and a level-up caused by the bonus shows 0602's screen.
- [x] Snapshot of the new screen or boxes.
- [ ] Nick signed off.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: rewards returned by `apply_result`.
- Snapshot / integration: Harness win flow and a snapshot of the new UI.

## Completion notes

**Nick's decision** (2026-10-01, on four rendered mockups: A message
boxes, B results screen, C EXP bars on the map, D one summary box): "1B 2A
3B". One results screen; right after `VICTORY`, before the victory scenes;
a press only skips the bars' animation, each level-up screen waits for its
own press. Recorded in `docs/design/death-and-difficulty.md` (*Results
screen*).

**What was done**

- `core`: `BattleRewards` (which `apply_result` already returned, from
  0801) now also holds `deployed`, the deployed units still in the roster
  as they were before the bonus, in slot order, and `exp_gained(unit)`
  reports what each got. The screen reads everything from it and computes
  no EXP or gold.
- `ui`: `screens/results.rs`, `ResultsScreen`: the gold, the rewinds left,
  the bonus, and a row per unit whose EXP bar fills; then the level-up
  pages of 0602 (`Progress::without_exp_bars`, since the screen shows the
  bars itself), drawn in the middle of the screen. `ui::flow` hosts it as
  `Stage::Results` between the battle and the victory scenes.
- Tests: unit tests of the rewards in `core` (0 charges: no EXP; 3: 21
  each; a unit at the level cap: none); unit tests and two snapshots of the
  screen (`results`, `results_level_up`); Harness tests over the test
  chapter (`a_won_battle_shows_its_gold_and_the_rewind_bonus` with a
  snapshot, `used_rewind_charges_shrink_the_bonus_to_nothing`, and
  `a_level_up_from_the_rewind_bonus_shows_the_level_up_screen`).

**Deviations:** the ticket's suggested `BattleRewards { exp: Vec<(UnitId,
before, gained)>, level_ups }` became `deployed` + the existing `events` +
`exp_gained`, because 0801 had already made the struct and the level-up
pages are built from those events.

**Claude's starting rules** (presentation, where the docs were silent;
Nick may veto any):

- With no rewind charge left, the screen shows the gold and `0 of 3` and
  nothing about EXP (no bonus line, no unit rows).
- A battle that gives no clear gold (the debug Quick Battle) shows no gold
  line.
- A unit at the level cap is listed, with its bar unchanged.
- The bars wait 0.5 s, then fill over 1 s; holding Confirm fills them 4
  times as fast (as on the EXP bar after a combat).
- The wording is the mockup's: `Gold for clearing the map`, `Rewinds
  unused`, `Bonus EXP for each unit`, `LEVEL UP`.
- Units get a blank row between them while they all fit (up to 7); a
  bigger army is listed without blank rows. The panel has room for 14.

**Not met:** *Nick signed off* waits for Nick to play it on Pages after the
merge.

**Follow-ups:** none. Recruits who joined and Classic deaths aren't on the
screen (the ticket is about gold and the rewind bonus only).
