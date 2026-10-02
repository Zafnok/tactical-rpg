//! The screens after a battle's end (ticket 0801): Game Over after a
//! defeat (`Retry Battle` / `Title`, `docs/design/death-and-difficulty.md`) and
//! "To be continued" after the last chapter there is.

use super::print_centred;
use crate::color::UiColor;
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// Game Over's heading.
pub const GAME_OVER: &str = "GAME OVER";
/// Game Over's items.
const RETRY: &str = "Retry Battle";
/// Back to the title.
const TITLE: &str = "Title";
/// The last chapter's closing words.
pub const TO_BE_CONTINUED: &str = "To be continued...";

/// Row of the headings.
const HEADING_ROW: i32 = 10;
/// Top row of Game Over's menu.
const MENU_ROW: i32 = 13;

/// What the player picked on Game Over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOverChoice {
    /// Start the battle again from its first turn, every rewind charge back.
    Retry,
    /// Back to the title screen.
    Title,
}

/// Fills `buf` with blank cells.
fn clear(ctx: &Ctx, buf: &mut GlyphBuffer) {
    let c = |u| ctx.palette.get(u);
    buf.fill_rect(
        buf.bounds(),
        Cell::new(' ', c(UiColor::Text), c(UiColor::Black)),
    );
}

/// Prints `help` on the bottom row.
fn draw_help(ctx: &Ctx, buf: &mut GlyphBuffer, help: &str) {
    let c = |u| ctx.palette.get(u);
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, help, c(UiColor::TextDim), c(UiColor::Black));
}

/// `GAME OVER` and a `Retry Battle` / `Title` menu. Pops once one is chosen
/// ([`result`](Self::result)); there is nothing to back out of.
#[derive(Debug, Clone)]
pub struct GameOverScreen {
    menu: Menu,
    chosen: Option<GameOverChoice>,
}

impl GameOverScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "game_over";

    /// The screen with `Retry Battle` focused.
    pub fn new() -> Self {
        let items = [RETRY, TITLE].map(MenuItem::new).to_vec();
        Self {
            menu: Menu::new(items).without_cancel(),
            chosen: None,
        }
    }

    /// The choice, once the screen has popped.
    pub fn result(&self) -> Option<GameOverChoice> {
        self.chosen
    }

    /// The bottom help line.
    pub fn help(ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        help_line(&[
            (Some(cursor_keys_name(km)), "choose"),
            (Some(key_name(km, Action::Confirm)), "select"),
        ])
    }
}

impl Default for GameOverScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for GameOverScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            if let Some(MenuEvent::Chosen(i)) = self.menu.handle_with_sound(action, &mut ctx.audio)
            {
                self.chosen = Some(if i == 0 {
                    GameOverChoice::Retry
                } else {
                    GameOverChoice::Title
                });
                return Transition::Pop;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        clear(ctx, buf);
        let black = c(UiColor::Black);
        print_centred(
            buf,
            HEADING_ROW,
            GAME_OVER,
            c(UiColor::TextHighlight),
            black,
        );
        let (w, _) = self.menu.size();
        let x = (i32::from(buf.width()) - w) / 2;
        self.menu.draw(&ctx.palette, buf, x, MENU_ROW);
        draw_help(ctx, buf, &Self::help(ctx));
    }
}

/// "To be continued...": Confirm goes back to the title (pops).
#[derive(Debug, Clone, Copy, Default)]
pub struct ToBeContinuedScreen;

impl ToBeContinuedScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "to_be_continued";

    /// The bottom help line.
    pub fn help(ctx: &Ctx) -> String {
        help_line(&[(Some(key_name(ctx.help_keys(), Action::Confirm)), "title")])
    }
}

impl Screen for ToBeContinuedScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if input.actions.contains(&Action::Confirm) {
            ctx.audio.menu(crate::audio::MenuSound::Select);
            return Transition::Pop;
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        clear(ctx, buf);
        let black = c(UiColor::Black);
        print_centred(
            buf,
            HEADING_ROW,
            TO_BE_CONTINUED,
            c(UiColor::TextHighlight),
            black,
        );
        draw_help(ctx, buf, &Self::help(ctx));
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::harness::Harness;
    use crate::screen::tests::ctx;

    fn update(s: &mut dyn Screen, actions: &[Action]) -> String {
        let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
        format!("{:?}", s.update(&mut ctx(), &input))
    }

    #[test]
    fn game_over_offers_retry_and_title() {
        let mut s = GameOverScreen::new();
        assert_eq!(s.name(), "game_over");
        // Nothing to back out of.
        assert_eq!(update(&mut s, &[Action::Cancel]), "None");
        assert_eq!(s.result(), None);
        assert_eq!(update(&mut s, &[Action::Confirm]), "Pop");
        assert_eq!(s.result(), Some(GameOverChoice::Retry));
        let mut s = GameOverScreen::default();
        assert_eq!(
            update(&mut s, &[Action::CursorDown, Action::Confirm]),
            "Pop"
        );
        assert_eq!(s.result(), Some(GameOverChoice::Title));
    }

    #[test]
    fn to_be_continued_waits_for_confirm() {
        let mut s = ToBeContinuedScreen;
        assert_eq!(s.name(), "to_be_continued");
        assert_eq!(
            update(&mut s, &[Action::Cancel, Action::CursorDown]),
            "None"
        );
        assert_eq!(update(&mut s, &[Action::Confirm]), "Pop");
    }

    #[test]
    fn game_over_snapshot() {
        let h = Harness::with_screen(Box::new(GameOverScreen::new()));
        assert_snapshot!(h.snapshot());
    }

    #[test]
    fn to_be_continued_snapshot() {
        let h = Harness::with_screen(Box::new(ToBeContinuedScreen));
        assert_snapshot!(h.snapshot());
    }
}
