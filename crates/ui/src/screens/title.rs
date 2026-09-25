//! The placeholder title screen (the real game flow comes with ticket 0801).

use super::{centre_x, print_centred};
use crate::color::UiColor;
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// Title text.
pub const TITLE: &str = "tactical-rpg";
/// Line under the title.
pub const SUBTITLE: &str = "an ASCII tactics game";

/// Row of the title text.
const TITLE_ROW: i32 = 9;
/// Row of the subtitle.
const SUBTITLE_ROW: i32 = 11;
/// Top row of the menu box.
const MENU_ROW: i32 = 14;

/// Menu index of "New Game".
const NEW_GAME: usize = 0;
/// Menu index of "Quit".
const QUIT: usize = 1;

/// Fills `buf` with blank `text`-on-`black` cells.
fn clear(ctx: &Ctx, buf: &mut GlyphBuffer) {
    let p = &ctx.palette;
    buf.fill_rect(
        buf.bounds(),
        Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black)),
    );
}

/// Title, subtitle and a `New Game` / `Quit` menu, with a help line naming
/// the keys of the active layout.
#[derive(Debug, Clone)]
pub struct TitleScreen {
    menu: Menu,
}

impl TitleScreen {
    /// The title screen with `New Game` focused.
    pub fn new() -> Self {
        Self {
            menu: Menu::new(vec![MenuItem::new("New Game"), MenuItem::new("Quit")]),
        }
    }

    /// The bottom help line, e.g. `arrows move · f select · d back`.
    pub fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        help_line(&[
            (cursor_keys_name(km), "move"),
            (key_name(km, Action::Confirm), "select"),
            (key_name(km, Action::Cancel), "back"),
        ])
    }
}

impl Default for TitleScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for TitleScreen {
    fn name(&self) -> &'static str {
        "title"
    }

    fn update(&mut self, _ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            match self.menu.handle(action) {
                Some(MenuEvent::Chosen(NEW_GAME)) => {
                    return Transition::Push(Box::new(PlaceholderScreen));
                }
                Some(MenuEvent::Chosen(QUIT)) => return Transition::Quit,
                // Nothing to back out of on the title screen.
                Some(MenuEvent::Chosen(_) | MenuEvent::Cancelled) | None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        clear(ctx, buf);
        print_centred(buf, TITLE_ROW, TITLE, c(UiColor::TextHighlight), black);
        print_centred(buf, SUBTITLE_ROW, SUBTITLE, c(UiColor::TextDim), black);
        let (w, _) = self.menu.size();
        let x = centre_x(buf, usize::try_from(w).unwrap_or(0));
        self.menu.draw(&ctx.palette, buf, x, MENU_ROW);
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &Self::help(ctx), c(UiColor::TextDim), black);
    }
}

/// Stand-in for screens that don't exist yet; Cancel goes back.
#[derive(Debug, Clone, Copy)]
pub struct PlaceholderScreen;

impl PlaceholderScreen {
    /// The message shown, e.g. `Coming soon — press d to go back`.
    pub fn message(ctx: &Ctx) -> String {
        match key_name(&ctx.keymap, Action::Cancel) {
            Some(key) => format!("Coming soon — press {key} to go back"),
            None => "Coming soon".to_owned(),
        }
    }
}

impl Screen for PlaceholderScreen {
    fn name(&self) -> &'static str {
        "placeholder"
    }

    fn update(&mut self, _ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if input.actions.contains(&Action::Cancel) {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        clear(ctx, buf);
        let y = i32::from(buf.height()) / 2;
        let p = &ctx.palette;
        print_centred(
            buf,
            y,
            &Self::message(ctx),
            p.get(UiColor::Text),
            p.get(UiColor::Black),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::tests::ctx;

    fn input(actions: &[Action]) -> FrameInput {
        FrameInput::new(actions.to_vec(), 0.0, vec![])
    }

    fn outcome(screen: &mut dyn Screen, actions: &[Action]) -> String {
        format!("{:?}", screen.update(&mut ctx(), &input(actions)))
    }

    #[test]
    fn title_menu_transitions() {
        use Action::{Cancel, Confirm, CursorDown, CursorUp};
        let mut t = TitleScreen::new();
        assert_eq!(t.name(), "title");
        assert!(!t.is_overlay());
        assert_eq!(outcome(&mut t, &[]), "None");
        assert_eq!(outcome(&mut t, &[Cancel]), "None");
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(placeholder)");
        assert_eq!(outcome(&mut t, &[CursorDown, Confirm]), "Quit");
        assert_eq!(outcome(&mut t, &[CursorUp]), "None");
        // Actions after the one that transitions are dropped.
        assert_eq!(outcome(&mut t, &[Confirm, CursorDown]), "Push(placeholder)");
        assert_eq!(outcome(&mut t, &[Confirm]), "Push(placeholder)");
    }

    #[test]
    fn placeholder_pops_on_cancel_only() {
        let mut p = PlaceholderScreen;
        assert_eq!(p.name(), "placeholder");
        assert_eq!(outcome(&mut p, &[Action::Confirm]), "None");
        assert_eq!(outcome(&mut p, &[Action::Confirm, Action::Cancel]), "Pop");
    }

    #[test]
    fn texts_name_the_layout_keys() {
        let mut c = ctx();
        assert_eq!(TitleScreen::help(&c), "arrows move · f select · d back");
        assert_eq!(
            PlaceholderScreen::message(&c),
            "Coming soon — press d to go back"
        );
        c.content
            .keymap
            .bindings
            .retain(|_, a| *a != Action::Cancel);
        c.keymap = crate::input::Keymap::from_def(&c.content.keymap);
        assert_eq!(TitleScreen::help(&c), "arrows move · f select");
        assert_eq!(PlaceholderScreen::message(&c), "Coming soon");
    }

    /// Opaque screens must paint every cell, not rely on `Game` clearing.
    #[test]
    fn screens_cover_the_whole_buffer() {
        use crate::console::{CONSOLE_H, CONSOLE_W};
        let c = ctx();
        let screens: [&dyn Screen; 2] = [&TitleScreen::new(), &PlaceholderScreen];
        for screen in screens {
            let stale = Cell::new(
                'x',
                c.palette.get(UiColor::Enemy),
                c.palette.get(UiColor::Enemy),
            );
            let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
            screen.draw(&c, &mut buf);
            let left = (0..i32::from(CONSOLE_H))
                .flat_map(|y| (0..i32::from(CONSOLE_W)).map(move |x| (x, y)))
                .filter(|&(x, y)| buf.get(x, y) == Some(&stale))
                .count();
            assert_eq!(left, 0, "{} left stale cells", screen.name());
        }
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(TitleScreen::default().menu, TitleScreen::new().menu);
    }
}
