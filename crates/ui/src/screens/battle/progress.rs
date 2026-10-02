//! EXP bar and level-up screen (ticket 0602): after a command's combat
//! playback (or at once, for a heal or a tile cast), each player unit that
//! gained something gets its pages in turn: the EXP bar filling from its old
//! EXP to its new ([`Event::ExpGained`], wrapping at 100 for each level),
//! then one level-up page per [`Event::LeveledUp`] (portrait, `LEVEL UP!`,
//! `Lv 4 → 5` and each stat revealed one row at a time), then its class
//! progress ([`Event::ClassLeveledUp`], [`Event::ClassMastered`] and a
//! `Learned: <name>` line per [`Event::SkillLearned`] and
//! [`Event::SpellLearned`]). Everything shown comes from the events; this
//! only times and draws them.
//!
//! A class change (ticket 0603; between battles, on the class-choice
//! screen) uses the same pages: a promotion ([`Event::Promoted`]) shows the
//! level-up panel with `PROMOTED!`, `Guard → Iron Rider` and its stat
//! bonus; a reclass ([`Event::Reclassed`]) shows the class box with
//! `Guard → Mage`. Spells learned and items sent to the stock
//! ([`Event::ItemStowed`]) are listed in the class box.
//!
//! Holding Confirm plays a page [`ProgressTimings::fast`] times as fast;
//! pressing it finishes the page's animation (reveals every stat), and on a
//! finished page that waits for the player, closes it. Cancel does the same
//! (Nick, PR #87: "cancel can do the same as confirm on these screens").

use std::collections::BTreeMap;

use trpg_content::Portrait;
use trpg_core::progression::EXP_PER_LEVEL;
use trpg_core::{
    BattleState, ClassId, ClassLevel, ClassTable, Event, Faction, ItemTable, Level, SkillTable,
    SpellTable, StatGains, StatKind, StatValue, Unit, UnitId,
};

use super::info::stat_name;
use super::layout::MAP_VIEW;
use super::playback::BOX;
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::portrait::draw_portrait;
use crate::widgets::help::{HelpKeys, help_line, key_name};

/// How long each page's parts take, in seconds. *Tunable.*
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressTimings {
    /// The EXP bar filling from the old EXP to the new.
    pub exp_fill: f32,
    /// The filled EXP bar stays up before the next page.
    pub exp_hold: f32,
    /// The level-up page before its first stat row.
    pub level_intro: f32,
    /// Between two stat rows appearing.
    pub stat_reveal: f32,
    /// A class-level notice (no mastery, nothing learned) stays up.
    pub class_hold: f32,
    /// Speed-up while Confirm is held.
    pub fast: f32,
}

/// The game's timings (ticket 0602: the bar fills over ~0.6 s, one stat
/// every ~0.12 s; ×4 while Confirm is held, like the combat playback).
pub const PROGRESS_TIMINGS: ProgressTimings = ProgressTimings {
    exp_fill: 0.6,
    exp_hold: 0.5,
    level_intro: 0.3,
    stat_reveal: 0.12,
    class_hold: 1.2,
    fast: 4.0,
};

/// The EXP bar of one unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpPage {
    /// The unit.
    pub unit: UnitId,
    /// Its name.
    pub name: String,
    /// Its EXP before the command (towards its next level).
    pub from: u32,
    /// EXP gained (all of the command's awards to it).
    pub gained: u32,
}

/// One level up of one unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelUpPage {
    /// The unit.
    pub unit: UnitId,
    /// Its name.
    pub name: String,
    /// Its character's portrait id, if it is a named character.
    pub character: Option<String>,
    /// The new level.
    pub level: Level,
    /// Each growable stat before this level up ([`StatKind::GROWABLE`]
    /// order).
    pub before: [StatValue; 7],
    /// What each stat gained.
    pub gains: StatGains,
}

/// The stat bonus of one promotion (ticket 0603), shown like a level up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionPage {
    /// The unit.
    pub unit: UnitId,
    /// Its name.
    pub name: String,
    /// Its character's portrait id, if it is a named character.
    pub character: Option<String>,
    /// The name of the class it left.
    pub from: String,
    /// The name of its new class.
    pub to: String,
    /// Each growable stat before the promotion ([`StatKind::GROWABLE`]
    /// order).
    pub before: [StatValue; 7],
    /// The promotion bonus of each stat.
    pub gains: StatGains,
}

/// A unit's class progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassPage {
    /// The unit.
    pub unit: UnitId,
    /// Its name.
    pub name: String,
    /// The class's name (`Guard → Mage` for a reclass).
    pub class: String,
    /// The class level before and after, if it rose.
    pub class_levels: Option<(ClassLevel, ClassLevel)>,
    /// Whether the class was mastered.
    pub mastered: bool,
    /// Names of the skills and spells learned, in event order.
    pub learned: Vec<String>,
    /// Whether the unit changed class with a Reclass Seal.
    pub changed: bool,
    /// Names of the items a class change sent to the stock, in event order.
    pub stowed: Vec<String>,
}

/// One page of the sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    /// The EXP bar.
    Exp(ExpPage),
    /// A level up.
    LevelUp(LevelUpPage),
    /// A promotion's stat bonus.
    Promotion(PromotionPage),
    /// Class progress.
    Class(ClassPage),
}

