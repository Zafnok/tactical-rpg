//! How a unit looks on the map (ADR-0018, `docs/design/look-and-feel.md`):
//! its two-letter map label in its faction colour on the terrain background,
//! lowercased and dimmed once it has acted, and a 2-px HP bar along the
//! bottom of its tile.

use trpg_core::{Faction, StatValue, Unit};

use crate::color::{Palette, UiColor};
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{Cell, GlyphBuffer, Layer, Overlay, Rect};

/// Full HP bar length, in pixels: the tile's width (two 8-px cells).
pub const HP_BAR_W: i32 = 16;

/// HP bar thickness, in pixels, at the bottom of the tile.
pub const HP_BAR_H: i32 = 2;

/// How far an acted unit's label fades toward the background (`0` = not at
/// all, `1` = invisible). *Tunable.*
pub const ACTED_DIM: f32 = 0.5;

/// The colour of `faction`: player blue, enemy red, ally green, neutral
/// yellow.
pub const fn faction_color(faction: Faction) -> UiColor {
    match faction {
        Faction::Player => UiColor::Player,
        Faction::Enemy => UiColor::Enemy,
        Faction::Ally => UiColor::Ally,
        Faction::Neutral => UiColor::Neutral,
    }
}

/// The label as drawn: the unit's map label, lowercased once it has acted
/// (so "acted" never relies on colour alone).
pub fn shown_label(unit: &Unit) -> String {
    if unit.acted {
        unit.map_label
            .chars()
            .map(|c| c.to_lowercase().next().unwrap_or(c))
            .collect()
    } else {
        unit.map_label.clone()
    }
}

/// The filled length of the HP bar (`round(16 × hp / max)`, in `0..=16`)
/// and its colour: `hp_high` above 2/3, `hp_mid` above 1/3, else `hp_low`.
/// HP is clamped to `0..=max`; a non-positive `max` gives an empty bar.
pub fn hp_bar(hp: StatValue, max: StatValue) -> (i32, UiColor) {
    if max <= 0 {
        return (0, UiColor::HpLow);
    }
    let (hp, max) = (i64::from(hp.clamp(0, max)), i64::from(max));
    let full = i64::from(HP_BAR_W);
    // Round half up, in integers.
    let width = (2 * full * hp + max) / (2 * max);
    let color = if 3 * hp > 2 * max {
        UiColor::HpHigh
    } else if 3 * hp > max {
        UiColor::HpMid
    } else {
        UiColor::HpLow
    };
    (i32::try_from(width).unwrap_or(0), color)
}

/// Draws `unit` on the tile whose left cell is `(x, y)`: the label over the
/// cells' existing (terrain) background, then the HP bar overlays.
pub fn draw_unit(buf: &mut GlyphBuffer, palette: &Palette, unit: &Unit, x: i32, y: i32) {
    let faction = palette.get(faction_color(unit.faction));
    for (i, glyph) in (0..).zip(shown_label(unit).chars()) {
        let Some(&cell) = buf.get(x + i, y) else {
            continue;
        };
        let fg = if unit.acted {
            faction.lerp(cell.bg, ACTED_DIM)
        } else {
            faction
        };
        buf.set(x + i, y, Cell { glyph, fg, ..cell });
    }
    let (width, color) = hp_bar(unit.hp, unit.stats.hp);
    let px = x * i32::from(CELL_W_PX);
    let py = (y + 1) * i32::from(CELL_H_PX) - HP_BAR_H;
    let bar = |bx: i32, w: i32, c: UiColor| {
        Overlay::new(Rect::new(bx, py, w, HP_BAR_H), palette.get(c), Layer::Over)
    };
    if width > 0 {
        buf.add_overlay(bar(px, width, color));
    }
    if width < HP_BAR_W {
        buf.add_overlay(bar(px + width, HP_BAR_W - width, UiColor::Black));
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use trpg_core::{ClassId, Pos, Stats, UnitId};

    use super::*;
    use crate::color::Rgb;
    use crate::color::tests::game_palette;
    use crate::screens::battle::layout::TILE_W_CELLS;

    fn unit(label: &str, hp: StatValue, max: StatValue, acted: bool) -> Unit {
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
        u
    }

    #[test]
    fn hp_bar_spans_the_tile() {
        assert_eq!(HP_BAR_W, TILE_W_CELLS * i32::from(CELL_W_PX));
    }

    #[test]
    fn faction_colours() {
        assert_eq!(faction_color(Faction::Player), UiColor::Player);
        assert_eq!(faction_color(Faction::Enemy), UiColor::Enemy);
        assert_eq!(faction_color(Faction::Ally), UiColor::Ally);
        assert_eq!(faction_color(Faction::Neutral), UiColor::Neutral);
    }

    #[test]
    fn labels_lowercase_when_acted() {
        assert_eq!(shown_label(&unit("Br", 1, 1, false)), "Br");
        assert_eq!(shown_label(&unit("Br", 1, 1, true)), "br");
        assert_eq!(shown_label(&unit("KN", 1, 1, true)), "kn");
        assert_eq!(shown_label(&unit("Él", 1, 1, true)), "él");
    }

    #[test]
    fn hp_bar_at_the_thresholds() {
        use UiColor::{HpHigh, HpLow, HpMid};
        assert_eq!(hp_bar(30, 30), (16, HpHigh));
        assert_eq!(hp_bar(21, 30), (11, HpHigh)); // 11.2
        assert_eq!(hp_bar(20, 30), (11, HpMid)); // exactly 2/3: 10.67
        assert_eq!(hp_bar(11, 30), (6, HpMid)); // 5.87
        assert_eq!(hp_bar(10, 30), (5, HpLow)); // exactly 1/3: 5.33
        assert_eq!(hp_bar(1, 30), (1, HpLow)); // 0.53 rounds up
        assert_eq!(hp_bar(1, 40), (0, HpLow)); // 0.4 rounds down
        assert_eq!(hp_bar(0, 30), (0, HpLow));
        assert_eq!(hp_bar(1, 32), (1, HpLow)); // exactly 0.5 rounds up
        assert_eq!(hp_bar(-5, 30), (0, HpLow));
        assert_eq!(hp_bar(99, 30), (16, HpHigh));
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

    fn drawn(u: &Unit) -> GlyphBuffer {
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
            [bar(8, 11, UiColor::HpMid), bar(19, 5, UiColor::Black)]
        );
        let full = drawn(&unit("Br", 30, 30, false));
        assert_eq!(full.overlays(), [bar(8, 16, UiColor::HpHigh)]);
        let empty = drawn(&unit("Br", 0, 30, false));
        assert_eq!(empty.overlays(), [bar(8, 16, UiColor::Black)]);
    }

    #[test]
    fn acted_units_are_lowercase_and_dimmed_toward_the_background() {
        let p = game_palette();
        let bg = Rgb::new(0, 0, 100);
        let b = drawn(&unit("Br", 30, 30, true));
        let dim = p.get(UiColor::Enemy).lerp(bg, ACTED_DIM);
        assert_eq!(b.get(1, 1), Some(&Cell::new('b', dim, bg)));
        assert_eq!(b.get(2, 1), Some(&Cell::new('r', dim, bg)));
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
                Rect::new(0, 14, 8, 2),
                p.get(UiColor::HpMid),
                Layer::Over
            )]
        );
    }
}
