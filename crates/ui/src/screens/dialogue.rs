//! The dialogue screen (ticket 0704, ADR-0018): plays a [`DialoguePlayer`]
//! with two portraits, name plates and a typewriter text box. Full-screen,
//! or as an overlay that leaves the battle map visible behind it.
//!
//! Layout (100×32): 32×16-cell portraits in 34×18 frames at `x = 1` and
//! `x = 65` from row 1, name plates on row 19, the text box on rows 21–27.
//! While the lead's replies are up, the text box grows upward to list them
//! under the line being answered (Nick, 0708: like Stardew Valley).

use trpg_content::{Names, Scene, Side};
use trpg_core::{LEAD_ID, LeadProfile};

use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::dialogue::{DialoguePlayer, Portrait, View};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::portrait::draw_portrait;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::screens::print_centred;
use crate::widgets::help::{SEPARATOR, help_line, key_name};
use crate::widgets::menu::{Menu, MenuEvent, MenuItem};
use crate::widgets::word_wrap;

/// Top row of the portrait frames.
const FRAME_Y: i32 = 1;
/// Portrait frame size: a 32×16-cell portrait plus its border.
const FRAME: (i32, i32) = (34, 18);
/// Left column of the left and right frames.
const LEFT_X: i32 = 1;
/// Left column of the right frame.
const RIGHT_X: i32 = 65;
/// Row of the name plates.
const PLATE_Y: i32 = 19;
/// The text box.
const TEXT_BOX: Rect = Rect::new(0, 21, 100, 7);
/// Left column of the text.
const TEXT_X: i32 = 4;
/// First text row.
const TEXT_Y: i32 = 23;
/// Characters per text line: the box less a 3-cell margin each side.
pub const TEXT_W: usize = 92;
/// Text lines per box; longer text pages.
pub const TEXT_LINES: usize = 3;
/// How much the listener (and both portraits during narration) is dimmed.
pub const LISTENER_DIM: f32 = 0.45;
/// Reveal speed multiplier while Confirm is held.
pub const FAST_FORWARD: f32 = 6.0;
/// The `▼` is shown for this long, then hidden for as long, while waiting.
const BLINK_S: f32 = 0.5;

/// Plays a scene: Confirm reveals the text at once, or moves on when it's
/// all shown; holding Confirm reveals faster. The End turn key does
/// everything Confirm does. Cancel asks whether to skip
/// the scene. At a reply choice, the cursor keys move through the lead's
/// replies and Confirm picks one; skipping stops at each choice. Pops when
/// the scene ends or is skipped.
#[derive(Debug, Clone)]
pub struct DialogueScreen {
    player: DialoguePlayer,
    overlay: bool,
    /// The current text box, word-wrapped.
    lines: Vec<String>,
    /// Which page of `lines` is shown ([`TEXT_LINES`] lines each).
    page: usize,
    /// Characters of the page revealed so far (fractional between frames).
    shown: f32,
    /// Seconds since the page was fully revealed, for the blinking `▼`.
    waiting: f32,
    /// Whether the "Skip scene?" question is open.
    asking_skip: bool,
    /// The lead's replies, while a choice is open.
    menu: Option<Menu>,
}

