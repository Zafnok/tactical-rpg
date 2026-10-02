//! The class-choice screen (ticket 0603; `docs/design/progression.md`,
//! *Promotion* and *Reclass*): a unit promotes or changes class **between
//! battles**, spending a seal from the party's stock (Nick: no seal works
//! in a battle).
//!
//! One column per class the unit can go to, side by side: the class's
//! name, tier (and, for a reclass, the class level saved there or `new`),
//! Mov and movement type, tags, weapon kinds with the unit's rank before
//! and after, the active skill it gives, and every stat `current → new`
//! with the gain highlighted. A stat at its hard ceiling is marked `MAX`
//! (shown out of battle only: `progression.md`, *Showing a maxed stat*).
//! Every number is what [`promote`] or [`reclass`] does to a copy of the
//! unit. A class the change is refused for (no seal in stock, class not
//! mastered) is dimmed, with the reason, and can't be chosen.
//!
//! Left and right pick the column (the columns scroll when there are more
//! than [`COLUMNS`]), Confirm chooses it and asks `Promote … ?`, Confirm
//! again makes the change, and its result plays as the level-up overlay
//! ([`Progress`]): the stat bonus of a promotion, spells learned, items
//! sent to the stock (on an empty screen: the pages sit where they do over
//! the battle map). Then the screen closes. Cancel backs out of the
//! question, or closes the screen with nothing changed.
//!
//! Until the between-battle menus exist (Preparations, 0408; camp), the
//! screen is reached from the debug menu with a test unit
//! ([`ClassChangeScreen::demo`]).

use trpg_core::progression::class_change::ChangeTables;
use trpg_core::{
    CharacterId, ClassChangeError, ClassDef, ClassId, ClassRecord, Event, Faction, ItemDef, Pos,
    SealKind, StatKind, StatValue, Stock, Unit, UnitId, promote, promotion_targets, reclass,
    reclass_targets,
};

use super::battle::info::stat_name;
use super::battle::progress::{self, PROGRESS_TIMINGS, Progress, ProgressTables};
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{SEPARATOR, help_line, key_name};

/// Columns shown at once.
pub const COLUMNS: usize = 4;

/// Width of a column's box, in cells.
const COLUMN_W: i32 = 24;

/// Column of the first column's box.
const COLUMNS_X: i32 = 2;

/// Row of the columns' top border.
const COLUMNS_Y: i32 = 4;

/// Height of a column's box.
const COLUMN_H: i32 = 25;

/// Widest text in a column (one blank cell inside each border).
const TEXT_W: usize = 20;

/// Row of the title.
const TITLE_ROW: i32 = 1;

/// Row of the unit's line.
const UNIT_ROW: i32 = 2;

/// Row of the `← more` / `more →` marks.
const MORE_ROW: i32 = 3;

/// The mark after a stat at its hard ceiling.
pub const MAX_MARK: &str = "MAX";

/// What kind of class change the screen makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// A promotion, with the seal of the new class's tier.
    Promote,
    /// A full class change, with a Reclass Seal.
    Reclass,
}

impl ChangeKind {
    /// The screen's title.
    const fn title(self) -> &'static str {
        match self {
            ChangeKind::Promote => "Promotion",
            ChangeKind::Reclass => "Class change",
        }
    }

    /// The seal a change into `class` uses up.
    const fn seal(self, class: &ClassDef) -> SealKind {
        match self {
            ChangeKind::Promote => SealKind::Tier(class.tier),
            ChangeKind::Reclass => SealKind::Reclass,
        }
    }
}

/// One class the unit could change to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassOption {
    /// The class.
    pub class: ClassId,
    /// The unit after the change, or why the change is refused.
    pub after: Result<Unit, ClassChangeError>,
}

/// Where the screen is.
#[derive(Debug, Clone, PartialEq)]
enum Stage {
    /// Picking a column.
    Choosing,
    /// `Promote … ?`
    Confirming,
    /// The change was made: its pages play.
    Result(Progress),
    /// Nothing left to show: the screen closes.
    Done,
}

/// The class-choice screen. See the module docs.
#[derive(Debug, Clone)]
pub struct ClassChangeScreen {
    kind: ChangeKind,
    unit: Unit,
    stock: Stock,
    options: Vec<ClassOption>,
    focus: usize,
    /// The leftmost column shown.
    first: usize,
    stage: Stage,
}

