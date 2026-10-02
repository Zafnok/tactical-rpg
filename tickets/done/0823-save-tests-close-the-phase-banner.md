---
id: "0823"
title: "Save tests close the PLAYER PHASE banner (main's CI is red)"
type: bug
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-02
---

# 0823 — Save tests close the PLAYER PHASE banner (main's CI is red)

## Context

The `test` jobs fail on `main` since 019788f ([0802] save/load and suspend,
PR #141): 11 of the 13 tests in `crates/ui/tests/save.rs` fail, so every
open PR's CI is red.

Why: 0435 (PR #156, merged just before) made a battle started through the
game flow open on turn 1's `PLAYER PHASE` banner, after its battle notes
(0411). While a banner is up only Confirm does anything. PR #141 was written
before that and merged without running against it, so its key scripts are
one Confirm short:

- `to_battle` closes the notes and leaves the banner up. The next keys
  (`d Down Down f` in `suspend`, `r f f` for a rewind) go to the banner:
  "Suspend the battle and return to the title?" never shows
  (`save.rs:161`), the rewind isn't spent (`save.rs:172`, 3 charges, not 2).
- `win` presses Confirm once for the `VICTORY` banner; it closes the phase
  banner instead and the victory scene never starts (`save.rs:73`).
- `the_quick_battle_suspends_too` and the `Restart Battle` in
  `a_continued_battle_restarts_from_its_first_turn` start a battle the same
  way.

Repro: `cargo test -p trpg-ui --test save` on `main`.

Suspend and Continue themselves must be checked too: is it only the tests
that are out of step, or is the feature broken in play?

## Nick input

`None.`

## Scope

**In:**
- The helpers and key scripts of `crates/ui/tests/save.rs`.
- Checking, through those tests, that suspend / Continue / Restart Battle
  work in play with the banner there.

**Out (do not do):**
- Changing the banner, the notes or their order (0435, 0411).
- Changing what Continue shows: a continued battle carries on without notes
  or banner, as 0802 built it.
- Making CI re-run a PR against the latest `main` before it merges (a
  merge queue or "branch must be up to date"): ticket 0115.

## Implementation steps

1. In `crates/ui/tests/save.rs` add `banner_open(h)` (the battle's
   `BattleScreen::banner()` is some) and `close_banner(h)` (asserts the
   banner is up and names `PLAYER PHASE`, presses Confirm, asserts it is
   gone), next to `notes_open` / `close_notes`.
2. `to_battle`: `close_notes`, then `close_banner`.
3. `the_quick_battle_suspends_too`: `close_banner` after `Down f` (the
   Quick Battle has no notes).
4. `a_continued_battle_restarts_from_its_first_turn`: after
   `Restart Battle`, `close_notes` then `close_banner` before `win`.
5. Run the file; fix any other script the banner changed. If a test still
   fails once the scripts are right, suspend / Continue is really broken:
   fix it in `crates/ui/src` with that test as the failing test.

## Acceptance criteria

- [x] `cargo test -p trpg-ui --test save`: 13 passed.
- [x] No file under `crates/*/src` changes unless step 5 found a real bug (it didn't).
- [x] No snapshot changes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: the Harness tests of `crates/ui/tests/save.rs`.

## Completion notes

**Done.** Only the tests were out of step; suspend, Continue and
`Restart Battle` work in play. `crates/ui/tests/save.rs` needed a
`close_banner` helper, used by `to_battle` (after the notes), by
`the_quick_battle_suspends_too` (the Quick Battle has no notes) and after
the `Restart Battle` in `a_continued_battle_restarts_from_its_first_turn`.
Nothing under `crates/*/src` changed, and no snapshot.

**The same fix reached `main` first:** 0810 (PR #152) merged while this PR
was open and made these three changes to `save.rs` itself (it had to, to
get its own CI green). The merge took `main`'s version of the file, so this
PR ends up changing no code: it records the bug and adds ticket 0115.

What the tests now show about play: a battle started from New Game, the
Quick Battle or `Restart Battle` opens on its notes (if any), then the
`PLAYER PHASE` banner; once that is closed the map menu's `Suspend` works.
A continued battle carries on where it was left, with no notes and no
banner (as 0802 built it; `suspend_then_continue_restores_the_battle_exactly`
rewinds straight after Continue).

**No gameplay rules decided.**

**Why it reached `main`:** PR #141's CI ran before PR #156 merged, and
nothing made it run again: the ruleset on `main` requires no status checks
and no up-to-date branch. Not fixed here (see Out). **Follow-up ticket:
0115** (PRs must pass CI on the latest main before they merge).
