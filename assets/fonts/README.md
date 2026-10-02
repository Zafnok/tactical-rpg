# Fonts

The game draws every cell with one bitmap font (ADR-0003, ADR-0018).

## Choice: Terminus 8×16 (`ter-u16n`, medium weight)

| Candidate | Licence (checked at source) | Verdict |
| --------- | --------------------------- | ------- |
| **Terminus Font 4.49.1** | SIL OFL-1.1 (`OFL.TXT` in the release tarball) | **Chosen** |
| Spleen 8x16 | BSD-2-Clause | Not needed; Terminus already covers every required glyph |
| GNU Unifont | GPL-2+ with font exception, or OFL-1.1 | Not needed as a fallback; heavier, less even look |

Why Terminus:

- A true 8×16 cell (`FONTBOUNDINGBOX 8 16 0 -4`), matching ADR-0018.
- Covers **every** glyph in `trpg_content::font::REQUIRED_GLYPHS` on its own:
  printable ASCII, Latin-1, all box drawing, blocks, and the CP437 symbols
  (`♣ ♠ ♥ ♦ ≈ · • ˇ ▲ ▼ ◄ ► ← ↑ → ↓ ☺ ☻ ♪ ¤ †`). No fallback font needed.
- Clean, even strokes designed for long reading sessions at small sizes.
- OFL-1.1 is allowed for fonts by ADR-0013 and permits commercial bundling.

## Files

| File | What |
| ---- | ---- |
| `atlas.png` | 551 glyphs, white on transparent, 32 cells of 8×16 per row (256×288 px) |
| `atlas.ron` | `FontAtlasDef`: cell size, columns and `char → cell index` map |
| `Terminus-LICENSE.txt` | The font's licence (OFL-1.1), shipped with the game |

Both atlas files are **generated**; never edit them by hand. The sources are
in `assets-src/fonts/` (outside `assets/`, so they aren't embedded in the
game): `ter-u16n.bdf`, the font, and `pad-shapes.bdf`, our own extra glyphs
(below). To regenerate:

```bash
cargo xtask font-atlas assets-src/fonts/ter-u16n.bdf assets-src/fonts/pad-shapes.bdf assets/fonts
```

A test in `xtask` fails if the committed atlas differs from what the tool
produces. The glyph selection is `ATLAS_RANGES` in
`crates/xtask/src/font_atlas.rs`.

## Our own glyphs (`pad-shapes.bdf`)

Terminus has no `□`, `△` or `✕`, and its `○` is only 4 px wide, so the
PlayStation face buttons' shapes are glyphs we drew ourselves (ticket 0220,
ADR-0036; Nick picked the look from
[`docs/screenshots/0220-ps-glyphs.png`](../../docs/screenshots/0220-ps-glyphs.png)):

| Glyph | Code point | Used for |
| ----- | ---------- | -------- |
| `✕` | U+2715 | PlayStation cross |
| `◯` | U+25EF | PlayStation circle (U+25CB `○` stays Terminus's small one) |
| `□` | U+25A1 | PlayStation square |
| `△` | U+25B3 | PlayStation triangle |

Each is 7×7 px on the baseline, as tall as a lower-case letter. To add a
glyph of our own: add a `STARTCHAR` block to `pad-shapes.bdf` (or a new BDF
passed after the font), make sure its code point is in `ATLAS_RANGES`, and
regenerate. The tool refuses an extra file that redefines a glyph the font
already has, so Terminus's own glyphs are never changed.

## Licence notes (OFL-1.1)

- Copyright (C) 2020 Dimitar Toshkov Zhekov, Reserved Font Name
  "Terminus Font". The licence text must travel with the game; it lives here
  and is embedded with the assets.
- The atlas is a format conversion of the font with four glyphs of our own
  added (above), which makes it a Modified Version in the OFL's words. The
  OFL allows that provided the result stays under the OFL and doesn't use
  the Reserved Font Name. We never present it (or anything in the game)
  under the name "Terminus Font"; mentions here are acknowledgement only,
  as the OFL allows. `ter-u16n.bdf` itself is unmodified.
- The font is never sold on its own, only bundled with the game.
