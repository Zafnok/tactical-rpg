---
id: "0426"
title: Combat actives that add range offer their extra targets
type: bug
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] Harness: with Long Shot chosen, an enemy just beyond the bow's range can be targeted and attacked; without it, it can't.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness test as above.

## Completion notes
