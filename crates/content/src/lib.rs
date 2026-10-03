//! Data schemas, parsers and validation for game content. See ADR-0004 and
//! ADR-0005. All content comes from the embedded asset bundle ([`bundle`]);
//! [`load_embedded`] loads and validates everything, reporting every error.

pub mod ai;
pub mod art;
pub mod audio;
pub mod battle;
pub mod bundle;
pub mod chapter;
pub mod character;
pub mod class;
pub mod credits;
pub mod dialogue;
mod enums;
pub mod error;
pub mod font;
pub mod image;
pub mod item;
pub mod keymap;
pub mod map;
pub mod names;
pub mod palette;
pub mod portrait;
pub mod ron_loader;
pub mod skill;
pub mod spell;
pub mod terrain;
pub mod tileset;
pub mod tip;
pub mod trigger;

use std::collections::BTreeMap;

use std::sync::Arc;

use trpg_core::{
    AiWeights, ArtTable, BattleDef, ClassTable, GameTables, ItemTable, SkillTable, SpellTable,
};

pub use audio::{AudioManifest, Credit, CreditRef, MusicCue, SoundCue};
pub use battle::BattleRefs;
pub use chapter::{ChapterDef, NewGameDef, battle_campaign, new_campaign};
pub use character::{CharacterTable, GenericTemplate, character_unit, check_map_labels};
pub use credits::{CreditEntry, CreditGroup, Credits};
pub use dialogue::{ChoiceOption, DialogueTable, MusicLine, Scene, Side, Step};
pub use error::{ContentError, ContentErrors};
pub use font::FontAtlasDef;
pub use image::{ImageId, ImageInfo, ImageTable};
pub use keymap::{
    Action, Bindings, Button, Chord, Key, KeymapDef, Layout, LayoutKeys, PadKeys, RepeatDef, SLOTS,
    StickDef,
};
pub use map::{MapDef, MapLegend};
pub use names::Names;
pub use palette::PaletteDef;
pub use portrait::Portrait;
pub use terrain::{TerrainDef, TerrainDisplay, TerrainDisplayTable};
pub use tileset::{ImageRect, Picture, Tileset};
pub use tip::{Tip, TipTable, TipTrigger};
pub use trigger::check_triggers;

/// All validated game content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// Named colours (ADR-0012).
    pub palette: PaletteDef,
    /// Key bindings for every layout, and repeat timings (ADR-0015).
    pub keymap: KeymapDef,
    /// Font atlas layout; the image is `bundle::bytes(font::ATLAS_PNG_PATH)`.
    pub font: FontAtlasDef,
    /// Every other image in the bundle, by path, with its size (ADR-0038).
    pub images: ImageTable,
    /// What sprite map skins paint battle maps with, by id (ADR-0038).
    pub tilesets: BTreeMap<String, Tileset>,
    /// Terrain rules and looks.
    pub terrain: TerrainDef,
    /// Battle maps by id (file stem).
    pub maps: BTreeMap<String, MapDef>,
    /// The class tree and progression tables.
    pub classes: ClassTable,
    /// Items and item rules.
    pub items: ItemTable,
    /// Spells.
    pub spells: SpellTable,
    /// Class skills.
    pub skills: SkillTable,
    /// Combat Arts.
    pub arts: ArtTable,
    /// Display names by name id (ticket 0709).
    pub names: Names,
    /// Named characters and generic unit templates.
    pub characters: CharacterTable,
    /// Character portraits by character id (file stem).
    pub portraits: BTreeMap<String, Portrait>,
    /// Dialogue scenes by id.
    pub dialogue: DialogueTable,
    /// The AI's numbers.
    pub ai: AiWeights,
    /// One-time contextual tips.
    pub tips: TipTable,
    /// Sound and music cues, pools and credits (ADR-0026).
    pub audio: AudioManifest,
    /// Every third-party work, for the credits screen (ticket 0808).
    pub credits: Credits,
    /// Battles by id (file stem).
    pub battles: BTreeMap<String, BattleDef>,
    /// Chapters by id (file stem).
    pub chapters: BTreeMap<String, ChapterDef>,
    /// How a new game starts.
    pub new_game: NewGameDef,
}

