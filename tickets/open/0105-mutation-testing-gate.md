---
id: "0105"
title: "CI: mutation testing gate with cargo-mutants"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: in-progress
blocked_by: ["0102"]
nick_input: none
completed:
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

