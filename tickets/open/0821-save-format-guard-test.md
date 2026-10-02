---
id: "0821"
title: Save format guard - a golden save fails the build when a saved type changes without a version bump
type: infra
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: low
status: todo
blocked_by: ["0802"]
nick_input: none
completed:
---

# 0821 — Save format guard test

## Context

Saves are RON `SaveFile`s with a `SAVE_VERSION`
([ADR-0039](../../docs/adr/0039-save-file-format.md), ticket 0802). The
version must be raised whenever a saved type (`Campaign`, `Unit`,
`BattleState`, `BattleHistory`, `Command`, anything inside them) changes
shape or meaning; then old saves show "This save is from an incompatible
version" instead of failing to parse ("This save can't be read") or, worse,
loading as something else.

Nothing enforces that today: a ticket that adds a field to `Unit` passes
every gate without touching `SAVE_VERSION`. Found while writing ADR-0039.
It must be in place before saves reach players (0901 publishes to itch.io,
so 0901 is blocked by this ticket).

## Nick input

`None.`

## Scope

**In:**
- Golden save files for the current `SAVE_VERSION` and a test that reads
  them.
- A short "when this test fails" note where the next model will see it.

**Out (do not do):**
- Migrations of old saves (no ticket yet; ADR-0039 says saves of another
  version are refused).
- Any change to the save format itself.

## Implementation steps

1. Add `crates/core/tests/it/save_format.rs` and two fixtures under
   `crates/core/tests/fixtures/`: `save_v1_chapter.ron` (a
   `SaveFile::chapter_cleared`) and `save_v1_suspend.ron` (a
   `SaveFile::suspended` of a battle with a few commands, at least one
   attack, a rewind charge spent). Build them once from the fixtures in
   `crates/core/src/battle/tests.rs` (`setup`, `cast`, `attack`) with a
   one-off test that prints `ron::to_string(&save)`, and commit the text.
   Name the files after the version they hold. A crate keeps one
   integration-test program (ticket 0114; an `xtask` test enforces it), so
   first `git mv crates/core/tests/replay.rs crates/core/tests/it/replay.rs`
   and add `tests/it/main.rs` with `mod replay;` and `mod save_format;`.
2. The test, for each fixture named after the current `SAVE_VERSION`:
   - parse it as `SaveFile` (must succeed);
   - serialise it again and compare with the fixture text, byte for byte
     (catches a new field with `#[serde(default)]`, a renamed variant, a
     reordered field);
   - for the suspend save: `restore_tables`, `state_at(len)`, and compare a
     few facts written in the test (turn, a unit's HP, charges left), so a
     rule change that makes old commands replay differently is caught too.
3. The test's failure message says what to do: "a saved type changed: raise
   `SAVE_VERSION` in `crates/core/src/save.rs`, regenerate the fixtures as
   `save_v<N>_*.ron` and delete the old ones (ADR-0039)". Put the same two
   sentences in the doc comment of `SAVE_VERSION`.
4. A second test: every fixture of an **older** version still in the folder
   (none at first) parses as `SaveHeader` with that version.

## Acceptance criteria

- [ ] Adding a field to `Unit` (try it locally, then revert) makes
      `cargo test -p trpg-core --test it save_format::` fail with the message
      from step 3.
- [ ] The fixtures are named after the version they hold, and the test
      fails if no fixture exists for the current `SAVE_VERSION`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration (`crates/core/tests/`): the golden-file tests above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
