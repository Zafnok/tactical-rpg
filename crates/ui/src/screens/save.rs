//! The save screens (ticket 0802, `docs/design/death-and-difficulty.md`):
//! "Save your progress?" after a chapter's victory ([`SavePromptScreen`])
//! and the slot picker ([`SlotPickerScreen`]), which saves the campaign into
//! one of the [`SLOTS`](crate::save::SLOTS) slots (asking before it overwrites one) or loads one.
//!
//! Both pop when they are done and say what happened through `result`; the
//! game flow hosts them ([`crate::flow`]). The slot picker is made to be
//! opened from other menus too (the world map's `Save`, 1007).

use trpg_core::{Campaign, GameMode, SaveFile};

use super::print_centred;
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::save::{self, Slot, playtime_text, slot_key};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{SEPARATOR, cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// The question after a chapter's victory.
pub const SAVE_QUESTION: &str = "Save your progress?";
/// The slot picker's title when saving.
pub const SAVE_TITLE: &str = "Save";
/// The slot picker's title when loading.
pub const LOAD_TITLE: &str = "Load Game";
/// What an empty slot shows.
pub const EMPTY_SLOT: &str = "Empty";

/// Row of the question.
const QUESTION_ROW: i32 = 10;
/// Top row of the question's menu.
const MENU_ROW: i32 = 13;

/// The slot picker's panel, in cells.
const PANEL: Rect = Rect::new(2, 1, 96, 26);
/// Row of the column headings.
const HEADING_ROW: i32 = PANEL.y + 2;
/// Row of the first slot shown.
const FIRST_ROW: i32 = PANEL.y + 4;
/// Slots shown at once.
const VISIBLE: usize = 20;
/// Column of the slot numbers.
const SLOT_X: i32 = PANEL.x + 3;
/// Column of the chapter titles.
const CHAPTER_X: i32 = SLOT_X + 6;
/// Cells a chapter title may take.
const CHAPTER_W: usize = 48;
/// Column of the game modes.
const MODE_X: i32 = CHAPTER_X + 50;
/// Column of the army sizes.
const ARMY_X: i32 = MODE_X + 10;
/// Column where the playtimes end (they are right-aligned).
const TIME_END_X: i32 = PANEL.x + PANEL.w - 3;
/// Row of the message under the panel.
const MESSAGE_ROW: i32 = PANEL.y + PANEL.h + 1;
/// Height of the overwrite question's box.
const OVERWRITE_H: i32 = 6;

/// "Save your progress?" with `Yes` / `No`. Pops once one is chosen
/// ([`result`](Self::result)); there is nothing to back out of.
#[derive(Debug, Clone)]
pub struct SavePromptScreen {
    menu: Menu,
    chosen: Option<bool>,
}

impl SavePromptScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "save_prompt";

    /// The question with `Yes` focused.
    pub fn new() -> Self {
        let items = ["Yes", "No"].map(MenuItem::new).to_vec();
        Self {
            menu: Menu::new(items).without_cancel(),
            chosen: None,
        }
    }

    /// Whether the player wants to save, once the screen has popped.
    pub fn result(&self) -> Option<bool> {
        self.chosen
    }

    /// The bottom help line.
    pub fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        help_line(&[
            (Some(cursor_keys_name(km)), "choose"),
            (Some(key_name(km, Action::Confirm)), "select"),
        ])
    }
}

impl Default for SavePromptScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for SavePromptScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            if let Some(MenuEvent::Chosen(i)) = self.menu.handle_with_sound(action, &mut ctx.audio)
            {
                self.chosen = Some(i == 0);
                return Transition::Pop;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        let highlight = c(UiColor::TextHighlight);
        print_centred(buf, QUESTION_ROW, SAVE_QUESTION, highlight, black);
        let (w, _) = self.menu.size();
        let x = (i32::from(buf.width()) - w) / 2;
        self.menu.draw(&ctx.palette, buf, x, MENU_ROW);
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &Self::help(ctx), c(UiColor::TextDim), black);
    }
}

