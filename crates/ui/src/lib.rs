//! Glyph buffer, screens and input handling, independent of macroquad. See
//! ADR-0004, and `crates/ui/README.md` for how screens fit together.

pub mod audio;
pub mod color;
pub mod console;
pub mod debug;
pub mod dialogue;
pub mod flow;
pub mod game;

pub mod glyph_buffer;
#[cfg(any(test, feature = "harness"))]
pub mod harness;
pub mod input;
pub mod map_view;
pub mod portrait;
pub mod screen;
pub mod screens;
pub mod snapshot;
pub mod storage;
pub mod tips;
pub mod widgets;

pub use audio::{AudioQueue, AudioRequest, MusicClock, MusicCommand, MusicState};
pub use color::{Palette, Rgb, UiColor};
pub use game::{FrameOutput, Game, RawInputEvent};
pub use glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Layer, Overlay, PxRect, Rect};
pub use screen::{Ctx, FrameInput, KeyPrompt, LoadError, Screen, ScreenStack, Transition};
pub use storage::{MemoryStorage, Storage, StorageError};
