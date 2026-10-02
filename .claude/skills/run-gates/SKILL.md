---
name: run-gates
description: Run the local quality gates (fmt, clippy, tests, content validation, snapshot review, WASM build) before pushing. Mutation testing runs in CI only. Use before every commit you intend to push, and whenever CI fails and you need to reproduce it.
---

# Run the local gates

These mirror CI (ADR-0014). Commands become valid once ticket 0101 has created
the workspace; tools are added by tickets 0102–0105. If a tool isn't installed
locally, install it with `cargo install --locked <tool>`.

## Environment facts

- Nick's Windows machine uses the **GNU** Rust host toolchain
  (`x86_64-pc-windows-gnu`); **MSVC is not installed**. Don't add anything that
  needs MSVC locally. CI builds MSVC.
- `cargo-binstall` doesn't build on this machine; use `cargo install --locked`.

## Order (fast → slow; stop at first failure and fix)

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask check-keys                    # no hard-coded keys (keyboard-input skill)
cargo doc --workspace --no-deps          # with RUSTDOCFLAGS="-D warnings"
cargo build -p trpg-app --target wasm32-unknown-unknown
cargo deny check                          # after ticket 0103
cargo machete                             # after ticket 0103
typos                                     # after ticket 0103
```

**The gates never read the bought art** (ADR-0040): run them exactly as
above, without `--features private-assets`, whether or not `assets-private/`
is on the machine. A ticket that changes files in `assets-private/game/`
also runs the one test that opts in, and looks at the result in a build:

```bash
cargo test -p trpg-content --features private-assets --test private_assets
cargo run -p trpg-app --features private-assets
```

**Mutation testing is not a local gate.** CI's `mutants (diff)` job
(`.github/workflows/mutants.yml`) runs `cargo mutants` on every PR's diff.
Running it locally too means waiting for it twice. So: run the gates above,
push, open the PR and watch CI. If CI reports `MISSED` mutants, fix them on
the branch (see *Mutation testing results* below). Run it locally only to
reproduce a CI mutants failure, and scope it to the mutants in question
(e.g. `cargo mutants --in-diff <(git diff origin/main) -f <file>`). Diff
against `origin/main`, because the local `main` may be stale.

On PowerShell, write the diff to a file first:
`git diff main > $env:TEMP\pr.diff; cargo mutants --in-diff $env:TEMP\pr.diff`.

## Snapshots (insta)

- A failing snapshot test writes `*.snap.new`. **Read the new snapshot** and
  decide if the change is intended.
- If intended: `cargo insta accept`. If not: fix the code.
- Never accept snapshots blindly; `cargo insta review` is interactive and won't
  work in this environment.

## Mutation testing results

- `MISSED` mutants in code you wrote = your tests don't check that behaviour.
  Add an assertion that would fail under that mutant.
- `#[mutants::skip]` only with a comment explaining why the code is untestable
  (e.g. `Debug` formatting), never to make the gate pass.

## When CI fails but local passes

Most likely: OS difference (path separators, line endings), MSVC vs GNU, or a
tool version. Reproduce with the exact command from the failing job's log.
