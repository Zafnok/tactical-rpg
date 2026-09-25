---
id: "0102"
title: "CI: format, lint, test on 3 OSes, WASM build, docs"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0101"]
nick_input: none
completed:
---

# 0102 — CI: format, lint, test on 3 OSes, WASM build, docs

## Context

First CI gates from [ADR-0008](../../docs/adr/0008-ci-quality-gates.md).
GitHub Actions is free and unlimited for this public repo.

## Nick input

None.

## Scope

**In:** `.github/workflows/ci.yml` with jobs `fmt`, `clippy`, `test`
(matrix), `wasm`, `docs`.

**Out:** security scanners (0103), coverage/Sonar (0104), mutants (0105),
branch protection (0106), releases (0107).

## Implementation steps

1. Create `.github/workflows/ci.yml`:
   - `on: pull_request` and `push: branches: [main]`.
   - Top-level `permissions: contents: read`.
   - `concurrency: group: ci-${{ github.ref }}, cancel-in-progress: true`.
   - `env: CARGO_TERM_COLOR: always, RUSTFLAGS: -D warnings`.
2. Every job: `actions/checkout`, then `rustup show` (installs the toolchain
   from `rust-toolchain.toml`), then `Swatinem/rust-cache`.
3. **Pin every third-party action by full commit SHA** with a version comment,
   e.g. `uses: actions/checkout@<40-char-sha> # v4.2.2`. Find SHAs with
   `gh api repos/actions/checkout/git/ref/tags/v4.2.2 --jq .object.sha`
   (use the latest release tag of each action; dereference annotated tags if
   `.object.type` is `tag`).
4. Jobs:
   - `fmt` (ubuntu): `cargo fmt --all --check`.
   - `clippy` (ubuntu): `cargo clippy --workspace --all-targets --locked -- -D warnings`.
   - `test` (matrix `windows-latest`, `ubuntu-latest`, `macos-latest`,
     `fail-fast: false`): `cargo test --workspace --locked`.
   - `wasm` (ubuntu): `cargo build -p trpg-app --target wasm32-unknown-unknown --release --locked`.
   - `docs` (ubuntu): `cargo doc --workspace --no-deps --locked` with
     `RUSTDOCFLAGS: -D warnings`.
5. Linux runners: if macroquad needs system libraries to build, add a step
   `sudo apt-get update && sudo apt-get install -y libasound2-dev libx11-dev libxi-dev libgl1-mesa-dev`
   to the jobs that compile `trpg-app`. Only add what's actually needed.
6. Commit `Cargo.lock` (binary project) if 0101 didn't.
7. Open the PR and confirm all jobs pass.

## Acceptance criteria

- [ ] All five jobs run on the PR and pass; test matrix passes on all three OSes.
- [ ] All actions pinned by SHA with version comments.
- [ ] Workflow has `permissions: contents: read` at top level.
- [ ] Deliberately breaking formatting in a scratch commit makes `fmt` fail
      (verify, then revert before merging).

## Tests required

The workflow itself is the test. Record the run URL in Completion notes.

## Completion notes