/// The tables a class change reads, from the game's content.
fn tables(ctx: &Ctx) -> ChangeTables<'_> {
    ChangeTables {
        classes: &ctx.content.classes,
        items: &ctx.content.items,
        spells: &ctx.content.spells,
    }
}

/// Makes the `kind` of change of `unit` into `target`.
fn change(
    kind: ChangeKind,
    unit: &mut Unit,
    target: &ClassId,
    stock: &mut Stock,
    tables: &ChangeTables<'_>,
) -> Result<Vec<Event>, ClassChangeError> {
    match kind {
        ChangeKind::Promote => promote(unit, target, stock, tables),
        ChangeKind::Reclass => reclass(unit, target, stock, tables),
    }
}

/// The text of stat `kind`'s row: `Str   9 → 12  +3` if it grows, else
/// `Mag   1`; with [`MAX_MARK`] after a stat that ends at its `ceiling`.
pub fn stat_line(
    kind: StatKind,
    before: StatValue,
    after: StatValue,
    ceiling: StatValue,
) -> String {
    let row = progress::stat_row(kind, before, after - before);
    if after >= ceiling {
        format!("{row} {MAX_MARK}")
    } else {
        row
    }
}

/// The Mov row: `Mov  4 → 6` if it changes, else `Mov  5`.
pub fn mov_line(before: StatValue, after: StatValue) -> String {
    let name = stat_name(StatKind::Mov);
    if before == after {
        format!("{name:<4}{before:>3}")
    } else {
        format!("{name:<4}{before:>3} → {after}")
    }
}

/// The weapon rows of `class` for `unit`: each kind the class wields with
/// the unit's rank now and on entering the class, `Spear    D → C`, or
/// just the rank if it stays.
pub fn weapon_lines(unit: &Unit, class: &ClassDef) -> Vec<String> {
    class
        .weapons
        .iter()
        .map(|w| {
            let now = unit.rank(w.kind);
            let then = now.max(w.start);
            let kind = format!("{:?}", w.kind);
            if now == then {
                format!("{kind:<9}{now:?}")
            } else {
                format!("{kind:<9}{now:?} → {then:?}")
            }
        })
        .collect()
}

