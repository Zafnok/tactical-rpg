//! The portrait viewer (debug builds, ticket 0703): every portrait in the
//! content, one expression at a time, plus its mirrored and dimmed
//! "listener" look, for checking art (0706).

use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::portrait::draw_portrait;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{help_line, key_name};

/// Left column of the character list.
const LIST_X: i32 = 1;
/// Top row of the frames and the list.
const TOP: i32 = 2;
/// Left column of the speaker frame (bright, facing right).
const SPEAKER_X: i32 = 22;
/// Left column of the listener frame (mirrored and dimmed).
const LISTENER_X: i32 = 62;
/// Frame size: a 32×16-cell portrait plus the border.
const FRAME: (i32, i32) = (34, 18);
/// How much the listener is dimmed (the dialogue screen's value is 0704's).
const LISTENER_DIM: f32 = 0.5;

/// Browses the portraits: `CursorLeft`/`CursorRight` switch expression,
/// `CursorUp`/`CursorDown` switch character, `Cancel` closes.
#[derive(Debug, Clone, Default)]
pub struct PortraitViewerScreen {
    /// Index of the character, in id order.
    character: usize,
    /// Index of the expression, in file order.
    expression: usize,
}

impl PortraitViewerScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "portrait_viewer";

    /// The viewer on the first portrait's first expression.
    pub fn new() -> Self {
        Self::default()
    }

    /// The character id and expression name shown, if there are portraits.
    pub fn showing<'a>(&self, ctx: &'a Ctx) -> Option<(&'a str, &'a str)> {
        let (id, portrait) = ctx.content.portraits.iter().nth(self.character)?;
        let expression = portrait.expressions.get(self.expression)?;
        Some((id.as_str(), expression.name.as_str()))
    }

    /// The help line, e.g. `Left/Right expression · Down/Up character · d back`.
    fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        let pair = |a, b| Some(format!("{}/{}", key_name(km, a), key_name(km, b)));
        help_line(&[
            (pair(Action::CursorLeft, Action::CursorRight), "expression"),
            (pair(Action::CursorDown, Action::CursorUp), "character"),
            (Some(key_name(km, Action::Cancel)), "back"),
        ])
    }
}

/// `i` stepped by `delta` within `0..n`, wrapping; 0 when `n` is 0.
fn wrap(i: usize, delta: isize, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    (i + n).wrapping_add_signed(delta) % n
}

impl Screen for PortraitViewerScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        let portraits = &ctx.content.portraits;
        for &action in &input.actions {
            match action {
                Action::Cancel => {
                    ctx.audio.menu(MenuSound::Cancel);
                    return Transition::Pop;
                }
                Action::CursorDown | Action::CursorUp => {
                    let delta = if action == Action::CursorDown { 1 } else { -1 };
                    let character = wrap(self.character, delta, portraits.len());
                    if character != self.character {
                        ctx.audio.menu(MenuSound::Move);
                    }
                    self.character = character;
                    self.expression = 0;
                }
                Action::CursorLeft | Action::CursorRight => {
                    let n = portraits
                        .values()
                        .nth(self.character)
                        .map_or(0, |p| p.expressions.len());
                    let delta = if action == Action::CursorRight { 1 } else { -1 };
                    let expression = wrap(self.expression, delta, n);
                    if expression != self.expression {
                        ctx.audio.menu(MenuSound::Move);
                    }
                    self.expression = expression;
                }
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let (black, text, dim, hi) = (
            c(UiColor::Black),
            c(UiColor::Text),
            c(UiColor::TextDim),
            c(UiColor::TextHighlight),
        );
        buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
        buf.print(LIST_X, 0, "Portraits (debug)", hi, black);
        let bottom = i32::from(buf.height()) - 1;
        buf.print(LIST_X, bottom, &Self::help(ctx), dim, black);
        let Some((id, expr)) = self.showing(ctx) else {
            buf.print(
                LIST_X,
                TOP,
                "No portraits in assets/portraits/",
                text,
                black,
            );
            return;
        };

        for (y, name) in (TOP..).zip(ctx.content.portraits.keys()) {
            let (mark, fg) = if name == id { ("►", hi) } else { (" ", text) };
            buf.print(LIST_X, y, &format!("{mark} {name}"), fg, black);
        }

        let portrait = &ctx.content.portraits[id];
        let bg = c(UiColor::PanelBg);
        let frames = [
            (
                SPEAKER_X,
                BoxStyle::Double,
                UiColor::PanelBorderFocus,
                0.0,
                false,
                "speaker",
            ),
            (
                LISTENER_X,
                BoxStyle::Single,
                UiColor::PanelBorder,
                LISTENER_DIM,
                true,
                "listener",
            ),
        ];
        for (x, style, border, dimmed, mirror, label) in frames {
            let rect = Rect::new(x, TOP, FRAME.0, FRAME.1);
            buf.fill_rect(rect, Cell::new(' ', text, bg));
            buf.draw_box(rect, style, c(border), bg);
            draw_portrait(
                buf,
                &ctx.palette,
                (x + 1, TOP + 1),
                portrait,
                expr,
                dimmed,
                mirror,
            );
            buf.print(x + 1, TOP + FRAME.1, label, dim, black);
        }

        let caption_y = TOP + FRAME.1 + 2;
        let count = portrait.expressions.len();
        let caption = format!("{id} — {expr} ({}/{count})", self.expression + 1);
        buf.print(SPEAKER_X, caption_y, &caption, hi, black);
        let mut x = SPEAKER_X;
        for e in &portrait.expressions {
            let fg = if e.name == expr { hi } else { dim };
            x += i32::from(buf.print(x, caption_y + 2, &e.name, fg, black)) + 2;
        }
    }
}

#[cfg(test)]
mod tests;
