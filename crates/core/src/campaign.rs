//! The campaign: what carries over from one battle to the next (ticket
//! 0801), and the battles it plays.
//!
//! # Rules
//!
//! Sources: `docs/design/death-and-difficulty.md` (falling units, rewind
//! charges and their bonus, game mode), `docs/design/weapons-and-items.md`
//! (the battle pack) and `docs/design/battle-scenes-and-recruitment.md`
//! (recruits join after the battle).
//!
//! - **Roster.** Every character in the army, one [`Unit`] each with its
//!   levels, stats, loadout, weapon ranks and durability. Roster units are
//!   always named characters (the lead, story joins, recruits).
//! - **A battle** ([`BattleDef`], from a battle file) names the tile of each
//!   roster member it deploys ([`PlayerSlot`]). A slot whose character isn't
//!   in the roster (a Classic death) stays empty; a roster member without a
//!   slot stays out of the battle. [`Campaign::battle_setup`] builds the
//!   [`BattleSetup`]: slot `i`'s unit gets id `i + 1` (the battle file's
//!   enemies are numbered after the slots, [`BattleDef::first_enemy_id`]),
//!   the battle's default pack (no Preparations yet, 0408), the campaign's
//!   gold, stock and mode, and the rewind charges of the battle's
//!   [`Difficulty`].
//! - **After a victory** ([`Campaign::apply_result`]), in this order:
//!   1. Every deployed player unit's roster entry becomes the battle's unit
//!      (levels, EXP, loadout, weapon ranks, durability all carry over). A
//!      fallen one in **Classic** leaves the roster and everything in its
//!      loadout goes to the stock (weapons keep their durability); in
//!      **Casual** it stays, with what it had earned before falling.
//!   2. Every deployed unit still in the roster (standing, or retreated in
//!      Casual) gets the unused-charge bonus: [`UNUSED_CHARGE_PERCENT`]% of
//!      one level's EXP ([`EXP_PER_LEVEL`]) per unused rewind charge, as one
//!      award ([`grant_exp`]: at most [`MAX_AWARD`], level cap), in slot
//!      order. Level ups roll on a copy of the battle's RNG, so the same
//!      battle always gives the same result.
//!   3. The battle's recruits ([`BattleState::recruited`]) join the roster,
//!      as player units.
//!   4. The stock becomes the battle's (with what was bought or found),
//!      plus the pack's unused items; gold becomes the battle's plus the
//!      battle's `clear_gold`.
//!   5. Every roster unit is made ready for the next battle: full HP, not
//!      acted, no timed effects (*Claude's starting rule* for standing
//!      units, as in Fire Emblem; the design says it for Casual retreats).
//!
//!   A defeat changes nothing ([`ApplyError::NotWon`]): the game is over,
//!   and Retry starts the battle again from its setup.
//! - **Mode.** Classic → Casual only ([`Campaign::downgrade_mode`]).
//! - **Lead.** [`Campaign::new_game`] gives the lead's unit (character
//!   [`LEAD_ID`]) the player's name, and a map label of that name's first
//!   two letters (*Claude's starting rule*).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::art::ArtTable;
use crate::battle::{BattleSetup, BattleState, Event, GameMode, Objective, Outcome, Reinforcement, Trigger};
use crate::class::ClassTable;
use crate::geom::Pos;
use crate::item::{BattlePack, ItemId, ItemTable, Stock};
use crate::lead::{LEAD_ID, LeadProfile};
use crate::map::BattleMap;
use crate::progression::{EXP_PER_LEVEL, MAX_AWARD, grant_exp};
use crate::shop::Gold;
use crate::skill::SkillTable;
use crate::spell::SpellTable;
use crate::terrain::TerrainTable;
use crate::unit::{CharacterId, Faction, Role, Unit, UnitId, default_map_label};

/// EXP bonus per unused rewind charge, in percent of one level's EXP
/// (`death-and-difficulty.md`: 7% of a level, 7 EXP at 100 per level).
pub const UNUSED_CHARGE_PERCENT: u32 = 7;

/// A battle's difficulty tier, which sets its rewind charges
/// (`death-and-difficulty.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Difficulty {
    /// 2 charges.
    Easy,
    /// 3 charges.
    #[default]
    Normal,
    /// 5 charges.
    Hard,
    /// A big, hard map (a finale): 8 charges.
    Finale,
}

impl Difficulty {
    /// Every tier, easiest first.
    pub const ALL: [Difficulty; 4] = [
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::Finale,
    ];

    /// Rewind charges a battle of this tier starts with.
    pub const fn rewind_charges(self) -> u8 {
        match self {
            Difficulty::Easy => 2,
            Difficulty::Normal => 3,
            Difficulty::Hard => 5,
            Difficulty::Finale => 8,
        }
    }
}

