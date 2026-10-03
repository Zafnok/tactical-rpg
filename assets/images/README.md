# Images (`.png`)

Pictures the game draws as **sprite items** (ADR-0038): a screen puts a
`Sprite` in the frame that names an image by its path here, and `app` draws
it from a texture. No picture is ever drawn as per-pixel cells or
rectangles.

Every `*.png` under `assets/` (in any directory, the font atlas aside) is in
the image table, `Content::images` (`trpg_content::image`), under its path
relative to `assets/`, e.g. `images/test_card.png`. The all-assets test
(ADR-0005) fails on a file named `.png` that isn't a PNG, an empty image, or
one with a side over 4096 pixels.

This directory is for images that belong to no other kind of asset. A kind
with its own directory and sidecar files (portraits, 0711; combat pictures,
0413; tilesets, 0433) keeps its PNGs there.

| File | What |
| ---- | ---- |
| `test_card.png` | 16×16 test image: a one-pixel white border around four coloured quadrants (red and green above blue and yellow), so a flip, a crop and a scale each show. Used by tests and the "Sprite test" debug tool |
| `effect_marks.png` | 14×7: the arrows a sprite map skin puts on a unit under a timed effect (ADR-0049, `docs/design/look-and-feel.md`): an **up arrow** for a bonus, then a **down arrow** for a penalty, each 5×5 with a 1-pixel dark edge. Our own picture, in the palette's `effect_bonus`, `effect_penalty` and `black` |

Both are **generated**; never edit them by hand. To regenerate (the arrows
after changing their palette colours):

```bash
cargo xtask test-card
cargo xtask effect-marks
```

A test in `xtask` fails if a committed file differs from what the tool
makes.

## Rules for images

- PNG only, at most 4096×4096.
- Draw at whole-number scales: `app` samples the nearest pixel, so 1×, 2×,
  3×… stay sharp and other scales don't.
- Only images under a licence ADR-0013 allows, each listed in
  `THIRD_PARTY_ASSETS.md`. Bought art stays out of this repository
  (ADR-0032); a public placeholder stands in for it here.
