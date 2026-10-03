//! The Options screen (ticket 0805), opened from the title and the map
//! menu: the player's [`Settings`] one per row, changed with the cursor's
//! left and right keys and saved at once; `Layout` opens the layout picker
//! ([`LayoutPickerScreen::change`]) and `Key bindings` the
//! [`KeyBindingsScreen`]; with a campaign loaded, `Game mode` shows its
//! mode and switches Classic to Casual after a confirm, never back
//! (`docs/design/death-and-difficulty.md`); `Reset tips` shows every
//! one-time tip again; `Restore defaults` puts every setting back, after a
//! confirm (custom keys are reset on the Key bindings screen, and the
//! layout stays).

use trpg_core::GameMode;

use super::{KeyBindingsScreen, LayoutPickerScreen, layout_picker, print_centred};
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::map_view::CursorStyle;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::settings::{AnimSpeed, EnemyPhaseSpeed, MAX_VOLUME, Settings, TextSpeed};
use crate::tips::reset_tips;

/// Text key ([`Ctx::text`]) of the screen title.
pub const TITLE: &str = "options.title";
/// Text key of the message under the panel once the tips are reset.
pub const TIPS_RESET_MESSAGE: &str = "options.message.tips_reset";
/// Text key of the message once the defaults are back.
pub const RESTORED_MESSAGE: &str = "options.message.restored";
/// Text key of the message once the campaign is Casual.
pub const CASUAL_MESSAGE: &str = "options.message.casual";
/// Text key of the message if something couldn't be saved.
pub const NOT_SAVED_MESSAGE: &str = "options.message.not_saved";

/// The panel, in cells.
const PANEL: Rect = Rect::new(20, 3, 60, 20);
/// Column of the row labels.
const LABEL_X: i32 = PANEL.x + 4;
/// Column of the row values.
const VALUE_X: i32 = PANEL.x + 30;
/// Row of the message under the panel.
const MESSAGE_ROW: i32 = PANEL.y + PANEL.h + 1;
/// Height of a question's box: its two lines, a blank row above and below,
/// and the border.
const QUESTION_H: i32 = 6;

/// One row of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// [`Settings::text_speed`].
    TextSpeed,
    /// [`Settings::anim_speed`].
    AnimSpeed,
    /// [`Settings::combat_animations`].
    CombatAnimations,
    /// [`Settings::enemy_phase_speed`].
    EnemyPhaseSpeed,
    /// [`Settings::auto_end_turn`].
    AutoEnd,
    /// [`Settings::fullscreen`].
    Fullscreen,
    /// [`Settings::cursor_style`].
    Cursor,
    /// [`Settings::music_volume`].
    MusicVolume,
    /// [`Settings::sound_volume`].
    SoundVolume,
    /// The right- or left-handed layout: opens the layout picker.
    Layout,
    /// Opens the Key bindings screen.
    KeyBindings,
    /// The campaign's mode (only with a campaign loaded).
    GameMode,
    /// Shows every one-time tip again.
    ResetTips,
    /// Puts every setting back to its default.
    RestoreDefaults,
}

impl Row {
    /// Every row, top to bottom.
    pub const ALL: [Row; 14] = [
        Row::TextSpeed,
        Row::AnimSpeed,
        Row::CombatAnimations,
        Row::EnemyPhaseSpeed,
        Row::AutoEnd,
        Row::Fullscreen,
        Row::Cursor,
        Row::MusicVolume,
        Row::SoundVolume,
        Row::Layout,
        Row::KeyBindings,
        Row::GameMode,
        Row::ResetTips,
        Row::RestoreDefaults,
    ];

    /// The text key ([`Ctx::text`]) of its label.
    pub const fn key(self) -> &'static str {
        match self {
            Row::TextSpeed => "options.row.text_speed",
            Row::AnimSpeed => "options.row.anim_speed",
            Row::CombatAnimations => "options.row.combat_animations",
            Row::EnemyPhaseSpeed => "options.row.enemy_phase_speed",
            Row::AutoEnd => "options.row.auto_end",
            Row::Fullscreen => "options.row.fullscreen",
            Row::Cursor => "options.row.cursor",
            Row::MusicVolume => "options.row.music_volume",
            Row::SoundVolume => "options.row.sound_volume",
            Row::Layout => "options.row.layout",
            Row::KeyBindings => "options.row.key_bindings",
            Row::GameMode => "options.row.game_mode",
            Row::ResetTips => "options.row.reset_tips",
            Row::RestoreDefaults => "options.row.restore_defaults",
        }
    }

    /// Whether the cursor's left and right keys change its value.
    pub const fn is_setting(self) -> bool {
        !matches!(
            self,
            Row::Layout | Row::KeyBindings | Row::GameMode | Row::ResetTips | Row::RestoreDefaults
        )
    }

    /// Whether a blank row comes before it (the first row of a group).
    const fn starts_group(self) -> bool {
        matches!(self, Row::Layout | Row::ResetTips)
    }
}

