//! The unit info screen (ticket 0405): everything about one unit, over the
//! map and side panel. A 24 × 12 portrait area on the left (a placeholder
//! box until portraits exist, 0703), then the unit's name, level, EXP, HP
//! and stats as plain numbers (`Str 7`; no class caps, ticket 0423), and on
//! the right its loadout, spells and skills.

use trpg_core::skill::effect_bonuses;
use trpg_core::{BattleState, EffectSource, StatKind, Stats, TimedEffect, Unit, WEAPON_SLOTS};

use super::layout::MAP_VIEW;
use super::skills::{cost_text, effect_text, skill_cost, skill_name, timed_text, until_text};
use super::units::{faction_color, hp_fill};
use crate::color::{Palette, Rgb, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};

/// The whole screen above the help bar.
pub const INFO: Rect = Rect::new(0, 0, 100, MAP_VIEW.h);

/// The portrait area.
pub const PORTRAIT: Rect = Rect::new(2, 1, 24, 12);

/// Left column (under the portrait): movement, tags, weapon ranks.
const LEFT_X: i32 = 3;

/// Middle column: name, level, HP, stats, skills.
const MID_X: i32 = 29;

/// Right column: loadout and spells.
const RIGHT_X: i32 = 56;

/// Widest text in the right column, so nothing reaches the border.
const RIGHT_W: usize = 42;

/// Widest item name or bonus, indented two cells in the right column.
const ITEM_W: usize = 40;

/// Most skills listed, two rows each: name and cost, then the effect (rows
/// 16..28).
const SKILL_ROWS: usize = 12;

/// Column of the timed effects, right of the stats.
const EFFECT_X: i32 = 37;

/// Widest text in the effects column.
const EFFECT_W: usize = 18;

/// Most timed effects listed, three rows each (rows 7..13).
const EFFECT_ROWS: usize = 6;

/// Most spells listed (rows 18..28).
const SPELL_ROWS: usize = 10;

/// Length of the HP bar, in cells.
const HP_BAR_CELLS: i32 = 10;

/// The stat's short name, as the design docs write it.
pub const fn stat_name(kind: StatKind) -> &'static str {
    match kind {
        StatKind::Hp => "HP",
        StatKind::Str => "Str",
        StatKind::Mag => "Mag",
        StatKind::Dex => "Dex",
        StatKind::Spd => "Spd",
        StatKind::Def => "Def",
        StatKind::Res => "Res",
        StatKind::Mov => "Mov",
    }
}

/// A gear bonus as text: its non-zero stats, `Def +2  Res +1`.
pub fn bonus_text(bonus: &Stats) -> String {
    StatKind::ALL
        .iter()
        .filter(|&&k| bonus.get(k) != 0)
        .map(|&k| format!("{} +{}", stat_name(k), bonus.get(k)))
        .collect::<Vec<_>>()
        .join("  ")
}

/// A range as text: `1` or `1-2`.
fn range_text(min: u32, max: u32) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

/// Where the screen is being drawn and in which colours.
struct Pen<'a> {
    buf: &'a mut GlyphBuffer,
    palette: &'a Palette,
    bg: Rgb,
}

impl Pen<'_> {
    fn text(&mut self, x: i32, y: i32, text: &str, color: UiColor) -> i32 {
        let fg = self.palette.get(color);
        let n = self.buf.print(x, y, text, fg, self.bg);
        x + i32::from(n)
    }

    /// `text`, cut to `width` characters.
    fn cut(&mut self, x: i32, y: i32, text: &str, width: usize, color: UiColor) {
        let cut: String = text.chars().take(width).collect();
        self.text(x, y, &cut, color);
    }
}