impl ClassChangeScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "class_change";

    /// The screen for `unit` and the party's `stock`: the classes a
    /// promotion ([`promotion_targets`]) or a Reclass Seal
    /// ([`reclass_targets`]) offers it, each with the unit as the change
    /// would leave it.
    pub fn new(ctx: &Ctx, kind: ChangeKind, unit: Unit, stock: Stock) -> Self {
        let tables = tables(ctx);
        let targets = match kind {
            ChangeKind::Promote => promotion_targets(&unit, tables.classes),
            ChangeKind::Reclass => reclass_targets(&unit, tables.classes),
        };
        let options = targets
            .into_iter()
            .map(|class| {
                let (mut after, mut stock) = (unit.clone(), stock.clone());
                let result = change(kind, &mut after, &class.id, &mut stock, &tables);
                ClassOption {
                    class: class.id.clone(),
                    after: result.map(|_| after),
                }
            })
            .collect();
        Self {
            kind,
            unit,
            stock,
            options,
            focus: 0,
            first: 0,
            stage: Stage::Choosing,
        }
    }

    /// The screen on a test unit, for the debug menu: the placeholder
    /// knight (a Guard) at level 12, its Guard mastered and a Rider record
    /// saved at class level 6, Def one short of its hard ceiling (so a
    /// `MAX` shows), and a stock with one of every seal. `None` if the
    /// content has no such character.
    pub fn demo(ctx: &Ctx, kind: ChangeKind) -> Option<Self> {
        let content = &ctx.content;
        let classes = &content.classes;
        let def = content
            .characters
            .characters
            .get(&CharacterId("test_knight".into()))?;
        let mut unit = trpg_content::character_unit(
            def,
            UnitId(1),
            classes,
            &content.items,
            Faction::Player,
            Pos::new(0, 0),
        )
        .ok()?;
        unit.level = 12;
        unit.exp = 40;
        unit.stats.hp += 8;
        unit.stats.str += 4;
        unit.stats.dex += 3;
        unit.stats.spd += 3;
        unit.stats.def = classes.hard_ceilings.def - 1;
        unit.hp = unit.stats.hp;
        let mastered = ClassRecord {
            class_level: classes.class_level_cap,
            class_points: 0,
        };
        unit.class_records.insert(unit.class.clone(), mastered);
        let saved = ClassRecord {
            class_level: 6,
            class_points: 0,
        };
        unit.class_records.insert(ClassId("rider".into()), saved);
        let mut stock = Stock::default();
        for (id, item) in &content.items.items {
            if matches!(item, ItemDef::Seal(_)) {
                stock.add(id.clone());
            }
        }
        Some(Self::new(ctx, kind, unit, stock))
    }

    /// The unit, as it is now (changed once a change was confirmed).
    pub fn unit(&self) -> &Unit {
        &self.unit
    }

    /// The party's stock, as it is now.
    pub fn stock(&self) -> &Stock {
        &self.stock
    }

    /// The classes offered, in column order.
    pub fn options(&self) -> &[ClassOption] {
        &self.options
    }

    /// The column the cursor is on.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Whether the `Promote … ?` question is up.
    pub fn is_confirming(&self) -> bool {
        self.stage == Stage::Confirming
    }

    /// The result's pages, once a change was made.
    pub fn progress(&self) -> Option<&Progress> {
        match &self.stage {
            Stage::Result(p) => Some(p),
            _ => None,
        }
    }

    /// The option the cursor is on.
    fn focused(&self) -> Option<&ClassOption> {
        self.options.get(self.focus)
    }

    /// Moves the cursor one column (wrapping) and scrolls it into view.
    /// Returns whether it moved.
    fn move_focus(&mut self, forward: bool) -> bool {
        let n = self.options.len();
        if n < 2 {
            return false;
        }
        self.focus = if forward {
            (self.focus + 1) % n
        } else {
            (self.focus + n - 1) % n
        };
        // The window of `COLUMNS` columns moves just far enough to hold it.
        let leftmost = (self.focus + 1).saturating_sub(COLUMNS);
        self.first = self.first.min(self.focus).max(leftmost);
        true
    }

    /// Makes the change into the focused class and starts its pages.
    fn apply(&mut self, ctx: &Ctx) {
        let Some(target) = self.focused().map(|o| o.class.clone()) else {
            self.stage = Stage::Choosing;
            return;
        };
        let before = self.unit.clone();
        let made = change(
            self.kind,
            &mut self.unit,
            &target,
            &mut self.stock,
            &tables(ctx),
        );
        self.stage = match made {
            Ok(events) => {
                let content = &ctx.content;
                let names = ProgressTables {
                    classes: &content.classes,
                    skills: &content.skills,
                    spells: &content.spells,
                    items: &content.items,
                };
                Progress::with_tables(&events, &[before], &names, PROGRESS_TIMINGS)
                    .map_or(Stage::Done, Stage::Result)
            }
            // Only an option whose preview worked can be confirmed.
            Err(_) => Stage::Choosing,
        };
    }

    /// One action while picking a column. Returns whether to close.
    fn step_choosing(&mut self, ctx: &mut Ctx, action: Action) -> bool {
        match action {
            Action::CursorLeft | Action::CursorRight => {
                if self.move_focus(action == Action::CursorRight) {
                    ctx.audio.menu(MenuSound::Move);
                }
            }
            Action::Confirm => {
                if self.focused().is_some_and(|o| o.after.is_ok()) {
                    self.stage = Stage::Confirming;
                    ctx.audio.menu(MenuSound::Select);
                } else {
                    ctx.audio.menu(MenuSound::Denied);
                }
            }
            Action::Cancel => {
                ctx.audio.menu(MenuSound::Cancel);
                return true;
            }
            _ => {}
        }
        false
    }

    /// The name of the seal a change into `class` uses and how many the
    /// stock holds: the seal's item name, or the kind if no item is one.
    fn seal_of(&self, ctx: &Ctx, class: &ClassDef) -> (String, u32) {
        let kind = self.kind.seal(class);
        let items = &ctx.content.items;
        let held: u32 = self
            .stock
            .items
            .iter()
            .filter(|(id, _)| items.seal(id).is_some_and(|s| s.kind == kind))
            .map(|(_, n)| n)
            .sum();
        let name = items
            .items
            .values()
            .find_map(|d| match d {
                ItemDef::Seal(s) if s.kind == kind => Some(s.name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| match kind {
                SealKind::Tier(tier) => format!("tier {tier} seal"),
                SealKind::Reclass => "reclass seal".to_owned(),
            });
        (name, held)
    }

    /// Why the change into `class` is refused, in the player's words.
    fn reason(&self, ctx: &Ctx, class: &ClassDef, error: &ClassChangeError) -> String {
        match error {
            ClassChangeError::NoSeal(_) => format!("No {}", self.seal_of(ctx, class).0),
            ClassChangeError::NotMastered(_) => "Class not mastered".to_owned(),
            other => other.to_string(),
        }
    }

    /// The question asked before the change, and what it costs.
    fn question(&self, ctx: &Ctx, class: &ClassDef) -> [String; 2] {
        let ask = match self.kind {
            ChangeKind::Promote => format!("Promote {} to {}?", self.unit.name, class.name),
            ChangeKind::Reclass => {
                format!("Change {} to {}?", self.unit.name, class.name)
            }
        };
        let (seal, held) = self.seal_of(ctx, class);
        [ask, format!("Uses 1 {seal} ({held} in stock).")]
    }

    /// The help line for the stage.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
        let cancel = |label| (Some(key_name(km, Action::Cancel)), label);
        match &self.stage {
            Stage::Choosing => {
                let pick = format!(
                    "{}/{}",
                    key_name(km, Action::CursorLeft),
                    key_name(km, Action::CursorRight)
                );
                let many = (self.options.len() > 1).then_some(pick);
                let can = self.focused().is_some_and(|o| o.after.is_ok());
                let choose = (can.then(|| key_name(km, Action::Confirm)), "choose");
                help_line(&[(many, "class"), choose, cancel("back")])
            }
            Stage::Confirming => help_line(&[confirm("yes"), cancel("no")]),
            Stage::Result(p) => progress::help(p, km),
            Stage::Done => String::new(),
        }
    }

    /// The title, the unit's line and the seals in stock for the focused
    /// class.
    fn draw_header(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        let classes = &ctx.content.classes;
        buf.print(
            COLUMNS_X,
            TITLE_ROW,
            self.kind.title(),
            c(UiColor::TextHighlight),
            black,
        );
        let n = buf.print(
            COLUMNS_X,
            UNIT_ROW,
            &self.unit.name,
            c(UiColor::Player),
            black,
        );
        let class = classes
            .get(&self.unit.class)
            .map_or(self.unit.class.0.as_str(), |c| c.name.as_str());
        let line = format!("{class}  Lv {}", self.unit.level);
        buf.print(
            COLUMNS_X + i32::from(n) + 2,
            UNIT_ROW,
            &line,
            c(UiColor::Text),
            black,
        );
        let Some(class) = self.focused().and_then(|o| classes.get(&o.class)) else {
            return;
        };
        let (seal, held) = self.seal_of(ctx, class);
        let text = format!("{seal} ×{held}");
        let w = i32::try_from(text.chars().count()).unwrap_or(0);
        let right = COLUMNS_X + COLUMN_W * i32::try_from(COLUMNS).unwrap_or(0);
        let color = if held == 0 {
            UiColor::HpLow
        } else {
            UiColor::Text
        };
        buf.print(right - w, UNIT_ROW, &text, c(color), black);
    }

    /// The columns on screen, and the marks for those scrolled away.
    fn draw_columns(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        if self.options.is_empty() {
            let none = "No class to change to.";
            buf.print(COLUMNS_X, COLUMNS_Y + 1, none, c(UiColor::TextDim), black);
            return;
        }
        let shown = self.options.iter().enumerate().skip(self.first);
        for (slot, (index, option)) in (0..).zip(shown.take(COLUMNS)) {
            let x = COLUMNS_X + slot * COLUMN_W;
            self.draw_column(ctx, buf, x, option, index == self.focus);
        }
        let dim = c(UiColor::TextDim);
        if self.first > 0 {
            buf.print(COLUMNS_X, MORE_ROW, "← more", dim, black);
        }
        if self.first + COLUMNS < self.options.len() {
            let text = "more →";
            let w = i32::try_from(text.chars().count()).unwrap_or(0);
            let right = COLUMNS_X + COLUMN_W * i32::try_from(COLUMNS).unwrap_or(0);
            buf.print(right - w, MORE_ROW, text, dim, black);
        }
    }

    /// One class's column, its box's left edge at column `x`.
    fn draw_column(
        &self,
        ctx: &Ctx,
        buf: &mut GlyphBuffer,
        x: i32,
        option: &ClassOption,
        focused: bool,
    ) {
        let content = &ctx.content;
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let rect = Rect::new(x, COLUMNS_Y, COLUMN_W, COLUMN_H);
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        let (style, border) = if focused {
            (BoxStyle::Double, UiColor::PanelBorderFocus)
        } else {
            (BoxStyle::Single, UiColor::PanelBorder)
        };
        buf.draw_box(rect, style, c(border), bg);
        let Some(class) = content.classes.get(&option.class) else {
            return;
        };
        let offered = option.after.is_ok();
        // A refused class is dimmed whole.
        let shade = |color| if offered { color } else { UiColor::TextDim };
        let pen = |buf: &mut GlyphBuffer, row: i32, text: &str, color: UiColor| {
            let cut: String = text.chars().take(TEXT_W).collect();
            buf.print(x + 2, COLUMNS_Y + row, &cut, c(shade(color)), bg);
        };
        let title = if focused {
            UiColor::TextHighlight
        } else {
            UiColor::Text
        };
        pen(buf, 1, &class.name, title);
        let tier = format!("Tier {}", class.tier);
        let record = match (self.kind, self.unit.class_records.get(&class.id)) {
            (ChangeKind::Promote, _) => String::new(),
            (ChangeKind::Reclass, Some(r)) => format!("CL {}", r.class_level),
            (ChangeKind::Reclass, None) => "new".to_owned(),
        };
        pen(buf, 2, &format!("{tier:<9}{record}"), UiColor::TextDim);
        let movement = content
            .terrain
            .rules
            .movement_types
            .get(usize::from(class.movement_type.0))
            .map_or("", String::as_str);
        let mov = mov_line(self.unit.stats.mov, class.move_points);
        pen(buf, 4, &format!("{mov:<12}{movement}"), UiColor::Text);
        let tags: Vec<&str> = [
            (class.tags.mounted, "Mounted"),
            (class.tags.flying, "Flying"),
            (class.tags.armored, "Armored"),
        ]
        .iter()
        .filter(|(on, _)| *on)
        .map(|&(_, name)| name)
        .collect();
        pen(buf, 5, &tags.join(" "), UiColor::Text);
        pen(buf, 7, "Weapons", UiColor::TextHighlight);
        let weapons = weapon_lines(&self.unit, class);
        if weapons.is_empty() {
            pen(buf, 8, "--", UiColor::TextDim);
        }
        for (row, line) in (8..11).zip(&weapons) {
            pen(buf, row, line, UiColor::Text);
        }
        pen(buf, 12, "Active", UiColor::TextHighlight);
        let active = class.active.as_ref().map(|id| {
            content
                .skills
                .get(id)
                .map_or_else(|| id.0.clone(), |s| s.name.clone())
        });
        match active {
            Some(name) => pen(buf, 13, &name, UiColor::Text),
            None => pen(buf, 13, "--", UiColor::TextDim),
        }
        self.draw_stats(ctx, buf, x, option);
    }

    /// The stat rows of `option`'s column (its box's left edge at column
    /// `x`), and under them the reason a refused class is refused. A
    /// refused class changes nothing: its rows show the stats as they are,
    /// dimmed.
    fn draw_stats(&self, ctx: &Ctx, buf: &mut GlyphBuffer, x: i32, option: &ClassOption) {
        let content = &ctx.content;
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let offered = option.after.is_ok();
        let shade = |color| if offered { color } else { UiColor::TextDim };
        let after = option.after.as_ref().map_or(&self.unit, |u| u);
        for (row, kind) in (15..).zip(StatKind::GROWABLE) {
            let (before, then) = (self.unit.stats.get(kind), after.stats.get(kind));
            let ceiling = content.classes.hard_ceilings.get(kind);
            let line = stat_line(kind, before, then, ceiling);
            let color = if then > before {
                UiColor::Text
            } else {
                UiColor::TextDim
            };
            let (tx, ty) = (x + 2, COLUMNS_Y + row);
            buf.print(tx, ty, &line, c(shade(color)), bg);
            // The gain and the `MAX` mark in the highlight colour.
            let plain = progress::stat_row(kind, before, 0).chars().count();
            let tail: String = line.chars().skip(plain).collect();
            if let Some(at) = tail.find(['+', 'M']) {
                let skipped = plain + tail[..at].chars().count();
                let col = i32::try_from(skipped).unwrap_or(0);
                let hi = c(shade(UiColor::TextHighlight));
                buf.print(tx + col, ty, &tail[at..], hi, bg);
            }
        }
        if let (Err(error), Some(class)) = (&option.after, content.classes.get(&option.class)) {
            let reason: String = self
                .reason(ctx, class, error)
                .chars()
                .take(TEXT_W)
                .collect();
            buf.print(x + 2, COLUMNS_Y + 23, &reason, c(UiColor::HpLow), bg);
        }
    }

    /// The `Promote … ?` box, centred.
    fn draw_question(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let Some(class) = self
            .focused()
            .and_then(|o| ctx.content.classes.get(&o.class))
        else {
            return;
        };
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let km = ctx.help_keys();
        let yes_no = help_line(&[
            (Some(key_name(km, Action::Confirm)), "yes"),
            (Some(key_name(km, Action::Cancel)), "no"),
        ])
        .replace(SEPARATOR, " / ");
        let [ask, cost] = self.question(ctx, class);
        let lines = [
            (ask, UiColor::Text),
            (cost, UiColor::Text),
            (yes_no, UiColor::TextDim),
        ];
        let widest = lines.iter().map(|(l, _)| l.chars().count()).max();
        let w = i32::try_from(widest.unwrap_or(0)).unwrap_or(0) + 4;
        let h = i32::try_from(lines.len()).unwrap_or(0) + 4;
        let (bw, bh) = (i32::from(buf.width()), i32::from(buf.height()));
        let rect = Rect::new((bw - w) / 2, (bh - h) / 2, w, h);
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
        for (y, (line, color)) in (rect.y + 2..).zip(&lines) {
            buf.print(rect.x + 2, y, line, c(*color), bg);
        }
    }
}

