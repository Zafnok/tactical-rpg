---
id: "0114"
title: One integration-test program per crate
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0113"]
nick_input: none
completed: 2026-10-02
---

# 0114 — One integration-test program per crate

## Context

Cargo builds every file directly under a crate's `tests/` folder as its own
program, and each one links a full copy of the crates it tests. On
2026-10-01 `crates/ui/tests/` had 9 files (about 100–115 MB each as debug
`.exe`s on the GNU toolchain) and `crates/content/tests/` had 4 (about
70 MB each). Together with the unit-test programs, the game and `xtask`,
one `cargo test --workspace` links about 20 programs, and every new
`tests/*.rs` file adds another. See ticket 0113 for the measurements; that
ticket shrinks each program, this one cuts how many there are (and the link
time that goes with them).

Blocked by 0113 only so the before/after sizes are measured against the
smaller debug info and the two don't conflict in `Cargo.toml`.

Other open tickets name these files; either order works:

- 0803 adds `crates/ui/tests/ch01_winnable.rs`. If 0114 is done first, 0803
  adds it as `crates/ui/tests/it/ch01_winnable.rs` plus a `mod` line; if
  0803 is done first, this ticket moves that file with the rest.
- 0505 refers to `crates/content/tests/ai_soak.rs`; after this ticket it is
  `crates/content/tests/it/ai_soak.rs`. Update the path in 0505's text (and
  in any other open ticket `grep -rn "crates/.*/tests/" tickets/open` finds)
  in this PR.
- PRs open when this ticket starts that add or change files under
  `crates/ui/tests/` will conflict. Merge `main` into them after this lands
  and move their new test files and snapshots the same way.

## Nick input

`None.`

## Scope

**In:**
- `crates/ui/tests/` and `crates/content/tests/`: one test program each.
- Renaming the insta snapshot files to match.
- A rule in the docs so new test files join the single program.

**Out (do not do):**
- Don't change what any test checks, and don't delete or weaken a test.
- Don't touch `crates/core/tests/replay.rs` or `crates/xtask/tests/cli.rs`
  (one file each already).
- Don't move unit tests out of `src/`.
- Don't accept a snapshot whose *content* changed; only file names and the
  `source:` header line may differ.

## Implementation steps

1. Record the number and total size of `target/debug/deps/*.exe` after
   `cargo test --workspace --no-run` in a fresh worktree.
2. **`crates/ui`:** `git mv` every `crates/ui/tests/<name>.rs` to
   `crates/ui/tests/it/<name>.rs` and add `crates/ui/tests/it/main.rs` with
   one `mod <name>;` line per file and a doc comment that says new
   integration tests are added here as modules. Cargo picks up
   `tests/it/main.rs` as the test target `it` without a `[[test]]` entry.
   Move helper code shared by several files into `tests/it/common.rs` only
   if it is already duplicated; otherwise leave the files as they are.
3. **Snapshots.** insta names a snapshot file after the module path, so
   `battle__title.snap` becomes `it__battle__title.snap`, and the folder
   moves next to the source file (`crates/ui/tests/it/snapshots/`).
   `git mv` each file to its new name and folder rather than regenerating,
   then run `cargo test -p trpg-ui`. If insta writes any `*.snap.new`, read
   the diff: only the `source:` line may change. Fix names until no
   `.snap.new` is written, and leave no orphaned `.snap` behind (`cargo
   insta test --unreferenced reject` if `cargo-insta` is installed).
4. **`crates/content`:** the same move to `crates/content/tests/it/` with
   `main.rs` (no snapshots there).
5. Check nothing names the old test targets: `grep -rn -- "--test "
   .github .claude docs crates` and fix any hit (`--test battle` becomes
   `--test it battle::`).
6. **Docs.** In ADR-0007's successor note or, if ADR-0007 can't be edited
   (accepted ADRs are superseded, not edited), in the `work-ticket` skill's
   *3. Implement* list: "New integration tests go in the crate's
   `tests/it/` as a module listed in `main.rs`; never add a file directly
   under `tests/`." Add an `xtask` test next to the existing workspace-lint
   test that fails when any `crates/*/tests/` folder holds more than one
   `.rs` file at its top level.
7. Record the new count and size from step 1's command.

## Acceptance criteria

- [x] `cargo test --workspace` runs the same number of tests as before
      (compare the `test result:` totals; put both in the Completion notes).
- [x] `crates/ui/tests/` and `crates/content/tests/` each build exactly one
      test program (`it`).
- [x] No snapshot content changed: `git diff --stat -M main -- '*.snap'`
      shows renames only (at most the `source:` line differs).
- [x] An `xtask` test fails if a second `.rs` file is added directly under
      any crate's `tests/` folder.
- [x] The Completion notes give the `.exe` count and total size before and
      after.
- [x] All gates in the `run-gates` skill pass, and CI's `mutants (diff)` job
      passes on the PR.

## Tests required

- Unit: the `xtask` test from step 6.
- Property: none new.
- Snapshot / integration: the existing ones, moved, all passing unchanged.

## Completion notes

- `crates/ui/tests/*.rs` (13 files) and `crates/content/tests/*.rs` (6 files,
  counting `private_assets.rs`, added by 0110 after this ticket was written)
  moved to `tests/it/`, each crate with a `tests/it/main.rs` listing them.
  The 45 ui snapshots moved to `crates/ui/tests/it/snapshots/` as
  `it__<old name>.snap`; `git diff -M --stat origin/main -- '*.snap'` shows
  45 renames with 0 lines changed (not even `source:`; insta matched them
  as they were). `cargo insta test -p trpg-ui --unreferenced reject` finds
  no orphans. No shared helpers moved to `common.rs`.
- `private_assets` is now `#[cfg(feature = "private-assets")] mod
  private_assets;` in `main.rs`; its command is `cargo test -p trpg-content
  --features private-assets --test it private_assets::` (updated in
  `pages.yml`, the `run-gates` skill and the file's doc comment). Ran it once
  with the feature on: 1 passed.
- **Programs built by `cargo test --workspace --no-run`** (fresh worktree,
  `target/debug/deps/*.exe`): before **29, 1,181,719,940 bytes (1.18 GB)**;
  after **12, 473,993,488 bytes (0.47 GB)**.
- **Test totals** (`cargo test --workspace --no-fail-fast`, sum of `test
  result:` lines): before 2440 passed, 0 failed, 3 ignored; after 2441
  passed, 0 failed, 3 ignored. The one extra is the new `xtask` test
  `each_crate_has_at_most_one_integration_test_program`, which fails when
  any `crates/*/tests/` holds more than one `.rs` file at its top level (and
  checks on a temp folder that a second file is seen).
- The rule is in the `work-ticket` skill's *3. Implement* list (ADR-0007 is
  accepted, so not edited), in both `main.rs` doc comments and in
  `crates/ui/README.md`.
- Paths updated in open tickets 0229, 0230, 0434, 0803; 0505 is already
  done. **0821** planned a second file in `crates/core/tests/`, which the
  new test would reject: its step 1 now says to move `replay.rs` into
  `crates/core/tests/it/` with a `main.rs` first. Accepted ADRs 0021 and
  0040 still name the old paths (0040's `--test private_assets` command is
  now `--test it private_assets::`); left unedited per the ADR rule.
- No gameplay rules decided. Nothing changes in the game.
