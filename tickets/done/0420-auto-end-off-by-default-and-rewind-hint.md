---
id: "0420"
title: Auto-end off by default; show the Rewind key in the help bar
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: ["0307"]
nick_input: none
completed: 2026-09-28
---

# 0420 — Auto-end off by default; show the Rewind key in the help bar

## Context

Nick's feedback (2026-09-28): "turn auto-end off by default, make the rewind
keybind visible in the lower part of the screen (like the other tooltips)".

- Auto-end (`docs/design/turn-structure.md`) is not implemented yet; it lands
  with 0405 (map menu / end turn) and is saved by 0805 (options). Only the
  decided default changes here.
- Turn rewind (0307) is opened with the `Rewind` key (`r` right-handed, `u`
  left-handed, `docs/design/controls.md`) but the battle help bar never named
  it.

## Nick input

None. (The request is Nick's design call.)

## Scope

**In:**
- `docs/design/turn-structure.md`: auto-end is OFF by default; record the change.
- Tickets 0405 and 0805: update the default they implement.
- `BattleScreen::help`: while browsing in the player phase (whenever Rewind
  would open the rewind screen), add `<Rewind key> rewind` before the back hint.

**Out (do not do):**
- Implementing auto-end or its toggle (0405).
- Showing charges left in the help bar.

## Implementation steps

1. Edit the design doc and the two tickets as above.
2. In `crates/ui/src/screens/battle/mod.rs` `help()`, `Mode::Idle` arm: a
   `rewind` hint whose key is `key_name(km, Action::Rewind)` filtered by
   `can_open_rewind()`, placed before `back`.
3. Update help-string tests and snapshots.

## Acceptance criteria

- [x] `turn-structure.md` says auto-end is OFF by default; 0405/0805 match.
- [x] Browsing a player-phase battle, the help bar reads e.g.
      `f select · e info · s next unit · r rewind · d back` (`u rewind` left-handed).
- [x] With a unit selected (Rewind does nothing) the hint is not shown
      (`rewind_opens_only_while_browsing`).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit/harness: help strings in `battle/mod.rs`, `attack_tests.rs`,
  `tests/battle.rs`, `rewind_tests.rs`.
- Snapshot: battle screen snapshots (help row only changes).

## Completion notes

Done as planned. The rewind hint follows the same rule as the key itself
(`can_open_rewind`: browsing, player phase, battle not over), so it never
advertises a key that does nothing. Snapshot diffs touch only the help row.
No gameplay rules decided.
