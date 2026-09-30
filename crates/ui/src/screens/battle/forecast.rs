//! The attack forecast (ticket 0404, `docs/design/look-and-feel.md` →
//! *Attack forecast*): while a target is chosen it replaces the side panel.
//! Both sides' names, weapons, HP (with a bar shading what each would lose
//! if every strike hit), hit and crit; then every strike in the order it
//! happens, a skull on the strike that kills, and each side's total. All
//! numbers come from [`AttackPreview`]; nothing is computed here.

use trpg_core::{
    ArtNote, AttackPreview, BattleState, Equipped, PlannedStrike, Side, SideForecast, StatValue,
    Unit,
};

use super::art_list::{art_name, durability_text, note_text};
use super::attack::{Targeting, weapon_durability, weapon_name};
use super::layout::SIDE_PANEL;
use super::panel::TEXT_X;
use super::skills::skill_name;
use super::units::{faction_color, hp_fill};
use crate::color::{Palette, Rgb, UiColor};
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Layer, Overlay, Rect};

/// The panel's title, on its top border.
pub const TITLE: &str = " Forecast ";

/// Left column (the attacker).
pub const LEFT_X: i32 = TEXT_X;

/// Right column (the target).
pub const RIGHT_X: i32 = SIDE_PANEL.x + 16;

/// Widest text in a column.
pub const COLUMN_W: usize = 13;

/// Row of the combat active's name and durability change (`Keen Edge (20 →
/// 17)`), under the title, when one is chosen (0412).
pub const SKILL_ROW: i32 = SIDE_PANEL.y + 1;

/// Widest text of the skill line, inside the panel's border.
const SKILL_W: usize = 26;

/// Row of the names; weapons, a broken marker, HP, hit and crit follow.
pub const NAME_ROW: i32 = SIDE_PANEL.y + 2;

/// Row of the HP line.
pub const HP_ROW: i32 = NAME_ROW + 3;

/// Row of the `(broken)` marker: under the weapon, above the HP line.
pub const BROKEN_ROW: i32 = HP_ROW - 1;

/// Row of the line above the strikes.
pub const RULE_ROW: i32 = HP_ROW + 4;

/// Row of the first strike.
pub const STRIKE_ROW: i32 = RULE_ROW + 1;

/// Length of each side's HP bar, in cells.
pub const HP_BAR_CELLS: i32 = 5;

/// Column of the strike numbers, between the two sides.
pub const NUMBER_X: i32 = SIDE_PANEL.x + 14;

/// The kill mark: a skull, one cell wide, `X` = a pixel of the mark.
pub const SKULL: [&str; 9] = [
    "..XXXX..", ".XXXXXX.", "XXXXXXXX", "X..XX..X", "X..XX..X", "XXX..XXX", ".XXXXXX.", ".X.XX.X.",
    "..XXXX..",
];

/// Pixel row of the skull's top inside its cell.
const SKULL_TOP: i32 = 4;

/// The skull drawn in the cell at `(x, y)` in `color`: one overlay per run
/// of pixels in a row.
pub fn skull(x: i32, y: i32, color: Rgb) -> Vec<Overlay> {
    let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
    let mut runs = Vec::new();
    for (row, line) in (0..).zip(SKULL) {
        let mut start = None;
        for (col, px) in (0..).zip(line.chars().chain(['.'])) {
            match (px == 'X', start) {
                (true, None) => start = Some(col),
                (false, Some(s)) => {
                    let rect = Rect::new(x * cw + s, y * ch + SKULL_TOP + row, col - s, 1);
                    runs.push(Overlay::new(rect, color, Layer::Over));
                    start = None;
                }
                _ => {}
            }
        }
    }
    runs
}

/// Prints `text` at `(x, y)` on the cells' background, cut to `w` chars.
fn put(buf: &mut GlyphBuffer, x: i32, y: i32, text: &str, fg: Rgb, w: usize) {
    let bg = buf.get(x, y).map_or(fg, |c| c.bg);
    let cut: String = text.chars().take(w).collect();
    buf.print(x, y, &cut, fg, bg);
}

/// The name of what `unit` fights back with: its equipped weapon or spell.
fn equipped_name(state: &BattleState, unit: &Unit) -> String {
    match &unit.loadout.equipped {
        Some(Equipped::Weapon(slot)) => weapon_name(state, unit.id, *slot),
        Some(Equipped::Spell(id)) => state
            .spells()
            .get(id)
            .map(|s| s.name.clone())
            .unwrap_or_default(),
        None => String::new(),
    }
}

