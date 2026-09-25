---
id: "0202"
title: GlyphBuffer virtual console, colours, drawing primitives, snapshot format
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0201"]
nick_input: none
completed: 2026-09-25
---

# 0202 — GlyphBuffer virtual console

## Context

Every screen draws into a `GlyphBuffer` — a grid of cells with glyph + fg + bg
colour — which the `app` crate blits to the window
([ADR-0003](../../docs/adr/0003-rendering-glyph-grid-macroquad.md),
[ADR-0004](../../docs/adr/0004-crate-architecture.md),
[ADR-0012](../../docs/adr/0012-visual-style.md)). It is also what snapshot
tests capture ([ADR-0007](../../docs/adr/0007-testing-strategy.md)), so its
text dump format matters.

## Nick input

None.

## Scope

**In:** `trpg-ui` modules `color`, `glyph_buffer`, `snapshot`; console-size
constants; `insta` set up.

**Out:** fonts and blitting (0203); screens (0205); map drawing (0401).

## Implementation steps

1. `crates/ui/src/color.rs`:
   - `#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)] pub struct Rgb { r: u8, g: u8, b: u8 }`.
   - `Rgb::lerp(self, other, t: f32) -> Rgb` (t clamped 0..=1), `Rgb::scale(self, factor: f32)` for dimming.
   - `pub struct Palette` built from `trpg_content::PaletteDef`;
     `palette.get(name) -> Rgb`. Unknown names must be impossible at render
     time: expose the required UI colours as **typed accessors or an enum**
     (`UiColor::Player` etc.) whose names are exactly `REQUIRED_COLORS`
     (checked in a test), and a fallible `lookup(name) -> Option<Rgb>` for data-driven names
     (terrain) that are validated at load time by later tickets.
2. `crates/ui/src/console.rs`: `pub const CONSOLE_W: u16 = 100; CONSOLE_H: u16 = 32; CELL_W_PX: u16 = 8; CELL_H_PX: u16 = 16;`
3. `crates/ui/src/glyph_buffer.rs`:
   - `Cell { glyph: char, fg: Rgb, bg: Rgb }`; `GlyphBuffer { width, height, cells: Vec<Cell> }`.
   - `new(w, h, fill: Cell)`, `width()`, `height()`, `get(x, y) -> Option<&Cell>`,
     `set(x, y, cell)` (out of bounds → silently ignored),
     `print(x, y, text, fg, bg) -> u16` (returns cells written; clipped; one
     `char` per cell), `print_fg(x, y, text, fg)` (keeps existing bg),
     `fill_rect(rect, cell)`, `draw_box(rect, BoxStyle::{Single, Double}, fg, bg)`,
     `blend_bg(rect, color, t)` (for range overlays), `dim(rect, factor)`
     (inactive portraits), `blit(&other, dest_x, dest_y)` (clipped).
   - `Rect { x, y, w, h }` with `contains`, `intersect`.
   - Use `i32` for positions in drawing APIs so negative offsets clip correctly.
4. `crates/ui/src/snapshot.rs`: `GlyphBuffer::to_snapshot(&self, palette: &Palette) -> String`:
   ```
   <glyph rows, exactly width chars each, trailing spaces kept>
   --- colours ---
   <rows of single-char keys, one per cell>
   --- legend ---
   a = fg:text bg:panel_bg
   b = fg:player bg:move_range
   ```
   Keys are assigned in first-seen order (`a-z`, `A-Z`, `0-9`); colours are
   reverse-looked-up to palette names where exact, else `#rrggbb`. Deterministic.
5. Add `insta` as a dev-dependency (workspace). One sample snapshot test drawing a
   double box with a title and a few coloured words.

## Acceptance criteria

- [x] All drawing primitives clip safely (no panic for any coordinates).
- [x] Snapshot format as specified; deterministic across runs and OSes.
- [x] Test asserting `UiColor` names == `REQUIRED_COLORS` from 0201.
- [x] No macroquad dependency in `trpg-ui`.

## Tests required

- Unit: each primitive incl. edge/corner clipping, `lerp` endpoints, box corners.
- Property: for random rects/strings/positions, `print`/`fill_rect`/`blit` never
  panic and never modify cells outside the target rect ∩ buffer.
- Snapshot: sample box (insta).

## Completion notes

- Added `trpg-ui` modules `color` (`Rgb` with `lerp`/`scale`/`to_hex`,
  `UiColor` enum, `Palette`), `console` (size constants), `glyph_buffer`
  (`Cell`, `Rect`, `BoxStyle`, `GlyphBuffer` with every primitive listed) and
  `snapshot` (`GlyphBuffer::to_snapshot`). `insta` is a workspace
  dev-dependency (Apache-2.0; `cargo deny` passes).
- `Palette::new(&PaletteDef)` is fallible (returns the missing `UiColor`
  names) because `PaletteDef::default()` is empty; a palette loaded by
  `trpg_content` always succeeds. `palette.get(UiColor)` is infallible;
  `lookup(name)` is the `Option` path for data-driven names. `name_of(rgb)`
  does the snapshot reverse lookup: `UiColor` names win ties, then
  alphabetical.
- `Rect` uses `i32` for position *and* size; a non-positive `w`/`h` is empty.
  `intersect` returns `Option<Rect>`.
- `draw_box` draws only the border (the interior is left alone; fill first
  if needed). When clipped, corners stay at the real rectangle's corners.
- Snapshot details beyond the spec: control characters in glyphs print as
  `U+FFFD` so rows keep their width; past 62 colour pairs keys continue with
  CJK ideographs (then `?`). `.editorconfig` keeps trailing spaces in
  `*.snap` files.
- Deviation: none of substance. No follow-up tickets.

