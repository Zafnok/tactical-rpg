# ADR-0003: Glyph-grid rendering on macroquad

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

The game is ASCII art in the Dwarf Fortress / Rogue tradition: a grid of
characters, each with a foreground and background colour. Nick wants good
clarity of detail and colour. It must ship as a Windows exe on itch/Steam and
ideally run in a browser.

## Decision

1. **We render our own virtual console in a window; we do not run in a real
   terminal.** The game draws a fixed-size grid of cells (glyph + fg colour +
   bg colour) using a bundled bitmap font.
2. **The window/graphics layer is [macroquad](https://github.com/not-fl3/macroquad).**
   It is small, has no system dependencies on Windows, builds to WASM with one
   command, and is enough for drawing textured quads from a font atlas.
3. **macroquad is confined to the `app` crate.** Every other crate draws into a
   plain-Rust `GlyphBuffer` (see ADR-0004). The `app` crate's only rendering
   job is to blit that buffer to the screen.

## Consequences

- We control the exact font, cell size and colours on every machine. A player's
  terminal settings can't break the look.
- A Windows `.exe` double-clicks and runs; there is no terminal window.
- Screens can be snapshot-tested headlessly because they draw to a `GlyphBuffer`,
  not to the GPU.
- If macroquad ever becomes a problem (maintenance, a Steam overlay issue, etc.),
  swapping it means rewriting one thin crate, not the game.
- We must pick and license a font (ticket 0203) and write a small blitter.

## Alternatives considered

- **Real terminal (ratatui + crossterm)** — colours and glyph support depend on
  the player's terminal; Steam users expect a window; the web build would need a
  separate terminal emulator. Rejected.
- **bracket-lib / RLTK** — built for exactly this, but effectively unmaintained
  since 2023. Rejected to avoid inheriting a dead dependency.
- **Bevy** — excellent engine, but heavy compile times and large WASM bundles
  for what is a 2D character grid. Its ECS is also overkill for a turn-based
  game whose rules we want as plain, testable functions.
- **Raw winit + wgpu** — maximum control, but hundreds of lines of plumbing
  (surfaces, pipelines, web canvas) that macroquad already solves.
