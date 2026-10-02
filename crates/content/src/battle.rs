//! Battle files (`assets/battles/*.ron`, ticket 0801): everything needed to
//! play one battle. See `assets/battles/README.md` for the format; the
//! loader builds a [`BattleDef`] and checks every rule listed there.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::{
    AiBehavior, BattleDef, BattleMap, BattleMusic, CharacterId, ClassTable, Difficulty, Faction,
    ItemDef, ItemId, ItemTable, Level, Objective, PlayerSlot, Pos, Reinforcement, Role,
    TerrainTable, Trigger, Turn, Unit, UnitId, default_map_label,
};

use crate::audio::AudioManifest;
use crate::bundle;
use crate::character::{CharacterTable, RawLoadout, character_unit, check_map_labels};
use crate::dialogue::DialogueTable;
use crate::error::ContentError;
use crate::map::MapDef;
use crate::ron_loader::parse_ron;
use crate::trigger::check_triggers;

/// Directory of the battle files inside the asset bundle.
pub const BATTLES_DIR: &str = "battles";

/// Extension of a battle file.
const EXTENSION: &str = ".ron";

/// The message for a battle that asks for the Preparations screen, which
/// doesn't exist yet (ticket 0408 removes this check).
pub const NO_PREPARATIONS: &str = "Preparations screen not built yet, ticket 0408";