impl Page {
    /// How long the page's animation lasts at normal speed.
    pub fn len(&self, t: &ProgressTimings) -> f32 {
        match self {
            Page::Exp(_) => t.exp_fill + t.exp_hold,
            // Seven stats: small, exact.
            #[allow(clippy::cast_precision_loss)]
            Page::LevelUp(_) | Page::Promotion(_) => {
                t.level_intro + t.stat_reveal * StatKind::GROWABLE.len() as f32
            }
            Page::Class(c) if c.waits() => 0.0,
            Page::Class(_) => t.class_hold,
        }
    }

    /// Whether the page stays up, once played, until Confirm closes it
    /// (otherwise it closes by itself).
    pub fn waits(&self) -> bool {
        match self {
            Page::Exp(_) => false,
            Page::LevelUp(_) | Page::Promotion(_) => true,
            Page::Class(c) => c.waits(),
        }
    }
}

impl ClassPage {
    /// A mastery, something learned or a class change is worth stopping
    /// for.
    fn waits(&self) -> bool {
        self.mastered || !self.learned.is_empty() || self.changed || !self.stowed.is_empty()
    }
}

/// The content tables that name what a unit gained.
#[derive(Debug, Clone, Copy)]
pub struct ProgressTables<'a> {
    /// Classes.
    pub classes: &'a ClassTable,
    /// Skills.
    pub skills: &'a SkillTable,
    /// Spells.
    pub spells: &'a SpellTable,
    /// Items.
    pub items: &'a ItemTable,
}

impl<'a> ProgressTables<'a> {
    /// The tables of the battle `state`.
    pub fn of(state: &'a BattleState) -> Self {
        Self {
            classes: state.classes(),
            skills: state.skills(),
            spells: state.spells(),
            items: state.items(),
        }
    }

    /// The name of class `id` (its id if it is missing).
    fn class_name(&self, id: &ClassId) -> String {
        self.classes
            .get(id)
            .map_or_else(|| id.0.clone(), |c| c.name.clone())
    }
}

/// What a unit gained in one command, gathered from its events.
#[derive(Default)]
struct Gathered {
    exp: Option<u32>,
    levels: Vec<(Level, StatGains)>,
    class: Option<ClassId>,
    class_levels: Option<(ClassLevel, ClassLevel)>,
    mastered: bool,
    learned: Vec<String>,
    /// A promotion: the classes' names and the bonus.
    promoted: Option<(String, String, StatGains)>,
    /// A reclass: the classes' names.
    reclassed: Option<(String, String)>,
    stowed: Vec<String>,
}

impl Gathered {
    /// Adds `event` (one of this unit's); `tables` names what it gained.
    fn add(&mut self, event: &Event, tables: &ProgressTables<'_>) {
        match event {
            Event::ExpGained { amount, .. } => {
                self.exp = Some(self.exp.unwrap_or(0).saturating_add(*amount));
            }
            Event::LeveledUp { level, gains, .. } => self.levels.push((*level, *gains)),
            Event::ClassLeveledUp {
                class, class_level, ..
            } => {
                self.class = Some(class.clone());
                let from = self
                    .class_levels
                    .map_or(class_level.saturating_sub(1), |(from, _)| from);
                self.class_levels = Some((from, *class_level));
            }
            Event::ClassMastered { class, .. } => {
                self.class = Some(class.clone());
                self.mastered = true;
            }
            Event::SkillLearned { skill, .. } => {
                let name = tables.skills.get(skill).map(|s| s.name.clone());
                self.learned.extend(name);
            }
            Event::SpellLearned { spell, .. } => {
                let name = tables.spells.get(spell).map(|s| s.name.clone());
                self.learned.extend(name);
            }
            Event::Promoted {
                from, to, gains, ..
            } => {
                self.class = Some(to.clone());
                self.promoted = Some((tables.class_name(from), tables.class_name(to), *gains));
            }
            Event::Reclassed { from, to, .. } => {
                self.class = Some(to.clone());
                self.reclassed = Some((tables.class_name(from), tables.class_name(to)));
            }
            Event::ItemStowed { item, .. } => {
                let name = tables.items.get(item).map(|i| i.name().to_owned());
                self.stowed.extend(name);
            }
            _ => {}
        }
    }

    /// `unit`'s pages (as it was before the command): its EXP bar, a page
    /// per level up (each starting from the stats the last one left), a
    /// promotion's bonus, and its class progress.
    fn pages(self, unit: &Unit, tables: &ProgressTables<'_>) -> Vec<Page> {
        let mut pages = Vec::new();
        let (id, name) = (unit.id, unit.name.clone());
        if let Some(gained) = self.exp {
            pages.push(Page::Exp(ExpPage {
                unit: id,
                name: name.clone(),
                from: unit.exp,
                gained,
            }));
        }
        let mut running = StatKind::GROWABLE.map(|k| unit.stats.get(k));
        for (level, gains) in self.levels {
            pages.push(Page::LevelUp(LevelUpPage {
                unit: id,
                name: name.clone(),
                character: unit.character.as_ref().map(|c| c.0.clone()),
                level,
                before: running,
                gains,
            }));
            for (v, g) in running.iter_mut().zip(gains.0) {
                *v = v.saturating_add(g);
            }
        }
        if let Some((from, to, gains)) = self.promoted {
            pages.push(Page::Promotion(PromotionPage {
                unit: id,
                name: name.clone(),
                character: unit.character.as_ref().map(|c| c.0.clone()),
                from,
                to,
                before: running,
                gains,
            }));
        }
        let changed = self.reclassed.is_some();
        let progressed = self.class_levels.is_some() || self.mastered || changed;
        if progressed || !self.learned.is_empty() || !self.stowed.is_empty() {
            let class = match self.reclassed {
                Some((from, to)) => format!("{from} → {to}"),
                None => tables.class_name(&self.class.unwrap_or_else(|| unit.class.clone())),
            };
            pages.push(Page::Class(ClassPage {
                unit: id,
                name,
                class,
                class_levels: self.class_levels,
                mastered: self.mastered,
                learned: self.learned,
                changed,
                stowed: self.stowed,
            }));
        }
        pages
    }
}

