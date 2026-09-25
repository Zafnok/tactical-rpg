---
id: "0101"
title: Cargo workspace skeleton with empty crates and xtask
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-09-25
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
     `rust-version = "1.98"`, `publish = false`, `license-file = "LICENSE"`
     (proprietary, see ADR-0013; never `license = "MIT"` or similar),
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

- [x] `cargo build --workspace` and `cargo test --workspace` succeed on the GNU host.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [x] `cargo build -p trpg-app --target wasm32-unknown-unknown` succeeds.
- [x] `cargo run -p trpg-app` opens a window titled `tactical-rpg`.
- [x] `cargo xtask` prints a usage line.
- [x] Crate dependency directions match ADR-0004 (no `core` → anything).
- [x] No game logic added.

## Tests required

None beyond compiling (there is no logic yet).

## Completion notes

Built the workspace as specified: `rust-toolchain.toml`, root `Cargo.toml`
(workspace lints/profiles), `clippy.toml`, `rustfmt.toml`, the four game
crates (`trpg-core`, `trpg-content`, `trpg-ui`, `trpg-app`) each with only a
crate-doc `lib.rs`/`main.rs`, and `xtask` with the `cargo xtask` alias.

**Deviation — MSYS2 mingw-w64 toolchain now required locally.** Discovered
mid-ticket: rustup's self-contained `x86_64-pc-windows-gnu` linker (the
`rust-mingw-*` component) ships without `libimm32.a`. `miniquad` (via
`macroquad`) links `imm32` directly for IME support, so
`cargo build -p trpg-app` failed to link with "cannot find -limm32" — this
blocks every future ticket that touches `app`, not just this one. Tried a
`build.rs` workaround generating a minimal import library at build time
(first via `dlltool`, which needs an assembler that isn't bundled either;
then a hand-rolled short-form COFF import object, which linked but produced
a corrupt import table that segfaulted at runtime) — rejected both as too
fragile to ship. Asked Nick; he chose to install a full MSYS2 mingw-w64
toolchain (`winget install -e --id MSYS2.MSYS2`, then
`pacman -S mingw-w64-x86_64-gcc`), which ships a complete import library set.
`.cargo/config.toml` now points the `x86_64-pc-windows-gnu` target's linker
and `ar` at the MSYS2 install; documented as a setup step in `CLAUDE.md`. All
other tooling (rustc, cargo, clippy, rustfmt) is still plain rustup — MSYS2
supplies only the linker/lib set.

Also fixed two clippy findings in `xtask`'s stub `main` (`single_match_else`,
`print_stdout` — the latter allowed at the crate root since a CLI tool
printing usage to stdout is its whole job) that only surfaced once the crate
actually compiled.

Ran the `run-gates` skill: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, and
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` are all clean.
`cargo deny`, `cargo machete`, `typos` and mutation testing are not wired up
yet (ADR-0008: after tickets 0103/0105).

No follow-up tickets needed — the MSYS2 requirement is captured directly in
`CLAUDE.md`'s Environment section for future sessions.
