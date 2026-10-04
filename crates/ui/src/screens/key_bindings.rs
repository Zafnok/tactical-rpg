//! The Key bindings screen (tickets 0815 and 0816; `docs/design/controls.md`,
//! *Rebinding keys* and *Rebinding buttons*): every rebindable action with
//! its [`SLOTS`] key slots for the layout in use, or, on its controller
//! [`Side`], its [`SLOTS`] button slots. A row at the top switches between
//! the two. Confirm on a slot captures the next key or button pressed into
//! it; one taken from another slot moves; leaving is blocked while a
//! required action has no key or no button; Restore defaults asks first.
//!
//! The screen edits a copy of the layout's [`LayoutBindings`] and of the
//! [`PadBindings`] and hands them to [`Ctx`] when it closes, so it is steered
//! with the keys and buttons the player had when they opened it: moving
//! every cursor key elsewhere can't trap them here.
//!
//! No controller button is fixed, so the screen works with only a
//! controller: a button goes in the slot when it is let go, holding any
//! button backs out of a capture, and when the controller was used last
//! Confirm on a slot offers [`CHOICES`] (change it or clear it).
//!
//! Capture is the one place that reads raw key presses
//! ([`FrameInput::pressed_chords`]) and raw button presses
//! ([`FrameInput::pressed_buttons`]); which keys back out or clear a slot is
//! asked of `input` ([`is_capture_abort`], [`is_clear_slot`]), and a
//! button's name of [`PadKind::button_name`], never named here (the
//! `keyboard-input` skill).

use super::{layout_picker, print_centred};
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::{
    Action, Button, CAPTURE_BUTTON_PROMPT, CAPTURE_PROMPT, Chord, Device, Keymap, Layout,
    LayoutBindings, PadBindings, PadKind, SLOTS, capture_abort_key_name, clear_slot_key_name,
    is_capture_abort, is_clear_slot,
};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{
    HelpKeys, NOT_MAPPED, SEPARATOR, cursor_keys_name, help_line, key_name,
};

/// Every rebindable action with its player-facing label, in the order the
/// screen lists them (Nick's pick, ticket 0815): the required actions, then
/// the optional ones, as in `controls.md`'s *Required and optional* table.
pub const ROWS: [(Action, &str); 16] = [
    (Action::CursorUp, "Cursor up"),
    (Action::CursorDown, "Cursor down"),
    (Action::CursorLeft, "Cursor left"),
    (Action::CursorRight, "Cursor right"),
    (Action::Confirm, "Confirm"),
    (Action::Cancel, "Cancel"),
    (Action::EndTurn, "End turn"),
    (Action::Select, "Select"),
    (Action::ConfirmEndTurn, "Confirm end turn"),
    (Action::PrevUnit, "Previous ready unit"),
    (Action::NextUnit, "Next ready unit"),
    (Action::Info, "Unit info"),
    (Action::DangerZone, "Danger zone"),
    (Action::ToggleAutoEnd, "Auto-end on/off"),
    (Action::Rewind, "Rewind"),
    (Action::Menu, "Map menu"),
];

/// Which bindings the screen shows and edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The keys of the layout in use.
    Keyboard,
    /// The controller's buttons (one setup for both layouts).
    Controller,
}

impl Side {
    /// What a slot on this side shows while it waits.
    fn prompt(self) -> &'static str {
        match self {
            Self::Keyboard => CAPTURE_PROMPT,
            Self::Controller => CAPTURE_BUTTON_PROMPT,
        }
    }
}

