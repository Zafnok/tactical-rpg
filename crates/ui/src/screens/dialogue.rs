//! The dialogue screen (ticket 0704, ADR-0018): plays a [`DialoguePlayer`]
//! with two portraits, name plates and a typewriter text box. Full-screen,
//! or as an overlay that leaves the battle map visible behind it.
//!
//! Layout (100×32): 32×16-cell portraits in 34×18 frames at `x = 1` and
//! `x = 65` from row 1, name plates on row 19, the text box on rows 21–27.

use trpg_content::{Scene, Side};

use crate::color::UiColor;
use crate::dialogue::{DialoguePlayer, Portrait, View};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::portrait::draw_portrait;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::screens::print_centred;
use crate::widgets::help::{SEPARATOR, help_line, key_name};
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
/// all shown; holding Confirm reveals faster; Cancel asks whether to skip
/// the scene. Pops when the scene ends or is skipped.
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
}

impl DialogueScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "dialogue";

    /// `scene` full-screen.
    pub fn new(scene: Scene) -> Self {
        let mut screen = Self {
            player: DialoguePlayer::new(scene),
            overlay: false,
            lines: Vec::new(),
            page: 0,
            shown: 0.0,
            waiting: 0.0,
            asking_skip: false,
        };
        screen.start_box();
        screen
    }

    /// `scene` drawn over the screen below (the battle map).
    pub fn overlay(scene: Scene) -> Self {
        Self {
            overlay: true,
            ..Self::new(scene)
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

    /// Wraps the player's current text box and starts on its first page.
    fn start_box(&mut self) {
        self.lines = word_wrap(self.player.current().text.unwrap_or(""), TEXT_W);
        self.page = 0;
        self.start_page();
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
            match (self.asking_skip, action) {
                (true, Action::Confirm) => return Transition::Pop,
                (true, Action::Cancel) => self.asking_skip = false,
                (false, Action::Cancel) => self.asking_skip = true,
                (false, Action::Confirm) if !self.is_revealed() => {
                    self.shown = page_len(self.page_lines());
                }
                (false, Action::Confirm) if self.next() => return Transition::Pop,
                _ => {}
            }
        }
        if !self.asking_skip {
            if self.is_revealed() {
                self.waiting += input.dt;
            } else {
                let fast = if input.is_held(Action::Confirm) {
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
        if let Some(caption) = view.caption {
            let text = format!(" {caption} ");
            print_centred(buf, 0, &text, c(UiColor::TextHighlight), c(UiColor::Black));
        }
        for (side, portrait) in [(Side::Left, view.left), (Side::Right, view.right)] {
            if let Some(portrait) = portrait {
                draw_side(ctx, buf, side, portrait, view.speaker == Some(side));
            }
        }
        self.draw_text_box(ctx, buf, &view);
    }

    fn is_overlay(&self) -> bool {
        self.overlay
    }
}

/// A character's display name (their id if they have no character entry).
fn display_name<'a>(ctx: &'a Ctx, portrait: Portrait<'a>) -> &'a str {
    ctx.content
        .characters
        .characters
        .get(portrait.character)
        .map_or(portrait.character.0.as_str(), |c| c.name.as_str())
}

/// One side's frame, portrait and name plate.
fn draw_side(ctx: &Ctx, buf: &mut GlyphBuffer, side: Side, portrait: Portrait, speaking: bool) {
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
    if let Some(art) = ctx.content.portraits.get(portrait.character.0.as_str()) {
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
    let name = display_name(ctx, portrait);
    let w = i32::try_from(name.chars().count()).unwrap_or(0);
    buf.print(x + (FRAME.0 - w) / 2, PLATE_Y, name, c(name_fg), bg);
}

impl DialogueScreen {
    /// The text box: speaker name on the border, the revealed text (or the
    /// skip question) and the blinking `▼` once the page is shown.
    fn draw_text_box(&self, ctx: &Ctx, buf: &mut GlyphBuffer, view: &View) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let km = &ctx.keymap;
        buf.fill_rect(TEXT_BOX, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(TEXT_BOX, BoxStyle::Single, c(UiColor::PanelBorder), bg);
        let speaker = match view.speaker {
            Some(Side::Left) => view.left,
            Some(Side::Right) => view.right,
            None => None,
        };
        if let Some(speaker) = speaker {
            let name = format!(" {} ", display_name(ctx, speaker));
            buf.print(
                TEXT_BOX.x + 3,
                TEXT_BOX.y,
                &name,
                c(UiColor::TextHighlight),
                bg,
            );
        }

        if self.asking_skip {
            let yes_no = help_line(&[
                (key_name(km, Action::Confirm), "yes"),
                (key_name(km, Action::Cancel), "no"),
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
        // of the three rows (two or three fill them from the top).
        let top = TEXT_Y + i32::from(centred && lines.len() == 1);
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

        if self.is_revealed() {
            let right = TEXT_BOX.x + TEXT_BOX.w - 4;
            let bottom = TEXT_BOX.y + TEXT_BOX.h - 2;
            if let Some(key) = key_name(km, Action::Confirm) {
                let w = i32::try_from(key.chars().count()).unwrap_or(0);
                buf.print(right - 1 - w, bottom, &key, c(UiColor::TextDim), bg);
            }
            if self.waiting % (2.0 * BLINK_S) < BLINK_S {
                buf.print(right, bottom, "▼", c(UiColor::TextHighlight), bg);
            }
        }
    }
}

#[cfg(test)]
mod tests;
