//! The lead screen, after the mode (ticket 0801,
//! `docs/design/setting-and-tone.md`): pick the lead's gender, shown with
//! the `lead_m` and `lead_f` portraits, and first name (default
//! [`DEFAULT_NAME`]; the family name [`FAMILY_NAME`] is fixed).
//!
//! Keys never type letters (the `keyboard-input` skill: letters are other
//! actions' keys, and players rebind them), so the name is spelled on a
//! grid of letters with the cursor keys and Confirm, like a console
//! game's naming screen (*Claude's starting design*).

use trpg_core::lead::{DEFAULT_NAME, FAMILY_NAME, MAX_NAME_LEN};
use trpg_core::{LeadGender, LeadProfile};

use super::print_centred;
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::portrait::draw_portrait;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};

/// The heading.
pub const HEADING: &str = "Choose your lead";
/// The button that starts the game.
pub const START: &str = "Start";
/// Label of the name row.
const NAME_LABEL: &str = "Name";
/// The name grid's cell that types a space.
pub const SPACE: &str = "Blank";
/// Deletes the last letter.
pub const DELETE: &str = "Delete";
/// Closes the name grid.
pub const DONE: &str = "Done";

/// Row of the heading.
const HEADING_ROW: i32 = 1;
/// Portrait frame size: a 32×16-cell portrait plus its border.
const FRAME: (i32, i32) = (34, 18);
/// Top row of the portrait frames.
const FRAME_Y: i32 = 3;
/// Left column of the male and female frames.
const FRAME_X: [i32; 2] = [14, 52];
/// Row of the gender labels under the frames.
const LABEL_ROW: i32 = 21;
/// Row of the name.
const NAME_ROW: i32 = 24;
/// Row of the Start button.
const START_ROW: i32 = 27;
/// The name grid's box.
const GRID_BOX: Rect = Rect::new(27, 8, 46, 14);

/// The letters of the name grid, one row each.
const LETTER_ROWS: [&str; 4] = [
    "ABCDEFGHIJKLM",
    "NOPQRSTUVWXYZ",
    "abcdefghijklm",
    "nopqrstuvwxyz",
];

/// One cell of the name grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridCell {
    /// Types this character.
    Char(char),
    /// Types a space (between two words).
    Space,
    /// Deletes the last character.
    Delete,
    /// Closes the grid, keeping the name.
    Done,
}

impl GridCell {
    /// Text shown for the cell.
    pub fn label(self) -> String {
        match self {
            GridCell::Char(c) => c.to_string(),
            GridCell::Space => SPACE.to_owned(),
            GridCell::Delete => DELETE.to_owned(),
            GridCell::Done => DONE.to_owned(),
        }
    }
}

/// The name grid's rows: four rows of letters, then `-`, `'`, Blank,
/// Delete and Done.
pub fn grid() -> Vec<Vec<GridCell>> {
    let mut rows: Vec<Vec<GridCell>> = LETTER_ROWS
        .iter()
        .map(|r| r.chars().map(GridCell::Char).collect())
        .collect();
    rows.push(vec![
        GridCell::Char('-'),
        GridCell::Char('\''),
        GridCell::Space,
        GridCell::Delete,
        GridCell::Done,
    ]);
    rows
}

/// The name grid, open over the lead screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    /// The name being spelled.
    name: String,
    /// The name before the grid opened (Cancel on an empty name brings it
    /// back).
    before: String,
    /// The focused cell: row, column.
    at: (usize, usize),
}

/// What a key did in the name grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryEvent {
    /// Still spelling.
    Open,
    /// Closed; the name is its result.
    Closed,
}

