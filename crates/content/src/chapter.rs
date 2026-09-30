//! Chapter files (`assets/chapters/*.ron`, ticket 0801): the story beat
//! around a battle (its scenes and what comes next), and the New Game file
//! (`assets/data/new_game.ron`): the first chapter, roster, gold and stock.
//! See `assets/chapters/README.md`.

use std::collections::BTreeMap;

use serde::Deserialize;
use trpg_core::{
    BattleDef, Campaign, CharacterId, Faction, GameMode, ItemDef, ItemId, LEAD_ID, LeadProfile,
    Pos, Stock, Unit, UnitId,
};

use crate::bundle;
use crate::character::character_unit;
use crate::dialogue::DialogueTable;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::{CharacterTable, Content};

/// Directory of the chapter files inside the asset bundle.
pub const CHAPTERS_DIR: &str = "chapters";

/// Path of the New Game file inside the asset bundle.
pub const NEW_GAME_PATH: &str = "data/new_game.ron";

/// Extension of a chapter file.
const EXTENSION: &str = ".ron";

/// One chapter: scenes, a battle, scenes, then the next chapter.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterDef {
    /// The chapter's id (its file stem).
    pub id: String,
    /// Its title, e.g. `Chapter 1: …`.
    pub title: String,
    /// Scenes before the battle, in order.
    #[serde(default)]
    pub intro_scenes: Vec<String>,
    /// The battle, by id (`assets/battles/`).
    pub battle: String,
    /// Scenes after a victory, in order.
    #[serde(default)]
    pub victory_scenes: Vec<String>,
    /// The chapter after this one, if any ("To be continued" otherwise).
    #[serde(default)]
    pub next: Option<String>,
}

/// How a new game starts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewGameDef {
    /// The first chapter, by id.
    pub first_chapter: String,
    /// The characters in the army at the start, in roster order. Must
    /// include the lead.
    pub roster: Vec<CharacterId>,
    /// Starting gold.
    pub gold: u32,

    /// Starting stock, one entry per item.
    pub stock: Vec<ItemId>,
}

/// The New Game file as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNewGame {
    first_chapter: String,
    roster: Vec<CharacterId>,
    #[serde(default)]
    gold: u32,
    #[serde(default)]
    stock: Vec<String>,
}

/// Loads every chapter file in the bundle, keyed by id (the file stem),
/// checking its references against `battles` and `dialogue`. Reports every
/// error of every file.
pub fn load_all(
    battles: &BTreeMap<String, BattleDef>,
    dialogue: &DialogueTable,
) -> Result<BTreeMap<String, ChapterDef>, Vec<ContentError>> {
    let mut chapters = BTreeMap::new();
    let mut errors = Vec::new();
    for path in bundle::files_in(CHAPTERS_DIR) {
        let Some(stem) = path
            .strip_prefix(CHAPTERS_DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(EXTENSION))
        else {
            continue;
        };
        let file = bundle::display_path(path);
        let Some(source) = bundle::file(path) else {
            errors.push(ContentError::new(&file, "file is not valid UTF-8"));
            continue;
        };
        match parse_ron::<ChapterDef>(&file, source) {
            Ok(def) => {
                errors.extend(check_chapter(&file, stem, &def, battles, dialogue));
                chapters.insert(stem.to_owned(), def);
            }
            Err(e) => errors.push(e),
        }
    }
    errors.extend(check_next(&chapters));
    if errors.is_empty() {
        Ok(chapters)
    } else {
        Err(errors)
    }
}

/// Every problem with chapter `def` from `file`: an id that isn't the file
/// name `stem`, a scene not in `dialogue`, a battle not in `battles`.
pub fn check_chapter(
    file: &str,
    stem: &str,
    def: &ChapterDef,
    battles: &BTreeMap<String, BattleDef>,
    dialogue: &DialogueTable,
) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let mut err = |m: String| errors.push(ContentError::new(file, m));
    if def.id != stem {
        err(format!(
            "id \"{}\" must match the file name \"{stem}\"",
            def.id
        ));
    }
    if !battles.contains_key(&def.battle) {
        err(format!("no battle \"{}\"", def.battle));
    }
    for scene in def.intro_scenes.iter().chain(&def.victory_scenes) {
        if dialogue.get(scene).is_none() {
            err(format!("no dialogue scene \"{scene}\""));
        }
    }
    errors
}