/// The text key of a cursor style's name (`docs/design/look-and-feel.md`).
pub const fn cursor_key(style: CursorStyle) -> &'static str {
    match style {
        CursorStyle::Corners => "options.cursor.corners",
        CursorStyle::LargeCorners => "options.cursor.large_corners",
        CursorStyle::TileGlow => "options.cursor.tile_glow",
    }
}

/// Every cursor style, in the order the row steps through them.
const CURSOR_STYLES: [CursorStyle; 3] = [
    CursorStyle::Corners,
    CursorStyle::LargeCorners,
    CursorStyle::TileGlow,
];

/// The text key of a game mode's name.
const fn mode_key(mode: GameMode) -> &'static str {
    match mode {
        GameMode::Classic => "options.mode.classic",
        GameMode::Casual => "options.mode.casual",
    }
}

/// The text key of an on/off value.
const fn on_off_key(on: bool) -> &'static str {
    if on {
        "options.value.on"
    } else {
        "options.value.off"
    }
}

/// A volume as a bar of [`MAX_VOLUME`] cells and its number.
fn volume_text(volume: u8) -> String {
    let full = usize::from(volume.min(MAX_VOLUME));
    let empty = usize::from(MAX_VOLUME) - full;
    format!("{}{} {volume:>2}", "█".repeat(full), "░".repeat(empty))
}

/// `all`'s value one step after (`forward`) or before `current`, stopping
/// at the ends (`wrap`: going round instead).
fn stepped<T: Copy + PartialEq>(all: &[T], current: T, forward: bool, wrap: bool) -> T {
    let n = all.len();
    let i = all.iter().position(|&v| v == current).unwrap_or(0);
    let to = match (forward, wrap) {
        (true, true) => (i + 1) % n,
        (true, false) => (i + 1).min(n - 1),
        (false, true) => (i + n - 1) % n,
        (false, false) => i.saturating_sub(1),
    };
    all.get(to).copied().unwrap_or(current)
}

/// A volume one step up or down, within 0 to [`MAX_VOLUME`] (`wrap`: past
/// the loudest comes silence).
fn stepped_volume(volume: u8, forward: bool, wrap: bool) -> u8 {
    match (forward, wrap) {
        (true, true) if volume >= MAX_VOLUME => 0,
        (true, _) => volume.saturating_add(1).min(MAX_VOLUME),
        (false, _) => volume.saturating_sub(1),
    }
}

/// A yes/no question the screen asks before something that can't be taken
/// back with one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Question {
    /// Classic → Casual.
    SwitchToCasual,
    /// Every setting back to its default.
    RestoreDefaults,
}

impl Question {
    /// The text key of the question.
    pub const fn key(self) -> &'static str {
        match self {
            Question::SwitchToCasual => "options.question.casual",
            Question::RestoreDefaults => "options.question.restore",
        }
    }
}

/// The Options screen. Cancel closes it.
#[derive(Debug, Clone)]
pub struct OptionsScreen {
    /// The focused row.
    row: Row,
    /// The question open, if any.
    asking: Option<Question>,
    /// The text key of the message under the panel, if any.
    message: Option<&'static str>,
}

