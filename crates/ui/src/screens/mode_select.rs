//! The game mode screen, right after `New Game` (ticket 0801,
//! `docs/design/death-and-difficulty.md`): Classic or Casual, one line
//! explaining each.

use trpg_core::GameMode;

use super::print_centred;
use crate::color::UiColor;
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// The heading.
pub const HEADING: &str = "Choose a game mode";

/// Each mode, its name and the line explaining it.
pub const MODES: [(GameMode, &str, &str); 2] = [
    (
        GameMode::Classic,
        "Classic",
        "Classic: a unit that falls in battle dies and is gone for good.",
    ),
    (
        GameMode::Casual,
        "Casual",
        "Casual: a unit that falls retreats, and is back for the next battle.",
    ),
];

/// Row of the heading.
const HEADING_ROW: i32 = 8;
/// Top row of the menu box.
const MENU_ROW: i32 = 11;
/// Row of the first explanation line.
const TEXT_ROW: i32 = 17;

/// Classic or Casual: Confirm picks the focused mode, Cancel goes back.
/// Pops either way; [`result`](Self::result) says which.
#[derive(Debug, Clone)]
pub struct ModeSelectScreen {
    menu: Menu,
    chosen: Option<GameMode>,
}

impl ModeSelectScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "mode_select";

    /// The screen with Classic focused.
    pub fn new() -> Self {
        let items = MODES.iter().map(|&(_, name, _)| MenuItem::new(name));
        Self {
            menu: Menu::new(items.collect()),
            chosen: None,
        }
    }

    /// The mode picked, once the screen has popped; `None` if the player
    /// went back.
    pub fn result(&self) -> Option<GameMode> {
        self.chosen
    }

    /// The menu.
    pub fn menu(&self) -> &Menu {
        &self.menu
    }

    /// The bottom help line.
    pub fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        help_line(&[
            (Some(cursor_keys_name(km)), "choose"),
            (Some(key_name(km, Action::Confirm)), "select"),
            (Some(key_name(km, Action::Cancel)), "back"),
        ])
    }
}

impl Default for ModeSelectScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for ModeSelectScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            match self.menu.handle_with_sound(action, &mut ctx.audio) {
                Some(MenuEvent::Chosen(i)) => {
                    self.chosen = MODES.get(i).map(|&(mode, ..)| mode);
                    return Transition::Pop;
                }
                Some(MenuEvent::Cancelled) => return Transition::Pop,
                None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        print_centred(buf, HEADING_ROW, HEADING, c(UiColor::TextHighlight), black);
        let (w, _) = self.menu.size();
        self.menu.draw(
            &ctx.palette,
            buf,
            (i32::from(buf.width()) - w) / 2,
            MENU_ROW,
        );
        for (i, (y, &(_, _, line))) in (TEXT_ROW..).step_by(2).zip(&MODES).enumerate() {
            let fg = if i == self.menu.focus() {
                UiColor::Text
            } else {
                UiColor::TextDim
            };
            print_centred(buf, y, line, c(fg), black);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &Self::help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::harness::Harness;
    use crate::screen::tests::ctx;

    fn update(s: &mut ModeSelectScreen, actions: &[Action]) -> String {
        let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
        format!("{:?}", s.update(&mut ctx(), &input))
    }

    #[test]
    fn confirm_picks_the_focused_mode() {
        let mut s = ModeSelectScreen::new();
        assert_eq!(s.name(), "mode_select");
        assert_eq!(update(&mut s, &[Action::CursorDown]), "None");
        assert_eq!(s.result(), None);
        assert_eq!(update(&mut s, &[Action::Confirm]), "Pop");
        assert_eq!(s.result(), Some(GameMode::Casual));
        let mut s = ModeSelectScreen::default();
        assert_eq!(
            update(&mut s, &[Action::Confirm, Action::CursorDown]),
            "Pop"
        );
        assert_eq!(s.result(), Some(GameMode::Classic));
    }

    #[test]
    fn cancel_goes_back_without_a_mode() {
        let mut s = ModeSelectScreen::new();
        assert_eq!(update(&mut s, &[Action::Cancel]), "Pop");
        assert_eq!(s.result(), None);
    }

    /// The heading, the two modes (Casual focused) and both lines, the
    /// focused one brighter.
    #[test]
    fn mode_select_snapshot() {
        let mut h = Harness::with_screen(Box::new(ModeSelectScreen::new()));
        h.keys("Down");
        assert_snapshot!(h.snapshot());
    }
}
