//! The sprite test (debug builds, ticket 0231): the test card
//! (`assets/images/test_card.png`) drawn as sprite items in each way a
//! [`Sprite`] can be, for checking that a renderer draws them right
//! (ADR-0038).

use trpg_content::bundle::display_path;
use trpg_content::image::TEST_CARD_PATH;

use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::console::{CELL_H_PX, CELL_W_PX};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Layer, PxRect, Rect, Sprite};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{help_line, key_name};

/// Left column of the title, the help line and the first picture.
const LEFT: i32 = 2;
/// Rows of the first line of pictures: its labels, and the pictures' top.
const ROW_A: (i32, i32) = (2, 3);
/// The same for the second line.
const ROW_B: (i32, i32) = (11, 12);
/// The biggest scale shown: the 16×16 card as 80×80 pixels (10×5 cells).
const BIG: i32 = 5;
/// Opacity of the see-through picture: half.
const HALF: u8 = 128;
/// The lines printed over the picture drawn under the glyphs.
const OVER_TEXT: [&str; 5] = [
    "The glyphs",
    "are drawn",
    "over this",
    "picture.",
    "♣♠♥♦≈•▲▼◄►",
];

/// Shows the test card as sprites; `Cancel` closes.
#[derive(Debug, Clone, Copy)]
pub struct SpriteTestScreen;

impl SpriteTestScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "sprite_test";
}

/// The console pixel at the top-left of cell `(col, row)`.
fn cell_px(col: i32, row: i32) -> (i32, i32) {
    (col * i32::from(CELL_W_PX), row * i32::from(CELL_H_PX))
}

impl Screen for SpriteTestScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if input.actions.contains(&Action::Cancel) {
            ctx.audio.menu(MenuSound::Cancel);
            Transition::Pop
        } else {
            Transition::None
        }
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
        buf.print(LEFT, 0, "Sprite test (debug)", hi, black);
        let help = help_line(&[(Some(key_name(ctx.help_keys(), Action::Cancel)), "back")]);
        buf.print(LEFT, i32::from(buf.height()) - 1, &help, dim, black);

        let images = &ctx.content.images;
        let card = images.id(TEST_CARD_PATH);
        let Some((card, info)) = card.and_then(|id| Some((id, images.info(id)?))) else {
            let missing = format!("No {}", display_path(TEST_CARD_PATH));
            buf.print(LEFT, ROW_A.0, &missing, text, black);
            return;
        };
        let px = |v: u32| i32::try_from(v).unwrap_or(0);
        let (width, height) = (px(info.width), px(info.height));
        let whole = Rect::new(0, 0, width, height);
        // All of the card at `scale`, its top-left at cell `(col, row)`.
        let card_at = |col: i32, row: i32, scale: i32| {
            let (left, top) = cell_px(col, row);
            let dest: PxRect = Rect::new(left, top, width * scale, height * scale);
            Sprite::new(card, whole, dest, Layer::Over)
        };
        let label = |buf: &mut GlyphBuffer, col: i32, row: i32, name: &str| {
            buf.print(col, row, name, dim, black);
        };

        // First line: scales, a flip, a crop, and under the glyphs.
        let (labels, top) = ROW_A;
        for (col, scale) in [(LEFT, 1), (8, 3), (18, BIG)] {
            label(buf, col, labels, &format!("{scale}x"));
            buf.add_sprite(card_at(col, top, scale));
        }
        label(buf, 32, labels, "flipped");
        buf.add_sprite(Sprite {
            flip_x: true,
            ..card_at(32, top, BIG)
        });
        label(buf, 46, labels, "top-right quarter");
        buf.add_sprite(Sprite {
            src: Rect::new(width / 2, 0, width / 2, height / 2),
            ..card_at(46, top, BIG)
        });
        label(buf, 66, labels, "under the glyphs");
        buf.add_sprite(Sprite {
            layer: Layer::Under,
            ..card_at(66, top, BIG)
        });
        for (row, line) in (top..).zip(OVER_TEXT) {
            buf.print_fg(66, row, line, black);
        }

        // Second line: see-through over a panel, and cut by a later box.
        let (labels, top) = ROW_B;
        let panel_bg = c(UiColor::PanelBg);
        let panel = Rect::new(LEFT, top, 14, 7);
        label(buf, LEFT, labels, "half opacity");
        buf.fill_rect(panel, Cell::new(' ', text, panel_bg));
        buf.draw_box(panel, BoxStyle::Single, c(UiColor::PanelBorder), panel_bg);
        buf.add_sprite(Sprite {
            opacity: HALF,
            // Inside the panel's border, a cell clear of it on each side.
            ..card_at(4, top + 1, BIG)
        });
        label(buf, 22, labels, "cut by a box drawn after it");
        buf.add_sprite(card_at(22, top + 1, BIG));
        // Over the picture's right half, from its third row down.
        let text_box = Rect::new(27, top + 3, 16, 4);
        buf.fill_rect(text_box, Cell::new(' ', text, panel_bg));
        buf.draw_box(
            text_box,
            BoxStyle::Double,
            c(UiColor::PanelBorderFocus),
            panel_bg,
        );
        buf.print(29, top + 4, "A text box", text, panel_bg);
        buf.print(29, top + 5, "drawn later", text, panel_bg);
    }
}