/// The content a battle file refers to.
#[derive(Debug, Clone, Copy)]
pub struct BattleRefs<'a> {
    /// Maps by id.
    pub maps: &'a BTreeMap<String, MapDef>,
    /// Terrain rules (which tiles a unit can stand on).
    pub terrain: &'a TerrainTable,
    /// Classes.
    pub classes: &'a ClassTable,
    /// Items.
    pub items: &'a ItemTable,
    /// Named characters and generic templates.
    pub characters: &'a CharacterTable,
    /// Scenes, for the triggers.
    pub dialogue: &'a DialogueTable,
    /// Music cues and pools, for the battle's music.
    pub audio: &'a AudioManifest,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBattle {
    id: String,
    map: String,
    player_slots: Vec<RawSlot>,
    #[serde(default)]
    enemies: Vec<RawEnemy>,
    #[serde(default)]
    reinforcements: Vec<RawReinforcement>,
    #[serde(default)]
    preparations: bool,
    /// Written bare: `pack_cap: 3`.
    #[serde(default, deserialize_with = "bare")]
    pack_cap: Option<usize>,
    #[serde(default)]
    default_pack: Vec<String>,
    #[serde(default)]
    clear_gold: u32,
    objective: RawObjective,
    #[serde(default)]
    triggers: Vec<Trigger>,
    difficulty: Difficulty,
    music: BattleMusic,
    seed: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSlot {
    character: String,
    pos: (i32, i32),
    /// Numbered after the enemies instead of in slot order.
    #[serde(default)]
    after_enemies: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnemy {
    /// Written bare: `template: "brigand"`.
    #[serde(default, deserialize_with = "bare")]
    template: Option<String>,
    /// Written bare: `character: "rook"`.
    #[serde(default, deserialize_with = "bare")]
    character: Option<String>,
    #[serde(default)]
    level: Option<Level>,
    pos: (i32, i32),
    #[serde(default)]
    ai: AiBehavior,
    #[serde(default)]
    loadout: Option<RawLoadout>,
    #[serde(default)]
    boss: bool,
    #[serde(default)]
    name: Option<String>,
}

/// Reads an optional field written without `Some(…)`.
fn bare<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReinforcement {
    turn: Turn,
    unit: RawEnemy,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum RawObjective {
    Rout {
        #[serde(default)]
        turn_limit: Option<Turn>,
    },
    DefeatUnit {
        unit: String,
        #[serde(default)]
        turn_limit: Option<Turn>,
    },
    Seize {
        pos: (i32, i32),
        #[serde(default)]
        by_lord: bool,
        #[serde(default)]
        turn_limit: Option<Turn>,
    },
    Survive {
        turns: Turn,
    },
}

/// Loads every battle file in the bundle, keyed by id (the file stem).
/// Reports every error of every file.
pub fn load_all(refs: &BattleRefs<'_>) -> Result<BTreeMap<String, BattleDef>, Vec<ContentError>> {
    let mut battles = BTreeMap::new();
    let mut errors = Vec::new();
    for path in bundle::files_in(BATTLES_DIR) {
        let Some(stem) = path
            .strip_prefix(BATTLES_DIR)
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(EXTENSION))
        else {
            continue;
        };
        let file = bundle::display_path(path);
        let result = bundle::file(path)
            .ok_or_else(|| vec![ContentError::new(&file, "file is not valid UTF-8")])
            .and_then(|source| from_source(&file, stem, source, refs));
        match result {
            Ok(def) => {
                battles.insert(stem.to_owned(), def);
            }
            Err(e) => errors.extend(e),
        }
    }
    if errors.is_empty() {
        Ok(battles)
    } else {
        Err(errors)
    }
}

/// Parses and validates battle file `source` (errors attributed to
/// `file`), whose id must be `stem`. Reports every problem found.
pub fn from_source(
    file: &str,
    stem: &str,
    source: &str,
    refs: &BattleRefs<'_>,
) -> Result<BattleDef, Vec<ContentError>> {
    let raw: RawBattle = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut v = Checker {
        file,
        refs,
        errors: Vec::new(),
        taken: BTreeMap::new(),
        cast: BTreeSet::new(),
    };
    if raw.id != stem {
        v.err(format!(
            "id \"{}\" must match the file name \"{stem}\"",
            raw.id
        ));
    }
    let Some(map) = refs.maps.get(&raw.map).map(|m| m.map.clone()) else {
        v.err(format!("no map \"{}\"", raw.map));
        return Err(v.errors);
    };
    if raw.preparations {
        v.err(NO_PREPARATIONS.to_owned());
    }
    let slot_ids = slot_ids(&raw.player_slots, raw.enemies.len());
    let early = raw.player_slots.iter().filter(|s| !s.after_enemies).count();
    let mut next_id = BattleDef::first_enemy_id(early);
    let players = v.players(&raw.player_slots, &slot_ids, &map);
    let enemies: Vec<Unit> = raw
        .enemies
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let unit = v.enemy(&format!("enemy {}", i + 1), e, UnitId(next_id), &map, true);
            next_id += 1;
            unit
        })
        .collect();
    // The slots numbered after the enemies come before the reinforcements.
    let late = raw.player_slots.iter().filter(|s| s.after_enemies).count();
    next_id = next_id.saturating_add(u32::try_from(late).unwrap_or(u32::MAX));
    let reinforcements: Vec<Reinforcement> = raw
        .reinforcements
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let what = format!("reinforcement {}", i + 1);
            if r.turn == 0 {
                v.err(format!("{what}: turn 0; turns start at 1"));
            }
            let unit = v.enemy(&what, &r.unit, UnitId(next_id), &map, false);
            next_id += 1;
            Some(Reinforcement {
                turn: r.turn,
                unit: unit?,
            })
        })
        .collect();
    let everyone: Vec<Unit> = players
        .iter()
        .chain(&enemies)
        .chain(reinforcements.iter().map(|r| &r.unit))
        .cloned()
        .collect();
    let objective = v.objective(&raw.objective, &map, &everyone);
    let pack_cap = raw.pack_cap.unwrap_or(refs.items.rules.default_pack_cap);
    let default_pack = v.pack(&raw.default_pack, pack_cap);
    v.music(&raw.music);
    v.errors.extend(check_map_labels(file, &everyone));
    v.errors.extend(check_triggers(
        file,
        &raw.triggers,
        &everyone,
        &map,
        refs.dialogue,
    ));
    if !v.errors.is_empty() {
        return Err(v.errors);
    }
    Ok(BattleDef {
        id: raw.id,
        map,
        player_slots: raw
            .player_slots
            .iter()
            .zip(&slot_ids)
            .map(|(s, &id)| PlayerSlot {
                character: CharacterId(s.character.clone()),
                pos: pos(s.pos),
                id,
            })
            .collect(),
        enemies,
        reinforcements,
        preparations: raw.preparations,
        pack_cap,
        default_pack,
        clear_gold: raw.clear_gold,
        objective: objective.unwrap_or(Objective::Rout { turn_limit: None }),
        triggers: raw.triggers,
        difficulty: raw.difficulty,
        music: raw.music,
        seed: raw.seed,
    })
}

