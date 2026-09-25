//! Debug screens. The glyph sampler shows every font glyph and palette
//! colour, for judging the look (ticket 0011).

use crate::color::{Palette, UiColor};
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};

/// The [`glyph_sampler`] as a screen (F12 in debug builds); Cancel closes it.
#[derive(Debug, Clone)]
pub struct GlyphSamplerScreen {
    /// Drawn once on creation; the sampler never changes.
    sampler: GlyphBuffer,
}

impl GlyphSamplerScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "glyph_sampler";

    /// The sampler for the font and palette in `ctx`.
    pub fn new(ctx: &Ctx) -> Self {
        let glyphs: Vec<char> = ctx.content.font.glyphs.keys().copied().collect();
        Self {
            sampler: glyph_sampler(&ctx.palette, &glyphs),
        }
    }
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
/// First row of the bottom half.
const BOTTOM: i32 = 16;

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

    let colors: Vec<_> = palette.iter().collect();
    let title = format!("Palette ({})", colors.len());
    b.print(1, BOTTOM, &title, c(UiColor::TextHighlight), black);
    let mut swatch_rows = 0;
    for (i, &(name, rgb)) in (0..).zip(&colors) {
        let (x, y) = (
            1 + i % SWATCH_COLUMNS * SWATCH_W,
            BOTTOM + 1 + i / SWATCH_COLUMNS,
        );
        b.print(x, y, "██", rgb, black);
        b.print(x + 3, y, name, dim, black);
        swatch_rows = i / SWATCH_COLUMNS + 1;
    }

    let y = BOTTOM + 2 + swatch_rows;
    b.print(
        1,
        y,
        "The quick brown fox jumps over the lazy dog. 0123456789 ÀÉÎÕÜ àéîõü ß ¿¡ «» ← ↑ → ↓",
        text,
        black,
    );
    sample_panels(&mut b, palette, y + 1);
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

    #[test]
    fn sampler_shows_every_glyph_and_colour() {
        let p = game_palette();
        let glyphs = atlas_glyphs();
        let b = glyph_sampler(&p, &glyphs);
        assert_eq!((b.width(), b.height()), (CONSOLE_W, CONSOLE_H));
        let row_end = 2 + 2 * i32::try_from(GLYPHS_PER_ROW).unwrap();
        let shown: String = (1..BOTTOM)
            .flat_map(|y| (2..row_end).step_by(2).map(move |x| (x, y)))
            .map(|(x, y)| b.get(x, y).unwrap().glyph)
            .collect();
        let expected: String = glyphs.iter().collect();
        assert!(
            shown.starts_with(&expected),
            "top half must list every glyph in order"
        );
        for (name, rgb) in p.iter() {
            let found = (BOTTOM..i32::from(CONSOLE_H)).any(|y| {
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
        let rows = atlas_glyphs().len().div_ceil(GLYPHS_PER_ROW);
        assert!(rows < usize::try_from(BOTTOM).unwrap());
        assert!(2 + 2 * GLYPHS_PER_ROW <= usize::from(CONSOLE_W));
        assert!(SWATCH_COLUMNS * SWATCH_W < i32::from(CONSOLE_W));
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
        assert_eq!(buf, glyph_sampler(p, &atlas_glyphs()));
        let frame = |a: &[Action]| FrameInput::new(a.to_vec(), 0.0, vec![]);
        let stay = screen.update(&mut ctx, &frame(&[Action::Confirm]));
        assert!(matches!(stay, Transition::None));
        let pop = screen.update(&mut ctx, &frame(&[Action::Confirm, Action::Cancel]));
        assert!(matches!(pop, Transition::Pop));
    }

    #[test]
    fn sampler_snapshot() {
        let p = game_palette();
        let snap = glyph_sampler(&p, &atlas_glyphs()).to_snapshot(&p);
        assert_snapshot!(snap);
    }
}
