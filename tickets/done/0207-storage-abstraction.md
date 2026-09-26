---
id: "0207"
title: "Storage abstraction: files on native, localStorage on web"
type: feature
milestone: M1 Engine
model: sonnet-5
effort: medium
status: done
blocked_by: ["0205"]
nick_input: none
completed: 2026-09-26
---

# 0207 — Storage abstraction

## Context

Saves (0802) and settings/keybinding overrides (0805) must persist on native
and in the browser ([ADR-0009](../../docs/adr/0009-distribution.md)). `trpg-ui`
stays pure, so it defines a trait; `app` implements it per platform.

## Nick input

None.

## Scope

**In:** `trpg_ui::storage::Storage` trait, `MemoryStorage` (tests), native
file implementation and web implementation in `app`, wiring into `Ctx`.

**Out:** save file format (0802), settings contents (0805).

## Implementation steps

1. `trpg-ui::storage`:
   ```rust
   pub trait Storage {
       fn read(&self, key: &str) -> Result<Option<String>, StorageError>;
       fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError>;
       fn delete(&mut self, key: &str) -> Result<(), StorageError>;
       fn list(&self) -> Result<Vec<String>, StorageError>;
   }
   ```
   Keys are validated: `^[a-z0-9_-]{1,64}$` (reject others with an error — they
   become file names). `MemoryStorage` implements it with a `BTreeMap`.
2. Add `Box<dyn Storage>` to `Ctx` (0205). Harness uses `MemoryStorage`.
3. `app` native (`cfg(not(target_arch = "wasm32"))`): `FileStorage` using the
   `directories` crate (`ProjectDirs::from("", "", "tactical-rpg")` →
   `data_dir()`); key → `<dir>/<key>.ron`; **atomic writes** (write
   `<key>.ron.tmp`, then rename); create the directory on first write.
4. `app` web (`cfg(target_arch = "wasm32")`): `WebStorage` over `localStorage`
   using `quad-storage` (check it's maintained and works with the macroquad
   version; if not, write a tiny JS plugin per macroquad's plugin docs). Prefix
   keys with `tactical-rpg/`.
5. A debug-only smoke check: in debug builds, on startup write and read back a
   `smoke_test` key and log the result (remove the key afterwards).

## Acceptance criteria

- [x] `MemoryStorage` passes a shared conformance test suite (write/read/overwrite/delete/list/invalid key).
- [x] `FileStorage` passes the same suite against a temp directory (`tempfile` dev-dependency).
- [ ] Native debug run logs a successful smoke check; web debug run does too (browser console) — **not verified end-to-end**: this environment has no X display (`cargo run -p trpg-app` panics on `XOpenDisplay()` before reaching game code) and no browser/wasm runner. `smoke_check`'s own logic is covered by `FileStorage`'s unit tests and `run_smoke_check`'s straight-line code, and it's wired into `main.rs` before the render loop starts; Nick should confirm the log line on his machine (native) and in the browser console (web) the first time he runs a build from this branch.
- [x] No `std::fs` usage outside `app`.

## Tests required

- Unit: key validation (property test over random strings: accepted ⇔ regex matches, `crates/ui/src/storage.rs`).
- Conformance suite (`trpg_ui::storage::conformance_suite`, gated behind the `harness` feature) run against `MemoryStorage` (in `trpg-ui`) and `FileStorage` (in `trpg-app`, against a `tempfile` temp dir). Also a test proving the suite itself catches a broken implementation (kills the "replace with a no-op" mutant), and native-only tests for atomic writes and directory-creation-on-first-write.

## Completion notes

- `Storage`, `StorageError`, `MemoryStorage`, `is_valid_key` and the shared
  `conformance_suite` live in `crates/ui/src/storage.rs`; `Ctx` now holds
  `pub storage: Box<dyn Storage>` (defaulting to `MemoryStorage`, so the
  harness and existing `Ctx::new`/`Ctx::embedded` call sites are unchanged)
  plus a `Ctx::with_storage` builder method `app` uses to install the real
  backend. Dropped `Ctx`'s `Clone` derive (nothing used it) since
  `Box<dyn Storage>` isn't `Clone`; kept `Debug` by giving `Storage` a
  `fmt::Debug` supertrait.
- **Deviation from the ticket's step 3:** used a hand-rolled `data_dir()`
  (env vars: `%APPDATA%` on Windows, `~/Library/Application Support` on
  macOS, `$XDG_DATA_HOME`/`~/.local/share` elsewhere) instead of the
  `directories` crate. `directories` → `dirs-sys` unconditionally depends on
  `option-ext` (MPL-2.0), which ADR-0013 explicitly denies, and `cargo deny
  check licenses` confirmed the rejection. `dirs` has the same transitive
  dependency. The three lines of env-var logic needed for our three shipped
  native targets (ADR-0009) didn't seem worth a licensing exception.
- `quad-storage` (web `localStorage`) checked out fine: MIT/Apache-2.0,
  small, and the crate's own author is macroquad's maintainer. It hasn't
  been updated since 2021, but the API is tiny and stable; noted in the PR
  for visibility rather than blocking on it.
- `quad-storage` needs its own JS plugin (`quad-storage.js`) plus
  `sapp_jsutils.js` (its JS↔Rust marshalling) loaded in `web/index.html`
  before the wasm module — both vendored, license text included, rows added
  to `THIRD_PARTY_ASSETS.md`, `web/README.md` explains provenance.
  `sapp-jsutils` ships no upstream `LICENSE` file despite declaring
  `MIT/Apache-2.0` in `Cargo.toml`, so the standard MIT text was
  reconstructed with the crate's declared author as copyright holder.
- Linking `quad-storage`/`sapp-jsutils` for `wasm32-unknown-unknown` failed
  with "undefined symbol" for their JS-plugin `extern "C"` functions until
  `-C link-arg=--allow-undefined` was added for that target in
  `.cargo/config.toml` — necessary for any miniquad JS plugin, not just this
  one, so documented there for the next one.
- `cargo xtask web` now also copies `sapp_jsutils.js` and `quad-storage.js`
  into `dist/web/` (`SHELL_FILES` in `crates/xtask/src/web.rs`, tests
  updated).
- Ran the full `run-gates` list locally: `fmt`, `clippy -D warnings` (both
  workspace, all targets), `cargo test --workspace`, `cargo doc` with `-D
  warnings`, `cargo build -p trpg-app --target wasm32-unknown-unknown`,
  `cargo deny check`, `cargo machete`, `typos`, and `cargo mutants --in-diff`
  on `trpg-ui`/`trpg-app` (all mutants caught after adding the
  broken-storage test above). No follow-up tickets opened; nothing found
  outside this ticket's scope.
