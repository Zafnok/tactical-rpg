//! The glyph skin: the battle map drawn with coloured glyphs (ADR-0018,
//! ADR-0024, ADR-0029, `docs/design/look-and-feel.md`). A tile is two 8 × 16
//! cells: a 16 × 16 pixel square. That size is known only here.

pub mod cursor;
pub mod path;
pub mod units;

use trpg_core::{Pos, TerrainId};

use super::scene::{MapScene, RangeKind};
use super::skin::MapSkin;
use crate::color::{Palette, Rgb, UiColor};
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{Cell, GlyphBuffer, PxRect, Rect};
use crate::screen::Ctx;

/// Cells per map tile, across (a tile is two glyphs wide, ADR-0018).
const TILE_W_CELLS: i32 = 2;

/// How far range overlays tint a tile's background toward their colour
/// (`look-and-feel.md`: about 75%). *Tunable.*
pub const OVERLAY_BLEND: f32 = 0.75;

/// The battle map as coloured glyphs: today's look (ADR-0038).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlyphSkin;

impl MapSkin for GlyphSkin {
    fn name(&self) -> &'static str {
        "glyph"
    }

    fn view_tiles(&self, area: Rect) -> (i32, i32) {
        (area.w.max(0) / TILE_W_CELLS, area.h.max(0))
    }

    /// Terrain, then the ranges tinting it, then the units, the path and the
    /// cursor.
    fn paint(&self, ctx: &Ctx, scene: &MapScene, area: Rect, buf: &mut GlyphBuffer) {
        let layout = Layout::new(scene, area);
        draw_tiles(ctx, buf, scene, &layout);
        for unit in &scene.units {
            if let Some((x, y)) = tile_to_cell(unit.pos, &layout) {
                units::draw_unit(buf, &ctx.palette, unit, x, y);
            }
        }
        let color = ctx.palette.get(UiColor::Path);
        for overlay in path::path_overlays(&scene.path, &layout, color) {
            buf.add_overlay(overlay);
        }
        if let Some(c) = &scene.cursor
            && let Some((x, y)) = tile_to_cell(c.pos, &layout)
        {
            cursor::draw_cursor(buf, &ctx.palette, c, &layout, x, y);
        }
    }

    fn tile_px(&self, scene: &MapScene, area: Rect, tile: Pos) -> Option<PxRect> {
        let (x, y) = tile_to_cell(tile, &Layout::new(scene, area))?;
        Some(px_rect(Rect::new(x, y, TILE_W_CELLS, 1)))
    }
}

/// Where a scene's tiles go in an area of cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// The map tile in the area's top-left corner.
    origin: Pos,
    /// The top-left cell.
    cell: (i32, i32),
    /// Tiles drawn, across × down: the scene's, or as many as fit.
    tiles: (i32, i32),
}

impl Layout {
    /// The layout of `scene` in `area`.
    pub fn new(scene: &MapScene, area: Rect) -> Self {
        let (fit_w, fit_h) = GlyphSkin.view_tiles(area);
        Self {
            origin: scene.origin,
            cell: (area.x, area.y),
            tiles: (scene.size.0.clamp(0, fit_w), scene.size.1.clamp(0, fit_h)),
        }
    }

    /// The cells the tiles cover.
    fn cells(&self) -> Rect {
        let (x, y) = self.cell;
        Rect::new(x, y, TILE_W_CELLS * self.tiles.0, self.tiles.1)
    }

    /// The top-left pixel of `tile` on the console (it may lie outside the
    /// area).
    fn tile_px(&self, tile: Pos) -> (i32, i32) {
        let cx = self.cell.0 + TILE_W_CELLS * (tile.x - self.origin.x);
        let cy = self.cell.1 + (tile.y - self.origin.y);
        (cx * i32::from(CELL_W_PX), cy * i32::from(CELL_H_PX))
    }
}

