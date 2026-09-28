---
id: "0423"
title: Unit info screen shows stats as plain numbers (no class caps)
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-09-28
---

# 0423 — Unit info screen shows stats as plain numbers (no class caps)

## Context

The unit info screen (0405) listed stats as `value/class cap` (`HP 19/42`,
`Str 6/22`). That layout was Claude's idea when writing 0405, not a design
decision. Nick (2026-09-28) read `HP 19/42` as "injured", and asked why a
growth ceiling is shown in battle at all: "why is this info relevant during
battle when you aren't gonna level up 50 times in one battle". Agreed: remove
it.

## Nick input

None (Nick's call above). Whether classes should cap stats at all is a
separate decision: ticket 0019.

## Scope

**In:** `crates/ui/src/screens/battle/info.rs`: stats as plain numbers; drop
the HP row from the stat list (max HP is already on the `HP 19/19` line).

**Out (do not do):** changing the cap rules in `core` (0019 decides), the
side panel.

## Implementation steps

1. `draw_middle`: list `StatKind::GROWABLE` without `Hp`, value only.
2. Update `info_shows_plain_stats_and_the_loadout` and the info snapshots.

## Acceptance criteria

- [x] The info screen shows `Str   6` etc., no `/cap`, and no HP row under
      Stats (`info_shows_plain_stats_and_the_loadout`, snapshots).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit/harness test above; info screen snapshots.

## Completion notes

Done as planned. Caps still apply to level-ups until 0019 decides otherwise.
