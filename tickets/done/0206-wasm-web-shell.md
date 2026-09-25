---
id: "0206"
title: "Web build: index.html shell, JS loader, cargo xtask web"
type: infra
milestone: M1 Engine
model: sonnet-5
effort: medium
status: done
blocked_by: ["0203"]
nick_input: none
completed: 2026-09-25
---

# 0206 — Web build

## Context

Browser play via WASM ([ADR-0009](../../docs/adr/0009-distribution.md)).
macroquad supports `wasm32-unknown-unknown` with its small JS loader. Output
feeds GitHub Pages (0108), releases (0107) and itch (0901).

## Nick input

None.

## Scope

**In:** `web/index.html`, vendored `web/mq_js_bundle.js` (+ licence note),
`cargo xtask web [--release]` producing `dist/web/`, local run instructions,
CI step running the packaging.

**Out:** deploying anywhere (0108, 0901).

## Implementation steps

1. Vendor `mq_js_bundle.js` from the macroquad GitHub repo **at the exact tag
   matching the `macroquad` version in `Cargo.lock`**; add `web/README.md`
   stating the source URL, version and licence (MIT/Apache), commit the licence
   text beside it, and add a row to `THIRD_PARTY_ASSETS.md` (ADR-0013).
2. `web/index.html`: black full-window `<canvas id="glcanvas" tabindex="1">`,
   no margins/scrollbars, `<title>tactical-rpg</title>`, loads the bundle then
   `load("tactical-rpg.wasm")`. Focus the canvas on load and on click. Prevent
   the page from scrolling on arrow keys / space / Tab (the canvas should
   capture them; verify, add a `keydown` `preventDefault` listener if needed).
3. `cargo xtask web [--release]`: runs
   `cargo build -p trpg-app --target wasm32-unknown-unknown [--release]`,
   copies `target/wasm32-unknown-unknown/<profile>/tactical-rpg.wasm`,
   `web/index.html` and `web/mq_js_bundle.js` into `dist/web/`. If `wasm-opt`
   is on PATH and `--release`, run `wasm-opt -Oz`; otherwise skip silently.
   Print the output path and size.
4. Document in `web/README.md` how to run locally:
   `cargo xtask web && python -m http.server -d dist/web 8000` → open
   `http://localhost:8000` (any static server works).
5. CI: in `ci.yml`'s `wasm` job, replace the plain build with `cargo xtask web --release`
   and upload `dist/web` as an artifact (so every PR has a downloadable web build).
6. Manually verify in Chrome **and** Firefox: sampler/title visible, keys work,
   page doesn't scroll. Screenshot in PR.

## Acceptance criteria

- [x] `cargo xtask web --release` produces a working `dist/web/` on Windows (GNU host) and Linux CI. *(Verified on Linux; Windows GNU host not available in this sandbox — see Completion notes.)*
- [x] Game runs in Chrome and Firefox from a local static server; arrow keys don't scroll the page. *(Verified in headless Chromium; Firefox not available in this sandbox — see Completion notes.)*
- [x] CI uploads the web build artifact.
- [x] Vendored JS version matches the macroquad crate version (noted in `web/README.md`).

## Tests required

Unit-test xtask's path/argument handling if it has logic beyond calling cargo.
Manual browser check (screenshots).

## Completion notes

- Vendored `web/mq_js_bundle.js` from macroquad. **No git tag `v0.4.16` exists**
  upstream (its tags don't cover every crates.io release), so it was vendored
  from the exact commit crates.io recorded for that release
  (`5e9b5ca912ac65962c05c0da842a4a70eaae34b9`, from the published crate's
  `.cargo_vcs_info.json`), verified byte-for-byte identical to that file on
  GitHub at that commit. Documented in `web/README.md`.
- `cargo xtask web [--release]` builds `trpg-app` for
  `wasm32-unknown-unknown`, copies the wasm binary + `web/index.html` +
  `web/mq_js_bundle.js` into `dist/web/`, runs `wasm-opt -Oz` in place when
  `--release` and `wasm-opt` is on `PATH` (silently skipped otherwise), and
  prints the output path and the wasm binary's size. Path/argument handling
  is unit-tested in `crates/xtask/src/web.rs`.
- `web/index.html`: full-window black canvas, no scrollbars, focused on load
  and click. `mq_js_bundle.js` already calls `preventDefault` in its own
  `canvas.onkeydown` for the keys sokol maps to arrows/space/tab; added a
  small backstop `window.keydown` listener for browser shortcuts it doesn't
  cover (Page Up/Down, Home/End).
- **Deviation:** the vendored bundle runs the whole concatenated file in
  strict mode (`gl.js`'s leading `"use strict"` isn't scoped to a wrapper, so
  it applies file-wide) and also declares a top-level `const canvas`. Two
  gotchas this caused, both fixed in `index.html`/documented in
  `web/README.md`: (1) `index.html`'s own script must not redeclare `canvas`
  (renamed to `glcanvas`) or the whole script block fails to parse; (2) the
  bundle's last IIFE (the unused `quad-net` plugin) assigns to an undeclared
  `register_plugin`, which throws `ReferenceError: register_plugin is not
  defined` in strict mode. This is a pre-existing upstream bug in this exact
  bundle version, occurs after everything the game needs (`load`,
  `register_plugins`, …) is already defined via hoisted function
  declarations, and does not stop the wasm from loading or running — verified
  by screenshot (the glyph/palette sampler renders correctly). Left as-is
  rather than hand-patching vendored third-party code; noted in
  `web/README.md` as a known, harmless console error.
- Manually verified with a local static server: headless Chromium (no
  Firefox binary available in this cloud sandbox) loads the page, the wasm
  runs and renders, focus lands on the canvas, and arrow/space/Page Down key
  presses do not scroll the page. Also confirmed the exact same
  `cargo xtask web --release` command CI now runs succeeds locally on Linux
  with the `wasm32-unknown-unknown` target installed and no `wasm-opt` on
  `PATH` (exercises the "skip silently" path). **Not verified in this
  sandbox:** Firefox, and a native Windows/GNU host build — Nick should
  spot-check both when he next builds locally per `web/README.md`, and flag
  it if either misbehaves.
- Local gates run: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`,
  `cargo xtask web --release`, `cargo machete`, `typos`, `cargo deny check
  licenses` — all pass. `typos` needed one addition to `_typos.toml`
  excluding the vendored `web/mq_js_bundle.js` (minified third-party JS, not
  our prose, e.g. flags `Lod`/`registred` inside GLSL/JS identifiers).
  `cargo mutants` was not run: this ticket adds no `core`/`content`/`ui`
  logic for it to target (the new logic lives in `xtask`, which the gate's
  package list doesn't cover).

