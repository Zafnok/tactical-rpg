---
id: "0207"
title: "Storage abstraction: files on native, localStorage on web"
type: feature
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0205"]
nick_input: none
completed:
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

- [ ] `MemoryStorage` passes a shared conformance test suite (write/read/overwrite/delete/list/invalid key).
- [ ] `FileStorage` passes the same suite against a temp directory (`tempfile` dev-dependency).
- [ ] Native debug run logs a successful smoke check; web debug run does too (browser console).
- [ ] No `std::fs` usage outside `app`.

## Tests required

- Unit: key validation (property test over random strings: accepted ⇔ regex matches).
- Conformance suite run against both implementations (web impl verified manually).

## Completion notes

