---
id: "0203"
title: Choose a licensed bitmap font, build its atlas, blit the GlyphBuffer, glyph sampler
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0202"]
nick_input: none
completed:
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
   `♣ ♠ ♥ ♦ ≈ · • ˇ ▲ ▼ ◄ ► ← ↑ → ↓ ☺ ☻ ♪ ¤ †`, plus Latin-1 letters; licence allowing commercial
   redistribution (OFL, MIT, BSD, CC0, CC-BY; CC-BY-SA acceptable for an unmodified font
   file with attribution). Candidates to compare (verify licences at source):
   **Spleen 8x16** (BSD-2), **Terminus** (OFL), **GNU Unifont** (OFL; very wide coverage),
   **Px437 IBM VGA 8x16** from the Ultimate Oldschool PC Font Pack (CC BY-SA 4.0).
   Pick one (or a primary + Unifont fallback for missing glyphs). Record the
   choice and reasons in `assets/fonts/README.md`; put the licence text in
   `assets/fonts/<FONT>-LICENSE.txt`.
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

- [ ] Font licence permits commercial redistribution; licence file and README in `assets/fonts/`.
- [ ] All `REQUIRED_GLYPHS` present in the atlas (content validation test).
- [ ] Running the app shows the crisp (non-blurry) sampler, integer-scaled and centred at several window sizes.
- [ ] `layout()` unit-tested for exact-fit, oversize and undersize windows (scale never 0).
- [ ] Sampler snapshot committed. Screenshot in the PR.

## Tests required

- Unit: `layout()`, atlas-rect lookup, `FontAtlasDef` validation.
- Snapshot: sampler.
- Content validation test extended with the atlas.

## Completion notes

