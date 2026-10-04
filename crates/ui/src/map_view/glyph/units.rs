//! How the glyph skin draws a unit (ADR-0018, `docs/design/look-and-feel.md`):
//! its two-letter map label in its faction colour on the terrain background,
//! dimmed once it has acted (the label keeps its case), and a 2-px HP bar
//! along the bottom of its tile, one pixel in from each side.

use trpg_core::StatValue;

use super::TILE_W_CELLS;
use crate::color::{Palette, UiColor};
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{Cell, GlyphBuffer, Layer, Overlay, Rect};
use crate::map_view::scene::UnitView;
use crate::screens::battle::units::{faction_color, hp_fill};

/// How far the HP bar stops short of each side of its tile, in pixels, so
/// the bars of units standing side by side don't run together.
pub const HP_BAR_INSET: i32 = 1;

/// Full HP bar length, in pixels: the tile's width (two 8-px cells) less
/// the inset on each side.
const HP_BAR_W: i32 = 14;

/// HP bar thickness, in pixels, at the bottom of the tile.
pub const HP_BAR_H: i32 = 2;

/// How far an acted unit's label fades toward the background (`0` = not at
/// all, `1` = invisible). *Tunable.*
pub const ACTED_DIM: f32 = 0.5;

/// How much of the effect colour a unit under a timed effect gets on its
/// glyphs' background (`0` = none, `1` = all of it). *Tunable.*
pub const EFFECT_BLEND: f32 = 0.8;

/// The filled length of the HP bar (`round(14 × hp / max)`, in `0..=14`)
/// and its colour: see [`hp_fill`].
pub fn hp_bar(hp: StatValue, max: StatValue) -> (i32, UiColor) {
    hp_fill(hp, max, HP_BAR_W)
}