impl NameEntry {
    /// The grid for `name`, on its first letter.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            before: name.to_owned(),
            at: (0, 0),
        }
    }

    /// The name spelled so far.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The focused cell.
    pub fn focus(&self) -> (usize, usize) {
        self.at
    }

    /// The name's length in characters.
    fn len(&self) -> usize {
        self.name.chars().count()
    }

    /// Handles `action`: the cursor keys move over the grid (wrapping along
    /// a row, keeping to the last cell of a shorter row), Confirm uses the
    /// focused cell, Cancel deletes the last character (on an empty name it
    /// closes the grid, putting the old name back). Plays the menu sounds.
    fn handle(&mut self, action: Action, ctx: &mut Ctx) -> EntryEvent {
        let rows = grid();
        let (row, col) = self.at;
        let width = |r: usize| rows[r].len();
        let moved = match action {
            Action::CursorLeft => (row, (col + width(row) - 1) % width(row)),
            Action::CursorRight => (row, (col + 1) % width(row)),
            Action::CursorUp => {
                let r = (row + rows.len() - 1) % rows.len();
                (r, col.min(width(r) - 1))
            }
            Action::CursorDown => {
                let r = (row + 1) % rows.len();
                (r, col.min(width(r) - 1))
            }
            Action::Confirm => return self.confirm(rows[row][col], ctx),
            Action::Cancel => {
                if self.name.pop().is_some() {
                    ctx.audio.menu(MenuSound::Cancel);
                    return EntryEvent::Open;
                }
                self.name.clone_from(&self.before);
                ctx.audio.menu(MenuSound::Cancel);
                return EntryEvent::Closed;
            }
            _ => return EntryEvent::Open,
        };
        if moved != self.at {
            self.at = moved;
            ctx.audio.menu(MenuSound::Move);
        }
        EntryEvent::Open
    }

    /// Confirm on `cell`. A character past [`MAX_NAME_LEN`], a space at
    /// the start or after another, and Done on a blank name are refused.
    fn confirm(&mut self, cell: GridCell, ctx: &mut Ctx) -> EntryEvent {
        let full = self.len() >= MAX_NAME_LEN;
        let ok = match cell {
            GridCell::Char(c) if !full => {
                self.name.push(c);
                true
            }
            GridCell::Space if !full && !self.name.is_empty() && !self.name.ends_with(' ') => {
                self.name.push(' ');
                true
            }
            GridCell::Delete => self.name.pop().is_some(),
            GridCell::Done if !self.name.trim().is_empty() => {
                self.name = self.name.trim().to_owned();
                ctx.audio.menu(MenuSound::Select);
                return EntryEvent::Closed;
            }
            _ => false,
        };
        ctx.audio.menu(if ok {
            MenuSound::Select
        } else {
            MenuSound::Denied
        });
        EntryEvent::Open
    }
}

/// Which row of the lead screen has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// The two portraits: Left and Right pick the gender.
    Gender,
    /// The first name: Confirm opens the grid.
    Name,
    /// Confirm starts the game.
    Start,
}

impl Row {
    const ALL: [Row; 3] = [Row::Gender, Row::Name, Row::Start];
}

/// Gender and first name of the lead. Confirm on `Start` pops with the
/// profile ([`result`](Self::result)); Cancel pops without one (back to the
/// mode).
#[derive(Debug, Clone)]
pub struct LeadSelectScreen {
    gender: LeadGender,
    name: String,
    row: Row,
    entry: Option<NameEntry>,
    result: Option<LeadProfile>,
}

