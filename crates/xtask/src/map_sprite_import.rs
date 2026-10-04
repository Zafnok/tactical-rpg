//! `cargo xtask map-sprite-import`: copies the bought map sprites the game
//! uses into `assets-private/game/units/` under stable names and writes the
//! tileset that names them, `assets-private/game/tilesets/tiny_tales.ron`
//! (ticket 0436, ADR-0049).
//!
//! A bought map sprite is one 48×80 sheet per character or class: 3
//! columns (walking frames) × 4 rows (facing down, left, right, up) of
//! 16×20 frames. The tileset names each one's standing, front-facing
//! frame, column 1 of row 0. Which sprite each class and character gets is
//! the table [`SPRITES`] (`docs/design/look-and-feel.md` has it in words).
//!
//! The output is bought art (ADR-0040): it goes to the private assets
//! checkout, never into this repository.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use trpg_content::image::png_size;

/// Shown for `--help` and after a bad argument.
pub const USAGE: &str = "usage: cargo xtask map-sprite-import [--list]\n\n\
Copies each bought map sprite the game uses from\n\
assets-private/library/tiny-tales/characters/ to assets-private/game/units/<name>.png\n\
and writes assets-private/game/tilesets/tiny_tales.ron, which names them.\n\
--list  print which sprite each class and character gets, and copy nothing\n\n\
Run `cargo xtask private-assets --library` first. Afterwards commit and push in\n\
assets-private/, then run `cargo xtask private-assets --pin`.";

/// The sorted bought characters, relative to the repo root.
pub const LIBRARY_DIR: &str = "assets-private/library/tiny-tales/characters";
/// The private assets the game is built with, relative to the repo root.
pub const GAME_DIR: &str = "assets-private/game";
/// The tileset's id (`trpg_ui::map_view::GAME_TILESET`).
pub const TILESET_ID: &str = "tiny_tales";
/// A bought sheet's size in pixels.
pub const SHEET: (u32, u32) = (48, 80);
/// A frame's size in pixels.
pub const FRAME: (u32, u32) = (16, 20);
/// The standing, front-facing frame.
pub const STANDING: (u32, u32) = (1, 0);

/// Who a sprite is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum For {
    /// A character id (or the lead's `lead_m` / `lead_f`).
    Character(&'static str),
    /// A class id.
    Class(&'static str),
    /// Any unit with neither.
    Fallback,
}

/// One bought sprite the game uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapSprite {
    /// Who it is for.
    pub who: For,
    /// Its file name in `game/units/`, without `.png`.
    pub name: &'static str,
    /// The bought sheet, relative to [`LIBRARY_DIR`].
    pub source: &'static str,
}

const fn sprite(who: For, name: &'static str, source: &'static str) -> MapSprite {
    MapSprite { who, name, source }
}

/// Every class and named character of the Quick Battle and Chapter 1, and
/// the bought sprite each is drawn as. Two rows may share a sprite.
pub const SPRITES: [MapSprite; 13] = [
    // The lead, by gender; the Exile class (the placeholder lord) as the
    // male lead.
    sprite(
        For::Character("lead_m"),
        "fighter_male",
        "heroes/FighterMale/map_sprite.png",
    ),
    sprite(
        For::Character("lead_f"),
        "fighter_female",
        "heroes/FighterFemale/map_sprite.png",
    ),
    sprite(
        For::Class("exile"),
        "fighter_male",
        "heroes/FighterMale/map_sprite.png",
    ),
    sprite(For::Class("mage"), "witch", "heroes/Witch/map_sprite.png"),
    sprite(
        For::Class("archer"),
        "archer",
        "heroes/Archer/map_sprite.png",
    ),
    sprite(
        For::Class("guard"),
        "church_knight",
        "battler-classes/faith-and-evil/Human_Church_6__Church_Knight/map_sprite.png",
    ),
    sprite(
        For::Class("cleric"),
        "church_cleric",
        "battler-classes/faith-and-evil/Human_Church_1__Church_Cleric/map_sprite.png",
    ),
    // Nothing mounted exists in the bundle: a knight on foot until ticket
    // 0040 finds mounted art.
    sprite(
        For::Class("rider"),
        "knight_m1",
        "map-sprites-only/human-knights/Knight_M1_A.png",
    ),
    sprite(
        For::Class("brigand"),
        "warrior_m1",
        "map-sprites-only/human-advanced/Warrior_M1.png",
    ),
    sprite(
        For::Class("raider"),
        "fighter_m1",
        "map-sprites-only/human-advanced/Fighter_M1.png",
    ),
    sprite(
        For::Class("fire_elemental"),
        "fire_elemental",
        "battler-classes/elemental-forces/Fire_Elemental/map_sprite.png",
    ),
    sprite(
        For::Class("frost_elemental"),
        "ice_elemental",
        "battler-classes/elemental-forces/Ice_Elemental/map_sprite.png",
    ),
    sprite(
        For::Fallback,
        "adventurer_m1",
        "map-sprites-only/human-advanced/Adventurer_M1.png",
    ),
];

