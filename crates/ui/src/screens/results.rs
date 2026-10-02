//! The results of a won battle (ticket 0810,
//! `docs/design/death-and-difficulty.md`, *Results screen*): one screen with
//! the clear gold, the rewind charges left and the EXP bonus they gave, and
//! every deployed unit's EXP bar filling at once; then the level-up pages
//! (0602) of the units the bonus levelled, one at a time.
//!
//! Everything shown comes from `core`'s [`BattleRewards`]; this only times
//! and draws it. Confirm or Cancel fills the bars at once, and holding
//! Confirm fills them [`ProgressTimings::fast`] times as fast; once they are
//! full a press goes on. Each level-up page still waits for its own press
//! (Nick), as after a combat.

use trpg_core::progression::EXP_PER_LEVEL;
use trpg_core::{BattleRewards, BattleState, Gold, Level};

use super::battle::layout::MAP_VIEW;
use super::battle::playing_help;
use super::battle::progress::{
    self, EXP_BAR_CELLS, PROGRESS_TIMINGS, Progress, ProgressTimings, exp_line,
};
use super::{centre_x, print_centred};
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};

/// The screen's heading, on the panel's border.
pub const HEADING: &str = "VICTORY";
/// The gold line.
pub const GOLD_LABEL: &str = "Gold for clearing the map";
/// The rewind charges line.
pub const REWINDS_LABEL: &str = "Rewinds unused";
/// The bonus line.
pub const BONUS_LABEL: &str = "Bonus EXP for each unit";
/// Marks a unit the bonus levelled.
pub const LEVEL_UP: &str = "LEVEL UP";

/// Seconds before the bars start to fill. *Tunable.*
pub const INTRO_S: f32 = 0.5;
/// Seconds the bars take to fill. *Tunable.*
pub const FILL_S: f32 = 1.0;

/// The panel.
pub const PANEL: Rect = Rect::new(14, 3, 72, 24);
/// Column of the labels and the unit names.
const X: i32 = PANEL.x + 4;
/// Column of the numbers on the gold, rewinds and bonus lines.
const VALUE_X: i32 = X + 44;
/// Row of the gold line.
const GOLD_ROW: i32 = PANEL.y + 2;
/// Row of the rewinds line; the bonus line is the next one.
const REWINDS_ROW: i32 = PANEL.y + 4;
/// Row of the rule between the lines and the units.
const RULE_ROW: i32 = PANEL.y + 7;
/// Row of the first unit.
pub const UNIT_ROW: i32 = PANEL.y + 9;
/// Unit rows the panel has room for.
const UNIT_ROWS: i32 = PANEL.y + PANEL.h - 1 - UNIT_ROW;
/// Columns of a unit row, from [`X`]: class, level, `EXP`, bar, number, mark.
const COLS: [i32; 6] = [14, 23, 29, 33, 54, 58];
/// Widest name and class shown.
const NAME_W: usize = 13;
const CLASS_W: usize = 8;

/// One deployed unit's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Its name.
    pub name: String,
    /// Its class's name.
    pub class: String,
    /// Its level before the bonus.
    pub level: Level,
    /// Its EXP towards the next level before the bonus.
    pub exp: u32,
    /// The bonus EXP it got.
    pub gained: u32,
}

/// The results of a won battle. Pops once the player has seen them and
/// every level-up page.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultsScreen {
    clear_gold: Gold,
    gold: Gold,
    unused: u8,
    charges: u8,
    bonus: u32,
    rows: Vec<Row>,
    /// Seconds since the screen opened.
    t: f32,
    timings: ProgressTimings,
    /// Seconds before the bars start to fill, and seconds they take.
    intro_s: f32,
    fill_s: f32,
    /// The level-up pages still to show after the bars.
    pages: Option<Progress>,
    /// Whether the bars are done with and the pages are up.
    paging: bool,
}

