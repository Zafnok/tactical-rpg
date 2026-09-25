//! The game's screens. Debug-only screens live in [`crate::debug`].

pub mod title;

pub use title::{PlaceholderScreen, TitleScreen};

use crate::color::Rgb;
use crate::glyph_buffer::GlyphBuffer;

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
