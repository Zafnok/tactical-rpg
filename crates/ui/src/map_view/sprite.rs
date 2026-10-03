//! The sprite skin (ADR-0038): the battle map painted from a tileset
//! (`assets/tilesets/<id>.ron` and its image, `trpg_content::tileset`).
//! A tile is the tileset's `tile_px`; that size is known only here, behind
//! [`SpriteSkin::tile_size`].
//!
//! Only the debug menu switches to it, with the generated test tileset
//! (ticket 0433). What the art alone doesn't show is drawn with
//! **placeholders**, Claude's and not decided (ticket 0039 decides them,
//! 0436 builds them): a bar in the unit's side's colour along the top of
//! its tile (its two-letter label isn't drawn); the HP bar along the
//! bottom, as on the glyph skin; a small mark in the effect colour in the
//! top-right corner for a unit under a timed effect; a light square behind
//! a unit a battle note picks out; and the cursor's corner marks just
//! inside the tile.

use trpg_content::{ImageRect, Picture, Tileset};
use trpg_core::Pos;

use super::glyph::cursor::GLOW_MAX;
use super::glyph::units::{ACTED_DIM, HP_BAR_H};
use super::glyph::{OVERLAY_BLEND, named, range_color};
use super::grid::{Grid, px_rect};
use super::path;
use super::scene::{CursorStyle, CursorView, MapScene, TileView, UnitView};
use super::skin::MapSkin;
use crate::color::{Rgb, UiColor, to_channel};
use crate::glyph_buffer::{GlyphBuffer, Layer, Overlay, PxRect, Rect, Sprite};
use crate::screen::Ctx;
use crate::screens::battle::units::{faction_color, hp_fill};

/// The faction bar's thickness, in pixels, along the top of a unit's tile
/// (a placeholder). *Tunable.*
pub const FACTION_BAR_H: i32 = 2;

/// The side of the effect mark's square, in pixels, in the top-right
/// corner of a unit's tile (a placeholder). *Tunable.*
pub const EFFECT_MARK: i32 = 3;

/// The battle map painted from a tileset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSkin {
    tileset: Tileset,
}

impl SpriteSkin {
    /// The skin of `tileset`.
    pub const fn new(tileset: Tileset) -> Self {
        Self { tileset }
    }

    /// Its tileset.
    pub const fn tileset(&self) -> &Tileset {
        &self.tileset
    }

    /// A map tile's size on screen, across × down, in console pixels:
    /// everything that depends on the tile size goes through this.
    pub fn tile_size(&self) -> (i32, i32) {
        let (w, h) = self.tileset.tile_px;
        (side(w), side(h))
    }

    /// Where `scene`'s tiles go in `area`: as many as fit, the pixels left
    /// over split evenly around them.
    pub fn grid(&self, scene: &MapScene, area: Rect) -> Grid {
        let px = px_rect(area);
        let tile = self.tile_size();
        let fit = self.view_tiles(area);
        let margin = |len: i32, n: i32, side: i32| (len.max(0) - n * side) / 2;
        Grid {
            origin: scene.origin,
            corner: (
                px.x + margin(px.w, fit.0, tile.0),
                px.y + margin(px.h, fit.1, tile.1),
            ),
            tile,
            tiles: (scene.size.0.clamp(0, fit.0), scene.size.1.clamp(0, fit.1)),
        }
    }

    /// Paints every tile of `scene` in `grid`: its terrain, tinted by its
    /// flashes, its ranges and the cursor's glow, in that order; then what
    /// a spell would turn it into, over it all.
    fn paint_tiles(&self, ctx: &Ctx, scene: &MapScene, grid: &Grid, buf: &mut GlyphBuffer) {
        let black = ctx.palette.get(UiColor::Black);
        for dy in 0..grid.tiles.1 {
            for dx in 0..grid.tiles.0 {
                let Some(tile) = scene.tile_at(dx, dy) else {
                    continue;
                };
                let pos = Pos::new(scene.origin.x + dx, scene.origin.y + dy);
                let rect = grid.rect_at(dx, dy);
                let tints = tints(ctx, tile, scene.cursor.filter(|c| c.pos == pos));
                let src = tile.terrain.and_then(|id| self.tileset.tile(id));
                paint_tinted(
                    buf,
                    rect,
                    src.map(|s| (self.tileset.image, s)),
                    &tints,
                    black,
                );
                if let Some(src) = tile.becomes.and_then(|id| self.tileset.tile(id)) {
                    buf.add_sprite(tile_sprite(self.tileset.image, src, rect));
                }
            }
        }
    }

    /// Paints `unit` on the pixels `tile`, nothing outside `view`: a light
    /// square if it is picked out, its picture, and the placeholders for
    /// what the picture doesn't show.
    fn paint_unit(
        &self,
        ctx: &Ctx,
        unit: &UnitView,
        tile: PxRect,
        view: PxRect,
        buf: &mut GlyphBuffer,
    ) {
        let color = |c| ctx.palette.get(c);
        if unit.highlight {
            buf.add_overlay(Overlay::new(tile, color(UiColor::Text), Layer::Under));
        }
        let picture = self
            .tileset
            .unit_picture(unit.character.as_ref(), &unit.class);
        let mut sprite = Sprite::new(
            picture.image,
            src_rect(picture.rect),
            unit_dest(picture, tile),
            Layer::Under,
        );
        sprite.opacity = unit_opacity(unit);
        if let Some(clip) = sprite.dest.intersect(&view) {
            sprite.clip = clip;
            buf.add_sprite(sprite);
        }
        if fade(unit) >= 0.5 {
            // Its marks go with the first half of its fall, as the glyph
            // skin's label and HP bar do.
            return;
        }
        let bar = Rect::new(tile.x, tile.y, tile.w, FACTION_BAR_H);
        let faction = color(faction_color(unit.faction));
        buf.add_overlay(Overlay::new(bar, faction, Layer::Over));
        if unit.has_effect {
            let x = tile.x + tile.w - EFFECT_MARK;
            let mark = Rect::new(x, tile.y, EFFECT_MARK, EFFECT_MARK);
            buf.add_overlay(Overlay::new(mark, color(UiColor::Effect), Layer::Over));
        }
        let (filled, hp) = hp_fill(unit.hp.0, unit.hp.1, tile.w);
        let y = tile.y + tile.h - HP_BAR_H;
        let full = Rect::new(tile.x, y, filled, HP_BAR_H);
        let empty = Rect::new(tile.x + filled, y, tile.w - filled, HP_BAR_H);
        // An empty part (full or zero HP) is dropped by `add_overlay`.
        buf.add_overlay(Overlay::new(full, color(hp), Layer::Over));
        buf.add_overlay(Overlay::new(empty, color(UiColor::Black), Layer::Over));
    }
}