/// Draws the forecast of `t` into the side panel.
pub fn draw_forecast(buf: &mut GlyphBuffer, palette: &Palette, state: &BattleState, t: &Targeting) {
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    buf.fill_rect(SIDE_PANEL, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(SIDE_PANEL, BoxStyle::Double, c(UiColor::PanelBorder), bg);
    let title_w = i32::try_from(TITLE.chars().count()).unwrap_or(0);
    let title_x = SIDE_PANEL.x + (SIDE_PANEL.w - title_w) / 2;
    put(
        buf,
        title_x,
        SIDE_PANEL.y,
        TITLE,
        c(UiColor::TextHighlight),
        TITLE.len(),
    );
    let (Some(attacker), Some(target)) = (state.unit(t.sel.unit), state.unit(t.target())) else {
        return;
    };
    let p = &t.preview;
    let durability = |unit: &Unit, slot: Option<usize>| {
        let (_, left, max) = weapon_durability(state, unit.id, slot?)?;
        Some((left, max))
    };
    let sides = [
        Column {
            x: LEFT_X,
            unit: attacker,
            weapon: weapon_name(state, attacker.id, t.slot),
            durability: durability(attacker, Some(t.slot)),
            numbers: Some(p.forecast.attacker),
            hp_after: p.plan.attacker_hp,
        },
        Column {
            x: RIGHT_X,
            unit: target,
            weapon: equipped_name(state, target),
            durability: durability(target, target.loadout.equipped_slot()),
            numbers: p.forecast.defender,
            hp_after: p.plan.defender_hp,
        },
    ];
    for side in &sides {
        draw_side(buf, palette, side);
    }
    draw_skill(buf, palette, state, t);
    let total_row = draw_strikes(buf, palette, p);
    draw_notes(buf, palette, p, total_row + 2);
}

/// The art's effects that aren't numbers, one per row from `row`
/// (`pierces`, `pins: Mov −3`…). `no counter` already shows in the
/// target's column, so it isn't repeated.
fn draw_notes(buf: &mut GlyphBuffer, palette: &Palette, p: &AttackPreview, row: i32) {
    let notes = p.notes.iter().filter(|n| **n != ArtNote::NoCounter);
    let bottom = SIDE_PANEL.y + SIDE_PANEL.h - 1;
    for (y, note) in (row..bottom).zip(notes) {
        put(
            buf,
            LEFT_X,
            y,
            &note_text(note),
            palette.get(UiColor::Effect),
            SKILL_W,
        );
    }
}

/// The chosen art's or combat active's line: its name and the weapon's
/// durability before and after (`Guard Break (20 → 16)`).
fn draw_skill(buf: &mut GlyphBuffer, palette: &Palette, state: &BattleState, t: &Targeting) {
    let p = &t.preview;
    let name = match (&p.art, &p.active) {
        (Some(art), _) => art_name(state, art),
        (None, Some(skill)) => skill_name(state, skill),
        (None, None) => return,
    };
    let cost = p
        .durability
        .map_or_else(String::new, |(from, to)| format!(" ({from} → {to})"));
    let text = format!("{name}{cost}");
    put(
        buf,
        LEFT_X,
        SKILL_ROW,
        &text,
        palette.get(UiColor::TextHighlight),
        SKILL_W,
    );
}

/// One side of the forecast, as [`draw_side`] draws it.
struct Column<'a> {
    /// Its column.
    x: i32,
    /// The unit.
    unit: &'a Unit,
    /// What it fights with.
    weapon: String,
    /// That weapon's durability left and max (none for a spell).
    durability: Option<(u32, u32)>,
    /// Its numbers (`None`: it can't strike back).
    numbers: Option<SideForecast>,
    /// Its HP if every strike hit.
    hp_after: StatValue,
}

