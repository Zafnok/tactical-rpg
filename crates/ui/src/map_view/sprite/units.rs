//! How a sprite skin draws units (`docs/design/look-and-feel.md`, *Battle
//! map: bought tiles and unit sprites*; ADR-0049): each unit's picture with
//! a 1-pixel outline in its side's colour, standing on its HP bar; grey and
//! darker once it has acted; an arrow in the top-right corner of its tile
//! while it is under a timed effect.
//!
//! Units are painted in two passes: every outline and picture, the top row
//! first, so a head overlaps the tile above and never the reverse; then
//! every HP bar and mark, so none is under a neighbour's head.

use trpg_content::image::EFFECT_MARKS_PATH;
use trpg_content::{Picture, Tileset};
use trpg_core::{CharacterId, Pos};

use super::super::glyph::units::{HP_BAR_H, HP_BAR_INSET};
use super::super::grid::Grid;
use super::super::scene::{MapScene, UnitView};
use super::{side, src_rect};
use crate::color::{UiColor, to_channel};
use crate::glyph_buffer::{GlyphBuffer, Layer, Overlay, Paint, PxRect, Rect, Sprite};
use crate::screen::Ctx;
use crate::screens::battle::units::{faction_color, hp_fill};

/// The side of an effect arrow's box in [`EFFECT_MARKS_PATH`], in pixels:
/// the 5 × 5 arrow and its 1-pixel dark edge.
pub const MARK_BOX: i32 = 7;

/// How far the arrow's box reaches past its tile's right edge, in pixels.
pub const MARK_PAST_RIGHT: i32 = 1;

/// How far the arrow's box reaches above its tile's top edge, in pixels.
pub const MARK_ABOVE: i32 = 3;

/// How much lower the arrow sits when it can't reach above its tile
/// (another unit stands there, or the tile is on the view's top row), in
/// pixels: inside its own tile.
pub const MARK_LOWERED: i32 = 4;

/// How long an arrow rests, and then how long it stays bounced (the up
/// arrow 1 pixel up, the down arrow 1 pixel down), in milliseconds.
/// *Tunable.*
pub const BOUNCE_MS: u64 = 375;

/// How long each arrow of a unit with both a bonus and a penalty shows
/// before the other takes its turn, in milliseconds: one rest and one
/// bounce. *Tunable.*
pub const TURN_MS: u64 = 2 * BOUNCE_MS;

/// An effect arrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mark {
    /// The up arrow: a bonus.
    Bonus,
    /// The down arrow: a penalty.
    Penalty,
}

impl Mark {
    /// Its box in [`EFFECT_MARKS_PATH`]: the up arrow, then the down arrow.
    const fn src(self) -> PxRect {
        let x = match self {
            Mark::Bonus => 0,
            Mark::Penalty => MARK_BOX,
        };
        Rect::new(x, 0, MARK_BOX, MARK_BOX)
    }

    /// Which way it bounces, in pixels down.
    const fn bounce(self) -> i32 {
        match self {
            Mark::Bonus => -1,
            Mark::Penalty => 1,
        }
    }
}

/// The arrow a unit with a `bonus`, a `penalty` or both shows at `clock_ms`
/// on the animation clock, and whether it is bounced: each arrow rests for
/// [`BOUNCE_MS`], then is bounced for as long; with both, the up arrow has
/// the first [`TURN_MS`] and the down arrow the next.
pub fn effect_mark(bonus: bool, penalty: bool, clock_ms: u64) -> Option<(Mark, bool)> {
    let mark = match (bonus, penalty) {
        (false, false) => return None,
        (true, false) => Mark::Bonus,
        (false, true) => Mark::Penalty,
        (true, true) => {
            if (clock_ms / TURN_MS).is_multiple_of(2) {
                Mark::Bonus
            } else {
                Mark::Penalty
            }
        }
    };
    Some((mark, clock_ms % TURN_MS >= BOUNCE_MS))
}

/// Where a unit's `picture` goes on the pixels `tile`, at its own size:
/// centred across, its feet on top of the HP bar (a taller picture reaches
/// into the tile above).
pub fn unit_dest(picture: Picture, tile: PxRect) -> PxRect {
    let (w, h) = (side(picture.rect.w), side(picture.rect.h));
    let bottom = tile.y + tile.h - HP_BAR_H;
    Rect::new(tile.x + (tile.w - w) / 2, bottom - h, w, h)
}

