---
id: "0232"
title: "cargo xtask frame-png: render a scripted frame to a PNG without a window"
type: infra
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0231"]
nick_input: none
completed: 2026-10-02
---

# 0232 — Render a frame to a PNG without a window

## Context

Claude checks the game three ways: text snapshots, the scripted `Harness`
(ADR-0007), and the web build in the browser pane. Text snapshots show a
sprite as one line (`sprite 8,8 240x240 portraits/…`), which says where a
picture is but not what the frame looks like. Pictures for Nick (mockups,
sign-off screenshots; `ascii-art` skill, *Mockups for Nick*) are made by
throwaway tools outside the repo that only know glyphs, rewritten each
time.

ADR-0038 (rule 4) asks for one tool in the repo that turns any Harness
frame, sprites included, into a PNG. It is how a session looks at a new
skin (0433), a bought portrait in the dialogue screen (0711) or a combat
scene (0413) without a window, and it replaces the throwaway mockup tools.

## Nick input

None.

## Scope

**In:**
- A software renderer for a `GlyphBuffer` (cells, rectangles, sprites) in
  `xtask`, matching `app`'s `Renderer::draw` pixel for pixel at whole
  scales.
- `cargo xtask frame-png`: start the game in the Harness, press scripted
  keys, write the frame.
- Updating the skills that tell sessions how to make pictures.

**Out (do not do):**
- Pixel-comparison tests in CI (ADR-0007 rejected them; snapshots stay
  text). This tool is for looking, not for gating.
- Animated output (GIFs, frame strips). A follow-up if 0413 needs one.
- Any change to `app`'s renderer.

## Implementation steps

1. `crates/xtask/src/frame_png.rs`:
   `render(buf: &GlyphBuffer, atlas: &FontAtlasDef, atlas_png: &[u8],
   images: &ImageTable, scale: u32) -> RgbaImage` (a plain
   `Vec<u8>` + size is enough; `xtask` already depends on `png`). Same
   order as `app`: clear colour, cell backgrounds, `Under` items in order,
   glyphs tinted with fg, `Over` items in order. Sprites: nearest
   sampling, `flip_x`, `opacity` as alpha over what is beneath, `clip`.
   Decode each image once per run. A missing glyph or image draws magenta,
   as in `app`.
2. `xtask` gains a dependency on `trpg-ui` with the `harness` feature.
   Check `cargo deny` and the licence gate still pass.
3. **Command:**
   `cargo xtask frame-png <out.png> [--keys "f j j f"] [--pad "South"]
   [--wait 0.5] [--layout right|left] [--scale 2] [--web]`, steps applied
   in the order given (allow `--keys`, `--pad` and `--wait` to repeat).
   Starts as `Harness::with_layout` does (`--web`: `on_web_with_layout`),
   then writes `h.game().buffer()`. Print the top screen's name and the
   output path. The key names are the Harness's (`Chord::parse`), which
   are test-script input, not game code, so rule 7 (never hard-code a key)
   isn't broken; say so in `--help`.
4. Built with the `private-assets` feature (ADR-0040) the picture shows
   the bought art, so **never commit an output PNG made with it**. Print a
   warning when the feature is on.
5. Update the `ascii-art` skill (*Mockups for Nick*: use this command
   instead of a throwaway tool), the `run` notes in `crates/ui/README.md`,
   and `docs/adr/0038-graphics-are-a-skin.md` only if the command's name
   changed.

## Acceptance criteria

- [x] `cargo xtask frame-png target/title.png` writes an 800×512 × scale
      PNG of the title screen; `--keys` reaches Quick Battle and writes the
      battle map.
- [x] Unit test `matches_a_hand_checked_frame`: a 3×2-cell buffer with a
      glyph, an `Under` rectangle, an `Over` rectangle and a flipped,
      half-opacity, clipped sprite of `images/test_card.png` renders to the
      exact pixels listed in the test (at scale 1 and 2).
- [x] The same frame drawn by `app` (the "Sprite test" debug tool from
      0231, in the web build) and by this tool look the same; Completion
      notes say what was compared.
- [x] Same arguments twice give byte-identical files.
- [x] The `ascii-art` skill points at the command.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the renderer (cells, each layer, sprite flip/opacity/clip, missing
  glyph and image), argument parsing.
- Property: none.
- Snapshot / integration: the command run on the title screen in a test
  (output size and a few known pixels).

## Completion notes

- `crates/xtask/src/frame_png.rs`: `Painter`, a software copy of `app`'s
  `Renderer::draw` (clear colour, cell backgrounds, `Under` items, glyphs
  tinted with fg, `Over` items; nearest sampling, `flip_x`, opacity as
  alpha, `clip`; magenta for a missing glyph or image), and the
  `cargo xtask frame-png` command with `--keys`/`--pad`/`--wait` steps in
  order, `--layout`, `--scale` (1–8, default 2) and `--web`. It prints the
  top screen and the path, and a warning when built with
  `--features private-assets` (a new `xtask` feature forwarding to
  `trpg-content`). Key and button names are checked before the run, so a
  typo is an error, not a Harness panic.
- `xtask` now depends on `trpg-ui` (`harness` feature); `cargo deny` and
  `cargo machete` pass.
- **Deviation:** step 1's free function `render(buf, atlas, atlas_png,
  images, scale)` became `Painter::new(atlas, atlas_png, images, read,
  clear)` + `painter.render(buf, scale)`, so each image is decoded once per
  run and tests can leave an image out. `clear` (the palette's `black`) is
  an argument because `app` skips cells whose bg equals it.
- `decode_png` (from `font_atlas`) is no longer test-only and also strips
  16-bit channels, for 16-bit PNGs.
- **Compared with `app`:** the web build (`--release --debug-tools`), "Sprite
  test" debug tool, read back from the WebGL canvas at scale 2
  (1792×1344 canvas, console at (96, 160)), against
  `frame-png --keys "F2 Up Up Up f" --scale 2`. Every pixel was
  byte-identical except inside the "half opacity" sprite. There the canvas
  holds premultiplied colour with alpha 191: macroquad blends the alpha
  channel too. Shown over the page's black background, those pixels are
  the tool's to within 0.4 of a level, so on screen the two match.
- Also checked by eye: the title at 2× (1600×1024) and the Quick Battle map
  (`--keys "Down f" --wait 1.5`).
- The ascii-art skill (*Mockups for Nick*) and `crates/ui/README.md`
  (*Seeing a frame*) point at the command. ADR-0038 already names
  `cargo xtask frame-png`, so it is unchanged.
- No gameplay rules decided; no follow-up tickets.
