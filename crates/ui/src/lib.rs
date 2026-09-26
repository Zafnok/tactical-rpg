//! Glyph buffer, screens and input handling, independent of macroquad. See
//! ADR-0004, and `crates/ui/README.md` for how screens fit together.

pub mod color;
pub mod console;
pub mod debug;
pub mod game;
pub mod glyph_buffer;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod input;
pub mod screen;
pub mod screens;
pub mod snapshot;
pub mod storage;
pub mod widgets;

pub use color::{Palette, Rgb, UiColor};
pub use game::{FrameOutput, Game, RawKeyEvent};
pub use glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
pub use screen::{Ctx, FrameInput, LoadError, Screen, ScreenStack, Transition};
pub use storage::{MemoryStorage, Storage, StorageError};
