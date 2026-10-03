---
id: "0236"
title: "Wide glyphs: the Japanese font in the atlas, two cells per glyph"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0042"]
nick_input: none
completed:
---

# 0236 — Wide glyphs: the Japanese font in the atlas, two cells per glyph

## Context

The font is Terminus 8×16 and a cell holds one 8×16 `char` (ADR-0016).
Japanese glyphs are 16×16. [ADR-0045 §4](../../docs/adr/0045-languages-text-by-key-and-line-ids.md):
the atlas gains 16×16 glyphs from the font Nick chose in 0042, and a wide
glyph takes two cells. This ticket makes Japanese text *drawable*; 0237
makes it wrap and fit.

## Nick input

None (the font is 0042's answer).

## Scope

**In:** the font's BDF in `assets-src/fonts/`; the atlas tool, format and
validation; `GlyphBuffer` printing; the renderer; `frame-png`; text width.

**Out (do not do):** line breaking and length limits (0237); translating
(0719); changing any Latin glyph. If 0042 chose a font that ships only as
TTF, converting it to BDF at 16 px is part of this ticket (a one-off,
documented in `assets/fonts/README.md`; the BDF is what is committed).

## Implementation steps

1. Add the font's BDF and licence text (`assets-src/fonts/`,
   `assets/fonts/<Name>-LICENSE.txt`), a row in `THIRD_PARTY_ASSETS.md`
   and a section in `assets/fonts/README.md`. If the BDF is in a JIS
   encoding, convert code points to Unicode in the tool, not by hand.
2. `crates/xtask/src/font_atlas.rs`: a wide font argument
   (`--wide <font.bdf>`). Wide glyphs taken are exactly the characters
   outside the narrow atlas that appear in `assets/lang/*/` text (and in
   `lang.ron` names), plus a fixed minimum set (hiragana, katakana, the
   Japanese punctuation block) so name entry and tests don't depend on a
   translation. They are laid out after the narrow glyphs, each using two
   atlas cells side by side.
3. `trpg_content::font::FontAtlasDef`: mark wide glyphs (a second map or
   a flag per glyph). Validation: a wide glyph's two cells fit the PNG;
   no char is both narrow and wide. The stale-atlas test passes the wide
   font too.
4. `crates/ui/src/glyph_buffer.rs`: `print` and friends advance two cells
   for a wide char. The first cell holds the char; the second holds a
   continuation marker (a reserved `char` constant, never drawn) with the
   same colours. Overwriting either half clears the other to a space. A
   wide char that would start in the last column isn't drawn. The buffer
   needs the width table: give it a `fn is_wide(char) -> bool` from the
   atlas definition held where the buffer is created, defaulting to
   "nothing is wide" so existing tests don't change.
5. One `text_width(&str) -> u16` in `ui` (cells), used by 0235's length
   checks and by every widget that centres or right-aligns text. Replace
   `chars().count()` / `len()` used for layout in `crates/ui/src`.
6. `crates/app/src/render.rs`: draw a wide glyph as one 16×16 quad over
   both cells; skip continuation cells. `cargo xtask frame-png` the same.
7. Text snapshots: a continuation cell prints as nothing, so a snapshot
   row with Japanese reads naturally; document it in the Harness.
8. A screenshot for the PR: the test pack (0233) with a few Japanese
   characters on the title screen, via `frame-png`.

## Acceptance criteria

- [ ] Printing `"あA"` fills three cells: wide, continuation, narrow (unit test).
- [ ] Overwriting half of a wide glyph leaves no orphan half (unit test).
- [ ] A frame PNG with Japanese text matches a committed reference (test, like the existing `frame-png` tests).
- [ ] The atlas contains exactly the wide characters the packs use plus the minimum set, and the stale-atlas test fails when a pack gains a new kanji (test).
- [ ] Every existing snapshot and the narrow atlas cells are unchanged.
- [ ] The font's licence is shipped and listed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: buffer printing and overwriting; `text_width`; atlas validation.
- Property: printing any mix of narrow and wide chars never leaves a continuation cell without its first half.
- Snapshot / integration: a frame PNG; the look test reads cells, as ADR-0038 allows for tests of a look.

## Completion notes

