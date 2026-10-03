---
id: "0441"
title: "Glyph look: the map HP bar is 14 pixels wide"
type: tuning
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-03
---

# 0441 — Glyph look: the map HP bar is 14 pixels wide

## Context

Nick decided on 2026-10-02 (ticket 0039, `docs/design/look-and-feel.md`,
*Units on the map (the glyph look)* and *Battle map: bought tiles and unit
sprites*) that a unit's HP bar on the battle map is **14 pixels wide, one
pixel in from each side of its tile**, in both looks: "can we also adjust
hp bar to be 1 less pixel left and right? this would be always active" …
"I think glyph look should also have 14px hp bar". With the full 16
pixels, the bars of units standing side by side run together.

Sprite units get the 14-pixel bar in 0436. This ticket does the glyph
look, which draws it today in `crates/ui/src/screens/battle/units.rs`
(`HP_BAR_W`, `hp_bar`, the two `Overlay`s at the end of the unit-drawing
function: the filled part, then the dark rest).

## Nick input

None.

## Scope

**In:** the map HP bar under a glyph unit starts 1 pixel in from the
tile's left edge and is 14 pixels wide; its fill is worked out over those
14 pixels.

**Out (do not do):**
- The bar's height (2 px), colours and thresholds.
- The HP bars in the side panel, the forecast and the unit info screen
  (`HP_BAR_CELLS`): they are text cells, not this bar.
- The cursor, whose corner marks stop above the bar (`HP_BAR_H` in
  `cursor.rs`): the bar's height doesn't change, so neither do they.
- Sprite units (0436).
- 0432 (the map scene and map skins): **either order works.** If 0432 is
  done, the bar is in the glyph skin (`crates/ui/src/map_view/`) instead
  of `units.rs`; make the same change there. If 0436 is done too, share
  its bar rectangle and fill maths instead of having two copies.

## Implementation steps

1. Set `HP_BAR_W` to 14 and add the 1-pixel left inset where the two
   overlays are built (`px + 1`), keeping the fill rule: full HP fills all
   14, and any HP above 0 shows at least 1 pixel if that is today's rule
   (read `hp_bar` and keep its rounding).
2. Update the unit tests of `hp_bar` and of the overlays' rectangles.
3. Review the changed snapshots (`cargo insta review`): only HP-bar
   overlay lines change, each 1 pixel further right and 2 narrower in
   total.
4. Render the Quick Battle (`cargo xtask frame-png` if 0232 is done,
   otherwise the web build) and **look at it**: two units side by side
   have a 2-pixel gap between their bars.

## Acceptance criteria

- [x] Test: a full-HP unit's bar is the rectangle `(tile x + 1, tile
      bottom − 2, 14, 2)`; at half HP the filled part is 7 wide and the
      dark part 7.
- [x] Changed snapshots differ only in HP-bar overlay lines.
- [x] The frame was looked at (step 4).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the bar's rectangle and fill at full, half, 1 HP and 0 HP.
- Snapshot / integration: the existing battle snapshots, reviewed.

## Completion notes

- 0432 was already done, so the change is in the glyph skin
  (`crates/ui/src/map_view/glyph/units.rs`): `HP_BAR_W` is 14 and a new
  `HP_BAR_INSET` (1 px) moves both overlays right. 0436 is not done, so
  there is nothing to share yet; the sprite skin still draws its own
  16-pixel bar until 0436.
- The fill keeps its rule (round to the nearest pixel, half up) over 14
  pixels: full HP fills 14, half fills 7.
- 43 snapshot files changed. Checked with a script, not by eye: every
  changed line is an HP-bar overlay (1 pixel right, 2 narrower, or the
  matching change for a part-filled or clipped bar), plus stale
  `source:` / `expression:` header lines insta rewrote.
- Looked at the Quick Battle with `cargo xtask frame-png`: the Mage and
  the Knight stand side by side and their bars have a gap between them.
- No game rule was decided here.
- For Nick: with 14 pixels instead of 16, a unit needs a little more HP
  to show its first pixel. A unit at 1 HP of 30 showed 1 pixel before
  and now shows an all-dark bar (a unit at 1 of 40 already did). This is
  the existing rounding, which the ticket says to keep.