/// What the slot picker is open for.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Purpose {
    /// Saving this campaign, whose chapter was just won.
    Save(Box<Campaign>),
    /// Loading a chapter save.
    Load,
}

/// What the player did in the slot picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotOutcome {
    /// The campaign was saved into this slot (1 to [`SLOTS`](crate::save::SLOTS)).
    Saved(usize),
    /// This slot's save was chosen to load.
    Loaded(usize, Box<SaveFile>),
}

/// The [`SLOTS`](crate::save::SLOTS) save slots, 20 at a time: each with its chapter,
/// game mode, army size and playtime. Confirm saves into the focused slot
/// (after "Overwrite slot N?" if it holds something) or loads it; Cancel
/// goes back. Pops either way; [`result`](Self::result) says which.
#[derive(Debug, Clone)]
pub struct SlotPickerScreen {
    purpose: Purpose,
    slots: Vec<Slot>,
    /// Index of the focused slot (slot number − 1).
    focus: usize,
    /// Index of the first slot shown.
    top: usize,
    /// Whether "Overwrite slot N?" is open.
    asking: bool,
    /// The message under the panel, if any.
    message: Option<String>,
    outcome: Option<SlotOutcome>,
}

impl SlotPickerScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "save_slots";

    /// The picker for saving `campaign` (its chapter just won), focused on
    /// slot `focus` (the one last used), else the first empty slot.
    pub fn save(ctx: &Ctx, campaign: Campaign, focus: Option<usize>) -> Self {
        let slots = save::slots(ctx.storage.as_ref(), &ctx.content);
        let empty = slots.iter().position(|s| *s == Slot::Empty);
        Self::new(Purpose::Save(Box::new(campaign)), slots, focus, empty)
    }

    /// The picker for loading, focused on slot `focus` (the one last used),
    /// else the first slot with a save.
    pub fn load(ctx: &Ctx, focus: Option<usize>) -> Self {
        let slots = save::slots(ctx.storage.as_ref(), &ctx.content);
        let saved = slots.iter().position(|s| matches!(s, Slot::Saved(_)));
        Self::new(Purpose::Load, slots, focus, saved)
    }

    fn new(purpose: Purpose, slots: Vec<Slot>, slot: Option<usize>, or: Option<usize>) -> Self {
        let last = slots.len().saturating_sub(1);
        let focus = slot
            .and_then(|n| n.checked_sub(1))
            .or(or)
            .unwrap_or(0)
            .min(last);
        let mut picker = Self {
            purpose,
            slots,
            focus,
            top: 0,
            asking: false,
            message: None,
            outcome: None,
        };
        picker.scroll();
        picker
    }

    /// What the player did, once the screen has popped; `None` if they
    /// went back.
    pub fn result(&self) -> Option<&SlotOutcome> {
        self.outcome.as_ref()
    }

    /// [`result`](Self::result), taking it.
    pub fn into_result(self) -> Option<SlotOutcome> {
        self.outcome
    }

    /// The focused slot's number (1 to [`SLOTS`](crate::save::SLOTS)).
    pub fn focused_slot(&self) -> usize {
        self.focus + 1
    }

    /// Whether the picker saves (rather than loads).
    pub fn is_saving(&self) -> bool {
        matches!(self.purpose, Purpose::Save(_))
    }

    /// Whether "Overwrite slot N?" is open.
    pub fn is_asking(&self) -> bool {
        self.asking
    }

    /// The message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The overwrite question for the focused slot.
    pub fn overwrite_question(&self) -> String {
        format!("Overwrite slot {:02}?", self.focused_slot())
    }

    /// Keeps the focused slot among those shown.
    fn scroll(&mut self) {
        if self.focus < self.top {
            self.top = self.focus;
        } else if self.focus >= self.top + VISIBLE {
            self.top = self.focus + 1 - VISIBLE;
        }
    }

    /// Moves the focus one slot down or up, wrapping.
    fn step(&mut self, ctx: &mut Ctx, down: bool) {
        let n = self.slots.len();
        if n < 2 {
            return;
        }
        self.focus = if down {
            (self.focus + 1) % n
        } else {
            (self.focus + n - 1) % n
        };
        self.scroll();
        self.message = None;
        ctx.audio.menu(MenuSound::Move);
    }

    /// Writes the campaign into the focused slot. Returns whether it was
    /// saved; a failure shows as the message.
    fn write(&mut self, ctx: &mut Ctx) -> bool {
        let Purpose::Save(campaign) = &self.purpose else {
            return false;
        };
        let file = SaveFile::chapter_cleared(campaign.as_ref().clone());
        let slot = self.focused_slot();
        match save::write(ctx.storage.as_mut(), &slot_key(slot), &file) {
            Ok(()) => {
                self.outcome = Some(SlotOutcome::Saved(slot));
                ctx.audio.menu(MenuSound::Select);
                true
            }
            Err(e) => {
                self.message = Some(e.to_string());
                ctx.audio.menu(MenuSound::Denied);
                false
            }
        }
    }

    /// Confirm on the focused slot. Returns whether the picker is done.
    fn confirm(&mut self, ctx: &mut Ctx) -> bool {
        self.message = None;
        let slot = self.slots.get(self.focus);
        if self.is_saving() {
            if slot.is_none_or(|s| *s == Slot::Empty) {
                return self.write(ctx);
            }
            self.asking = true;
            ctx.audio.menu(MenuSound::Select);
            return false;
        }
        match slot {
            Some(Slot::Saved(summary)) => {
                let save = Box::new(summary.save.clone());
                self.outcome = Some(SlotOutcome::Loaded(self.focused_slot(), save));
                ctx.audio.menu(MenuSound::Select);
                true
            }
            Some(Slot::Unreadable(e)) => {
                self.message = Some(e.to_string());
                ctx.audio.menu(MenuSound::Denied);
                false
            }
            Some(Slot::Empty) | None => {
                ctx.audio.menu(MenuSound::Denied);
                false
            }
        }
    }

    /// An action while the overwrite question is open. Returns whether the
    /// picker is done.
    fn answer(&mut self, ctx: &mut Ctx, action: Action) -> bool {
        match action {
            Action::Confirm => {
                self.asking = false;
                self.write(ctx)
            }
            Action::Cancel => {
                self.asking = false;
                ctx.audio.menu(MenuSound::Cancel);
                false
            }
            _ => false,
        }
    }

    /// The bottom help line.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        let confirm = |label| (Some(key_name(km, Action::Confirm)), label);
        let cancel = |label| (Some(key_name(km, Action::Cancel)), label);
        if self.asking {
            return help_line(&[confirm("yes"), cancel("no")]);
        }
        let choose = (Some(cursor_keys_name(km)), "choose");
        let pick = confirm(if self.is_saving() {
            "save here"
        } else {
            "load"
        });
        help_line(&[choose, pick, cancel("back")])
    }

    /// Draws slot `index` on console row `y`.
    fn draw_slot(&self, ctx: &Ctx, buf: &mut GlyphBuffer, index: usize, y: i32) {
        let color = |u| ctx.palette.get(u);
        let panel = color(UiColor::PanelBg);
        let focused = index == self.focus;
        let (bg, text, dim, bad) = if focused {
            let bar = color(UiColor::PanelBorderFocus);
            buf.fill_rect(
                Rect::new(PANEL.x + 1, y, PANEL.w - 2, 1),
                Cell::new(' ', panel, bar),
            );
            // On the focus bar every colour is the bar's text colour.
            (bar, panel, panel, panel)
        } else {
            (
                panel,
                color(UiColor::Text),
                color(UiColor::TextDim),
                color(UiColor::HpLow),
            )
        };
        buf.print(SLOT_X, y, &format!("{:02}", index + 1), text, bg);
        match self.slots.get(index) {
            Some(Slot::Saved(saved)) => {
                let title: String = saved.chapter.chars().take(CHAPTER_W).collect();
                buf.print(CHAPTER_X, y, &title, text, bg);
                buf.print(MODE_X, y, mode_name(saved.mode), text, bg);
                buf.print(ARMY_X, y, &army_text(saved.roster), text, bg);
                let time = playtime_text(saved.playtime_s);
                let width = i32::try_from(time.chars().count()).unwrap_or(0);
                buf.print(TIME_END_X - width, y, &time, text, bg);
            }
            Some(Slot::Unreadable(e)) => {
                buf.print(CHAPTER_X, y, &e.to_string(), bad, bg);
            }
            Some(Slot::Empty) | None => {
                buf.print(CHAPTER_X, y, EMPTY_SLOT, dim, bg);
            }
        }
    }

    /// Draws the overwrite question in a double-bordered box in the middle
    /// of the screen, with its answers' keys under it.
    fn draw_question(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let question = self.overwrite_question();
        let answers = self.help(ctx).replace(SEPARATOR, " / ");
        let widest = question.chars().count().max(answers.chars().count());
        let w = i32::try_from(widest).unwrap_or(0) + 4;
        let rect = Rect::new(
            (i32::from(buf.width()) - w) / 2,
            (i32::from(buf.height()) - OVERWRITE_H) / 2,
            w,
            OVERWRITE_H,
        );
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
        buf.print(rect.x + 2, rect.y + 2, &question, c(UiColor::Text), bg);
        buf.print(rect.x + 2, rect.y + 3, &answers, c(UiColor::TextDim), bg);
    }
}