impl Content {
    /// The tables battles read, shared.
    pub fn tables(&self) -> GameTables {
        GameTables {
            terrain: Arc::new(self.terrain.rules.clone()),
            classes: Arc::new(self.classes.clone()),
            items: Arc::new(self.items.clone()),
            spells: Arc::new(self.spells.clone()),
            skills: Arc::new(self.skills.clone()),
            arts: Arc::new(self.arts.clone()),
        }
    }
}

/// Loads and validates every content type from the embedded bundle. Runs all
/// loaders and returns every error found, not just the first. Checks that
/// depend on another file (terrain colours, map legends) are skipped when that
/// file failed, so one broken file doesn't flood the report.
pub fn load_embedded() -> Result<Content, ContentErrors> {
    let palette = PaletteDef::load();
    let terrain = TerrainDef::load(palette.as_ref().ok());
    let maps = match &terrain {
        Ok(t) => map::load_all(&t.display),
        Err(_) => Ok(BTreeMap::new()),
    };
    let classes = class::load(
        terrain
            .as_ref()
            .ok()
            .map(|t| t.rules.movement_types.as_slice()),
    );
    let items = check_seals(item::load(), classes.as_ref().ok());
    let maps = check_map_features(maps, items.as_ref().ok(), terrain.as_ref().ok());
    let names = names::load();
    let characters = character::load(
        classes.as_ref().ok(),
        items.as_ref().ok(),
        names.as_ref().ok(),
    );
    let spells = check_spell_references(
        spell::load(terrain.as_ref().ok().map(|t| &t.display)),
        classes.as_ref().ok(),
        characters.as_ref().ok(),
    );
    let skills = check_skill_references(skill::load(), classes.as_ref().ok());
    let arts = check_art_references(art::load(), items.as_ref().ok());
    let portraits = match &palette {
        Ok(p) => portrait::load_all(p),
        Err(_) => Ok(BTreeMap::new()),
    };
    let audio = audio::load();
    let dialogue = dialogue::load(
        characters.as_ref().ok(),
        portraits.as_ref().ok(),
        names.as_ref().ok(),
        audio.as_ref().ok(),
    );
    let (battles, chapters, new_game) = load_story(
        maps.as_ref().ok(),
        terrain.as_ref().ok(),
        classes.as_ref().ok(),
        items.as_ref().ok(),
        characters.as_ref().ok(),
        dialogue.as_ref().ok(),
        audio.as_ref().ok(),
    );
    let credits = credits::load(audio.as_ref().ok());
    let images = ImageTable::load();
    let tilesets = load_tilesets(
        images.as_ref().ok(),
        terrain.as_ref().ok(),
        classes.as_ref().ok(),
        characters.as_ref().ok(),
    );
    assemble(
        palette,
        KeymapDef::load(),
        FontAtlasDef::load(),
        terrain,
        maps,
        Loaded {
            classes,
            items,
            spells,
            skills,
            arts,
            names,
            characters,
            portraits,
            dialogue,
            ai: ai::load(),
            tips: tip::load(),
            audio,
            credits,
            images,
            tilesets,
            battles,
            chapters,
            new_game,
        },
    )
}

/// Adds the seal checks ([`item::check_seals`]: a seal for every tier a
/// class promotes into, and a Reclass Seal) to the items' result. Skipped
/// when the items or classes failed to load.
fn check_seals(
    items: Result<ItemTable, Vec<ContentError>>,
    classes: Option<&ClassTable>,
) -> Result<ItemTable, Vec<ContentError>> {
    match (items, classes) {
        (Ok(items), Some(classes)) => {
            let errors = item::check_seals(&items, classes);
            if errors.is_empty() {
                Ok(items)
            } else {
                Err(errors)
            }
        }
        (items, _) => items,
    }
}

