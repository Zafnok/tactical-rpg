---
id: "0426"
title: Combat actives that add range offer their extra targets
type: bug
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-09-29
---

# 0426 — Combat actives that add range offer their extra targets

## Context

0412 lets the player cycle a combat active on the attack forecast, but the
target list ([`attack::targets`](../../crates/ui/src/screens/battle/attack.rs))
is built from the plain weapon attack. An active with `range: N` (Long Shot)
can hit further than that in `core`, but the UI never offers those targets, so
the extra reach can't be used from the menu.

## Nick input

`None.`

## Scope

**In:**
- When a combat active with extra range is chosen, targets it can reach
  become selectable (still validated by `BattleState::preview_attack`).
- The attack range tint shows them.

**Out (do not do):**
- Core rules.

## Implementation steps

1. Let the targeting state list targets per chosen active, keeping the
   cursor on the current target when it is still valid.
2. Test with an archer knowing Long Shot against an enemy one tile beyond
   its bow's range.

## Acceptance criteria

- [x] Harness: with Long Shot chosen, an enemy just beyond the bow's range can be targeted and attacked; without it, it can't.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Harness test as above.

## Completion notes

Target lists are now per chosen active (`targets_with`). A weapon is offered
if it reaches someone plain or with any combat active; targeting starts plain,
or, when only an active reaches anyone, on the first active that does.
Choosing an active re-lists targets (keeping the current one), and the
range tint follows since it draws the target list. Cycling skips entries the
core refuses (e.g. plain against a target only Long Shot reaches).
Test: `long_shot_offers_a_target_just_beyond_the_bows_range` and
`choosing_long_shot_adds_targets_and_dropping_it_removes_them` (unit-level on
`Targeting`, not the key-driven harness).

Gameplay rules decided: none. No follow-up tickets.
