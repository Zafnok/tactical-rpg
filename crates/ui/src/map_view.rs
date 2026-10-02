//! The battle map's view (ADR-0038): a screen says *what* is on the visible
//! map as a [`MapScene`] (plain data), and a [`MapSkin`] says how it looks.
//! The [`GlyphSkin`] is the only skin so far.
//!
//! Anything new shown on the map (a village, a spell's flash) is added to
//! the scene and painted by every skin; it is never drawn straight into the
//! buffer by a screen.

pub mod glyph;
pub mod scene;
pub mod skin;

use std::rc::Rc;

pub use glyph::GlyphSkin;
pub use scene::{CursorStyle, CursorView, MapScene, RangeKind, TileView, UnitView};
pub use skin::MapSkin;

/// The skin the game starts with: the [`GlyphSkin`] (ADR-0038: until Nick
/// decides otherwise).
pub fn default_skin() -> Rc<dyn MapSkin> {
    Rc::new(GlyphSkin)
}
