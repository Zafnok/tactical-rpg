//! The battle map's view (ADR-0038): a screen says *what* is on the visible
//! map as a [`MapScene`] (plain data), and a [`MapSkin`] says how it looks.
//! The [`GlyphSkin`] is the game's; the [`SpriteSkin`] paints from a
//! tileset file and is reachable only from the debug menu.
//!
//! Anything new shown on the map (a village, a spell's flash) is added to
//! the scene and painted by every skin; it is never drawn straight into the
//! buffer by a screen.

pub mod glyph;
pub mod grid;
pub mod path;
pub mod scene;
pub mod skin;
pub mod sprite;

use std::rc::Rc;

use trpg_content::Content;

pub use glyph::GlyphSkin;
pub use scene::{CursorStyle, CursorView, MapScene, RangeKind, TileView, UnitView};
pub use skin::MapSkin;
pub use sprite::SpriteSkin;

/// The skin the game starts with: the [`GlyphSkin`] (ADR-0038: until Nick
/// decides otherwise).
pub fn default_skin() -> Rc<dyn MapSkin> {
    Rc::new(GlyphSkin)
}

/// The tileset the debug menu's sprite skin paints with: the generated test
/// tileset (`cargo xtask test-tileset`).
pub const TEST_TILESET: &str = "test";

/// The skin called `name` ([`MapSkin::name`]): `"glyph"`, or `"sprite"`
/// painting with the [`TEST_TILESET`] of `content`. `None` for any other
/// name, or without that tileset. For the debug menu and tests.
pub fn skin_named(content: &Content, name: &str) -> Option<Rc<dyn MapSkin>> {
    match name {
        "glyph" => Some(default_skin()),
        "sprite" => {
            let tileset = content.tilesets.get(TEST_TILESET)?.clone();
            Some(Rc::new(SpriteSkin::new(tileset)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skins_are_found_by_name() {
        let content = trpg_content::load_embedded().unwrap();
        assert_eq!(default_skin().name(), "glyph");
        assert_eq!(
            skin_named(&content, "glyph").map(|s| s.name()),
            Some("glyph")
        );
        assert_eq!(
            skin_named(&content, "sprite").map(|s| s.name()),
            Some("sprite")
        );
        assert!(skin_named(&content, "ascii").is_none());
        let mut bare = content;
        bare.tilesets.clear();
        assert!(skin_named(&bare, "sprite").is_none());
    }
}