/// Screen title.
pub const TITLE: &str = "Key bindings";
/// Text key of the switch row's name for the keyboard side, which the
/// layout's name follows.
pub const KEYBOARD_SIDE: &str = "key_bindings.keyboard";
/// Text key of the switch row's name for the controller side.
pub const CONTROLLER_SIDE: &str = "key_bindings.controller";
/// Heading over the required actions, on the keyboard side.
pub const REQUIRED_HEADING: &str = "Must have a key";
/// Text key of the heading over the required actions, on the controller
/// side.
pub const REQUIRED_BUTTON_HEADING: &str = "key_bindings.must_have_button";
/// Text key of the heading over the optional actions.
pub const OPTIONAL_HEADING: &str = "key_bindings.optional";
/// The row under the actions that puts the shown side's defaults back.
pub const RESTORE_DEFAULTS: &str = "Restore defaults";
/// Help text while a slot waits for a key.
pub const CAPTURE_HELP: &str = "Press the key to put here";
/// Text key of the help for backing out of a capture with only a
/// controller.
pub const HOLD_TO_CANCEL: &str = "key_bindings.hold_to_cancel";
/// Text keys of what Confirm on a slot offers when the controller was used
/// last: put another key or button in it, or empty it.
pub const CHOICES: [&str; 2] = ["key_bindings.change", "key_bindings.clear"];
/// Shown when the key pressed for a slot is reserved.
pub const RESERVED_MESSAGE: &str = "That key can't be used";
/// How long the row that just lost its key to another slot stays
/// highlighted, in seconds (*tunable*).
pub const MOVED_FLASH_SECS: f32 = 1.5;
/// How long a controller button is held to back out of a capture, in
/// seconds (*tunable*). Let go sooner, it goes in the slot.
pub const HOLD_TO_CANCEL_SECS: f32 = 1.0;

/// The panel, in cells.
const PANEL: Rect = Rect::new(2, 1, 96, 27);
/// Column of the action labels.
const LABEL_X: i32 = PANEL.x + 4;
/// Column of the title and the group headings, two cells left of the
/// labels.
const HEADING_X: i32 = LABEL_X - 2;
/// Row of the keyboard / controller switch.
const SWITCH_Y: i32 = PANEL.y + 1;
/// Row of the slot columns' headings; the required actions' heading is on
/// the next row.
const COLUMNS_Y: i32 = PANEL.y + 3;
/// Column of the first slot's highlight bar; its text starts one cell in.
const SLOTS_X: i32 = LABEL_X + 23;
/// Cells a slot's key name may take (the longest chord name fits).
const SLOT_W: usize = 15;
/// Columns from one slot to the next.
const SLOT_PITCH: i32 = 18;
/// What an empty slot shows.
const EMPTY_SLOT: &str = "·";
/// Height of the restore question's box: its two lines, a blank row above
/// and below, and the border.
const QUESTION_H: i32 = 6;
/// The box of the [`CHOICES`], in cells: a line each and the border.
const CHOICE_W: i32 = 12;
const CHOICE_H: i32 = 4;
/// Row of the message under the panel.
const MESSAGE_ROW: i32 = PANEL.y + PANEL.h + 1;

/// [`KeyBindingsScreen::row`] on Restore defaults, the row after the
/// actions.
const RESTORE_ROW: usize = ROWS.len();
/// [`KeyBindingsScreen::row`] on the keyboard / controller switch: above
/// the first action, so one up from it and one down from Restore defaults.
const SWITCH_ROW: usize = ROWS.len() + 1;

/// The actions that move the focus.
const CURSOR_ACTIONS: [Action; 4] = [
    Action::CursorUp,
    Action::CursorDown,
    Action::CursorLeft,
    Action::CursorRight,
];

/// The player-facing name of `action` ([`ROWS`]); its keymap name for an
/// action not on the screen (Debug).
pub fn label(action: Action) -> &'static str {
    ROWS.iter()
        .find(|&&(a, _)| a == action)
        .map_or(action.name(), |&(_, label)| label)
}

/// The blocked-leave message for `action` on `side` (`Give Cancel a key
/// first`, `Give Cancel a button first`).
pub fn blocked_message(ctx: &Ctx, side: Side, action: Action) -> String {
    match side {
        Side::Keyboard => format!("Give {} a key first", label(action)),
        Side::Controller => {
            ctx.text_with("key_bindings.blocked_button", &[("action", &label(action))])
        }
    }
}

/// Left edge of slot `i`'s highlight bar.
fn slot_x(i: usize) -> i32 {
    SLOTS_X + i32::try_from(i).unwrap_or(0) * SLOT_PITCH
}

/// Console row of row `i` of [`ROWS`]: under the columns' and the required
/// actions' headings, with a blank row and the optional actions' heading
/// before the first optional one.
fn row_y(i: usize) -> i32 {
    let required = ROWS.iter().filter(|(a, _)| a.is_required()).count();
    let gap = if i >= required { 2 } else { 0 };
    COLUMNS_Y + 2 + i32::try_from(i).unwrap_or(0) + gap
}

