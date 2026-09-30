---
id: "0223"
title: Fix the console errors the web build logs on load
type: bug
milestone: M1 Engine
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-09-30
---

# 0223 — Fix the console errors the web build logs on load

## Context

Nick opened the GitHub Pages build and found these in the browser console
(all reported from `mq_js_bundle.js`):

1. `Uncaught ReferenceError: register_plugin is not defined`. The vendored
   macroquad bundle (`web/mq_js_bundle.js`, ticket 0206) begins with
   `"use strict"`, and its last plugin, `quad_net`, assigns to an undeclared
   global `register_plugin`. Strict mode throws there, so the bundle stops
   before registering `quad_net`. We don't use `quad_net`, but the uncaught
   error is noise that hides real errors.
2. `Plugin quad_storage version mismatchjs version: 0.1.2, crate version:
   65536`. `web/quad-storage.js` (ticket 0207) declares `version: "0.1.2"`,
   but the loader compares it with `quad_storage_crate_version()` from the
   `quad-storage-sys` crate, which returns `(major << 24) + (minor << 16) +
   patch`: `65536` for 0.1.0. The strings can never match. Storage works
   anyway; only the check fails.
3. `The AudioContext was not allowed to start` and title music silent until
   the first click or key. Browsers block audio until the player interacts
   with the page. The bundle already resumes audio on the first
   key/mouse/touch, so this is expected browser behaviour, **not** fixed
   here (see Scope).

## Nick input

**Sign-off:** after merge, open the Pages build with the browser console open
(F12 → Console). The `register_plugin` and `quad_storage version mismatch`
errors should be gone. The AudioContext warning remains (browser rule).

## Scope

**In:**
- Declare the global `register_plugin` in `web/index.html` before
  `mq_js_bundle.js` loads, so the bundle's strict-mode assignment succeeds.
  Don't edit the vendored bundle.
- Set `web/quad-storage.js`'s plugin `version` to the number the crate
  reports, marked as a local patch in `web/README.md` and
  `THIRD_PARTY_ASSETS.md`.
- A test that fails if either fix is lost (e.g. on re-vendoring).

**Out (do not do):**
- Any "click to start" screen or other gate for autoplay. That is a
  player-facing choice for Nick.
- Bumping macroquad or quad-storage.

## Implementation steps

1. `web/index.html`: add `<script>var register_plugin;</script>` (with a
   comment) before `<script src="mq_js_bundle.js">`.
2. `web/quad-storage.js`: `version: 65536` with a comment giving the
   encoding.
3. `crates/xtask/src/web.rs` tests: read the real `web/index.html` and assert
   the declaration comes before the bundle; read `web/quad-storage.js` and
   `Cargo.lock`, compute the encoded `quad-storage-sys` version, and assert
   the JS declares it.
4. Update `web/README.md` and the `THIRD_PARTY_ASSETS.md` row.

## Acceptance criteria

- [x] Local web build served over HTTP logs neither error in the console.
- [x] The new xtask tests pass, and fail if the fixes are reverted.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: xtask tests over the real `web/` files and `Cargo.lock`.

## Completion notes

- `web/index.html` declares `var register_plugin;` before the bundle, so
  the bundle's strict-mode assignment works and all five JS plugins
  register (checked in the browser: `macroquad_audio`, `sapp_jsutils` ×2,
  `quad_net`, `quad_storage`).
- `web/quad-storage.js` declares `version: 65536`; the change is recorded as
  a local patch in `web/README.md` and `THIRD_PARTY_ASSETS.md`.
- New xtask tests `index_html_declares_register_plugin_before_the_bundle`
  and `quad_storage_js_version_matches_the_locked_crate` read the real files
  and `Cargo.lock`. Both failed with the old files and pass now.
- Checked by serving `cargo xtask web --debug-tools` locally: the console
  shows neither error, only the info line "Plugin quad_net is present in
  JS bundle, but is not used in the rust code." The live Pages site, loaded
  the same way, shows both errors.
- The autoplay warning and silent title (item 3) are unchanged: browsers
  block audio until the first click or key press. Music starts on that first
  input. Making it start with the title needs a player-facing "click to
  start" step. Nick chose "Press any key" on the title, web only: decision
  ticket 0034, built by ticket 0224.
