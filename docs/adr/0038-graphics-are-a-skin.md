# ADR-0038: Graphics are a skin: screens say what to show, a skin says how it looks

- **Status:** Accepted
- **Date:** 2026-10-01
- **Related tickets:** 0229, 0230, 0432, 0433, 0434, 0711, 0413, 1006, 0228,
  0817, 0813, 0505, 0508
- **Extends:** ADR-0003 (the frame gains images), ADR-0007 (what snapshots
  show), ADR-0018 (the map's look becomes one skin among several)

(ADR-0036 and ADR-0037 are taken by pull requests open on 2026-10-01.)

## Context

Nick wants the graphics to be replaceable: swapping the battle map's glyphs
for sprites from image files must be a small job, not an engine rewrite
(2026-10-01). An audit on that date found:

**Already replaceable.**

- The rules (`core`) hold nothing about the look, and state changes only
  through `Command` → `Event`s (ADR-0004). The one display-flavoured field,
  `Unit::map_label`, is a two-letter name, not drawing.
- The playtest bots choose from `legal_commands` and call
  `BattleState::apply` (ADR-0033). They never see a screen. Recorded human
  play (0508) is a command list.
- macroquad is only in `app` (ADR-0003).

**Not replaceable.**

1. **A frame can't hold a picture.** A `GlyphBuffer` is cells plus
   solid-colour rectangles (`Overlay`). Ticket 0711 planned to draw a PNG as
   one rectangle per run of same-coloured pixels: over a thousand per face,
   more for 0413's battle pictures, each a line in every snapshot.
2. **The battle map's look is written into the battle screen.**
   `BattleScreen::draw_terrain`, `draw_ranges`, `draw_units` and
   `draw_cursor_and_path` write glyph cells directly, and "a tile is two 8×16
   cells" is spread over constants (`TILE_W_CELLS`, `VIEW_TILES_W/H`,
   `path::TILE_PX`, `units::HP_BAR_W`) and the cursor's reliance on the
   font's blank pixel columns (ADR-0024).
3. **Tests read the look to learn the state.** Battle tests find a unit by
   the letters in a cell, a range by a cell's background colour, the cursor
   by its overlay rectangles.
4. **There's no way to see a frame without a window.** Mockups and sign-off
   pictures come from throwaway tools that only know glyphs.

## Decision

### 1. Four layers

