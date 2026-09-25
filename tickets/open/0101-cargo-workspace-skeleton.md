---
id: "0101"
title: Cargo workspace skeleton with empty crates and xtask
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0101 — Cargo workspace skeleton with empty crates and xtask

## Context

First code ticket. Creates the workspace described in
[ADR-0004](../../docs/adr/0004-crate-architecture.md) with **no game logic**.
Every later ticket builds on this.

## Nick input

None.

## Scope

**In:** workspace `Cargo.toml`, toolchain pin, lint config, formatter config,
four game crates that compile, an `app` binary that opens an empty macroquad
window, and an empty `xtask` tooling crate (later tickets add commands to it).

**Out (do not do):** any game types (grid, units, glyph buffer…), CI workflows
(0102), cargo-deny config (0103), assets.

## Implementation steps

1. `rust-toolchain.toml` at repo root:
   ```toml
   [toolchain]
   channel = "1.98.1"          # current stable at time of writing; pin exactly
   components = ["rustfmt", "clippy"]
   targets = ["wasm32-unknown-unknown"]
   ```
   Do **not** set a host triple — Nick's machine uses the GNU host, CI uses MSVC.
2. Root `Cargo.toml`:
   - `[workspace]` with `members = ["crates/*"]`, `resolver = "3"`.
   - `[workspace.package]`: `version = "0.1.0"`, `edition = "2024"`,
     `rust-version = "1.98"`, `publish = false`,
     `repository = "https://github.com/Zafnok/tactical-rpg"`.
   - `[workspace.dependencies]`: `macroquad` (latest 0.4.x). Nothing else yet.
   - `[workspace.lints.rust]`: `unsafe_code = "forbid"`, `missing_docs = "warn"`.
   - `[workspace.lints.clippy]`: `all = { level = "warn", priority = -1 }`,
     `pedantic = { level = "warn", priority = -1 }`, then allow the noisy ones:
     `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`,
     `missing_panics_doc`. Also `unwrap_used = "warn"`, `expect_used = "warn"`,
     `dbg_macro = "warn"`, `todo = "warn"`, `print_stdout = "warn"`.
   - `[profile.release]`: `lto = "thin"`, `codegen-units = 1`, `strip = true`.
   - `[profile.dev.package."*"]`: `opt-level = 2` (dependencies fast in debug).
3. `clippy.toml`: `allow-unwrap-in-tests = true`, `allow-expect-in-tests = true`.
4. `rustfmt.toml`: `max_width = 100`, `edition = "2024"`.
5. Crates, each with `[lints] workspace = true` and `version.workspace = true`
   etc.:
   - `crates/core` → package `trpg-core`, lib. `src/lib.rs`: crate doc comment
     only ("Pure, deterministic game rules. See ADR-0004.").
   - `crates/content` → `trpg-content`, lib, depends on `trpg-core`.
   - `crates/ui` → `trpg-ui`, lib, depends on `trpg-core`, `trpg-content`.
   - `crates/app` → `trpg-app`, `[[bin]] name = "tactical-rpg"`, depends on
     all three + `macroquad`. `main.rs`: a `#[macroquad::main(window_conf)]`
     loop that clears to black and draws the text `tactical-rpg` so we know the
     window works. `window_conf` sets title `"tactical-rpg"`, 1600×1024,
     resizable.
6. Tooling crate `crates/xtask` (package `xtask`, `publish = false`, binary)
   with a `main` that matches on the first argument and prints a usage line
   listing available commands (none yet) for unknown/missing commands, exiting
   with code 2. Add `.cargo/config.toml` with
   `[alias] xtask = "run -q -p xtask --"`. No `clap` — plain `std::env::args`.
   `cargo xtask` must print the usage line.
7. Verify locally (GNU host):
   ```bash
   cargo build --workspace
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo fmt --all --check
   rustup target add wasm32-unknown-unknown
   cargo build -p trpg-app --target wasm32-unknown-unknown
   cargo run -p trpg-app      # window opens; close it
   ```
8. Update the "Map of the repo" in `CLAUDE.md` (crates now exist) and the
   README "Status" line only if wording is now wrong.

## Acceptance criteria

- [ ] `cargo build --workspace` and `cargo test --workspace` succeed on the GNU host.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [ ] `cargo build -p trpg-app --target wasm32-unknown-unknown` succeeds.
- [ ] `cargo run -p trpg-app` opens a window titled `tactical-rpg`.
- [ ] `cargo xtask` prints a usage line.
- [ ] Crate dependency directions match ADR-0004 (no `core` → anything).
- [ ] No game logic added.

## Tests required

None beyond compiling (there is no logic yet).

## Completion notes

