# Tilesets (`.ron` + `.png`)

What a **sprite map skin** paints the battle map with (ADR-0038): one
image, and a table from the game's ids (terrain, classes, characters) to
rectangles in it. The rules never read a tileset; only the look hangs off
these ids. Loaded and checked by `trpg_content::tileset` into
`Content::tilesets`, by id.

The glyph skin (coloured letters) stays the game's look; a sprite skin is
reachable only from the debug menu (F2 → *Map skin*) until Nick decides
otherwise.

## Format

`assets/tilesets/<id>.ron`, next to its image:

```ron
(
    id: "test",                      // the file's name, without .ron
    image: "tilesets/test.png",      // a PNG in the asset bundle, by its path under assets/
    tile_px: (24, 24),               // a map tile, across × down: in the image, and on screen in console pixels
    terrain: {                       // terrain id (terrain.ron) → (column, row) in tile_px steps from the image's top-left
        "plain": (0, 0), "road": (1, 0), "forest": (2, 0),
    },
    unit_px: (24, 24),               // a unit picture, across × down; may be taller than a tile
    units: (
        characters: {},              // character id (characters.ron) → (column, row) in unit_px steps
        classes: { "brigand": (0, 1), "rider": (1, 1) },   // class id (classes.ron) → (column, row)
        fallback: (7, 4),            // the picture of a unit with neither
    ),
    units_origin_px: (0, 48),        // the pixel where the unit grid's (0, 0) starts
)
```

- A unit's picture is its **character's**, else its **class's**, else the
  **fallback**. It is drawn at its own size, centred across its tile and
  standing on the tile's bottom edge: a picture taller than a tile reaches
  into the tile above.
- What the art doesn't show (whose side a unit is on, that it has acted,
  its HP, a timed effect, the cursor) the skin draws itself; the test
  skin's marks are placeholders until ticket 0039 decides them.

## Checks

The all-assets test (ADR-0005) reports every problem in every file, with
its name:

- the file must parse, with no unknown fields, and its `id` must be its
  name;
- `image` must be a PNG in the bundle, and every rectangle (each terrain
  tile, each unit picture, the fallback) must lie inside it;
- each side of `tile_px` and `unit_px` must be 8 to 64 px;
- **every terrain in `terrain.ron` must have a tile** (`terrain "…" has no
  tile`), and every terrain, class and character named must exist.

## Files

| File | What |
| ---- | ---- |
| `test.ron`, `test.png` | The **test tileset**: 24 × 24 tiles, so nothing can quietly assume the glyph skin's 16 × 16. Each terrain's tile is its `bg` colour with its two glyphs in its `fg` colour; each class's picture is a grey disc with the first two letters of its name in white, by class id; then a `??` fallback. Not art: generated from our own data and the font atlas |

`test.ron` and `test.png` are **generated**; never edit them by hand. After
changing the terrain, the classes, the palette or the font, regenerate them:

```bash
cargo xtask test-tileset
```

A test in `xtask` fails if the committed files differ from what the tool
makes.

## Adding a tileset

1. Put the image and its `.ron` here (bought art goes in the private assets
   repository's `game/tilesets/` instead, ADR-0040).
2. Give every terrain a tile; give classes and characters pictures where
   the art has them.
3. Run the content tests: they name anything missing or outside the image.
