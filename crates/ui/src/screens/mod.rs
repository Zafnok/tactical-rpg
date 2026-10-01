//! The game's screens. Debug-only screens live in [`crate::debug`].

pub mod battle;
pub mod dialogue;
pub mod game_over;
pub mod layout_picker;
pub mod lead_select;
pub mod mode_select;
pub mod title;

pub use battle::BattleScreen;
pub use dialogue::DialogueScreen;
pub use game_over::{GameOverScreen, ToBeContinuedScreen};
pub use layout_picker::LayoutPickerScreen;
pub use lead_select::LeadSelectScreen;
pub use mode_select::ModeSelectScreen;
pub use title::TitleScreen;

use crate::color::{Rgb, UiColor};
use crate::glyph_buffer::GlyphBuffer;
use crate::input::Action;
use crate::screen::Ctx;
use crate::widgets::help::key_name;

/// Column at which `width` cells are centred in `buf` (rounded left).
pub(crate) fn centre_x(buf: &GlyphBuffer, width: usize) -> i32 {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    (i32::from(buf.width()) - width) / 2
}

/// Prints `text` centred on row `y`.
pub(crate) fn print_centred(buf: &mut GlyphBuffer, y: i32, text: &str, fg: Rgb, bg: Rgb) {
    let x = centre_x(buf, text.chars().count());
    buf.print(x, y, text, fg, bg);
}

/// The debug-menu hint, e.g. `F2 debug`: `None` unless debug tools are on
/// and the Debug action has a key (it isn't rebindable, so no
/// `! not mapped` hint for it).
pub(crate) fn debug_hint(ctx: &Ctx) -> Option<String> {
    let bound = ctx.keymap.primary(Action::Debug).is_some();
    (ctx.debug_tools && bound).then(|| format!("{} debug", key_name(&ctx.keymap, Action::Debug)))
}

/// Prints the [`debug_hint`] right-aligned on row `y`, one cell in from the
/// edge, unless those cells or the one before them already hold text.
pub(crate) fn draw_debug_hint(ctx: &Ctx, buf: &mut GlyphBuffer, y: i32) {
    let Some(hint) = debug_hint(ctx) else {
        return;
    };
    let len = i32::try_from(hint.chars().count()).unwrap_or(i32::MAX);
    let x = i32::from(buf.width()) - 1 - len;
    let free = (x - 1..x + len).all(|cx| buf.get(cx, y).is_some_and(|c| c.glyph == ' '));
    if free {
        let dim = ctx.palette.get(UiColor::TextDim);
        buf.print(x, y, &hint, dim, ctx.palette.get(UiColor::Black));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::tests::ctx;

    fn blank(c: &Ctx) -> GlyphBuffer {
        let black = c.palette.get(UiColor::Black);
        GlyphBuffer::new(20, 1, crate::glyph_buffer::Cell::new(' ', black, black))
    }

    fn row(buf: &GlyphBuffer, y: i32) -> String {
        (0..i32::from(buf.width()))
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect()
    }

    #[test]
    fn debug_hint_names_the_key_only_with_debug_tools() {
        let mut c = ctx();
        assert_eq!(debug_hint(&c).as_deref(), Some("F2 debug"));
        c.debug_tools = false;
        assert_eq!(debug_hint(&c), None);
    }

    #[test]
    fn debug_hint_is_hidden_when_debug_is_unbound() {
        let mut c = ctx();
        c.keymap = crate::input::Keymap::new(
            Action::ALL
                .iter()
                .flat_map(|&a| c.keymap.chords_for(a).into_iter().map(move |ch| (ch, a)))
                .filter(|&(_, a)| a != Action::Debug),
            c.keymap.repeat(),
        );
        assert_eq!(debug_hint(&c), None);
    }

    #[test]
    fn debug_hint_is_right_aligned_and_skipped_over_text() {
        let c = ctx();
        let black = c.palette.get(UiColor::Black);
        let mut buf = blank(&c);
        buf.print(0, 0, "0123456789", black, black);
        draw_debug_hint(&c, &mut buf, 0);
        assert_eq!(row(&buf, 0), "0123456789 F2 debug ");
        // No gap before the hint's first cell, or text under it: no hint.
        for end in ["01234567890", "0123456789 F2 debu"] {
            let mut buf = blank(&c);
            buf.print(0, 0, end, black, black);
            draw_debug_hint(&c, &mut buf, 0);
            assert_eq!(row(&buf, 0).trim_end(), end);
        }
        // Text after the hint's cells (the right margin) doesn't matter.
        let mut buf = blank(&c);
        buf.print(19, 0, "x", black, black);
        draw_debug_hint(&c, &mut buf, 0);
        assert_eq!(row(&buf, 0), "           F2 debugx");
    }
}