/// The bundle path of sprite `name`.
fn bundle_path(name: &str) -> String {
    format!("units/{name}.png")
}

/// The tileset file naming `sprites`: no terrain tiles (the glyph skin
/// paints the ground until ticket 0437), each unit its standing frame.
pub fn tileset(sprites: &[MapSprite]) -> String {
    let entry = |s: &MapSprite| {
        let (path, (column, row)) = (bundle_path(s.name), STANDING);
        format!("(image: \"{path}\", frame: ({column}, {row}))")
    };
    let table = |pick: fn(For) -> Option<&'static str>| {
        let mut rows = String::new();
        for s in sprites {
            if let Some(id) = pick(s.who) {
                // Writing to a `String` can't fail.
                let _ = writeln!(rows, "            \"{id}\": {},", entry(s));
            }
        }
        rows
    };
    let characters = table(|who| match who {
        For::Character(id) => Some(id),
        _ => None,
    });
    let classes = table(|who| match who {
        For::Class(id) => Some(id),
        _ => None,
    });
    let fallback = sprites.iter().find(|s| s.who == For::Fallback);
    let fallback = fallback.map(entry).unwrap_or_default();
    format!(
        "// Written by `cargo xtask map-sprite-import` (ticket 0436). Do not edit: change\n\
         // the table in crates/xtask/src/map_sprite_import.rs and run it again.\n\
         (\n    id: \"{TILESET_ID}\",\n    unit_px: ({}, {}),\n    units: (\n        \
         characters: {{\n{characters}        }},\n        classes: {{\n{classes}        }},\n        \
         fallback: {fallback},\n    ),\n)\n",
        FRAME.0, FRAME.1
    )
}

/// Which sprite each class and character gets, one per line.
pub fn list(sprites: &[MapSprite]) -> String {
    let mut out = String::new();
    for s in sprites {
        let who = match s.who {
            For::Character(id) => format!("character {id}"),
            For::Class(id) => format!("class {id}"),
            For::Fallback => "any other unit".to_owned(),
        };
        let _ = writeln!(out, "{who:<24} {:<28} {}", bundle_path(s.name), s.source);
    }
    out
}