/// Draws the info screen for `unit` of `state`.
pub fn draw_info(buf: &mut GlyphBuffer, palette: &Palette, state: &BattleState, unit: &Unit) {
    let bg = palette.get(UiColor::PanelBg);
    buf.fill_rect(INFO, Cell::new(' ', palette.get(UiColor::Text), bg));
    buf.draw_box(
        INFO,
        BoxStyle::Double,
        palette.get(UiColor::PanelBorder),
        bg,
    );
    buf.draw_box(
        PORTRAIT,
        BoxStyle::Single,
        palette.get(UiColor::TextDim),
        bg,
    );
    let mut pen = Pen { buf, palette, bg };
    let label = "portrait";
    let w = i32::try_from(label.len()).unwrap_or(0);
    pen.text(
        PORTRAIT.x + (PORTRAIT.w - w) / 2,
        PORTRAIT.y + PORTRAIT.h / 2,
        label,
        UiColor::TextDim,
    );
    draw_left(&mut pen, state, unit);
    draw_middle(&mut pen, state, unit);
    draw_right(&mut pen, state, unit);
}

/// Under the portrait: Mov and movement type, class tags, weapon ranks.
fn draw_left(pen: &mut Pen<'_>, state: &BattleState, unit: &Unit) {
    let class = state.classes().get(&unit.class);
    let mut y = PORTRAIT.y + PORTRAIT.h + 1;
    let x = pen.text(LEFT_X, y, "Mov ", UiColor::TextDim);
    let x = pen.text(x, y, &unit.move_points().to_string(), UiColor::Text);
    if let Some(name) = class.and_then(|c| {
        state
            .terrain()
            .movement_types
            .get(usize::from(c.movement_type.0))
    }) {
        pen.text(x + 2, y, name, UiColor::Text);
    }
    y += 1;
    let tags = class.map(|c| c.tags).unwrap_or_default();
    let names: Vec<&str> = [
        (tags.mounted, "Mounted"),
        (tags.flying, "Flying"),
        (tags.armored, "Armored"),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .map(|&(_, name)| name)
    .collect();
    if !names.is_empty() {
        pen.text(LEFT_X, y, &names.join(" "), UiColor::Text);
    }
    y += 2;
    pen.text(LEFT_X, y, "Weapon ranks", UiColor::TextHighlight);
    for (kind, rank) in &unit.weapon_ranks {
        y += 1;
        let x = pen.text(LEFT_X, y, &format!("{kind:?}"), UiColor::Text);
        pen.text(x.max(LEFT_X + 10), y, &format!("{rank:?}"), UiColor::Text);
    }
}

/// Name, class and level, EXP, HP with a bar, stats, skills. Max HP is
/// shown on the HP line, so the stat list leaves HP out.
fn draw_middle(pen: &mut Pen<'_>, state: &BattleState, unit: &Unit) {
    let class = state.classes().get(&unit.class);
    let x = MID_X;
    pen.cut(x, 1, &unit.name, 25, faction_color(unit.faction));
    let class_name = class.map_or(unit.class.0.as_str(), |c| c.name.as_str());
    pen.cut(
        x,
        2,
        &format!("{class_name}  Lv {}", unit.level),
        25,
        UiColor::Text,
    );
    let at_cap = unit.level >= state.classes().level_cap;
    let exp = if at_cap {
        "--".to_owned()
    } else {
        unit.exp.to_string()
    };
    let ex = pen.text(x, 3, "EXP ", UiColor::TextDim);
    pen.text(ex, 3, &exp, UiColor::Text);
    let hx = pen.text(x, 4, "HP ", UiColor::TextDim);
    pen.text(
        hx,
        4,
        &format!("{}/{}", unit.hp, unit.stats.hp),
        UiColor::Text,
    );
    let (filled, color) = hp_fill(unit.hp, unit.stats.hp, HP_BAR_CELLS);
    for i in 0..HP_BAR_CELLS {
        let (glyph, fg) = if i < filled {
            ('█', color)
        } else {
            ('░', UiColor::TextDim)
        };
        pen.text(x + 11 + i, 4, &glyph.to_string(), fg);
    }
    pen.text(x, 6, "Stats", UiColor::TextHighlight);
    let listed = StatKind::GROWABLE
        .into_iter()
        .filter(|&k| k != StatKind::Hp);
    // Timed effects count: a raised or lowered stat shows in the effect
    // colour.
    let shown = effect_bonuses(&unit.effects).apply(unit.stats);
    for (y, kind) in (7..).zip(listed) {
        pen.text(x, y, stat_name(kind), UiColor::Text);
        let value = format!("{:>3}", shown.get(kind));
        let color = if shown.get(kind) == unit.stats.get(kind) {
            UiColor::Text
        } else {
            UiColor::Effect
        };
        pen.text(x + 4, y, &value, color);
    }
    draw_effects(pen, state, unit);
    draw_skills(pen, state, unit);
}

/// The unit's timed effects right of the stats: name, what it changes, and
/// until when it lasts (0412).
fn draw_effects(pen: &mut Pen<'_>, state: &BattleState, unit: &Unit) {
    if unit.effects.is_empty() {
        return;
    }
    pen.text(EFFECT_X, 6, "Effects", UiColor::TextHighlight);
    let mut y = 7;
    for effect in unit.effects.iter().take(EFFECT_ROWS / 3) {
        pen.cut(
            EFFECT_X,
            y,
            &effect_name(state, effect),
            EFFECT_W,
            UiColor::Effect,
        );
        pen.cut(
            EFFECT_X,
            y + 1,
            &timed_text(&effect.mods),
            EFFECT_W,
            UiColor::Text,
        );
        pen.cut(
            EFFECT_X,
            y + 2,
            &until_text(effect),
            EFFECT_W,
            UiColor::TextDim,
        );
        y += 3;
    }
}

/// What gave a timed effect: a skill's or an art's id, as a name.
fn effect_name(state: &BattleState, effect: &TimedEffect) -> String {
    match &effect.source {
        EffectSource::Skill(id) => skill_name(state, id),
        EffectSource::Art(id) => state
            .arts()
            .get(id)
            .map_or_else(|| id.0.clone(), |a| a.name.clone()),
    }
}

/// The skills block: `P` (passive, always on) or `A` (active) markers, the
/// rank, the cost of an active, and each effect in a line under it.
fn draw_skills(pen: &mut Pen<'_>, state: &BattleState, unit: &Unit) {
    let x = MID_X;
    pen.text(x, 15, "Skills", UiColor::TextHighlight);
    let skills = unit.usable_skills(state.classes(), state.skills());
    if skills.is_empty() {
        pen.text(x, 16, "--", UiColor::TextDim);
    }
    for (y, skill) in (16..)
        .step_by(2)
        .zip(skills.into_iter().take(SKILL_ROWS / 2))
    {
        let cost = skill_cost(skill);
        pen.text(
            x,
            y,
            if cost.is_some() { "A" } else { "P" },
            UiColor::TextHighlight,
        );
        let name_w = if cost.is_some() { 16 } else { 23 };
        pen.cut(x + 2, y, &skill.name, name_w, UiColor::Text);
        if let Some(cost) = cost {
            let text = cost_text(cost);
            let w = i32::try_from(text.len()).unwrap_or(0);
            pen.text(x + 25 - w, y, &text, UiColor::TextDim);
        }
        pen.cut(x + 2, y + 1, &effect_text(skill), 23, UiColor::TextDim);
    }
}

/// The three weapon slots (the equipped one marked `E`), armour, accessory
/// and spells with uses left.
fn draw_right(pen: &mut Pen<'_>, state: &BattleState, unit: &Unit) {
    let x = RIGHT_X;
    let items = state.items();
    let loadout = &unit.loadout;
    pen.text(x, 1, "Weapons", UiColor::TextHighlight);
    let mut y = 2;
    for slot in 0..WEAPON_SLOTS {
        let weapon = loadout
            .weapon(slot)
            .and_then(|w| Some((w, items.weapon(&w.def)?)));
        match weapon {
            Some((inst, def)) => {
                let mark = if loadout.equipped_slot() == Some(slot) {
                    "E "
                } else {
                    "  "
                };
                let color = if inst.is_broken() {
                    UiColor::HpLow
                } else {
                    UiColor::Text
                };
                pen.text(x, y, mark, UiColor::TextHighlight);
                pen.cut(x + 2, y, &def.name, 28, color);
                let dur = format!("{:>2}/{}", inst.durability_left, def.durability);
                let dw = i32::try_from(dur.len()).unwrap_or(0);
                let right = x + i32::try_from(RIGHT_W).unwrap_or(0);
                pen.text(right - dw, y, &dur, color);
                let numbers = format!(
                    "Mt {}  Hit {}  Crit {}  Rng {}  Wt {}",
                    def.might,
                    def.hit,
                    def.crit,
                    range_text(def.min_range, def.max_range),
                    def.weight
                );
                pen.cut(x + 2, y + 1, &numbers, ITEM_W, UiColor::TextDim);
            }
            None => {
                pen.text(x + 2, y, "--", UiColor::TextDim);
            }
        }
        y += 2;
    }
    y += 1;
    let gear = [
        (
            "Armour",
            loadout
                .armour
                .as_ref()
                .and_then(|id| items.armour(id))
                .map(|a| (a.name.as_str(), bonus_text(&a.bonus))),
        ),
        (
            "Accessory",
            loadout
                .accessory
                .as_ref()
                .and_then(|id| items.accessory(id))
                .map(|a| (a.name.as_str(), bonus_text(&a.bonus))),
        ),
    ];
    for (title, piece) in gear {
        pen.text(x, y, title, UiColor::TextHighlight);
        match piece {
            Some((name, bonus)) => {
                pen.cut(x + 2, y + 1, name, ITEM_W, UiColor::Text);
                pen.cut(x + 2, y + 2, &bonus, ITEM_W, UiColor::TextDim);
            }
            None => {
                pen.text(x + 2, y + 1, "--", UiColor::TextDim);
            }
        }
        y += 4;
    }
    pen.text(x, y, "Spells", UiColor::TextHighlight);
    let spells: Vec<_> = unit
        .learned
        .iter()
        .filter_map(|id| state.spells().get(id))
        .collect();
    if spells.is_empty() {
        pen.text(x + 2, y + 1, "--", UiColor::TextDim);
    }
    for (row, spell) in (y + 1..).zip(spells.into_iter().take(SPELL_ROWS)) {
        let uses = unit.spells.uses_left(&spell.id);
        pen.cut(x + 2, row, &spell.name, 28, UiColor::Text);
        let text = format!("{uses}/{}", spell.uses);
        let w = i32::try_from(text.len()).unwrap_or(0);
        pen.text(
            x + i32::try_from(RIGHT_W).unwrap_or(0) - w,
            row,
            &text,
            UiColor::Text,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_names_bonuses_and_ranges() {
        assert_eq!(
            StatKind::ALL.map(stat_name),
            ["HP", "Str", "Mag", "Dex", "Spd", "Def", "Res", "Mov"]
        );
        let mut bonus = Stats::default();
        assert_eq!(bonus_text(&bonus), "");
        bonus.def = 2;
        bonus.res = 1;
        assert_eq!(bonus_text(&bonus), "Def +2  Res +1");
        assert_eq!(
            (range_text(1, 1), range_text(1, 2)),
            ("1".into(), "1-2".into())
        );
    }

    #[test]
    fn the_columns_fit_inside_the_border() {
        const { assert!(PORTRAIT.x + PORTRAIT.w < MID_X) };
        assert!(RIGHT_X + i32::try_from(RIGHT_W).unwrap() < INFO.x + INFO.w - 1);
        const { assert!(ITEM_W + 2 <= RIGHT_W) };
        // The lists end above the bottom border (row 29).
        assert!(16 + i32::try_from(SKILL_ROWS).unwrap() < INFO.h);
        assert!(18 + i32::try_from(SPELL_ROWS).unwrap() < INFO.h);
    }
}