/// Where a battle deploys a roster member.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerSlot {
    /// The roster member.
    pub character: CharacterId,
    /// Its starting tile.
    pub pos: Pos,
}

/// One battle, as a battle file describes it (validated by `content`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleDef {
    /// The battle's id (its file stem).
    pub id: String,
    /// The battlefield.
    pub map: BattleMap,
    /// Where roster members are placed, in unit order.
    pub player_slots: Vec<PlayerSlot>,
    /// The other units on the map at the start, numbered from
    /// [`first_enemy_id`](Self::first_enemy_id).
    pub enemies: Vec<Unit>,
    /// Units that arrive later, numbered after the enemies.
    pub reinforcements: Vec<Reinforcement>,
    /// Whether the Preparations screen (0408) comes first. Not built yet:
    /// the content validator refuses `true` for now.
    pub preparations: bool,
    /// How many consumables the pack may hold.
    pub pack_cap: usize,
    /// The pack brought in without Preparations.
    pub default_pack: Vec<ItemId>,
    /// Gold for winning.
    pub clear_gold: Gold,
    /// How to win.
    pub objective: Objective,
    /// The battle's story moments (0705).
    pub triggers: Vec<Trigger>,
    /// Sets the rewind charges.
    pub difficulty: Difficulty,
    /// Seed of the battle's RNG.
    pub seed: u64,
}

impl BattleDef {
    /// The unit id of player slot `index`.
    pub fn slot_id(index: usize) -> UnitId {
        UnitId(u32::try_from(index + 1).unwrap_or(u32::MAX))
    }

    /// The id of the first enemy: the one after the last slot's, so that
    /// slot ids and enemy ids never clash.
    pub fn first_enemy_id(slots: usize) -> u32 {
        u32::try_from(slots + 1).unwrap_or(u32::MAX)
    }
}

/// The shared content tables every battle reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameTables {
    /// Terrain rules.
    pub terrain: Arc<TerrainTable>,
    /// Classes.
    pub classes: Arc<ClassTable>,
    /// Items.
    pub items: Arc<ItemTable>,
    /// Spells.
    pub spells: Arc<SpellTable>,
    /// Skills.
    pub skills: Arc<SkillTable>,
    /// Combat Arts.
    pub arts: Arc<ArtTable>,
}

/// The player's game between battles: saved by 0802.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Campaign {
    /// Classic or Casual, picked at New Game.
    pub mode: GameMode,
    /// Who the player made the lead.
    pub lead: LeadProfile,
    /// The chapter being played (or next, between chapters), by id.
    pub chapter: String,
    /// The army, in roster order.
    pub roster: Vec<Unit>,
    /// Items not in a loadout.
    pub stock: Stock,
    /// The party's gold.
    pub gold: Gold,
    /// Story flags set by chapters.
    pub flags: BTreeMap<String, bool>,
    /// Seconds played.
    pub playtime_s: u64,
}

/// What a won battle gave, for the results (0810).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BattleRewards {
    /// The battle's clear gold.
    pub clear_gold: Gold,
    /// Unused rewind charges.
    pub unused_charges: u8,
    /// EXP each deployed unit was offered for them.
    pub bonus_exp: u32,
    /// The bonus's EXP and level-up events, with the battle's unit ids.
    pub events: Vec<Event>,
    /// Characters who died (Classic) and left the roster.
    pub lost: Vec<CharacterId>,
    /// Characters who joined.
    pub joined: Vec<CharacterId>,
}

/// Why a battle's result wasn't applied. Nothing changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyError {
    /// The battle wasn't won (a defeat, or not over).
    NotWon,
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::NotWon => f.write_str("the battle wasn't won"),
        }
    }
}

impl std::error::Error for ApplyError {}

impl Campaign {
    /// A new game at `chapter`, with `roster` (made from the character
    /// data), `gold` and `stock`. The lead's unit takes `lead`'s name.
    pub fn new_game(
        mode: GameMode,
        lead: LeadProfile,
        chapter: impl Into<String>,
        mut roster: Vec<Unit>,
        gold: Gold,
        stock: Stock,
    ) -> Self {
        for u in &mut roster {
            if u.character.as_ref().is_some_and(|c| c.0 == LEAD_ID) {
                u.name.clone_from(&lead.name);
                u.map_label = default_map_label(&lead.name);
            }
            rest(u);
        }
        Self {
            mode,
            lead,
            chapter: chapter.into(),
            roster,
            stock,
            gold,
            flags: BTreeMap::new(),
            playtime_s: 0,
        }
    }

    /// The roster member playing `character`.
    pub fn member(&self, character: &CharacterId) -> Option<&Unit> {
        self.roster
            .iter()
            .find(|u| u.character.as_ref() == Some(character))
    }

