---
id: "0419"
title: Fix main's test build and rewind tests after parallel merges
type: bug
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-09-28
---

# 0419 — Fix main's test build and rewind tests after parallel merges

## Context

CI fails on `main` at c54bb70 ([0307] Turn rewind, PR #62). Four tickets were
merged within a minute of each other, each green on its own branch but not
together:

1. **0501 × 0601.** `crates/core/src/ai/tests.rs` (0501, enemy AI) builds a
   `Unit` in `fn unit(...)` by struct literal. 0601 (EXP and level-up) added
   `Unit::talent: Option<StatKind>`, so `cargo test -p trpg-core` fails to
   compile with E0063 (missing field `talent`).
2. **0307 × 0418.** `crates/ui/src/screens/battle/rewind_tests.rs` (0307, turn
   rewind) skips the lord's combat playback with a `Confirm` tap. 0418 moved
   "skip the playback" to **Cancel** (Nick's decision, `docs/design/controls.md`
   § "Fight playback: Cancel skips, hold Confirm speeds up"); a Confirm tap no
   longer skips. So after `attacked()` the screen is still in `Mode::Combat`
   (playback at t ≈ 0.15 s), Rewind can't open (`can_open_rewind` needs
   `Mode::Idle`), and these fail:
   - `attack_then_rewind_restores_exactly_the_state_before_the_attack`
     (`s.rewind().expect("open")` panics)
   - `with_no_charges_the_screen_says_so_and_cant_confirm` (same)
   - `the_list_is_newest_first_and_the_map_shows_the_highlighted_point`
     (the archer's inputs are eaten by the playback: history has 1 command)
   - `rewind_screen_snapshot` (the rewind panel never opens)

   Not caused by 0601's EXP gain or by 0501's AI: verified by printing the
   screen's mode after `attacked()`.

## Nick input

`None.` The control binding is already decided in `docs/design/controls.md`.

## Scope

**In:**
- Add the missing `talent` field to the test unit in `crates/core/src/ai/tests.rs`.
- Update the playback skip in `rewind_tests.rs` to the current control (Cancel).

**Out (do not do):**
- Don't change the playback controls, rewind rules, or any assertion.
- Don't touch 0601's EXP / level-up code.

## Implementation steps

1. In `crates/core/src/ai/tests.rs`, `fn unit`, add `talent: None,` (as the
   generic units in `crates/core/src/unit.rs` do).
2. In `crates/ui/src/screens/battle/rewind_tests.rs`, in `attacked()` and in
   `the_same_attack_after_a_rewind_gives_the_same_result`, replace the last
   `Confirm` of `[Confirm, Confirm, CursorRight, Confirm, Confirm]` (the skip)
   with `Cancel`, and fix the comment ("…attack, skip the playback (Cancel)").
3. Run `cargo test -p trpg-ui rewind`; review the snapshot if `insta` reports a
   change (it should match the committed one: the rewind screen before the
   attack).

## Acceptance criteria

- [x] `cargo test -p trpg-core` compiles and passes.
- [x] All 13 rewind tests in `trpg-ui` pass, including the four above, with no
      assertion removed or loosened.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit / integration: the existing rewind tests and core AI tests (no new
  tests: the failing tests are the reproduction).

## Completion notes

- Added `talent: None` to the AI tests' `fn unit`: `trpg-core` compiles and its
  518 tests pass.
- The rewind tests now skip the playback with Cancel, per 0418 and
  `docs/design/controls.md`. No assertion changed; the committed rewind snapshot
  matches as is. All 13 rewind tests pass.
- Root cause of the rewind failures: the 0418 control change, not 0601's EXP
  step or 0501's AI. 0307 and 0418 were each green on their own branch.
- No gameplay rules decided. No follow-up tickets.