/// Loads the tilesets, checked against the images, terrain, classes and
/// characters. Skipped (none, no errors) when one of those failed.
fn load_tilesets(
    images: Option<&ImageTable>,
    terrain: Option<&TerrainDef>,
    classes: Option<&ClassTable>,
    characters: Option<&CharacterTable>,
) -> Result<BTreeMap<String, Tileset>, Vec<ContentError>> {
    let (Some(images), Some(terrain), Some(classes), Some(characters)) =
        (images, terrain, classes, characters)
    else {
        return Ok(BTreeMap::new());
    };
    tileset::load_all(&tileset::TilesetRefs {
        images,
        terrain: &terrain.display,
        classes,
        characters,
    })
}

/// Loader results for the battles, chapters and New Game file.
type Story = (
    Result<BTreeMap<String, BattleDef>, Vec<ContentError>>,
    Result<BTreeMap<String, ChapterDef>, Vec<ContentError>>,
    Result<NewGameDef, Vec<ContentError>>,
);

/// Loads the battles, then the chapters, then the New Game file, each
/// checked against what it refers to. Each is skipped (empty, no errors)
/// when a file it depends on failed, so one broken file doesn't flood the
/// report.
fn load_story(
    maps: Option<&BTreeMap<String, MapDef>>,
    terrain: Option<&TerrainDef>,
    classes: Option<&ClassTable>,
    items: Option<&ItemTable>,
    characters: Option<&CharacterTable>,
    dialogue: Option<&DialogueTable>,
    audio: Option<&AudioManifest>,
) -> Story {
    let (
        Some(maps),
        Some(terrain),
        Some(classes),
        Some(items),
        Some(characters),
        Some(dialogue),
        Some(audio),
    ) = (maps, terrain, classes, items, characters, dialogue, audio)
    else {
        return (
            Ok(BTreeMap::new()),
            Ok(BTreeMap::new()),
            Ok(NewGameDef::default()),
        );
    };
    let refs = BattleRefs {
        maps,
        terrain: &terrain.rules,
        classes,
        items,
        characters,
        dialogue,
        audio,
    };
    let battles = battle::load_all(&refs);
    let Ok(loaded) = &battles else {
        return (battles, Ok(BTreeMap::new()), Ok(NewGameDef::default()));
    };
    let chapters = chapter::load_all(loaded, dialogue);
    let new_game = match &chapters {
        Ok(chapters) => chapter::load_new_game(chapters, characters, items),
        Err(_) => Ok(NewGameDef::default()),
    };
    (battles, chapters, new_game)
}

/// Adds the art reference checks ([`art::check_references`]: every
/// weapon's arts exist, are weapon arts and match its kind) to the arts'
/// result. Skipped when the arts or items failed to load.
fn check_art_references(
    arts: Result<ArtTable, Vec<ContentError>>,
    items: Option<&ItemTable>,
) -> Result<ArtTable, Vec<ContentError>> {
    match (arts, items) {
        (Ok(arts), Some(items)) => {
            let errors = art::check_references(&arts, items);
            if errors.is_empty() {
                Ok(arts)
            } else {
                Err(errors)
            }
        }
        (arts, _) => arts,
    }
}

/// Adds the skill reference checks ([`skill::check_references`]: every
/// class active and passive exists, of the right kind) to the skills'
/// result. Skipped when the skills or classes failed to load.
fn check_skill_references(
    skills: Result<SkillTable, Vec<ContentError>>,
    classes: Option<&ClassTable>,
) -> Result<SkillTable, Vec<ContentError>> {
    match (skills, classes) {
        (Ok(skills), Some(classes)) => {
            let errors = skill::check_references(&skills, classes);
            if errors.is_empty() {
                Ok(skills)
            } else {
                Err(errors)
            }
        }
        (skills, _) => skills,
    }
}

/// Adds the spell reference checks ([`spell::check_references`]: every class
/// and personal spell exists) to the spells' result. Skipped when the
/// spells, classes or characters failed to load.
fn check_spell_references(
    spells: Result<SpellTable, Vec<ContentError>>,
    classes: Option<&ClassTable>,
    characters: Option<&CharacterTable>,
) -> Result<SpellTable, Vec<ContentError>> {
    match (spells, classes, characters) {
        (Ok(spells), Some(classes), Some(characters)) => {
            let errors = spell::check_references(&spells, classes, characters);
            if errors.is_empty() {
                Ok(spells)
            } else {
                Err(errors)
            }
        }
        (spells, ..) => spells,
    }
}

