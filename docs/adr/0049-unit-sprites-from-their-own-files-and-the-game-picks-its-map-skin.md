# ADR-0049: Unit sprites come from their own image files, a tileset may leave terrain to the glyph skin, and the game picks its map skin from what it has

- **Status:** Accepted
- **Date:** 2026-10-03
- **Related tickets:** 0436, 0433, 0437, 0440, 0413, 0824
- **Extends:** ADR-0038 (§2: the sprite item gains a paint; §3: what a
  tileset file names, and which skin the game starts with), ADR-0040 (the
  bought map sprites and their tileset are private files)

(ADR-0048 is the backdrop, from ticket 0228, which landed the same day.)

## Context

- Nick decided the battle map is drawn with the bought Tiny Tales art
  (`docs/design/look-and-feel.md`, *Battle map: bought tiles and unit
  sprites*). Units come first (0436), terrain later (0437); until then
  the sprites stand on the glyph terrain.
- 0433's tileset file names **one image** and reads every unit picture
  from a grid in it. The bought map sprites are **one file per character
  or class**: a 48×80 sheet, 3 columns (walking frames) × 4 rows (facing
  down, left, right, up) of 16×20 frames. There are hundreds; only the
  ones the game uses are imported.
- The decided look needs two things a sprite item can't do: a **1-pixel
  outline in the side's colour** (the picture's silhouette in one colour)
  and an **acted unit grey and darker**. ADR-0038 forbids drawing a
  picture as per-pixel rectangles. Ticket 0413 needs the silhouette too,
  for its hit flash.
- The bought files are private (ADR-0040): a build without them must run
  and pass every gate, and ADR-0038 said "until then the glyph skin is the
  default". Nick has since decided the pictures are the default where
  they exist.
- The lead's look depends on the gender the player picked; portraits
  already go by `lead_m` and `lead_f` (`trpg_core::lead`).

## Decision

### 1. A unit picture is a frame of the tileset's image or of a file of its own

In a tileset's `units`, each entry (a character's, a class's, the
fallback) is either:

- `(column, row)`: 0433's cell of the tileset's own `image`, counted in
  `unit_px` steps from `units_origin_px`; or
- `(image: "units/fighter_male.png", frame: (1, 0))`: frame `(column,
  row)` of that image, counted in `unit_px` steps from its top-left.

`unit_px` is the frame size for both kinds. Validation reports an image
that isn't a PNG in the bundle, a frame outside its image, and a class or
character that doesn't exist. A `(column, row)` entry needs the tileset to
name an `image`.

**The lead's picture goes by gender.** `characters` may name `lead_m` and
`lead_f` as well as ids from `characters.ron`. For the lead the skin looks
up the picture of the player's gender first, then `lead`, then the class.

### 2. A tileset may have no terrain tiles

`image`, `tile_px` and `terrain` may be left out. A tileset without
`terrain` has only unit pictures, and its skin is the **mixed skin**:
terrain, ranges, the path and the cursor are painted by the glyph skin's
own code on the glyph skin's tiles (shared, not copied), and units by the
sprite skin's unit code. A unit's tile keeps its terrain background and
loses the terrain's glyphs under the picture.

`terrain` and `tile_px` come together (and need `image`); `tile_px`
without `terrain` is an error, since the tile size is then the glyph
skin's. With `terrain`, 0433's rule "every terrain has a tile" stands.

(Ticket 0436 wrote `terrain: None`; the field is left out instead, so the
existing files keep their plain `terrain: { … }` and RON needs no
extension.)

`MapSkin::name` is the kind of skin: `glyph`, `sprite` (a tileset with
terrain) or `sprite_units` (one without). `MapSkin::tileset_id` names the
tileset.

### 3. Units look the same under every sprite skin

`crates/ui/src/map_view/sprite/units.rs` paints them, in two passes:
every unit's outline and picture, the top row first; then every unit's HP
bar and effect arrow. Pictures are `Over` items, so a head covers the
glyphs of the tile above. The rules (where the picture stands, the 14
pixel bar, the shaving under another unit, the arrows and their timing)
are Nick's, in `look-and-feel.md`; the numbers are constants in that
file.

The scene says what a skin needs for it: `UnitView::bonus` and `penalty`
replace `has_effect`, and `MapScene::clock_ms` is the animation clock the
arrows bounce and take turns by.

### 4. A sprite item has a paint

`Sprite::paint` says how its pixels are coloured. Whichever it is, a pixel
stays as see-through as the image has it.