/// One side's block: name, weapon, its durability (`20/20`, or
/// `(broken)`), HP with its bar, hit, crit.
fn draw_side(buf: &mut GlyphBuffer, palette: &Palette, side: &Column<'_>) {
    let c = |u| palette.get(u);
    let (text, dim) = (c(UiColor::Text), c(UiColor::TextDim));
    let (x, unit, numbers) = (side.x, side.unit, side.numbers);
    put(
        buf,
        x,
        NAME_ROW,
        &unit.name,
        c(faction_color(unit.faction)),
        COLUMN_W,
    );
    put(buf, x, NAME_ROW + 1, &side.weapon, dim, COLUMN_W);
    let broken = numbers.is_some_and(|n| n.broken) || side.durability.is_some_and(|d| d.0 == 0);
    if broken {
        put(buf, x, BROKEN_ROW, "(broken)", c(UiColor::HpLow), COLUMN_W);
    } else if let Some((left, max)) = side.durability {
        put(
            buf,
            x,
            BROKEN_ROW,
            &durability_text(left, max),
            dim,
            COLUMN_W,
        );
    }
    let hp_after = side.hp_after;
    put(buf, x, HP_ROW, "HP", dim, 2);
    put(buf, x + 3, HP_ROW, &format!("{:>2}", unit.hp), text, 3);
    draw_bar(buf, palette, x + 6, unit.hp, hp_after, unit.stats.hp);
    let shown = |v: Option<u8>| v.map_or_else(|| "--".to_owned(), |v| v.to_string());
    put(buf, x, HP_ROW + 1, "Hit", dim, 3);
    put(
        buf,
        x + 5,
        HP_ROW + 1,
        &shown(numbers.map(|n| n.hit)),
        text,
        3,
    );
    put(buf, x, HP_ROW + 2, "Crit", dim, 4);
    put(
        buf,
        x + 5,
        HP_ROW + 2,
        &shown(numbers.map(|n| n.crit)),
        text,
        3,
    );
}

/// The HP bar at `(x, HP_ROW)`: `after` HP filled in its HP colour, the HP
/// that would be lost (`hp - after`) shaded in `hp_low`, the rest empty.
fn draw_bar(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    x: i32,
    hp: StatValue,
    after: StatValue,
    max: StatValue,
) {
    let c = |u| palette.get(u);
    let (kept, color) = hp_fill(after, max, HP_BAR_CELLS);
    let (now, _) = hp_fill(hp.max(after), max, HP_BAR_CELLS);
    let bg = c(UiColor::PanelBg);
    for i in 0..HP_BAR_CELLS {
        let (glyph, fg) = if i < kept {
            ('█', c(color))
        } else if i < now {
            ('▒', c(UiColor::HpLow))
        } else {
            ('░', c(UiColor::TextDim))
        };
        buf.set(x + i, HP_ROW, Cell::new(glyph, fg, bg));
    }
}

/// The strike list: one row per strike in combat order, numbered in the
/// middle; the attacker's on the left (`8 dmg →`), the target's on the
/// right (`← 9 dmg`), `!` after an effective strike's `dmg`, a skull on
/// the kill and strikes after it dimmed; then each side's total. Returns
/// the totals' row.
fn draw_strikes(buf: &mut GlyphBuffer, palette: &Palette, p: &AttackPreview) -> i32 {
    let c = |u| palette.get(u);
    let (text, dim) = (c(UiColor::Text), c(UiColor::TextDim));
    let rule = c(UiColor::PanelBorder);
    let rule_w = usize::try_from(SIDE_PANEL.w - 4).unwrap_or(0);
    let line = "─".repeat(rule_w);
    put(buf, LEFT_X, RULE_ROW, &line, rule, rule_w);
    let strikes = &p.plan.strikes;
    for (y, (n, s)) in (STRIKE_ROW..).zip(strikes.iter().enumerate()) {
        let numbers = match s.by {
            Side::Attacker => Some(p.forecast.attacker),
            Side::Defender => p.forecast.defender,
        };
        let effective = numbers.is_some_and(|f| f.effective);
        put(buf, NUMBER_X, y, &(n + 1).to_string(), dim, 1);
        let fg = if s.after_a_fall { dim } else { text };
        let (dmg_x, arrow_x, arrow) = match s.by {
            Side::Attacker => (LEFT_X, LEFT_X + 9, "→"),
            Side::Defender => (RIGHT_X + 2, RIGHT_X, "←"),
        };
        put(buf, dmg_x, y, &strike_text(s), fg, 7);
        put(buf, arrow_x, y, arrow, dim, 1);
        if effective {
            put(buf, dmg_x + 6, y, "!", c(UiColor::TextHighlight), 1);
        }
        if s.kills() {
            for o in skull(dmg_x + 7, y, c(UiColor::HpLow)) {
                buf.add_overlay(o);
            }
        }
    }
    if p.forecast.defender.is_none() {
        put(buf, RIGHT_X + 2, STRIKE_ROW, "no counter", dim, 11);
    }
    let rows = i32::try_from(strikes.len()).unwrap_or(0);
    let total_row = STRIKE_ROW + rows + 1;
    put(buf, LEFT_X, total_row - 1, &line, rule, rule_w);
    put(buf, NUMBER_X - 2, total_row, "Total", dim, 5);
    let total = |by: Side| {
        let mine = strikes.iter().filter(|s| s.by == by);
        let dmg: StatValue = mine.clone().map(|s| s.damage).sum();
        format!("{dmg:>2} ×{}", mine.count())
    };
    put(buf, LEFT_X, total_row, &total(Side::Attacker), text, 7);
    if p.forecast.defender.is_some() {
        put(buf, RIGHT_X + 2, total_row, &total(Side::Defender), text, 7);
    }
    total_row
}

