# Third-party assets

Every non-crate third-party item shipped with the game (fonts, vendored
JavaScript, images, audio, SDK files) must be listed here, per
[ADR-0013](docs/adr/0013-licensing-and-third-party-policy.md). Rust crates are
checked automatically by `cargo-deny` and listed in the generated
`THIRD_PARTY_LICENSES.html` in release packages.

Only licenses allowed by ADR-0013 are permitted. The license text must be
committed next to the item.

| Item | Source URL | Version | License | License file | Used for | Added by ticket |
| ---- | ---------- | ------- | ------- | ------------ | -------- | --------------- |
| Terminus Font (`ter-u16n`, 8×16), converted to `assets/fonts/atlas.png` | https://terminus-font.sourceforge.net/ | 4.49.1 | OFL-1.1 | [`assets/fonts/Terminus-LICENSE.txt`](assets/fonts/Terminus-LICENSE.txt) | The game's only font (every glyph on screen); source BDF in `assets-src/fonts/` | 0203 |
| `web/mq_js_bundle.js` (macroquad's JS/WASM loader) | https://github.com/not-fl3/macroquad/blob/5e9b5ca912ac65962c05c0da842a4a70eaae34b9/js/mq_js_bundle.js | 0.4.16 (commit `5e9b5ca9`; no matching git tag, see `web/README.md`) | MIT | [`web/mq_js_bundle-LICENSE-MIT.txt`](web/mq_js_bundle-LICENSE-MIT.txt) | Loads and runs the WASM binary in the browser build | 0206 |
| `web/quad-storage.js` (miniquad `localStorage` JS plugin) | https://github.com/optozorax/quad-storage/blob/3760b953aec17d65cc4ca8edfa39c38e7337ec3a/js/quad-storage.js | 0.1.3 (commit `3760b953`, see `web/README.md`) | MIT | [`web/quad-storage-LICENSE-MIT.txt`](web/quad-storage-LICENSE-MIT.txt) | Backs `trpg_ui::storage::Storage` on web | 0207 |
| `web/sapp_jsutils.js` (JS↔Rust marshalling `quad-storage.js` needs) | https://github.com/not-fl3/sapp-jsutils, shipped inside the `sapp-jsutils` crate's `js/sapp_jsutils.js` | 0.1.7 | MIT (no upstream `LICENSE` file; standard MIT text reconstructed from the crate's declared license, see `web/README.md`) | [`web/sapp_jsutils-LICENSE-MIT.txt`](web/sapp_jsutils-LICENSE-MIT.txt) | Backs `trpg_ui::storage::Storage` on web | 0207 |