| Paint | Each pixel is drawn | For |
| ----- | ------------------- | --- |
| `Image` (the default) | in its own colour | everything so far |
| `Solid(colour)` | in that one colour | a unit's outline (the same frame drawn four times, one pixel up, down, left and right, under the picture); 0413's hit flash |
| `Dimmed` | `Rgb::dimmed`: 75% of the way to its own grey, then 0.75 as bright (*tunable*, in `color.rs`) | a unit that has acted, and its arrow |

A snapshot line gains ` solid=colour` or ` dimmed`. `app` draws a painted
sprite from a **recoloured copy of the image's texture**, made the first
time a sprite needs it (a white silhouette, tinted; or the dimmed
pixels). `cargo xtask frame-png` applies the paint per pixel. Both use
`Paint::apply`, so they can't drift.

### 5. The effect arrows are a generated image of our own

`assets/images/effect_marks.png` (14×7: an up arrow, then a down arrow,
each 5×5 with a 1-pixel dark edge) is written by `cargo xtask
effect-marks` from the palette's `effect_bonus`, `effect_penalty` and
`black`. The colours are baked in; a test fails if the committed file
isn't what the tool makes. The skin draws it as a sprite item.

### 6. The game picks its map skin from what it has

At start-up `Ctx::map_skin` is the sprite skin of the tileset called
`tiny_tales` (`map_view::GAME_TILESET`) if `Content::tilesets` has it,
else the glyph skin. That tileset exists only in
`assets-private/game/tilesets/`, so:

- a build with the `private-assets` feature starts with the bought art;
- every other build, and every gate, starts with the glyph skin, and its
  glyph snapshots don't change.

The debug menu's "Map skin" goes round: the glyph skin, each tileset the
content has, the glyph skin again.

### 7. The bought sprites are imported by a tool, from a table

`cargo xtask map-sprite-import` copies each sprite the game uses from
`assets-private/library/tiny-tales/characters/` to
`assets-private/game/units/<name>.png`, whole (0440 needs the walking
frames), and writes `assets-private/game/tilesets/tiny_tales.ron`. Which
sprite each class and character gets is one table in
`crates/xtask/src/map_sprite_import.rs`; `--list` prints it. Then commit
and push in `assets-private/` and run `cargo xtask private-assets --pin`
(ADR-0040).

The public stand-in is `assets/tilesets/test_units.ron` and its sheets
(`cargo xtask test-tileset`): generated 48×80 sheets shaped like the
bought ones, for tests and the debug menu.

## Consequences

- 0437 adds terrain by giving `tiny_tales.ron` an `image`, a `tile_px` and
  a `terrain` table; the skin then paints the whole map and the unit code
  doesn't change. The mixed skin stays, for a tileset with only units.
- 0440 has the walking frames already in the bundle: it picks another
  frame of the same file.
- 0413's hit flash is `Paint::Solid`.
- A tileset with a class per side (an enemy archer that looks different
  from ours) isn't possible: pictures hang off the class and the
  character, and the outline says the side. If Nick wants that, the
  format grows a table by side.
- Each recoloured texture is a second copy of its image on the graphics
  card. Map sprites are 48×80; the arrows 14×7.
- A sprite skin with terrain tiles uses the same unit look now, so 0433's
  placeholders (the bar along the top, the 3×3 mark) are gone and the test
  tileset's snapshots changed.
- The private repository's content test (ADR-0040 §5) loads the new
  tileset, so a missing sprite file or a class that was renamed fails it.
- 0824 (the Options entry "Map look") switches between `default_skin` and
  the glyph skin.

## Alternatives considered

- **Pack the bought sprites into one image for 0433's grid**: the importer
  would rewrite bought files into a sheet that has to be remade whenever a
  class is added, and 0440 would need three frames in four directions for
  each: a second layout to describe. A file per unit is how the art comes.
- **Draw the outline as rectangles around the silhouette**: needs the
  pixels in `content` or `ui`, and ADR-0038 rules it out.
- **A shader in `app` for the solid and dimmed draws**: a second render
  path to keep working on WebGL 1 and the desktop, for two effects a
  recoloured copy of a 48×80 texture gives exactly.
- **Pre-made outline and grey frames in the imported files**: four sides'
  colours times every sprite, changed bought files, and the palette could
  no longer recolour a side.
- **A cargo `cfg` that picks the skin when the feature is on**: the skin is
  chosen from the content instead, so a test can give the content that
  tileset and check the choice, and the feature stays only about which
  files are embedded.
- **A separate `MixedSkin` type**: it would hold a tileset and paint units
  exactly as `SpriteSkin` does; the only difference is who paints the
  ground, which is one branch.
- **Tint the arrows from a white image**: a sprite's paint has no
  "multiply", and the arrow has two colours (its body and its dark edge).
  Baking the palette's colours in keeps the sprite item simple.
