//! The battle map's view (ADR-0038): a screen says *what* is on the visible
//! map as a [`MapScene`] (plain data), and a [`MapSkin`] says how it looks.
//! The [`GlyphSkin`] is what a build without the bought art shows; a
//! [`SpriteSkin`] paints from a tileset file, and the game starts with the
//! one of the bought art when it has it (ADR-0049).
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
pub use scene::{CursorStyle, CursorView, MapScene, RangeKind, TileView, UnitEffects, UnitView};
pub use skin::MapSkin;
pub use sprite::SpriteSkin;

/// The tileset the game paints its battle maps with when it has it: the
/// bought art's, which only a build with the private assets has
/// (ADR-0040).
pub const GAME_TILESET: &str = "tiny_tales";

/// The generated test tileset with terrain tiles (`cargo xtask
/// test-tileset`).
pub const TEST_TILESET: &str = "test";

/// The generated test tileset of unit sheets, with no terrain tiles
/// (`cargo xtask test-tileset`).
pub const TEST_UNITS_TILESET: &str = "test_units";

/// The skin the game starts with: the [`SpriteSkin`] of the
/// [`GAME_TILESET`] if `content` has it, else the [`GlyphSkin`].
pub fn default_skin(content: &Content) -> Rc<dyn MapSkin> {
    skin_named(content, GAME_TILESET).unwrap_or_else(|| Rc::new(GlyphSkin))
}

/// The skin called `name`: `"glyph"`; `"sprite"` and `"sprite_units"`,
/// painting with the [`TEST_TILESET`] and the [`TEST_UNITS_TILESET`]; or
/// the id of any tileset of `content`. `None` for any other name, or
/// without that tileset. For the debug menu and tests.
pub fn skin_named(content: &Content, name: &str) -> Option<Rc<dyn MapSkin>> {
    let id = match name {
        "glyph" => return Some(Rc::new(GlyphSkin)),
        "sprite" => TEST_TILESET,
        "sprite_units" => TEST_UNITS_TILESET,
        id => id,
    };
    let tileset = content.tilesets.get(id)?.clone();
    Some(Rc::new(SpriteSkin::new(tileset)))
}

/// The skin after `current` in the debug menu's round: the glyph skin,
/// then each tileset of `content` by id, then the glyph skin again.
pub fn next_skin(content: &Content, current: &dyn MapSkin) -> Rc<dyn MapSkin> {
    let mut ids = content.tilesets.keys();
    if let Some(id) = current.tileset_id() {
        // Past the current one (all of them, if the content lacks it).
        ids.by_ref().take_while(|next| next.as_str() != id).count();
    }
    ids.next()
        .and_then(|id| skin_named(content, id))
        .unwrap_or_else(|| Rc::new(GlyphSkin))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skins_are_found_by_name() {
        let content = trpg_content::load_embedded().unwrap();
        let found = |name| skin_named(&content, name).map(|s| (s.name(), s.tileset_id().is_none()));
        assert_eq!(found("glyph"), Some(("glyph", true)));
        let tileset = |name| {
            let skin = skin_named(&content, name)?;
            Some((skin.name(), skin.tileset_id()?.to_owned()))
        };
        assert_eq!(tileset("sprite"), Some(("sprite", "test".to_owned())));
        assert_eq!(tileset("test"), tileset("sprite"));
        let units = Some(("sprite_units", "test_units".to_owned()));
        assert_eq!(tileset("sprite_units"), units);
        assert_eq!(tileset("test_units"), units);
        assert!(skin_named(&content, "ascii").is_none());
        let mut bare = content;
        bare.tilesets.clear();
        assert!(skin_named(&bare, "sprite").is_none());
        assert!(skin_named(&bare, "sprite_units").is_none());
        assert!(skin_named(&bare, "glyph").is_some());
    }

    #[test]
    fn the_game_starts_with_the_bought_arts_skin_only_when_it_has_it() {
        let mut content = trpg_content::load_embedded().unwrap();
        // A build without the private assets: glyphs.
        assert!(!content.tilesets.contains_key(GAME_TILESET));
        let skin = default_skin(&content);
        assert_eq!((skin.name(), skin.tileset_id()), ("glyph", None));
        // With a tileset of that id: its skin.
        let mut tileset = content.tilesets[TEST_UNITS_TILESET].clone();
        tileset.id = GAME_TILESET.to_owned();
        content.tilesets.insert(GAME_TILESET.to_owned(), tileset);
        let skin = default_skin(&content);
        assert_eq!(skin.name(), "sprite_units");
        assert_eq!(skin.tileset_id(), Some(GAME_TILESET));
    }

    #[test]
    fn the_debug_round_goes_through_every_tileset_and_back_to_glyphs() {
        let mut content = trpg_content::load_embedded().unwrap();
        let mut skin: Rc<dyn MapSkin> = Rc::new(GlyphSkin);
        let mut round = Vec::new();
        for _ in 0..4 {
            skin = next_skin(&content, skin.as_ref());
            round.push(skin.tileset_id().map(str::to_owned));
        }
        let id = |s: &str| Some(s.to_owned());
        assert_eq!(round, [id("test"), id("test_units"), None, id("test")]);
        // A skin whose tileset is gone: back to glyphs.
        content.tilesets.clear();
        assert_eq!(next_skin(&content, skin.as_ref()).name(), "glyph");
        assert_eq!(next_skin(&content, &GlyphSkin).name(), "glyph");
    }
}