/// Draws `unit` on the tile whose left cell is `(x, y)`: the label over the
/// cells' existing (terrain) background, then the HP bar overlays.
///
/// A unit part of the way through falling ([`UnitView::fade`], ticket 0404)
/// fades: over the first half its label and HP bar fade into the tile's
/// background, over the second half the terrain's glyphs fade back in; at
/// `1` (or more) only the terrain is left.
pub fn draw_unit(buf: &mut GlyphBuffer, palette: &Palette, unit: &UnitView, x: i32, y: i32) {
    let fade = if unit.fade.is_finite() {
        unit.fade.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if fade >= 0.5 {
        let k = (fade - 0.5) * 2.0;
        for i in 0..TILE_W_CELLS {
            if let Some(&cell) = buf.get(x + i, y) {
                let fg = cell.bg.lerp(cell.fg, k);
                buf.set(x + i, y, Cell { fg, ..cell });
            }
        }
        return;
    }
    let k = fade * 2.0;
    let faction = palette.get(faction_color(unit.faction));
    let effect = palette.get(UiColor::Effect);
    let tile_bg = buf.get(x, y).map_or(faction, |c| c.bg);
    for (i, glyph) in (0..TILE_W_CELLS).zip(unit.label.chars()) {
        let Some(&cell) = buf.get(x + i, y) else {
            continue;
        };
        let fg = if unit.acted {
            faction.lerp(cell.bg, ACTED_DIM)
        } else {
            faction
        };
        let fg = fg.lerp(cell.bg, k);
        // Under a timed effect (a buff or a debuff) the glyphs sit on the
        // effect colour, so it shows on the map (0412).
        let bg = if unit.has_effect() {
            cell.bg.lerp(effect, EFFECT_BLEND)
        } else {
            cell.bg
        };
        buf.set(x + i, y, Cell::new(glyph, fg, bg));
    }
    let (width, color) = hp_bar(unit.hp.0, unit.hp.1);
    let px = x * i32::from(CELL_W_PX) + HP_BAR_INSET;
    let py = (y + 1) * i32::from(CELL_H_PX) - HP_BAR_H;
    let bar = |bx: i32, w: i32, c: UiColor| {
        let color = palette.get(c).lerp(tile_bg, k);
        Overlay::new(Rect::new(bx, py, w, HP_BAR_H), color, Layer::Over)
    };
    // An empty part (full or zero HP) is dropped by `add_overlay`.
    buf.add_overlay(bar(px, width, color));
    buf.add_overlay(bar(px + width, HP_BAR_W - width, UiColor::Black));
}

/// Takes the terrain's glyphs off the tile whose left cell is `(x, y)`,
/// for a unit drawn there as a picture (ADR-0049): the tile keeps its
/// background. A `highlight`ed unit (one a battle note is about) stands on
/// the terrain's glyph colour instead. Over the second half of a unit's
/// fall (`fade` from `0.5` to `1`) the glyphs fade back in, as under
/// [`draw_unit`].
pub fn clear_glyphs(buf: &mut GlyphBuffer, x: i32, y: i32, fade: f32, highlight: bool) {
    let back = if fade.is_finite() {
        (fade.clamp(0.0, 1.0) - 0.5).max(0.0) * 2.0
    } else {
        0.0
    };
    for i in 0..TILE_W_CELLS {
        let Some(&cell) = buf.get(x + i, y) else {
            continue;
        };
        let bg = if highlight { cell.fg } else { cell.bg };
        let cleared = if back > 0.0 {
            let fg = bg.lerp(cell.fg, back);
            Cell { fg, bg, ..cell }
        } else {
            Cell::new(' ', cell.fg, bg)
        };
        buf.set(x + i, y, cleared);
    }
}

/// Swaps the text and background colours of the tile whose left cell is
/// `(x, y)`: how a highlighted unit (one a battle note is about, 0411) is
/// picked out.
pub fn invert_tile(buf: &mut GlyphBuffer, x: i32, y: i32) {
    for i in 0..TILE_W_CELLS {
        if let Some(&cell) = buf.get(x + i, y) {
            let (fg, bg) = (cell.bg, cell.fg);
            buf.set(x + i, y, Cell { fg, bg, ..cell });
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_core::{ClassId, Faction, Pos, Stats, Unit, UnitId};

    use super::*;
    use crate::color::Rgb;
    use crate::color::tests::game_palette;

    fn unit(label: &str, hp: StatValue, max: StatValue, acted: bool) -> UnitView {
        let content = trpg_content::load_embedded().unwrap();
        let mut u = Unit::generic(
            UnitId(1),
            &ClassId("brigand".into()),
            &content.classes,
            1,
            Faction::Enemy,
            Pos::new(0, 0),
        )
        .unwrap();
        u.map_label = label.into();
        u.stats = Stats { hp: max, ..u.stats };
        u.hp = hp;
        u.acted = acted;
        UnitView::of(&u)
    }

    #[test]
    fn hp_bar_spans_the_tile_less_a_pixel_each_side() {
        let tile = TILE_W_CELLS * i32::from(CELL_W_PX);
        assert_eq!(HP_BAR_INSET, 1);
        assert_eq!(HP_BAR_W, tile - 2 * HP_BAR_INSET);
        assert_eq!(HP_BAR_W, 14);
    }

    #[test]
    fn hp_bar_at_the_thresholds() {
        use UiColor::{HpHigh, HpLow, HpMid};
        assert_eq!(hp_bar(30, 30), (14, HpHigh));
        assert_eq!(hp_bar(21, 30), (10, HpHigh)); // 9.8
        assert_eq!(hp_bar(20, 30), (9, HpMid)); // exactly 2/3: 9.33
        assert_eq!(hp_bar(15, 30), (7, HpMid)); // half
        assert_eq!(hp_bar(11, 30), (5, HpMid)); // 5.13
        assert_eq!(hp_bar(10, 30), (5, HpLow)); // exactly 1/3: 4.67
        assert_eq!(hp_bar(1, 20), (1, HpLow)); // 0.7 rounds up
        assert_eq!(hp_bar(1, 30), (0, HpLow)); // 0.47 rounds down
        assert_eq!(hp_bar(0, 30), (0, HpLow));
        assert_eq!(hp_bar(1, 28), (1, HpLow)); // exactly 0.5 rounds up
        assert_eq!(hp_bar(-5, 30), (0, HpLow));
        assert_eq!(hp_bar(99, 30), (14, HpHigh));
        assert_eq!(hp_bar(5, 0), (0, HpLow));
        assert_eq!(hp_bar(5, -3), (0, HpLow));
    }

    proptest! {
        #[test]
        fn hp_bar_width_is_in_range(hp in any::<StatValue>(), max in any::<StatValue>()) {
            let (w, _) = hp_bar(hp, max);
            prop_assert!((0..=HP_BAR_W).contains(&w));
        }
    }

    fn drawn(u: &UnitView) -> GlyphBuffer {
        let p = game_palette();
        let bg = Rgb::new(0, 0, 100);
        let mut b = GlyphBuffer::new(4, 2, Cell::new('.', Rgb::new(1, 1, 1), bg));
        draw_unit(&mut b, &p, u, 1, 1);
        b
    }

    #[test]
    fn unit_is_drawn_on_the_terrain_background() {
        let p = game_palette();
        let bg = Rgb::new(0, 0, 100);
        let b = drawn(&unit("Br", 20, 30, false));
        let enemy = p.get(UiColor::Enemy);
        assert_eq!(b.get(1, 1), Some(&Cell::new('B', enemy, bg)));
        assert_eq!(b.get(2, 1), Some(&Cell::new('r', enemy, bg)));
        assert_eq!(b.get(3, 1).map(|c| c.glyph), Some('.'));
        let bar = |x, w, c| Overlay::new(Rect::new(x, 30, w, 2), p.get(c), Layer::Over);
        assert_eq!(
            b.overlays(),
            [bar(9, 9, UiColor::HpMid), bar(18, 5, UiColor::Black)]
        );
    }

    /// The bar is the rectangle `(tile x + 1, tile bottom - 2, 14, 2)`: here
    /// the tile's left edge is at pixel 8 and its bottom at pixel 32.
    #[test]
    fn hp_bar_is_14_pixels_wide_one_pixel_in_from_each_side() {
        let p = game_palette();
        let bar = |x, w, c| Overlay::new(Rect::new(x, 30, w, 2), p.get(c), Layer::Over);
        let full = drawn(&unit("Br", 30, 30, false));
        assert_eq!(full.overlays(), [bar(9, 14, UiColor::HpHigh)]);
        let half = drawn(&unit("Br", 15, 30, false));
        assert_eq!(
            half.overlays(),
            [bar(9, 7, UiColor::HpMid), bar(16, 7, UiColor::Black)]
        );
        let one = drawn(&unit("Br", 1, 20, false));
        assert_eq!(
            one.overlays(),
            [bar(9, 1, UiColor::HpLow), bar(10, 13, UiColor::Black)]
        );
        let empty = drawn(&unit("Br", 0, 30, false));
        assert_eq!(empty.overlays(), [bar(9, 14, UiColor::Black)]);
    }

    #[test]
    fn a_falling_unit_fades_into_its_tile_then_the_terrain_comes_back() {
        let p = game_palette();
        let (terrain, bg) = (Rgb::new(1, 1, 1), Rgb::new(0, 0, 100));
        let tile = |b: &mut GlyphBuffer| b.set(2, 1, Cell::new(',', terrain, bg));
        let fading = |fade| {
            let mut b = GlyphBuffer::new(4, 2, Cell::new('.', terrain, bg));
            tile(&mut b);
            draw_unit(&mut b, &p, &unit("Br", 15, 30, false).fading(fade), 1, 1);
            b
        };
        let enemy = p.get(UiColor::Enemy);
        // Not faded: as drawn normally.
        let mut normal = GlyphBuffer::new(4, 2, Cell::new('.', terrain, bg));
        tile(&mut normal);
        draw_unit(&mut normal, &p, &unit("Br", 15, 30, false), 1, 1);
        assert_eq!(fading(0.0), normal);
        assert_eq!(fading(f32::NAN), fading(0.0));
        // A quarter: the letters and the bar halfway to the background.
        let b = fading(0.25);
        assert_eq!(b.get(1, 1), Some(&Cell::new('B', enemy.lerp(bg, 0.5), bg)));
        let half = p.get(hp_bar(15, 30).1).lerp(bg, 0.5);
        assert_eq!(b.overlays()[0].color, half);
        // Three quarters: the terrain back, halfway to its own colour.
        let b = fading(0.75);
        assert_eq!(
            b.get(1, 1),
            Some(&Cell::new('.', bg.lerp(terrain, 0.5), bg))
        );
        assert_eq!(
            b.get(2, 1),
            Some(&Cell::new(',', bg.lerp(terrain, 0.5), bg))
        );
        assert!(b.overlays().is_empty());
        // Gone: the tile as it was.
        let b = fading(1.0);
        let mut plain = GlyphBuffer::new(4, 2, Cell::new('.', terrain, bg));
        tile(&mut plain);
        assert_eq!(b, plain);
        assert_eq!(fading(7.0), b);
    }

    #[test]
    fn acted_units_keep_their_label_case_and_are_dimmed_toward_the_background() {
        let p = game_palette();
        let bg = Rgb::new(0, 0, 100);
        let b = drawn(&unit("Br", 30, 30, true));
        let dim = p.get(UiColor::Enemy).lerp(bg, ACTED_DIM);
        assert_eq!(b.get(1, 1), Some(&Cell::new('B', dim, bg)));
        assert_eq!(b.get(2, 1), Some(&Cell::new('r', dim, bg)));
    }

    #[test]
    fn a_unit_under_an_effect_has_the_effect_colour_behind_its_letters() {
        let p = game_palette();
        let bg = Rgb::new(0, 0, 100);
        let mut u = unit("Br", 30, 30, false);
        u.effects.penalty = true;
        let b = drawn(&u);
        // A bonus, or both, looks the same here.
        for (bonus, penalty) in [(true, false), (true, true)] {
            let other = UnitView {
                effects: crate::map_view::UnitEffects { bonus, penalty },
                ..u.clone()
            };
            assert_eq!(drawn(&other), b);
        }
        let (enemy, effect) = (p.get(UiColor::Enemy), p.get(UiColor::Effect));
        let behind = bg.lerp(effect, EFFECT_BLEND);
        assert_ne!(behind, bg);
        assert_eq!(b.get(1, 1), Some(&Cell::new('B', enemy, behind)));
        assert_eq!(b.get(2, 1), Some(&Cell::new('r', enemy, behind)));
        // The terrain beside it keeps its own.
        assert_eq!(b.get(3, 1).map(|c| c.bg), Some(bg));
    }

    #[test]
    fn clearing_a_tile_takes_its_glyphs_and_keeps_its_background() {
        let (fg, bg) = (Rgb::new(200, 100, 0), Rgb::new(0, 0, 100));
        let cleared = |fade, highlight| {
            let mut b = GlyphBuffer::new(4, 2, Cell::new('.', fg, bg));
            clear_glyphs(&mut b, 1, 1, fade, highlight);
            // Only the tile's two cells change.
            assert_eq!(b.get(0, 1), Some(&Cell::new('.', fg, bg)));
            assert_eq!(b.get(3, 1), Some(&Cell::new('.', fg, bg)));
            assert_eq!(b.get(1, 0), Some(&Cell::new('.', fg, bg)));
            assert_eq!(b.get(1, 1), b.get(2, 1));
            *b.get(1, 1).unwrap()
        };
        assert_eq!(cleared(0.0, false), Cell::new(' ', fg, bg));
        assert_eq!(cleared(0.5, false), Cell::new(' ', fg, bg));
        assert_eq!(cleared(f32::NAN, false), Cell::new(' ', fg, bg));
        assert_eq!(cleared(-1.0, false), Cell::new(' ', fg, bg));
        // Picked out: on the terrain's glyph colour.
        assert_eq!(cleared(0.0, true), Cell::new(' ', fg, fg));
        // Falling: the glyphs come back over the second half.
        assert_eq!(cleared(0.75, false), Cell::new('.', bg.lerp(fg, 0.5), bg));
        assert_eq!(cleared(1.0, false), Cell::new('.', fg, bg));
        assert_eq!(cleared(7.0, false), Cell::new('.', fg, bg));
        // Picked out while falling: still on the glyph colour.
        assert_eq!(cleared(0.75, true), Cell::new('.', fg, fg));
        // At the buffer's edge: the cell that exists.
        let mut b = GlyphBuffer::new(1, 1, Cell::new('.', fg, bg));
        clear_glyphs(&mut b, 0, 0, 0.0, false);
        assert_eq!(b.get(0, 0), Some(&Cell::new(' ', fg, bg)));
    }

    #[test]
    fn a_unit_at_the_edge_is_clipped() {
        let p = game_palette();
        let mut b = GlyphBuffer::new(1, 1, Cell::new('.', Rgb::new(1, 1, 1), Rgb::new(0, 0, 0)));
        draw_unit(&mut b, &p, &unit("Br", 15, 30, false), 0, 0);
        assert_eq!(b.get(0, 0).map(|c| c.glyph), Some('B'));
        assert_eq!(
            b.overlays(),
            [Overlay::new(
                Rect::new(1, 14, 7, 2),
                p.get(UiColor::HpMid),
                Layer::Over
            )]
        );
    }
}