impl DialogueScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "dialogue";

    /// `scene` full-screen, with `lead`'s name and pronouns and the names
    /// from `names`.
    pub fn new(scene: Scene, lead: LeadProfile, names: Names) -> Self {
        let mut screen = Self {
            player: DialoguePlayer::new(scene, lead, names),
            overlay: false,
            lines: Vec::new(),
            page: 0,
            shown: 0.0,
            waiting: 0.0,
            asking_skip: false,
            menu: None,
        };
        screen.start_box();
        screen
    }

    /// `scene` drawn over the screen below (the battle map).
    pub fn overlay(scene: Scene, lead: LeadProfile, names: Names) -> Self {
        Self {
            overlay: true,
            ..Self::new(scene, lead, names)
        }
    }

    /// The scene being played.
    pub fn player(&self) -> &DialoguePlayer {
        &self.player
    }

    /// The lines of the page on screen, in full.
    pub fn page_lines(&self) -> &[String] {
        let start = (self.page * TEXT_LINES).min(self.lines.len());
        let end = (start + TEXT_LINES).min(self.lines.len());
        &self.lines[start..end]
    }

    /// Whether the whole page is revealed.
    pub fn is_revealed(&self) -> bool {
        self.shown >= page_len(self.page_lines())
    }

    /// Whether the "Skip scene?" question is open.
    pub fn is_asking_skip(&self) -> bool {
        self.asking_skip
    }

    /// The replies menu, while a choice is open.
    pub fn menu(&self) -> Option<&Menu> {
        self.menu.as_ref()
    }

    /// Wraps the player's current text box and starts on its first page.
    /// At a choice, the line being answered stays up, fully shown (its last
    /// page), under the replies menu.
    fn start_box(&mut self) {
        let view = self.player.current();
        let lines = word_wrap(view.text.as_deref().unwrap_or(""), TEXT_W);
        let menu = view.choices.map(|choices| {
            Menu::new(choices.into_iter().map(MenuItem::new).collect()).without_cancel()
        });
        self.lines = lines;
        self.menu = menu;
        self.page = 0;
        self.start_page();
        if self.menu.is_some() {
            self.page = self.lines.len().saturating_sub(1) / TEXT_LINES;
            self.shown = page_len(self.page_lines());
        }
    }

    fn start_page(&mut self) {
        self.shown = 0.0;
        self.waiting = 0.0;
    }

    /// Confirm with everything shown: the next page, else the next box.
    /// Returns `true` once the scene is over.
    fn next(&mut self) -> bool {
        if (self.page + 1) * TEXT_LINES < self.lines.len() {
            self.page += 1;
            self.start_page();
        } else {
            self.player.advance();
            self.start_box();
        }
        self.player.is_finished()
    }
}

/// The actions that advance the dialogue: Confirm, and End turn, so either
/// key reads through a scene.
const ADVANCE_KEYS: [Action; 2] = [Action::Confirm, Action::EndTurn];

/// `action`, with End Turn treated as Confirm.
fn advance_key(action: Action) -> Action {
    if ADVANCE_KEYS.contains(&action) {
        Action::Confirm
    } else {
        action
    }
}

/// Characters on a page.
fn page_len(lines: &[String]) -> f32 {
    let n: usize = lines.iter().map(|l| l.chars().count()).sum();
    // Pages are at most 3 × 92 characters, so this is exact.
    #[expect(clippy::cast_precision_loss, reason = "at most a few hundred")]
    let n = n as f32;
    n
}

impl Screen for DialogueScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if self.player.is_finished() {
            return Transition::Pop;
        }
        for &action in &input.actions {
            let action = advance_key(action);
            if let Some(menu) = self.menu.as_mut() {
                if let Some(MenuEvent::Chosen(i)) = menu.handle_with_sound(action, &mut ctx.audio) {
                    self.player.choose(i);
                    self.start_box();
                    if self.player.is_finished() {
                        return Transition::Pop;
                    }
                }
                continue;
            }
            // Only the skip prompt sounds; reading on plays nothing.
            match (self.asking_skip, action) {
                (true, Action::Confirm) => {
                    ctx.audio.menu(MenuSound::Select);
                    self.asking_skip = false;
                    self.player.skip_to_choice();
                    if self.player.is_finished() {
                        return Transition::Pop;
                    }
                    self.start_box();
                    // The rest of this frame's keys can't pick a reply.
                    break;
                }
                (true, Action::Cancel) => {
                    ctx.audio.menu(MenuSound::Cancel);
                    self.asking_skip = false;
                }
                // Opening the skip prompt sounds like opening a menu.
                (false, Action::Cancel) => {
                    ctx.audio.menu(MenuSound::Select);
                    self.asking_skip = true;
                }
                (false, Action::Confirm) if !self.is_revealed() => {
                    self.shown = page_len(self.page_lines());
                }
                (false, Action::Confirm) if self.next() => return Transition::Pop,
                // A choice just opened: the rest of this frame's keys
                // can't pick a reply.
                (false, Action::Confirm) if self.menu.is_some() => break,
                _ => {}
            }
        }
        if !self.asking_skip {
            if self.is_revealed() {
                self.waiting += input.dt;
            } else {
                let fast = if ADVANCE_KEYS.iter().any(|&a| input.is_held(a)) {
                    FAST_FORWARD
                } else {
                    1.0
                };
                let len = page_len(self.page_lines());
                self.shown = (self.shown + input.dt * ctx.text_speed * fast).min(len);
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let blank = Cell::new(' ', c(UiColor::Text), c(UiColor::Black));
        if self.overlay {
            // Below the text box is the battle's key help, whose keys do
            // nothing while the scene plays.
            // (Clipped to the buffer.)
            let below = TEXT_BOX.y + TEXT_BOX.h;
            let rest = Rect::new(0, below, i32::from(buf.width()), i32::from(buf.height()));
            buf.fill_rect(rest, blank);
        } else {
            buf.fill_rect(buf.bounds(), blank);
        }
        let view = self.player.current();
        if let Some(caption) = &view.caption {
            let text = format!(" {caption} ");
            print_centred(buf, 0, &text, c(UiColor::TextHighlight), c(UiColor::Black));
        }
        for (side, portrait) in [(Side::Left, view.left), (Side::Right, view.right)] {
            if let Some(portrait) = portrait {
                draw_side(
                    ctx,
                    buf,
                    self.player.lead(),
                    side,
                    portrait,
                    view.speaker == Some(side),
                );
            }
        }
        self.draw_text_box(ctx, buf, &view);
    }

    fn is_overlay(&self) -> bool {
        self.overlay
    }
}

