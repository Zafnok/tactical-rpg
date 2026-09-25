# Web build

Browser play via WASM ([ADR-0009](../docs/adr/0009-distribution.md)).

## Files

- `index.html` — the page shell: a full-window black `<canvas id="glcanvas">`,
  no margins or scrollbars, focused on load and on click, loads the bundle
  then `tactical-rpg.wasm`.
- `mq_js_bundle.js` — macroquad's JS loader, vendored (see below). It already
  calls `preventDefault` on the sokol-mapped arrow/space/tab keys inside its
  own `canvas.onkeydown` handler; `index.html` adds a small backstop
  `keydown` listener for browser shortcuts it doesn't cover (Page Up/Down,
  Home/End).

## Vendored `mq_js_bundle.js`

- Source: https://github.com/not-fl3/macroquad, `js/mq_js_bundle.js`.
- Version: matches the `macroquad` crate version pinned in the workspace
  `Cargo.lock` (currently `0.4.16`).
- **No git tag `v0.4.16` exists in the macroquad repo** (its tags jump from
  `v0.4.14` straight to `v0.4.1x`+ series releases that were published to
  crates.io from untagged commits). Vendored instead from the exact commit
  crates.io recorded for that release, `5e9b5ca912ac65962c05c0da842a4a70eaae34b9`,
  found in the published crate's `.cargo_vcs_info.json` and verified
  byte-for-byte identical to `js/mq_js_bundle.js` at that commit on GitHub.
- License: MIT (macroquad is dual MIT/Apache-2.0; the license text is
  committed at [`mq_js_bundle-LICENSE-MIT.txt`](mq_js_bundle-LICENSE-MIT.txt)).
- See [THIRD_PARTY_ASSETS.md](../THIRD_PARTY_ASSETS.md) for the registry row.

When bumping the `macroquad` crate version, re-vendor this file from the
matching tag (or commit, if no tag exists) and update this note and the
`THIRD_PARTY_ASSETS.md` row.

## Building

```
cargo xtask web [--release]
```

Builds `trpg-app` for `wasm32-unknown-unknown` and packages the wasm binary
with `index.html` and `mq_js_bundle.js` into `dist/web/`. With `--release`
and `wasm-opt` on `PATH`, runs `wasm-opt -Oz` on the binary; otherwise skips
that step silently. Prints the output path and the wasm binary's size.

## Running locally

```
cargo xtask web && python -m http.server -d dist/web 8000
```

Then open `http://localhost:8000` (any static file server works — the WASM
binary must be served over HTTP, not opened as a `file://` URL).