/// A game mode's name.
pub fn mode_name(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Classic => "Classic",
        GameMode::Casual => "Casual",
    }
}

/// An army of `n` as text: `1 unit`, `3 units`.
pub fn army_text(n: usize) -> String {
    let units = if n == 1 { "unit" } else { "units" };
    format!("{n} {units}")
}

impl Screen for SlotPickerScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            let done = if self.asking {
                self.answer(ctx, action)
            } else {
                match action {
                    Action::CursorDown => {
                        self.step(ctx, true);
                        false
                    }
                    Action::CursorUp => {
                        self.step(ctx, false);
                        false
                    }
                    Action::Confirm => self.confirm(ctx),
                    Action::Cancel => {
                        ctx.audio.menu(MenuSound::Cancel);
                        true
                    }
                    _ => false,
                }
            };
            if done {
                return Transition::Pop;
            }
            if self.asking && action == Action::Confirm {
                // The key that opened the question isn't its answer.
                break;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let (black, bg) = (c(UiColor::Black), c(UiColor::PanelBg));
        let (text, dim) = (c(UiColor::Text), c(UiColor::TextDim));
        buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
        buf.fill_rect(PANEL, Cell::new(' ', text, bg));
        buf.draw_box(PANEL, BoxStyle::Single, c(UiColor::PanelBorder), bg);
        let title = if self.is_saving() {
            SAVE_TITLE
        } else {
            LOAD_TITLE
        };
        let highlight = c(UiColor::TextHighlight);
        buf.print(PANEL.x + 2, PANEL.y, &format!(" {title} "), highlight, bg);

        buf.print(SLOT_X, HEADING_ROW, "Slot", dim, bg);
        buf.print(CHAPTER_X, HEADING_ROW, "Chapter", dim, bg);
        buf.print(MODE_X, HEADING_ROW, "Mode", dim, bg);
        buf.print(ARMY_X, HEADING_ROW, "Army", dim, bg);
        buf.print(TIME_END_X - 4, HEADING_ROW, "Time", dim, bg);
        let shown = self.top..(self.top + VISIBLE).min(self.slots.len());
        // Which slots are on screen, in the bottom border.
        let range = format!(
            " {}-{} of {} ",
            shown.start + 1,
            shown.end,
            self.slots.len()
        );
        let range_w = i32::try_from(range.chars().count()).unwrap_or(0);
        let range_x = PANEL.x + PANEL.w - 2 - range_w;
        buf.print(range_x, PANEL.y + PANEL.h - 1, &range, dim, bg);
        for (y, i) in (FIRST_ROW..).zip(shown) {
            self.draw_slot(ctx, buf, i, y);
        }

        if let Some(message) = &self.message {
            print_centred(buf, MESSAGE_ROW, message, c(UiColor::HpLow), black);
        }
        if self.asking {
            self.draw_question(ctx, buf);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), dim, black);
    }
}

#[cfg(test)]
mod tests;