impl OptionsScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "options";

    /// The screen with its first row focused.
    pub fn new() -> Self {
        Self {
            row: Row::TextSpeed,
            asking: None,
            message: None,
        }
    }

    /// The rows shown: every one, `Game mode` only with a campaign loaded.
    pub fn rows(ctx: &Ctx) -> Vec<Row> {
        let campaign = ctx.campaign_mode.is_some();
        Row::ALL
            .into_iter()
            .filter(|&r| r != Row::GameMode || campaign)
            .collect()
    }

    /// The focused row.
    pub fn focus(&self) -> Row {
        self.row
    }

    /// The question open, if any.
    pub fn asking(&self) -> Option<Question> {
        self.asking
    }

    /// The text key of the message under the panel, if any.
    pub fn message(&self) -> Option<&str> {
        self.message
    }

    /// What `row` shows as its value: the setting, the layout in use, the
    /// campaign's mode; nothing for the rows that only do something.
    pub fn value(ctx: &Ctx, row: Row) -> String {
        let s = ctx.settings();
        let key = match row {
            Row::TextSpeed => s.text_speed.key(),
            Row::AnimSpeed => s.anim_speed.key(),
            Row::CombatAnimations => on_off_key(s.combat_animations),
            Row::EnemyPhaseSpeed => s.enemy_phase_speed.key(),
            Row::AutoEnd => on_off_key(s.auto_end_turn),
            Row::Fullscreen => on_off_key(s.fullscreen),
            Row::Cursor => cursor_key(s.cursor_style),
            Row::MusicVolume => return volume_text(s.music_volume),
            Row::SoundVolume => return volume_text(s.sound_volume),
            Row::Layout => {
                let layout = ctx.layout().map(layout_picker::label);
                return layout.unwrap_or_default().to_owned();
            }
            Row::GameMode => match ctx.campaign_mode {
                Some(mode) => mode_key(mode),
                None => return String::new(),
            },
            Row::KeyBindings | Row::ResetTips | Row::RestoreDefaults => return String::new(),
        };
        ctx.text(key).to_owned()
    }

    /// Moves the focus one row up or down, wrapping.
    fn move_row(&mut self, ctx: &mut Ctx, down: bool) {
        self.row = stepped(&Self::rows(ctx), self.row, down, true);
        self.message = None;
        ctx.audio.menu(MenuSound::Move);
    }

    /// Steps the focused setting one value on (`forward`) or back; `wrap`
    /// goes round at the ends. Saves it and plays the move sound if it
    /// changed (at the new volume, for the sound volume).
    fn change(&mut self, ctx: &mut Ctx, forward: bool, wrap: bool) {
        let row = self.row;
        let before = ctx.settings().clone();
        let saved = ctx.change_settings(|s| step_setting(s, row, forward, wrap));
        if *ctx.settings() == before {
            return;
        }
        self.message = saved.is_err().then_some(NOT_SAVED_MESSAGE);
        ctx.audio.menu(MenuSound::Move);
    }

    /// Confirm on the focused row.
    fn confirm(&mut self, ctx: &mut Ctx) -> Transition {
        self.message = None;
        match self.row {
            Row::Layout => {
                ctx.audio.menu(MenuSound::Select);
                return Transition::Push(Box::new(LayoutPickerScreen::change(ctx)));
            }
            Row::KeyBindings => {
                ctx.audio.menu(MenuSound::Select);
                return Transition::Push(Box::new(KeyBindingsScreen::new(ctx)));
            }
            Row::GameMode if ctx.campaign_mode == Some(GameMode::Classic) => {
                self.asking = Some(Question::SwitchToCasual);
                ctx.audio.menu(MenuSound::Select);
            }
            // Casual never goes back to Classic.
            Row::GameMode => ctx.audio.menu(MenuSound::Denied),
            Row::ResetTips => {
                let reset = reset_tips(&mut *ctx.storage);
                self.message = Some(if reset.is_ok() {
                    TIPS_RESET_MESSAGE
                } else {
                    NOT_SAVED_MESSAGE
                });
                ctx.audio.menu(MenuSound::Select);
            }
            Row::RestoreDefaults => {
                self.asking = Some(Question::RestoreDefaults);
                ctx.audio.menu(MenuSound::Select);
            }
            _ => self.change(ctx, true, true),
        }
        Transition::None
    }

    /// An action while `question` is open: Confirm does it, Cancel backs
    /// out; either closes the question (and returns `true`). Nothing else
    /// does anything.
    fn answer(&mut self, ctx: &mut Ctx, question: Question, action: Action) -> bool {
        match action {
            Action::Confirm => {
                self.message = Some(match question {
                    Question::SwitchToCasual => {
                        ctx.switch_to_casual();
                        CASUAL_MESSAGE
                    }
                    Question::RestoreDefaults => {
                        let saved = ctx.change_settings(|s| *s = s.restored());
                        if saved.is_ok() {
                            RESTORED_MESSAGE
                        } else {
                            NOT_SAVED_MESSAGE
                        }
                    }
                });
                ctx.audio.menu(MenuSound::Select);
            }
            Action::Cancel => ctx.audio.menu(MenuSound::Cancel),
            _ => return false,
        }
        self.asking = None;
        true
    }

    /// The text key of the help line for the focused row: what Confirm
    /// does there, if anything.
    fn help_key(&self, ctx: &Ctx) -> &'static str {
        if self.asking.is_some() {
            return "options.help.question";
        }
        match self.row {
            Row::Layout | Row::KeyBindings => "options.help.open",
            Row::GameMode if ctx.campaign_mode == Some(GameMode::Classic) => "options.help.switch",
            Row::GameMode => "options.help.none",
            Row::ResetTips => "options.help.reset",
            Row::RestoreDefaults => "options.help.restore",
            _ => "options.help.change",
        }
    }

    /// The bottom help line, naming the keys of the active keymap (or
    /// their buttons, on a controller).
    pub fn help(&self, ctx: &Ctx) -> String {
        ctx.text_with(self.help_key(ctx), &[])
    }

    /// Draws `question` in a double-bordered box in the middle of the
    /// screen, with its answers' keys under it (the look of the Key
    /// bindings screen's question).
    fn draw_question(ctx: &Ctx, buf: &mut GlyphBuffer, question: Question) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let answers = ctx.text_with("options.question.answers", &[]);
        let text = ctx.text(question.key());
        let widest = text.chars().count().max(answers.chars().count());
        let w = i32::try_from(widest).unwrap_or(0) + 4;
        let rect = Rect::new(
            (i32::from(buf.width()) - w) / 2,
            (i32::from(buf.height()) - QUESTION_H) / 2,
            w,
            QUESTION_H,
        );
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorderFocus), bg);
        buf.print(rect.x + 2, rect.y + 2, text, c(UiColor::Text), bg);
        buf.print(rect.x + 2, rect.y + 3, &answers, c(UiColor::TextDim), bg);
    }
}

