//! The Key bindings screen (ticket 0815; `docs/design/controls.md`,
//! *Rebinding keys*): every rebindable action with its [`SLOTS`] key slots
//! for the layout in use. Confirm on a slot captures the next key pressed
//! into it; a key taken from another slot moves; the clear-slot key empties
//! a slot; leaving is blocked while a required action has no key.
//!
//! The screen edits a copy of the layout's [`LayoutBindings`] and hands it
//! to [`Ctx::set_layout_bindings`] when it closes, so it is steered with the
//! keys the player had when they opened it: moving every cursor key
//! elsewhere can't trap them here.
//!
//! Capture is the one place that reads raw key presses
//! ([`FrameInput::pressed_chords`]); which keys back out or clear a slot is
//! asked of `input` ([`is_capture_abort`], [`is_clear_slot`]), never named
//! here (the `keyboard-input` skill).

use super::{layout_picker, print_centred};
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::{
    Action, CAPTURE_PROMPT, Chord, Keymap, Layout, LayoutBindings, SLOTS, capture_abort_key_name,
    clear_slot_key_name, is_capture_abort, is_clear_slot,
};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{NOT_MAPPED, SEPARATOR, cursor_keys_name, help_line, key_name};

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

/// Screen title, followed by the layout's name.
pub const TITLE: &str = "Key bindings";
/// Heading over the required actions.
pub const REQUIRED_HEADING: &str = "Must have a key";
/// Heading over the optional actions.
pub const OPTIONAL_HEADING: &str = "Optional";
/// The row under the actions that puts the layout's default keys back.
pub const RESTORE_DEFAULTS: &str = "Restore defaults";
/// Help text while a slot waits for a key.
pub const CAPTURE_HELP: &str = "Press the key to put here";
/// Shown when the key pressed for a slot is reserved.
pub const RESERVED_MESSAGE: &str = "That key can't be used";
/// How long the row that just lost its key to another slot stays
/// highlighted, in seconds (*tunable*).
pub const MOVED_FLASH_SECS: f32 = 1.5;

/// The panel, in cells.
const PANEL: Rect = Rect::new(2, 1, 96, 27);
/// Column of the group headings.
const HEADING_X: i32 = PANEL.x + 2;
/// Column of the action labels.
const LABEL_X: i32 = PANEL.x + 4;
/// Column of the first slot's highlight bar; its text starts one cell in.
const SLOTS_X: i32 = LABEL_X + 23;
/// Cells a slot's key name may take (the longest chord name fits).
const SLOT_W: usize = 15;
/// Columns from one slot to the next.
const SLOT_PITCH: i32 = 18;
/// What an empty slot shows.
const EMPTY_SLOT: &str = "·";
/// Row of the message under the panel.
const MESSAGE_ROW: i32 = PANEL.y + PANEL.h + 1;

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

/// The blocked-leave message for `action` (`Give Cancel a key first`).
pub fn blocked_message(action: Action) -> String {
    format!("Give {} a key first", label(action))
}

/// Left edge of slot `i`'s highlight bar.
fn slot_x(i: usize) -> i32 {
    SLOTS_X + i32::try_from(i).unwrap_or(0) * SLOT_PITCH
}

/// The Key bindings screen for one layout. Cancel saves and closes it,
/// unless a required action has no key.
#[derive(Debug, Clone)]
pub struct KeyBindingsScreen {
    /// The layout being edited.
    layout: Layout,
    /// The keymap the screen was opened with: what steers it (`Game` keeps
    /// using it until the edits are saved on leaving) and what its help
    /// text names.
    opened_with: Keymap,
    /// The edited copy, saved on leaving.
    bindings: LayoutBindings,
    /// Focused row: an index into [`ROWS`], or `ROWS.len()` for Restore
    /// defaults.
    row: usize,
    /// Focused slot (kept while on Restore defaults, which has none).
    slot: usize,
    /// Whether the focused slot is waiting for a key.
    capturing: bool,
    /// The message under the panel, if any.
    message: Option<String>,
    /// The action that just lost a key to another slot, and the seconds its
    /// row stays highlighted.
    moved: Option<(Action, f32)>,
    /// Set when a capture ends: cursor moves are ignored until no cursor
    /// key is held, because the key just pressed for the slot may be one
    /// (under the opened-with keymap), still down and repeating.
    await_release: bool,
}