/// Adds the map feature checks ([`map::check_features`]) to the maps'
/// result. Skipped when the maps, items or terrain failed to load.
fn check_map_features(
    maps: Result<BTreeMap<String, MapDef>, Vec<ContentError>>,
    items: Option<&ItemTable>,
    terrain: Option<&TerrainDef>,
) -> Result<BTreeMap<String, MapDef>, Vec<ContentError>> {
    match (maps, items, terrain) {
        (Ok(maps), Some(items), Some(terrain)) => {
            let errors = map::check_features(&maps, items, &terrain.rules);
            if errors.is_empty() {
                Ok(maps)
            } else {
                Err(errors)
            }
        }
        (maps, ..) => maps,
    }
}

/// Loader results for the unit data (kept together to keep [`assemble`]'s
/// argument list short).
struct Loaded {
    classes: Result<ClassTable, Vec<ContentError>>,
    items: Result<ItemTable, Vec<ContentError>>,
    spells: Result<SpellTable, Vec<ContentError>>,
    skills: Result<SkillTable, Vec<ContentError>>,
    arts: Result<ArtTable, Vec<ContentError>>,
    names: Result<Names, Vec<ContentError>>,
    characters: Result<CharacterTable, Vec<ContentError>>,
    portraits: Result<BTreeMap<String, Portrait>, Vec<ContentError>>,
    dialogue: Result<DialogueTable, Vec<ContentError>>,
    ai: Result<AiWeights, Vec<ContentError>>,
    tips: Result<TipTable, Vec<ContentError>>,
    audio: Result<AudioManifest, Vec<ContentError>>,
    credits: Result<Credits, Vec<ContentError>>,
    images: Result<ImageTable, Vec<ContentError>>,
    tilesets: Result<BTreeMap<String, Tileset>, Vec<ContentError>>,
    battles: Result<BTreeMap<String, BattleDef>, Vec<ContentError>>,
    chapters: Result<BTreeMap<String, ChapterDef>, Vec<ContentError>>,
    new_game: Result<NewGameDef, Vec<ContentError>>,
}

/// Takes a loader's value, or moves its errors into `errors` and returns a
/// default.
fn take<T: Default>(result: Result<T, Vec<ContentError>>, errors: &mut Vec<ContentError>) -> T {
    result.unwrap_or_else(|e| {
        errors.extend(e);
        T::default()
    })
}

