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