    /// The battle `def` with this campaign's army (see the module docs).
    pub fn battle_setup(&self, def: &BattleDef, tables: &GameTables) -> BattleSetup {
        let players = def
            .player_slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| {
                let mut unit = self.member(&slot.character)?.clone();
                unit.id = BattleDef::slot_id(i);
                unit.pos = slot.pos;
                Some(unit)
            });
        let units = players.chain(def.enemies.iter().cloned()).collect();
        BattleSetup {
            map: def.map.clone(),
            terrain: Arc::clone(&tables.terrain),
            classes: Arc::clone(&tables.classes),
            items: Arc::clone(&tables.items),
            spells: Arc::clone(&tables.spells),
            skills: Arc::clone(&tables.skills),
            arts: Arc::clone(&tables.arts),
            pack: BattlePack {
                items: def.default_pack.clone(),
                cap: def.pack_cap,
            },
            gold: self.gold,
            stock: self.stock.clone(),
            units,
            reinforcements: def.reinforcements.clone(),
            objective: def.objective,
            rewind_charges: def.difficulty.rewind_charges(),
            seed: def.seed,
            triggers: def.triggers.clone(),
            mode: self.mode,
        }
    }

    /// Applies the won battle `state` of `def`, with `unused_charges`
    /// rewind charges left (see the module docs).
    pub fn apply_result(
        &mut self,
        def: &BattleDef,
        state: &BattleState,
        unused_charges: u8,
    ) -> Result<BattleRewards, ApplyError> {
        if state.outcome() != Some(Outcome::Victory) {
            return Err(ApplyError::NotWon);
        }
        let mut rewards = BattleRewards {
            clear_gold: def.clear_gold,
            unused_charges,
            ..BattleRewards::default()
        };
        let mut stock = state.stock().clone();
        let is_player = |u: &&Unit| u.faction == Faction::Player && u.character.is_some();
        let standing = state.units().iter().filter(is_player);
        let fallen = state.fallen().iter().filter(is_player);
        // Deployed units still in the roster, for the bonus, in slot order.
        let mut deployed: Vec<Unit> = Vec::new();
        for u in standing {
            deployed.push(u.clone());
        }
        for u in fallen {
            match self.mode {
                GameMode::Classic => {
                    self.lose(u, &mut stock);
                    rewards.lost.extend(u.character.clone());
                }
                GameMode::Casual => deployed.push(u.clone()),
            }
        }
        deployed.sort_by_key(|u| u.id);
        let per_charge = EXP_PER_LEVEL * UNUSED_CHARGE_PERCENT / 100;
        rewards.bonus_exp = (per_charge * u32::from(unused_charges)).min(MAX_AWARD);
        let mut rng = state.rng().clone();
        for mut u in deployed {
            let events = grant_exp(&mut u, rewards.bonus_exp, state.classes(), &mut rng);
            rewards.events.extend(events);
            self.replace(u);
        }
        for r in state.recruited() {
            let joined = r.character.clone();
            if joined.as_ref().is_some_and(|c| self.member(c).is_none()) {
                self.roster.push(Unit {
                    faction: Faction::Player,
                    role: Role::Regular,
                    ..r.clone()
                });
                rewards.joined.extend(joined);
            }
        }
        stock.unpack(state.pack().clone());
        self.stock = stock;
        self.gold = state.gold().saturating_add(def.clear_gold);
        for u in &mut self.roster {
            rest(u);
        }
        Ok(rewards)
    }

    /// Classic → Casual. Returns whether the mode changed (Casual never
    /// goes back).
    pub fn downgrade_mode(&mut self) -> bool {
        let was = self.mode;
        self.mode = GameMode::Casual;
        was != self.mode
    }

    /// Replaces `unit`'s roster entry (by character) with it.
    fn replace(&mut self, unit: Unit) {
        if let Some(entry) = self
            .roster
            .iter_mut()
            .find(|r| r.character == unit.character)
        {
            *entry = unit;
        }
    }

    /// Removes the dead `unit` from the roster; its loadout goes to `stock`.
    fn lose(&mut self, unit: &Unit, stock: &mut Stock) {
        self.roster.retain(|r| r.character != unit.character);
        let loadout = &unit.loadout;
        stock
            .weapons
            .extend(loadout.weapons.iter().flatten().cloned());
        for item in loadout.armour.iter().chain(&loadout.accessory) {
            stock.add(item.clone());
        }
    }
}

/// Makes `unit` ready for the next battle: a player unit at full HP, not
/// acted, with no timed effects.
fn rest(unit: &mut Unit) {
    unit.hp = unit.stats.hp;
    unit.acted = false;
    unit.effects.clear();
}

#[cfg(test)]
mod tests;