fn pos((x, y): (i32, i32)) -> Pos {
    Pos::new(x, y)
}

/// The unit id of each slot: the slots numbered in order from 1, skipping
/// those marked `after_enemies`, which follow the `enemies` enemies in
/// order.
fn slot_ids(slots: &[RawSlot], enemies: usize) -> Vec<UnitId> {
    let early = slots.iter().filter(|s| !s.after_enemies).count();
    let (mut next_early, mut next_late) = (0, early + enemies);
    slots
        .iter()
        .map(|s| {
            let next = if s.after_enemies {
                &mut next_late
            } else {
                &mut next_early
            };
            *next += 1;
            BattleDef::slot_id(*next - 1)
        })
        .collect()
}

/// Collects the errors of one battle file.
struct Checker<'a, 'r> {
    file: &'a str,
    refs: &'a BattleRefs<'r>,
    errors: Vec<ContentError>,
    /// Who stands on each starting tile.
    taken: BTreeMap<Pos, String>,
    /// Characters already placed.
    cast: BTreeSet<String>,
}

impl Checker<'_, '_> {
    fn err(&mut self, message: String) {
        self.errors.push(ContentError::new(self.file, message));
    }

    /// The player units the slots would hold, with the characters' own
    /// data (the campaign's roster replaces them in play).
    fn players(&mut self, slots: &[RawSlot], ids: &[UnitId], map: &BattleMap) -> Vec<Unit> {
        let refs = self.refs;
        let mut units = Vec::new();
        for (i, (slot, &unit_id)) in slots.iter().zip(ids).enumerate() {
            let what = format!("player slot {} (\"{}\")", i + 1, slot.character);
            let id = CharacterId(slot.character.clone());
            let Some(def) = refs.characters.characters.get(&id) else {
                self.err(format!("{what}: no character \"{}\"", slot.character));
                continue;
            };
            let at = pos(slot.pos);
            match character_unit(def, unit_id, refs.classes, refs.items, Faction::Player, at) {
                Ok(unit) => {
                    self.place(&what, &unit, map, true);
                    units.push(unit);
                }
                Err(e) => self.err(format!("{what}: {e}")),
            }
        }
        units
    }

    /// The unit of enemy entry `e`, with id `id`. `start`: it is on the map
    /// at the start (so no other unit may share its tile).
    fn enemy(
        &mut self,
        what: &str,
        e: &RawEnemy,
        id: UnitId,
        map: &BattleMap,
        start: bool,
    ) -> Option<Unit> {
        let refs = self.refs;
        let at = pos(e.pos);
        let built = match (&e.template, &e.character) {
            (Some(t), None) => {
                let Some(template) = refs.characters.generics.get(t) else {
                    self.err(format!("{what}: no generic template \"{t}\""));
                    return None;
                };
                let mut template = template.clone();
                if let Some(level) = e.level {
                    if level < 1 || level > refs.classes.level_cap {
                        let cap = refs.classes.level_cap;
                        self.err(format!("{what}: level {level} is outside 1..={cap}"));
                    }
                    template.level = level;
                }
                if let Some(loadout) = &e.loadout {
                    template.loadout = loadout.to_def();
                }
                template.unit(id, refs.classes, refs.items, Faction::Enemy, at)
            }
            (None, Some(c)) => {
                let Some(def) = refs.characters.characters.get(&CharacterId(c.clone())) else {
                    self.err(format!("{what}: no character \"{c}\""));
                    return None;
                };
                if e.level.is_some() {
                    self.err(format!(
                        "{what}: a character's level comes from characters.ron"
                    ));
                }
                let mut def = def.clone();
                if let Some(loadout) = &e.loadout {
                    def.loadout = loadout.to_def();
                }
                character_unit(&def, id, refs.classes, refs.items, Faction::Enemy, at)
            }
            _ => {
                self.err(format!("{what}: give a template or a character, not both"));
                return None;
            }
        };
        let mut unit = match built {
            Ok(unit) => unit,
            Err(e) => {
                self.err(format!("{what}: {e}"));
                return None;
            }
        };
        unit.ai = e.ai;
        if e.boss {
            unit.role = Role::Boss;
        }
        if let Some(name) = &e.name {
            unit.name.clone_from(name);
            unit.map_label = default_map_label(name);
        }
        self.place(what, &unit, map, start);
        Some(unit)
    }

