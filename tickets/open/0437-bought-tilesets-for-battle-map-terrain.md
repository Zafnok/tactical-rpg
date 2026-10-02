---
id: "0437"
title: "Battle-map terrain drawn from the bought tilesets"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: xhigh
status: todo
blocked_by: ["0433", "0110", "0436"]
nick_input: sign-off
completed:
---

# 0437 — Battle-map terrain from the bought tilesets

## Context

Nick decided on 2026-10-02 (ticket 0038, `docs/design/look-and-feel.md`,
*Battle map: bought tiles and unit sprites*) that the battle map's terrain
is drawn with the bought Tiny Tales tilesets. 0436 puts the unit sprites
on the map; this ticket replaces the glyph terrain under them.

**What was bought** (sorted on Nick's machine in
`D:\tactical-rpg\Tiny Tales Bundle Assets\tilesets\<set>\`; 0110 decides
how files reach `assets-private/`). Every tile is 16×16 pixels, our tile
size, and every set comes as Tiled files: `Images/*.png` (256×256, a
16×16 grid of tiles), one `.tsx` per image, and `Sample Maps/*.tmx` with a
rendered `.png`.

| Set | Has |
| --- | --- |
| `world-map` | Grass, dirt, sand and snow ground in three colour families (Standard, Beige, Grey), coasts with shallow and deep water, waterfalls, bridges, mountains and tall peaks, tree masses in seven colours, single-tile towns, castles, towers and caves (`Set_C_Icons`). The spike's render C was its sample map `Island2`. |
| `overworld` | Walk-around scale: paths, cliffs, ponds, trees two tiles tall, houses several tiles big, interiors, desert and snow versions. |
| `dungeons-1`, `dungeons-2` | Stone floors and walls, castle exteriors, mines, doors and chests (`animated-objects/`). |
| `tower` | Tower floors, walls and sky pictures. |

**Edges depend on neighbours.** A grass tile next to water is a different
picture from grass next to grass. The `.tsx` files already say which is
which: Tiled's corner terrains, e.g. `<tile id="17" terrain=",,0,0"/>`
(the four corners, top-left, top-right, bottom-left, bottom-right, each
naming a `<terrain>` or empty). 0433 left auto-tiling out; this ticket
adds it.

Our maps are one terrain id per tile (`assets/maps/*.map`), written by
Claude as text and later by the skirmish generator (0510). So the tile
pictures must be **worked out from the terrain grid**, not hand-laid.

## Nick input

**Sign-off:** rendered frames of the Quick Battle map and the Chapter 1
map (if 0803 is done) in the bought tiles at 1×, sent to Nick, never
committed. Tell him plainly where a terrain has no good bought picture
(below) and what stands in.

## Scope

**In:**
- An importer that turns a bought Tiled tileset into our tileset file and
  one packed image in `assets-private/tilesets/`.
- Auto-tiling in the sprite skin: each tile's picture chosen from its own
  terrain and its neighbours'.
- A table from each of our 16 terrain ids (`assets/data/terrain.ron`) to
  bought tiles, for one outdoor look (the World Map set, Standard colours)
  and one indoor look (a Dungeons set).
- Which look a map uses, named in the map file's header as look data that
  never reaches `trpg-core`.
- The bought-tile skin becomes the default when its private tileset is
  present (0436 made the unit half the default).
- A generated public fixture so tests and a public clone exercise the same
  code.

**Out (do not do):**
- Lighting (0438) and zoom (0439).
- Animated tiles (the water and waterfall frames): one still frame. A
  follow-up ticket if Nick wants them moving.
- Objects bigger than one tile (the large castles and houses) and
  hand-placed decoration. A follow-up ticket; say in the PR what the maps
  lack without them.
- New terrain types (villages, gates, thrones: 0037, 0313).
- The 48-pixel packs (*Terrains Plus*, *Trees*): another style and size.
- Drawing tiles. Recolours only (`look-and-feel.md`).
- Committing any bought file, or a picture made from one.

## Implementation steps

1. **ADR** (`write-adr`), extending ADR-0038 section 3: auto-tiling from
   corner terrains; the importer as the only reader of Tiled files (the
   game reads only our RON and PNG, so no XML code ships); map look data
   in the map header; which skin is the default.
2. **Format** (`crates/content/src/tileset.rs`): next to 0433's
   `terrain: { id: (column, row) }`, allow an auto-tiled entry: for a
   terrain id, a list of `(corners: (tl, tr, bl, br), at: (column, row))`
   where each corner is that terrain or a named neighbour group, plus a
   plain fallback tile. Validate that every terrain id has at least a
   fallback. Keep it small: the importer does the thinking.
3. **Corner rule** (pure function, `crates/ui/src/map_view/`): a corner of
   a tile takes the terrain shared by the (up to four) tiles that touch
   that corner, by a fixed priority order from the tileset file (e.g.
   water over sand over grass), so two neighbouring tiles always agree on
   their shared corner. Off-map tiles count as the tile's own terrain.
   Test it on hand-made 3×3 grids.
4. **Importer** `cargo xtask tileset-import <tiled-dir> <mapping.ron>
   <out-id>`: reads the `.tsx` corner tables (an XML reader crate is fine
   in `xtask` only; check its licence against ADR-0013), takes the tiles
   the mapping names, packs them into one PNG and writes the tileset RON
   into `assets-private/tilesets/`. The mapping file (which bought terrain
   stands for which of our terrain ids) is ours and may be committed: it
   holds names and numbers, no art.
5. **Mapping**, outdoor (`world-map`, Standard): `plain` grass, `road`
   dirt, `forest` the tree masses, `thicket` the darker tree masses,
   `mountain` and `peak` from `Set_F_Mountains`, `water` shallow and `sea`
   deep coast water, `bridge` from `Set_B_World` (pick the horizontal or
   vertical one from which neighbours are water), `fort` a single-tile
   castle from `Set_C_Icons`, `ice` the snow set's frozen ground, `burnt`
   bare trees or dark ground. Indoor (`dungeons-1`): `floor`, `wall`,
   `door`. A terrain a set lacks falls back to the other set's tile.
   **Known gap:** nothing in the bundle is a burning tile. Use the forest
   tile recoloured toward the `fire` palette colour (a recolour is
   allowed) and tell Nick; the dungeon sets' brazier sprites show the
   artist's own fire if he wants a better one made from them.
6. **Map look**: the `.map` header gains an optional
   `look: (tiles: "outdoor")` (default `"outdoor"`). `trpg_content::map`
   keeps it in a `MapLook` beside the core map (ADR-0038 section 5: map
   files name terrain by id; looks hang off ids). Document in
   `assets/maps/README.md`. 0438 adds the light to the same struct.
7. **Skin**: `SpriteSkin` paints terrain through the corner rule; ranges
   tint the tile as in 0433; the cursor and path are unchanged. With the
   private tileset present, `Ctx.map_skin` is this skin with 0436's unit
   sprites; otherwise the glyph skin.
8. **Public fixture**: extend `cargo xtask test-tileset` to emit an
   auto-tiled test tileset (coloured corner quarters), so the snapshot and
   property tests need no bought file.
9. Render the Quick Battle and Chapter 1 maps with `cargo xtask frame-png`
   (0232 if done), **look at them** (no seams, shores join, bridges point
   the right way), and send them to Nick.

## Acceptance criteria

- [ ] The corner rule: for every pair of neighbouring tiles in a random
      grid, the corners they share are equal (property test).
- [ ] Every terrain id in `terrain.ron` gets a tile from both mappings;
      the validator's "terrain has no tile" error still fires for a
      tileset that lacks one.
- [ ] A clone without `assets-private/` builds, runs with the glyph skin
      and passes every gate; glyph snapshots unchanged.
- [ ] Snapshot of the Quick Battle under the auto-tiled public fixture.
- [ ] `the_skin_never_changes_the_game` (0433) passes under the new skin.
- [ ] No bought file and no picture made from one is in the PR.
- [ ] Nick was sent the rendered maps and the list of stand-ins.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the corner rule on fixed grids (a lake, a river with a bridge, a
  map edge); the tileset format's new entries and errors; the importer on
  a small made-up `.tsx` and image (never a bought one); `MapLook`
  parsing and its default.
- Property: shared corners agree; every visible tile gets exactly one
  terrain sprite inside the map area.
- Snapshot / integration: as listed; behaviour tests read the `MapScene`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
