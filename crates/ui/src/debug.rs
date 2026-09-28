//! Debug screens (F2 in debug builds): a menu of tools. The glyph sampler
//! shows every font glyph and palette colour, for judging the look (ticket
//! 0011); the portrait viewer shows every portrait (ticket 0703).

mod portrait_viewer;

pub use portrait_viewer::PortraitViewerScreen;

use crate::color::{Palette, UiColor};
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::screens::{centre_x, print_centred};
use crate::widgets::help::{cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};
use trpg_content::palette::REQUIRED_COLORS;

/// Names of every debug screen: the debug key does nothing while one is on
/// top.
pub const SCREENS: [&str; 3] = [
    DebugMenuScreen::NAME,
    GlyphSamplerScreen::NAME,
    PortraitViewerScreen::NAME,
];

/// The debug tools, in menu order.
const TOOLS: [&str; 2] = ["Glyph sampler", "Portraits"];
/// Row of the debug menu's title.
const MENU_TITLE_ROW: i32 = 9;

/// The debug menu the debug key opens; Cancel closes it.
#[derive(Debug, Clone)]
pub struct DebugMenuScreen {
    menu: Menu,
}

impl DebugMenuScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "debug_menu";

    /// The menu with its first tool focused.
    pub fn new() -> Self {
        Self {
            menu: Menu::new(TOOLS.iter().map(|&t| MenuItem::new(t)).collect()),
        }
    }
}

impl Default for DebugMenuScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for DebugMenuScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            match self.menu.handle(action) {
                Some(MenuEvent::Cancelled) => return Transition::Pop,
                Some(MenuEvent::Chosen(0)) => {
                    return Transition::Push(Box::new(GlyphSamplerScreen::new(ctx)));
                }
                Some(MenuEvent::Chosen(_)) => {
                    return Transition::Push(Box::new(PortraitViewerScreen::new()));
                }
                None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        let hi = c(UiColor::TextHighlight);
        print_centred(buf, MENU_TITLE_ROW, "Debug tools", hi, black);
        let (w, _) = self.menu.size();
        let x = centre_x(buf, usize::try_from(w).unwrap_or(0));
        self.menu.draw(&ctx.palette, buf, x, MENU_TITLE_ROW + 2);
        let km = &ctx.keymap;
        let help = help_line(&[
            (cursor_keys_name(km), "move"),
            (key_name(km, Action::Confirm), "open"),
            (key_name(km, Action::Cancel), "back"),
        ]);
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &help, c(UiColor::TextDim), black);
    }
}

/// The [`glyph_sampler`] as a screen; Cancel closes it.
#[derive(Debug, Clone)]
pub struct GlyphSamplerScreen {
    /// Drawn once on creation; the sampler never changes.
    sampler: GlyphBuffer,
}

impl GlyphSamplerScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "glyph_sampler";

    /// The sampler for the font and palette in `ctx`, without the portrait
    /// colours ([`sampler_palette`]).
    pub fn new(ctx: &Ctx) -> Self {
        let glyphs: Vec<char> = ctx.content.font.glyphs.keys().copied().collect();
        Self {
            sampler: glyph_sampler(&sampler_palette(ctx), &glyphs),
        }
    }
}

/// The palette the sampler shows: every colour except those portraits use
/// (there are too many to fit, and the portrait viewer shows them in
/// context). UI colours always stay.
pub fn sampler_palette(ctx: &Ctx) -> Palette {
    let content = &ctx.content;
    let portrait_only = |name: &str| {
        !REQUIRED_COLORS.contains(&name)
            && content
                .portraits
                .values()
                .any(|p| p.colors.values().any(|c| c == name))
    };
    let mut def = content.palette.clone();
    def.colors.retain(|name, _| !portrait_only(name));
    // The UI colours are kept, so this can't fail.
    Palette::new(&def).unwrap_or_else(|_| ctx.palette.clone())
}

impl Screen for GlyphSamplerScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, _ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if input.actions.contains(&Action::Cancel) {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, _ctx: &Ctx, buf: &mut GlyphBuffer) {
        buf.blit(&self.sampler, 0, 0);
    }
}