/// The Key bindings screen for one layout and the controller. Cancel saves
/// and closes it, unless a required action has no key or no button.
#[derive(Debug, Clone)]
pub struct KeyBindingsScreen {
    /// The layout being edited.
    layout: Layout,
    /// The keymap the screen was opened with: what steers it (`Game` keeps
    /// using it until the edits are saved on leaving) and what its help
    /// text names.
    opened_with: Keymap,
    /// Which bindings are shown.
    side: Side,
    /// The edited copy of the layout's keys, saved on leaving.
    bindings: LayoutBindings,
    /// The edited copy of the controller's buttons, saved on leaving.
    pad: PadBindings,
    /// Focused row: an index into [`ROWS`], [`RESTORE_ROW`] or
    /// [`SWITCH_ROW`].
    row: usize,
    /// Focused slot (kept while on a row that has none).
    slot: usize,
    /// Whether the focused slot is waiting for a key or button.
    capturing: bool,
    /// The controller button held since it went down during a capture, and
    /// for how many seconds.
    held: Option<(Button, f32)>,
    /// The focused line of [`CHOICES`], while they are open.
    choice: Option<usize>,
    /// The message under the panel, if any.
    message: Option<String>,
    /// The action that just lost a key to another slot, and the seconds its
    /// row stays highlighted.
    moved: Option<(Action, f32)>,
    /// Whether the "restore the defaults?" question is open.
    asking_restore: bool,
    /// Set when a capture ends with a cursor key or button held: cursor
    /// moves are ignored until none is, because the one just pressed for
    /// the slot (or held to back out) may be one under the opened-with
    /// keymap, still down and repeating.
    await_release: bool,
}