impl Screen for ClassChangeScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            match &mut self.stage {
                Stage::Choosing => {
                    if self.step_choosing(ctx, action) {
                        return Transition::Pop;
                    }
                }
                Stage::Confirming => match action {
                    Action::Confirm => {
                        ctx.audio.menu(MenuSound::Select);
                        self.apply(ctx);
                    }
                    Action::Cancel => {
                        ctx.audio.menu(MenuSound::Cancel);
                        self.stage = Stage::Choosing;
                    }
                    _ => {}
                },
                // Confirm or Cancel finishes or closes a page, as in battle.
                Stage::Result(pages) => {
                    if matches!(action, Action::Confirm | Action::Cancel) {
                        pages.confirm();
                    }
                }
                Stage::Done => {}
            }
        }
        if let Stage::Result(pages) = &mut self.stage {
            pages.tick(input.dt, input.is_held(Action::Confirm));
            if pages.done() {
                self.stage = Stage::Done;
            }
        }
        if self.stage == Stage::Done {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        match &self.stage {
            Stage::Choosing => {
                self.draw_header(ctx, buf);
                self.draw_columns(ctx, buf);
            }
            Stage::Confirming => {
                self.draw_header(ctx, buf);
                self.draw_columns(ctx, buf);
                self.draw_question(ctx, buf);
            }
            // The pages name the unit and its classes themselves.
            Stage::Result(pages) => {
                progress::draw(buf, &ctx.palette, &ctx.content.portraits, pages);
            }
            Stage::Done => {}
        }
        let bottom = i32::from(buf.height()) - 1;
        buf.print(1, bottom, &self.help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests;