| Layer | Crate | Knows |
| ----- | ----- | ----- |
| Rules | `core` | Nothing about the look. No glyphs, colours, image names or pixel sizes. (`map_label` stays: it's a short name.) |
| What to show | `ui` screens | The *scene*: which terrain is on each visible tile, which unit and its state, which tiles are in range, where the cursor and path are. Plain data. |
| How it looks | `ui` skins + `content` data | Turns a scene into frame items. Glyphs and palette names, or tiles and sprites from image files. |
| Drawing | `app` | Draws frame items. Knows nothing about the game. |

### 2. The frame holds three kinds of item

`GlyphBuffer` keeps its name and its cells and rectangles, and gains
**sprite items** (0229): a picture from an asset file, named by its path in
the asset bundle, with a source rectangle, a destination rectangle in console
pixels, a layer, a left-right flip and an opacity. `app` draws it from a
texture. A sprite is one line in a snapshot, so tests stay readable text.

No picture is ever drawn as per-pixel rectangles or cells again. The old
32×32 text portraits are the last of that kind and go when 0711 lands.

### 3. The battle map is painted by a map skin

The battle screen builds a `MapScene` each frame and a `MapSkin` paints it
(0432). Anything else that shows a battle map (the cinematic's map shots,
0817) uses the same skin.

- **The glyph skin** is today's look (ADR-0018, ADR-0024, ADR-0029), moved,
  not changed. It stays in the game for good, whatever ships as the default.
- **A sprite skin** (0433) reads a tileset file: one image plus a table from
  terrain ids, class ids and character ids to rectangles in it.
- **Tile size belongs to the skin.** The camera and viewport ask the skin how
  many tiles fit; nothing outside a skin assumes 16×16 pixels or two cells.
- A new thing on the map (a village, a spell's terrain flash, a building
  tile) is added to `MapScene` and painted by each skin. It is not drawn
  straight into the buffer from the battle screen.

Menus, panels, text boxes and help bars stay glyph text in every skin. A
picture inside one (a portrait, a combat picture) is a sprite item.

Which skin the game ships with, and whether players may choose, is look and
feel: Nick decides it (`ask-nick`), per ticket 1006 or a later `00xx`
ticket. Until then the glyph skin is the default and other skins are
reachable only from debug tools.

### 4. Tests

- **Behaviour is asserted on the scene and the rules, not on the look.**
  The `Harness` gives the top battle's `MapScene` and a text dump of it
  (0432). "The unit moved", "these tiles are in range", "the cursor is
  here" are scene or `BattleState` assertions. Existing tests that read
  cells for this are moved in 0434.
- **The look is asserted per skin**, with frame snapshots (ADR-0007 layer
  3). Glyph-skin snapshots stay as they are.
- **A skin never changes the game.** It may change the frame and how many
  tiles fit on screen; never the battle state, the events, the sounds or
  the tile the cursor is on. A test in 0433 plays one script under each
  skin and compares them.
- **Looking at a frame needs no window:** `cargo xtask frame-png` (0230)
  renders any Harness frame, sprites included, to a PNG. Claude uses it to
  check a look and to make pictures for Nick. The browser pane on the web
  build remains the end-to-end check of `app`.

### 5. Bots and recordings never touch the look

Restating ADR-0033 as a rule that tickets are checked against:

- `trpg-bots` depends on `trpg-core` (and `trpg-content` for loading), never
  on `trpg-ui` or `trpg-app`.
- A play record (0508) holds commands, seeds and results: no cell
  coordinates, no skin name, nothing a change of graphics could break.
- Battle files, map files and anything the autobalancer (0509) or the
  skirmish generator (0510) writes name terrain, classes and characters by
  id. Looks hang off those ids in the skins' own data.

### 6. Where look data lives

- Glyph skin: `terrain.ron`'s `glyphs`/`fg`/`bg`, `palette.ron`, the font
  atlas. Unchanged.
- Sprite skins: `assets/tilesets/<id>.ron` and its PNG (0433).
- Portraits and combat pictures: their sidecar files (0711, 0413).
- Bought images stay out of this repository (ADR-0032); every skin and
  picture has a public placeholder so a clone builds and passes every gate.

## Consequences

- Replacing the map's graphics becomes: add a tileset file and its image,
  and (if the art needs something new, like animation frames) extend the
  sprite skin. The battle screen, the rules, the bots and the behaviour
  tests don't change.
- 0711 and 0413 get simpler: a portrait or a combat picture is one sprite
  item, and `content` reads only a PNG's size, as it does for the font
  atlas. Dimming is the sprite's opacity over the frame's background, which
  matches ADR-0018's "lerp toward the background".
- Effects that cover the whole frame (0813's fades, 0817's shot fades, 0228's
  zoomed scene) must handle sprite items as well as cells. Those tickets say
  so.
- 0432 moves a lot of battle-screen drawing. It changes no pixel (existing
  snapshots must not change), but it will conflict with other open
  battle-screen branches; it says how to land it.
- Battle tests that read cells to learn the state are moved to scene
  assertions in 0434. New tests follow rule 4.
- One more thing to keep in step: a new map feature needs a line in
  `MapScene` and paint code in each skin. A skin that can't paint something
  must fail a test, not silently skip it (0433's coverage test).
- Colour themes (0806) recolour the glyph skin and the UI. Images keep their
  own colours.

## Alternatives considered

- **Keep drawing pictures as coloured rectangles (0711's plan)** — works for
  two faces, but it puts thousands of lines in snapshots, makes every
  `fill_rect` walk them, and gives map sprites and animation no path.
- **Replace `GlyphBuffer` with a general draw list (no cells)** — the menus,
  panels and text are cells and will stay cells; every screen and snapshot
  would be rewritten for no gain.
- **A second renderer in `app` for a sprite map, beside the glyph one** —
  puts game knowledge (tiles, units, ranges) in `app`, where nothing is
  tested (ADR-0004, ADR-0007).
- **Make the skin a trait object chosen at build time (a cargo feature)** —
  tests couldn't run both skins in one build, and the Pages build couldn't
  show Nick both.
- **Do nothing until sprites are wanted** — every battle-UI ticket adds more
  glyph-only drawing to the battle screen, and 0711/0413 are about to build
  on per-pixel rectangles; the lift grows with each one.
