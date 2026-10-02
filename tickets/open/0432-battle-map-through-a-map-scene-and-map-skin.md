---
id: "0432"
title: "Battle map drawn through a map scene and a map skin (the look doesn't change)"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0432 — Battle map through a map scene and a map skin

## Context

Nick wants graphics to be replaceable (2026-10-01): if the battle map's
glyphs are swapped for sprites, that must be a small job. ADR-0038 sets the
rules. Today the map's look is written into the battle screen
(`crates/ui/src/screens/battle/mod.rs`): `draw_terrain`, `draw_ranges`,
`draw_units` and `draw_cursor_and_path` write glyph cells directly, and
"a tile is two 8×16 cells" is spread over `layout::TILE_W_CELLS`,
`layout::VIEW_TILES_W/H`, `camera::tile_to_cell`, `path::TILE_PX`,
`units::HP_BAR_W` and the cursor's pixel offsets (`cursor.rs`, ADR-0024).

This ticket puts a seam there and changes **no pixel**: the battle screen
builds a plain-data `MapScene` (what is on the visible map), and a `MapSkin`
paints it. The only skin is the glyph skin, which is today's drawing code,
moved. Ticket 0433 adds a second skin; 0434 moves tests onto the scene.

Numbering: 0430 is used by two open pull requests and 0431 will go to
whichever is renumbered, so this block continues at 0432.

## Nick input

None. Nothing a player sees changes.

## Scope

**In:**
- `MapScene`, `MapSkin`, `GlyphSkin` in a new `crates/ui/src/map_view/`
  module.
- The battle screen builds a scene and calls the skin for everything
  inside `MAP_VIEW`.
- Tile size and viewport size in tiles come from the skin.
- `Harness` access to the scene, and a text dump of it.

**Out (do not do):**
- Any change to how the map looks. Every existing snapshot must be
  byte-identical.
- A sprite skin, tileset files, a skin setting (0433).
- Rewriting existing tests to use the scene (0434).
- Panels, menus, the forecast, the combat box, dialogs: they stay cell
  drawing in the battle screen.
- Moving `Unit::map_label` out of `core` (ADR-0038 keeps it).

## Implementation steps

1. **Read ADR-0038** (sections 1, 3, 4).
2. **`crates/ui/src/map_view/scene.rs`**, plain data, `Clone + PartialEq +
   Debug`, no palette colours and no cell or pixel coordinates:
   ```rust
   pub struct MapScene {
       pub origin: Pos,               // map tile at the view's top-left (may be negative)
       pub size: (i32, i32),          // visible tiles, across × down
       pub tiles: Vec<TileView>,      // row-major, size.0 × size.1
       pub units: Vec<UnitView>,      // only units on visible tiles, in state order
       pub cursor: Option<CursorView>,
       pub path: Vec<Pos>,            // the selected unit's path, origin first; empty = none
   }
   pub struct TileView {
       pub terrain: Option<TerrainId>,   // None = off the map
       pub tints: Vec<RangeKind>,        // in the order they are laid on
   }
   pub enum RangeKind { Danger, Move, Attack, Heal }
   pub struct UnitView {
       pub id: UnitId,
       pub pos: Pos,                  // where it is drawn (mid-walk positions included)
       pub faction: Faction,
       pub label: String,             // core's map_label
       pub class: ClassId,
       pub character: Option<CharacterId>,
       pub acted: bool,
       pub hp: (StatValue, StatValue),   // current, max
       pub has_effect: bool,          // under a timed effect (0412)
       pub fade: f32,                 // 0 = solid … 1 = gone (falling, 0404)
   }
   pub struct CursorView { pub pos: Pos, pub brightness: f32, pub style: CursorStyle }
   ```
   Adjust fields to what the current drawing code really reads (check
   `units::draw_fading_unit`, `draw_ranges`, `draw_cursor_and_path`); the
   rule is that a skin needs nothing but the scene, the palette and
   content.
3. **`BattleScreen::scene(&self, ctx: &Ctx) -> MapScene`**: the logic now
   in `draw_terrain`, `draw_ranges`, `draw_units` and
   `draw_cursor_and_path` that decides *what* is shown (which mode shows
   which ranges, when the cursor is hidden, playback's shown units and
   fades), with none of the drawing.