    /// Checks `unit` stands on a tile of `map` it can stand on, as the only
    /// unit there (`start`), and that its character isn't placed twice.
    fn place(&mut self, what: &str, unit: &Unit, map: &BattleMap, start: bool) {
        let at = unit.pos;
        let class = self.refs.classes.get(&unit.class);
        let tile = map.tiles.get(at).copied();
        match (tile, class) {
            (None, _) => self.err(format!(
                "{what}: ({}, {}) is outside the {}×{} map",
                at.x,
                at.y,
                map.tiles.width(),
                map.tiles.height()
            )),
            (Some(t), Some(class))
                if self
                    .refs
                    .terrain
                    .move_cost(t, class.movement_type)
                    .is_none() =>
            {
                self.err(format!(
                    "{what}: a {} can't stand on ({}, {})",
                    class.name, at.x, at.y
                ));
            }
            _ => {}
        }
        if start {
            if let Some(other) = self.taken.get(&at) {
                let other = other.clone();
                self.err(format!(
                    "{what}: ({}, {}) is already taken by {other}",
                    at.x, at.y
                ));
            } else {
                self.taken.insert(at, what.to_owned());
            }
        }
        if let Some(c) = &unit.character
            && !self.cast.insert(c.0.clone())
        {
            self.err(format!("{what}: character \"{}\" is placed twice", c.0));
        }
    }

    fn objective(
        &mut self,
        raw: &RawObjective,
        map: &BattleMap,
        everyone: &[Unit],
    ) -> Option<Objective> {
        Some(match *raw {
            RawObjective::Rout { turn_limit } => Objective::Rout { turn_limit },
            RawObjective::DefeatUnit {
                ref unit,
                turn_limit,
            } => {
                let target = everyone.iter().find(|u| {
                    u.faction != Faction::Player
                        && u.character.as_ref().is_some_and(|c| &c.0 == unit)
                });
                let Some(target) = target else {
                    self.err(format!(
                        "objective: no enemy of character \"{unit}\" to defeat"
                    ));
                    return None;
                };
                Objective::DefeatUnit {
                    unit: target.id,
                    turn_limit,
                }
            }
            RawObjective::Seize {
                pos: at,
                by_lord,
                turn_limit,
            } => {
                let at = pos(at);
                if !map.tiles.in_bounds(at) {
                    self.err(format!(
                        "objective: the seize tile ({}, {}) is outside the map",
                        at.x, at.y
                    ));
                }
                Objective::Seize {
                    pos: at,
                    by_lord,
                    turn_limit,
                }
            }
            RawObjective::Survive { turns } => {
                if turns == 0 {
                    self.err("objective: survive 0 turns".to_owned());
                }
                Objective::Survive { turns }
            }
        })
        .inspect(|o| {
            if o.turn_limit() == Some(0) {
                self.err("objective: turn limit 0".to_owned());
            }
        })
    }

    /// The music names a music cue, or a pool, of the audio manifest.
    fn music(&mut self, music: &BattleMusic) {
        let audio = self.refs.audio;
        match music {
            BattleMusic::Cue(cue) if !audio.music.contains_key(cue) => {
                self.err(format!("music: no music cue \"{cue}\""));
            }
            BattleMusic::Pool(pool) if !audio.pools.contains_key(pool) => {
                self.err(format!("music: no music pool \"{pool}\""));
            }
            _ => {}
        }
    }

    /// The default pack's items: known consumables, at most `cap`.
    fn pack(&mut self, items: &[String], cap: usize) -> Vec<ItemId> {
        if items.len() > cap {
            self.err(format!(
                "default_pack: {} items, more than the pack cap {cap}",
                items.len()
            ));
        }
        for item in items {
            match self.refs.items.get(&ItemId::new(item)) {
                Some(ItemDef::Consumable(_)) => {}
                Some(_) => self.err(format!("default_pack: \"{item}\" isn't a consumable")),
                None => self.err(format!("default_pack: no item \"{item}\"")),
            }
        }
        items.iter().map(|i| ItemId::new(i)).collect()
    }
}

#[cfg(test)]
mod tests;
