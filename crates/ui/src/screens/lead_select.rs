//! The lead screen, after the mode (ticket 0801,
//! `docs/design/setting-and-tone.md`): pick the lead's gender, shown with
//! the `lead_m` and `lead_f` portraits, and first name (default
//! [`DEFAULT_NAME`]; the family name [`FAMILY_NAME`] is fixed).
//!
//! The name (Nick, PR #127: "C"): on a keyboard the player **types** it
//! ([`NameBox`]). While the box is open the game's keys do nothing (Select
//! and the rest are letters then), and a hint says to type; the text box's
//! own fixed keys finish, delete and cancel ([`crate::input::text_key`]).
//! When the name is chosen with a controller button, it is spelled on a
//! letter grid instead ([`NameEntry`]) with the cursor and Confirm, like a
//! console naming screen.
//!
//! Name rules for both: letters (A-Z, a-z), `-`, `'` and single spaces
//! between words, at most [`MAX_NAME_LEN`] characters, not blank.

use trpg_core::lead::{DEFAULT_NAME, FAMILY_NAME, MAX_NAME_LEN};
use trpg_core::{LeadGender, LeadProfile};

use super::print_centred;
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::{Action, TextKey, text_key, text_keys_help};
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
/// The typing box.
const TYPE_BOX: Rect = Rect::new(27, 10, 46, 7);
/// The typing box's hint.
pub const TYPE_HINT: &str = "Type a name on your keyboard.";

/// Adds `c` to `name` if the name rules allow it (see the module docs).
fn add_char(name: &mut String, c: char) -> bool {
    let full = name.chars().count() >= MAX_NAME_LEN;
    let ok = match c {
        _ if full => false,
        ' ' => !name.is_empty() && !name.ends_with(' '),
        c => c.is_ascii_alphabetic() || c == '-' || c == '\'',
    };
    if ok {
        name.push(c);
    }
    ok
}

/// `name` finished: trimmed, or `None` if blank.
fn finished(name: &str) -> Option<String> {
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// The name being typed on the keyboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameBox {
    /// The name typed so far.
    name: String,
    /// The name before the box opened (Cancel brings it back).
    before: String,
}

impl NameBox {
    /// The box for `name`, which the player types on from.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            before: name.to_owned(),
        }
    }

    /// The name typed so far.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One frame: the characters typed, then the text box's keys. Actions
    /// are ignored. A character the rules refuse sounds denied; typing
    /// makes no sound.
    fn handle(&mut self, input: &FrameInput, ctx: &mut Ctx) -> EntryEvent {
        for &c in input.text() {
            if !add_char(&mut self.name, c) {
                ctx.audio.menu(MenuSound::Denied);
            }
        }
        for &chord in input.pressed_chords() {
            match text_key(chord) {
                Some(TextKey::Done) => match finished(&self.name) {
                    Some(name) => {
                        self.name = name;
                        ctx.audio.menu(MenuSound::Select);
                        return EntryEvent::Closed;
                    }
                    None => ctx.audio.menu(MenuSound::Denied),
                },
                Some(TextKey::Delete) => {
                    if self.name.pop().is_some() {
                        ctx.audio.menu(MenuSound::Cancel);
                    }
                }
                Some(TextKey::Cancel) => {
                    self.name.clone_from(&self.before);
                    ctx.audio.menu(MenuSound::Cancel);
                    return EntryEvent::Closed;
                }
                None => {}
            }
        }
        EntryEvent::Open
    }
}

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
        let ok = match cell {
            GridCell::Char(c) => add_char(&mut self.name, c),
            GridCell::Space => add_char(&mut self.name, ' '),
            GridCell::Delete => self.name.pop().is_some(),
            GridCell::Done => {
                if let Some(name) = finished(&self.name) {
                    self.name = name;
                    ctx.audio.menu(MenuSound::Select);
                    return EntryEvent::Closed;
                }
                false
            }
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
    /// The first name: Confirm opens the typing box.
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
    /// The typing box, while open.
    typing: Option<NameBox>,
    /// The letter grid, while open (controller players).
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
            typing: None,
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

    /// The first name so far (the typing box's or the grid's, while open).
    pub fn first_name(&self) -> &str {
        match (&self.typing, &self.entry) {
            (Some(t), _) => t.name(),
            (None, Some(e)) => e.name(),
            (None, None) => &self.name,
        }
    }

    /// The typing box, while open.
    pub fn typing(&self) -> Option<&NameBox> {
        self.typing.as_ref()
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
        if self.typing.is_some() {
            return text_keys_help();
        }
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

    /// Handles one action outside the grid. `pad`: it came from a
    /// controller, so the name opens the letter grid, not the typing box.
    /// Returns whether the screen is done (chose Start or went back).
    fn step(&mut self, action: Action, pad: bool, ctx: &mut Ctx) -> bool {
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
                if pad {
                    self.entry = Some(NameEntry::new(&self.name));
                } else {
                    self.typing = Some(NameBox::new(&self.name));
                }
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
            if self.row == row && self.entry.is_none() && self.typing.is_none() {
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

    /// The typing box: the name so far with a cursor, and the hint to type.
    fn draw_typing(ctx: &Ctx, buf: &mut GlyphBuffer, typing: &NameBox) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        buf.fill_rect(TYPE_BOX, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(TYPE_BOX, BoxStyle::Double, c(UiColor::PanelBorder), bg);
        let title = " First name ";
        buf.print(
            TYPE_BOX.x + 2,
            TYPE_BOX.y,
            title,
            c(UiColor::TextHighlight),
            bg,
        );
        let centred = |text: &str| {
            let w = i32::try_from(text.chars().count()).unwrap_or(0);
            TYPE_BOX.x + (TYPE_BOX.w - w) / 2
        };
        let shown = format!("{}_", typing.name());
        buf.print(
            centred(&shown),
            TYPE_BOX.y + 2,
            &shown,
            c(UiColor::Text),
            bg,
        );
        let hint_fg = c(UiColor::TextDim);
        buf.print(centred(TYPE_HINT), TYPE_BOX.y + 4, TYPE_HINT, hint_fg, bg);
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
        // While typing, the game's keys do nothing: they are letters.
        if let Some(typing) = &mut self.typing {
            if typing.handle(input, ctx) == EntryEvent::Closed {
                self.name = typing.name().to_owned();
                self.typing = None;
            }
            return Transition::None;
        }
        for &action in &input.actions {
            if let Some(entry) = &mut self.entry {
                if entry.handle(action, ctx) == EntryEvent::Closed {
                    self.name = entry.name().to_owned();
                    self.entry = None;
                }
                continue;
            }
            if self.step(action, input.pad_pressed(), ctx) {
                return Transition::Pop;
            }
            // The key that opened the typing box types nothing, and the
            // rest of this frame's keys wait for the box.
            if self.typing.is_some() {
                break;
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
        if let Some(typing) = &self.typing {
            Self::draw_typing(ctx, buf, typing);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests;