/// A chapter whose `next` isn't a chapter, one error each.
fn check_next(chapters: &BTreeMap<String, ChapterDef>) -> Vec<ContentError> {
    chapters
        .values()
        .filter_map(|c| {
            let next = c.next.as_ref()?;
            (!chapters.contains_key(next)).then(|| {
                let file = bundle::display_path(&format!("{CHAPTERS_DIR}/{}{EXTENSION}", c.id));
                ContentError::new(file, format!("next: no chapter \"{next}\""))
            })
        })
        .collect()
}

/// Loads the New Game file, checking its references.
pub fn load_new_game(
    chapters: &BTreeMap<String, ChapterDef>,
    characters: &CharacterTable,
    items: &trpg_core::ItemTable,
) -> Result<NewGameDef, Vec<ContentError>> {
    let file = bundle::display_path(NEW_GAME_PATH);
    let source = bundle::file(NEW_GAME_PATH)
        .ok_or_else(|| vec![ContentError::new(&file, "file not found in asset bundle")])?;
    new_game_from_source(&file, source, chapters, characters, items)
}

/// Parses and checks New Game `source` (errors attributed to `file`): the
/// first chapter exists, the roster's characters exist (the lead among
/// them, each once), and the stock's items exist.
pub fn new_game_from_source(
    file: &str,
    source: &str,
    chapters: &BTreeMap<String, ChapterDef>,
    characters: &CharacterTable,
    items: &trpg_core::ItemTable,
) -> Result<NewGameDef, Vec<ContentError>> {
    let raw: RawNewGame = parse_ron(file, source).map_err(|e| vec![e])?;
    let def = NewGameDef {
        first_chapter: raw.first_chapter,
        roster: raw.roster,
        gold: raw.gold,
        stock: raw.stock.iter().map(|i| ItemId::new(i)).collect(),
    };

    let mut errors = Vec::new();
    let mut err = |m: String| errors.push(ContentError::new(file, m));
    if !chapters.contains_key(&def.first_chapter) {
        err(format!("first_chapter: no chapter \"{}\"", def.first_chapter));
    }
    for (i, c) in def.roster.iter().enumerate() {
        if !characters.characters.contains_key(c) {
            err(format!("roster: no character \"{}\"", c.0));
        }
        if def.roster[..i].contains(c) {
            err(format!("roster: \"{}\" is listed twice", c.0));
        }
    }
    if !def.roster.iter().any(|c| c.0 == LEAD_ID) {
        err(format!("roster: the lead (\"{LEAD_ID}\") must be in it"));
    }
    for item in &def.stock {
        if items.get(item).is_none() {
            err(format!("stock: no item \"{}\"", item.0));
        }
    }
    if errors.is_empty() {
        Ok(def)
    } else {
        Err(errors)
    }
}

/// Units of `characters` (in that order) as they start the game, from the
/// character data. Unknown characters (content validation rules them out)
/// are skipped.
pub fn roster_units(content: &Content, characters: &[CharacterId]) -> Vec<Unit> {
    characters
        .iter()
        .filter_map(|c| {
            let def = content.characters.characters.get(c)?;
            character_unit(
                def,
                UnitId(0),
                &content.classes,
                &content.items,
                Faction::Player,
                Pos::new(0, 0),
            )
            .ok()
        })
        .collect()
}

/// `items` as a stock: weapons as fresh copies, anything else counted.
/// Unknown items are skipped.
pub fn stock_of(content: &Content, items: &[ItemId]) -> Stock {
    let mut stock = Stock::default();
    for item in items {
        match content.items.get(item) {
            Some(ItemDef::Weapon(_)) => stock.weapons.extend(content.items.new_weapon(item)),
            Some(_) => stock.add(item.clone()),
            None => {}
        }
    }
    stock
}

/// A new campaign in `mode` with `lead`, as the New Game file starts it.
pub fn new_campaign(content: &Content, mode: GameMode, lead: LeadProfile) -> Campaign {
    let start = &content.new_game;
    Campaign::new_game(
        mode,
        lead,
        start.first_chapter.clone(),
        roster_units(content, &start.roster),
        start.gold,
        stock_of(content, &start.stock),
    )
}

/// A throwaway campaign for playing battle `battle` on its own (the debug
/// Quick Battle): the characters of its player slots, fresh from the
/// character data, no gold and no stock.
pub fn battle_campaign(
    content: &Content,
    battle: &BattleDef,
    mode: GameMode,
    lead: LeadProfile,
) -> Campaign {
    let characters: Vec<CharacterId> = battle
        .player_slots
        .iter()
        .map(|s| s.character.clone())
        .collect();
    Campaign::new_game(
        mode,
        lead,
        String::new(),
        roster_units(content, &characters),
        0,
        Stock::default(),
    )
}

#[cfg(test)]
mod tests;
