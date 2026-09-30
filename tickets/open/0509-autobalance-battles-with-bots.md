---
id: "0509"
title: "cargo xtask autobalance: tune a battle's enemies until the persona bots land in their bands"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0506"]
nick_input: sign-off
completed:
---

# 0509 — Autobalance battles with the persona bots

## Context

Nick (0033, `docs/design/playtest-bots.md`, *Autobalancing*): "the point of
automated balancing is I don't want to manually do this stuff", and a missed
target "should never happen" because battles are autobalanced. 0506 gives
three persona bots and a report that checks each bot's success rate against
a band for the battle's tier. This ticket adds the tool that changes a
battle until all three bands are met.

What it may change depends on the battle kind (Nick, Q11):

| Kind | Knobs |
| ---- | ----- |
| Random skirmish | Everything: enemy levels, count, classes, gear, positions, reinforcements, terrain (terrain and layout knobs come with the generator, 0510) |
| Fixed skirmish, side quest, story battle | Enemy numbers only: level, count, gear, which classes appear. Map, named units (bosses, characters) and objectives stay as authored. |

**The bots are never tuned to pass** (0506 context): only the battle changes.

## Nick input

**Sign-off:** Nick plays one autobalanced battle on the Pages build and says
whether it felt about as tight as the design asks ("a bit tight" in Casual).
Sign-off never blocks the PR.

## Scope

**In:**
- `cargo xtask autobalance <battle-id> [--apply] [--runs 100] [--seed 1]`.
- The "enemy numbers" knob set (fixed skirmish / side quest / story battle).
- A battle `kind` field if 0801's battle files don't have one yet.

**Out (do not do):**
- Terrain, positions and reinforcement knobs (0510 adds them for random
  skirmishes).
- Changing persona settings or the bands (`docs/design/playtest-bots.md`).
- Running autobalance in CI: it's a dev tool Claude runs and commits the
  result of; CI only runs the 0506 check.

## Implementation steps

1. **Kind:** if the battle file format (0801) has no kind, add
   `kind: Story | FixedSkirmish | SideQuest | RandomSkirmish` (default
   `Story`), validated by the content check. Document it where the battle
   format is documented.
2. **Knobs** (`crates/bots/src/balance.rs`, pure): a `Knob` enum for the
   enemy-numbers set, applied to a loaded battle definition (not a running
   `BattleState`):
   - enemy level offset for all generic (unnamed) enemies, and per enemy;
   - remove a generic enemy / add a copy of a generic enemy on a free tile
     within 2 tiles of the original (same group and AI);
   - swap a generic enemy's class for another class already in that battle,
     or a weapon for the next tier down/up in the same weapon type.
   Named units, the map and objectives are never touched; a unit test
   enforces it.
3. **Search:** start from the authored battle; evaluate = run the three
   bots (0506) with a reduced run count (e.g. 30 tries each) and compute
   distance to the bands (0 when all three are inside). Coarse first (the
   global level offset, binary search on the Casual success rate), then
   greedy single-knob steps that reduce the distance, fewest changes
   preferred. Stop when all bands are met at the full `--runs` count, or
   after a step budget; then print why it stopped.
4. **Output:** the list of changes in plain words ("Brigand ×2 → lv 4",
   "removed Archer at 12,5") and the before/after 0506 report. `--apply`
   writes the changed battle file (keeping its formatting and comments as
   far as the RON writer allows; otherwise document the loss).
5. **Docs:** `docs/playtesting.md` gets an *Autobalancing* section.

## Acceptance criteria

- [ ] On a test battle made deliberately too hard (enemies +5 levels),
      `cargo xtask autobalance <it>` finds changes after which all three
      bots are inside their bands, and `--apply` writes them.
- [ ] Unit test `autobalance_never_touches_named_units_map_or_objectives`.
- [ ] Same seed → same changes.
- [ ] Chapter 1 (`ch01`) report before/after in Completion notes; if
      Chapter 1 already meets its bands, say so and don't change it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each knob; the named-units guard; distance to bands.
- Property: any sequence of knobs yields a battle that passes content
  validation.
- Snapshot / integration: the deliberately-too-hard battle is fixed.

## Completion notes