/// The unit an EXP, level, class or learning event is about.
fn progress_unit(event: &Event) -> Option<UnitId> {
    match event {
        Event::ExpGained { unit, .. }
        | Event::LeveledUp { unit, .. }
        | Event::ClassLeveledUp { unit, .. }
        | Event::ClassMastered { unit, .. }
        | Event::SkillLearned { unit, .. }
        | Event::SpellLearned { unit, .. }
        | Event::Promoted { unit, .. }
        | Event::Reclassed { unit, .. }
        | Event::ItemStowed { unit, .. } => Some(*unit),
        _ => None,
    }
}

/// The sequence of pages after one command, and where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pages: Vec<Page>,
    page: usize,
    t: f32,
    timings: ProgressTimings,
}

impl Progress {
    /// The pages for `events` (one applied command), unit by unit in the
    /// order they first gained something, or `None` if no player unit
    /// gained anything. `before` is every unit before the command (for the
    /// old EXP and stats); `state` is the battle after it (for class, skill
    /// and spell names).
    pub fn new(
        events: &[Event],
        before: &[Unit],
        state: &BattleState,
        timings: ProgressTimings,
    ) -> Option<Self> {
        Self::with_tables(events, before, &ProgressTables::of(state), timings)
    }

    /// [`Progress::new`] with the content `tables` instead of a battle's:
    /// for events made outside a battle (a class change).
    pub fn with_tables(
        events: &[Event],
        before: &[Unit],
        tables: &ProgressTables<'_>,
        timings: ProgressTimings,
    ) -> Option<Self> {
        let mut gathered: Vec<(&Unit, Gathered)> = Vec::new();
        for event in events {
            let Some(id) = progress_unit(event) else {
                continue;
            };
            if !gathered.iter().any(|(u, _)| u.id == id) {
                let player = |u: &&Unit| u.id == id && u.faction == Faction::Player;
                let Some(unit) = before.iter().find(player) else {
                    continue;
                };
                gathered.push((unit, Gathered::default()));
            }
            if let Some((_, g)) = gathered.iter_mut().find(|(u, _)| u.id == id) {
                g.add(event, tables);
            }
        }
        let pages: Vec<Page> = gathered
            .into_iter()
            .flat_map(|(unit, g)| g.pages(unit, tables))
            .collect();
        if pages.is_empty() {
            return None;
        }
        Some(Self {
            pages,
            page: 0,
            t: 0.0,
            timings,
        })
    }

    /// Every page, in order.
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// The page on screen, if any is left.
    pub fn page(&self) -> Option<&Page> {
        self.pages.get(self.page)
    }

    /// Which page is on screen (`pages().len()` once done).
    pub fn index(&self) -> usize {
        self.page
    }

    /// Seconds into the page on screen.
    pub fn time(&self) -> f32 {
        self.t
    }

    /// Whether every page has closed.
    pub fn done(&self) -> bool {
        self.page >= self.pages.len()
    }

    /// Whether the page on screen has played to its end.
    pub fn page_played(&self) -> bool {
        self.page().is_none_or(|p| self.t >= p.len(&self.timings))
    }

    /// Moves to the next page.
    fn next(&mut self) {
        self.page += 1;
        self.t = 0.0;
    }

    /// Advances the clock by `dt` seconds ([`ProgressTimings::fast`] times
    /// faster while Confirm is held); a page that doesn't wait closes once
    /// played. A bad `dt` counts as 0.
    pub fn tick(&mut self, dt: f32, confirm_held: bool) {
        let Some(page) = self.page() else {
            return;
        };
        let len = page.len(&self.timings);
        let waits = page.waits();
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let speed = if confirm_held { self.timings.fast } else { 1.0 };
        self.t = (self.t + dt * speed).min(len);
        if !waits && self.t >= len {
            self.next();
        }
    }

    /// Confirm was pressed: finishes the page's animation, or closes a
    /// played page.
    pub fn confirm(&mut self) {
        let Some(page) = self.page() else {
            return;
        };
        let len = page.len(&self.timings);
        if self.t < len {
            self.t = len;
        } else {
            self.next();
        }
    }

    /// The EXP page's shown EXP, counted from its old EXP (so `from +
    /// gained` when full; the bar shows it modulo [`EXP_PER_LEVEL`]).
    pub fn exp_shown(&self) -> Option<u32> {
        let Some(Page::Exp(p)) = self.page() else {
            return None;
        };
        let frac = (self.t / self.timings.exp_fill.max(f32::EPSILON)).clamp(0.0, 1.0);
        // EXP awards are small (at most a few levels): exact in f32.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let moved = ((p.gained as f32 * frac) + 1e-3).floor() as u32;
        Some(p.from.saturating_add(moved.min(p.gained)))
    }

    /// How many of the level-up (or promotion) page's stat rows are shown.
    pub fn stats_shown(&self) -> usize {
        let Some(Page::LevelUp(_) | Page::Promotion(_)) = self.page() else {
            return 0;
        };
        let t = self.t - self.timings.level_intro;
        if t < 0.0 {
            return 0;
        }
        // Whole reveal periods since the intro, plus the first row.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (t / self.timings.stat_reveal.max(f32::EPSILON) + 1e-3) as usize;
        (n + 1).min(StatKind::GROWABLE.len())
    }
}