/// A character's display name: the name the player gave the lead, else
/// the character entry's (their id if they have none).
fn display_name<'a>(ctx: &'a Ctx, lead: &'a LeadProfile, portrait: Portrait<'a>) -> &'a str {
    if portrait.character.0 == LEAD_ID {
        return &lead.name;
    }
    ctx.content
        .characters
        .characters
        .get(portrait.character)
        .map_or(portrait.character.0.as_str(), |c| c.name.as_str())
}

/// One side's frame, portrait (the lead's by gender) and name plate.
fn draw_side(
    ctx: &Ctx,
    buf: &mut GlyphBuffer,
    lead: &LeadProfile,
    side: Side,
    portrait: Portrait,
    speaking: bool,
) {
    let c = |u| ctx.palette.get(u);
    let bg = c(UiColor::PanelBg);
    let x = match side {
        Side::Left => LEFT_X,
        Side::Right => RIGHT_X,
    };
    let frame = Rect::new(x, FRAME_Y, FRAME.0, FRAME.1);
    buf.fill_rect(frame, Cell::new(' ', c(UiColor::Text), bg));
    let (style, border, dim, name_fg) = if speaking {
        (
            BoxStyle::Double,
            UiColor::PanelBorderFocus,
            0.0,
            UiColor::Text,
        )
    } else {
        (
            BoxStyle::Single,
            UiColor::PanelBorder,
            LISTENER_DIM,
            UiColor::TextDim,
        )
    };
    buf.draw_box(frame, style, c(border), bg);
    let art_id = lead.portrait_for(&portrait.character.0);
    if let Some(art) = ctx.content.portraits.get(art_id) {
        let mirror = side == Side::Right;
        draw_portrait(
            buf,
            &ctx.palette,
            (x + 1, FRAME_Y + 1),
            art,
            portrait.expression,
            dim,
            mirror,
        );
    }
    let plate = Rect::new(x, PLATE_Y, FRAME.0, 1);
    buf.fill_rect(plate, Cell::new(' ', c(UiColor::Text), bg));
    let name = display_name(ctx, lead, portrait);
    let w = i32::try_from(name.chars().count()).unwrap_or(0);
    buf.print(x + (FRAME.0 - w) / 2, PLATE_Y, name, c(name_fg), bg);
}