/// Runs the command under repo root `root`: copies every sprite of
/// `sprites` and writes the tileset. Every source is checked before any
/// file is written.
pub fn run(root: &Path, sprites: &[MapSprite]) -> Result<String, String> {
    let library = root.join(LIBRARY_DIR);
    if !library.is_dir() || !root.join(GAME_DIR).is_dir() {
        return Err(format!(
            "{LIBRARY_DIR}/ or {GAME_DIR}/ is missing: run `cargo xtask private-assets --library` first"
        ));
    }
    let mut files: Vec<(&str, Vec<u8>)> = Vec::new();
    for s in sprites {
        let path = library.join(s.source);
        let bytes = fs::read(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
        if png_size(&bytes) != Some(SHEET) {
            return Err(format!(
                "{} is not a {}×{} px map sprite sheet",
                path.display(),
                SHEET.0,
                SHEET.1
            ));
        }
        match files.iter().find(|(name, _)| *name == s.name) {
            Some((_, other)) if *other != bytes => {
                return Err(format!("two different sprites are named {}", s.name));
            }
            Some(_) => {}
            None => files.push((s.name, bytes)),
        }
    }
    let game = root.join(GAME_DIR);
    let write = |path: std::path::PathBuf, bytes: &[u8]| {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
        }
        fs::write(&path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))
    };
    for (name, bytes) in &files {
        write(game.join(bundle_path(name)), bytes)?;
    }
    let tileset_path = format!("tilesets/{TILESET_ID}.ron");
    write(game.join(&tileset_path), tileset(sprites).as_bytes())?;
    Ok(format!(
        "map-sprite-import: wrote {} sprites to {GAME_DIR}/units/ and {GAME_DIR}/{tileset_path}",
        files.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The start of a `w × h` PNG file: enough for `png_size`.
    fn png_header(w: u32, h: u32) -> Vec<u8> {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend(w.to_be_bytes());
        png.extend(h.to_be_bytes());
        png
    }

    const TWO: [MapSprite; 4] = [
        sprite(For::Character("lead_f"), "b", "heroes/B/map_sprite.png"),
        sprite(For::Class("exile"), "a", "heroes/A/map_sprite.png"),
        sprite(For::Class("mage"), "b", "heroes/B/map_sprite.png"),
        sprite(For::Fallback, "a", "heroes/A/map_sprite.png"),
    ];

    /// A repo root with a library holding sheets `A` and `B`.
    fn root(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("xtask-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (hero, tail) in [("A", 1u8), ("B", 2)] {
            let dir = root.join(LIBRARY_DIR).join("heroes").join(hero);
            fs::create_dir_all(&dir).unwrap();
            let mut png = png_header(48, 80);
            png.push(tail);
            fs::write(dir.join("map_sprite.png"), png).unwrap();
        }
        fs::create_dir_all(root.join(GAME_DIR)).unwrap();
        root
    }

    #[test]
    fn the_tileset_names_each_units_standing_frame() {
        assert_eq!(
            tileset(&TWO),
            "// Written by `cargo xtask map-sprite-import` (ticket 0436). Do not edit: change\n\
             // the table in crates/xtask/src/map_sprite_import.rs and run it again.\n\
             (\n    id: \"tiny_tales\",\n    unit_px: (16, 20),\n    units: (\n        \
             characters: {\n            \
             \"lead_f\": (image: \"units/b.png\", frame: (1, 0)),\n        },\n        \
             classes: {\n            \
             \"exile\": (image: \"units/a.png\", frame: (1, 0)),\n            \
             \"mage\": (image: \"units/b.png\", frame: (1, 0)),\n        },\n        \
             fallback: (image: \"units/a.png\", frame: (1, 0)),\n    ),\n)\n"
        );
    }

    #[test]
    fn the_games_table_names_real_ids_once_each_and_one_fallback() {
        let content = trpg_content::load_embedded().unwrap();
        let mut seen = Vec::new();
        for s in SPRITES {
            assert!(!seen.contains(&s.who), "{:?} twice", s.who);
            seen.push(s.who);
            match s.who {
                For::Class(id) => {
                    let class = trpg_core::ClassId(id.to_owned());
                    assert!(content.classes.get(&class).is_some(), "class {id}");
                }
                For::Character(id) => {
                    assert!(trpg_content::tileset::LEAD_PICTURES.contains(&id), "{id}");
                }
                For::Fallback => {}
            }
            let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_';
            assert!(s.name.chars().all(valid), "{}", s.name);
            // One name, one source.
            let same = SPRITES.iter().filter(|o| o.name == s.name);
            assert!(same.clone().all(|o| o.source == s.source), "{}", s.name);
        }
        assert!(seen.contains(&For::Fallback));
        // Every class of the Quick Battle has a sprite of its own.
        let state = trpg_ui::screens::battle::quick_battle(&content).unwrap();
        for unit in state.units() {
            let id = unit.class.0.as_str();
            let has = |who: &For| matches!(who, For::Class(class) if *class == id);
            assert!(seen.iter().any(has), "class {id}");
        }
    }

    #[test]
    fn the_list_says_who_gets_which_sprite() {
        let text = list(&TWO);
        assert_eq!(text.lines().count(), 4);
        assert!(text.starts_with("character lead_f         units/b.png"));
        assert!(text.contains("class exile              units/a.png"));
        assert!(text.contains("any other unit           units/a.png"));
        assert!(text.ends_with("heroes/A/map_sprite.png\n"));
    }

    #[test]
    fn run_copies_each_sprite_once_and_writes_the_tileset() {
        let root = root("map-sprite-import");
        let summary = run(&root, &TWO).unwrap();
        assert_eq!(
            summary,
            "map-sprite-import: wrote 2 sprites to assets-private/game/units/ and \
             assets-private/game/tilesets/tiny_tales.ron"
        );
        let game = root.join(GAME_DIR);
        let read = |path: &str| fs::read(game.join(path)).unwrap();
        assert_eq!(read("units/a.png").last(), Some(&1));
        assert_eq!(read("units/b.png").last(), Some(&2));
        assert_eq!(read("tilesets/tiny_tales.ron"), tileset(&TWO).into_bytes());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_checkout_or_a_bad_sheet_writes_nothing() {
        let bare = std::env::temp_dir().join(format!("xtask-msi-bare-{}", std::process::id()));
        let err = run(&bare, &TWO).unwrap_err();
        assert!(
            err.contains("cargo xtask private-assets --library"),
            "{err}"
        );
        let root = root("map-sprite-import-bad");
        let game = root.join(GAME_DIR);
        // A sheet that isn't there.
        let missing = [sprite(For::Fallback, "c", "heroes/C/map_sprite.png")];
        let err = run(&root, &missing).unwrap_err();
        assert!(err.starts_with("reading "), "{err}");
        // A file of another size.
        let sheet = root.join(LIBRARY_DIR).join("heroes/B/map_sprite.png");
        fs::write(&sheet, png_header(80, 80)).unwrap();
        let err = run(&root, &TWO).unwrap_err();
        assert!(err.ends_with("is not a 48×80 px map sprite sheet"), "{err}");
        // One name for two different sheets.
        fs::write(&sheet, [png_header(48, 80), vec![9]].concat()).unwrap();
        let clash = [
            TWO[1],
            sprite(For::Fallback, "a", "heroes/B/map_sprite.png"),
        ];
        let err = run(&root, &clash).unwrap_err();
        assert_eq!(err, "two different sprites are named a");
        assert_eq!(fs::read_dir(&game).unwrap().count(), 0);
        // Without game/, even with the library.
        fs::remove_dir_all(&game).unwrap();
        assert!(run(&root, &TWO).is_err());
        // A file where the units folder goes fails the write.
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("units"), "x").unwrap();
        let err = run(&root, &[TWO[1], TWO[3]]).unwrap_err();
        assert!(err.starts_with("creating "), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }
}