/// The four places a picture at `dest` is drawn in one colour to make its
/// outline: one pixel up, down, left and right.
pub fn outline_dests(dest: PxRect) -> [PxRect; 4] {
    [(0, -1), (0, 1), (-1, 0), (1, 0)].map(|(dx, dy)| Rect {
        x: dest.x + dx,
        y: dest.y + dy,
        ..dest
    })
}

/// A unit's HP bar on the pixels `tile`: the filled part and the empty
/// part, along the bottom, [`HP_BAR_INSET`] in from each side: the glyph
/// skin's bar (ticket 0441), for any tile size.
pub fn hp_bar(unit: &UnitView, tile: PxRect) -> [(PxRect, UiColor); 2] {
    let full = (tile.w - 2 * HP_BAR_INSET).max(0);
    let (filled, color) = hp_fill(unit.hp.0, unit.hp.1, full);
    let (x, y) = (tile.x + HP_BAR_INSET, tile.y + tile.h - HP_BAR_H);
    [
        (Rect::new(x, y, filled, HP_BAR_H), color),
        (
            Rect::new(x + filled, y, full - filled, HP_BAR_H),
            UiColor::Black,
        ),
    ]
}

/// Where `mark`'s box goes for a unit on the pixels `tile`: its top-right
/// corner on the tile's, [`MARK_PAST_RIGHT`] right and [`MARK_ABOVE`] up;
/// [`MARK_LOWERED`] lower if `lowered`; one pixel its own way if `bounced`.
pub fn mark_dest(mark: Mark, tile: PxRect, lowered: bool, bounced: bool) -> PxRect {
    let x = tile.x + tile.w + MARK_PAST_RIGHT - MARK_BOX;
    let mut y = tile.y - MARK_ABOVE;
    if lowered {
        y += MARK_LOWERED;
    }
    if bounced {
        y += mark.bounce();
    }
    Rect::new(x, y, MARK_BOX, MARK_BOX)
}