impl ResultsScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "results";

    /// The results `rewards` of the won battle `state`, which began with
    /// `charges` rewind charges; `gold` is the party's gold now.
    pub fn new(rewards: &BattleRewards, state: &BattleState, charges: u8, gold: Gold) -> Self {
        let class = |u: &trpg_core::Unit| {
            let def = state.classes().get(&u.class);
            def.map_or_else(|| u.class.0.clone(), |c| c.name.clone())
        };
        // No unused charge, no bonus: no bars to show.
        let deployed = rewards.deployed.iter().filter(|_| rewards.bonus_exp > 0);
        let rows = deployed
            .map(|u| Row {
                name: u.name.clone(),
                class: class(u),
                level: u.level,
                exp: u.exp,
                gained: rewards.exp_gained(u.id),
            })
            .collect();
        let timings = PROGRESS_TIMINGS;
        let pages = Progress::new(&rewards.events, &rewards.deployed, state, timings)
            .and_then(Progress::without_exp_bars);
        Self {
            clear_gold: rewards.clear_gold,
            gold,
            unused: rewards.unused_charges,
            charges,
            bonus: rewards.bonus_exp,
            rows,
            t: 0.0,
            timings,
            intro_s: INTRO_S,
            fill_s: FILL_S,
            pages,
            paging: false,
        }
    }

    /// The same screen with its bars waiting `intro_s` seconds and filling
    /// over `fill_s` (the game uses [`INTRO_S`] and [`FILL_S`]).
    #[must_use]
    pub fn with_timings(mut self, intro_s: f32, fill_s: f32) -> Self {
        (self.intro_s, self.fill_s) = (intro_s, fill_s);
        self
    }

    /// Seconds from the screen opening to full bars.
    fn full_at(&self) -> f32 {
        self.intro_s + self.fill_s
    }

    /// The units' rows.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The level-up pages after the bars, if the bonus levelled anyone.
    pub fn pages(&self) -> Option<&Progress> {
        self.pages.as_ref()
    }

    /// Whether the level-up pages are up (the bars are done with).
    pub fn paging(&self) -> bool {
        self.paging
    }

    /// Whether the bars are full (at once, when there are none).
    pub fn filled(&self) -> bool {
        self.rows.is_empty() || self.t >= self.full_at()
    }

    /// How much of each unit's bonus the bars show: 0 to 1.
    fn fill(&self) -> f32 {
        if self.filled() {
            return 1.0;
        }
        ((self.t - self.intro_s) / self.fill_s).clamp(0.0, 1.0)
    }

    /// The EXP `row`'s bar shows, counted from its old EXP (so past
    /// [`EXP_PER_LEVEL`] once it has levelled).
    pub fn exp_shown(&self, row: &Row) -> u32 {
        // Bonuses are small (at most a few levels): exact in f32.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let moved = ((row.gained as f32 * self.fill()) + 1e-3).floor() as u32;
        row.exp.saturating_add(moved.min(row.gained))
    }

    /// Confirm or Cancel: fills the bars, or goes on from full bars (to the
    /// level-up pages, or closing), or presses the level-up page. Returns
    /// whether the screen is over.
    fn press(&mut self) -> bool {
        if self.paging {
            if let Some(pages) = &mut self.pages {
                pages.confirm();
            }
        } else if self.filled() {
            self.paging = true;
        } else {
            self.t = self.full_at();
        }
        self.over()
    }

    /// Whether the bars and every page are done with.
    fn over(&self) -> bool {
        self.paging && self.pages.as_ref().is_none_or(Progress::done)
    }

    /// The bottom help line.
    pub fn help(&self, ctx: &Ctx) -> String {
        let played = match &self.pages {
            Some(pages) if self.paging => pages.page_played(),
            _ => self.filled(),
        };
        playing_help(played, ctx.help_keys())
    }

    /// The gold, rewinds and bonus lines and the units' bars.
    fn draw_tally(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let (text, dim, hi) = (
            c(UiColor::Text),
            c(UiColor::TextDim),
            c(UiColor::TextHighlight),
        );
        buf.fill_rect(PANEL, Cell::new(' ', text, bg));
        buf.draw_box(PANEL, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
        buf.print(PANEL.x + 2, PANEL.y, &format!(" {HEADING} "), hi, bg);
        if self.clear_gold > 0 {
            buf.print(X, GOLD_ROW, GOLD_LABEL, text, bg);
            let n = buf.print(VALUE_X, GOLD_ROW, &format!("+{}", self.clear_gold), hi, bg);
            let now = format!("(now {})", self.gold);
            buf.print(VALUE_X + i32::from(n) + 1, GOLD_ROW, &now, dim, bg);
        }
        buf.print(X, REWINDS_ROW, REWINDS_LABEL, text, bg);
        let left = format!("{} of {}", self.unused, self.charges);
        buf.print(VALUE_X, REWINDS_ROW, &left, hi, bg);
        if self.rows.is_empty() {
            return;
        }
        buf.print(X, REWINDS_ROW + 1, BONUS_LABEL, text, bg);
        let bonus = format!("+{}", self.bonus);
        buf.print(VALUE_X, REWINDS_ROW + 1, &bonus, hi, bg);
        for x in X..PANEL.x + PANEL.w - 4 {
            buf.set(x, RULE_ROW, Cell::new('─', c(UiColor::PanelBorder), bg));
        }
        // A blank row between units while they all fit that way.
        let count = i32::try_from(self.rows.len()).unwrap_or(i32::MAX);
        let step: usize = if count * 2 - 1 <= UNIT_ROWS { 2 } else { 1 };
        let rows = (0..UNIT_ROWS).step_by(step).zip(&self.rows);
        for (dy, row) in rows {
            let y = UNIT_ROW + dy;
            let shown = self.exp_shown(row);
            let levels = shown / EXP_PER_LEVEL;
            let name: String = row.name.chars().take(NAME_W).collect();
            let class: String = row.class.chars().take(CLASS_W).collect();
            buf.print(X, y, &name, c(UiColor::Player), bg);
            buf.print(X + COLS[0], y, &class, dim, bg);
            let lv = format!("Lv {}", row.level.saturating_add(levels));
            buf.print(X + COLS[1], y, &lv, text, bg);
            buf.print(X + COLS[2], y, "EXP", dim, bg);
            let (filled, number) = exp_line(shown);
            for i in 0..EXP_BAR_CELLS {
                let (glyph, fg) = if i < filled {
                    ('█', c(UiColor::ExpBar))
                } else {
                    ('░', dim)
                };
                buf.set(X + COLS[3] + i, y, Cell::new(glyph, fg, bg));
            }
            buf.print(X + COLS[4], y, &format!("{number:>2}"), text, bg);
            if levels > 0 {
                buf.print(X + COLS[5], y, LEVEL_UP, hi, bg);
            }
        }
    }
}