/// A strike's damage as listed: `8 dmg`, or `+9 hp` for an Absorb heal.
pub fn strike_text(s: &PlannedStrike) -> String {
    if s.healed {
        format!("+{:>1} hp", s.damage)
    } else {
        format!("{:>2} dmg", s.damage)
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{Pos, UnitId};

    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::screen::tests::ctx;
    use crate::screens::battle::attack::weapon_choices;
    use crate::screens::battle::mode::Selection;
    use crate::screens::battle::testing::skirmish;

    /// The skirmish's lord at (7, 2) targeting the brigand (at
    /// `brigand_hp`) with its iron sword.
    fn targeting(state: &BattleState) -> Targeting {
        let mut sel = Selection::new(state, UnitId(1)).unwrap();
        sel.path = vec![sel.origin(), Pos::new(7, 2)];
        let choice = &weapon_choices(state, &sel)[0];
        let mut t = Targeting::new(state, sel, choice, None).unwrap();
        t.cycle(true, state);
        assert_eq!(t.target(), UnitId(4));
        t
    }

    fn render(state: &BattleState, t: &Targeting) -> GlyphBuffer {
        let c = ctx();
        let blank = Cell::new(' ', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank);
        draw_forecast(&mut buf, &c.palette, state, t);
        buf
    }

    fn text(buf: &GlyphBuffer, x: i32, y: i32, n: i32) -> String {
        (x..x + n).map(|x| buf.get(x, y).unwrap().glyph).collect()
    }

    #[test]
    fn the_skull_is_one_overlay_per_run_of_pixels_inside_its_cell() {
        let red = Rgb::new(200, 0, 0);
        let runs = skull(3, 2, red);
        let pixels: i32 = runs.iter().map(|o| o.rect.w * o.rect.h).sum();
        let drawn = SKULL.iter().flat_map(|l| l.chars()).filter(|&c| c == 'X');
        assert_eq!(pixels, i32::try_from(drawn.count()).unwrap());
        let (cw, ch) = (i32::from(CELL_W_PX), i32::from(CELL_H_PX));
        for o in &runs {
            assert_eq!((o.color, o.layer, o.rect.h), (red, Layer::Over, 1));
            assert!(o.rect.x >= 3 * cw && o.rect.x + o.rect.w <= 4 * cw);
            assert!(o.rect.y >= 2 * ch && o.rect.y < 3 * ch);
        }
        // The top row: one run of 4 pixels, 2 in from the left.
        assert_eq!(runs[0].rect, Rect::new(3 * cw + 2, 2 * ch + 4, 4, 1));
        // A row with gaps: `X..XX..X` is three runs.
        let eyes: Vec<_> = runs.iter().filter(|o| o.rect.y == 2 * ch + 4 + 3).collect();
        assert_eq!(eyes.len(), 3);
    }

    #[test]
    fn strikes_read_as_damage_or_a_heal() {
        let s = PlannedStrike {
            by: Side::Attacker,
            damage: 8,
            healed: false,
            target_hp_after: 4,
            after_a_fall: false,
        };
        assert_eq!(strike_text(&s), " 8 dmg");
        assert_eq!(strike_text(&PlannedStrike { damage: 12, ..s }), "12 dmg");
        assert_eq!(strike_text(&PlannedStrike { healed: true, ..s }), "+8 hp");
    }

    #[test]
    fn an_effective_strike_is_marked_and_a_broken_weapon_says_so() {
        let c = ctx();
        let state = skirmish(&c, 20);
        let mut t = targeting(&state);
        let plain = render(&state, &t);
        assert_eq!(text(&plain, LEFT_X + 6, STRIKE_ROW, 1), " ");
        // Under the weapon: its durability, until it is broken.
        assert_eq!(text(&plain, LEFT_X, NAME_ROW + 2, 8), "20/20   ");
        t.preview.forecast.attacker.effective = true;
        t.preview.forecast.attacker.broken = true;
        let marked = render(&state, &t);
        // `!` after both of the lord's strikes, in the highlight colour.
        for row in [STRIKE_ROW, STRIKE_ROW + 2] {
            let cell = marked.get(LEFT_X + 6, row).unwrap();
            assert_eq!(cell.glyph, '!');
            assert_eq!(cell.fg, c.palette.get(UiColor::TextHighlight));
        }
        // Two rows under the name, right under the weapon.
        assert_eq!(text(&marked, LEFT_X, NAME_ROW + 2, 8), "(broken)");
        assert_eq!(text(&marked, RIGHT_X + 8, STRIKE_ROW + 1, 1), " ");
    }

    #[test]
    fn strikes_after_the_kill_are_dimmed_and_the_bars_shade_the_loss() {
        let c = ctx();
        let p = &c.palette;
        // The brigand at 3 HP falls to the first strike.
        let state = skirmish(&c, 3);
        let t = targeting(&state);
        let buf = render(&state, &t);
        let fg = |x, y| buf.get(x, y).unwrap().fg;
        assert_eq!(fg(LEFT_X + 1, STRIKE_ROW), p.get(UiColor::Text));
        assert_eq!(fg(RIGHT_X + 3, STRIKE_ROW + 1), p.get(UiColor::TextDim));
        assert_eq!(fg(LEFT_X + 1, STRIKE_ROW + 2), p.get(UiColor::TextDim));
        // The skull on the first strike only.
        let skulls: Vec<i32> = buf
            .overlays()
            .iter()
            .filter(|o| o.color == p.get(UiColor::HpLow))
            .map(|o| o.rect.y / i32::from(CELL_H_PX))
            .collect();
        assert!(!skulls.is_empty() && skulls.iter().all(|&y| y == STRIKE_ROW));
        // The lord keeps all its HP (5 cells); the brigand's 3 of 20 HP
        // round to 1 cell, all of it lost.
        let bar = |x| text(&buf, x + 6, HP_ROW, HP_BAR_CELLS);
        assert_eq!(bar(LEFT_X), "█████");
        assert_eq!(bar(RIGHT_X), "▒░░░░");
        // The title sits in the top border.
        assert!(text(&buf, SIDE_PANEL.x, SIDE_PANEL.y, 30).contains(TITLE));
    }

    #[test]
    fn notes_skip_no_counter_and_stop_above_the_bottom_border() {
        let c = ctx();
        let state = skirmish(&c, 20);
        let mut p = targeting(&state).preview;
        p.notes = vec![ArtNote::NoCounter, ArtNote::Pierces, ArtNote::Pierces];
        let blank = Cell::new(' ', Rgb::new(1, 2, 3), Rgb::new(1, 2, 3));
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank);
        let border = SIDE_PANEL.y + SIDE_PANEL.h - 1;
        draw_notes(&mut buf, &c.palette, &p, border - 1);
        // `no counter` isn't repeated; one row is left for the two notes.
        assert_eq!(text(&buf, LEFT_X, border - 1, 7), "pierces");
        assert_eq!(text(&buf, LEFT_X, border, 7), "       ");
        assert_eq!(text(&buf, LEFT_X, border + 1, 7), "       ");
    }

    #[test]
    fn a_spell_or_nothing_equipped_names_the_counter_weapon_that_way() {
        let c = ctx();
        let state = skirmish(&c, 20);
        let mut units = state.units().to_vec();
        units[3].loadout.equipped = None;
        let brigand = &units[3];
        assert_eq!(equipped_name(&state, brigand), "");
        let mut mage = brigand.clone();
        let spell = c.content.spells.spells.keys().next().unwrap().clone();
        mage.loadout.equipped = Some(Equipped::Spell(spell.clone()));
        let name = c.content.spells.get(&spell).unwrap().name.clone();
        assert_eq!(equipped_name(&state, &mage), name);
    }
}