/// Glyphs per sampler row; each glyph is followed by a blank cell.
const GLYPHS_PER_ROW: usize = 48;
/// Width of one palette swatch column (`██ name`).
const SWATCH_W: i32 = 24;
/// Swatch columns across the console.
const SWATCH_COLUMNS: i32 = 4;

/// Row of the "Palette" heading: right after the glyph grid's own title and
/// rows, no blank row between. Computed from `glyphs` (rather than a fixed
/// constant) so the bottom half doesn't waste rows the top half didn't use,
/// however many glyphs the font ends up with.
fn palette_top(glyphs: &[char]) -> i32 {
    let glyph_rows: i32 = glyphs
        .chunks(GLYPHS_PER_ROW)
        .count()
        .try_into()
        .unwrap_or(i32::MAX);
    1 + glyph_rows
}

/// Row where [`sample_panels`] starts, for `palette`'s size and `glyphs`'
/// row count: the palette title, its swatch rows, then the sample sentence,
/// with no blank rows between (every row here is scarce once the palette is
/// large).
fn panels_top(palette: &Palette, glyphs: &[char]) -> i32 {
    let swatch_rows: i32 = palette
        .iter()
        .count()
        .div_ceil(SWATCH_COLUMNS as usize)
        .try_into()
        .unwrap_or(i32::MAX);
    palette_top(glyphs) + 2 + swatch_rows
}

/// A console-sized buffer: the top half shows every glyph in `glyphs` in a
/// grid; the bottom half shows every palette colour as a swatch plus its
/// name, a line of sample text and two sample panels.
pub fn glyph_sampler(palette: &Palette, glyphs: &[char]) -> GlyphBuffer {
    let c = |u| palette.get(u);
    let (text, dim, black) = (c(UiColor::Text), c(UiColor::TextDim), c(UiColor::Black));
    let mut b = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, Cell::new(' ', text, black));

    let title = format!("Glyphs ({})", glyphs.len());
    b.print(1, 0, &title, c(UiColor::TextHighlight), black);
    for (row, chunk) in (1..).zip(glyphs.chunks(GLYPHS_PER_ROW)) {
        for (col, &g) in (0..).zip(chunk) {
            b.print(2 + col * 2, row, &g.to_string(), text, black);
        }
    }

    let bottom = palette_top(glyphs);
    let colors: Vec<_> = palette.iter().collect();
    let title = format!("Palette ({})", colors.len());
    b.print(1, bottom, &title, c(UiColor::TextHighlight), black);
    for (i, &(name, rgb)) in (0..).zip(&colors) {
        let (x, y) = (
            1 + i % SWATCH_COLUMNS * SWATCH_W,
            bottom + 1 + i / SWATCH_COLUMNS,
        );
        b.print(x, y, "██", rgb, black);
        b.print(x + 3, y, name, dim, black);
    }

    let panels_top = panels_top(palette, glyphs);
    b.print(
        1,
        panels_top - 1,
        "The quick brown fox jumps over the lazy dog. 0123456789 ÀÉÎÕÜ àéîõü ß ¿¡ «» ← ↑ → ↓",
        text,
        black,
    );
    sample_panels(&mut b, palette, panels_top);
    b
}

