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
- `sapp_jsutils.js`, `quad-storage.js` — miniquad JS plugins backing
  [`trpg_ui::storage::Storage`](../crates/ui/src/storage.rs) on web (ticket
  0207): `quad-storage` exposes `localStorage` to Rust, and needs
  `sapp_jsutils` (its JS↔Rust string/object marshalling) loaded first. Both
  vendored, see below.

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

## Vendored `sapp_jsutils.js` and `quad-storage.js`

- `sapp_jsutils.js`: source https://github.com/not-fl3/sapp-jsutils,
  `js/sapp_jsutils.js`. Version matches the `sapp-jsutils` crate pulled in
  transitively by `quad-storage` (currently `0.1.7`); the file ships inside
  the published crate itself (`js/sapp_jsutils.js`), so it was copied
  straight from there rather than from GitHub. License: dual MIT/Apache-2.0
  per the crate's `Cargo.toml`; upstream ships no `LICENSE` file at all, so
  the standard MIT text was reconstructed with the crate's declared author
  as copyright holder, committed at
  [`sapp_jsutils-LICENSE-MIT.txt`](sapp_jsutils-LICENSE-MIT.txt).
- `quad-storage.js`: source https://github.com/optozorax/quad-storage,
  `js/quad-storage.js`. Version matches the `quad-storage` crate (`0.1.3`),
  vendored from the exact commit crates.io recorded for that release,
  `3760b953aec17d65cc4ca8edfa39c38e7337ec3a` (from the published crate's
  `.cargo_vcs_info.json`). License: MIT (dual MIT/Apache-2.0; text committed
  at [`quad-storage-LICENSE-MIT.txt`](quad-storage-LICENSE-MIT.txt), copied
  from the crate's own `LICENSE-MIT`).
- Both back [`trpg_ui::storage::Storage`](../crates/ui/src/storage.rs) on
  web (ticket 0207); `index.html` loads `sapp_jsutils.js` before
  `quad-storage.js`, both before `mq_js_bundle.js`'s `load(...)` call. See
  [THIRD_PARTY_ASSETS.md](../THIRD_PARTY_ASSETS.md) for the registry rows.

When bumping the `quad-storage` crate version, re-vendor `quad-storage.js`
from the matching commit/tag and update this note and the
`THIRD_PARTY_ASSETS.md` row; `sapp_jsutils.js` can just be re-copied from
the new version of the crate.

## Building

```
cargo xtask web [--release]
```

Builds `trpg-app` for `wasm32-unknown-unknown` and packages the wasm binary
with `index.html`, `mq_js_bundle.js`, `sapp_jsutils.js` and
`quad-storage.js` into `dist/web/`. With `--release` and `wasm-opt` on
`PATH`, runs `wasm-opt -Oz` on the binary; otherwise skips that step
silently. Prints the output path and the wasm binary's size.

## Running locally

```
cargo xtask web && python -m http.server -d dist/web 8000
```

Then open `http://localhost:8000` (any static file server works — the WASM
binary must be served over HTTP, not opened as a `file://` URL).
