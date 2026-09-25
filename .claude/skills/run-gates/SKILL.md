---
name: run-gates
description: Run the local quality gates (fmt, clippy, tests, content validation, snapshot review, WASM build, mutation testing on the diff) before pushing. Use before every commit you intend to push, and whenever CI fails and you need to reproduce it.
---

# Run the local gates

These mirror CI (ADR-0008). Commands become valid once ticket 0101 has created
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
cargo doc --workspace --no-deps          # with RUSTDOCFLAGS="-D warnings"
cargo build -p trpg-app --target wasm32-unknown-unknown
cargo deny check                          # after ticket 0103
cargo machete                             # after ticket 0103
typos                                     # after ticket 0103
cargo mutants --in-diff <(git diff main) -p trpg-core -p trpg-content -p trpg-ui   # after ticket 0105
```

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