/// Steps the setting `row` shows; other rows have none.
fn step_setting(s: &mut Settings, row: Row, forward: bool, wrap: bool) {
    match row {
        Row::TextSpeed => s.text_speed = stepped(&TextSpeed::ALL, s.text_speed, forward, wrap),
        Row::AnimSpeed => s.anim_speed = stepped(&AnimSpeed::ALL, s.anim_speed, forward, wrap),
        Row::EnemyPhaseSpeed => {
            s.enemy_phase_speed =
                stepped(&EnemyPhaseSpeed::ALL, s.enemy_phase_speed, forward, wrap);
        }
        // On/off rows read `Off  On` left to right.
        Row::CombatAnimations => {
            s.combat_animations = stepped(&[false, true], s.combat_animations, forward, wrap);
        }
        Row::AutoEnd => s.auto_end_turn = stepped(&[false, true], s.auto_end_turn, forward, wrap),
        Row::Fullscreen => s.fullscreen = stepped(&[false, true], s.fullscreen, forward, wrap),
        Row::Cursor => s.cursor_style = stepped(&CURSOR_STYLES, s.cursor_style, forward, wrap),
        Row::MusicVolume => s.music_volume = stepped_volume(s.music_volume, forward, wrap),
        Row::SoundVolume => s.sound_volume = stepped_volume(s.sound_volume, forward, wrap),
        Row::Layout | Row::KeyBindings | Row::GameMode | Row::ResetTips | Row::RestoreDefaults => {}
    }
}

impl Default for OptionsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for OptionsScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            if let Some(question) = self.asking {
                // The answer ends this frame's keys.
                if self.answer(ctx, question, action) {
                    break;
                }
                continue;
            }
            match action {
                Action::CursorUp => self.move_row(ctx, false),
                Action::CursorDown => self.move_row(ctx, true),
                Action::CursorLeft => self.change(ctx, false, false),
                Action::CursorRight => self.change(ctx, true, false),
                Action::Confirm => {
                    let transition = self.confirm(ctx);
                    if !matches!(transition, Transition::None) {
                        return transition;
                    }
                    if self.asking.is_some() {
                        // The key that asked isn't the answer.
                        break;
                    }
                }
                Action::Cancel => {
                    ctx.audio.menu(MenuSound::Cancel);
                    return Transition::Pop;
                }
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
        let title = format!(" {} ", ctx.text(TITLE));
        buf.print(PANEL.x + 2, PANEL.y, &title, c(UiColor::TextHighlight), bg);

        let mut y = PANEL.y + 2;
        for row in Self::rows(ctx) {
            if row.starts_group() {
                y += 1;
            }
            let focused = row == self.row;
            let label = ctx.text(row.key());
            if focused {
                buf.print(LABEL_X - 1, y, &format!(" {label} "), bg, bar);
            } else {
                buf.print(LABEL_X, y, label, text, bg);
            }
            let value = Self::value(ctx, row);
            let value_fg = if focused {
                c(UiColor::TextHighlight)
            } else {
                text
            };
            buf.print(VALUE_X, y, &value, value_fg, bg);
            if focused && row.is_setting() {
                buf.print(VALUE_X - 2, y, "◄", dim, bg);
                let after = VALUE_X + i32::try_from(value.chars().count()).unwrap_or(0) + 1;
                buf.print(after, y, "►", dim, bg);
            }
            y += 1;
        }

        if let Some(message) = self.message {
            let message = ctx.text(message);
            print_centred(buf, MESSAGE_ROW, message, c(UiColor::TextHighlight), black);
        }
        if let Some(question) = self.asking {
            Self::draw_question(ctx, buf, question);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), dim, black);
    }
}

#[cfg(test)]
mod tests;