impl KeyBindingsScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "key_bindings";

    /// The screen for the layout in use (right-handed if none is chosen
    /// yet), focused on the first action's first slot, showing the side the
    /// player pressed last: the controller's buttons after a button press,
    /// else the keys.
    pub fn new(ctx: &Ctx) -> Self {
        let layout = ctx.layout().unwrap_or(Layout::RightHanded);
        let side = match ctx.device {
            Device::Keyboard => Side::Keyboard,
            Device::Pad(_) => Side::Controller,
        };
        Self {
            layout,
            opened_with: ctx.keymap.clone(),
            side,
            bindings: ctx.layout_bindings(layout),
            pad: ctx.pad_bindings(),
            row: 0,
            slot: 0,
            capturing: false,
            held: None,
            choice: None,
            message: None,
            moved: None,
            asking_restore: false,
            await_release: false,
        }
    }

    /// The same screen editing `bindings` instead of the saved ones.
    #[cfg(test)]
    fn with_bindings(mut self, bindings: LayoutBindings) -> Self {
        self.bindings = bindings;
        self
    }

    /// The edited keys (saved when the screen closes).
    pub fn bindings(&self) -> &LayoutBindings {
        &self.bindings
    }

    /// The edited controller buttons (saved when the screen closes).
    pub fn pad_bindings(&self) -> &PadBindings {
        &self.pad
    }

    /// Which bindings are shown.
    pub fn side(&self) -> Side {
        self.side
    }

    /// The focused action and slot; `None` on Restore defaults and on the
    /// keyboard / controller switch.
    pub fn focus(&self) -> Option<(Action, usize)> {
        ROWS.get(self.row).map(|&(action, _)| (action, self.slot))
    }

    /// Whether the focus is on the keyboard / controller switch.
    pub fn is_on_switch(&self) -> bool {
        self.row == SWITCH_ROW
    }

    /// Whether the focused slot is waiting for a key or button.
    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    /// The focused line of [`CHOICES`], while they are open.
    pub fn choice(&self) -> Option<usize> {
        self.choice
    }

    /// Whether the screen is asking before it restores the defaults.
    pub fn is_asking_restore(&self) -> bool {
        self.asking_restore
    }

    /// The question asked before restoring the shown side's defaults.
    pub fn restore_question(&self, ctx: &Ctx) -> String {
        match self.side {
            Side::Keyboard => format!(
                "Restore the default keys for {}?",
                layout_picker::label(self.layout)
            ),
            Side::Controller => ctx.text("key_bindings.restore_buttons").to_owned(),
        }
    }

    /// The message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The names in `action`'s slots on the shown side; buttons as the
    /// controller used last names them (an Xbox pad's names until one is
    /// used).
    fn slot_names(&self, ctx: &Ctx, action: Action) -> [Option<String>; SLOTS] {
        match self.side {
            Side::Keyboard => self
                .bindings
                .slots(action)
                .map(|c| c.map(|c| c.to_string())),
            Side::Controller => {
                let kind = match ctx.device {
                    Device::Pad(kind) => kind,
                    Device::Keyboard => PadKind::default(),
                };
                let name = |b| kind.button_name(b).to_owned();
                self.pad.slots(action).map(|b| b.map(name))
            }
        }
    }

    /// Whether `action` has nothing in any slot on the shown side.
    fn is_unmapped(&self, action: Action) -> bool {
        match self.side {
            Side::Keyboard => self.bindings.is_unmapped(action),
            Side::Controller => self.pad.is_unmapped(action),
        }
    }

    /// The first row of the screen whose required action has nothing on
    /// `side`.
    fn first_unmapped(&self, side: Side) -> Option<Action> {
        let unmapped = match side {
            Side::Keyboard => self.bindings.unmapped_required(),
            Side::Controller => self.pad.unmapped_required(),
        };
        let mut rows = ROWS.iter().map(|&(action, _)| action);
        rows.find(|action| unmapped.contains(action))
    }

    /// Counts the moved-row highlight down.
    fn tick(&mut self, dt: f32) {
        if let Some((_, left)) = &mut self.moved {
            *left -= dt;
            // A NaN frame time ends the highlight too.
            if left.is_nan() || *left <= 0.0 {
                self.moved = None;
            }
        }
    }

    /// Moves the focus one row up or down, wrapping: the actions, Restore
    /// defaults, the switch, and the first action again.
    fn move_row(&mut self, ctx: &mut Ctx, down: bool) {
        let rows = ROWS.len() + 2;
        self.row = if down {
            (self.row + 1) % rows
        } else {
            (self.row + rows - 1) % rows
        };
        self.message = None;
        ctx.audio.menu(MenuSound::Move);
    }

    /// Left or right: on the switch, shows the keyboard (left) or the
    /// controller (right); on a slot, moves the focus one slot, stopping at
    /// the ends. Does nothing on Restore defaults.
    fn move_across(&mut self, ctx: &mut Ctx, right: bool) {
        if self.is_on_switch() {
            let side = if right {
                Side::Controller
            } else {
                Side::Keyboard
            };
            self.show(ctx, side);
            return;
        }
        if self.focus().is_none() {
            return;
        }
        let to = if right {
            (self.slot + 1).min(SLOTS - 1)
        } else {
            self.slot.saturating_sub(1)
        };
        if to != self.slot {
            self.slot = to;
            self.message = None;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Shows `side`, if it isn't shown already.
    fn show(&mut self, ctx: &mut Ctx, side: Side) {
        if side != self.side {
            self.side = side;
            self.message = None;
            self.moved = None;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Confirm: on a slot, starts capturing (after the controller was used,
    /// opens the [`CHOICES`] instead); on Restore defaults, asks whether to
    /// restore; on the switch, shows the other side.
    fn confirm(&mut self, ctx: &mut Ctx) {
        self.message = None;
        if self.is_on_switch() {
            let other = match self.side {
                Side::Keyboard => Side::Controller,
                Side::Controller => Side::Keyboard,
            };
            self.show(ctx, other);
            return;
        }
        if self.focus().is_none() {
            self.asking_restore = true;
        } else if matches!(ctx.device, Device::Pad(_)) {
            self.choice = Some(0);
        } else {
            self.capturing = true;
        }
        ctx.audio.menu(MenuSound::Select);
    }

    /// An action while the [`CHOICES`] are open: up and down move between
    /// them, Confirm takes the focused one (capture, or empty the slot) and
    /// Cancel closes them. Returns whether they closed.
    fn answer_choice(&mut self, ctx: &mut Ctx, action: Action) -> bool {
        let Some(line) = self.choice else {
            return true;
        };
        match action {
            Action::CursorUp | Action::CursorDown => {
                self.choice = Some((line + 1) % CHOICES.len());
                ctx.audio.menu(MenuSound::Move);
                return false;
            }
            Action::Confirm if line == 0 => {
                self.capturing = true;
                ctx.audio.menu(MenuSound::Select);
            }
            Action::Confirm => {
                if !self.clear_slot(ctx) {
                    ctx.audio.menu(MenuSound::Select);
                }
            }
            Action::Cancel => ctx.audio.menu(MenuSound::Cancel),
            _ => return false,
        }
        self.choice = None;
        true
    }

    /// An action while the restore question is open: Confirm restores the
    /// shown side's defaults, Cancel backs out; either closes the question
    /// (and returns `true`). Nothing else does anything.
    fn answer_restore(&mut self, ctx: &mut Ctx, action: Action) -> bool {
        match action {
            Action::Confirm => {
                let def = &ctx.content.keymap;
                match self.side {
                    Side::Keyboard => self.bindings = LayoutBindings::defaults(def, self.layout),
                    Side::Controller => self.pad = PadBindings::defaults(def),
                }
                self.moved = None;
                ctx.audio.menu(MenuSound::Select);
            }
            Action::Cancel => ctx.audio.menu(MenuSound::Cancel),
            _ => return false,
        }
        self.asking_restore = false;
        true
    }

    /// Empties the focused slot on the shown side. Returns whether it held
    /// anything.
    fn clear_slot(&mut self, ctx: &mut Ctx) -> bool {
        let Some((action, slot)) = self.focus() else {
            return false;
        };
        if self.slot_names(ctx, action)[slot].is_none() {
            return false;
        }
        match self.side {
            Side::Keyboard => self.bindings.clear(action, slot),
            Side::Controller => self.pad.clear(action, slot),
        }
        self.message = None;
        ctx.audio.menu(MenuSound::Select);
        true
    }

    /// Ends the capture with nothing changed.
    fn abort_capture(&mut self, ctx: &mut Ctx) {
        self.end_capture();
        ctx.audio.menu(MenuSound::Cancel);
    }

    /// Ends the capture, clearing what belongs to it.
    fn end_capture(&mut self) {
        self.capturing = false;
        self.held = None;
        self.message = None;
    }

    /// The capture took something that was in slot `from`, if any: ends it
    /// and lights up the action that lost it.
    fn captured(&mut self, ctx: &mut Ctx, from: Option<(Action, usize)>) {
        self.end_capture();
        self.moved = from.map(|(lost, _)| (lost, MOVED_FLASH_SECS));
        ctx.audio.menu(MenuSound::Select);
    }

    /// A key pressed while capturing. Returns whether capturing ended.
    fn capture(&mut self, ctx: &mut Ctx, chord: Chord) -> bool {
        if is_clear_slot(chord) {
            return false;
        }
        if is_capture_abort(chord) {
            self.abort_capture(ctx);
            return true;
        }
        let Some((action, slot)) = self.focus() else {
            self.end_capture();
            return true;
        };
        if self.side == Side::Controller {
            // A slot waiting for a button takes no key.
            return false;
        }
        let Ok(from) = self.bindings.bind(action, slot, chord) else {
            self.message = Some(RESERVED_MESSAGE.to_owned());
            ctx.audio.menu(MenuSound::Denied);
            return false;
        };
        self.captured(ctx, from);
        true
    }

    /// This frame's controller buttons while capturing: the first button
    /// to go down is watched. Let go before [`HOLD_TO_CANCEL_SECS`], it
    /// goes in a slot that waits for a button; held that long, it backs out
    /// of the capture with nothing changed.
    fn capture_buttons(&mut self, ctx: &mut Ctx, input: &FrameInput) {
        let fresh = self.held.is_none();
        if fresh {
            self.held = input.pressed_buttons().first().map(|&b| (b, 0.0));
        }
        let Some((button, held_secs)) = self.held else {
            return;
        };
        if input.released_buttons().contains(&button) {
            self.held = None;
            self.tapped(ctx, button);
            return;
        }
        // The frame it went down in doesn't count; nor does a NaN frame.
        let secs = if fresh {
            0.0
        } else {
            held_secs + input.dt.max(0.0)
        };
        self.held = Some((button, secs));
        if secs >= HOLD_TO_CANCEL_SECS {
            self.abort_capture(ctx);
        }
    }

    /// `button` was pressed and let go while capturing: on the controller
    /// side it goes in the focused slot.
    fn tapped(&mut self, ctx: &mut Ctx, button: Button) {
        let (Side::Controller, Some((action, slot))) = (self.side, self.focus()) else {
            return;
        };
        // The focused action is rebindable and its slot exists.
        if let Ok(from) = self.pad.bind(action, slot, button) {
            self.captured(ctx, from);
        }
    }

    /// Cancel: saves and returns `true` if every required action has a key
    /// and a button; otherwise shows the side that lacks one and names the
    /// first action on the screen that does (the shown side first).
    fn leave(&mut self, ctx: &mut Ctx) -> bool {
        let other = match self.side {
            Side::Keyboard => Side::Controller,
            Side::Controller => Side::Keyboard,
        };
        let blocked = [self.side, other]
            .into_iter()
            .find_map(|side| Some((side, self.first_unmapped(side)?)));
        if let Some((side, action)) = blocked {
            self.side = side;
            self.message = Some(blocked_message(ctx, side, action));
            ctx.audio.menu(MenuSound::Denied);
            return false;
        }
        // If saving fails the keys and buttons still apply for this session.
        if self.bindings != ctx.layout_bindings(self.layout) {
            let _ = ctx.set_layout_bindings(self.layout, self.bindings.clone());
        }
        if self.pad != ctx.pad_bindings() {
            let _ = ctx.set_pad_bindings(self.pad.clone());
        }
        ctx.audio.menu(MenuSound::Cancel);
        true
    }

    /// The bottom help line, naming the keys the screen was opened with
    /// (or their buttons, on a controller): the same as `ctx`'s until the
    /// screen closes.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = HelpKeys::new(&self.opened_with, ctx.device);
        let on_pad = matches!(ctx.device, Device::Pad(_));
        if self.capturing {
            let what = match self.side {
                Side::Keyboard => CAPTURE_HELP,
                Side::Controller => CAPTURE_BUTTON_PROMPT,
            };
            let hold = on_pad || self.side == Side::Controller;
            let hold = hold.then(|| ctx.text(HOLD_TO_CANCEL));
            let back = (!on_pad).then(|| help_line(&[(Some(capture_abort_key_name()), "back")]));
            let hints = [Some(what), hold, back.as_deref()];
            return hints
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(SEPARATOR);
        }
        let confirm = || Some(key_name(km, Action::Confirm));
        let cancel = || Some(key_name(km, Action::Cancel));
        if self.asking_restore {
            return help_line(&[(confirm(), "yes"), (cancel(), "no")]);
        }
        if self.choice.is_some() {
            return ctx.text_with("key_bindings.help_choice", &[]);
        }
        if self.is_on_switch() {
            return ctx.text_with("key_bindings.help_switch", &[]);
        }
        let on_slot = self.focus().is_some();
        if on_slot && on_pad {
            return ctx.text_with("key_bindings.help_slot_pad", &[]);
        }
        help_line(&[
            (Some(cursor_keys_name(km)), "move"),
            (confirm(), if on_slot { "bind" } else { "restore" }),
            (on_slot.then(clear_slot_key_name), "clear"),
            (cancel(), "back"),
        ])
    }

    /// Draws the restore question in a double-bordered box in the middle
    /// of the screen, with its answers' keys under it (the look of the
    /// battle's end-turn question).
    fn draw_restore_question(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let km = HelpKeys::new(&self.opened_with, ctx.device);
        let question = self.restore_question(ctx);
        let answers = format!(
            "{} yes / {} no",
            key_name(km, Action::Confirm),
            key_name(km, Action::Cancel)
        );
        let widest = question.chars().count().max(answers.chars().count());
        let w = i32::try_from(widest).unwrap_or(0) + 4;
        let rect = Rect::new(
            (i32::from(buf.width()) - w) / 2,
            (i32::from(buf.height()) - QUESTION_H) / 2,
            w,
            QUESTION_H,
        );
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
        buf.print(rect.x + 2, rect.y + 2, &question, c(UiColor::Text), bg);
        buf.print(rect.x + 2, rect.y + 3, &answers, c(UiColor::TextDim), bg);
    }

    /// Draws the [`CHOICES`] in a double-bordered box under the focused
    /// slot (over it on the last rows, where it wouldn't fit), the focused
    /// line as a bar.
    fn draw_choices(&self, ctx: &Ctx, buf: &mut GlyphBuffer, line: usize) {
        let c = |u| ctx.palette.get(u);
        let (bg, bar) = (c(UiColor::PanelBg), c(UiColor::PanelBorderFocus));
        let slot_y = row_y(self.row);
        let below = slot_y + 1;
        let y = if below + CHOICE_H < PANEL.y + PANEL.h {
            below
        } else {
            slot_y - CHOICE_H
        };
        let rect = Rect::new(slot_x(self.slot), y, CHOICE_W, CHOICE_H);
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Double, bar, bg);
        let inner = usize::try_from(CHOICE_W - 3).unwrap_or(0);
        for (i, choice) in (0..).zip(CHOICES.map(|key| ctx.text(key))) {
            let (x, y) = (rect.x + 1, rect.y + 1 + i);
            if usize::try_from(i).is_ok_and(|i| i == line) {
                buf.print(x, y, &format!(" {choice:<inner$}"), bg, bar);
            } else {
                buf.print(x + 1, y, choice, c(UiColor::Text), bg);
            }
        }
    }

    /// Draws the keyboard / controller switch: the shown side highlighted
    /// (as a bar while the focus is on the switch), the other dim.
    fn draw_switch(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let (bg, bar) = (c(UiColor::PanelBg), c(UiColor::PanelBorderFocus));
        let keyboard = format!(
            "{}{SEPARATOR}{}",
            ctx.text(KEYBOARD_SIDE),
            layout_picker::label(self.layout)
        );
        let mut x = LABEL_X;
        for (side, name) in [
            (Side::Keyboard, keyboard.as_str()),
            (Side::Controller, ctx.text(CONTROLLER_SIDE)),
        ] {
            if side != self.side {
                buf.print(x, SWITCH_Y, name, c(UiColor::TextDim), bg);
            } else if self.is_on_switch() {
                buf.print(x - 1, SWITCH_Y, &format!(" {name} "), bg, bar);
            } else {
                buf.print(x, SWITCH_Y, name, c(UiColor::TextHighlight), bg);
            }
            x += i32::try_from(name.chars().count()).unwrap_or(0) + 4;
        }
    }

    /// Draws row `i` of [`ROWS`].
    fn draw_row(&self, ctx: &Ctx, buf: &mut GlyphBuffer, i: usize) {
        let c = |u| ctx.palette.get(u);
        let (bg, text, dim) = (c(UiColor::PanelBg), c(UiColor::Text), c(UiColor::TextDim));
        let Some(&(action, label)) = ROWS.get(i) else {
            return;
        };
        let y = row_y(i);
        let focused = self.row == i;
        let label_fg = if focused {
            c(UiColor::TextHighlight)
        } else if self.moved.is_some_and(|(lost, _)| lost == action) {
            c(UiColor::White)
        } else {
            text
        };
        let label_w = buf.print(LABEL_X, y, label, label_fg, bg);
        let fixed: Vec<String> = match self.side {
            Side::Keyboard => Keymap::fixed_chords_for(action)
                .iter()
                .map(ToString::to_string)
                .collect(),
            Side::Controller => Vec::new(),
        };
        if !fixed.is_empty() {
            let x = LABEL_X + i32::from(label_w) + 2;
            buf.print(x, y, &format!("+ {}", fixed.join("/")), dim, bg);
        }
        for (j, name) in self.slot_names(ctx, action).iter().enumerate() {
            let x = slot_x(j);
            let filled = name.as_deref();
            if focused && self.slot == j {
                let shown = if self.capturing {
                    self.side.prompt()
                } else {
                    filled.unwrap_or("")
                };
                let bar = format!(" {shown:<SLOT_W$}");
                buf.print(x, y, &bar, bg, c(UiColor::PanelBorderFocus));
            } else if let Some(name) = filled {
                buf.print(x + 1, y, name, text, bg);
            } else {
                buf.print(x + 1, y, EMPTY_SLOT, dim, bg);
            }
        }
        if self.is_unmapped(action) {
            let fg = if action.is_required() {
                c(UiColor::HpLow)
            } else {
                dim
            };
            buf.print(slot_x(SLOTS) + 1, y, NOT_MAPPED, fg, bg);
        }
    }
}

impl Screen for KeyBindingsScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        self.tick(input.dt);
        if self.capturing {
            // This frame's actions and repeats are ignored: the key or
            // button is for the slot.
            if !input.pressed_chords().iter().any(|&c| self.capture(ctx, c)) {
                self.capture_buttons(ctx, input);
            }
            if !self.capturing {
                self.await_release = CURSOR_ACTIONS.iter().any(|&a| input.is_held(a));
            }
            return Transition::None;
        }
        if self.asking_restore {
            for &action in &input.actions {
                if self.answer_restore(ctx, action) {
                    break;
                }
            }
            return Transition::None;
        }
        if self.choice.is_some() {
            for &action in &input.actions {
                if self.answer_choice(ctx, action) {
                    break;
                }
            }
            return Transition::None;
        }
        if input.pressed_chords().iter().any(|&c| is_clear_slot(c)) {
            self.clear_slot(ctx);
        }
        self.await_release &= CURSOR_ACTIONS.iter().any(|&a| input.is_held(a));
        for &action in &input.actions {
            match action {
                cursor if self.await_release && CURSOR_ACTIONS.contains(&cursor) => {}
                Action::CursorUp => self.move_row(ctx, false),
                Action::CursorDown => self.move_row(ctx, true),
                Action::CursorLeft => self.move_across(ctx, false),
                Action::CursorRight => self.move_across(ctx, true),
                Action::Confirm => {
                    self.confirm(ctx);
                    if self.capturing || self.asking_restore || self.choice.is_some() {
                        // The key that confirmed isn't the key to bind (or
                        // the answer), and nothing after it this frame
                        // counts.
                        break;
                    }
                }
                Action::Cancel if self.leave(ctx) => return Transition::Pop,
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let (black, bg) = (c(UiColor::Black), c(UiColor::PanelBg));
        let (text, dim) = (c(UiColor::Text), c(UiColor::TextDim));
        let bar = c(UiColor::PanelBorderFocus);
        buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
        buf.fill_rect(PANEL, Cell::new(' ', text, bg));
        buf.draw_box(PANEL, BoxStyle::Single, c(UiColor::PanelBorder), bg);
        let title = format!(" {TITLE} ");
        buf.print(HEADING_X, PANEL.y, &title, c(UiColor::TextHighlight), bg);
        self.draw_switch(ctx, buf);

        for j in 0..SLOTS {
            let heading = match self.side {
                Side::Keyboard => format!("Key {}", j + 1),
                Side::Controller => ctx.text_with("key_bindings.button_column", &[("n", &(j + 1))]),
            };
            buf.print(slot_x(j) + 1, COLUMNS_Y, &heading, dim, bg);
        }
        let mut group = None;
        for (i, &(action, _)) in ROWS.iter().enumerate() {
            let required = action.is_required();
            if group != Some(required) {
                let heading = match (required, self.side) {
                    (true, Side::Keyboard) => REQUIRED_HEADING,
                    (true, Side::Controller) => ctx.text(REQUIRED_BUTTON_HEADING),
                    (false, _) => ctx.text(OPTIONAL_HEADING),
                };
                buf.print(HEADING_X, row_y(i) - 1, heading, bar, bg);
                group = Some(required);
            }
            self.draw_row(ctx, buf, i);
        }
        let y = row_y(ROWS.len()) + 1;
        if self.row == RESTORE_ROW {
            buf.print(LABEL_X - 1, y, &format!(" {RESTORE_DEFAULTS} "), bg, bar);
        } else {
            buf.print(LABEL_X, y, RESTORE_DEFAULTS, text, bg);
        }

        if let Some(message) = &self.message {
            print_centred(buf, MESSAGE_ROW, message, c(UiColor::HpLow), black);
        }
        if self.asking_restore {
            self.draw_restore_question(ctx, buf);
        }
        if let Some(line) = self.choice {
            self.draw_choices(ctx, buf, line);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), dim, black);
    }
}

#[cfg(test)]
mod tests;
