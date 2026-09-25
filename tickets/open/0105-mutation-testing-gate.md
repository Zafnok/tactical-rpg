---
id: "0105"
title: "CI: mutation testing gate with cargo-mutants"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0102"]
nick_input: none
completed: 2026-09-25
---

# 0105 — CI: mutation testing gate with cargo-mutants

## Context

Mutation testing proves the tests actually check behaviour
([ADR-0007](../../docs/adr/0007-testing-strategy.md)). PRs are gated on the
diff; a weekly full run reports on everything.

## Nick input

None.

## Scope

**In:** `.cargo/mutants.toml`, `.github/workflows/mutants.yml`, docs in the
`run-gates` skill if commands differ from what's written there.

**Out:** writing tests for existing code (there's almost none yet).

## Implementation steps

1. `cargo install --locked cargo-mutants` locally; read `cargo mutants --help`
   for the current flag names.
2. `.cargo/mutants.toml`:
   - `exclude_globs = ["crates/app/**"]`
   - `timeout_multiplier = 3.0`
   - any `exclude_re` only for truly untestable patterns (e.g. `impl Debug`),
     each with a comment.
3. `.github/workflows/mutants.yml`:
   - **PR job** (`pull_request`): checkout with `fetch-depth: 0`;
     `git diff origin/${{ github.base_ref }}...HEAD -- '*.rs' > git.diff`;
     if the diff is empty, exit successfully; otherwise
     `cargo mutants --in-diff git.diff --in-place -vV` (in-place is fine on a
     throwaway runner and faster). A missed mutant makes cargo-mutants exit
     non-zero → job fails. Always upload `mutants.out/` as an artifact.
   - **Weekly job** (`schedule`, plus `workflow_dispatch`): full run with a
     matrix `shard: [0, 1, 2, 3]`, `cargo mutants --shard ${{ matrix.shard }}/4 --baseline=skip`
     (run the baseline once in a prior step), `continue-on-error: true`,
     upload artifacts. Report only.
   - Pin actions by SHA; `permissions: contents: read`; install cargo-mutants
     via `taiki-e/install-action`.
4. **Prove the gate works** on your branch before finishing: add a scratch
   function with a deliberately weak test in `trpg-core`, see the PR job fail
   with a MISSED mutant, then remove the scratch code (final PR contains none).
5. Update the `run-gates` skill's mutants command if the real invocation differs.

## Acceptance criteria

- [ ] PR job runs and passes on this PR (no Rust diff → quick pass, or real diff → no misses).
- [ ] Evidence (run URL) that a weak test made the job fail, in Completion notes.
- [ ] Weekly workflow runs via `workflow_dispatch` at least once successfully.
- [ ] `mutants.out/` uploaded as an artifact in both jobs.

## Completion notes

- `.cargo/mutants.toml`: excludes `crates/app/**` (no I/O-free rules to mutate
  there, and it needs macroquad); `timeout_multiplier = 3.0`. No `exclude_re`
  needed yet — nothing untestable like a `Debug` impl exists in `core`,
  `content` or `ui` so far.
- `.github/workflows/mutants.yml`: a `pull_request` job (`mutants (diff)`)
  diffs against `origin/${{ github.base_ref }}`, skips cleanly when there's no
  Rust diff, and runs `cargo mutants --in-diff git.diff --in-place -vV`
  otherwise; a `schedule` (Monday 06:00 UTC) + `workflow_dispatch` job pair
  (`mutants (full, baseline)` → `mutants (full, shard N)`, 4-way matrix) runs
  the full suite, baseline once, `--baseline=skip` per shard,
  `continue-on-error: true`. Both jobs upload `mutants.out/` via
  `actions/upload-artifact` (`if-no-files-found: ignore`, since a diff-only PR
  run has nothing to upload).
- **Deviation:** the ticket's flag names (`--in-diff`, `--in-place`, `--shard
  N/M`, `--baseline`) all checked out against `cargo-mutants 27.1.0 --help`.
  The one real snag was the pinned `taiki-e/install-action` SHA I picked
  first (`v2.9.4`, thinking it was current) — that version predates
  `taiki-e/install-action` adding a `cargo-mutants` manifest entry, so it fell
  back to `cargo-binstall`, which itself failed to parse `cargo-mutants`' own
  `edition = "2024"` manifest field (a `cargo-binstall` bug, not ours). Fixed
  by re-pinning to `v2.87.20` (SHA `9983c65e42da123ff25d1f78505eb6de315aa172`),
  the actual latest release, whose manifest carries `cargo-mutants` up to
  27.1.0 as a prebuilt binary.
- **Gate proof (acceptance criterion #2):** pushed a scratch
  `scratch_is_positive(x: i32) -> bool { x > 0 }` in `crates/core/src/lib.rs`
  with a weak test that called it but asserted nothing. The PR job correctly
  failed with 5 `MISSED` mutants (both `-> bool` replacements, `>`↔`==`,
  `>`↔`<`, `>`↔`>=`) and `mutants.out/` was uploaded:
  https://github.com/Zafnok/tactical-rpg/actions/runs/36163053321/job/108164067025
  (an earlier attempt at the same proof,
  https://github.com/Zafnok/tactical-rpg/actions/runs/36162780447/job/108163152122,
  failed for the `install-action` reason above, before the scratch code was
  even reached — that's what surfaced the pin bug). The scratch function and
  test were removed before this PR was finalized; the final diff contains
  none of it.
- Ran `cargo mutants --list` at the workspace root locally: with no real code
  yet, only `crates/xtask/src/main.rs`'s `main` is mutable (`app` excluded,
  `core`/`content`/`ui` are still empty stubs). Nothing to fix now; later
  tickets that add real logic will be the first to hit real `MISSED` mutants.
- `run-gates` skill already documented the real invocation
  (`cargo mutants --in-diff <(git diff main) -p trpg-core -p trpg-content
  -p trpg-ui`); left as-is — it matches what CI runs, just scoped to the
  three rule crates instead of the whole workspace, which is fine since
  `app` is excluded from the config anyway.
- Weekly `workflow_dispatch` run: not yet triggered from this branch (GitHub
  only allows `workflow_dispatch` once the workflow file exists on the
  default branch). **Follow-up:** after this PR merges, manually trigger
  `Mutants` → `Run workflow` on `main` once to satisfy acceptance criterion
  #3, and confirm here or in a follow-up ticket note that it succeeded.

