---
id: "0206"
title: "Web build: index.html shell, JS loader, cargo xtask web"
type: infra
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0203"]
nick_input: none
completed:
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

- [ ] `cargo xtask web --release` produces a working `dist/web/` on Windows (GNU host) and Linux CI.
- [ ] Game runs in Chrome and Firefox from a local static server; arrow keys don't scroll the page.
- [ ] CI uploads the web build artifact.
- [ ] Vendored JS version matches the macroquad crate version (noted in `web/README.md`).

## Tests required

Unit-test xtask's path/argument handling if it has logic beyond calling cargo.
Manual browser check (screenshots).

## Completion notes