/// The help line of an EXP bar, level-up or class page: skip and fast while
/// it plays, then continue.
pub fn help(p: &Progress, km: HelpKeys<'_>) -> String {
    let confirm = key_name(km, Action::Confirm);
    if p.page_played() {
        help_line(&[(Some(confirm), "continue")])
    } else {
        let hold = Some(format!("hold {confirm}"));
        help_line(&[(Some(confirm), "skip"), (hold, "fast")])
    }
}

/// The EXP box: where the combat playback's box was.
pub const EXP_BOX: Rect = Rect::new(BOX.x, BOX.y, BOX.w, 3);

/// Length of the EXP bar, in cells (5 EXP each).
pub const EXP_BAR_CELLS: i32 = 20;

/// Widest unit name in the EXP and class boxes.
const NAME_W: usize = 12;

/// Where the EXP bar or the class line starts, after the name.
const NAME_COL: i32 = 13;

/// Widest class line (the box's inside, less the name column and padding).
const CLASS_W: usize = 27;

/// The level-up panel, over the map view.
pub const LEVEL_PANEL: Rect = Rect::new(MAP_VIEW.x + 5, MAP_VIEW.y + 5, 60, 20);

/// The portrait frame in the level-up panel: a 32×16-cell portrait and its
/// border.
pub const PORTRAIT_FRAME: Rect = Rect::new(LEVEL_PANEL.x + 2, LEVEL_PANEL.y + 1, 34, 18);

/// The text column of the level-up panel.
pub const LEVEL_TEXT_X: i32 = PORTRAIT_FRAME.x + PORTRAIT_FRAME.w + 2;

/// Row of the `LEVEL UP!` banner.
pub const LEVEL_BANNER_ROW: i32 = LEVEL_PANEL.y + 2;

/// Row of the first stat.
pub const STAT_ROW: i32 = LEVEL_PANEL.y + 7;

/// The expression shown on a level up, else the portrait's first.
const LEVEL_UP_EXPRESSIONS: [&str; 2] = ["happy", "neutral"];

/// The EXP bar at `exp` (counted from any level: shown modulo
/// [`EXP_PER_LEVEL`]): the cells filled and the number shown.
pub fn exp_line(exp: u32) -> (i32, u32) {
    let shown = exp % EXP_PER_LEVEL;
    // shown < 100: exact.
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    let filled = (shown * EXP_BAR_CELLS as u32 / EXP_PER_LEVEL) as i32;
    (filled, shown)
}

/// The text of stat `kind`'s row: `Str   6 → 7  +1` if it grew, else
/// `Def   5`.
pub fn stat_row(kind: StatKind, before: StatValue, gain: StatValue) -> String {
    let name = stat_name(kind);
    if gain > 0 {
        format!("{name:<4}{before:>3} → {:<3} +{gain}", before + gain)
    } else {
        format!("{name:<4}{before:>3}")
    }
}

/// The class line: `Exile  CL 3 → 4`, or just the class's name.
pub fn class_line(page: &ClassPage) -> String {
    match page.class_levels {
        Some((from, to)) => format!("{}  CL {from} → {to}", page.class),
        None => page.class.clone(),
    }
}

/// Draws the page on screen.
pub fn draw(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    portraits: &BTreeMap<String, Portrait>,
    progress: &Progress,
) {
    match progress.page() {
        Some(Page::Exp(p)) => draw_exp(buf, palette, p, progress.exp_shown().unwrap_or(p.from)),
        Some(Page::LevelUp(p)) => {
            let portrait = p.character.as_ref().and_then(|c| portraits.get(c));
            draw_level_up(buf, palette, p, portrait, progress.stats_shown());
        }
        Some(Page::Promotion(p)) => {
            let portrait = p.character.as_ref().and_then(|c| portraits.get(c));
            draw_promotion(buf, palette, p, portrait, progress.stats_shown());
        }
        Some(Page::Class(p)) => draw_class(buf, palette, p),
        None => {}
    }
}

