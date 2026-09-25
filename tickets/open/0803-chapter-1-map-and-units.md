---
id: "0803"
title: Chapter 1 content — map, roster, enemies, triggers, winning-replay test
type: content
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: todo
blocked_by: ["0009", "0410", "0411", "0501", "0706", "0707", "0801"]
nick_input: sign-off
completed:
---

# 0803 — Chapter 1 content

## Context

Builds the actual first chapter from `docs/design/chapter-1.md` (0009), the
cast in `docs/story/characters/` (0701), the script (0707) and portraits (0706).
Proves the chapter is winnable with an automated replay.

## Nick input

**Sign-off** happens in the playtest ticket 0804.

## Scope

**In:** `assets/maps/ch01.map`, `assets/chapters/ch01.ron`, real characters
in `characters.ron` (with class/level/stats/loadouts), enemy templates, trigger
wiring, New Game starting at ch01, a winning-replay test, a difficulty sim.

**Out:** Chapter 2; balance changes to core formulas (file tickets instead).

## Implementation steps

1. **Map:** design `ch01.map` at the size in `chapter-1.md`, following FE map
   principles: a clear start area, 2–3 lanes/chokepoints, terrain that matters
   (forests for defence, a river with a bridge, a fort), the boss on a
   defensible tile (throne/fort/gate). Sketch it in the PR description.
2. **Roster:** real Chapter 1 characters from the story (replace
   `// PLACEHOLDER` entries; keep test fixtures in `tests/fixtures/` instead of
   shipped data). Stats per `progression.md` class bases + personal modifiers.
   Loadouts, starting weapon ranks, the pack cap, default pack and gold per
   `chapter-1.md` and `weapons-and-items.md`.
3. **Enemies:** count/mix per `chapter-1.md`; AI mix (mostly `Aggressive`, a
   `Guard` group near the boss, boss `Stationary`), sensible loadouts (mix
   weapon kinds so Nick's per-type traits show: a mounted enemy for spears,
   a flyer for bows if the story allows), one boss with
   a name and portrait.
4. **Triggers:** wire `ch01_*` scene ids from 0707 (intro, prebattle, boss
   engage, talk-recruit if any, death quotes, victory, tbc).
5. New Game → `ch01`.
6. **Winning replay test** `crates/ui/tests/ch01_winnable.rs` (or core): a
   hand-authored command list from the chapter seed that wins the map. Keep it
   in a readable `.ron`/text fixture. It must keep passing — if a later balance
   change breaks it, that change must update the replay consciously.
7. **Difficulty sim** (test marked `#[ignore]`, run manually):
   player side controlled by the same AI (Aggressive) over 200 seeds → print
   win rate and average player losses. Record numbers in Completion notes. Target:
   AI-player wins 30–70% (a thinking human should win comfortably; a mindless one
   shouldn't always win). Adjust enemy levels/placement to land in range.

## Acceptance criteria

- [ ] Chapter 1 playable from New Game to "To be continued".
- [ ] Winning replay test passes.
- [ ] Sim numbers recorded and within target (or justified).
- [ ] Map sketch + screenshot in the PR.

## Tests required

- Content validation (all-assets), winning replay, sim (ignored).

## Completion notes