4. **`crates/ui/src/map_view/skin.rs`**:
   ```rust
   pub trait MapSkin {
       fn name(&self) -> &'static str;                 // "glyph"
       /// Visible tiles for a map area of `area` cells.
       fn view_tiles(&self, area: Rect) -> (i32, i32);
       /// Paints `scene` into `area`. Must touch nothing outside it.
       fn paint(&self, ctx: &Ctx, scene: &MapScene, area: Rect, buf: &mut GlyphBuffer);
       /// The console pixel rectangle of `tile`, if visible: where menus and
       /// popups anchor.
       fn tile_px(&self, scene: &MapScene, area: Rect, tile: Pos) -> Option<PxRect>;
   }
   ```
   `Ctx` gains `map_skin: Rc<dyn MapSkin>` (default `GlyphSkin`).
5. **`crates/ui/src/map_view/glyph.rs`, `GlyphSkin`**: move the bodies of
   `draw_terrain`, the tinting in `draw_ranges`, `units.rs`, the drawing
   half of `cursor.rs` and `path::path_overlays` behind `paint`. Keep the
   functions and their unit tests; change their callers, not their output.
   `TILE_W_CELLS`, `path::TILE_PX`, `HP_BAR_W` and the cursor offsets
   become private to the glyph skin.
6. **Camera** (`camera.rs`): `Camera::centred_on` and `follow` take the
   view size in tiles as an argument instead of reading
   `VIEW_TILES_W/H`. `tile_to_cell` stays as the glyph skin's helper;
   the battle screen's menu and popup placement (`draw_menu`,
   `draw_popups`) asks `skin.tile_px` and converts to cells
   (`x / CELL_W_PX`, `y / CELL_H_PX`). Remove `VIEW_TILES_W/H` from
   `layout.rs` or keep them only as the glyph skin's values in its tests.
7. **`BattleScreen::draw`** calls
   `ctx.map_skin.paint(ctx, &self.scene(ctx), MAP_VIEW, buf)` where the
   four map draw calls were. Order relative to menus and panels is
   unchanged.
8. **Text dump**, `MapScene::to_text(&self, content: &Content) -> String`,
   for tests and bug reports: a header (`origin`, `size`), one row of
   terrain string ids per tile row with range marks, then a line per unit
   (`Br enemy brigand (7,3) hp 20/30 acted effect fade=0.25`), the cursor
   and the path. Deterministic; no colours.
9. **Harness** (`crates/ui/src/harness.rs`): `map_scene(&self) ->
   Option<MapScene>` (the battle screen on the stack, through
   `Screen::as_any` or `flow()`), and `map_text()`.
10. If the cinematic player (0817) is already done, make its map shots
    paint through `ctx.map_skin` too and delete its copy of the terrain
    drawing. If it isn't, 0817 says to use the skin.
11. Update `crates/ui/README.md` (a "Map view" section: scene, skin, the
    rule from ADR-0038 that new map things go in the scene).
12. **Landing it.** This moves code that other open battle-screen branches
    touch. Rebase on `main` right before opening the PR, keep moves and
    edits in separate commits, and do nothing else in the PR.

## Acceptance criteria

- [ ] `git diff --stat` shows no `.snap` file changed.
- [ ] `grep -rn "TILE_W_CELLS\|TILE_PX\|VIEW_TILES_" crates/ui/src` finds
      matches only under `crates/ui/src/map_view/glyph*`.
- [ ] `BattleScreen` no longer calls `buf.set`, `blend_bg` or
      `add_overlay` for anything inside `MAP_VIEW` except through the
      skin.
- [ ] Unit: `scene()` in each mode that shows ranges (selected, threat,
      move-after, targeting, skill target, item target, danger zone on)
      lists the tints the old code drew; the cursor is `None` in the modes
      that hid it.
- [ ] Property `glyph_skin_paints_only_the_area`: for any scene and area,
      cells and items outside `area` are untouched.
- [ ] Property `scene_units_are_visible`: every `UnitView::pos` lies inside
      `origin..origin + size`.
- [ ] Harness test: `map_text()` of the Quick Battle start is pinned as an
      insta snapshot; after `keys` that move the cursor one tile, only the
      cursor line changes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `scene()` per mode; camera with a view size other than 35 × 30;
  `to_text` format.
- Property: the two above.
- Snapshot / integration: the `map_text()` snapshot; every existing battle
  snapshot unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