/// The console cell of the left glyph of `tile`, or `None` if the tile is
/// outside the layout. The only place the "two cells per tile" rule lives
/// (ADR-0018).
pub fn tile_to_cell(tile: Pos, layout: &Layout) -> Option<(i32, i32)> {
    let dx = tile.x.checked_sub(layout.origin.x)?;
    let dy = tile.y.checked_sub(layout.origin.y)?;
    let inside = (0..layout.tiles.0).contains(&dx) && (0..layout.tiles.1).contains(&dy);
    inside.then(|| (layout.cell.0 + TILE_W_CELLS * dx, layout.cell.1 + dy))
}

/// `cells` in console pixels.
fn px_rect(cells: Rect) -> PxRect {
    let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
    Rect::new(cells.x * cw, cells.y * ch, cells.w * cw, cells.h * ch)
}

/// The palette colour of a range.
const fn range_color(kind: RangeKind) -> UiColor {
    match kind {
        RangeKind::Danger => UiColor::DangerZone,
        RangeKind::Move => UiColor::MoveRange,
        RangeKind::Attack => UiColor::AttackRange,
        RangeKind::Heal => UiColor::HealRange,
    }
}

/// Draws terrain `id` on the tile whose left cell is `(x, y)`: its two
/// glyphs in its colours. Nothing for a terrain the content lacks.
fn draw_terrain(ctx: &Ctx, buf: &mut GlyphBuffer, id: TerrainId, (x, y): (i32, i32)) {
    let Some(t) = ctx.content.terrain.display.get(id) else {
        return;
    };
    let fg = named(&ctx.palette, &t.fg, UiColor::Text);
    let bg = named(&ctx.palette, &t.bg, UiColor::Black);
    for (i, glyph) in (0..).zip(t.glyphs) {
        buf.set(x + i, y, Cell::new(glyph, fg, bg));
    }
}

/// Draws every tile: its terrain (tiles off the map are left as they are,
/// blank); a tile whose terrain just changed flashes, its background tinted
/// towards its glyph colour, fading out; then its ranges tint it, the first
/// laid on undermost; and a tile a spell would change is drawn as the
/// terrain it would become instead (0410).
fn draw_tiles(ctx: &Ctx, buf: &mut GlyphBuffer, scene: &MapScene, layout: &Layout) {
    for dy in 0..layout.tiles.1 {
        for dx in 0..layout.tiles.0 {
            let Some(tile) = scene.tile_at(dx, dy) else {
                continue;
            };
            let (x, y) = (layout.cell.0 + TILE_W_CELLS * dx, layout.cell.1 + dy);
            let cells = Rect::new(x, y, TILE_W_CELLS, 1);
            if let Some(id) = tile.terrain {
                draw_terrain(ctx, buf, id, (x, y));
            }
            for &strength in &tile.flashes {
                if let Some(fg) = buf.get(x, y).map(|c| c.fg) {
                    buf.blend_bg(cells, fg, OVERLAY_BLEND * strength);
                }
            }
            for &kind in &tile.tints {
                let color = ctx.palette.get(range_color(kind));
                buf.blend_bg(cells, color, OVERLAY_BLEND);
            }
            if let Some(id) = tile.becomes {
                draw_terrain(ctx, buf, id, (x, y));
            }
        }
    }
}