impl KeyBindingsScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "key_bindings";

    /// The screen for the layout in use (right-handed if none is chosen
    /// yet), focused on the first action's first slot.
    pub fn new(ctx: &Ctx) -> Self {
        let layout = ctx.layout().unwrap_or(Layout::RightHanded);
        Self {
            layout,
            opened_with: ctx.keymap.clone(),
            bindings: ctx.layout_bindings(layout),
            row: 0,
            slot: 0,
            capturing: false,
            message: None,
            moved: None,
            await_release: false,
        }
    }

    /// The same screen editing `bindings` instead of the saved ones.
    #[cfg(test)]
    fn with_bindings(mut self, bindings: LayoutBindings) -> Self {
        self.bindings = bindings;
        self
    }

    /// The edited bindings (saved when the screen closes).
    pub fn bindings(&self) -> &LayoutBindings {
        &self.bindings
    }

    /// The focused action and slot; `None` on Restore defaults.
    pub fn focus(&self) -> Option<(Action, usize)> {
        ROWS.get(self.row).map(|&(action, _)| (action, self.slot))
    }

    /// Whether the focused slot is waiting for a key.
    pub fn is_capturing(&self) -> bool {
        self.capturing
    }

    /// The message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
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

    /// Moves the focus one row up or down, wrapping.
    fn move_row(&mut self, ctx: &mut Ctx, down: bool) {
        let rows = ROWS.len() + 1;
        self.row = if down {
            (self.row + 1) % rows
        } else {
            (self.row + rows - 1) % rows
        };
        self.message = None;
        ctx.audio.menu(MenuSound::Move);
    }

    /// Moves the focus one slot left or right, stopping at the ends. Does
    /// nothing on Restore defaults, which has no slots.
    fn move_slot(&mut self, ctx: &mut Ctx, right: bool) {
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

    /// Confirm: starts capturing on a slot, or restores the layout's
    /// default keys on Restore defaults.
    fn confirm(&mut self, ctx: &mut Ctx) {
        self.message = None;
        if self.focus().is_some() {
            self.capturing = true;
        } else {
            self.bindings = LayoutBindings::defaults(&ctx.content.keymap, self.layout);
            self.moved = None;
        }
        ctx.audio.menu(MenuSound::Select);
    }

    /// The clear-slot key: empties the focused slot.
    fn clear_slot(&mut self, ctx: &mut Ctx) {
        let Some((action, slot)) = self.focus() else {
            return;
        };
        if self.bindings.slots(action)[slot].is_some() {
            self.bindings.clear(action, slot);
            self.message = None;
            ctx.audio.menu(MenuSound::Select);
        }
    }

    /// A key pressed while capturing. Returns whether capturing ended.
    fn capture(&mut self, ctx: &mut Ctx, chord: Chord) -> bool {
        if is_clear_slot(chord) {
            return false;
        }
        self.await_release = true;
        if is_capture_abort(chord) {
            self.capturing = false;
            self.message = None;
            ctx.audio.menu(MenuSound::Cancel);
            return true;
        }
        let Some((action, slot)) = self.focus() else {
            self.capturing = false;
            return true;
        };
        let Ok(from) = self.bindings.bind(action, slot, chord) else {
            self.message = Some(RESERVED_MESSAGE.to_owned());
            ctx.audio.menu(MenuSound::Denied);
            return false;
        };
        self.capturing = false;
        self.message = None;
        self.moved = from.map(|(lost, _)| (lost, MOVED_FLASH_SECS));
        ctx.audio.menu(MenuSound::Select);
        true
    }

    /// Cancel: saves and returns `true` if every required action has a
    /// key; otherwise names the first one on the screen that doesn't.
    fn leave(&mut self, ctx: &mut Ctx) -> bool {
        let unmapped = self.bindings.unmapped_required();
        let first = ROWS
            .iter()
            .map(|&(action, _)| action)
            .find(|action| unmapped.contains(action));
        if let Some(action) = first {
            self.message = Some(blocked_message(action));
            ctx.audio.menu(MenuSound::Denied);
            return false;
        }
        if self.bindings != ctx.layout_bindings(self.layout) {
            // If saving fails the keys still apply for this session.
            let _ = ctx.set_layout_bindings(self.layout, self.bindings.clone());
        }
        ctx.audio.menu(MenuSound::Cancel);
        true
    }

    /// The bottom help line, naming the keys the screen was opened with.
    pub fn help(&self) -> String {
        let km = &self.opened_with;
        if self.capturing {
            let back = help_line(&[(Some(capture_abort_key_name()), "back")]);
            return format!("{CAPTURE_HELP}{SEPARATOR}{back}");
        }
        let on_slot = self.focus().is_some();
        help_line(&[
            (Some(cursor_keys_name(km)), "move"),
            (
                Some(key_name(km, Action::Confirm)),
                if on_slot { "bind" } else { "restore" },
            ),
            (on_slot.then(clear_slot_key_name), "clear"),
            (Some(key_name(km, Action::Cancel)), "back"),
        ])
    }

    /// Draws row `i` of [`ROWS`] on console row `y`.
    fn draw_row(&self, ctx: &Ctx, buf: &mut GlyphBuffer, i: usize, y: i32) {
        let c = |u| ctx.palette.get(u);
        let (bg, text, dim) = (c(UiColor::PanelBg), c(UiColor::Text), c(UiColor::TextDim));
        let Some(&(action, label)) = ROWS.get(i) else {
            return;
        };
        let focused = self.row == i;
        let label_fg = if focused {
            c(UiColor::TextHighlight)
        } else if self.moved.is_some_and(|(lost, _)| lost == action) {
            c(UiColor::White)
        } else {
            text
        };
        let label_w = buf.print(LABEL_X, y, label, label_fg, bg);
        let fixed: Vec<String> = Keymap::fixed_chords_for(action)
            .iter()
            .map(ToString::to_string)
            .collect();
        if !fixed.is_empty() {
            let x = LABEL_X + i32::from(label_w) + 2;
            buf.print(x, y, &format!("+ {}", fixed.join("/")), dim, bg);
        }
        for (j, chord) in self.bindings.slots(action).iter().enumerate() {
            let x = slot_x(j);
            let name = chord.map_or_else(String::new, |c| c.to_string());
            if focused && self.slot == j {
                let shown = if self.capturing {
                    CAPTURE_PROMPT
                } else {
                    &name
                };
                let bar = format!(" {shown:<SLOT_W$}");
                buf.print(x, y, &bar, bg, c(UiColor::PanelBorderFocus));
            } else if chord.is_some() {
                buf.print(x + 1, y, &name, text, bg);
            } else {
                buf.print(x + 1, y, EMPTY_SLOT, dim, bg);
            }
        }
        if self.bindings.is_unmapped(action) {
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
            // This frame's actions and repeats are ignored: the key is
            // for the slot.
            for &chord in &input.pressed_chords {
                if self.capture(ctx, chord) {
                    break;
                }
            }
            return Transition::None;
        }
        if input.pressed_chords.iter().any(|&c| is_clear_slot(c)) {
            self.clear_slot(ctx);
        }
        self.await_release &= CURSOR_ACTIONS.iter().any(|&a| input.is_held(a));
        for &action in &input.actions {
            match action {
                cursor if self.await_release && CURSOR_ACTIONS.contains(&cursor) => {}
                Action::CursorUp => self.move_row(ctx, false),
                Action::CursorDown => self.move_row(ctx, true),
                Action::CursorLeft => self.move_slot(ctx, false),
                Action::CursorRight => self.move_slot(ctx, true),
                Action::Confirm => {
                    self.confirm(ctx);
                    if self.capturing {
                        // The key that confirmed isn't the key to bind,
                        // and nothing after it this frame counts.
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
        let title = format!(" {TITLE}{SEPARATOR}{} ", layout_picker::label(self.layout));
        buf.print(PANEL.x + 2, PANEL.y, &title, c(UiColor::TextHighlight), bg);

        let mut y = PANEL.y + 2;
        for j in 0..SLOTS {
            buf.print(slot_x(j) + 1, y, &format!("Key {}", j + 1), dim, bg);
        }
        y += 1;
        let mut group = None;
        for (i, &(action, _)) in ROWS.iter().enumerate() {
            let required = action.is_required();
            if group != Some(required) {
                if group.is_some() {
                    y += 1;
                }
                let heading = if required {
                    REQUIRED_HEADING
                } else {
                    OPTIONAL_HEADING
                };
                buf.print(HEADING_X, y, heading, bar, bg);
                y += 1;
                group = Some(required);
            }
            self.draw_row(ctx, buf, i, y);
            y += 1;
        }
        y += 1;
        if self.focus().is_none() {
            buf.print(LABEL_X - 1, y, &format!(" {RESTORE_DEFAULTS} "), bg, bar);
        } else {
            buf.print(LABEL_X, y, RESTORE_DEFAULTS, text, bg);
        }

        if let Some(message) = &self.message {
            print_centred(buf, MESSAGE_ROW, message, c(UiColor::HpLow), black);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(), dim, black);
    }
}

#[cfg(test)]
mod tests;