impl MapSkin for SpriteSkin {
    fn name(&self) -> &'static str {
        "sprite"
    }

    /// The area's pixels divided by the tile size, rounded down.
    fn view_tiles(&self, area: Rect) -> (i32, i32) {
        let px = px_rect(area);
        let (w, h) = self.tile_size();
        (px.w.max(0) / w, px.h.max(0) / h)
    }

    /// Terrain (tinted by flashes, ranges and the cursor's glow), the path
    /// line, the units, the path's arrowhead, then the cursor's corners.
    fn paint(&self, ctx: &Ctx, scene: &MapScene, area: Rect, buf: &mut GlyphBuffer) {
        let grid = self.grid(scene, area);
        self.paint_tiles(ctx, scene, &grid, buf);
        let color = ctx.palette.get(UiColor::Path);
        let (line, arrowhead): (Vec<Overlay>, Vec<Overlay>) =
            path::path_overlays(&scene.path, &grid, color)
                .into_iter()
                .partition(|o| o.layer == Layer::Under);
        for overlay in line {
            buf.add_overlay(overlay);
        }
        for unit in &scene.units {
            if let Some(tile) = grid.rect(unit.pos) {
                self.paint_unit(ctx, unit, tile, grid.bounds(), buf);
            }
        }
        for overlay in arrowhead {
            buf.add_overlay(overlay);
        }
        if let Some(c) = &scene.cursor
            && let Some(tile) = grid.rect(c.pos)
        {
            let color = ctx.palette.get(UiColor::Cursor).scale(c.brightness);
            let arm = match c.style {
                CursorStyle::Corners => 3,
                CursorStyle::LargeCorners => 4,
                // Tinted with the tile.
                CursorStyle::TileGlow => 0,
            };
            for r in corner_arms(tile, arm) {
                buf.add_overlay(Overlay::new(r, color, Layer::Over));
            }
        }
    }

    fn tile_px(&self, scene: &MapScene, area: Rect, tile: Pos) -> Option<PxRect> {
        self.grid(scene, area).rect(tile)
    }
}

/// A tileset size as console pixels.
fn side(px: u32) -> i32 {
    i32::try_from(px).unwrap_or(i32::MAX)
}

/// `rect` of an image as a sprite's `src`.
fn src_rect(rect: ImageRect) -> PxRect {
    Rect::new(side(rect.x), side(rect.y), side(rect.w), side(rect.h))
}

/// A solid sprite of tile `src` of `image` on the pixels `dest`.
fn tile_sprite(image: trpg_content::ImageId, src: ImageRect, dest: PxRect) -> Sprite {
    Sprite::new(image, src_rect(src), dest, Layer::Under)
}

/// Where a unit's `picture` goes on the pixels `tile`, at its own size:
/// centred across, its bottom on the tile's (a taller picture reaches into
/// the tile above).
fn unit_dest(picture: Picture, tile: PxRect) -> PxRect {
    let (w, h) = (side(picture.rect.w), side(picture.rect.h));
    Rect::new(tile.x + (tile.w - w) / 2, tile.y + tile.h - h, w, h)
}