/// A single-line info panel and a double-line (focused) panel from row `top`.
fn sample_panels(buf: &mut GlyphBuffer, palette: &Palette, top: i32) {
    let ui = |u| palette.get(u);
    let bg = ui(UiColor::PanelBg);
    let height = i32::from(CONSOLE_H) - top;

    let panel = Rect::new(1, top, 40, height);
    buf.fill_rect(panel, Cell::new(' ', ui(UiColor::Text), bg));
    buf.draw_box(panel, BoxStyle::Single, ui(UiColor::PanelBorder), bg);
    buf.print(3, top, " Unit ", ui(UiColor::TextHighlight), bg);
    buf.print(3, top + 1, "Aldo", ui(UiColor::Player), bg);
    buf.print_fg(8, top + 1, "vs", ui(UiColor::TextDim));
    buf.print(11, top + 1, "Brigand", ui(UiColor::Enemy), bg);
    buf.print(3, top + 2, "HP", ui(UiColor::Text), bg);
    buf.print(6, top + 2, "■■■■■■", ui(UiColor::HpHigh), bg);
    buf.print(12, top + 2, "■■■", ui(UiColor::HpMid), bg);
    buf.print(15, top + 2, "■", ui(UiColor::HpLow), bg);
    buf.print(18, top + 2, "░▒▓█▀▄▌▐", ui(UiColor::ExpBar), bg);

    let focus = Rect::new(43, top, 56, height);
    buf.fill_rect(focus, Cell::new(' ', ui(UiColor::Text), bg));
    buf.draw_box(focus, BoxStyle::Double, ui(UiColor::PanelBorderFocus), bg);
    buf.print(45, top, " Ranges ", ui(UiColor::TextHighlight), bg);
    for (i, (label, tint)) in (0..).zip([
        ("move", UiColor::MoveRange),
        ("attack", UiColor::AttackRange),
        ("heal", UiColor::HealRange),
        ("danger", UiColor::DangerZone),
    ]) {
        let x = 45 + i * 13;
        buf.blend_bg(Rect::new(x, top + 1, 12, 1), ui(tint), 1.0);
        buf.print_fg(x, top + 1, &format!("{label:^12}"), ui(UiColor::Text));
    }
    // Two-cell tiles, as in ADR-0012's examples.
    let terrain = ["grass", "forest", "water", "mountain", "stone"];
    let glyphs = ["..", "♣♣", "≈≈", "^^", "▓▓"];
    for (i, (name, g)) in (0..).zip(terrain.iter().zip(glyphs)) {
        let fg = palette.lookup(name).unwrap_or(ui(UiColor::White));
        buf.print(45 + i * 3, top + 2, g, fg, bg);
    }
    buf.print(61, top + 2, "A·", ui(UiColor::Player), bg);
    buf.print(64, top + 2, "Bˇ", ui(UiColor::Enemy).scale(0.5), bg);
    buf.print(67, top + 2, "C!", ui(UiColor::Ally), bg);
    buf.print(70, top + 2, "D•", ui(UiColor::Neutral), bg);
    buf.print(73, top + 2, "[]", ui(UiColor::Cursor), bg);
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use trpg_content::FontAtlasDef;

    use super::*;
    use crate::color::tests::game_palette;

    fn atlas_glyphs() -> Vec<char> {
        FontAtlasDef::load().unwrap().glyphs.into_keys().collect()
    }

    /// The palette the sampler screen shows.
    fn sampler_colours() -> Palette {
        sampler_palette(&crate::screen::tests::ctx())
    }

    #[test]
    fn sampler_leaves_out_portrait_colours_only() {
        let all = game_palette();
        let shown = sampler_colours();
        assert_eq!(shown.lookup("skin_light"), None);
        assert_eq!(shown.lookup("grass"), all.lookup("grass"));
        for c in UiColor::ALL {
            assert_eq!(shown.get(*c), all.get(*c));
        }
        // A UI colour a portrait uses stays.
        let mut ctx = crate::screen::tests::ctx();
        if let Some(p) = ctx.content.portraits.values_mut().next() {
            p.colors.insert('Z', "cursor".to_owned());
        }
        assert!(sampler_palette(&ctx).lookup("cursor").is_some());
        // Without portraits, nothing is left out.
        ctx.content.portraits.clear();
        assert_eq!(sampler_palette(&ctx), all);
    }

    #[test]
    fn sampler_shows_every_glyph_and_colour() {
        let p = sampler_colours();
        let glyphs = atlas_glyphs();
        let b = glyph_sampler(&p, &glyphs);
        assert_eq!((b.width(), b.height()), (CONSOLE_W, CONSOLE_H));
        let bottom = palette_top(&glyphs);
        let row_end = 2 + 2 * i32::try_from(GLYPHS_PER_ROW).unwrap();
        let shown: String = (1..bottom)
            .flat_map(|y| (2..row_end).step_by(2).map(move |x| (x, y)))
            .map(|(x, y)| b.get(x, y).unwrap().glyph)
            .collect();
        let expected: String = glyphs.iter().collect();
        assert!(
            shown.starts_with(&expected),
            "top half must list every glyph in order"
        );
        for (name, rgb) in p.iter() {
            let found = (bottom..i32::from(CONSOLE_H)).any(|y| {
                (0..i32::from(CONSOLE_W)).any(|x| {
                    let cell = b.get(x, y).unwrap();
                    cell.glyph == '█' && cell.fg == rgb
                })
            });
            assert!(found, "no swatch for {name}");
        }
    }

    #[test]
    fn glyph_grid_fits_top_half() {
        let glyphs = atlas_glyphs();
        let rows = glyphs.len().div_ceil(GLYPHS_PER_ROW);
        assert!(i32::try_from(rows).unwrap() < palette_top(&glyphs));
        assert!(2 + 2 * GLYPHS_PER_ROW <= usize::from(CONSOLE_W));
        assert!(SWATCH_COLUMNS * SWATCH_W < i32::from(CONSOLE_W));
    }

    #[test]
    fn demo_panels_fit_below_the_embedded_palette() {
        let p = sampler_colours();
        let glyphs = atlas_glyphs();
        assert!(panels_top(&p, &glyphs) <= i32::from(CONSOLE_H) - 4);
    }

    #[test]
    fn sampler_screen_draws_the_sampler_and_closes_on_cancel() {
        let mut ctx = crate::screen::tests::ctx();
        let mut screen = GlyphSamplerScreen::new(&ctx);
        assert_eq!(screen.name(), "glyph_sampler");
        let p = &ctx.palette;
        let mut buf = GlyphBuffer::new(
            CONSOLE_W,
            CONSOLE_H,
            Cell::new('x', p.get(UiColor::Text), p.get(UiColor::Black)),
        );
        screen.draw(&ctx, &mut buf);
        assert_eq!(buf, glyph_sampler(&sampler_palette(&ctx), &atlas_glyphs()));
        let frame = |a: &[Action]| FrameInput::new(a.to_vec(), 0.0, vec![]);
        let stay = screen.update(&mut ctx, &frame(&[Action::Confirm]));
        assert!(matches!(stay, Transition::None));
        let pop = screen.update(&mut ctx, &frame(&[Action::Confirm, Action::Cancel]));
        assert!(matches!(pop, Transition::Pop));
    }

    #[test]
    fn debug_menu_opens_each_tool() {
        use Action::{Cancel, Confirm, CursorDown, CursorUp};
        let mut ctx = crate::screen::tests::ctx();
        let mut menu = DebugMenuScreen::default();
        assert_eq!(menu.name(), "debug_menu");
        let mut outcome = |m: &mut DebugMenuScreen, a: &[Action]| {
            format!(
                "{:?}",
                m.update(&mut ctx, &FrameInput::new(a.to_vec(), 0.0, vec![]))
            )
        };
        assert_eq!(outcome(&mut menu, &[CursorUp]), "None");
        assert_eq!(
            outcome(&mut menu, &[CursorDown, Confirm]),
            "Push(glyph_sampler)"
        );
        assert_eq!(
            outcome(&mut menu, &[CursorDown, Confirm]),
            "Push(portrait_viewer)"
        );
        assert_eq!(outcome(&mut menu, &[Cancel, Confirm]), "Pop");
        assert_eq!(SCREENS, ["debug_menu", "glyph_sampler", "portrait_viewer"]);
    }

    #[test]
    fn debug_menu_snapshot() {
        let ctx = crate::screen::tests::ctx();
        let p = &ctx.palette;
        let mut buf = GlyphBuffer::new(
            CONSOLE_W,
            CONSOLE_H,
            Cell::new('x', p.get(UiColor::Text), p.get(UiColor::PanelBg)),
        );
        DebugMenuScreen::new().draw(&ctx, &mut buf);
        assert_snapshot!(buf.to_snapshot(p));
    }

    #[test]
    fn sampler_snapshot() {
        let p = sampler_colours();
        let snap = glyph_sampler(&p, &atlas_glyphs()).to_snapshot(&p);
        assert_snapshot!(snap);
    }
}