impl Screen for ResultsScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, _ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for action in &input.actions {
            if matches!(action, Action::Confirm | Action::Cancel) && self.press() {
                return Transition::Pop;
            }
        }
        let held = input.is_held(Action::Confirm);
        let dt = if input.dt.is_finite() {
            input.dt.max(0.0)
        } else {
            0.0
        };
        if self.paging {
            if let Some(pages) = &mut self.pages {
                pages.tick(dt, held);
            }
            if self.over() {
                return Transition::Pop;
            }
        } else if !self.filled() {
            let speed = if held { self.timings.fast } else { 1.0 };
            self.t = (self.t + dt * speed).min(self.full_at());
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let blank = Cell::new(' ', c(UiColor::Text), c(UiColor::Black));
        buf.fill_rect(buf.bounds(), blank);
        match &self.pages {
            Some(pages) if self.paging => {
                // The pages are laid out on the battle's map view: drawn
                // there, then moved to the middle of the screen.
                let (w, h) = (MAP_VIEW.x + MAP_VIEW.w, MAP_VIEW.y + MAP_VIEW.h);
                let size = |v: i32| u16::try_from(v).unwrap_or(0);
                let mut page = GlyphBuffer::new(size(w), size(h), blank);
                progress::draw(&mut page, &ctx.palette, &ctx.content.portraits, pages);
                buf.blit(&page, centre_x(buf, usize::from(page.width())), 0);
            }
            _ => self.draw_tally(ctx, buf),
        }
        let bottom = i32::from(buf.height()) - 1;
        let (dim, bg) = (c(UiColor::TextDim), c(UiColor::Black));
        print_centred(buf, bottom, &self.help(ctx), dim, bg);
    }
}

#[cfg(test)]
mod tests;