/// The EXP box: `Test Lord   EXP ████████░░░░ 80`.
fn draw_exp(buf: &mut GlyphBuffer, palette: &Palette, page: &ExpPage, exp: u32) {
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    buf.fill_rect(EXP_BOX, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(EXP_BOX, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
    let y = EXP_BOX.y + 1;
    let x = EXP_BOX.x + 2;
    let name: String = page.name.chars().take(NAME_W).collect();
    buf.print(x, y, &name, c(UiColor::Player), bg);
    let bar_x = x + NAME_COL;
    buf.print(bar_x, y, "EXP", c(UiColor::TextDim), bg);
    let (filled, shown) = exp_line(exp);
    for i in 0..EXP_BAR_CELLS {
        let (glyph, fg) = if i < filled {
            ('█', c(UiColor::ExpBar))
        } else {
            ('░', c(UiColor::TextDim))
        };
        buf.set(bar_x + 4 + i, y, Cell::new(glyph, fg, bg));
    }
    let n = format!("{shown:>2}");
    buf.print(bar_x + 5 + EXP_BAR_CELLS, y, &n, c(UiColor::Text), bg);
}

/// The level-up panel: portrait (or a placeholder box), `LEVEL UP!`, the
/// name, `Lv 4 → 5`, and the first `shown` stat rows.
fn draw_level_up(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    page: &LevelUpPage,
    portrait: Option<&Portrait>,
    shown: usize,
) {
    let text = GainText {
        banner: "LEVEL UP!",
        name: &page.name,
        line: format!("Lv {} → {}", page.level.saturating_sub(1), page.level),
    };
    draw_gains(
        buf,
        palette,
        &text,
        (&page.before, &page.gains),
        portrait,
        shown,
    );
}

/// The banner of a promotion's page.
pub const PROMOTED_BANNER: &str = "PROMOTED!";

/// The level-up panel for a promotion: `PROMOTED!`, the name,
/// `Guard → Iron Rider`, and the first `shown` stat rows with the bonus.
fn draw_promotion(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    page: &PromotionPage,
    portrait: Option<&Portrait>,
    shown: usize,
) {
    let text = GainText {
        banner: PROMOTED_BANNER,
        name: &page.name,
        line: format!("{} → {}", page.from, page.to),
    };
    draw_gains(
        buf,
        palette,
        &text,
        (&page.before, &page.gains),
        portrait,
        shown,
    );
}

/// The words of the level-up panel.
struct GainText<'a> {
    /// The banner, e.g. `LEVEL UP!`.
    banner: &'a str,
    /// The unit's name.
    name: &'a str,
    /// What changed, e.g. `Lv 4 → 5`.
    line: String,
}

/// Widest text in the level-up panel's text column.
const LEVEL_TEXT_W: usize = 20;

/// The level-up panel: portrait (or a placeholder box), `text`, and the
/// first `shown` rows of the stats `before` with their `gains`.
fn draw_gains(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    text: &GainText<'_>,
    (before, gains): (&[StatValue; 7], &StatGains),
    portrait: Option<&Portrait>,
    shown: usize,
) {
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    buf.fill_rect(LEVEL_PANEL, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(
        LEVEL_PANEL,
        BoxStyle::Double,
        c(UiColor::PanelBorderFocus),
        bg,
    );
    buf.draw_box(
        PORTRAIT_FRAME,
        BoxStyle::Single,
        c(UiColor::PanelBorder),
        bg,
    );
    let drawn = portrait.is_some_and(|art| {
        let expr = LEVEL_UP_EXPRESSIONS
            .iter()
            .copied()
            .find(|e| art.expression(e).is_some())
            .or_else(|| art.expressions.first().map(|e| e.name.as_str()));
        expr.is_some_and(|expr| {
            let at = (PORTRAIT_FRAME.x + 1, PORTRAIT_FRAME.y + 1);
            draw_portrait(buf, palette, at, art, expr, 0.0, false)
        })
    });
    if !drawn {
        let label = "portrait";
        let w = i32::try_from(label.len()).unwrap_or(0);
        let x = PORTRAIT_FRAME.x + (PORTRAIT_FRAME.w - w) / 2;
        let y = PORTRAIT_FRAME.y + PORTRAIT_FRAME.h / 2;
        buf.print(x, y, label, c(UiColor::TextDim), bg);
    }
    let x = LEVEL_TEXT_X;
    buf.print(
        x,
        LEVEL_BANNER_ROW,
        text.banner,
        c(UiColor::TextHighlight),
        bg,
    );
    let cut = |s: &str| s.chars().take(LEVEL_TEXT_W).collect::<String>();
    buf.print(
        x,
        LEVEL_BANNER_ROW + 2,
        &cut(text.name),
        c(UiColor::Player),
        bg,
    );
    buf.print(
        x,
        LEVEL_BANNER_ROW + 3,
        &cut(&text.line),
        c(UiColor::Text),
        bg,
    );
    for (i, kind) in StatKind::GROWABLE.iter().enumerate().take(shown) {
        let gain = gains.0[i];
        let row = stat_row(*kind, before[i], gain);
        let y = STAT_ROW + i32::try_from(i).unwrap_or(0);
        if gain > 0 {
            let n = buf.print(x, y, &row, c(UiColor::Text), bg);
            // The `+1` in the highlight colour.
            let plus = format!("+{gain}");
            let at = i32::from(n) - i32::try_from(plus.chars().count()).unwrap_or(0);
            buf.print(x + at, y, &plus, c(UiColor::TextHighlight), bg);
        } else {
            buf.print(x, y, &row, c(UiColor::TextDim), bg);
        }
    }
}

/// The class box: the name and `Exile  CL 3 → 4`, then `CLASS MASTERED!`,
/// a `Learned: <name>` line each, and a `To stock: <item>` line each.
fn draw_class(buf: &mut GlyphBuffer, palette: &Palette, page: &ClassPage) {
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    let mastery = i32::from(page.mastered);
    let lines = page.learned.len() + page.stowed.len();
    let h = 3 + mastery + i32::try_from(lines).unwrap_or(0);
    let rect = Rect::new(BOX.x, BOX.y, BOX.w, h);
    buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
    let x = rect.x + 2;
    let mut y = rect.y + 1;
    let name: String = page.name.chars().take(NAME_W).collect();
    buf.print(x, y, &name, c(UiColor::Player), bg);
    let line: String = class_line(page).chars().take(CLASS_W).collect();
    buf.print(x + NAME_COL, y, &line, c(UiColor::Text), bg);
    if page.mastered {
        y += 1;
        buf.print(x, y, "CLASS MASTERED!", c(UiColor::TextHighlight), bg);
    }
    let learned = page.learned.iter().map(|s| format!("Learned: {s}"));
    let stowed = page.stowed.iter().map(|s| format!("To stock: {s}"));
    for line in learned.chain(stowed) {
        y += 1;
        let line: String = line.chars().take(38).collect();
        buf.print(x, y, &line, c(UiColor::Text), bg);
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{SkillId, SpellId};

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::quick_battle;

    const T: ProgressTimings = PROGRESS_TIMINGS;

    fn state() -> BattleState {
        quick_battle(&ctx().content).unwrap()
    }

    fn gains(g: [StatValue; 7]) -> StatGains {
        StatGains(g)
    }

    /// The lord (unit 1, 19 HP, Str 6 …) with 90 EXP gains 30: one level
    /// up; then Exile CL 9 → 10, mastery and a passive.
    fn events() -> Vec<Event> {
        let lord = UnitId(1);
        vec![
            Event::ExpGained {
                unit: lord,
                amount: 30,
            },
            Event::LeveledUp {
                unit: lord,
                level: 2,
                gains: gains([1, 1, 0, 0, 2, 0, 0]),
            },
            Event::ClassPointsGained {
                unit: lord,
                class: ClassId("exile".into()),
                amount: 3,
            },
            Event::ClassLeveledUp {
                unit: lord,
                class: ClassId("exile".into()),
                class_level: 10,
            },
            Event::ClassMastered {
                unit: lord,
                class: ClassId("exile".into()),
            },
            Event::SkillLearned {
                unit: lord,
                skill: SkillId::new("leadership_1"),
            },
        ]
    }

    fn progress() -> Progress {
        let s = state();
        let mut before = s.units().to_vec();
        before[0].exp = 90;
        Progress::new(&events(), &before, &s, T).unwrap()
    }

    #[test]
    fn pages_come_exp_then_level_up_then_class() {
        let pb = progress();
        let pages = pb.pages();
        assert_eq!(pages.len(), 3);
        assert_eq!(
            pages[0],
            Page::Exp(ExpPage {
                unit: UnitId(1),
                name: "Test Lord".into(),
                from: 90,
                gained: 30,
            })
        );
        let Page::LevelUp(lv) = &pages[1] else {
            panic!("{pages:?}")
        };
        assert_eq!((lv.level, lv.before[0], lv.before[1]), (2, 19, 6));
        assert_eq!(lv.character.as_deref(), Some("test_lord"));
        let Page::Class(cl) = &pages[2] else {
            panic!("{pages:?}")
        };
        assert_eq!(class_line(cl), "Exile  CL 9 → 10");
        assert!(cl.mastered);
        // Mastery alone is worth stopping for.
        let mastery = Page::Class(ClassPage {
            learned: vec![],
            ..cl.clone()
        });
        assert!(mastery.waits());
        assert!(mastery.len(&T).abs() < f32::EPSILON);
        assert_eq!(cl.learned.len(), 1);
    }

    #[test]
    fn nothing_gained_no_pages_and_other_factions_are_ignored() {
        let s = state();
        let before = s.units().to_vec();
        assert_eq!(Progress::new(&[], &before, &s, T), None);
        let brigand = [Event::ExpGained {
            unit: UnitId(4),
            amount: 10,
        }];
        assert_eq!(Progress::new(&brigand, &before, &s, T), None);
    }

    #[test]
    fn two_level_ups_stack_their_stats_and_awards_add_up() {
        let s = state();
        let before = s.units().to_vec();
        let lord = UnitId(1);
        let events = [
            Event::ExpGained {
                unit: lord,
                amount: 150,
            },
            Event::LeveledUp {
                unit: lord,
                level: 2,
                gains: gains([1, 1, 0, 0, 0, 0, 0]),
            },
            Event::ExpGained {
                unit: lord,
                amount: 60,
            },
            Event::LeveledUp {
                unit: lord,
                level: 3,
                gains: gains([2, 0, 0, 0, 0, 0, 1]),
            },
            Event::SpellLearned {
                unit: lord,
                spell: SpellId::new("fire"),
            },
        ];
        let pb = Progress::new(&events, &before, &s, T).unwrap();
        let Page::Exp(exp) = &pb.pages()[0] else {
            panic!()
        };
        assert_eq!(exp.gained, 210);
        let Page::LevelUp(second) = &pb.pages()[2] else {
            panic!()
        };
        assert_eq!((second.before[0], second.before[1]), (20, 7));
        let Page::Class(cl) = &pb.pages()[3] else {
            panic!()
        };
        // A spell learned on its own: the class's name, no level.
        assert_eq!((class_line(cl), cl.mastered), ("Exile".to_owned(), false));
        assert_eq!(cl.learned, ["Fire"]);
        // So is something learned.
        assert!(pb.pages()[3].waits());
    }

    #[test]
    fn the_exp_bar_fills_over_its_time_and_wraps_at_100() {
        let mut pb = progress();
        assert_eq!(pb.exp_shown(), Some(90));
        pb.tick(T.exp_fill / 2.0, false);
        assert_eq!(pb.exp_shown(), Some(105));
        assert_eq!(exp_line(105), (1, 5));
        pb.tick(T.exp_fill / 2.0, false);
        assert_eq!(pb.exp_shown(), Some(120));
        assert_eq!(exp_line(120), (4, 20));
        assert_eq!(exp_line(80), (16, 80));
        // The hold, then the level-up page by itself.
        pb.tick(T.exp_hold - 0.01, false);
        assert_eq!(pb.index(), 0);
        pb.tick(0.02, false);
        assert_eq!(pb.index(), 1);
        assert!(pb.time().abs() < f32::EPSILON);
    }

    #[test]
    fn holding_confirm_speeds_up_and_pressing_finishes_the_bar() {
        let mut pb = progress();
        pb.tick(0.1, true);
        assert!((pb.time() - 0.4).abs() < 1e-5);
        pb.confirm();
        assert_eq!(pb.exp_shown(), Some(120));
        assert!(pb.page_played());
        // A played page that doesn't wait closes on the next frame.
        pb.tick(0.0, false);
        assert_eq!(pb.index(), 1);
    }

    #[test]
    fn stats_are_revealed_one_at_a_time_then_confirm_closes() {
        let mut pb = progress();
        pb.confirm();
        pb.tick(0.0, false);
        assert_eq!(pb.stats_shown(), 0);
        pb.tick(T.level_intro, false);
        assert_eq!(pb.stats_shown(), 1);
        pb.tick(T.stat_reveal, false);
        assert_eq!(pb.stats_shown(), 2);
        pb.tick(T.stat_reveal * 2.5, false);
        assert_eq!(pb.stats_shown(), 4);
        pb.tick(10.0, false);
        assert_eq!(pb.stats_shown(), 7);
        // It waits for Confirm.
        assert_eq!(pb.index(), 1);
        pb.confirm();
        assert_eq!(pb.index(), 2);
    }

    #[test]
    fn a_press_reveals_every_stat_before_closing() {
        let mut pb = progress();
        pb.confirm();
        pb.tick(0.0, false);
        pb.tick(T.level_intro, false);
        pb.confirm();
        assert_eq!((pb.index(), pb.stats_shown()), (1, 7));
        pb.confirm();
        // The mastery page waits too; Confirm closes the last page.
        assert!(matches!(pb.page(), Some(Page::Class(_))));
        pb.tick(10.0, false);
        assert!(!pb.done());
        pb.confirm();
        assert!(pb.done());
        assert!(pb.page_played());
    }

    #[test]
    fn a_class_level_alone_closes_by_itself() {
        let s = state();
        let before = s.units().to_vec();
        let events = [Event::ClassLeveledUp {
            unit: UnitId(1),
            class: ClassId("exile".into()),
            class_level: 4,
        }];
        let mut pb = Progress::new(&events, &before, &s, T).unwrap();
        assert!(!pb.pages()[0].waits());
        pb.tick(T.class_hold / 2.0, false);
        assert!(!pb.done(), "it stays up a moment");
        pb.tick(T.class_hold / 2.0 + 0.01, false);
        assert!(pb.done());
        // Bad frame times count as nothing.
        let mut pb = Progress::new(&events, &before, &s, T).unwrap();
        pb.tick(f32::NAN, false);
        pb.tick(-1.0, false);
        assert!(pb.time().abs() < f32::EPSILON);
    }

    #[test]
    fn a_unit_without_a_portrait_gets_the_placeholder_box() {
        let c = ctx();
        let mut pb = progress();
        pb.confirm();
        pb.tick(0.0, false);
        let Some(Page::LevelUp(page)) = pb.page() else {
            panic!("{:?}", pb.page())
        };
        let page = LevelUpPage {
            character: None,
            ..page.clone()
        };
        let p = &c.palette;
        let blank = Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black));
        let mut buf = GlyphBuffer::new(100, 32, blank);
        draw_level_up(&mut buf, p, &page, None, 7);
        // Centred in the frame at (7, 6), 34 × 18.
        let label: String = (20..28).map(|x| buf.get(x, 15).unwrap().glyph).collect();
        assert_eq!(label, "portrait");
        assert_eq!(buf.get(19, 15).unwrap().glyph, ' ');
    }

    /// The lord promotes (ticket 0603): its bonus, a spell and a weapon
    /// sent to the stock.
    fn promotion_events() -> Vec<Event> {
        let lord = UnitId(1);
        vec![
            Event::Promoted {
                unit: lord,
                from: ClassId("exile".into()),
                to: ClassId("blade_heir".into()),
                gains: gains([6, 3, 0, 4, 5, 2, 2]),
            },
            Event::SpellLearned {
                unit: lord,
                spell: SpellId::new("fire"),
            },
            Event::ItemStowed {
                unit: lord,
                item: trpg_core::ItemId::new("iron_sword"),
            },
        ]
    }

    #[test]
    fn a_promotion_shows_its_bonus_like_a_level_up_then_what_came_with_it() {
        let s = state();
        let before = s.units().to_vec();
        let tables = ProgressTables::of(&s);
        let mut pb = Progress::with_tables(&promotion_events(), &before, &tables, T).unwrap();
        assert_eq!(
            pb.pages()[0],
            Page::Promotion(PromotionPage {
                unit: UnitId(1),
                name: "Test Lord".into(),
                character: Some("test_lord".into()),
                from: "Exile".into(),
                to: "Blade Heir".into(),
                before: [19, 6, 1, 7, 7, 4, 3],
                gains: gains([6, 3, 0, 4, 5, 2, 2]),
            })
        );
        let Page::Class(cl) = &pb.pages()[1] else {
            panic!("{:?}", pb.pages())
        };
        assert_eq!(class_line(cl), "Blade Heir");
        assert_eq!((cl.mastered, cl.changed), (false, false));
        assert_eq!(
            (&cl.learned, &cl.stowed),
            (&vec!["Fire".into()], &vec!["Iron Sword".to_owned()])
        );
        assert_eq!(pb.pages().len(), 2);
        // The stats are revealed one at a time, and the page waits.
        assert!(pb.pages()[0].waits());
        assert_eq!(pb.stats_shown(), 0);
        pb.tick(T.level_intro + T.stat_reveal, false);
        assert_eq!(pb.stats_shown(), 2);
        pb.tick(60.0, false);
        assert_eq!((pb.stats_shown(), pb.index()), (7, 0));
        assert!(pb.page_played());
        pb.confirm();
        assert_eq!((pb.stats_shown(), pb.index()), (0, 1));
        // Drawn: the banner and the classes where a level up has its own.
        let c = ctx();
        let p = &c.palette;
        let blank = Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black));
        let mut buf = GlyphBuffer::new(100, 32, blank);
        let Page::Promotion(page) = &pb.pages()[0] else {
            panic!()
        };
        draw_promotion(&mut buf, p, page, None, 7);
        let row = |buf: &GlyphBuffer, y: i32| {
            (LEVEL_TEXT_X..LEVEL_PANEL.x + LEVEL_PANEL.w - 1)
                .map(|x| buf.get(x, y).unwrap().glyph)
                .collect::<String>()
                .trim_end()
                .to_owned()
        };
        assert_eq!(row(&buf, LEVEL_BANNER_ROW), PROMOTED_BANNER);
        assert_eq!(row(&buf, LEVEL_BANNER_ROW + 2), "Test Lord");
        assert_eq!(row(&buf, LEVEL_BANNER_ROW + 3), "Exile → Blade Heir");
        assert_eq!(row(&buf, STAT_ROW), "HP   19 → 25  +6");
        assert_eq!(row(&buf, STAT_ROW + 2), "Mag   1");
        // A level up's words, in the same places.
        let level_up = LevelUpPage {
            unit: page.unit,
            name: "A name longer than twenty cells".into(),
            character: None,
            level: 5,
            before: page.before,
            gains: page.gains,
        };
        draw_level_up(&mut buf, p, &level_up, None, 1);
        assert_eq!(row(&buf, LEVEL_BANNER_ROW), "LEVEL UP!");
        assert_eq!(row(&buf, LEVEL_BANNER_ROW + 2), "A name longer than t");
        assert_eq!(row(&buf, LEVEL_BANNER_ROW + 3), "Lv 4 → 5");
        assert_eq!(row(&buf, STAT_ROW + 1), "");
    }

    #[test]
    fn a_reclass_or_a_stowed_item_is_worth_stopping_for() {
        let s = state();
        let before = s.units().to_vec();
        let lord = UnitId(1);
        // Class ids missing from the table are shown as they are.
        let reclassed = [Event::Reclassed {
            unit: lord,
            from: ClassId("exile".into()),
            to: ClassId("gone".into()),
        }];
        let pb = Progress::new(&reclassed, &before, &s, T).unwrap();
        let [Page::Class(cl)] = pb.pages() else {
            panic!("{:?}", pb.pages())
        };
        assert_eq!(class_line(cl), "Exile → gone");
        assert!(cl.changed && cl.learned.is_empty() && cl.stowed.is_empty());
        assert!(pb.pages()[0].waits());
        assert!(pb.pages()[0].len(&T).abs() < f32::EPSILON);
        let stowed = [Event::ItemStowed {
            unit: lord,
            item: trpg_core::ItemId::new("leather_vest"),
        }];
        let pb = Progress::new(&stowed, &before, &s, T).unwrap();
        let [Page::Class(cl)] = pb.pages() else {
            panic!("{:?}", pb.pages())
        };
        assert_eq!((class_line(cl), cl.changed), ("Exile".to_owned(), false));
        assert_eq!(cl.stowed, ["Leather Vest"]);
        assert!(pb.pages()[0].waits());
        // Its lines are drawn under the name's.
        let c = ctx();
        let p = &c.palette;
        let blank = Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black));
        let mut buf = GlyphBuffer::new(100, 32, blank);
        let page = ClassPage {
            learned: vec!["Fire".into()],
            ..cl.clone()
        };
        draw_class(&mut buf, p, &page);
        let row = |y: i32| {
            (BOX.x + 2..BOX.x + BOX.w - 1)
                .map(|x| buf.get(x, y).unwrap().glyph)
                .collect::<String>()
                .trim_end()
                .to_owned()
        };
        assert_eq!(row(BOX.y + 2), "Learned: Fire");
        assert_eq!(row(BOX.y + 3), "To stock: Leather Vest");
        // The box is as tall as its lines: its bottom border is next.
        assert_eq!(buf.get(BOX.x, BOX.y + 4).unwrap().glyph, '╚');
        // An item missing from the table has no line.
        let ghost = [Event::ItemStowed {
            unit: lord,
            item: trpg_core::ItemId::new("ghost"),
        }];
        assert_eq!(Progress::new(&ghost, &before, &s, T), None);
    }

    #[test]
    fn the_help_line_follows_the_page() {
        let c = ctx();
        let mut pb = progress();
        assert_eq!(help(&pb, c.help_keys()), "f skip · hold f fast");
        pb.confirm();
        assert_eq!(help(&pb, c.help_keys()), "f continue");
    }

    #[test]
    fn stat_rows_show_the_gain_or_just_the_value() {
        assert_eq!(stat_row(StatKind::Str, 6, 1), "Str   6 → 7   +1");
        assert_eq!(stat_row(StatKind::Hp, 19, 2), "HP   19 → 21  +2");
        assert_eq!(stat_row(StatKind::Def, 5, 0), "Def   5");
    }
}
