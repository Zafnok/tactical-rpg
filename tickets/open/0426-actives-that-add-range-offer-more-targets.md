---
id: "0426"
title: Combat actives and arts that change range offer their extra targets
type: bug
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0426 — Actives and arts that change range offer their extra targets

## Context

The target list ([`attack::targets`](../../crates/ui/src/screens/battle/attack.rs))
is built from the plain weapon attack. Two things reach targets the plain
attack can't, in `core`:

- an active with `range: N` (Long Shot, 0412) hits further than the weapon;
- the Combat Art **Close Shot** (0414, `docs/design/combat-arts.md`) lets a
  bow shoot an **adjacent** enemy (minimum range 1).

Since 0414 the art or active is picked from the arts list *after* a target
is chosen, so those targets are never offered and the extra reach can't be
used from the menu. (Close Shot is still listed at distance 2, where it only
costs durability and hit.)

## Nick input

`None.`

## Scope

**In:**
- Targets that only an art or active can reach become selectable (still
  validated by `BattleState::preview_attack`), with the attack range tint.
- On such a target, the arts list shows only the lines that reach it, and
  `Attack` is dimmed with its reason (e.g. `out of range`).

**Out (do not do):**
- Core rules.

## Implementation steps

1. Build the targets from every line of the arts list (`art_list::art_choices`
   with each art and active), not just the plain attack; keep the `(y, x)`
   order.
2. When the target changes, focus the first usable line (the plain attack
   if it reaches).
3. Tests: an archer knowing Long Shot against an enemy one tile beyond its
   bow's range; an archer with Close Shot against an adjacent enemy.

## Acceptance criteria

- [ ] Harness: with Long Shot, an enemy just beyond the bow's range can be targeted and attacked; without it, it can't.
- [ ] Harness: with Close Shot (Bow E), an adjacent enemy can be targeted and shot with Close Shot; a plain attack on it can't be chosen.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness tests as above.

## Completion notes
