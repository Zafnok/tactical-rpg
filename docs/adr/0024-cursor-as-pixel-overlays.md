# ADR-0024: Draw the battle cursor as pixel overlays, with selectable styles

- **Status:** Accepted
- **Date:** 2026-09-27
- **Related tickets:** 0415, 0402, 0403, 0805
- **Supersedes:** the "Cursor and path" browsing rule of ADR-0018 (the rest of
  ADR-0018 stands)

## Context

ADR-0018 drew the browsing cursor as `[` and `]` glyphs in the cells either
side of the tile. Those cells belong to the neighbouring tiles, and a unit's
two-letter label fills both cells of its tile, so a cursor next to a unit hid
one of its letters (Nick's playtest, ticket 0415). The Terminus 8×16 font
leaves fixed gaps: letters are inked only in pixel columns 0–6 of a cell
(column 0 only for `M T W Y m w`; column 7 never) and rows 2–11 (descenders of
`g j p q y Q` reach rows 12–14). The HP bar takes rows 14–15 of the tile.

Nick chose corner marks (3 px arms), with 4 px corners and a tile glow as
accessibility options (`docs/design/look-and-feel.md`).

## Decision

- `trpg_ui::screens::battle::cursor::CursorStyle { Corners, LargeCorners,
  TileGlow }`, default `Corners`. The current style lives in
  `Ctx::cursor_style` until the Options menu (0805) moves it into `Settings`.
- **Corner marks** are 1 px `Layer::Over` overlays in `cursor` colour × pulse
  brightness. In tile-relative pixels (tile = 16×16, `x` = 0 at its left
  edge): vertical arms in columns `−1` (the left neighbour's never-inked
  column 7) and `15` (the tile's own column 7), running down from row 0 and up
  from row 13; horizontal arms on rows 0 and 13 (row 13 is just above the HP
  bar), running inward from those columns. Arm length is 3 px (`Corners`) or
  4 px (`LargeCorners`). So inside the letters' rows 2–11 the cursor only uses
  columns no letter uses, and it never changes a cell's glyph or colours.
- **Tile glow** blends the tile's two cells' backgrounds towards `cursor` by
  `GLOW_MAX × brightness` (`GLOW_MAX = 0.3`, tunable), and draws no overlays.
- Everything is clipped to `MAP_VIEW`.
- A property test (`cursor::tests::never_covers_letters`) holds this for every
  style, pulse phase and tile position.

## Consequences

- A neighbour's initials are never hidden by the cursor, and terrain glyphs
  such as the fort's `[]` no longer get overwritten either.
- The mark relies on the font's gaps. A new font (or a change to the tile
  layout) must keep an always-blank pixel column at the right of each cell and
  blank rows 0–1 and 13, or this rule must be revisited.
- Row 13 can touch the descender tips of `g q y` in a label's second letter
  (one pixel). The HP bar already covers their rows 14–15, so labels already
  avoid relying on descenders.
- Tests find the cursor from its overlays, not from glyphs.
- The selected-unit arrows `►` `◄` (ADR-0018, ticket 0403) still use the
  neighbouring cells and would hide a neighbour's letter. That's an open design
  question for 0403.

## Alternatives considered

- **Thin brackets** (1 px `[` `]` in the same gap columns): Nick preferred the
  corner marks.
- **Drawing the brackets under the unit's glyphs** (z-ordering): Nick expected
  it to clutter, and the letter and bracket would merge.
- **Custom narrow bracket glyphs in the font atlas**: a cell holds one glyph,
  so a narrow bracket in a neighbour's cell would still replace its letter.