impl LeadSelectScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "lead_select";

    /// The screen with the male lead and the default name, focus on the
    /// portraits.
    pub fn new() -> Self {
        Self {
            gender: LeadGender::Male,
            name: DEFAULT_NAME.to_owned(),
            row: Row::Gender,
            entry: None,
            result: None,
        }
    }

    /// The lead the player made, once the screen has popped; `None` if they
    /// went back.
    pub fn result(&self) -> Option<&LeadProfile> {
        self.result.as_ref()
    }

    /// The gender picked so far.
    pub fn gender(&self) -> LeadGender {
        self.gender
    }

    /// The first name so far (the grid's, while it is open).
    pub fn first_name(&self) -> &str {
        self.entry.as_ref().map_or(&self.name, |e| e.name())
    }

    /// The focused row.
    pub fn row(&self) -> Row {
        self.row
    }

    /// The name grid, while open.
    pub fn entry(&self) -> Option<&NameEntry> {
        self.entry.as_ref()
    }

    /// The bottom help line.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        let keys = Some(cursor_keys_name(km));
        let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
        let cancel = |label| (Some(key_name(km, Action::Cancel)), label);
        if self.entry.is_some() {
            return help_line(&[(keys, "choose"), confirm("type"), cancel("delete")]);
        }
        let label = match self.row {
            Row::Gender => "next",
            Row::Name => "change name",
            Row::Start => "start",
        };
        help_line(&[(keys, "choose"), confirm(label), cancel("back")])
    }

    /// Moves the focus one row down (`down`) or up, wrapping.
    fn step_row(&mut self, down: bool, ctx: &mut Ctx) {
        let n = Row::ALL.len();
        let i = Row::ALL.iter().position(|&r| r == self.row).unwrap_or(0);
        let next = if down { (i + 1) % n } else { (i + n - 1) % n };
        self.row = Row::ALL[next];
        ctx.audio.menu(MenuSound::Move);
    }

    /// Handles one action outside the grid. Returns whether the screen is
    /// done (chose Start or went back).
    fn step(&mut self, action: Action, ctx: &mut Ctx) -> bool {
        match (self.row, action) {
            (_, Action::CursorDown) => self.step_row(true, ctx),
            (_, Action::CursorUp) => self.step_row(false, ctx),
            (Row::Gender, Action::CursorLeft | Action::CursorRight) => {
                self.gender = match self.gender {
                    LeadGender::Male => LeadGender::Female,
                    LeadGender::Female => LeadGender::Male,
                };
                ctx.audio.menu(MenuSound::Move);
            }
            (Row::Gender, Action::Confirm) => {
                self.row = Row::Name;
                ctx.audio.menu(MenuSound::Select);
            }
            (Row::Name, Action::Confirm) => {
                self.entry = Some(NameEntry::new(&self.name));
                ctx.audio.menu(MenuSound::Select);
            }
            (Row::Start, Action::Confirm) => {
                ctx.audio.menu(MenuSound::Select);
                self.result = Some(LeadProfile::new(self.name.clone(), self.gender));
                return true;
            }
            (_, Action::Cancel) => {
                ctx.audio.menu(MenuSound::Cancel);
                return true;
            }
            _ => {}
        }
        false
    }

    /// One gender's frame, portrait and label; the chosen one is lit, with
    /// a double border while the gender row has the focus.
    fn draw_gender(&self, ctx: &Ctx, buf: &mut GlyphBuffer, gender: LeadGender, x: i32) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let chosen = gender == self.gender;
        let (style, border) = match (chosen, self.row == Row::Gender) {
            (true, true) => (BoxStyle::Double, UiColor::PanelBorderFocus),
            (true, false) => (BoxStyle::Single, UiColor::PanelBorderFocus),
            (false, _) => (BoxStyle::Single, UiColor::PanelBorder),
        };
        let frame = Rect::new(x, FRAME_Y, FRAME.0, FRAME.1);
        buf.fill_rect(frame, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(frame, style, c(border), bg);
        let profile = LeadProfile::new(self.name.clone(), gender);
        if let Some(art) = ctx.content.portraits.get(profile.portrait_id()) {
            let dim = if chosen { 0.0 } else { 0.45 };
            draw_portrait(
                buf,
                &ctx.palette,
                (x + 1, FRAME_Y + 1),
                art,
                "neutral",
                dim,
                false,
            );
        }
        let label = match gender {
            LeadGender::Male => "Male",
            LeadGender::Female => "Female",
        };
        let fg = if chosen {
            UiColor::TextHighlight
        } else {
            UiColor::TextDim
        };
        let w = i32::try_from(label.chars().count()).unwrap_or(0);
        let black = c(UiColor::Black);
        buf.print(x + (FRAME.0 - w) / 2, LABEL_ROW, label, c(fg), black);
    }

    /// The name row and the Start button, the focused one lit.
    fn draw_rows(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        let lit = |row| {
            if self.row == row && self.entry.is_none() {
                UiColor::TextHighlight
            } else {
                UiColor::Text
            }
        };
        let first = self.first_name();
        let full = format!("{NAME_LABEL}   {first} {FAMILY_NAME}");
        let x = (i32::from(buf.width()) - i32::try_from(full.chars().count()).unwrap_or(0)) / 2;
        buf.print(x, NAME_ROW, NAME_LABEL, c(UiColor::TextDim), black);
        let name_x = x + i32::try_from(NAME_LABEL.len() + 3).unwrap_or(0);
        buf.print(name_x, NAME_ROW, first, c(lit(Row::Name)), black);
        let family_x = name_x + i32::try_from(first.chars().count() + 1).unwrap_or(0);
        buf.print(family_x, NAME_ROW, FAMILY_NAME, c(UiColor::TextDim), black);
        let start = format!("[ {START} ]");
        print_centred(buf, START_ROW, &start, c(lit(Row::Start)), black);
    }

    /// The name grid's box: the name so far with a cursor, then the grid,
    /// the focused cell as a lit bar.
    fn draw_entry(ctx: &Ctx, buf: &mut GlyphBuffer, entry: &NameEntry) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        buf.fill_rect(GRID_BOX, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(GRID_BOX, BoxStyle::Double, c(UiColor::PanelBorder), bg);
        buf.print(
            GRID_BOX.x + 2,
            GRID_BOX.y,
            " First name ",
            c(UiColor::TextHighlight),
            bg,
        );
        let shown = format!("{}_", entry.name());
        let w = i32::try_from(shown.chars().count()).unwrap_or(0);
        let name_x = GRID_BOX.x + (GRID_BOX.w - w) / 2;
        buf.print(name_x, GRID_BOX.y + 2, &shown, c(UiColor::Text), bg);
        for (r, row) in grid().iter().enumerate() {
            let y = GRID_BOX.y + 4 + i32::try_from(r * 2).unwrap_or(0);
            let labels: Vec<String> = row.iter().map(|g| g.label()).collect();
            // Each cell is its label with a space either side.
            let widths: Vec<i32> = labels
                .iter()
                .map(|l| i32::try_from(l.chars().count()).unwrap_or(0) + 2)
                .collect();
            let total: i32 = widths.iter().sum();
            let mut x = GRID_BOX.x + (GRID_BOX.w - total) / 2;
            for (col, (label, w)) in labels.iter().zip(&widths).enumerate() {
                let (fg, cell_bg) = if entry.focus() == (r, col) {
                    (c(UiColor::PanelBg), c(UiColor::PanelBorderFocus))
                } else {
                    (c(UiColor::Text), bg)
                };
                buf.print(x, y, &format!(" {label} "), fg, cell_bg);
                x += w;
            }
        }
    }
}

impl Default for LeadSelectScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for LeadSelectScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            if let Some(entry) = &mut self.entry {
                if entry.handle(action, ctx) == EntryEvent::Closed {
                    self.name = entry.name().to_owned();
                    self.entry = None;
                }
                continue;
            }
            if self.step(action, ctx) {
                return Transition::Pop;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        print_centred(buf, HEADING_ROW, HEADING, c(UiColor::TextHighlight), black);
        for (gender, x) in LeadGender::ALL.into_iter().zip(FRAME_X) {
            self.draw_gender(ctx, buf, gender, x);
        }
        self.draw_rows(ctx, buf);
        if let Some(entry) = &self.entry {
            Self::draw_entry(ctx, buf, entry);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests;