#[cfg(test)]
mod tests {
    use trpg_content::ImageId;

    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::screen::tests::ctx;

    fn draw(ctx: &Ctx) -> GlyphBuffer {
        let p = &ctx.palette;
        let mut buf = GlyphBuffer::new(
            CONSOLE_W,
            CONSOLE_H,
            Cell::new('x', p.get(UiColor::Text), p.get(UiColor::PanelBg)),
        );
        SpriteTestScreen.draw(ctx, &mut buf);
        buf
    }

    fn card(ctx: &Ctx) -> ImageId {
        ctx.content.images.id(TEST_CARD_PATH).unwrap()
    }

    #[test]
    fn cell_px_is_the_cells_top_left_pixel() {
        assert_eq!(cell_px(0, 0), (0, 0));
        assert_eq!(cell_px(3, 2), (24, 32));
    }

    #[test]
    fn shows_the_card_in_every_way_a_sprite_can_be_drawn() {
        let ctx = ctx();
        let buf = draw(&ctx);
        let whole = Rect::new(0, 0, 16, 16);
        let at =
            |x, y, side| Sprite::new(card(&ctx), whole, Rect::new(x, y, side, side), Layer::Over);
        let cut = at(176, 208, 80);
        assert_eq!(
            buf.sprites(),
            [
                at(16, 48, 16),
                at(64, 48, 48),
                at(144, 48, 80),
                Sprite {
                    flip_x: true,
                    ..at(256, 48, 80)
                },
                Sprite {
                    src: Rect::new(8, 0, 8, 8),
                    ..at(368, 48, 80)
                },
                Sprite {
                    layer: Layer::Under,
                    ..at(528, 48, 80)
                },
                Sprite {
                    opacity: 128,
                    ..at(32, 208, 80)
                },
                // Cut by the text box: the rows above it, and the left half.
                Sprite {
                    clip: Rect::new(176, 208, 80, 32),
                    ..cut
                },
                Sprite {
                    clip: Rect::new(176, 240, 40, 48),
                    ..cut
                },
            ]
        );
        // Only sprites: no rectangles stand in for a picture.
        assert!(buf.overlays().is_empty());
    }

    #[test]
    fn glyphs_sit_on_the_picture_drawn_under_them() {
        let ctx = ctx();
        let buf = draw(&ctx);
        let black = ctx.palette.get(UiColor::Black);
        for (row, line) in (3..).zip(OVER_TEXT) {
            assert!(line.chars().count() <= 10, "{line} is wider than the card");
            for (col, glyph) in (66..).zip(line.chars()) {
                assert_eq!(buf.get(col, row), Some(&Cell::new(glyph, black, black)));
            }
        }
    }

    #[test]
    fn says_so_when_the_card_is_missing() {
        let mut ctx = ctx();
        ctx.content.images.images.clear();
        let buf = draw(&ctx);
        assert_eq!(buf.items(), []);
        let row: String = (0..40).map(|x| buf.get(x, 2).unwrap().glyph).collect();
        assert_eq!(row.trim(), "No assets/images/test_card.png");
    }

    #[test]
    fn cancel_closes_with_the_cancel_sound() {
        let mut ctx = ctx();
        let mut screen = SpriteTestScreen;
        assert_eq!(screen.name(), "sprite_test");
        let frame = |a: &[Action]| FrameInput::new(a.to_vec(), 0.0, vec![]);
        let stay = screen.update(&mut ctx, &frame(&[Action::Confirm, Action::CursorDown]));
        assert!(matches!(stay, Transition::None));
        assert!(ctx.audio.pending().is_empty());
        let pop = screen.update(&mut ctx, &frame(&[Action::Confirm, Action::Cancel]));
        assert!(matches!(pop, Transition::Pop));
        let sounds = ctx.audio.take();
        let cues: Vec<_> = sounds.iter().map(|s| s.cue()).collect();
        assert_eq!(cues, [Some("menu_cancel")]);
    }
}