/// The palette colour called `name`, or `fallback` (content validation
/// makes sure terrain colours exist, so this only guards against a bug).
fn named(palette: &Palette, name: &str, fallback: UiColor) -> Rgb {
    palette
        .lookup(name)
        .unwrap_or_else(|| palette.get(fallback))
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;
    use trpg_core::{ClassId, Faction, UnitId};

    use super::*;
    use crate::map_view::scene::{CursorStyle, CursorView, TileView, UnitView};
    use crate::screen::tests::ctx;
    use crate::screens::battle::layout::MAP_VIEW;

    /// The battle screen's map area with the view's top-left on `origin`.
    pub fn layout_at(origin: Pos) -> Layout {
        let size = GlyphSkin.view_tiles(MAP_VIEW);
        Layout::new(&MapScene::new(origin, size), MAP_VIEW)
    }

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn the_battle_map_area_shows_35_by_30_tiles() {
        assert_eq!(GlyphSkin.name(), "glyph");
        assert_eq!(GlyphSkin.view_tiles(MAP_VIEW), (35, 30));
        // An odd cell is left over; an empty area shows nothing.
        assert_eq!(GlyphSkin.view_tiles(Rect::new(3, 4, 7, 5)), (3, 5));
        assert_eq!(GlyphSkin.view_tiles(Rect::new(0, 0, -4, -1)), (0, 0));
    }

    #[test]
    fn tile_to_cell_maps_two_cells_per_tile() {
        let c = layout_at(p(5, 2));
        assert_eq!(tile_to_cell(Pos::new(5, 2), &c), Some((0, 0)));
        assert_eq!(tile_to_cell(Pos::new(6, 3), &c), Some((2, 1)));
        assert_eq!(tile_to_cell(Pos::new(39, 31), &c), Some((68, 29)));
        assert_eq!(tile_to_cell(Pos::new(40, 2), &c), None);
        assert_eq!(tile_to_cell(Pos::new(5, 32), &c), None);
        assert_eq!(tile_to_cell(Pos::new(4, 2), &c), None);
        assert_eq!(tile_to_cell(Pos::new(5, 1), &c), None);
        let small = layout_at(p(-10, -11));
        assert_eq!(tile_to_cell(Pos::new(0, 0), &small), Some((20, 11)));
        assert_eq!(tile_to_cell(Pos::new(i32::MIN, 0), &c), None);
        assert_eq!(tile_to_cell(Pos::new(0, i32::MIN), &c), None);
    }

    #[test]
    fn a_layout_starts_at_its_area_and_draws_no_more_tiles_than_fit() {
        let area = Rect::new(4, 3, 7, 2);
        // The scene is bigger than the area: 3 × 2 tiles fit.
        let big = Layout::new(&MapScene::new(p(1, 1), (9, 9)), area);
        assert_eq!(big.cells(), Rect::new(4, 3, 6, 2));
        assert_eq!(tile_to_cell(p(1, 1), &big), Some((4, 3)));
        assert_eq!(tile_to_cell(p(3, 2), &big), Some((8, 4)));
        assert_eq!(tile_to_cell(p(4, 2), &big), None);
        assert_eq!(tile_to_cell(p(3, 3), &big), None);
        assert_eq!(big.tile_px(p(1, 1)), (32, 48));
        assert_eq!(big.tile_px(p(0, 3)), (16, 80));
        // The scene is smaller: only its tiles.
        let small = Layout::new(&MapScene::new(p(1, 1), (2, 1)), area);
        assert_eq!(small.cells(), Rect::new(4, 3, 4, 1));
        assert_eq!(tile_to_cell(p(2, 1), &small), Some((6, 3)));
        assert_eq!(tile_to_cell(p(3, 1), &small), None);
        assert_eq!(tile_to_cell(p(1, 2), &small), None);
    }

    #[test]
    fn px_rect_scales_cells_to_pixels() {
        assert_eq!(px_rect(Rect::new(3, 2, 5, 4)), Rect::new(24, 32, 40, 64));
        assert_eq!(px_rect(MAP_VIEW), Rect::new(0, 0, 560, 480));
        assert_eq!(layout_at(p(0, 0)).cells(), MAP_VIEW);
    }

    #[test]
    fn tile_px_is_the_16_px_square_of_a_visible_tile() {
        let scene = MapScene::new(p(-10, -11), GlyphSkin.view_tiles(MAP_VIEW));
        let px = |tile| GlyphSkin.tile_px(&scene, MAP_VIEW, tile);
        assert_eq!(px(p(0, 0)), Some(Rect::new(160, 176, 16, 16)));
        assert_eq!(px(p(24, 18)), Some(Rect::new(544, 464, 16, 16)));
        assert_eq!(px(p(25, 18)), None);
        assert_eq!(px(p(-11, 0)), None);
        assert_eq!(
            GlyphSkin.tile_cells(&scene, MAP_VIEW, p(3, 5)),
            Some(Rect::new(26, 16, 2, 1))
        );
        // In another area: from its corner.
        let area = Rect::new(4, 3, 7, 2);
        assert_eq!(
            GlyphSkin.tile_px(&scene, area, p(-9, -10)),
            Some(Rect::new(48, 64, 16, 16))
        );
    }

    #[test]
    fn ranges_have_their_palette_colours_and_unknown_names_fall_back() {
        assert_eq!(range_color(RangeKind::Danger), UiColor::DangerZone);
        assert_eq!(range_color(RangeKind::Move), UiColor::MoveRange);
        assert_eq!(range_color(RangeKind::Attack), UiColor::AttackRange);
        assert_eq!(range_color(RangeKind::Heal), UiColor::HealRange);
        let p = &ctx().palette;
        assert_eq!(
            named(p, "no_such_colour", UiColor::Cursor),
            p.get(UiColor::Cursor)
        );
    }

    /// A buffer the size of the console, every cell `fill()`.
    fn blank() -> GlyphBuffer {
        GlyphBuffer::new(100, 32, fill())
    }

    fn fill() -> Cell {
        Cell::new('x', Rgb::new(1, 2, 3), Rgb::new(4, 5, 6))
    }

    fn brigand(pos: Pos) -> UnitView {
        UnitView {
            id: UnitId(4),
            pos,
            faction: Faction::Enemy,
            label: "Br".into(),
            class: ClassId("brigand".into()),
            character: None,
            acted: false,
            hp: (30, 30),
            has_effect: false,
            fade: 0.0,
        }
    }

    #[test]
    fn paints_terrain_then_tints_then_units_path_and_cursor() {
        let c = ctx();
        let pal = &c.palette;
        let display = &c.content.terrain.display;
        let plain_id = display.id_of("plain").unwrap();
        let plain = display.get(plain_id).unwrap();
        let (fg, bg) = (
            pal.lookup(&plain.fg).unwrap(),
            pal.lookup(&plain.bg).unwrap(),
        );
        let area = Rect::new(10, 5, 8, 3);
        let mut scene = MapScene::new(p(2, 2), (4, 3));
        for pos in [p(2, 2), p(3, 2), p(4, 2), p(5, 2), p(2, 3)] {
            scene.tile_mut(pos).unwrap().terrain = Some(plain_id);
        }
        // A terrain the content lacks is left blank, as off the map.
        scene.tile_mut(p(3, 3)).unwrap().terrain = Some(TerrainId(999));
        scene.tint([p(3, 2)], RangeKind::Danger);
        scene.tint([p(3, 2), p(4, 2)], RangeKind::Move);
        // A range off the map still tints.
        scene.tint([p(4, 4)], RangeKind::Heal);
        scene.push_unit(brigand(p(4, 2)));
        scene.path = vec![p(2, 2), p(3, 2)];
        scene.cursor = Some(CursorView {
            pos: p(5, 2),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        let mut buf = blank();
        GlyphSkin.paint(&c, &scene, area, &mut buf);
        let cell = |x, y| *buf.get(x, y).unwrap();
        // Tile (2, 2) at cells (10, 5) and (11, 5): plain.
        assert_eq!(cell(10, 5), Cell::new(plain.glyphs[0], fg, bg));
        assert_eq!(cell(11, 5), Cell::new(plain.glyphs[1], fg, bg));
        // (3, 2): danger under move.
        let blend = |bg: Rgb, kind| bg.lerp(pal.get(range_color(kind)), OVERLAY_BLEND);
        let both = blend(blend(bg, RangeKind::Danger), RangeKind::Move);
        assert_eq!(cell(12, 5), Cell::new(plain.glyphs[0], fg, both));
        assert_eq!(cell(13, 5).bg, both);
        // (4, 2): the unit's letters on the tinted tile.
        let moved = blend(bg, RangeKind::Move);
        assert_eq!(cell(14, 5), Cell::new('B', pal.get(UiColor::Enemy), moved));
        assert_eq!(cell(15, 5), Cell::new('r', pal.get(UiColor::Enemy), moved));
        // (2, 3) plain; (3, 3) unknown and (4, 3) off the map: untouched;
        // (4, 4) off the map but tinted.
        assert_eq!(cell(10, 6).glyph, plain.glyphs[0]);
        assert_eq!(cell(12, 6), fill());
        assert_eq!(cell(14, 6), fill());
        let healed = Cell {
            bg: blend(fill().bg, RangeKind::Heal),
            ..fill()
        };
        assert_eq!((cell(14, 7), cell(15, 7)), (healed, healed));
        assert_eq!(cell(16, 7), fill());
        // Overlays in order: the unit's HP bar, the path, the cursor.
        let colors: Vec<Rgb> = buf.overlays().iter().map(|o| o.color).collect();
        let path = pal.get(UiColor::Path);
        let mut expect = vec![pal.get(UiColor::HpHigh), path];
        expect.extend([path; 6]);
        expect.extend([pal.get(UiColor::Cursor); 8]);
        assert_eq!(colors, expect);
        // The HP bar under the unit: cells 14..16 of row 5.
        assert_eq!(buf.overlays()[0].rect, Rect::new(112, 94, 16, 2));
    }

    #[test]
    fn a_changed_tile_flashes_and_a_tile_a_spell_would_change_shows_what_it_becomes() {
        let c = ctx();
        let pal = &c.palette;
        let display = &c.content.terrain.display;
        let look = |name: &str| {
            let t = display.get(display.id_of(name).unwrap()).unwrap();
            let (fg, bg) = (pal.lookup(&t.fg).unwrap(), pal.lookup(&t.bg).unwrap());
            (t.glyphs, fg, bg)
        };
        let (plain, forest) = (look("plain"), look("forest"));
        let mut scene = MapScene::new(p(0, 0), (5, 1));
        for tile in &mut scene.tiles {
            tile.terrain = display.id_of("plain");
        }
        // Half a flash; a full flash under a range; two flashes at once.
        scene.tile_mut(p(0, 0)).unwrap().flashes = vec![0.5];
        scene.tile_mut(p(1, 0)).unwrap().flashes = vec![1.0];
        scene.tile_mut(p(4, 0)).unwrap().flashes = vec![1.0, 0.5];
        scene.tint([p(1, 0), p(2, 0), p(3, 0)], RangeKind::Attack);
        // A forest-to-be, and a terrain the content lacks.
        scene.tile_mut(p(2, 0)).unwrap().becomes = display.id_of("forest");
        scene.tile_mut(p(3, 0)).unwrap().becomes = Some(TerrainId(999));
        let mut buf = blank();
        GlyphSkin.paint(&c, &scene, Rect::new(0, 0, 10, 1), &mut buf);
        let cell = |x| *buf.get(x, 0).unwrap();
        let (glyphs, fg, bg) = plain;
        let attack = pal.get(UiColor::AttackRange);
        // Towards the tile's own glyph colour, by the flash's strength.
        let half = bg.lerp(fg, OVERLAY_BLEND * 0.5);
        assert_eq!(cell(0), Cell::new(glyphs[0], fg, half));
        assert_eq!(cell(1), Cell::new(glyphs[1], fg, half));
        // The range goes over the flash.
        let full = bg.lerp(fg, OVERLAY_BLEND);
        assert_eq!(cell(2).bg, full.lerp(attack, OVERLAY_BLEND));
        // What it would become replaces the tile, range and all…
        assert_eq!(cell(4), Cell::new(forest.0[0], forest.1, forest.2));
        assert_eq!(cell(5), Cell::new(forest.0[1], forest.1, forest.2));
        // …unless the content lacks it.
        assert_eq!(
            cell(6),
            Cell::new(glyphs[0], fg, bg.lerp(attack, OVERLAY_BLEND))
        );
        // Each flash tints in turn.
        assert_eq!(cell(8).bg, full.lerp(fg, OVERLAY_BLEND * 0.5));
        assert_ne!(forest, plain);
    }

    #[test]
    fn units_and_the_cursor_off_the_view_are_not_drawn() {
        let c = ctx();
        let area = Rect::new(0, 0, 4, 1);
        let mut scene = MapScene::new(p(0, 0), (2, 1));
        // Not through `push_unit`: a scene built by hand.
        scene.units.push(brigand(p(2, 0)));
        scene.cursor = Some(CursorView {
            pos: p(0, 1),
            brightness: 1.0,
            style: CursorStyle::TileGlow,
        });
        let mut buf = blank();
        GlyphSkin.paint(&c, &scene, area, &mut buf);
        assert_eq!(buf, blank());
    }

    prop_compose! {
        fn any_pos()(x in -25..60i32, y in -25..60i32) -> Pos {
            Pos::new(x, y)
        }
    }

    prop_compose! {
        fn any_tile()(
            terrain in prop::option::of(0u16..24),
            flashes in prop::collection::vec(0.0f32..1.0, 0..2),
            tints in prop::collection::vec(0usize..4, 0..3),
            becomes in prop::option::of(0u16..24),
        ) -> TileView {
            let kinds = [RangeKind::Danger, RangeKind::Move, RangeKind::Attack, RangeKind::Heal];
            TileView {
                terrain: terrain.map(TerrainId),
                flashes,
                tints: tints.into_iter().map(|i| kinds[i]).collect(),
                becomes: becomes.map(TerrainId),
            }
        }
    }

    prop_compose! {
        fn any_unit()(
            pos in any_pos(),
            label in "[A-Za-z]{0,4}",
            hp in -5..40i32,
            flags in 0u8..4,
            fade in -0.5f32..1.5,
        ) -> UnitView {
            UnitView {
                label,
                acted: flags & 1 != 0,
                has_effect: flags & 2 != 0,
                hp: (hp, 30),
                fade,
                ..brigand(pos)
            }
        }
    }

    prop_compose! {
        fn any_cursor()(
            pos in any_pos(),
            brightness in 0.5f32..1.0,
            style in prop::sample::select(vec![
                CursorStyle::Corners,
                CursorStyle::LargeCorners,
                CursorStyle::TileGlow,
            ]),
        ) -> CursorView {
            CursorView { pos, brightness, style }
        }
    }

    prop_compose! {
        fn any_scene()(
            origin in any_pos(),
            size in (-2..45i32, -2..40i32),
            tiles in prop::collection::vec(any_tile(), 0..200),
            units in prop::collection::vec(any_unit(), 0..8),
            cursor in prop::option::of(any_cursor()),
            path in prop::collection::vec(any_pos(), 0..6),
        ) -> MapScene {
            MapScene { origin, size, tiles, units, cursor, path }
        }
    }

    proptest! {
        /// Whatever the scene and the area (even one partly off the
        /// console), the skin changes no cell outside the area and adds no
        /// overlay outside its pixels.
        #[test]
        fn glyph_skin_paints_only_the_area(
            scene in any_scene(),
            area in (-6..104i32, -4..34i32, -2..110i32, -2..40i32),
        ) {
            let c = ctx();
            let area = Rect::new(area.0, area.1, area.2, area.3);
            let mut buf = blank();
            GlyphSkin.paint(&c, &scene, area, &mut buf);
            for y in 0..32 {
                for x in 0..100 {
                    if !area.contains(x, y) {
                        prop_assert_eq!(buf.get(x, y), Some(&fill()), "cell ({}, {})", x, y);
                    }
                }
            }
            let px = px_rect(area);
            for o in buf.overlays() {
                prop_assert_eq!(o.rect.intersect(&px), Some(o.rect), "{:?} in {:?}", o, area);
            }
        }
    }
}
