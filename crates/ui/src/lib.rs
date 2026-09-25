//! Glyph buffer, screens and input handling, independent of macroquad. See ADR-0004.

pub mod color;
pub mod console;
pub mod debug;
pub mod glyph_buffer;
pub mod input;
pub mod snapshot;

pub use color::{Palette, Rgb, UiColor};
pub use glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
