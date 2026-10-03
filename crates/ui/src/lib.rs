//! Glyph buffer, screens and input handling, independent of macroquad. See
//! ADR-0004, and `crates/ui/README.md` for how screens fit together.

pub mod audio;
pub mod cinema;
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
pub mod save;
pub mod screen;
pub mod screens;
pub mod snapshot;
pub mod storage;
pub mod tips;
pub mod widgets;

pub use audio::{AudioQueue, AudioRequest, MusicClock, MusicCommand, MusicState};
pub use color::{Palette, Rgb, UiColor};
pub use game::{FrameOutput, Game, RawInputEvent};
pub use glyph_buffer::{
    Backdrop, BoxStyle, Cell, GlyphBuffer, Item, Layer, Overlay, Paint, PxRect, Rect, Sprite,
};
pub use screen::{Ctx, FrameInput, KeyPrompt, LoadError, Screen, ScreenStack, Transition};
pub use storage::{MemoryStorage, Storage, StorageError};
pub use trpg_content::ImageId;