/// Combines each loader's result into [`Content`], collecting the errors of
/// every loader that failed.
fn assemble(
    palette: Result<PaletteDef, Vec<ContentError>>,
    keymap: Result<KeymapDef, Vec<ContentError>>,
    font: Result<FontAtlasDef, Vec<ContentError>>,
    terrain: Result<TerrainDef, Vec<ContentError>>,
    maps: Result<BTreeMap<String, MapDef>, Vec<ContentError>>,
    units: Loaded,
) -> Result<Content, ContentErrors> {
    let mut errors = Vec::new();
    let content = Content {
        palette: take(palette, &mut errors),
        keymap: take(keymap, &mut errors),
        font: take(font, &mut errors),
        terrain: take(terrain, &mut errors),
        maps: take(maps, &mut errors),
        classes: take(units.classes, &mut errors),
        items: take(units.items, &mut errors),
        spells: take(units.spells, &mut errors),
        skills: take(units.skills, &mut errors),
        arts: take(units.arts, &mut errors),
        names: take(units.names, &mut errors),
        characters: take(units.characters, &mut errors),
        portraits: take(units.portraits, &mut errors),
        dialogue: take(units.dialogue, &mut errors),
        ai: take(units.ai, &mut errors),
        tips: take(units.tips, &mut errors),
        audio: take(units.audio, &mut errors),
        credits: take(units.credits, &mut errors),
        images: take(units.images, &mut errors),
        tilesets: take(units.tilesets, &mut errors),
        battles: take(units.battles, &mut errors),
        chapters: take(units.chapters, &mut errors),
        new_game: take(units.new_game, &mut errors),
    };
    if errors.is_empty() {
        Ok(content)
    } else {
        Err(ContentErrors(errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_terrain() -> Result<TerrainDef, Vec<ContentError>> {
        TerrainDef::load(PaletteDef::load().ok().as_ref())
    }

    fn ok_maps() -> Result<BTreeMap<String, MapDef>, Vec<ContentError>> {
        map::load_all(&ok_terrain().unwrap_or_default().display)
    }

    fn ok_classes() -> Result<ClassTable, Vec<ContentError>> {
        class::load(Some(&ok_terrain().unwrap_or_default().rules.movement_types))
    }

    fn ok_characters() -> Result<CharacterTable, Vec<ContentError>> {
        character::load(
            ok_classes().ok().as_ref(),
            item::load().ok().as_ref(),
            names::load().ok().as_ref(),
        )
    }

    fn ok_spells() -> Result<SpellTable, Vec<ContentError>> {
        spell::load(Some(&ok_terrain().unwrap_or_default().display))
    }

    fn ok_skills() -> Result<SkillTable, Vec<ContentError>> {
        skill::load()
    }

    fn ok_portraits() -> Result<BTreeMap<String, Portrait>, Vec<ContentError>> {
        portrait::load_all(&PaletteDef::load().unwrap_or_default())
    }

    fn ok_dialogue() -> Result<DialogueTable, Vec<ContentError>> {
        dialogue::load(
            ok_characters().ok().as_ref(),
            ok_portraits().ok().as_ref(),
            names::load().ok().as_ref(),
            audio::load().ok().as_ref(),
        )
    }

    fn ok_story() -> Story {
        let terrain = ok_terrain().ok();
        load_story(
            ok_maps().ok().as_ref(),
            terrain.as_ref(),
            ok_classes().ok().as_ref(),
            item::load().ok().as_ref(),
            ok_characters().ok().as_ref(),
            ok_dialogue().ok().as_ref(),
            audio::load().ok().as_ref(),
        )
    }

    fn ok_credits() -> Result<Credits, Vec<ContentError>> {
        credits::load(audio::load().ok().as_ref())
    }

    fn ok_tilesets() -> Result<BTreeMap<String, Tileset>, Vec<ContentError>> {
        load_tilesets(
            ImageTable::load().ok().as_ref(),
            ok_terrain().ok().as_ref(),
            ok_classes().ok().as_ref(),
            ok_characters().ok().as_ref(),
        )
    }

    fn ok_units() -> Loaded {
        let (battles, chapters, new_game) = ok_story();
        Loaded {
            classes: ok_classes(),
            items: item::load(),
            spells: ok_spells(),
            skills: ok_skills(),
            arts: art::load(),
            names: names::load(),
            characters: ok_characters(),
            portraits: ok_portraits(),
            dialogue: ok_dialogue(),
            ai: ai::load(),
            tips: tip::load(),
            audio: audio::load(),
            credits: ok_credits(),
            images: ImageTable::load(),
            tilesets: ok_tilesets(),
            battles,
            chapters,
            new_game,
        }
    }

    #[test]
    fn assemble_ok_keeps_content() {
        let content = assemble(
            PaletteDef::load(),
            KeymapDef::load(),
            FontAtlasDef::load(),
            ok_terrain(),
            ok_maps(),
            ok_units(),
        )
        .ok();
        assert_eq!(
            content.as_ref().map(|c| &c.palette),
            PaletteDef::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.keymap),
            KeymapDef::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.font),
            FontAtlasDef::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.terrain),
            ok_terrain().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.maps), ok_maps().ok().as_ref());
        assert_eq!(
            content.as_ref().map(|c| &c.classes),
            ok_classes().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.items),
            item::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.spells),
            ok_spells().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.skills),
            ok_skills().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.arts), art::load().ok().as_ref());
        assert_eq!(
            content.as_ref().map(|c| &c.names),
            names::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.characters),
            ok_characters().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.portraits),
            ok_portraits().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.dialogue),
            ok_dialogue().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.ai), ai::load().ok().as_ref());
        assert_eq!(
            content.as_ref().map(|c| &c.audio),
            audio::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.credits),
            ok_credits().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.images),
            ImageTable::load().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.tilesets),
            ok_tilesets().ok().as_ref()
        );
        assert!(
            content.as_ref().is_some_and(
                |c| c.maps.contains_key("test_small") && c.dialogue.get("test").is_some()
            )
        );
        assert!(
            content.is_some_and(
                |c| !c.classes.classes.is_empty() && !c.characters.characters.is_empty()
            )
        );
    }

    const NAMES: [&str; 23] = [
        "p", "k", "f", "t", "m", "c", "i", "s", "x", "a", "n", "u", "o", "d", "w", "y", "v", "r",
        "j", "z", "b", "h", "g",
    ];

    #[test]
    fn assemble_reports_loader_errors() {
        let e = |f: &str| vec![ContentError::new(f, "bad")];
        assert_eq!(
            assemble(
                Err(e("p")),
                Err(e("k")),
                Err(e("f")),
                Err(e("t")),
                Err(e("m")),
                Loaded {
                    classes: Err(e("c")),
                    items: Err(e("i")),
                    spells: Err(e("s")),
                    skills: Err(e("x")),
                    arts: Err(e("a")),
                    names: Err(e("n")),
                    characters: Err(e("u")),
                    portraits: Err(e("o")),
                    dialogue: Err(e("d")),
                    ai: Err(e("w")),
                    tips: Err(e("y")),
                    audio: Err(e("v")),
                    credits: Err(e("r")),
                    images: Err(e("j")),
                    tilesets: Err(e("z")),
                    battles: Err(e("b")),
                    chapters: Err(e("h")),
                    new_game: Err(e("g")),
                },
            ),
            Err(ContentErrors(NAMES.iter().flat_map(|f| e(f)).collect()))
        );
        let only = |i: usize| {
            assemble(
                if i == 0 {
                    Err(e("p"))
                } else {
                    PaletteDef::load()
                },
                if i == 1 {
                    Err(e("k"))
                } else {
                    KeymapDef::load()
                },
                if i == 2 {
                    Err(e("f"))
                } else {
                    FontAtlasDef::load()
                },
                if i == 3 { Err(e("t")) } else { ok_terrain() },
                if i == 4 { Err(e("m")) } else { ok_maps() },
                Loaded {
                    classes: if i == 5 { Err(e("c")) } else { ok_classes() },
                    items: if i == 6 { Err(e("i")) } else { item::load() },
                    spells: if i == 7 { Err(e("s")) } else { ok_spells() },
                    skills: if i == 8 { Err(e("x")) } else { ok_skills() },
                    arts: if i == 9 { Err(e("a")) } else { art::load() },
                    names: if i == 10 { Err(e("n")) } else { names::load() },
                    characters: if i == 11 {
                        Err(e("u"))
                    } else {
                        ok_characters()
                    },
                    portraits: if i == 12 { Err(e("o")) } else { ok_portraits() },
                    dialogue: if i == 13 { Err(e("d")) } else { ok_dialogue() },
                    ai: if i == 14 { Err(e("w")) } else { ai::load() },
                    tips: if i == 15 { Err(e("y")) } else { tip::load() },
                    audio: if i == 16 { Err(e("v")) } else { audio::load() },
                    credits: if i == 17 { Err(e("r")) } else { ok_credits() },
                    images: if i == 18 {
                        Err(e("j"))
                    } else {
                        ImageTable::load()
                    },
                    tilesets: if i == 19 { Err(e("z")) } else { ok_tilesets() },
                    battles: if i == 20 { Err(e("b")) } else { ok_story().0 },
                    chapters: if i == 21 { Err(e("h")) } else { ok_story().1 },
                    new_game: if i == 22 { Err(e("g")) } else { ok_story().2 },
                },
            )
        };
        for (i, f) in NAMES.iter().enumerate() {
            assert_eq!(only(i), Err(ContentErrors(e(f))));
        }
    }

    #[test]
    fn tilesets_are_skipped_when_what_they_need_failed() {
        let (images, terrain) = (ImageTable::load().ok(), ok_terrain().ok());
        let (classes, characters) = (ok_classes().ok(), ok_characters().ok());
        let all = [
            images.is_some(),
            terrain.is_some(),
            classes.is_some(),
            characters.is_some(),
        ];
        assert_eq!(all, [true; 4]);
        for missing in 0..4 {
            let loaded = load_tilesets(
                images.as_ref().filter(|_| missing != 0),
                terrain.as_ref().filter(|_| missing != 1),
                classes.as_ref().filter(|_| missing != 2),
                characters.as_ref().filter(|_| missing != 3),
            );
            assert_eq!(loaded, Ok(BTreeMap::new()), "{missing}");
        }
        let loaded = load_tilesets(
            images.as_ref(),
            terrain.as_ref(),
            classes.as_ref(),
            characters.as_ref(),
        );
        assert!(loaded.is_ok_and(|t| t.contains_key("test")));
    }

    #[test]
    fn story_files_are_skipped_when_what_they_need_failed() {
        let (battles, chapters, new_game) = load_story(None, None, None, None, None, None, None);
        assert_eq!(battles, Ok(BTreeMap::new()));
        assert_eq!(chapters, Ok(BTreeMap::new()));
        assert_eq!(new_game, Ok(NewGameDef::default()));
        // Battles name their music: no audio manifest, no battles.
        let terrain = ok_terrain().ok();
        let (battles, chapters, new_game) = load_story(
            ok_maps().ok().as_ref(),
            terrain.as_ref(),
            ok_classes().ok().as_ref(),
            item::load().ok().as_ref(),
            ok_characters().ok().as_ref(),
            ok_dialogue().ok().as_ref(),
            None,
        );
        assert_eq!(battles, Ok(BTreeMap::new()));
        assert_eq!(chapters, Ok(BTreeMap::new()));
        assert_eq!(new_game, Ok(NewGameDef::default()));
        let (battles, chapters, new_game) = ok_story();
        assert!(battles.is_ok_and(|b| b.contains_key("test")));
        assert!(chapters.is_ok_and(|c| c.contains_key("test")));
        assert!(new_game.is_ok_and(|n| n.first_chapter == "test"));
    }

    #[test]
    fn map_feature_checks_join_the_map_errors() {
        let terrain = ok_terrain().ok();
        let items = item::load().ok();
        let mut maps = ok_maps().unwrap_or_default();
        assert_eq!(
            check_map_features(Ok(maps.clone()), items.as_ref(), terrain.as_ref()),
            Ok(maps.clone())
        );
        if let Some(def) = maps.get_mut("test_small") {
            def.map.features.insert(
                trpg_core::Pos::new(0, 0),
                trpg_core::TileFeature::Chest(trpg_core::Loot::Item(trpg_core::ItemId::new("x"))),
            );
        }
        let errors = check_map_features(Ok(maps.clone()), items.as_ref(), terrain.as_ref());
        assert_eq!(errors.map_err(|e| e.len()), Err(1));
        // Skipped when another file failed.
        assert_eq!(
            check_map_features(Ok(maps.clone()), None, terrain.as_ref()),
            Ok(maps.clone())
        );
        assert_eq!(
            check_map_features(Ok(maps.clone()), items.as_ref(), None),
            Ok(maps)
        );
        let failed = Err(vec![ContentError::new("m", "bad")]);
        assert_eq!(
            check_map_features(failed.clone(), items.as_ref(), terrain.as_ref()),
            failed
        );
    }

    #[test]
    fn spell_reference_checks_join_the_spell_errors() {
        let classes = ok_classes().ok();
        let characters = ok_characters().ok();
        let spells = ok_spells();
        assert_eq!(
            check_spell_references(spells.clone(), classes.as_ref(), characters.as_ref()),
            spells
        );
        let empty = Ok(SpellTable::default());
        let errors = check_spell_references(empty.clone(), classes.as_ref(), characters.as_ref());
        assert!(errors.is_err_and(|e| !e.is_empty()));
        // Skipped when another file failed.
        assert_eq!(
            check_spell_references(empty.clone(), None, characters.as_ref()),
            empty
        );
        assert_eq!(
            check_spell_references(empty.clone(), classes.as_ref(), None),
            empty
        );
        let failed = Err(vec![ContentError::new("s", "bad")]);
        assert_eq!(
            check_spell_references(failed.clone(), classes.as_ref(), characters.as_ref()),
            failed
        );
    }

    #[test]
    fn skill_reference_checks_join_the_skill_errors() {
        let classes = ok_classes().ok();
        let skills = ok_skills();
        assert_eq!(
            check_skill_references(skills.clone(), classes.as_ref()),
            skills
        );
        let empty = Ok(SkillTable::default());
        let errors = check_skill_references(empty.clone(), classes.as_ref());
        assert!(errors.is_err_and(|e| !e.is_empty()));
        // Skipped when another file failed.
        assert_eq!(check_skill_references(empty.clone(), None), empty);
        let failed = Err(vec![ContentError::new("x", "bad")]);
        assert_eq!(
            check_skill_references(failed.clone(), classes.as_ref()),
            failed
        );
    }

    #[test]
    fn seal_checks_join_the_item_errors() {
        let classes = ok_classes().ok();
        let items = item::load();
        assert_eq!(check_seals(items.clone(), classes.as_ref()), items);
        // Without its seals, the class tree's promotions can't happen.
        let mut bare = items.clone().unwrap_or_default();
        bare.items
            .retain(|_, d| !matches!(d, trpg_core::ItemDef::Seal(_)));
        let errors: Vec<String> = check_seals(Ok(bare.clone()), classes.as_ref())
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            errors,
            [
                "assets/data/items.ron: no seal of kind Tier(2), but classes promote into tier 2",
                "assets/data/items.ron: no seal of kind Tier(3), but classes promote into tier 3",
                "assets/data/items.ron: no seal of kind Reclass",
            ]
        );
        // Skipped when another file failed.
        assert_eq!(check_seals(Ok(bare.clone()), None), Ok(bare));
        let failed = Err(vec![ContentError::new("i", "bad")]);
        assert_eq!(check_seals(failed.clone(), classes.as_ref()), failed);
    }

    #[test]
    fn art_reference_checks_join_the_art_errors() {
        let items = item::load().ok();
        let arts = art::load();
        assert_eq!(check_art_references(arts.clone(), items.as_ref()), arts);
        // A weapon naming an art that isn't there.
        let mut broken = items.clone().unwrap_or_default();
        if let Some(trpg_core::ItemDef::Weapon(w)) =
            broken.items.get_mut(&trpg_core::ItemId::new("iron_sword"))
        {
            w.arts.push(trpg_core::ArtId::new("nope"));
        }
        let errors = check_art_references(arts.clone(), Some(&broken));
        assert!(errors.is_err_and(|e| e.len() == 1));
        // Skipped when another file failed.
        assert_eq!(check_art_references(arts.clone(), None), arts);
        let failed = Err(vec![ContentError::new("a", "bad")]);
        assert_eq!(check_art_references(failed.clone(), items.as_ref()), failed);
    }

    #[test]
    fn embedded_content_loads() {
        let content = load_embedded();
        assert!(content.is_ok(), "{content:?}");
        let content = content.ok();
        assert_eq!(
            content.as_ref().map(|c| &c.terrain),
            ok_terrain().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.maps), ok_maps().ok().as_ref());
        assert_eq!(
            content.as_ref().map(|c| &c.classes),
            ok_classes().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.characters),
            ok_characters().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.skills),
            ok_skills().ok().as_ref()
        );
        assert_eq!(content.as_ref().map(|c| &c.arts), art::load().ok().as_ref());
        assert_eq!(
            content.as_ref().map(|c| &c.portraits),
            ok_portraits().ok().as_ref()
        );
        assert_eq!(
            content.as_ref().map(|c| &c.dialogue),
            ok_dialogue().ok().as_ref()
        );
    }
}
