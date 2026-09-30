---
id: "0510"
title: "cargo xtask skirmish: generate a balanced skirmish from a request (place, level, unit mix, chapters)"
type: feature
milestone: Post–Chapter 1
model: opus-5.5
effort: high
status: todo
blocked_by: ["0509", "1007", "1008"]
nick_input: sign-off
completed:
---

# 0510 — Generate a skirmish to order

## Context

Nick (0033, `docs/design/playtest-bots.md`, *Generating a skirmish to
order*): "I will tell you I think the map is empty and I want a skirmish on
this part of the grid and you will put one that makes sense for the lv I
think of." His examples:

- "a skirmish on grid 10,10 of open world map that's lv35 scaled
  difficulty, made up of a mix of units"
- "a skirmish on grid 13,13 lv40 scaled difficulty showing up in ch2 only
  made up wholly of mages"

**"Lv 35" means a lv 35 battle** (Nick): enemies around lv 35, and the bots
play it with a lv 35 army of the units the player would have by then.
Random skirmishes may change anything when autobalanced, terrain included
(Nick, Q11); fixed ones only enemy numbers (0509).

World map nodes and their grid come from 1007; skirmish nodes, level
markers and the random-skirmish pool from 1008; autobalancing from 0509.

## Nick input

**Sign-off:** Nick asks for one skirmish in his own words, plays it on the
Pages build, and says whether it felt like "a lv N battle".

## Scope

**In:**
- `cargo xtask skirmish new --at <x,y> --level <n> [--units mix|<class>|<weapon type>] [--chapters <list>] [--kind fixed|random] [--tier easy|normal]`.
- A **reference army** builder: the units recruited by a given chapter, at
  level N, with ordinary gear for that point.
- Map generation for skirmishes, and the random-skirmish knobs 0509 left
  out (positions, reinforcements, terrain).
- Adding the result to the game data (battle file + world map node).

**Out (do not do):**
- Generating skirmishes while the game runs: all generation is at authoring
  time. Random skirmishes in the game still pick from a pool (1008); this
  tool can fill that pool.
- Story battles and side quests (authored by hand; 0509 balances them).

## Implementation steps

1. **Reference army** (`crates/bots/src/army.rs`): from the story data, the
   player units recruited by the requested chapter (earliest if several);
   each at level N using its class's average growths, promoted where the
   class rules require (`progression.md`); gear = the best ordinary weapons
   and items the shops sell by that chapter. Document the approximation in
   `docs/playtesting.md`.
2. **Map generation**: a small map (size from `world-structure.md`'s
   skirmish sizes) using the terrain of the world map region around the
   grid point (1007), from a few layout templates with random variation.
   Deterministic from `--seed`.
3. **Enemies**: 4–6 enemies (random skirmish) or the fixed-skirmish size,
   classes from `--units`, around level N, placed by the generator.
4. **Balance**: run 0509's search with the full random-skirmish knob set
   (add position, reinforcement and terrain knobs here) until the three bots
   are inside the bands for the tier (Easy by default for random, as
   `world-structure.md`).
5. **Write**: a new battle file under `assets/battles/`, and the node at
   `<x,y>` in the world map data with its kind, level marker and chapters.
   Print the 0506 report for the result.
6. **Docs:** `docs/playtesting.md` gets a *Generating skirmishes* section with
   Nick's two example requests as commands.

## Acceptance criteria

- [ ] `cargo xtask skirmish new --at 10,10 --level 35 --units mix` writes a
      battle file and node that pass content validation, with enemies
      around lv 35 and all three bots inside their bands.
- [ ] `--units mage --chapters 2` produces only mage enemies and a node
      shown only in Chapter 2.
- [ ] Same arguments and seed → identical files.
- [ ] Unit test `reference_army_levels_and_roster`: for a chapter, the army
      holds exactly the units recruited by then, all at level N.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: reference army; map generator determinism; unit-mix filter.
- Property: generated battles always pass content validation.
- Snapshot / integration: one generated skirmish end to end.

## Completion notes

