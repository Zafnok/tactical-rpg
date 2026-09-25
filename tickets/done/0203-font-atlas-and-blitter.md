---
id: "0203"
title: Choose a licensed bitmap font, build its atlas, blit the GlyphBuffer, glyph sampler
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0202"]
nick_input: none
completed: 2026-09-25
---

# 0203 — Font, atlas, blitter, glyph sampler

## Context

The `app` crate draws the `GlyphBuffer` using a bundled bitmap font
([ADR-0003](../../docs/adr/0003-rendering-glyph-grid-macroquad.md),
[ADR-0012](../../docs/adr/0012-visual-style.md)). The sampler screen produced
here is what Nick sees in the look-and-feel sign-off (0011).

## Nick input

None (Nick judges the look in 0011).

## Scope

**In:** font selection + licence, atlas generation (`cargo xtask font-atlas`),
atlas + glyph map in `assets/fonts/`, blitter in `app` with integer scaling and
letterboxing, a pure `ui::debug::glyph_sampler()` drawing function, the app
showing the sampler.

**Out:** screens/state machine (0205 will move the sampler behind a debug key).

## Implementation steps

1. **Choose the font.** Requirements: 8×16 cells; covers printable ASCII,
   box drawing (`─│┌┐└┘├┤┬┴┼═║╔╗╚╝╠╣╦╩╬`), blocks (`░▒▓█▀▄▌▐`), and
   `♣ ♠ ♥ ♦ ≈ · • ˇ ▲ ▼ ◄ ► ← ↑ → ↓ ☺ ☻ ♪ ¤ †`, plus Latin-1 letters; licence
   **allowed by [ADR-0013](../../docs/adr/0013-licensing-and-third-party-policy.md)**
   (OFL-1.1, MIT, BSD, CC0; *not* CC-BY-SA, GPL-only or "free for personal use").
   Candidates to compare (verify licences at the source, not from memory):
   **Spleen 8x16** (BSD-2), **Terminus** (OFL), **GNU Unifont** (dual-licensed;
   take the OFL option; very wide coverage). Avoid fonts whose only licence is
   CC-BY-SA (e.g. the Ultimate Oldschool PC Font Pack).
   Pick one (or a primary + Unifont fallback for missing glyphs). Record the
   choice and reasons in `assets/fonts/README.md`; put the licence text in
   `assets/fonts/<FONT>-LICENSE.txt`; add a row to `THIRD_PARTY_ASSETS.md`.
2. **Atlas tool:** `cargo xtask font-atlas <font.bdf|.psf> <out-dir>` parses the
   bitmap font (use a small crate such as `bdf-parser`, or hand-parse PSF2) and
   writes `assets/fonts/atlas.png` (white glyphs on transparent, grid of 8×16
   cells, e.g. 32 per row) plus `assets/fonts/atlas.ron` (map `char → index`).
   Commit the generated files so the game build doesn't need the tool.
3. **Loader** in `trpg-content`: `FontAtlasDef { cell_w, cell_h, columns, glyphs: BTreeMap<char, u32> }`
   loaded from `atlas.ron`; `bundle::bytes("fonts/atlas.png")` for the PNG
   (extend bundle for binary files). Validate: every char in a `REQUIRED_GLYPHS`
   const (the list in step 1) is present.
4. **Blitter** in `crates/app/src/render.rs`:
   - Load PNG with `Texture2D::from_file_with_format`, `FilterMode::Nearest`.
   - `scale = max(1, floor(min(screen_w / (CONSOLE_W*8), screen_h / (CONSOLE_H*16))))`;
     centre with black letterbox. Recompute every frame (window resizes).
   - For each cell: `draw_rectangle` bg (skip if bg == black clear colour), then
     `draw_texture_ex` of the glyph's atlas rect tinted fg. Missing glyph → `?`
     in magenta, logged once.
   - Pure helper `layout(screen_w, screen_h) -> (scale, offset_x, offset_y)` lives
     in `trpg-ui` (no macroquad) so it can be unit-tested.
5. **Sampler** `trpg_ui::debug::glyph_sampler(palette, glyph_list) -> GlyphBuffer`:
   top half shows every atlas glyph in a grid; bottom half shows every palette
   colour as a swatch (`██` in fg, then the name), plus a line of sample text
   and a sample box. Snapshot-test it.
6. `app` main loop: build the sampler buffer once and blit it every frame.
7. Take a screenshot of the running app for the PR description (and for 0011).
8. Verify on WASM: `cargo build -p trpg-app --target wasm32-unknown-unknown` (full
   browser run is 0206).

## Acceptance criteria

- [x] Font licence permits commercial redistribution; licence file and README in `assets/fonts/`.
- [x] All `REQUIRED_GLYPHS` present in the atlas (content validation test).
- [x] Running the app shows the crisp (non-blurry) sampler, integer-scaled and centred at several window sizes.
- [x] `layout()` unit-tested for exact-fit, oversize and undersize windows (scale never 0).
- [x] Sampler snapshot committed. Screenshot in the PR.

## Tests required

- Unit: `layout()`, atlas-rect lookup, `FontAtlasDef` validation.
- Snapshot: sampler.
- Content validation test extended with the atlas.

## Completion notes

- **Font: Terminus 8×16 (`ter-u16n`, OFL-1.1)**, licence checked in the
  4.49.1 release tarball. It covers every required glyph on its own, so no
  Unifont fallback was needed. The reasons are in `assets/fonts/README.md`,
  the licence is in `assets/fonts/Terminus-LICENSE.txt`, and it's listed in
  `THIRD_PARTY_ASSETS.md`. The source BDF lives in a new `assets-src/fonts/`,
  kept outside `assets/` so its 180 KB isn't embedded in the game.
- **Atlas:** `cargo xtask font-atlas` (hand-written BDF parser, `png` crate)
  writes 547 glyphs (ASCII, Latin-1, Greek, punctuation, arrows, maths, box
  drawing, blocks, shapes, symbols) as a 256×288 PNG plus `atlas.ron`. An
  xtask test fails if the committed atlas is stale.
- **Loader:** `trpg_content::font` has `FontAtlasDef`, `REQUIRED_GLYPHS`,
  `glyph_rect` and validation (missing glyphs, duplicate indices, PNG size read
  from the header). `bundle::bytes` was added, and `Content` gained `font`.
- **Blitter:** `crates/app/src/render.rs`; `layout()` is in
  `trpg_ui::console`; the sampler is `trpg_ui::debug::glyph_sampler`.
- **Deviations:**
  - The window is now `high_dpi`, and `layout()` gets the *physical*
    framebuffer size. Without this, Windows display scaling (175% on this
    machine) stretched the console by a non-integer factor and blurred it.
  - The default window is 1640×1064, a little more than exactly 2× (the
    framebuffer can come out a pixel short, which would drop to 1×).
  - `·` and `¤` appear in `REQUIRED_GLYPHS` once, under Latin-1.
  - I added `■` because existing UI code (the HP bar) uses it.
  - The ticket asked for a `REQUIRED_GLYPHS` const; it is a `&str`.
  - Recorded in ADR-0016 (atlas format, blitting, DPI rule).
- **Verified by hand:** crisp at 2× (default window and 2476×1236,
  letterboxed and centred) and at 1× cropped evenly (676×386). WASM build
  passes; the browser run is 0206. Screenshot:
  `docs/screenshots/0203-glyph-sampler.png`.
- **For Nick (0011):** the app opens on the glyph sampler: every glyph, every
  palette colour, sample text and two sample panels.
