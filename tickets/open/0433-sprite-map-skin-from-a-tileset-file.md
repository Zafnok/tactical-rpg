---
id: "0433"
title: "A sprite map skin read from a tileset file, switchable from the debug menu"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0231", "0432"]
nick_input: none
completed:
---

# 0433 — A sprite map skin from a tileset file

## Context

ADR-0038: the battle map is painted by a map skin, and swapping glyphs for
sprites should mean "add a tileset file and its image". 0432 made the seam
with one skin (glyphs). A seam with one user is unproven, so this ticket
adds the second: a skin that paints the map from a tileset image, with a
**generated test tileset** at a different tile size (24×24) so nothing can
quietly assume 16×16.

It is a debug tool, not a new look for the game: this ticket still uses
only the generated test tileset.

**Changed 2026-10-02 (ticket 0038):** Nick has decided which art the game
uses on the map: the bought Tiny Tales tilesets and map sprites
(`docs/design/look-and-feel.md`, *Battle map: bought tiles and unit
sprites*). The tickets that put that art in build on this one and wait for
it: 0436 (unit sprites from one image file per unit), 0437 (terrain, with
auto-tiling), 0438 (per-map lighting) and 0439 (a 1× / 2× zoom toggle).
Keep this ticket's scope as written; two things help them:

- don't assume a unit picture is as wide as it is tall, or that it comes
  from the tileset's own image (the bought ones are 16×20 frames in a
  file per unit);
- keep everything that depends on `tile_px` behind one value, so 0439 can
  double it while a battle runs.

**On the Chapter 1 critical path** since 2026-10-02: Nick said the
playtest waits for the bought-art map, so 0804 waits for 0436 and 0437,
which wait for this ticket.

**From 0432 (2026-10-01):** switching `ctx.map_skin` during a battle
already works for the camera: the battle screen takes the view's size from
the skin at the start of every frame and re-centres on the cursor when it
changed. Two things it does not redo, because nothing could switch skins
yet: the camera saved for the player's next phase, and the camera pan of an
AI action already playing. Both were worked out for the old view size. If
the debug menu can switch skins during an enemy phase, handle them here.
`faction_color` and `hp_fill` are in `screens/battle/units.rs`; the path's
geometry is in `map_view/glyph/path.rs` (tile size is a constant there).

## Nick input

None. Nothing changes for players: the glyph skin stays the default, and
the sprite skin is reachable only from the debug menu.

The looks below for things art alone doesn't show (whose side a unit is on,
that it has acted, its HP, an active effect, the cursor) are **Claude's
placeholders for the test skin**, not decisions. Say so in the PR. They are
decided with Nick in ticket 0039 and built in 0436.

## Scope

**In:**
- The tileset format `assets/tilesets/<id>.ron` + PNG, its loader and
  validator.
- `SpriteSkin`, a `MapSkin` that paints a `MapScene` with sprite items
  (0231) and rectangles.
- `cargo xtask test-tileset`: generates `assets/tilesets/test.png` and
  `test.ron` from `terrain.ron`, `classes.ron`, the palette and the font
  atlas.
- A debug-menu entry to switch skins.
- Tests that the rules, the cursor and the sounds don't depend on the skin.

**Out (do not do):**
- An Options-menu setting or any player-facing way to switch (Nick
  decides).
- Real art: drawing tiles or sprites by hand, or using bought files. The
  test tileset is generated.
- Animated tiles or units, auto-tiling (edges that depend on neighbours).
  Follow-up tickets when real art needs them.
- Changing the glyph skin or any glyph snapshot.

## Implementation steps

1. **Format** (document in `assets/tilesets/README.md`):
   ```ron
   (
       id: "test",
       image: "tilesets/test.png",
       tile_px: (24, 24),               // a map tile on screen, in console pixels
       terrain: {                       // terrain string id → (column, row) in tile_px units
           "plain": (0, 0), "road": (1, 0), "forest": (2, 0),
       },
       unit_px: (24, 24),               // a unit picture; may be taller than a tile
       units: (
           characters: {},              // character id → (column, row) in unit_px units
           classes: { "brigand": (0, 4), "rider": (1, 4) },
           fallback: (7, 4),            // any unit with no entry
       ),
       units_origin_px: (0, 96),        // where the unit grid starts in the image
   )
   ```
2. **Loader and validator** (`crates/content/src/tileset.rs`,
   `Content::tilesets`, the all-assets test). All errors at once, with the
   file name: the image is in `Content::images`; every rectangle lies
   inside it; `tile_px` sides in `8..=64`; **every terrain id in
   `terrain.ron` has a tile**; every named class and character exists.