/// How far `unit` has fallen, in `0..=1` (`0` if not a number).
pub fn fade(unit: &UnitView) -> f32 {
    if unit.fade.is_finite() {
        unit.fade.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// The opacity of `unit`'s picture: solid when standing, fading out as it
/// falls.
pub fn unit_opacity(unit: &UnitView) -> u8 {
    to_channel(255.0 * (1.0 - fade(unit)))
}

/// The opacity of `unit`'s outline: gone by half way through its fall, as
/// its bar and marks are.
pub fn outline_opacity(unit: &UnitView) -> u8 {
    to_channel(255.0 * (1.0 - 2.0 * fade(unit)))
}

/// The ids `unit`'s picture is looked up by: the lead's by gender first
/// (`lead_m`, `lead_f`), then the character's own.
fn picture_ids(ctx: &Ctx, unit: &UnitView) -> Vec<CharacterId> {
    let Some(id) = &unit.character else {
        return Vec::new();
    };
    let by_gender = ctx.lead.portrait_for(&id.0);
    let mut ids = vec![id.clone()];
    if by_gender != id.0 {
        ids.insert(0, CharacterId(by_gender.to_owned()));
    }
    ids
}

/// Whether another unit of `scene` stands on the tile directly above
/// `unit`'s.
fn has_unit_above(scene: &MapScene, unit: &UnitView) -> bool {
    let above = Pos::new(unit.pos.x, unit.pos.y - 1);
    scene.units.iter().any(|u| u.pos == above)
}

/// Paints the outline and the picture of every unit of `scene` on a tile
/// of `grid`, the top row first: each in its side's colour (dimmed, with
/// the picture, once it has acted), nothing outside the grid, and nothing
/// above its own tile when another unit stands there.
pub fn paint_pictures(
    ctx: &Ctx,
    tileset: &Tileset,
    scene: &MapScene,
    grid: &Grid,
    buf: &mut GlyphBuffer,
) {
    let mut units: Vec<&UnitView> = scene.units.iter().collect();
    units.sort_by_key(|u| u.pos.y);
    for unit in units {
        let Some(tile) = grid.rect(unit.pos) else {
            continue;
        };
        let view = grid.bounds();
        let view = if has_unit_above(scene, unit) {
            let cut = tile.y - view.y;
            Rect::new(view.x, tile.y, view.w, view.h - cut)
        } else {
            view
        };
        let picture = tileset.unit_picture(&picture_ids(ctx, unit), &unit.class);
        let dest = unit_dest(picture, tile);
        let faction = ctx.palette.get(faction_color(unit.faction));
        let (color, paint) = if unit.acted {
            (faction.dimmed(), Paint::Dimmed)
        } else {
            (faction, Paint::Image)
        };
        let mut add = |dest: PxRect, paint: Paint, opacity: u8| {
            let mut sprite = Sprite::new(picture.image, src_rect(picture.rect), dest, Layer::Over);
            sprite.paint = paint;
            sprite.opacity = opacity;
            sprite.base = Some(tile);
            if opacity > 0
                && let Some(clip) = dest.intersect(&view)
            {
                sprite.clip = clip;
                buf.add_sprite(sprite);
            }
        };
        for outline in outline_dests(dest) {
            add(outline, Paint::Solid(color), outline_opacity(unit));
        }
        add(dest, paint, unit_opacity(unit));
    }
}

/// Paints the HP bar and the effect arrow of every unit of `scene` on a
/// tile of `grid`, over every picture. A unit half way through its fall
/// has neither.
pub fn paint_marks(ctx: &Ctx, scene: &MapScene, grid: &Grid, buf: &mut GlyphBuffer) {
    let arrows = ctx.content.images.id(EFFECT_MARKS_PATH);
    for unit in &scene.units {
        let Some(tile) = grid.rect(unit.pos) else {
            continue;
        };
        if fade(unit) >= 0.5 {
            continue;
        }
        // An empty part (full or zero HP) is dropped by `add_overlay`.
        for (rect, color) in hp_bar(unit, tile) {
            buf.add_overlay(Overlay::new(rect, ctx.palette.get(color), Layer::Over));
        }
        let mark = effect_mark(unit.effects.bonus, unit.effects.penalty, scene.clock_ms);
        if let (Some((mark, bounced)), Some(image)) = (mark, arrows) {
            let top_row = unit.pos.y == grid.origin.y;
            let lowered = top_row || has_unit_above(scene, unit);
            let dest = mark_dest(mark, tile, lowered, bounced);
            let mut sprite = Sprite::new(image, mark.src(), dest, Layer::Over);
            sprite.base = Some(tile);
            if unit.acted {
                sprite.paint = Paint::Dimmed;
            }
            if let Some(clip) = dest.intersect(&grid.bounds()) {
                sprite.clip = clip;
                buf.add_sprite(sprite);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_content::ImageRect;
    use trpg_core::{ClassId, Faction, LeadGender, LeadProfile, UnitId};

    use super::super::SpriteSkin;
    use super::super::tests::{brigand, p, painted, plains, sheets, skin, with_lord};
    use super::*;
    use crate::color::Rgb;
    use crate::glyph_buffer::Item;
    use crate::screen::tests::ctx;

    /// The brigand `id` on `pos`.
    fn unit(id: u32, pos: Pos) -> UnitView {
        UnitView {
            id: UnitId(id),
            ..brigand(pos)
        }
    }

    /// What `scene` paints after its four tiles under the test tileset
    /// (24 px tiles; 2 × 2 of them from (16, 24)).
    fn items(c: &Ctx, scene: &MapScene) -> Vec<Item> {
        painted(c, &skin(c), scene).items()[4..].to_vec()
    }

    fn sprites(items: &[Item]) -> Vec<Sprite> {
        let sprite = |i: &Item| match i {
            Item::Sprite(s) => Some(*s),
            Item::Rect(_) => None,
        };
        items.iter().filter_map(sprite).collect()
    }

    fn rects(items: &[Item]) -> Vec<(PxRect, Rgb)> {
        let rect = |i: &Item| match i {
            Item::Rect(o) => Some((o.rect, o.color)),
            Item::Sprite(_) => None,
        };
        items.iter().filter_map(rect).collect()
    }

    #[test]
    fn a_unit_is_its_outline_its_picture_and_its_hp_bar() {
        let c = ctx();
        let pal = &c.palette;
        let mut scene = plains(&c);
        let mut u = brigand(p(1, 1));
        u.hp = (20, 30);
        scene.push_unit(u);
        let items = items(&c, &scene);
        let class = skin(&c).tileset().classes[&ClassId("brigand".into())];
        // Tile (1, 1) is 40..64 × 48..72; the 24 × 24 picture stands 2 px
        // above its bottom edge.
        let dest = Rect::new(40, 46, 24, 24);
        let enemy = pal.get(UiColor::Enemy);
        let drawn = sprites(&items);
        assert_eq!(drawn.len(), 5);
        let outline: Vec<PxRect> = drawn[..4].iter().map(|s| s.dest).collect();
        assert_eq!(outline, outline_dests(dest));
        for s in &drawn[..4] {
            assert_eq!((s.paint, s.opacity), (Paint::Solid(enemy), 255));
        }
        assert_eq!((drawn[4].dest, drawn[4].paint), (dest, Paint::Image));
        for s in &drawn {
            assert_eq!((s.image, s.src), (class.image, src_rect(class.rect)));
            assert_eq!((s.layer, s.flip_x), (Layer::Over, false));
        }
        // The view ends at x = 64: the outline's right copy is cut there.
        assert_eq!(drawn[3].clip, Rect::new(41, 46, 23, 24));
        assert_eq!(drawn[4].clip, dest);
        // 20 / 30 HP of 22 px: 15 filled, along the bottom, 1 px in.
        assert_eq!(
            rects(&items),
            [
                (Rect::new(41, 70, 15, 2), pal.get(UiColor::HpMid)),
                (Rect::new(56, 70, 7, 2), pal.get(UiColor::Black)),
            ]
        );
        assert_eq!(items.len(), 7);
        // A unit off the view: nothing.
        let mut scene = plains(&c);
        scene.units.push(brigand(p(2, 0)));
        assert!(self::items(&c, &scene).is_empty());
    }

    #[test]
    fn the_outline_is_the_picture_one_pixel_each_way() {
        let dest = Rect::new(40, 46, 16, 20);
        assert_eq!(
            outline_dests(dest),
            [
                Rect::new(40, 45, 16, 20),
                Rect::new(40, 47, 16, 20),
                Rect::new(39, 46, 16, 20),
                Rect::new(41, 46, 16, 20),
            ]
        );
    }

    #[test]
    fn a_picture_stands_on_the_hp_bar_centred() {
        let c = ctx();
        let image = c.content.tilesets["test"].fallback.image;
        let picture = |w, h| Picture {
            image,
            rect: ImageRect { x: 0, y: 0, w, h },
        };
        // A bought map sprite, 16 × 20, on a 16 × 16 tile: its feet 2 px
        // above the tile's bottom edge, its head 6 px into the tile above.
        let tile = Rect::new(32, 48, 16, 16);
        assert_eq!(unit_dest(picture(16, 20), tile), Rect::new(32, 42, 16, 20));
        // Narrower: centred; wider: over both sides.
        assert_eq!(unit_dest(picture(8, 16), tile), Rect::new(36, 46, 8, 16));
        assert_eq!(unit_dest(picture(24, 16), tile), Rect::new(28, 46, 24, 16));
        assert_eq!(unit_dest(picture(17, 16), tile).x, 32);
        assert_eq!(HP_BAR_H, 2);
    }

    #[test]
    fn the_hp_bar_is_one_pixel_in_from_each_side_and_fills_by_hp() {
        use UiColor::{Black, HpHigh, HpLow, HpMid};
        let tile = Rect::new(32, 48, 16, 16);
        let bar = |hp, max| {
            let mut u = brigand(p(0, 0));
            u.hp = (hp, max);
            hp_bar(&u, tile)
        };
        // 14 px wide: tile columns 1 to 14.
        assert_eq!(
            bar(30, 30),
            [
                (Rect::new(33, 62, 14, 2), HpHigh),
                (Rect::new(47, 62, 0, 2), Black)
            ]
        );
        assert_eq!(
            bar(15, 30),
            [
                (Rect::new(33, 62, 7, 2), HpMid),
                (Rect::new(40, 62, 7, 2), Black)
            ]
        );
        // 1 / 30 of 14 px is 0.47: nothing filled.
        assert_eq!(
            bar(1, 30),
            [
                (Rect::new(33, 62, 0, 2), HpLow),
                (Rect::new(33, 62, 14, 2), Black)
            ]
        );
        assert_eq!(bar(10, 30)[0], (Rect::new(33, 62, 5, 2), HpLow));
        assert_eq!(bar(0, 30)[1].0.w, 14);
        // A tile too narrow for a bar has none.
        let narrow = hp_bar(&brigand(p(0, 0)), Rect::new(0, 0, 1, 16));
        assert_eq!((narrow[0].0.w, narrow[1].0.w), (0, 0));
    }

    #[test]
    fn an_arrow_rests_then_bounces_and_two_take_turns() {
        use Mark::{Bonus, Penalty};
        assert_eq!(effect_mark(false, false, 0), None);
        assert_eq!(effect_mark(false, false, 999), None);
        // One arrow: at rest for 375 ms, bounced for 375 ms, and again.
        for (bonus, penalty, mark) in [(true, false, Bonus), (false, true, Penalty)] {
            let at = |ms| effect_mark(bonus, penalty, ms);
            assert_eq!(at(0), Some((mark, false)));
            assert_eq!(at(374), Some((mark, false)));
            assert_eq!(at(375), Some((mark, true)));
            assert_eq!(at(749), Some((mark, true)));
            assert_eq!(at(750), Some((mark, false)));
            assert_eq!(at(1125), Some((mark, true)));
            assert_eq!(at(1500), Some((mark, false)));
        }
        // Both: the up arrow for 750 ms, then the down arrow for 750 ms.
        let at = |ms| effect_mark(true, true, ms);
        assert_eq!(at(0), Some((Bonus, false)));
        assert_eq!(at(375), Some((Bonus, true)));
        assert_eq!(at(749), Some((Bonus, true)));
        assert_eq!(at(750), Some((Penalty, false)));
        assert_eq!(at(1125), Some((Penalty, true)));
        assert_eq!(at(1499), Some((Penalty, true)));
        assert_eq!(at(1500), Some((Bonus, false)));
        assert_eq!(at(2250), Some((Penalty, false)));
        assert_eq!((BOUNCE_MS, TURN_MS), (375, 750));
    }

    #[test]
    fn the_arrow_sits_on_the_tiles_top_right_corner() {
        use Mark::{Bonus, Penalty};
        let tile = Rect::new(32, 48, 16, 16);
        let at = |mark, lowered, bounced| {
            let dest = mark_dest(mark, tile, lowered, bounced);
            assert_eq!((dest.w, dest.h), (7, 7));
            (dest.x, dest.y)
        };
        // The 7 × 7 box starts at tile x + 10, tile y − 3.
        assert_eq!(at(Bonus, false, false), (42, 45));
        assert_eq!(at(Penalty, false, false), (42, 45));
        // Bounced: the up arrow up, the down arrow down.
        assert_eq!(at(Bonus, false, true), (42, 44));
        assert_eq!(at(Penalty, false, true), (42, 46));
        // Lowered: 4 px, inside its own tile, bounce and all.
        assert_eq!(at(Bonus, true, false), (42, 49));
        assert_eq!(at(Bonus, true, true), (42, 48));
        assert_eq!(at(Penalty, true, true), (42, 50));
        // The up arrow is the image's left box, the down arrow its right.
        assert_eq!(Bonus.src(), Rect::new(0, 0, 7, 7));
        assert_eq!(Penalty.src(), Rect::new(7, 0, 7, 7));
        // The image is there, and is two boxes.
        let c = ctx();
        let image = c.content.images.id(EFFECT_MARKS_PATH).unwrap();
        let info = c.content.images.info(image).unwrap();
        assert_eq!((info.width, info.height), (14, 7));
    }

    /// The effect arrow `scene` paints under the unit sheets (16 px
    /// tiles; the 3 × 2 view starts at (16, 16)).
    fn arrow(c: &Ctx, scene: &MapScene) -> Option<Sprite> {
        let buf = painted(c, &sheets(c), scene);
        let marks = c.content.images.id(EFFECT_MARKS_PATH);
        buf.sprites().into_iter().find(|s| Some(s.image) == marks)
    }

    #[test]
    fn a_unit_under_an_effect_shows_its_arrow_by_the_scenes_clock() {
        let c = ctx();
        let mut scene = plains(&c);
        scene.push_unit(unit(1, p(1, 1)));
        assert_eq!(arrow(&c, &scene), None);
        // Tile (1, 1) is 32..48 × 32..48.
        scene.units[0].effects.bonus = true;
        let up = arrow(&c, &scene).unwrap();
        assert_eq!(up.src, Mark::Bonus.src());
        assert_eq!(up.dest, Rect::new(42, 29, 7, 7));
        let look = (up.layer, up.paint, up.opacity);
        assert_eq!(look, (Layer::Over, Paint::Image, 255));
        scene.clock_ms = 375;
        assert_eq!(arrow(&c, &scene).unwrap().dest, Rect::new(42, 28, 7, 7));
        // A penalty: the down arrow, bouncing down.
        scene.units[0].effects.bonus = false;
        scene.units[0].effects.penalty = true;
        let down = arrow(&c, &scene).unwrap();
        assert_eq!(down.src, Mark::Penalty.src());
        assert_eq!(down.dest, Rect::new(42, 30, 7, 7));
        // Both: the up arrow first, then the down arrow.
        scene.units[0].effects.bonus = true;
        assert_eq!(arrow(&c, &scene).unwrap().src, Mark::Bonus.src());
        scene.clock_ms = 750;
        assert_eq!(arrow(&c, &scene).unwrap().src, Mark::Penalty.src());
        // It dims with an acted unit, and goes half way through a fall.
        scene.units[0].acted = true;
        assert_eq!(arrow(&c, &scene).unwrap().paint, Paint::Dimmed);
        scene.units[0].fade = 0.5;
        assert_eq!(arrow(&c, &scene), None);
        // It is the last thing painted for the unit: over its bar.
        scene.units[0].fade = 0.0;
        let buf = painted(&c, &sheets(&c), &scene);
        let last = buf.items().last().copied();
        assert!(matches!(last, Some(Item::Sprite(s)) if s.src == Mark::Penalty.src()));
    }

    #[test]
    fn a_unit_below_another_is_shaved_at_its_tiles_top_and_its_arrow_lowered() {
        let c = ctx();
        let s = sheets(&c);
        let mut scene = plains(&c);
        let mut below = unit(2, p(1, 1));
        below.effects.penalty = true;
        scene.push_unit(below);
        // Nobody above: the whole 16 × 20 picture, 6 px into the tile above.
        let dest = Rect::new(32, 26, 16, 20);
        let alone = painted(&c, &s, &scene).sprites();
        assert_eq!((alone[4].dest, alone[4].clip), (dest, dest));
        assert_eq!(alone[0].clip, Rect::new(32, 25, 16, 20));
        assert_eq!(arrow(&c, &scene).unwrap().dest, Rect::new(42, 29, 7, 7));
        // A unit beside it, or diagonally above, changes nothing.
        for other in [p(0, 1), p(2, 1), p(0, 0), p(2, 0)] {
            let mut scene = scene.clone();
            scene.push_unit(unit(3, other));
            let drawn = painted(&c, &s, &scene).sprites();
            let own: Vec<&Sprite> = drawn.iter().filter(|s| s.dest == dest).collect();
            assert_eq!(own.len(), 1, "{other:?}");
            assert_eq!(own[0].clip, dest, "{other:?}");
        }
        // One directly above: the picture and its outline end at the
        // tile's top edge (y = 32), and the arrow sits 4 px lower.
        scene.push_unit(unit(1, p(1, 0)));
        let drawn = painted(&c, &s, &scene).sprites();
        // The unit above is painted first: five sprites, whole (but for
        // the view's top edge at y = 16).
        assert_eq!(drawn[4].dest, Rect::new(32, 10, 16, 20));
        assert_eq!(drawn[4].clip, Rect::new(32, 16, 16, 14));
        let shaved: Vec<PxRect> = drawn[5..10].iter().map(|s| s.clip).collect();
        assert_eq!(
            shaved,
            [
                Rect::new(32, 32, 16, 13),
                Rect::new(32, 32, 16, 15),
                Rect::new(31, 32, 16, 14),
                Rect::new(33, 32, 16, 14),
                Rect::new(32, 32, 16, 14),
            ]
        );
        assert_eq!(drawn[9].dest, dest);
        assert_eq!(arrow(&c, &scene).unwrap().dest, Rect::new(42, 33, 7, 7));
        // On the view's top row the arrow is lowered too, so it isn't cut.
        let mut scene = plains(&c);
        let mut top = unit(1, p(0, 0));
        top.effects.bonus = true;
        scene.push_unit(top);
        let mark = arrow(&c, &scene).unwrap();
        assert_eq!(mark.dest, Rect::new(26, 17, 7, 7));
        assert_eq!(mark.clip, mark.dest);
        // At the view's right edge it is cut where the view ends (x = 64).
        scene.units[0].pos = p(2, 1);
        let mark = arrow(&c, &scene).unwrap();
        assert_eq!(mark.dest, Rect::new(58, 29, 7, 7));
        assert_eq!(mark.clip, Rect::new(58, 29, 6, 7));
    }

    #[test]
    fn pictures_go_top_row_first_and_every_bar_and_mark_after_them() {
        let c = ctx();
        let s = sheets(&c);
        let mut scene = plains(&c);
        // In the battle's order: bottom row, top row, bottom row.
        let mut first = unit(1, p(0, 1));
        first.effects.bonus = true;
        scene.push_unit(first);
        scene.push_unit(unit(2, p(2, 0)));
        scene.push_unit(unit(3, p(1, 1)));
        let buf = painted(&c, &s, &scene);
        let marks = c.content.images.id(EFFECT_MARKS_PATH);
        let order: Vec<(&str, i32)> = buf
            .items()
            .iter()
            .map(|item| match item {
                Item::Sprite(s) if Some(s.image) == marks => ("mark", s.dest.x),
                Item::Sprite(s) => ("picture", s.dest.x),
                Item::Rect(o) => ("bar", o.rect.x),
            })
            .collect();
        // Five sprites a unit (the outline's left and right copies are
        // 1 px off): the top row's unit (x = 48), then the bottom row's
        // in the battle's order (x = 16, x = 32).
        let pictures: Vec<i32> = order.iter().take(15).map(|(_, x)| *x).collect();
        let five = |x| [x, x, x - 1, x + 1, x];
        assert_eq!(pictures, [five(48), five(16), five(32)].concat());
        assert!(order.iter().take(15).all(|(kind, _)| *kind == "picture"));
        // Then, unit by unit in the battle's order, its bar and its mark.
        assert_eq!(
            order[15..],
            [("bar", 17), ("mark", 26), ("bar", 49), ("bar", 33)]
        );
    }

    #[test]
    fn an_acted_unit_is_grey_and_darker_and_a_falling_one_fades() {
        let c = ctx();
        let enemy = c.palette.get(UiColor::Enemy);
        let drawn = |u: UnitView| {
            let mut scene = plains(&c);
            scene.push_unit(u);
            sprites(&items(&c, &scene))
        };
        let mut acted = brigand(p(0, 0));
        acted.acted = true;
        let s = drawn(acted.clone());
        assert_eq!(s[0].paint, Paint::Solid(enemy.dimmed()));
        assert_eq!((s[4].paint, s[4].opacity), (Paint::Dimmed, 255));
        assert_ne!(enemy.dimmed(), enemy);
        // Falling: the picture fades over the whole fall, its outline over
        // the first half.
        let standing = brigand(p(0, 0));
        let at = |fade| {
            let u = standing.clone().fading(fade);
            (unit_opacity(&u), outline_opacity(&u))
        };
        assert_eq!(at(0.0), (255, 255));
        assert_eq!(at(0.25), (191, 128));
        assert_eq!(at(0.5), (128, 0));
        assert_eq!(at(0.75), (64, 0));
        assert_eq!(at(1.0), (0, 0));
        assert_eq!(at(7.0), (0, 0));
        // Not a number, or below 0: standing.
        assert_eq!(at(f32::NAN), (255, 255));
        assert_eq!(at(f32::INFINITY), (255, 255));
        assert_eq!(at(-1.0), (255, 255));
        let s = drawn(acted.fading(0.25));
        let look = (s[0].opacity, s[4].opacity, s[4].paint);
        assert_eq!(look, (128, 191, Paint::Dimmed));
        // From half way: the picture alone, no outline, bar or mark.
        let mut scene = plains(&c);
        let mut falling = brigand(p(0, 0)).fading(0.5);
        falling.effects.bonus = true;
        scene.push_unit(falling);
        let left = items(&c, &scene);
        assert_eq!((left.len(), sprites(&left).len()), (1, 1));
        scene.units[0].fade = 1.0;
        assert!(items(&c, &scene).is_empty());
        scene.units[0].fade = 0.49;
        assert_eq!(items(&c, &scene).len(), 5 + 1 + 1);
    }

    #[test]
    fn each_side_has_its_outline_colour() {
        let c = ctx();
        let sides = [
            (Faction::Player, UiColor::Player),
            (Faction::Enemy, UiColor::Enemy),
            (Faction::Ally, UiColor::Ally),
            (Faction::Neutral, UiColor::Neutral),
        ];
        for (faction, color) in sides {
            let mut scene = plains(&c);
            let mut u = brigand(p(0, 0));
            u.faction = faction;
            scene.push_unit(u);
            let outline = sprites(&items(&c, &scene))[0].paint;
            assert_eq!(outline, Paint::Solid(c.palette.get(color)), "{faction:?}");
        }
    }

    #[test]
    fn a_picture_is_the_characters_else_its_class_else_the_fallback() {
        let mut c = ctx();
        let s = with_lord(&skin(&c));
        let tileset = s.tileset().clone();
        let src = |c: &Ctx, s: &SpriteSkin, unit: UnitView| {
            let mut scene = plains(c);
            scene.push_unit(unit);
            painted(c, s, &scene).sprites()[8].src
        };
        let lord = CharacterId("test_lord".into());
        let mut named = brigand(p(0, 0));
        named.character = Some(lord.clone());
        let own = src_rect(tileset.characters[&lord].rect);
        assert_eq!(src(&c, &s, named.clone()), own);
        named.character = Some(CharacterId("nobody".into()));
        let class = tileset.classes[&ClassId("brigand".into())];
        assert_eq!(src(&c, &s, named.clone()), src_rect(class.rect));
        named.class = ClassId("no_such_class".into());
        assert_eq!(src(&c, &s, named.clone()), src_rect(tileset.fallback.rect));
        assert_ne!(class.rect, tileset.fallback.rect);
        // The lead: the picture of the player's gender, else the lead's
        // own, else the class's.
        let mut both = tileset.clone();
        let picture = |x| Picture {
            image: tileset.fallback.image,
            rect: ImageRect {
                x,
                ..tileset.fallback.rect
            },
        };
        let id = |s: &str| CharacterId(s.to_owned());
        both.characters.insert(id("lead_f"), picture(24));
        both.characters.insert(id("lead"), picture(48));
        let s = SpriteSkin::new(both);
        named.character = Some(id("lead"));
        named.class = ClassId("brigand".into());
        c.lead = LeadProfile::new("A", LeadGender::Female);
        assert_eq!(src(&c, &s, named.clone()).x, 24);
        c.lead = LeadProfile::new("A", LeadGender::Male);
        assert_eq!(src(&c, &s, named.clone()).x, 48);
        assert_eq!(picture_ids(&c, &named), [id("lead_m"), id("lead")]);
        assert!(picture_ids(&c, &brigand(p(0, 0))).is_empty());
        // Nobody else goes by the lead's pictures.
        named.character = Some(lord.clone());
        assert_eq!(picture_ids(&c, &named), [lord]);
    }

    proptest! {
        /// For any unit frame up to the tile's size plus 16 pixels, on any
        /// tile, the picture is centred across its tile and its bottom is
        /// on top of the HP bar.
        #[test]
        fn a_picture_is_centred_and_stands_on_the_hp_bar(
            tile in (-200..800i32, -200..800i32, 8..=64i32, 8..=64i32),
            extra in (0..=16u32, 0..=16u32),
            smaller in (0..=8u32, 0..=8u32),
        ) {
            let c = ctx();
            let tile = Rect::new(tile.0, tile.1, tile.2, tile.3);
            let size = |side: i32, more: u32, less: u32| {
                (u32::try_from(side).unwrap_or(8) + more).saturating_sub(less).max(1)
            };
            let (w, h) = (size(tile.w, extra.0, smaller.0), size(tile.h, extra.1, smaller.1));
            let picture = Picture {
                image: c.content.tilesets["test"].fallback.image,
                rect: ImageRect { x: 0, y: 0, w, h },
            };
            let dest = unit_dest(picture, tile);
            prop_assert_eq!((dest.w, dest.h), (side(w), side(h)));
            prop_assert_eq!(dest.y + dest.h, tile.y + tile.h - HP_BAR_H);
            // Centred: the space left and right differs by at most 1 px.
            let (left, right) = (dest.x - tile.x, tile.x + tile.w - dest.x - dest.w);
            prop_assert!((left - right).abs() <= 1, "{} vs {}", left, right);
        }
    }
}