/// How far `unit` has fallen, in `0..=1` (`0` if not a number).
fn fade(unit: &UnitView) -> f32 {
    if unit.fade.is_finite() {
        unit.fade.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The opacity of `unit`'s picture: solid when standing, dimmed by the
/// glyph skin's [`ACTED_DIM`] once it has acted, and fading out as it
/// falls.
pub fn unit_opacity(unit: &UnitView) -> u8 {
    let acted = if unit.acted { 1.0 - ACTED_DIM } else { 1.0 };
    to_channel(255.0 * acted * (1.0 - fade(unit)))
}

/// What tints `tile`, in order, as colours and strengths: its flashes
/// (towards its terrain's glyph colour, as on the glyph skin), its ranges,
/// then the glow of `cursor` (if it is on the tile and glows).
fn tints(ctx: &Ctx, tile: &TileView, cursor: Option<CursorView>) -> Vec<(Rgb, f32)> {
    let display = &ctx.content.terrain.display;
    let fg = tile
        .terrain
        .and_then(|id| display.get(id))
        .map(|t| named(&ctx.palette, &t.fg, UiColor::Text));
    let mut out: Vec<(Rgb, f32)> = match fg {
        Some(fg) => tile
            .flashes
            .iter()
            .map(|&s| (fg, OVERLAY_BLEND * s))
            .collect(),
        None => Vec::new(),
    };
    for &kind in &tile.tints {
        out.push((ctx.palette.get(range_color(kind)), OVERLAY_BLEND));
    }
    if let Some(c) = cursor.filter(|c| c.style == CursorStyle::TileGlow) {
        out.push((ctx.palette.get(UiColor::Cursor), GLOW_MAX * c.brightness));
    }
    out
}

/// Tinting a picture by each of `tints` in turn (as `Rgb::lerp` towards
/// the colour by the strength, the glyph skin's `blend_bg`) leaves
/// `kept × picture + added`: returns `kept` and `added`.
fn mix(tints: &[(Rgb, f32)]) -> (f32, [f32; 3]) {
    let mut kept = 1.0;
    let mut added = [0.0; 3];
    for &(color, t) in tints {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        kept *= 1.0 - t;
        for (a, c) in added.iter_mut().zip([color.r, color.g, color.b]) {
            *a = *a * (1.0 - t) + f32::from(c) * t;
        }
    }
    (kept, added)
}

/// Paints a tile on the pixels `rect`, tinted by `tints`: its picture
/// (`image` and `src`) at reduced opacity over a rectangle of the colours
/// it is tinted towards, so it reads as tinted; with no picture (off the
/// map), the tints over `base`.
fn paint_tinted(
    buf: &mut GlyphBuffer,
    rect: PxRect,
    picture: Option<(trpg_content::ImageId, ImageRect)>,
    tints: &[(Rgb, f32)],
    base: Rgb,
) {
    let (kept, added) = mix(tints);
    let rgb = |[r, g, b]: [f32; 3]| Rgb::new(to_channel(r), to_channel(g), to_channel(b));
    match picture {
        Some((image, src)) => {
            if kept < 1.0 {
                let under = added.map(|a| a / (1.0 - kept));
                buf.add_overlay(Overlay::new(rect, rgb(under), Layer::Under));
            }
            let mut sprite = tile_sprite(image, src, rect);
            sprite.opacity = to_channel(255.0 * kept);
            buf.add_sprite(sprite);
        }
        None if !tints.is_empty() => {
            let base = [base.r, base.g, base.b];
            let mut color = added;
            for (c, b) in color.iter_mut().zip(base) {
                *c += kept * f32::from(b);
            }
            buf.add_overlay(Overlay::new(rect, rgb(color), Layer::Under));
        }
        None => {}
    }
}

/// The eight 1 px arms, each `arm` px long, of corner marks just inside
/// the corners of the pixels `tile`. None for an `arm` of 0.
fn corner_arms(tile: PxRect, arm: i32) -> Vec<Rect> {
    if arm <= 0 {
        return Vec::new();
    }
    let (left, top) = (tile.x, tile.y);
    let (right, bottom) = (tile.x + tile.w - 1, tile.y + tile.h - 1);
    let across = |x0, y0| Rect::new(x0, y0, arm, 1);
    let down = |x0, y0| Rect::new(x0, y0, 1, arm);
    vec![
        across(left, top),
        down(left, top),
        across(right - arm + 1, top),
        down(right, top),
        across(left, bottom),
        down(left, bottom - arm + 1),
        across(right - arm + 1, bottom),
        down(right, bottom - arm + 1),
    ]
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use proptest::prelude::*;
    use trpg_core::{CharacterId, ClassId, Faction, TerrainId, UnitId};

    use super::*;
    use crate::glyph_buffer::{Cell, Item};
    use crate::map_view::glyph::tests::any_scene;
    use crate::map_view::scene::RangeKind;
    use crate::screen::tests::ctx;
    use crate::screens::battle::layout::MAP_VIEW;

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// The skin of the test tileset (24 × 24 tiles).
    fn skin(c: &Ctx) -> SpriteSkin {
        SpriteSkin::new(c.content.tilesets["test"].clone())
    }

    /// [`skin`] with `w × h` tiles.
    fn sized(c: &Ctx, w: u32, h: u32) -> SpriteSkin {
        let mut tileset = c.content.tilesets["test"].clone();
        tileset.tile_px = (w, h);
        SpriteSkin::new(tileset)
    }

    /// [`skin`] with a picture for the test lord: the top-left class's.
    fn with_lord(c: &Ctx) -> SpriteSkin {
        let mut tileset = c.content.tilesets["test"].clone();
        let mut lord = tileset.fallback;
        lord.rect.x = 0;
        tileset
            .characters
            .insert(CharacterId("test_lord".into()), lord);
        SpriteSkin::new(tileset)
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
            highlight: false,
        }
    }

    /// A 3 × 2 view from (0, 0): plains, but a forest at (1, 0).
    fn plains(c: &Ctx) -> MapScene {
        let display = &c.content.terrain.display;
        let mut scene = MapScene::new(p(0, 0), (3, 2));
        for tile in &mut scene.tiles {
            tile.terrain = display.id_of("plain");
        }
        scene.tile_mut(p(1, 0)).unwrap().terrain = display.id_of("forest");
        scene
    }

    /// `scene` painted by `skin` into a blank console, in the 6 × 4 cells
    /// from (2, 1): 48 × 64 px from (16, 16), so 2 × 2 tiles of 24 px with
    /// 8 px to spare above and below.
    fn painted(c: &Ctx, skin: &SpriteSkin, scene: &MapScene) -> GlyphBuffer {
        let mut buf = blank();
        skin.paint(c, scene, Rect::new(2, 1, 6, 4), &mut buf);
        buf
    }

    /// The item at `i` as a rectangle.
    fn rect_at(buf: &GlyphBuffer, i: usize) -> Overlay {
        match buf.items()[i] {
            Item::Rect(o) => o,
            Item::Sprite(s) => panic!("item {i} is a sprite: {s:?}"),
        }
    }

    /// The item at `i` as a sprite.
    fn sprite_at(buf: &GlyphBuffer, i: usize) -> Sprite {
        match buf.items()[i] {
            Item::Sprite(s) => s,
            Item::Rect(o) => panic!("item {i} is a rectangle: {o:?}"),
        }
    }

    #[test]
    fn view_tiles_and_the_margin_follow_the_tile_size() {
        let c = ctx();
        let s = skin(&c);
        assert_eq!((s.name(), s.tile_size()), ("sprite", (24, 24)));
        assert_eq!(s.tileset().id, "test");
        // The map area is 560 × 480 px.
        let fit = |w, h, area| {
            let s = sized(&c, w, h);
            let scene = MapScene::new(p(0, 0), s.view_tiles(area));
            let g = s.grid(&scene, area);
            (g.tiles, g.corner)
        };
        // 552 × 480 px of 24 px tiles: 4 px to spare each side.
        assert_eq!(fit(24, 24, MAP_VIEW), ((23, 20), (4, 0)));
        assert_eq!(fit(16, 16, MAP_VIEW), ((35, 30), (0, 0)));
        assert_eq!(fit(64, 64, MAP_VIEW), ((8, 7), (24, 16)));
        // 9 × 13: 2 px to spare across, 12 down.
        assert_eq!(fit(9, 13, MAP_VIEW), ((62, 36), (1, 6)));
        // One cell: no tile across; the odd pixel goes right.
        assert_eq!(fit(9, 13, Rect::new(0, 0, 1, 1)), ((0, 1), (4, 1)));
        // An area away from the corner: 56 × 32 px from (32, 48).
        assert_eq!(fit(24, 24, Rect::new(4, 3, 7, 2)), ((2, 1), (36, 52)));
        // An empty area shows nothing.
        let empty = Rect::new(3, 3, -4, -1);
        assert_eq!(s.view_tiles(empty), (0, 0));
        assert_eq!(s.grid(&MapScene::new(p(0, 0), (5, 5)), empty).tiles, (0, 0));
        // Where the skin has a tile, for menus beside it.
        let scene = MapScene::new(p(-3, 2), s.view_tiles(MAP_VIEW));
        let px = |tile| s.tile_px(&scene, MAP_VIEW, tile);
        assert_eq!(px(p(-3, 2)), Some(Rect::new(4, 0, 24, 24)));
        assert_eq!(px(p(0, 3)), Some(Rect::new(76, 24, 24, 24)));
        assert_eq!(px(p(20, 2)), None);
        let cells = s.tile_cells(&scene, MAP_VIEW, p(0, 3));
        assert_eq!(cells, Some(Rect::new(9, 1, 4, 2)));
        // A scene smaller than the area: only its tiles.
        let small = s.grid(&MapScene::new(p(0, 0), (2, 1)), MAP_VIEW);
        assert_eq!(small.tiles, (2, 1));
    }

    #[test]
    fn a_tile_is_its_terrain_sprite() {
        let c = ctx();
        let s = skin(&c);
        let buf = painted(&c, &s, &plains(&c));
        let display = &c.content.terrain.display;
        let tile = |name| src_rect(s.tileset.tile(display.id_of(name).unwrap()).unwrap());
        let sprites = buf.sprites();
        // 2 × 2 tiles fit, from (16, 24).
        assert_eq!(sprites.len(), 4);
        let at = |x, y| Rect::new(x, y, 24, 24);
        let expect = [
            (tile("plain"), at(16, 24)),
            (tile("forest"), at(40, 24)),
            (tile("plain"), at(16, 48)),
            (tile("plain"), at(40, 48)),
        ];
        for (sprite, (src, dest)) in sprites.iter().zip(expect) {
            assert_eq!((sprite.src, sprite.dest, sprite.clip), (src, dest, dest));
            assert_eq!(sprite.image.path(), "tilesets/test.png");
            let look = (sprite.layer, sprite.opacity, sprite.flip_x);
            assert_eq!(look, (Layer::Under, 255, false));
        }
        assert_ne!(tile("plain"), tile("forest"));
        // No cell changes.
        let cells = |b: &GlyphBuffer| {
            let all = (0..32).flat_map(|y| (0..100).map(move |x| (x, y)));
            all.map(|(x, y)| *b.get(x, y).unwrap()).collect::<Vec<_>>()
        };
        assert_eq!(cells(&buf), cells(&blank()));
        // Tiles off the map, or of a terrain the tileset lacks: nothing.
        let mut scene = plains(&c);
        scene.tiles[0].terrain = None;
        scene.tiles[1].terrain = Some(TerrainId(999));
        assert_eq!(painted(&c, &s, &scene).sprites().len(), 2);
    }

    #[test]
    fn tints_mix_as_the_glyph_skin_blends() {
        let red = Rgb::new(200, 0, 0);
        let blue = Rgb::new(0, 0, 255);
        assert_eq!(mix(&[]), (1.0, [0.0; 3]));
        let (kept, added) = mix(&[(red, 0.75)]);
        assert!((kept - 0.25).abs() < 1e-6);
        assert!((added[0] - 150.0).abs() < 1e-4 && added[1].abs() < 1e-6);
        // Bad strengths: none (NaN) or clamped.
        assert_eq!(mix(&[(red, f32::NAN)]), (1.0, [0.0; 3]));
        assert_eq!(mix(&[(red, -1.0)]), (1.0, [0.0; 3]));
        assert_eq!(mix(&[(red, 2.0)]), mix(&[(red, 1.0)]));
        // Whatever the tile's colour, `kept × tile + added` is the tile
        // lerped towards each tint in turn.
        let tints = [(red, 0.75), (blue, 0.75), (Rgb::new(9, 200, 9), 0.3)];
        let (kept, added) = mix(&tints);
        for tile in [
            Rgb::new(0, 0, 0),
            Rgb::new(255, 255, 255),
            Rgb::new(30, 140, 60),
        ] {
            let glyph = tints.iter().fold(tile, |bg, &(c, t)| bg.lerp(c, t));
            let channels = [tile.r, tile.g, tile.b].into_iter().zip(added);
            let ours: Vec<f32> = channels.map(|(v, a)| kept * f32::from(v) + a).collect();
            for (o, g) in ours.iter().zip([glyph.r, glyph.g, glyph.b]) {
                assert!(
                    (o - f32::from(g)).abs() <= 1.5,
                    "{tile:?}: {ours:?} vs {glyph:?}"
                );
            }
        }
    }

    #[test]
    fn a_tinted_tile_is_its_sprite_faded_over_the_tint() {
        let c = ctx();
        let s = skin(&c);
        let pal = &c.palette;
        let mut scene = plains(&c);
        scene.tint([p(0, 0)], RangeKind::Move);
        scene.tint([p(1, 0)], RangeKind::Danger);
        scene.tint([p(1, 0)], RangeKind::Attack);
        let buf = painted(&c, &s, &scene);
        // A rectangle of the range's colour, then the tile at a quarter.
        let under = rect_at(&buf, 0);
        assert_eq!(under.rect, Rect::new(16, 24, 24, 24));
        let move_range = pal.get(UiColor::MoveRange);
        assert_eq!((under.color, under.layer), (move_range, Layer::Under));
        let tile = sprite_at(&buf, 1);
        assert_eq!((tile.dest, tile.opacity), (under.rect, 64));
        // Two ranges: their mix under a sixteenth of the tile.
        let both = rect_at(&buf, 2);
        let (danger, attack) = (pal.get(UiColor::DangerZone), pal.get(UiColor::AttackRange));
        assert_eq!(both.color, danger.lerp(attack, 0.8));
        assert_eq!(sprite_at(&buf, 3).opacity, 16);
        // Off the map, a range tints the black under it.
        let mut scene = plains(&c);
        scene.tiles[0].terrain = None;
        scene.tint([p(0, 0)], RangeKind::Heal);
        let off = rect_at(&painted(&c, &s, &scene), 0);
        let black = pal.get(UiColor::Black);
        let heal = pal.get(UiColor::HealRange);
        assert_eq!(off.color, black.lerp(heal, OVERLAY_BLEND));
        // Whatever is under it: here a colour that isn't black.
        let base = Rgb::new(200, 100, 40);
        let red = Rgb::new(250, 0, 0);
        let rect = Rect::new(8, 16, 24, 24);
        let mut buf = blank();
        paint_tinted(&mut buf, rect, None, &[(red, 0.5)], base);
        let over = rect_at(&buf, 0);
        assert_eq!((over.rect, over.color), (rect, Rgb::new(225, 50, 20)));
        assert_eq!(over.color, base.lerp(red, 0.5));
        // No tints, no picture: nothing.
        paint_tinted(&mut buf, rect, None, &[], base);
        assert_eq!(buf.items().len(), 1);
    }

    #[test]
    fn flashes_tint_towards_the_terrain_colour_and_becoming_covers_it_all() {
        let c = ctx();
        let s = skin(&c);
        let pal = &c.palette;
        let display = &c.content.terrain.display;
        let plain = display.get(display.id_of("plain").unwrap()).unwrap();
        let fg = pal.lookup(&plain.fg).unwrap();
        let mut scene = plains(&c);
        scene.tiles[0].flashes = vec![1.0];
        scene.tint([p(0, 0)], RangeKind::Attack);
        scene.tiles[0].becomes = display.id_of("forest");
        // Off the map, a flash has no colour to go towards: nothing.
        scene.tiles[3].terrain = None;
        scene.tiles[3].flashes = vec![1.0];
        // What the tileset lacks becomes nothing.
        scene.tiles[4].becomes = Some(TerrainId(999));
        let buf = painted(&c, &s, &scene);
        let under = rect_at(&buf, 0);
        let attack = pal.get(UiColor::AttackRange);
        let (kept, added) = mix(&[(fg, OVERLAY_BLEND), (attack, OVERLAY_BLEND)]);
        let expect = added.map(|a| to_channel(a / (1.0 - kept)));
        assert_eq!([under.color.r, under.color.g, under.color.b], expect);
        assert_ne!(under.color, attack);
        // Then the tile, then the forest it would become, solid, over it.
        let forest = s.tileset.tile(display.id_of("forest").unwrap()).unwrap();
        let becomes = sprite_at(&buf, 2);
        assert_eq!((becomes.src, becomes.opacity), (src_rect(forest), 255));
        assert_eq!(becomes.dest, under.rect);
        // Then (1, 0), the forest, and (1, 1): (0, 1) is off the map.
        assert_eq!(buf.items().len(), 3 + 2);
    }

    #[test]
    fn a_unit_is_its_picture_and_the_placeholders() {
        let c = ctx();
        let s = skin(&c);
        let pal = &c.palette;
        let mut scene = plains(&c);
        let mut unit = brigand(p(1, 1));
        unit.hp = (20, 30);
        unit.has_effect = true;
        scene.push_unit(unit);
        let buf = painted(&c, &s, &scene);
        let tile = Rect::new(40, 48, 24, 24);
        let class = s.tileset.unit_picture(None, &ClassId("brigand".into()));
        let picture = sprite_at(&buf, 4);
        assert_eq!(picture.src, src_rect(class.rect));
        let look = (picture.dest, picture.layer, picture.opacity);
        assert_eq!(look, (tile, Layer::Under, 255));
        let rects: Vec<(Rect, Rgb, Layer)> = (5..buf.items().len())
            .map(|i| rect_at(&buf, i))
            .map(|o| (o.rect, o.color, o.layer))
            .collect();
        let over = Layer::Over;
        assert_eq!(
            rects,
            [
                // Its side along the top.
                (Rect::new(40, 48, 24, 2), pal.get(UiColor::Enemy), over),
                // The effect mark in the top-right corner.
                (Rect::new(61, 48, 3, 3), pal.get(UiColor::Effect), over),
                // 20 / 30 HP: 16 of 24 px, along the bottom.
                (Rect::new(40, 70, 16, 2), pal.get(UiColor::HpMid), over),
                (Rect::new(56, 70, 8, 2), pal.get(UiColor::Black), over),
            ]
        );
        // A unit off the view: nothing.
        let mut scene = plains(&c);
        scene.units.push(brigand(p(2, 0)));
        assert_eq!(painted(&c, &s, &scene).items().len(), 4);
    }

    #[test]
    fn a_picture_is_the_characters_else_its_class_else_the_fallback() {
        let c = ctx();
        let s = with_lord(&c);
        let src = |unit: UnitView| {
            let mut scene = plains(&c);
            scene.push_unit(unit);
            sprite_at(&painted(&c, &s, &scene), 4).src
        };
        let lord = CharacterId("test_lord".into());
        let mut named = brigand(p(0, 0));
        named.character = Some(lord.clone());
        assert_eq!(
            src(named.clone()),
            src_rect(s.tileset.characters[&lord].rect)
        );
        named.character = Some(CharacterId("nobody".into()));
        let class = s.tileset.classes[&ClassId("brigand".into())];
        assert_eq!(src(named.clone()), src_rect(class.rect));
        named.class = ClassId("no_such_class".into());
        assert_eq!(src(named), src_rect(s.tileset.fallback.rect));
        assert_ne!(class.rect, s.tileset.fallback.rect);
    }

    #[test]
    fn a_picture_stands_on_the_bottom_of_its_tile_centred_and_clipped_to_the_view() {
        let c = ctx();
        let image = c.content.tilesets["test"].image;
        let picture = |w, h| Picture {
            image,
            rect: ImageRect { x: 0, y: 0, w, h },
        };
        let tile = Rect::new(40, 48, 24, 24);
        assert_eq!(unit_dest(picture(24, 24), tile), tile);
        // Taller: into the tile above; narrower: centred.
        assert_eq!(unit_dest(picture(16, 30), tile), Rect::new(44, 42, 16, 30));
        assert_eq!(unit_dest(picture(17, 24), tile), Rect::new(43, 48, 17, 24));
        // A tall picture on the top row is cut at the view's edge.
        let mut tileset = c.content.tilesets["test"].clone();
        tileset.classes.clear();
        tileset.fallback.rect.h = 30;
        tileset.fallback.rect.y -= 6;
        let s = SpriteSkin::new(tileset);
        let mut scene = plains(&c);
        scene.push_unit(brigand(p(0, 0)));
        let unit = sprite_at(&painted(&c, &s, &scene), 4);
        assert_eq!(unit.dest, Rect::new(16, 18, 24, 30));
        assert_eq!(unit.clip, Rect::new(16, 24, 24, 24));
    }

    #[test]
    fn acted_and_falling_units_fade() {
        let standing = brigand(p(0, 0));
        assert_eq!(unit_opacity(&standing), 255);
        let acted = UnitView {
            acted: true,
            ..standing.clone()
        };
        assert_eq!(unit_opacity(&acted), 128);
        let fading = |u: &UnitView, fade| unit_opacity(&u.clone().fading(fade));
        assert_eq!(fading(&standing, 0.25), 191);
        assert_eq!(fading(&acted, 0.5), 64);
        assert_eq!(fading(&standing, 1.0), 0);
        assert_eq!(fading(&standing, 7.0), 0);
        // Not a number, or below 0: standing.
        assert_eq!(fading(&standing, f32::NAN), 255);
        assert_eq!(fading(&standing, f32::INFINITY), 255);
        assert_eq!(fading(&standing, -1.0), 255);
        // The marks go at half way.
        let c = ctx();
        let s = skin(&c);
        let marks = |fade| {
            let mut scene = plains(&c);
            scene.push_unit(brigand(p(0, 0)).fading(fade));
            painted(&c, &s, &scene).overlays().len()
        };
        assert_eq!(
            (marks(0.0), marks(0.49), marks(0.5), marks(1.0)),
            (2, 2, 0, 0)
        );
        let mut scene = plains(&c);
        scene.push_unit(acted.fading(0.25));
        assert_eq!(sprite_at(&painted(&c, &s, &scene), 4).opacity, 96);
    }

    #[test]
    fn a_picked_out_unit_stands_on_a_light_square() {
        let c = ctx();
        let s = skin(&c);
        let mut scene = plains(&c);
        scene.push_unit(brigand(p(0, 0)).highlighted(true));
        let buf = painted(&c, &s, &scene);
        let square = rect_at(&buf, 4);
        assert_eq!(square.rect, Rect::new(16, 24, 24, 24));
        let text = c.palette.get(UiColor::Text);
        assert_eq!((square.color, square.layer), (text, Layer::Under));
        assert_eq!(sprite_at(&buf, 5).dest, square.rect);
    }

    #[test]
    fn the_cursor_marks_the_corners_inside_its_tile_or_glows() {
        let c = ctx();
        let s = skin(&c);
        let cursor = c.palette.get(UiColor::Cursor);
        let painted_with = |style, brightness| {
            let mut scene = plains(&c);
            scene.cursor = Some(CursorView {
                pos: p(1, 1),
                brightness,
                style,
            });
            painted(&c, &s, &scene)
        };
        let buf = painted_with(CursorStyle::Corners, 1.0);
        let r = Rect::new;
        let arms: Vec<_> = buf.overlays().iter().map(|o| o.rect).collect();
        // Tile (1, 1): pixels 40..64 × 48..72.
        assert_eq!(
            arms,
            [
                r(40, 48, 3, 1),
                r(40, 48, 1, 3),
                r(61, 48, 3, 1),
                r(63, 48, 1, 3),
                r(40, 71, 3, 1),
                r(40, 69, 1, 3),
                r(61, 71, 3, 1),
                r(63, 69, 1, 3),
            ]
        );
        let look = |o: &Overlay| (o.color, o.layer);
        assert!(
            buf.overlays()
                .iter()
                .all(|o| look(o) == (cursor, Layer::Over))
        );
        let large = painted_with(CursorStyle::LargeCorners, 0.5);
        assert_eq!(large.overlays()[2].rect, r(60, 48, 4, 1));
        assert_eq!(large.overlays()[0].color, cursor.scale(0.5));
        // The glow: the tile faded over the cursor colour, no marks.
        let glow = painted_with(CursorStyle::TileGlow, 1.0);
        let under = rect_at(&glow, 3);
        assert_eq!((under.rect, under.color), (r(40, 48, 24, 24), cursor));
        let opacity = to_channel(255.0 * (1.0 - GLOW_MAX));
        assert_eq!(sprite_at(&glow, 4).opacity, opacity);
        assert_eq!(glow.items().len(), 4 + 1);
        let dim = painted_with(CursorStyle::TileGlow, 0.5);
        let opacity = to_channel(255.0 * (1.0 - GLOW_MAX * 0.5));
        assert_eq!(sprite_at(&dim, 4).opacity, opacity);
        assert!(corner_arms(r(0, 0, 8, 8), 0).is_empty());
        // A cursor off the view: nothing.
        let mut scene = plains(&c);
        scene.cursor = Some(CursorView {
            pos: p(2, 0),
            brightness: 1.0,
            style: CursorStyle::TileGlow,
        });
        assert_eq!(painted(&c, &s, &scene), painted(&c, &s, &plains(&c)));
    }

    #[test]
    fn the_path_runs_under_units_and_its_arrowhead_over_them() {
        let c = ctx();
        let s = skin(&c);
        let mut scene = plains(&c);
        scene.push_unit(brigand(p(1, 1)));
        scene.path = vec![p(0, 1), p(1, 1)];
        let buf = painted(&c, &s, &scene);
        let path = c.palette.get(UiColor::Path);
        let kinds: Vec<&str> = buf
            .items()
            .iter()
            .map(|i| match i {
                Item::Sprite(_) => "sprite",
                Item::Rect(o) if o.color == path => "path",
                Item::Rect(_) => "mark",
            })
            .collect();
        let mut expect = vec!["sprite"; 4];
        expect.extend(["path", "sprite", "mark", "mark"]);
        expect.extend(["path"; 6]);
        assert_eq!(kinds, expect);
        // Through the centres of 24 px tiles: pixels 11..=13 of each.
        let line = rect_at(&buf, 4);
        assert_eq!(
            (line.rect, line.layer),
            (Rect::new(40, 59, 14, 3), Layer::Under)
        );
    }

    /// A small scene with one of everything, painted: the frame's items.
    #[test]
    fn sprite_skin_snapshot() {
        let c = ctx();
        let s = skin(&c);
        let mut scene = plains(&c);
        scene.tint([p(0, 0)], RangeKind::Move);
        scene.tile_mut(p(1, 0)).unwrap().flashes.push(0.5);
        let mut lord = brigand(p(0, 0));
        lord.faction = Faction::Player;
        lord.class = ClassId("exile".into());
        lord.hp = (5, 30);
        scene.push_unit(lord);
        let mut enemy = brigand(p(1, 1));
        enemy.acted = true;
        enemy.has_effect = true;
        scene.push_unit(enemy);
        scene.path = vec![p(0, 0), p(0, 1)];
        scene.cursor = Some(CursorView {
            pos: p(0, 1),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        let text = painted(&c, &s, &scene).to_snapshot(&c.palette);
        let items = text.split("--- overlays ---\n").nth(1).unwrap_or_default();
        assert_snapshot!(items);
    }

    /// One change to a scene the skin must show.
    struct Feature {
        name: &'static str,
        without: MapScene,
        with: MapScene,
    }

    /// Every feature of a [`MapScene`], each as a scene without it and the
    /// same scene with it. Every field of the scene is listed below, so a
    /// new one doesn't compile until it is listed here too, with a feature
    /// or the reason it has none.
    fn features(c: &Ctx) -> Vec<Feature> {
        let display = &c.content.terrain.display;
        let mut base = plains(c);
        base.push_unit(brigand(p(1, 1)));
        let MapScene {
            origin: _, // where the tiles are: every feature moves with it
            size: _,   // how many tiles: likewise
            tiles: _,
            units: _,
            cursor: _,
            path: _,
        } = &base;
        let TileView {
            terrain: _,
            flashes: _,
            tints: _,
            becomes: _,
        } = &base.tiles[0];
        let UnitView {
            id: _,    // which unit it is: not shown
            label: _, // the glyph skin's; this skin shows the picture instead
            pos: _,
            faction: _,
            class: _,
            character: _,
            acted: _,
            hp: _,
            has_effect: _,
            fade: _,
            highlight: _,
        } = &base.units[0];
        let CursorView {
            pos: _,
            brightness: _,
            style: _,
        } = CursorView {
            pos: p(0, 0),
            brightness: 1.0,
            style: CursorStyle::Corners,
        };
        let feature = |name, change: &dyn Fn(&mut MapScene)| {
            let mut with = base.clone();
            change(&mut with);
            Feature {
                name,
                without: base.clone(),
                with,
            }
        };
        let unit = |change: fn(&mut UnitView)| move |s: &mut MapScene| change(&mut s.units[0]);
        let mut out = vec![
            feature("terrain", &|s| s.tiles[0].terrain = display.id_of("water")),
            feature("flash", &|s| s.tiles[0].flashes.push(0.5)),
            feature("becomes", &|s| s.tiles[0].becomes = display.id_of("water")),
            feature("unit pos", &unit(|u| u.pos = p(0, 1))),
            feature("faction", &unit(|u| u.faction = Faction::Player)),
            feature("class", &unit(|u| u.class = ClassId("mage".into()))),
            feature(
                "character",
                &unit(|u| u.character = Some(CharacterId("test_lord".into()))),
            ),
            feature("acted", &unit(|u| u.acted = true)),
            feature("hp", &unit(|u| u.hp.0 = 10)),
            feature("effect", &unit(|u| u.has_effect = true)),
            feature("fade", &unit(|u| u.fade = 0.25)),
            feature("highlight", &unit(|u| u.highlight = true)),
            feature("path", &|s| s.path = vec![p(0, 0), p(1, 0)]),
        ];
        let ranges = [
            RangeKind::Danger,
            RangeKind::Move,
            RangeKind::Attack,
            RangeKind::Heal,
        ];
        for kind in ranges {
            // Doesn't compile once a range is added: list it above.
            match kind {
                RangeKind::Danger | RangeKind::Move | RangeKind::Attack | RangeKind::Heal => {}
            }
            out.push(feature("range", &move |s| s.tint([p(0, 0)], kind)));
        }
        let styles = [
            CursorStyle::Corners,
            CursorStyle::LargeCorners,
            CursorStyle::TileGlow,
        ];
        for style in styles {
            match style {
                CursorStyle::Corners | CursorStyle::LargeCorners | CursorStyle::TileGlow => {}
            }
            let at = move |brightness| CursorView {
                pos: p(0, 1),
                brightness,
                style,
            };
            out.push(feature("cursor", &move |s| s.cursor = Some(at(1.0))));
            let mut dim = feature("cursor brightness", &move |s| s.cursor = Some(at(0.5)));
            dim.without.cursor = Some(at(1.0));
            out.push(dim);
        }
        out
    }

    /// For each feature of a scene, the skin's frame with it differs from
    /// the frame without it: nothing in a scene goes unpainted (ADR-0038).
    #[test]
    fn every_scene_feature_is_painted() {
        let c = ctx();
        let s = with_lord(&c);
        let features = features(&c);
        assert_eq!(features.len(), 13 + 4 + 6);
        for f in features {
            assert_ne!(f.without, f.with, "{}: no change", f.name);
            let (without, with) = (painted(&c, &s, &f.without), painted(&c, &s, &f.with));
            assert_ne!(without, with, "{}: not painted", f.name);
        }
    }

    prop_compose! {
        fn any_area()(area in (-6..104i32, -4..34i32, -2..110i32, -2..40i32)) -> Rect {
            Rect::new(area.0, area.1, area.2, area.3)
        }
    }

    proptest! {
        /// Whatever the scene, the area (even one partly off the console)
        /// and the tile size, the skin changes no cell and adds no item
        /// (rectangle or sprite) outside the area's pixels.
        #[test]
        fn sprite_skin_paints_only_the_area(
            scene in any_scene(),
            area in any_area(),
            tile in (8..=64u32, 8..=64u32),
        ) {
            let c = ctx();
            let mut buf = blank();
            sized(&c, tile.0, tile.1).paint(&c, &scene, area, &mut buf);
            for y in 0..32 {
                for x in 0..100 {
                    prop_assert_eq!(buf.get(x, y), Some(&fill()), "cell ({}, {})", x, y);
                }
            }
            let px = px_rect(area);
            for item in buf.items() {
                let seen = item.visible();
                prop_assert_eq!(seen.intersect(&px), Some(seen), "{:?} in {:?}", item, area);
            }
        }

        /// For any tile size, every visible tile lies inside the area, and
        /// the tiles sit edge to edge without overlapping.
        #[test]
        fn tiles_lie_inside_the_area_edge_to_edge(
            area in any_area(),
            tile in (8..=64u32, 8..=64u32),
            origin in (-30..30i32, -30..30i32),
        ) {
            let c = ctx();
            let s = sized(&c, tile.0, tile.1);
            let (w, h) = (side(tile.0), side(tile.1));
            let scene = MapScene::new(Pos::new(origin.0, origin.1), s.view_tiles(area));
            let px = px_rect(area);
            let rect = |dx, dy| s.tile_px(&scene, area, Pos::new(origin.0 + dx, origin.1 + dy));
            for dy in 0..scene.size.1 {
                for dx in 0..scene.size.0 {
                    let r = rect(dx, dy);
                    prop_assert!(r.is_some());
                    let r = r.unwrap_or(Rect::new(0, 0, 0, 0));
                    prop_assert_eq!((r.w, r.h), (w, h));
                    prop_assert_eq!(r.intersect(&px), Some(r), "{:?} in {:?}", r, area);
                    if let Some(right) = rect(dx + 1, dy) {
                        prop_assert_eq!((right.x, right.y), (r.x + w, r.y));
                    }
                    if let Some(below) = rect(dx, dy + 1) {
                        prop_assert_eq!((below.x, below.y), (r.x, r.y + h));
                    }
                }
            }
            let (across, down) = scene.size;
            prop_assert!(rect(across, 0).is_none() && rect(0, down).is_none());
            prop_assert!(rect(-1, 0).is_none() && rect(0, -1).is_none());
        }
    }
}