3. **`cargo xtask test-tileset`** (`crates/xtask/src/test_tileset.rs`):
   for each terrain, a 24×24 tile filled with its `bg` colour with its two
   glyphs stamped in its `fg` colour from the font atlas, centred; for each
   class, a 24×24 picture: a filled circle with the first two letters of
   the class name stamped on it, in white (the skin shows the faction
   itself, step 4). An `xtask` test fails when the committed files are
   stale (as for the font atlas, ADR-0016). Generated from our own data
   and the OFL font: list it in `THIRD_PARTY_ASSETS.md` the way the atlas
   is listed.
4. **`crates/ui/src/map_view/sprite.rs`, `SpriteSkin { tileset }`**:
   - `view_tiles(area)`: `area` in pixels divided by `tile_px`, rounded
     down; the leftover pixels are split evenly as a margin.
   - Terrain: one `Under` sprite per visible tile.
   - Ranges: for a tinted tile, an `Under` rectangle in the range's
     palette colour, then the tile sprite at reduced opacity over it (the
     same blend strength the glyph skin uses, so the tile reads as tinted).
   - Units: one `Under` sprite, bottom-aligned on the tile and centred
     (a taller picture overlaps the tile above); looked up by character,
     then class, then `fallback`. Opacity from `acted` (the glyph skin's
     `ACTED_DIM`) and `fade`.
   - Placeholders (see *Nick input*): a 2-px faction-coloured bar along
     the tile's top edge with the unit's `label` not drawn; the HP bar as
     in the glyph skin, scaled to the tile's width; a 3×3-px `effect`
     colour mark in the tile's top-right corner when `has_effect`; the
     cursor as corner marks just inside the tile's corners, and for the
     glow style a `cursor`-coloured rectangle under the tile sprite drawn
     at reduced opacity, as for ranges (all three `CursorStyle`s must
     work); the path as the glyph skin's line
     through tile centres, computed from `tile_px`.
   - Share with the glyph skin whatever is the same (HP fill maths,
     `hp_fill`; the path's geometry given a tile size). Don't copy it.
5. **Debug menu** (`crates/ui/src/debug.rs`): "Map skin: glyph / test
   tileset", which swaps `ctx.map_skin`. Help line per the
   `keyboard-input` skill. Not saved.
6. **Harness:** `Harness::with_map_skin(name)` (or a setter on
   `ctx_mut()`), so any scripted test can run under either skin. Then
   run the tests 0434 moved onto the map scene under the sprite skin too
   (a loop over skins in their shared setup, e.g. `quick_battle()` in
   `crates/ui/tests/it/battle.rs`, `quick()` in `screens/battle/mod.rs`'s
   tests): they no longer depend on the look, so they must pass under
   both. The list is in 0434's completion notes.
7. Render Quick Battle under the sprite skin with `cargo xtask frame-png`
   (0232, if done; otherwise look at it in the web build's debug menu) and
   **look at it**: tiles line up, no seams, ranges read as tinted, a menu
   opened beside a unit sits beside it, popups cut the sprites beneath
   them cleanly.
8. Update `crates/ui/README.md` (how to add a skin; how to add a map
   feature to both).

## Acceptance criteria

- [ ] Content validation loads `tilesets/test.ron`; each validator error
      has a test with its message, including "terrain \"…\" has no tile".
- [ ] Integration test `the_skin_never_changes_the_game`: one long scripted
      Quick Battle (move, attack, an enemy phase, a rewind) run under the
      glyph skin and under the sprite skin ends with equal `BattleState`s,
      equal cursor tiles after every step, the same screen names and the
      same audio requests.
- [ ] Property `sprite_skin_paints_only_the_area`, as 0432's for the glyph
      skin.
- [ ] Test `every_scene_feature_is_painted`: for a scene with one of
      everything (each `RangeKind`, an acted unit, a fading unit, a unit
      with an effect, a cursor in each style, a path), the sprite skin's
      frame differs from the frame without that one feature. A new
      `MapScene` field without paint code must fail this test (build the
      list of features in one place both the test and reviewers can see).
- [ ] Snapshot: Quick Battle under the sprite skin (sprite lines), and with
      a unit selected.
- [ ] Every glyph-skin snapshot is unchanged.
- [ ] The frame was looked at (step 7); Completion notes say what was seen
      and list the placeholders for Nick.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the tileset loader and each error; `view_tiles` and the margin for
  several tile sizes; unit picture lookup order; opacity for acted and
  fading units.
- Property: the area property; for any tile size in `8..=64`, every
  visible tile's rectangle lies inside the map area and tiles don't
  overlap.
- Snapshot / integration: as listed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