impl DialogueScreen {
    /// The text box and its first text row. While the replies are up it
    /// grows upward (its bottom stays put) to fit the line being answered,
    /// a blank row and one row per reply.
    pub fn text_box(&self) -> (Rect, i32) {
        let Some(menu) = &self.menu else {
            return (TEXT_BOX, TEXT_Y);
        };
        let question = self.page_lines().len();
        let gap = usize::from(question > 0);
        let rows = i32::try_from(question + gap + menu.items().len()).unwrap_or(0);
        // The last content row is the one the ▼ uses, free during a choice.
        let bottom = TEXT_BOX.y + TEXT_BOX.h - 1;
        let text_y = (bottom - rows).min(TEXT_Y);
        let top = text_y - (TEXT_Y - TEXT_BOX.y);
        (
            Rect::new(TEXT_BOX.x, top, TEXT_BOX.w, bottom + 1 - top),
            text_y,
        )
    }

    /// The text box: speaker name on the border, the revealed text (or the
    /// skip question), the replies while a choice is open, and the
    /// blinking `▼` once the page is shown.
    fn draw_text_box(&self, ctx: &Ctx, buf: &mut GlyphBuffer, view: &View) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let km = &ctx.keymap;
        let (text_box, text_y) = self.text_box();
        buf.fill_rect(text_box, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(text_box, BoxStyle::Single, c(UiColor::PanelBorder), bg);
        let speaker = match view.speaker {
            Some(Side::Left) => view.left,
            Some(Side::Right) => view.right,
            None => None,
        };
        if let Some(speaker) = speaker {
            let name = format!(" {} ", display_name(ctx, self.player.lead(), speaker));
            buf.print(
                text_box.x + 3,
                text_box.y,
                &name,
                c(UiColor::TextHighlight),
                bg,
            );
        }

        if self.asking_skip {
            let yes_no = help_line(&[
                (Some(key_name(km, Action::Confirm)), "yes"),
                (Some(key_name(km, Action::Cancel)), "no"),
            ])
            .replace(SEPARATOR, " / ");
            buf.print(TEXT_X, TEXT_Y, "Skip scene?", c(UiColor::Text), bg);
            buf.print(TEXT_X, TEXT_Y + 1, &yes_no, c(UiColor::TextDim), bg);
            return;
        }

        let lines = self.page_lines();
        let (fg, centred) = if view.narration {
            (c(UiColor::TextDim), true)
        } else {
            (c(UiColor::Text), false)
        };
        // Narration is centred vertically too: one line goes in the middle
        // of the three rows (two or three fill them from the top). Not
        // above replies, which follow right after.
        let top = text_y + i32::from(centred && lines.len() == 1 && self.menu.is_none());
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "shown is between 0 and the page length"
        )]
        let mut budget = self.shown as usize;
        for (y, line) in (top..).zip(lines) {
            let len = line.chars().count();
            let x = if centred {
                TEXT_BOX.x + (TEXT_BOX.w - i32::try_from(len).unwrap_or(0)) / 2
            } else {
                TEXT_X
            };
            let visible: String = line.chars().take(budget).collect();
            buf.print(x, y, &visible, fg, bg);
            budget = budget.saturating_sub(len);
        }

        if let Some(menu) = &self.menu {
            let gap = i32::from(!lines.is_empty());
            let first = text_y + i32::try_from(lines.len()).unwrap_or(0) + gap;
            for (y, (i, item)) in (first..).zip(menu.items().iter().enumerate()) {
                let (marker, fg) = if i == menu.focus() {
                    ("> ", c(UiColor::TextHighlight))
                } else {
                    ("  ", c(UiColor::Text))
                };
                buf.print(TEXT_X, y, &format!("{marker}{}", item.label), fg, bg);
            }
        }

        if self.is_revealed() && self.menu.is_none() {
            let right = TEXT_BOX.x + TEXT_BOX.w - 4;
            let bottom = TEXT_BOX.y + TEXT_BOX.h - 2;
            let key = key_name(km, Action::Confirm);
            let w = i32::try_from(key.chars().count()).unwrap_or(0);
            buf.print(right - 1 - w, bottom, &key, c(UiColor::TextDim), bg);
            if self.waiting % (2.0 * BLINK_S) < BLINK_S {
                buf.print(right, bottom, "▼", c(UiColor::TextHighlight), bg);
            }
        }
    }
}

#[cfg(test)]
mod tests;
