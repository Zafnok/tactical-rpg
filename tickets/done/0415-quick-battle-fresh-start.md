---
id: "0415"
title: Start the Quick Battle fresh (full HP, nobody has acted)
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: ["0402"]
nick_input: none
completed: 2026-09-27
---

# 0415 — Start the Quick Battle fresh

## Context

The debug Quick Battle (0401, `quick_battle()` in
`crates/ui/src/screens/battle/mod.rs`) set up a mid-battle scene: the knight
and a brigand were wounded and the archer had already acted, so the wounded
and acted looks showed. Nick played it and said he'd rather start at the
**start** of a battle: every unit at full HP and none of them done for the turn.

## Nick input

None. (Nick asked for this after playing.)

## Scope

**In:**
- `quick_battle()` gives every unit full HP and applies no commands before
  returning, so it is turn 1, player phase, and every player unit is ready.
- Tests that relied on the wounded knight or the acted archer set that up
  themselves, so the wounded and acted looks stay covered.

**Out (do not do):**
- Changing the map, the units, their positions, the pack or the seed.

## Implementation steps

1. In `quick_battle()`, remove the lines that lower `units[1].hp` and
   `units[4].hp` and the `Command::Act … Wait` applied to the archer. Update
   the doc comment.
2. Fix the tests in `battle/mod.rs`: `quick_battle_state` asserts nobody is
   hurt or has acted; `knight_in_forest()` wounds the knight itself (9/20 HP);
   `help_depends_on_what_is_hovered` marks the archer acted itself;
   `next_and_prev_unit_cycle_ready_units_in_reading_order` includes the archer.
3. Update the snapshot description in `crates/ui/tests/battle.rs` and accept
   the changed snapshots.

## Acceptance criteria

- [x] In the Quick Battle every unit has full HP and every player unit is ready
      (test `quick_battle_state`).
- [x] The wounded-unit panel and the acted-unit help are still tested.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `quick_battle_state`.
- Snapshot / integration: updated Quick Battle snapshots.

## Completion notes

- `quick_battle()` no longer wounds the knight and a brigand or makes the
  archer wait: everyone starts at full HP, turn 1, player phase, all three
  player units ready.
- The wounded-unit panel test (`knight_in_forest`) now wounds the knight
  itself, and the help test makes the archer wait with a real
  `Command::Act … Wait`, so both looks stay covered.
- Unit cycling now visits the archer too (reading order: archer, lord, knight).
- No gameplay rules decided.
