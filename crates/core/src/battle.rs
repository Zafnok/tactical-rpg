//! A battle: its state, the [`Command`]s that change it and the [`Event`]s
//! they produce (ADR-0004 rule 2). The UI animates events, the AI issues the
//! same commands as the player, and saves and replays store the state and
//! the commands.
//!
//! # Rules
//!
//! Sources: `docs/design/turn-structure.md` (phases, actions, reinforcements,
//! turn limits) and `docs/design/death-and-difficulty.md` (falling units,
//! loss conditions).
//!
//! - **Phases.** Each turn runs `Player → Enemy → Other` ([`Phase`]; `Other`
//!   is the `Ally` and `Neutral` factions), then `turn += 1`. The battle
//!   starts at turn 1, Player phase. [`Command::EndPhase`] moves to the next
//!   phase that has living units or reinforcements arriving; a phase without
//!   either is skipped (no event). The turn still advances after the Other
//!   phase's slot, whether or not it ran.
//! - **Phase start**, in this order: every unit of the phase becomes ready
//!   (`acted = false`); the phase's reinforcements that are due arrive,
//!   already done ([`Event::UnitsArrived`]); then [`Event::PhaseStarted`]
//!   (before all that, tiles the phase's side set burning burn out, see
//!   *Terrain* below)
//!   (the banner). Units of other phases keep their `acted` flag (drawn
//!   dimmed until their own phase).
//! - **Auto-end is not here.** Ending the player phase when every unit has
//!   acted is a player setting handled by the battle screen, which issues
//!   `EndPhase` itself; AI phases end when the AI issues `EndPhase`.
//! - **Acting.** [`Command::Act`] commits a move and one action together, so
//!   the UI can let the player cancel a move freely before choosing the
//!   action. The unit must be on the map, belong to the current phase, not
//!   have acted, and `dest` must be a tile it can stop on
//!   ([`reachable`]). After the action it is done. No
//!   Canto: a unit never moves after its action (but see *Extending* below).
//! - **Attacking** names a loadout slot. The weapon there must be one the
//!   unit can wield, and the target a hostile unit on the map within its
//!   range *from `dest`*. Attacking equips that weapon ([`Event::Equipped`]
//!   if it changed). The defender counters with its equipped attack (a weapon
//!   it can wield, or an attack spell with uses left), if the attacker is in
//!   range. The combat uses [`forecast`]
//!   and [`resolve`] with the battle's [`SimRng`] and the item table's
//!   [`combat_rules`](ItemTable::combat_rules); each side's stats are
//!   gear-adjusted ([`Unit::combat_input`]); terrain comes from each unit's
//!   tile. Normal attacks cost no durability.
//! - **Spells** ([`UnitAction::Cast`], rules in [`crate::spell`]): the unit
//!   must have learned the spell and have a use left. At battle start every
//!   unit gets full uses and, if it has nothing equipped, its
//!   [default equip](Unit::default_equip) ([`Unit::prepare_for_battle`]).
//!   - An **attack spell** targets a hostile unit in the spell's range from
//!     `dest`, equips the spell ([`Event::Equipped`] if it changed) and fights
//!     exactly like a weapon attack ([`Event::SpellCast`], then
//!     [`Event::CombatResolved`]). Each side that struck with a spell spends 1
//!     use, however many strikes it made ([`Event::SpellUsesChanged`],
//!     attacker first, right after the combat); a defender that fell before
//!     striking spends nothing. Spells give no weapon EXP.
//!   - A **heal spell** targets an ally ([`Faction::is_allied_to`]) other than
//!     the caster, in range from `dest`, below max HP. It restores
//!     `min(heal_power + Mag, max HP − HP)` (gear-adjusted Mag), with no roll
//!     and no counter ([`Event::SpellCast`], [`Event::Healed`],
//!     [`Event::SpellUsesChanged`]).
//!   - A **tile cast** ([`CastTarget::Tile`], `magic.md` "Terrain magic")
//!     needs a spell with a [`terrain_effect`](SpellDef::terrain_effect). The
//!     tile must be on the map, in the spell's range from `dest`, empty (no
//!     unit, the caster at `dest` included) and of a terrain in the effect's
//!     `from`. An attack spell is equipped, as for an attack
//!     ([`Event::Equipped`] if it changed; Nick). The tile becomes `to` at
//!     once ([`Event::SpellCast`], [`Event::TerrainChanged`],
//!     [`Event::SpellUsesChanged`]); no damage, no counter. Casting at a unit
//!     never changes terrain.
//! - **Terrain** lives in the battle's own copy of the map
//!   ([`BattleState::map`]), which movement and combat read, so a changed
//!   tile counts at once. It changes only with an [`Event::TerrainChanged`].
//!   A tile whose effect lasts
//!   [until the caster's next phase](EffectDuration::UntilCastersNextPhase)
//!   (a burning forest) is [`Burning`]: when the caster's side's phase next
//!   comes round, first thing, a unit standing on it takes its `damage`
//!   ([`Event::BurnDamage`]; never below 1 HP, not reduced by Def or Res),
//!   then it becomes its `then` terrain; only then do reinforcements arrive
//!   (Nick). The only unit that can be on a burning tile is a reinforcement
//!   that arrived there. This happens even if that phase is then skipped for
//!   having no units (*Claude's starting rule*: otherwise a fire cast by a
//!   side that was wiped out would burn for ever).
//! - **Weapon EXP.** After a combat, each side still on the map that struck
//!   gains weapon EXP in its weapon's kind ([`Event::WeaponExpGained`], then
//!   [`Event::WeaponRankUp`] if its rank rose; attacker first), before
//!   anyone falls. See [`crate::item`] for the formula; a Combat Art
//!   doubles its user's base.
//! - **Items** ([`UnitAction::UseItem`]): a player unit uses a consumable
//!   from the shared [`BattlePack`]. The target is the unit itself or a non-hostile
//!   unit adjacent to `dest`. The item is used up ([`Event::ItemUsed`]),
//!   heals ([`Event::Healed`], never above max HP) and ends the action.
//! - **Shops** ([`UnitAction::Shop`]): a player unit on a shop tile applies
//!   one or more [`ShopTxn`]s in order, with the party's
//!   [`gold`](BattleState::gold) (rules in [`crate::shop`]). An empty list is
//!   refused (leaving a shop without doing anything is not an action).
//!   Bought weapons, armour and accessories go to the unit's free loadout
//!   slot (armour only if its class wears it), else the
//!   [`stock`](BattleState::stock); a bought weapon is equipped if the unit
//!   had none equipped and can wield it. Bought consumables go into the
//!   battle pack, past its cap if need be. Selling the equipped weapon equips
//!   the first other weapon the unit can wield (if any). Each transaction
//!   emits [`Event::Bought`], [`Event::Sold`] or [`Event::Repaired`] (then
//!   [`Event::Equipped`] if that changed), then [`Event::GoldChanged`]. If
//!   any transaction fails, none is applied.
//! - **Chests** ([`UnitAction::Open`]): a player unit on an unopened chest
//!   opens it ([`Event::ChestOpened`]): gold goes to the party (then
//!   [`Event::GoldChanged`]), a consumable into the pack, equipment into the
//!   stock. A chest opens once. Villages are on hold (no village tile,
//!   `docs/design/terrain.md`).
//! - **Skills** ([`crate::skill`] has the skill rules):
//!   - **Passives** of both sides, the **timed effects** on them, and the
//!     **ally auras** of other allied units (green ones too) in reach feed
//!     each combat: [`CombatMods`](crate::combat::CombatMods) and stat
//!     bonuses on its [`CombatantInput`], so the forecast shows them. Stat
//!     bonuses from skills count in combat only and may pass the hard ceilings. "Moved"
//!     conditions count the tiles of the attacker's path; a unit attacked
//!     has moved 0.
//!   - **Combat actives** are an option of an attack
//!     ([`UnitAction::Attack`]'s `active`, paid from the attacking weapon)
//!     or of an attack spell's cast ([`UnitAction::Cast`]'s `active`: spell
//!     actives, paid with 1 extra use). The unit must be able to
//!     [use](Unit::usable_active) it, the attack must suit it
//!     ([`WeaponReq`](crate::skill::WeaponReq)) and the cost must be payable
//!     ([`check_cost`](crate::skill::check_cost)). Events:
//!     [`Event::SkillUsed`], then the payment ([`Event::DurabilitySpent`] or
//!     [`Event::SpellUsesChanged`]), then the combat. Its bonuses count in
//!     this combat; a stance rider counts in it too if its
//!     [`this_combat`](crate::skill::Stance::this_combat) says so (once), and
//!     is then applied ([`Event::EffectApplied`]) if the user still stands; a drain
//!     heals the user after the combat ([`Event::Healed`]). A weapon the
//!     payment brought to 0 breaks after the combat ([`Event::ItemBroke`],
//!     before anyone falls).
//!   - **Other actives** ([`UnitAction::UseSkill`]) are paid from the
//!     **equipped** weapon (so a unit with a spell or nothing equipped can't
//!     use them) and end the action: a buff on the user or on the other
//!     allied units in reach of `dest` ([`Event::EffectApplied`] each; none
//!     in reach: refused), a heal of the wounded other allied units in reach
//!     by `Mag + power` + [`heal_bonus`] (none wounded: refused), or
//!     **Shove**. Area actives
//!     never include the user (*Claude's starting rule*).
//!   - **Shove** pushes a hostile unit adjacent to `dest` 1 tile straight
//!     away from it ([`Event::Pushed`]). If that tile is off the map, holds
//!     a unit or can't be entered by the target, the target **stays** and
//!     takes the skill's collision damage (Nick). Pushed into a burning tile
//!     (`magic.md`), it takes the collision damage and lands on the nearest
//!     free tile it can stand on: the first free neighbour of the burning
//!     tile in [`Dir::ALL`](crate::geom::Dir::ALL) order (its own tile is
//!     one, so it never lands further away). A unit it is pushed into takes
//!     the same damage ([`Event::CollisionDamage`]). Collisions can make
//!     either unit fall ([`Event::UnitFell`], pushed unit first; Nick).
//!   - **Moving after an attack** (`turn-structure.md`): after an attack
//!     with a post-action move (Vault, Swoop), if the unit still stands
//!     and has somewhere to go, it is offered the move
//!     ([`Event::MoveAfterOffered`], instead of `UnitActed`) and chooses it
//!     after seeing the combat (Nick): [`Command::MoveAfter`] to a tile within
//!     that many steps through empty tiles it can enter
//!     ([`BattleState::move_after_tiles`]), or to stay. Until then every
//!     other command is refused ([`CommandError::MoveAfterPending`]). The
//!     move ends its action ([`Event::UnitMoved`], [`Event::UnitActed`]).
//!   - **Timed effects** end at the start of their
//!     [`until`](crate::skill::TimedEffect::until) phase, right after its
//!     burn-outs, even if that phase is then skipped
//!     ([`Event::EffectExpired`]).
//!   - **Heal spells** and Sanctuary restore the caster's passives'
//!     [`heal_bonus`] more (White Magic; Nick).
//!   - **Who uses them**: player units, enemy [bosses](crate::unit::Role::Boss) and
//!     green units that aren't [non-combat](crate::unit::Role::Noncombatant) ones; any
//!     other unit is refused actives and arts
//!     ([`CommandError::ArtsNotAllowed`]; `combat-arts.md`).
//! - **Combat Arts** ([`crate::art`] has the art rules;
//!   `docs/design/combat-arts.md`): an option of a weapon attack
//!   ([`UnitAction::Attack`]'s `art`), never of a counter or a cast. An
//!   attack uses one art **or** one combat active
//!   ([`CommandError::ArtWithActive`]). The art must be one of the unit's
//!   [arts for](Unit::arts_for) the attacking weapon, and its cost payable
//!   from that weapon ([`CommandError::CannotPayArt`]: broken, or
//!   `durability_left < cost`). Events: [`Event::ArtUsed`], the payment
//!   ([`Event::DurabilitySpent`]), a stance ([`Event::EffectApplied`], on
//!   the user until the start of its next phase; it counts in this combat,
//!   once), then the combat, with the art's bonuses on **every** attacker
//!   strike ([`ArtEffect::apply`](crate::art::ArtEffect::apply)). After the
//!   combat's spell uses:
//!   - a **debuff** on the target if one of the attacker's strikes hit and
//!     the target still stands ([`Event::EffectApplied`], until the end of
//!     the target's next phase; the same art refreshes it). Mov and Spd
//!     debuffs feed movement, the danger zone and attack speed;
//!   - **Line Pierce**: if the attacker still stands and the tile one step
//!     past the target, on the straight or diagonal line from `dest`, holds
//!     a unit hostile to it (any other angle has no such tile), one strike at that unit, even if the
//!     target fell: the normal formulas against that unit, no counter, no
//!     extra strikes, rolled after the combat's strikes. It is its own
//!     [`Event::CombatResolved`] (a second combat for unit EXP and class
//!     points, 0601), with numbers worked out when the attack is validated.
//!
//!   Weapon EXP then counts both (one award; the pierce's damage adds to
//!   `dealt`), with the base doubled for the art's user. A weapon the
//!   payment brought to 0 fought unbroken and breaks after
//!   ([`Event::ItemBroke`]), before anyone falls (target, the pierced
//!   unit, then the attacker). [`BattleState::preview_attack`] gives the UI
//!   the forecast, the durability change and the art's
//!   [notes](crate::art::ArtNote) before committing.
//! - **Unit EXP and class points** ([`crate::progression`] has the rules;
//!   `docs/design/progression.md`), for **player units** only, each an
//!   award of EXP ([`Event::ExpGained`], then [`Event::LeveledUp`] and any
//!   [`Event::SpellLearned`]) then class points ([`Event::ClassPointsGained`],
//!   then [`Event::ClassLeveledUp`], [`Event::SpellLearned`],
//!   [`Event::ClassMastered`] and [`Event::SkillLearned`] as they come):
//!   - after an attack, once the fallen are gone: for each combat (a Line
//!     Pierce strike is its own), each side that struck and still stands,
//!     attacker first, gets the combat award (kill, damage or no damage,
//!     against the other unit's character level; a boss kill +40). A
//!     defender that couldn't counter took no part and gets nothing;
//!   - after a heal spell, a tile cast or a non-combat active, the user gets
//!     its award. A non-combat active gives the higher of its own award, a
//!     heal's if it healed (Sanctuary) and a kill's for each hostile unit
//!     it felled (Shove), with the kill's extra CP if it felled one (Nick).
//!
//!   **Ally**-faction units' EXP goes into the battle's
//!   [EXP pool](BattleState::exp_pool) instead. On a **victory**, just before
//!   [`Event::BattleEnded`], the pool is shared: each player unit on the map
//!   below the level cap (fallen units and reinforcements still to come
//!   don't count) gets `pool / their number` EXP, in unit order; the
//!   remainder is lost. On a defeat the pool is lost. Enemies and neutrals
//!   never gain.
//!
//!   **RNG order:** level ups roll on the battle's [`SimRng`] right where
//!   their [`Event::LeveledUp`] is, after the combat's strikes (and Line
//!   Pierce's): 7 [`roll_percent`](crate::rng::RandomSource::roll_percent)
//!   calls each, then a [`roll_below`](crate::rng::RandomSource::roll_below)
//!   per safety-net pick.
//! - **Equipping** ([`Command::Equip`]) is free: any weapon of the loadout
//!   the unit can wield, or any learned attack spell (even one with no uses
//!   left: it just can't counter), by a ready unit of the current phase. It
//!   doesn't end the action. No trading: loadouts are fixed for the battle.
//! - **Falling.** A unit at 0 HP falls ([`Event::UnitFell`]): it leaves the
//!   map, can't act, can't be targeted and doesn't block tiles. It moves to
//!   [`BattleState::fallen`]. Classic vs Casual only matters when the
//!   campaign applies the result (0801).
//! - **Outcome.** Defeat, in both modes, when a player lord falls or no player
//!   unit is left on the map. Victory per [`Objective`]. Both are checked
//!   after every command and when a turn ends; nothing else changes them
//!   (arrivals can't create a victory or a defeat). Loss is checked first.
//!   Once decided ([`Event::BattleEnded`]) every command fails with
//!   [`CommandError::BattleOver`] and the outcome never changes.
//! - **Survive `N`** wins, and a `turn_limit` of `N` loses, when turn `N`'s
//!   last phase ends: the moment `turn` would become `N + 1`.
//! - **Reinforcements** ([`Reinforcement`]) for turn `N` arrive at the start
//!   of their faction's phase on turn `N`, never acting on arrival. One whose
//!   tile is occupied waits and tries again at the same point next turn;
//!   the others of its wave still arrive. Earlier entries go first. A
//!   reinforcement arrives on a burning tile anyway, and takes the fire's
//!   damage when it burns out (Nick).
//! - **Errors change nothing.** [`BattleState::apply`] validates the whole
//!   command before touching the state, so on `Err` the state is unchanged.
//!
//! # Extending
//!
//! Every [`Event`] sequence of an `Act` ends with [`Event::UnitActed`] (unless
//! the unit fell, or is offered a move after its attack:
//! [`Event::MoveAfterOffered`], then `UnitActed` comes with the
//! [`Command::MoveAfter`]), optionally followed by [`Event::BattleEnded`]
//! (after the EXP pool's shares, on a victory).
//! Combat Arts pay their costs with [`pay_cost`](crate::skill::pay_cost),
//! as actives do.
//!
//! # Saving
//!
//! `BattleState` is serde-serialisable (RON via `content`/`app`, ADR-0019):
//! the map (with its current terrain), burning tiles, units (with their
//! learned skills, timed effects, role, EXP and class records), fallen units,
//! the EXP pool,
//! pending reinforcements, objective, turn,
//! phase, RNG position, battle pack, gold, stock, opened chests and outcome
//! are all saved (spell uses left live on the units). The
//! **terrain, class, item, spell, skill and art tables are not**: they are shared content, held by `Arc` and skipped. A
//! deserialised state has empty tables (every `Act` fails with
//! [`CommandError::UnknownClass`]) until [`BattleState::restore_tables`] is
//! called with the game's tables, as loaded from the same content. The map is
//! saved because it is battle state (terrain magic changes tiles).

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::art::{ArtId, ArtTable};
use crate::class::{ClassDef, ClassId, ClassLevel, ClassPoints, ClassTable};
use crate::combat::{CombatHp, CombatOutcome, CombatantInput, Forecast, Side, forecast, resolve};
use crate::geom::Pos;
use crate::item::{
    BattlePack, ConsumableEffect, Equipped, ItemDef, ItemId, ItemTable, Stock, WEAPON_SLOTS,
};
use crate::map::BattleMap;
use crate::movement::{MoveError, reachable};
use crate::progression::{self, CombatResult, StatGains};
use crate::rng::SimRng;
use crate::shop::{self, Gold, Loot, Shop, ShopError};
use crate::skill::{CostError, EffectSource, SkillId, SkillTable, heal_bonus};
use crate::spell::{EffectDuration, SpellDef, SpellId, SpellKind, SpellTable, TerrainEffect};
use crate::stats::StatValue;
use crate::terrain::{TerrainId, TerrainTable};
use crate::unit::{Faction, Level, Role, Unit, UnitId};
use crate::weapon::{WeaponKind, WeaponRank};

/// A turn number, from 1.
pub type Turn = u32;

/// A phase of a turn: which factions act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// The `Player` faction; controlled by the player.
    Player,
    /// The `Enemy` faction; AI.
    Enemy,
    /// The `Ally` and `Neutral` factions ("green" units); AI.
    Other,
}

impl Phase {
    /// Every phase, in turn order.
    pub const ALL: [Phase; 3] = [Phase::Player, Phase::Enemy, Phase::Other];

    /// The phase in which units of `faction` act.
    pub fn of(faction: Faction) -> Phase {
        match faction {
            Faction::Player => Phase::Player,
            Faction::Enemy => Phase::Enemy,
            Faction::Ally | Faction::Neutral => Phase::Other,
        }
    }

    /// The phase after this one (`Other` → `Player`, which starts a new turn).
    #[must_use]
    pub fn next(self) -> Phase {
        match self {
            Phase::Player => Phase::Enemy,
            Phase::Enemy => Phase::Other,
            Phase::Other => Phase::Player,
        }
    }
}

/// How a battle ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    /// The objective was met.
    Victory,
    /// A loss condition was met: game over.
    Defeat,
}

/// What the player must do to win. Every objective except [`Survive`]
/// may have a `turn_limit`: the player loses if it isn't met when that
/// turn's last phase ends.
///
/// [`Survive`]: Objective::Survive
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Objective {
    /// Defeat every enemy on the map.
    Rout {
        /// Last turn to do it in.
        turn_limit: Option<Turn>,
    },
    /// Defeat one unit (a boss).
    DefeatUnit {
        /// The unit to defeat.
        unit: UnitId,
        /// Last turn to do it in.
        turn_limit: Option<Turn>,
    },
    /// Move a player unit onto a tile and choose [`UnitAction::Seize`].
    Seize {
        /// The tile to seize.
        pos: Pos,
        /// Whether only a lord may seize.
        by_lord: bool,
        /// Last turn to do it in.
        turn_limit: Option<Turn>,
    },
    /// Don't lose until the end of turn `turns`.
    Survive {
        /// The turn whose end wins.
        turns: Turn,
    },
}

impl Objective {
    /// The turn limit, if the objective has one.
    pub fn turn_limit(&self) -> Option<Turn> {
        match *self {
            Objective::Rout { turn_limit }
            | Objective::DefeatUnit { turn_limit, .. }
            | Objective::Seize { turn_limit, .. } => turn_limit,
            Objective::Survive { .. } => None,
        }
    }
}

/// A unit that joins the battle later. Its faction decides its phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reinforcement {
    /// The turn it arrives on (at the start of its faction's phase).
    pub turn: Turn,
    /// The unit, at its arrival tile. Its id must be unique in the battle.
    pub unit: Unit,
}

/// Everything needed to start a battle. Content validation (map files,
/// 0803) guarantees unique unit ids, one unit per tile and units on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSetup {
    /// The battlefield.
    pub map: BattleMap,
    /// Terrain rules for the map's tiles.
    pub terrain: Arc<TerrainTable>,
    /// Classes of every unit.
    pub classes: Arc<ClassTable>,
    /// Every item units carry.
    pub items: Arc<ItemTable>,
    /// Every spell units know.
    pub spells: Arc<SpellTable>,
    /// Every skill units have.
    pub skills: Arc<SkillTable>,
    /// Every Combat Art.
    pub arts: Arc<ArtTable>,
    /// The player side's consumables.
    pub pack: BattlePack,
    /// The party's gold, from the campaign.
    pub gold: Gold,
    /// The party's stock (items not brought in), from the campaign; gets
    /// bought and found equipment that doesn't go in a loadout.
    pub stock: Stock,
    /// The units on the map at the start.
    pub units: Vec<Unit>,
    /// Units that arrive later.
    pub reinforcements: Vec<Reinforcement>,
    /// How to win.
    pub objective: Objective,
    /// Rewind charges for this battle, from the map's difficulty tier
    /// (0801); spent by turn rewind (0307).
    pub rewind_charges: u8,
    /// Seed of the battle's [`SimRng`].
    pub seed: u64,
}

/// What a unit does after moving. Ends its action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnitAction {
    /// Nothing.
    Wait,
    /// Fight `target` with the weapon in loadout slot `slot` (equips it).
    Attack {
        /// The unit attacked.
        target: UnitId,
        /// The weapon's loadout slot.
        slot: usize,
        /// A combat active to use in this attack.
        active: Option<SkillId>,
        /// A Combat Art to use in this attack (not with an active).
        art: Option<ArtId>,
    },
    /// Use a consumable on `target` (the unit itself or an adjacent ally).
    UseItem {
        /// Index in the battle pack.
        pack_index: usize,
        /// Who it is used on.
        target: UnitId,
    },
    /// Seize the objective tile (only on it, see [`Objective::Seize`]).
    Seize,
    /// Buy, sell and repair at the shop on `dest`, in order (at least one).
    Shop {
        /// The transactions.
        txns: Vec<ShopTxn>,
    },
    /// Open the chest on `dest`.
    Open,
    /// Cast a learned spell (an attack spell also equips it).
    Cast {
        /// The spell.
        spell: SpellId,
        /// What it is cast on.
        target: CastTarget,
        /// A spell active to use with an attack spell cast at a unit.
        active: Option<SkillId>,
    },
    /// Use a non-combat active skill, paid from the equipped weapon.
    UseSkill {
        /// The skill.
        skill: SkillId,
        /// The unit it is used on (Shove), or `None` (every other skill).
        target: Option<UnitId>,
    },
}

/// What a [`UnitAction::Cast`] is cast on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CastTarget {
    /// A unit: a hostile one for an attack spell, an ally for a heal.
    Unit(UnitId),
    /// An empty tile, to change its terrain (spells with a terrain effect).
    Tile(Pos),
}

/// A tile set burning by a tile cast, waiting to burn out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Burning {
    /// The tile.
    pub pos: Pos,
    /// The caster's phase: the tile burns out when it next starts.
    pub phase: Phase,
    /// The terrain it becomes then.
    pub then: TerrainId,
    /// Damage to a unit standing on it then.
    pub damage: StatValue,
}

/// One transaction of a [`UnitAction::Shop`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShopTxn {
    /// Buy one of an item the shop sells.
    Buy {
        /// The item.
        item: ItemId,
    },
    /// Sell one of the unit's items, or one from the battle pack.
    Sell {
        /// Which item.
        from: SellFrom,
    },
    /// Repair the weapon in a loadout slot (at a blacksmith).
    Repair {
        /// The loadout slot.
        slot: usize,
    },
}

/// Which item a [`ShopTxn::Sell`] sells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SellFrom {
    /// The weapon in this loadout slot.
    Weapon(usize),
    /// The worn armour.
    Armour,
    /// The worn accessory.
    Accessory,
    /// The battle pack item at this index.
    Pack(usize),
}

/// Where a bought item went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Destination {
    /// This loadout weapon slot.
    WeaponSlot(usize),
    /// The loadout armour slot.
    Armour,
    /// The loadout accessory slot.
    Accessory,
    /// The battle pack.
    Pack,
    /// The party's stock.
    Stock,
}

/// A request to change the battle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    /// Move `unit` to `dest` (its own tile to stay) and do `action`.
    Act {
        /// The unit acting.
        unit: UnitId,
        /// Where it ends its move.
        dest: Pos,
        /// What it does there.
        action: UnitAction,
    },
    /// Equip a weapon of `unit`'s loadout or one of its attack spells.
    /// Free: doesn't end the action.
    Equip {
        /// The unit.
        unit: UnitId,
        /// What to equip.
        equipped: Equipped,
    },
    /// Move `unit` after its attack ([`Event::MoveAfterOffered`]) to `to`,
    /// or stay (`None`). Ends its action.
    MoveAfter {
        /// The unit waiting to move.
        unit: UnitId,
        /// Where it moves.
        to: Option<Pos>,
    },
    /// End the current phase.
    EndPhase,
}

/// A unit that may still move after its attack, waiting for
/// [`Command::MoveAfter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingMove {
    /// The unit.
    pub unit: UnitId,
    /// How many tiles it may move.
    pub tiles: u32,
}

/// Something that happened, for the UI to show. See the module docs for the
/// order of events.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
    /// A phase began (the banner).
    PhaseStarted {
        /// The turn.
        turn: Turn,
        /// The phase.
        phase: Phase,
    },
    /// Reinforcements appeared at the start of their phase, already done.
    UnitsArrived {
        /// The arrivals, in arrival order.
        units: Vec<UnitId>,
    },
    /// A unit moved along `path` (its start tile first).
    UnitMoved {
        /// The unit.
        unit: UnitId,
        /// Every tile of the move.
        path: Vec<Pos>,
    },
    /// A unit equipped a weapon or an attack spell.
    Equipped {
        /// The unit.
        unit: UnitId,
        /// What it equipped.
        equipped: Equipped,
    },
    /// A unit cast a spell (before the combat or heal it causes).
    SpellCast {
        /// The caster.
        unit: UnitId,
        /// The spell.
        spell: SpellId,
        /// What it was cast on.
        target: CastTarget,
    },
    /// A tile's terrain changed (a tile cast, or a burning tile burning
    /// out).
    TerrainChanged {
        /// The tile.
        pos: Pos,
        /// Its terrain before.
        from: TerrainId,
        /// Its terrain now.
        to: TerrainId,
    },
    /// A burning tile burnt a unit standing on it, as it burnt out (just
    /// before its [`Event::TerrainChanged`]).
    BurnDamage {
        /// The unit.
        unit: UnitId,
        /// Its tile.
        pos: Pos,
        /// HP lost (the unit keeps at least 1).
        amount: StatValue,
    },
    /// A unit spent a spell use.
    SpellUsesChanged {
        /// The unit.
        unit: UnitId,
        /// The spell.
        spell: SpellId,
        /// Uses left.
        uses_left: u8,
    },
    /// A combat was fought.
    CombatResolved {
        /// Who attacked.
        attacker: UnitId,
        /// Who was attacked.
        defender: UnitId,
        /// The numbers the combat used.
        forecast: Forecast,
        /// Every strike and the final HP.
        outcome: CombatOutcome,
    },
    /// A unit gained weapon EXP after a combat.
    WeaponExpGained {
        /// The unit.
        unit: UnitId,
        /// The weapon kind.
        kind: WeaponKind,
        /// EXP gained.
        amount: u32,
    },
    /// A unit's weapon rank rose.
    WeaponRankUp {
        /// The unit.
        unit: UnitId,
        /// The weapon kind.
        kind: WeaponKind,
        /// The new rank.
        rank: WeaponRank,
    },
    /// A player unit gained unit EXP (cut to what the level cap allows).
    ExpGained {
        /// The unit.
        unit: UnitId,
        /// EXP gained.
        amount: u32,
    },
    /// A unit's character level rose (right after its
    /// [`Event::ExpGained`]; several in a row for a big award).
    LeveledUp {
        /// The unit.
        unit: UnitId,
        /// The new character level.
        level: Level,
        /// What each stat gained (current HP rose with max HP).
        gains: StatGains,
    },
    /// A player unit's current class gained class points.
    ClassPointsGained {
        /// The unit.
        unit: UnitId,
        /// The class.
        class: ClassId,
        /// CP gained (cut at mastery's total).
        amount: ClassPoints,
    },
    /// A unit's class level rose in its current class.
    ClassLeveledUp {
        /// The unit.
        unit: UnitId,
        /// The class.
        class: ClassId,
        /// The new class level.
        class_level: ClassLevel,
    },
    /// A unit reached the class level cap: its active is now permanent
    /// (followed by an [`Event::SkillLearned`] per passive).
    ClassMastered {
        /// The unit.
        unit: UnitId,
        /// The class.
        class: ClassId,
    },
    /// A unit learned a skill (a passive, on mastery).
    SkillLearned {
        /// The unit.
        unit: UnitId,
        /// The skill.
        skill: SkillId,
    },
    /// A unit learned a spell (a class spell at a class level, or a
    /// personal spell at a character level). It has no uses until the
    /// next battle.
    SpellLearned {
        /// The unit.
        unit: UnitId,
        /// The spell.
        spell: SpellId,
    },
    /// A unit used an active skill (before its payment and its effects).
    SkillUsed {
        /// The user.
        unit: UnitId,
        /// The skill.
        skill: SkillId,
    },
    /// A unit paid durability for a skill.
    DurabilitySpent {
        /// The unit.
        unit: UnitId,
        /// The weapon's loadout slot.
        slot: usize,
        /// The weapon.
        item: ItemId,
        /// Durability spent.
        amount: u32,
        /// Durability left.
        left: u32,
    },
    /// A unit used a Combat Art in its attack (before the payment's
    /// [`Event::DurabilitySpent`] and the combat).
    ArtUsed {
        /// The attacker.
        unit: UnitId,
        /// The art.
        art: ArtId,
        /// The weapon that pays.
        weapon: ItemId,
        /// Its durability before.
        durability_before: u32,
        /// Its durability after.
        durability_after: u32,
    },
    /// A timed effect was put on a unit (or refreshed).
    EffectApplied {
        /// The unit.
        unit: UnitId,
        /// The skill or art it comes from.
        source: EffectSource,
        /// It ends at the start of this phase.
        until: Phase,
    },
    /// A timed effect ended.
    EffectExpired {
        /// The unit.
        unit: UnitId,
        /// The skill or art it came from.
        source: EffectSource,
    },
    /// A unit was pushed.
    Pushed {
        /// The unit.
        unit: UnitId,
        /// Its tile before.
        from: Pos,
        /// Its tile now.
        to: Pos,
        /// The tile it hit (blocked or burning), if any.
        collided: Option<Pos>,
        /// HP lost to the collision (it may fall).
        damage: StatValue,
    },
    /// A pushed unit crashed into this unit (just after its
    /// [`Event::Pushed`]).
    CollisionDamage {
        /// The unit hit.
        unit: UnitId,
        /// The unit pushed into it.
        by: UnitId,
        /// HP lost (it may fall).
        damage: StatValue,
    },
    /// A weapon's durability reached 0.
    ItemBroke {
        /// The unit carrying it.
        unit: UnitId,
        /// The weapon.
        item: ItemId,
    },
    /// A unit used up a consumable on `target`.
    ItemUsed {
        /// The user.
        unit: UnitId,
        /// The consumable.
        item: ItemId,
        /// Who it was used on.
        target: UnitId,
    },
    /// A unit regained HP.
    Healed {
        /// The unit healed.
        target: UnitId,
        /// HP restored (after the max-HP cap).
        amount: StatValue,
    },
    /// A unit reached 0 HP and left the map.
    UnitFell {
        /// The unit.
        unit: UnitId,
    },
    /// A unit seized the objective tile.
    Seized {
        /// The unit.
        unit: UnitId,
        /// The tile.
        pos: Pos,
    },
    /// A unit bought an item.
    Bought {
        /// The buyer.
        unit: UnitId,
        /// The item.
        item: ItemId,
        /// Gold paid.
        price: Gold,
        /// Where the item went.
        to: Destination,
    },
    /// A unit sold an item.
    Sold {
        /// The seller.
        unit: UnitId,
        /// The item.
        item: ItemId,
        /// Gold got.
        price: Gold,
    },
    /// A unit had a weapon repaired to full durability.
    Repaired {
        /// The unit.
        unit: UnitId,
        /// The weapon's loadout slot.
        slot: usize,
        /// The weapon.
        item: ItemId,
        /// Gold paid.
        cost: Gold,
    },
    /// The party's gold changed.
    GoldChanged {
        /// The new total.
        gold: Gold,
    },
    /// A unit opened a chest.
    ChestOpened {
        /// The unit.
        unit: UnitId,
        /// The chest's tile.
        pos: Pos,
        /// What was inside.
        loot: Loot,
    },
    /// After its attack, a unit may move up to `tiles` tiles: it waits for a
    /// [`Command::MoveAfter`] (see [`BattleState::move_after_tiles`]).
    MoveAfterOffered {
        /// The unit.
        unit: UnitId,
        /// How far it may move.
        tiles: u32,
    },
    /// A unit finished its action and is done until its next phase.
    UnitActed {
        /// The unit.
        unit: UnitId,
    },
    /// The battle is over.
    BattleEnded {
        /// How it ended.
        outcome: Outcome,
    },
}

/// Why a command was refused. The state is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The battle has ended.
    BattleOver,
    /// No unit with this id is or was in the battle (reinforcements that
    /// haven't arrived included).
    UnknownUnit(UnitId),
    /// The unit has fallen.
    UnitFallen(UnitId),
    /// The unit doesn't act in the current phase.
    NotItsPhase {
        /// The unit.
        unit: UnitId,
        /// The current phase.
        phase: Phase,
    },
    /// The unit has already acted this phase.
    AlreadyActed(UnitId),
    /// A unit's class is not in the class table (e.g. tables not restored
    /// after loading).
    UnknownClass(ClassId),
    /// A unit stands outside the map.
    OffMap(UnitId),
    /// A tile's terrain is not in the terrain table.
    UnknownTerrain(Pos),
    /// The unit can't end its move on this tile.
    CannotStop(Pos),
    /// The target isn't hostile to the attacker.
    NotHostile(UnitId),
    /// The loadout slot holds no weapon.
    EmptySlot {
        /// The unit.
        unit: UnitId,
        /// The slot.
        slot: usize,
    },
    /// The unit can't wield this weapon (class kind or rank).
    CannotWield {
        /// The unit.
        unit: UnitId,
        /// The weapon.
        item: ItemId,
    },
    /// No item at this index of the pack.
    NoItem {
        /// The unit.
        unit: UnitId,
        /// The index.
        index: usize,
    },
    /// An item id is not in the item table (e.g. tables not restored).
    UnknownItem(ItemId),
    /// The item can't be used (it isn't a consumable).
    NotConsumable(ItemId),
    /// The item's target is neither the user nor an adjacent ally.
    BadItemTarget(UnitId),
    /// The target is outside the attacker's weapon range from `dest`.
    OutOfRange {
        /// The target.
        target: UnitId,
        /// Its distance from `dest`.
        distance: u32,
    },
    /// Seizing isn't possible: not a seize map, not the seize tile, not a
    /// player unit, or not a lord when the map needs one.
    CannotSeize,
    /// Only player units can shop, open chests or use items.
    PlayerOnly(UnitId),
    /// There is no shop on this tile.
    NoShop(Pos),
    /// A shop action with no transactions.
    NoTransactions,
    /// A shop transaction was refused (none was applied).
    Shop {
        /// The index of the refused transaction.
        txn: usize,
        /// Why.
        error: ShopError,
    },
    /// There is no chest on this tile.
    NoChest(Pos),
    /// The chest on this tile was already opened.
    AlreadyOpened(Pos),
    /// The unit hasn't learned this spell.
    SpellNotKnown {
        /// The unit.
        unit: UnitId,
        /// The spell.
        spell: SpellId,
    },
    /// A spell id is not in the spell table (e.g. tables not restored).
    UnknownSpell(SpellId),
    /// The spell has no uses left this battle.
    NoUsesLeft {
        /// The unit.
        unit: UnitId,
        /// The spell.
        spell: SpellId,
    },
    /// Only attack spells can be equipped.
    NotAttackSpell(SpellId),
    /// A heal's target is the caster itself or not an ally.
    BadHealTarget(UnitId),
    /// A heal's target is already at max HP.
    FullHp(UnitId),
    /// The spell can't be cast on a tile.
    NoTerrainEffect(SpellId),
    /// A tile cast's tile is outside the map.
    TileOffMap(Pos),
    /// A tile cast's tile is outside the spell's range from `dest`.
    TileOutOfRange {
        /// The tile.
        pos: Pos,
        /// Its distance from `dest`.
        distance: u32,
    },
    /// A tile cast's tile holds a unit.
    TileOccupied(Pos),
    /// The spell can't change this tile's terrain.
    WrongTerrain(Pos),
    /// A skill id is not in the skill table (e.g. tables not restored).
    UnknownSkill(SkillId),
    /// The unit can't use this active skill now.
    SkillNotUsable {
        /// The unit.
        unit: UnitId,
        /// The skill.
        skill: SkillId,
    },
    /// A non-combat active chosen for an attack or cast, a combat active
    /// used as an action, or an active chosen for a heal or tile cast.
    WrongSkillKind(SkillId),
    /// The attack doesn't use what the combat active needs.
    WrongWeaponForSkill(SkillId),
    /// The skill's cost can't be paid.
    CannotPay {
        /// The skill.
        skill: SkillId,
        /// Why.
        error: CostError,
    },
    /// The skill was given a target it can't take (or none when it needs
    /// one).
    BadSkillTarget(SkillId),
    /// Nobody is in the skill's reach (for a heal: nobody wounded).
    NoSkillTargets(SkillId),
    /// The unit can't move to this tile after its attack.
    CannotMoveAfter(Pos),
    /// This unit must first finish its move after its attack
    /// ([`Command::MoveAfter`]).
    MoveAfterPending(UnitId),
    /// This unit has no move after an attack to make.
    NoMoveAfter(UnitId),
    /// An art id is not in the art table (e.g. tables not restored).
    UnknownArt(ArtId),
    /// The unit can't use this art with this attack: it doesn't know it (or
    /// it isn't the weapon's), or the art is for another weapon kind.
    ArtNotUsable {
        /// The unit.
        unit: UnitId,
        /// The art.
        art: ArtId,
    },
    /// The art's cost can't be paid.
    CannotPayArt {
        /// The art.
        art: ArtId,
        /// Why.
        error: CostError,
    },
    /// An attack chose both a Combat Art and a combat active.
    ArtWithActive,
    /// The unit may not use arts and active skills: among enemies only
    /// bosses do, and non-combat green units never do.
    ArtsNotAllowed(UnitId),
    /// The action isn't an attack (for [`BattleState::preview_attack`]).
    NotAnAttack,
}

impl From<MoveError> for CommandError {
    fn from(e: MoveError) -> Self {
        match e {
            MoveError::UnknownUnit(id) => CommandError::UnknownUnit(id),
            MoveError::UnknownClass(c) => CommandError::UnknownClass(c),
            MoveError::OffMap(id) => CommandError::OffMap(id),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::BattleOver => f.write_str("the battle is over"),
            CommandError::UnknownUnit(id) => write!(f, "no unit with id {}", id.0),
            CommandError::UnitFallen(id) => write!(f, "unit {} has fallen", id.0),
            CommandError::NotItsPhase { unit, phase } => {
                write!(f, "unit {} doesn't act in the {phase:?} phase", unit.0)
            }
            CommandError::AlreadyActed(id) => write!(f, "unit {} has already acted", id.0),
            CommandError::UnknownClass(c) => write!(f, "unknown class \"{}\"", c.0),
            CommandError::OffMap(id) => write!(f, "unit {} is outside the map", id.0),
            CommandError::UnknownTerrain(p) => {
                write!(f, "unknown terrain at ({}, {})", p.x, p.y)
            }
            CommandError::CannotStop(p) => write!(f, "can't stop at ({}, {})", p.x, p.y),
            CommandError::NotHostile(id) => write!(f, "unit {} is not an enemy", id.0),
            CommandError::EmptySlot { unit, slot } => {
                write!(f, "unit {} has no weapon in slot {slot}", unit.0)
            }
            CommandError::CannotWield { unit, item } => {
                write!(f, "unit {} can't wield \"{}\"", unit.0, item.0)
            }
            CommandError::NoItem { unit, index } => {
                write!(f, "unit {} has no item {index}", unit.0)
            }
            CommandError::UnknownItem(i) => write!(f, "unknown item \"{}\"", i.0),
            CommandError::NotConsumable(i) => write!(f, "\"{}\" can't be used", i.0),
            CommandError::BadItemTarget(id) => {
                write!(f, "unit {} can't be given an item from here", id.0)
            }
            CommandError::OutOfRange { target, distance } => {
                write!(f, "unit {} is out of range ({distance} tiles)", target.0)
            }
            CommandError::CannotSeize => f.write_str("can't seize here"),
            CommandError::PlayerOnly(id) => write!(f, "unit {} is not a player unit", id.0),
            CommandError::NoShop(p) => write!(f, "no shop at ({}, {})", p.x, p.y),
            CommandError::NoTransactions => f.write_str("nothing bought, sold or repaired"),
            CommandError::Shop { txn, error } => write!(f, "transaction {txn}: {error}"),
            CommandError::NoChest(p) => write!(f, "no chest at ({}, {})", p.x, p.y),
            CommandError::AlreadyOpened(p) => {
                write!(f, "the chest at ({}, {}) is already open", p.x, p.y)
            }
            CommandError::SpellNotKnown { unit, spell } => {
                write!(f, "unit {} doesn't know \"{}\"", unit.0, spell.0)
            }
            CommandError::UnknownSpell(s) => write!(f, "unknown spell \"{}\"", s.0),
            CommandError::NoUsesLeft { unit, spell } => {
                write!(f, "unit {} has no uses of \"{}\" left", unit.0, spell.0)
            }
            CommandError::NotAttackSpell(s) => {
                write!(f, "\"{}\" isn't an attack spell", s.0)
            }
            CommandError::BadHealTarget(id) => write!(f, "unit {} can't be healed by it", id.0),
            CommandError::FullHp(id) => write!(f, "unit {} is at full HP", id.0),
            CommandError::NoTerrainEffect(s) => {
                write!(f, "\"{}\" can't be cast on a tile", s.0)
            }
            CommandError::TileOffMap(p) => write!(f, "({}, {}) is outside the map", p.x, p.y),
            CommandError::TileOutOfRange { pos, distance } => write!(
                f,
                "({}, {}) is out of range ({distance} tiles)",
                pos.x, pos.y
            ),
            CommandError::TileOccupied(p) => write!(f, "({}, {}) is occupied", p.x, p.y),
            CommandError::WrongTerrain(p) => {
                write!(
                    f,
                    "the spell can't change the terrain at ({}, {})",
                    p.x, p.y
                )
            }
            _ => self.fmt_skill_error(f),
        }
    }
}

impl CommandError {
    /// The messages of the skill, art and move-after errors (any other
    /// error shows its debug form).
    fn fmt_skill_error(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::UnknownSkill(s) => write!(f, "unknown skill \"{}\"", s.0),
            CommandError::SkillNotUsable { unit, skill } => {
                write!(f, "unit {} can't use \"{}\" now", unit.0, skill.0)
            }
            CommandError::WrongSkillKind(s) => write!(f, "\"{}\" can't be used that way", s.0),
            CommandError::WrongWeaponForSkill(s) => {
                write!(f, "\"{}\" can't be used with this attack", s.0)
            }
            CommandError::CannotPay { skill, error } => {
                write!(f, "can't pay for \"{}\": {error}", skill.0)
            }
            CommandError::BadSkillTarget(s) => write!(f, "\"{}\" can't target that", s.0),
            CommandError::NoSkillTargets(s) => write!(f, "\"{}\" would reach nobody", s.0),
            CommandError::CannotMoveAfter(p) => {
                write!(f, "can't move to ({}, {}) after attacking", p.x, p.y)
            }
            CommandError::MoveAfterPending(id) => {
                write!(f, "unit {} must first finish its move", id.0)
            }
            CommandError::NoMoveAfter(id) => {
                write!(f, "unit {} has no move to make", id.0)
            }
            CommandError::UnknownArt(a) => write!(f, "unknown art \"{}\"", a.0),
            CommandError::ArtNotUsable { unit, art } => {
                write!(f, "unit {} can't use \"{}\" in this attack", unit.0, art.0)
            }
            CommandError::CannotPayArt { art, error } => {
                write!(f, "can't pay for \"{}\": {error}", art.0)
            }
            CommandError::ArtWithActive => {
                f.write_str("an attack uses one art or one active, not both")
            }
            CommandError::ArtsNotAllowed(id) => {
                write!(f, "unit {} doesn't use arts or actives", id.0)
            }
            CommandError::NotAnAttack => f.write_str("the action isn't an attack"),
            other => write!(f, "{other:?}"),
        }
    }
}

impl std::error::Error for CommandError {}

/// The shared content tables a battle reads. Not saved (see the module docs).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Tables {
    terrain: Arc<TerrainTable>,
    classes: Arc<ClassTable>,
    items: Arc<ItemTable>,
    spells: Arc<SpellTable>,
    skills: Arc<SkillTable>,
    arts: Arc<ArtTable>,
}

/// A running battle. Changed only by [`BattleState::apply`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleState {
    #[serde(skip)]
    tables: Tables,
    map: BattleMap,
    burning: Vec<Burning>,
    turn: Turn,
    phase: Phase,
    units: Vec<Unit>,
    fallen: Vec<Unit>,
    pending: Vec<Reinforcement>,
    objective: Objective,
    rewind_charges: u8,
    pack: BattlePack,
    gold: Gold,
    stock: Stock,
    opened: BTreeSet<Pos>,
    rng: SimRng,
    outcome: Option<Outcome>,
    pending_move: Option<PendingMove>,
    /// EXP earned by Ally-faction units, shared out on a win.
    #[serde(default)]
    exp_pool: u32,
}

/// A validated action, ready to carry out.
enum Step {
    Wait,
    /// A combat, with a weapon or an attack spell.
    Attack(Box<AttackStep>),
    /// A non-combat active.
    Skill {
        active: Box<ActiveUse>,
        effect: SkillStep,
    },
    Heal {
        spell: SpellId,
        target: UnitId,
        amount: StatValue,
    },
    /// A tile cast.
    Terrain {
        spell: SpellId,
        pos: Pos,
        from: TerrainId,
        effect: TerrainEffect,
    },
    UseItem {
        index: usize,
        item: ItemId,
        effect: ConsumableEffect,
        target: UnitId,
    },
    Seize,
    /// The shop visit's result, worked out on copies.
    Shop(Box<Till>),
    Open {
        pos: Pos,
        loot: Loot,
    },
}

/// A validated combat.
struct AttackStep {
    target: UnitId,
    with: Equipped,
    forecast: Forecast,
    arms: Arms,
    /// The combat active used, if any.
    active: Option<ActiveUse>,
    /// The Combat Art used, if any.
    art: Option<ArtUse>,
    /// Line Pierce's strike after the combat: the unit behind the target
    /// and the strike's numbers (attacker side only, 1 strike).
    pierce: Option<(UnitId, Forecast)>,
    /// Tiles the attacker may move after the combat.
    move_after: u32,
}

/// What each side of a combat fights with: `[attacker, defender]`.
struct Arms {
    /// Weapon kinds, for weapon EXP (`None` for spells or no counter).
    kinds: [Option<WeaponKind>; 2],
    /// Attack spells, whose uses a strike spends.
    spells: [Option<SpellId>; 2],
}

/// A shop visit worked out on copies of what it changes, so that a refused
/// transaction leaves the battle untouched. Committed as a whole.
struct Till {
    unit: Unit,
    gold: Gold,
    pack: BattlePack,
    stock: Stock,
    events: Vec<Event>,
}

impl Till {
    /// Applies one transaction to the copies.
    fn apply(
        &mut self,
        shop: &Shop,
        class: &ClassDef,
        tables: &Tables,
        txn: &ShopTxn,
    ) -> Result<(), ShopError> {
        let items = &tables.items;
        let id = self.unit.id;
        match txn {
            ShopTxn::Buy { item } => {
                let price = shop::buy(shop, items, &mut self.gold, item)?;
                let to = self.receive(class, items, item);
                self.events.push(Event::Bought {
                    unit: id,
                    item: item.clone(),
                    price,
                    to,
                });
                if let Destination::WeaponSlot(slot) = to
                    && self.unit.loadout.equipped.is_none()
                    && self.unit.usable_weapon(slot, class, items).is_some()
                {
                    self.equip(Some(Equipped::Weapon(slot)));
                }
            }
            ShopTxn::Sell { from } => {
                let item = self.item_at(*from).ok_or(ShopError::NoItem)?;
                let price = shop::sell(shop, items, &mut self.gold, &item)?;
                self.events.push(Event::Sold {
                    unit: id,
                    item,
                    price,
                });
                self.remove(*from, class, tables);
            }
            ShopTxn::Repair { slot } => {
                let copy = self
                    .unit
                    .loadout
                    .weapons
                    .get_mut(*slot)
                    .and_then(Option::as_mut)
                    .ok_or(ShopError::NoItem)?;
                let cost = shop::repair(shop, items, &mut self.gold, copy)?;
                let item = copy.def.clone();
                self.events.push(Event::Repaired {
                    unit: id,
                    slot: *slot,
                    item,
                    cost,
                });
            }
        }
        self.events.push(Event::GoldChanged { gold: self.gold });
        Ok(())
    }

    /// Puts a bought `item` (known) where it goes and says where.
    fn receive(&mut self, class: &ClassDef, items: &ItemTable, item: &ItemId) -> Destination {
        let loadout = &mut self.unit.loadout;
        let slots = usize::from(class.weapon_slots).min(WEAPON_SLOTS);
        match items.get(item) {
            Some(ItemDef::Weapon(_)) => {
                if let Some(slot) = (0..slots).find(|&s| loadout.weapons[s].is_none()) {
                    loadout.weapons[slot] = items.new_weapon(item);
                    return Destination::WeaponSlot(slot);
                }
            }
            Some(ItemDef::Armour(a)) => {
                if loadout.armour.is_none() && class.armour.contains(&a.weight_class) {
                    loadout.armour = Some(item.clone());
                    return Destination::Armour;
                }
            }
            Some(ItemDef::Accessory(_)) => {
                if loadout.accessory.is_none() {
                    loadout.accessory = Some(item.clone());
                    return Destination::Accessory;
                }
            }
            Some(ItemDef::Consumable(_)) => {
                self.pack.gain(item.clone());
                return Destination::Pack;
            }
            None => {}
        }
        // Known items only reach here (`shop::buy` checked), so this can't
        // fail.
        let _ = shop::add_to_stock(&mut self.stock, items, item);
        Destination::Stock
    }

    /// The item a sale would sell, if it is there.
    fn item_at(&self, from: SellFrom) -> Option<ItemId> {
        let loadout = &self.unit.loadout;
        match from {
            SellFrom::Weapon(slot) => loadout.weapon(slot).map(|w| w.def.clone()),
            SellFrom::Armour => loadout.armour.clone(),
            SellFrom::Accessory => loadout.accessory.clone(),
            SellFrom::Pack(i) => self.pack.items.get(i).cloned(),
        }
    }

    /// Removes a sold item; re-equips if it was the equipped weapon.
    fn remove(&mut self, from: SellFrom, class: &ClassDef, tables: &Tables) {
        let loadout = &mut self.unit.loadout;
        match from {
            SellFrom::Weapon(slot) => {
                loadout.weapons[slot] = None;
                if loadout.equipped_slot() == Some(slot) {
                    let next = self
                        .unit
                        .default_equip(class, &tables.items, &tables.spells);
                    self.equip(next);
                }
            }
            SellFrom::Armour => loadout.armour = None,
            SellFrom::Accessory => loadout.accessory = None,
            SellFrom::Pack(i) => {
                self.pack.items.remove(i);
            }
        }
    }

    /// Equips `equipped` (or nothing), with an event unless nothing.
    fn equip(&mut self, equipped: Option<Equipped>) {
        self.unit.loadout.equipped.clone_from(&equipped);
        if let Some(equipped) = equipped {
            self.events.push(Event::Equipped {
                unit: self.unit.id,
                equipped,
            });
        }
    }
}

impl BattleState {
    /// Starts the battle at turn 1, Player phase. Every unit, reinforcements
    /// included, is [prepared](Unit::prepare_for_battle): full spell uses and
    /// a default equip. The events are the first phase start (with any
    /// turn-1 player reinforcements), or [`Event::BattleEnded`] if it is
    /// already decided (no player units, or a rout with no enemies).
    pub fn new(setup: BattleSetup) -> (BattleState, Vec<Event>) {
        let mut units = setup.units;
        let mut pending = setup.reinforcements;
        for u in units
            .iter_mut()
            .chain(pending.iter_mut().map(|r| &mut r.unit))
        {
            u.prepare_for_battle(&setup.classes, &setup.items, &setup.spells);
        }
        let mut state = BattleState {
            tables: Tables {
                terrain: setup.terrain,
                classes: setup.classes,
                items: setup.items,
                spells: setup.spells,
                skills: setup.skills,
                arts: setup.arts,
            },
            map: setup.map,
            burning: Vec::new(),
            turn: 1,
            phase: Phase::Player,
            units,
            fallen: Vec::new(),
            pending,
            objective: setup.objective,
            rewind_charges: setup.rewind_charges,
            pack: setup.pack,
            gold: setup.gold,
            stock: setup.stock,
            opened: BTreeSet::new(),
            rng: SimRng::new(setup.seed),
            outcome: None,
            pending_move: None,
            exp_pool: 0,
        };
        let mut events = Vec::new();
        if let Some(outcome) = state.judge() {
            state.finish(outcome, &mut events);
        } else {
            // Never skipped: judge() found player units on the map.
            state.start_phase(&mut events);
        }
        (state, events)
    }

    /// Reattaches the content tables after deserialising (see the module
    /// docs). They must be the tables the battle was started with.
    pub fn restore_tables(
        &mut self,
        terrain: Arc<TerrainTable>,
        classes: Arc<ClassTable>,
        items: Arc<ItemTable>,
        spells: Arc<SpellTable>,
        skills: Arc<SkillTable>,
        arts: Arc<ArtTable>,
    ) {
        self.tables = Tables {
            terrain,
            classes,
            items,
            spells,
            skills,
            arts,
        };
    }

    /// The battlefield.
    pub fn map(&self) -> &BattleMap {
        &self.map
    }

    /// Tiles burning now, in the order they were set burning.
    pub fn burning(&self) -> &[Burning] {
        &self.burning
    }

    /// Terrain rules.
    pub fn terrain(&self) -> &TerrainTable {
        &self.tables.terrain
    }

    /// Classes.
    pub fn classes(&self) -> &ClassTable {
        &self.tables.classes
    }

    /// Items.
    pub fn items(&self) -> &ItemTable {
        &self.tables.items
    }

    /// Spells.
    pub fn spells(&self) -> &SpellTable {
        &self.tables.spells
    }

    /// Skills.
    pub fn skills(&self) -> &SkillTable {
        &self.tables.skills
    }

    /// Combat Arts.
    pub fn arts(&self) -> &ArtTable {
        &self.tables.arts
    }

    /// The player side's consumables.
    pub fn pack(&self) -> &BattlePack {
        &self.pack
    }

    /// The party's gold.
    pub fn gold(&self) -> Gold {
        self.gold
    }

    /// The party's stock (bought and found equipment lands here).
    pub fn stock(&self) -> &Stock {
        &self.stock
    }

    /// Whether the chest at `pos` has been opened.
    pub fn is_opened(&self, pos: Pos) -> bool {
        self.opened.contains(&pos)
    }

    /// The current turn, from 1.
    pub fn turn(&self) -> Turn {
        self.turn
    }

    /// The current phase.
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Every unit on the map (living), in a stable order.
    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    /// The unit `id`, if it is on the map.
    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    /// Units that fell, in the order they fell (HP 0).
    pub fn fallen(&self) -> &[Unit] {
        &self.fallen
    }

    /// Reinforcements that haven't arrived yet.
    pub fn reinforcements(&self) -> &[Reinforcement] {
        &self.pending
    }

    /// How to win.
    pub fn objective(&self) -> Objective {
        self.objective
    }

    /// Rewind charges the battle started with (the charges left are
    /// [`BattleHistory::charges_left`](crate::BattleHistory::charges_left)).
    pub fn rewind_charges(&self) -> u8 {
        self.rewind_charges
    }

    /// How the battle ended, once it has.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// The unit waiting to move after its attack, if any.
    pub fn pending_move(&self) -> Option<PendingMove> {
        self.pending_move
    }

    /// EXP earned by Ally-faction units so far, shared out among the player
    /// units on a win.
    pub fn exp_pool(&self) -> u32 {
        self.exp_pool
    }

    /// Whether unit `id` may choose [`UnitAction::Seize`] after moving to
    /// `dest` (the action menu offers it only then). Doesn't check that it
    /// can reach `dest` or still act.
    pub fn can_seize(&self, id: UnitId, dest: Pos) -> bool {
        self.unit(id)
            .is_some_and(|u| self.check_seize(u, dest).is_ok())
    }

    /// Applies `cmd`: validates it, changes the state and returns what
    /// happened. On `Err` nothing changed.
    pub fn apply(&mut self, cmd: &Command) -> Result<Vec<Event>, CommandError> {
        if self.outcome.is_some() {
            return Err(CommandError::BattleOver);
        }
        let mut events = Vec::new();
        if let Some(pending) = self.pending_move
            && !matches!(cmd, Command::MoveAfter { .. })
        {
            return Err(CommandError::MoveAfterPending(pending.unit));
        }
        match cmd {
            Command::MoveAfter { unit, to } => self.move_after(*unit, *to, &mut events)?,
            Command::EndPhase => self.end_phase(&mut events),
            Command::Equip { unit, equipped } => {
                let u = self.check_ready(*unit)?;
                match equipped {
                    Equipped::Weapon(slot) => self.check_wield(*unit, *slot)?,
                    Equipped::Spell(spell) => {
                        if !self.known_spell(u, spell)?.is_attack() {
                            return Err(CommandError::NotAttackSpell(spell.clone()));
                        }
                    }
                }
                self.equip(*unit, equipped.clone(), &mut events);
            }
            Command::Act { unit, dest, action } => {
                let (path, step) = self.plan(*unit, *dest, action)?;
                self.act(*unit, path, step, &mut events);
            }
        }
        Ok(events)
    }

    /// Validates an `Act`: the move's path and the action to carry out.
    fn plan(
        &self,
        id: UnitId,
        dest: Pos,
        action: &UnitAction,
    ) -> Result<(Vec<Pos>, Step), CommandError> {
        let unit = self.check_ready(id)?;
        let reach = reachable(
            &self.map,
            &self.tables.terrain,
            &self.tables.classes,
            &self.units,
            id,
        )?;
        let path = reach
            .path_to(dest)
            .filter(|_| reach.is_stoppable(dest))
            .ok_or(CommandError::CannotStop(dest))?;
        let moved = u32::try_from(path.len().saturating_sub(1)).unwrap_or(u32::MAX);
        let step = match *action {
            UnitAction::Wait => Step::Wait,
            UnitAction::Attack {
                target,
                slot,
                ref active,
                ref art,
            } => {
                self.check_wield(unit.id, slot)?;
                let plan = AttackPlan {
                    dest,
                    moved,
                    target,
                    with: Equipped::Weapon(slot),
                    active: active.as_ref(),
                    art: art.as_ref(),
                };
                self.plan_attack(unit, plan)?
            }
            UnitAction::UseItem { pack_index, target } => {
                self.plan_item(unit, dest, pack_index, target)?
            }
            UnitAction::Seize => {
                self.check_seize(unit, dest)?;
                Step::Seize
            }
            UnitAction::Shop { ref txns } => self.plan_shop(unit, dest, txns)?,
            UnitAction::Open => self.plan_open(unit, dest)?,
            UnitAction::Cast {
                ref spell,
                target: CastTarget::Unit(target),
                ref active,
            } => {
                let plan = AttackPlan {
                    dest,
                    moved,
                    target,
                    with: Equipped::Spell(spell.clone()),
                    active: active.as_ref(),
                    art: None,
                };
                self.plan_cast(unit, spell, plan)?
            }
            UnitAction::Cast {
                ref spell,
                target: CastTarget::Tile(pos),
                ref active,
            } => {
                if let Some(skill) = active {
                    return Err(CommandError::WrongSkillKind(skill.clone()));
                }
                self.plan_tile_cast(unit, dest, spell, pos)?
            }
            UnitAction::UseSkill { ref skill, target } => {
                self.plan_skill(unit, dest, skill, target)?
            }
        };
        Ok((path, step))
    }

    /// The living unit `id` if it may act now: its phase, not yet acted.
    fn check_ready(&self, id: UnitId) -> Result<&Unit, CommandError> {
        let unit = self.living(id)?;
        if Phase::of(unit.faction) != self.phase {
            return Err(CommandError::NotItsPhase {
                unit: id,
                phase: self.phase,
            });
        }
        if unit.acted {
            return Err(CommandError::AlreadyActed(id));
        }
        Ok(unit)
    }

    /// Checks unit `id` can wield the weapon in `slot`.
    fn check_wield(&self, id: UnitId, slot: usize) -> Result<(), CommandError> {
        let unit = self.living(id)?;
        let copy = unit
            .loadout
            .weapon(slot)
            .ok_or(CommandError::EmptySlot { unit: id, slot })?;
        let class = self.class_of(unit)?;
        if self.tables.items.get(&copy.def).is_none() {
            return Err(CommandError::UnknownItem(copy.def.clone()));
        }
        match unit.usable_weapon(slot, class, &self.tables.items) {
            Some(_) => Ok(()),
            None => Err(CommandError::CannotWield {
                unit: id,
                item: copy.def.clone(),
            }),
        }
    }

    /// The spell `spell` if `unit` has learned it and it is in the table.
    fn known_spell(&self, unit: &Unit, spell: &SpellId) -> Result<&SpellDef, CommandError> {
        if !unit.learned.contains(spell) {
            return Err(CommandError::SpellNotKnown {
                unit: unit.id,
                spell: spell.clone(),
            });
        }
        self.tables
            .spells
            .get(spell)
            .ok_or_else(|| CommandError::UnknownSpell(spell.clone()))
    }

    /// The combat of `unit` attacking as `plan` says (its weapon or spell
    /// already checked usable, apart from its range).
    fn plan_attack(&self, unit: &Unit, plan: AttackPlan) -> Result<Step, CommandError> {
        let AttackPlan {
            dest,
            moved,
            target,
            with,
            active,
            art,
        } = plan;
        let defender = self.living(target)?;
        if !unit.faction.is_hostile_to(defender.faction) {
            return Err(CommandError::NotHostile(target));
        }
        if active.is_some() && art.is_some() {
            return Err(CommandError::ArtWithActive);
        }
        if active.is_some() || art.is_some() {
            check_arts_allowed(unit)?;
        }
        let active = active
            .map(|id| self.plan_active(unit, id, &with))
            .transpose()?;
        let art = art.map(|id| self.plan_art(unit, id, &with)).transpose()?;
        let fight = Fight {
            dest,
            moved,
            with: &with,
            active: active.as_ref(),
            art: art.as_ref(),
        };
        let (a, d, post_move) = self.fighters(unit, defender, &fight)?;
        let distance = Pos::manhattan(dest, defender.pos);
        let mut forecast = forecast(&self.tables.items.combat_rules(), &a, &d, distance)
            .ok_or(CommandError::OutOfRange { target, distance })?;
        if art.as_ref().is_some_and(|a| a.effect.no_counter) {
            forecast.defender = None;
        }
        let pierce = match &art {
            Some(a) if a.effect.line_pierce => {
                self.plan_pierce(unit, defender, &fight, distance)?
            }
            _ => None,
        };
        let kind = |c: &CombatantInput| c.weapon.as_ref().and_then(|w| w.kind);
        let counter_spell = forecast
            .defender
            .and(defender.loadout.equipped_spell())
            .cloned();
        let arms = Arms {
            kinds: [kind(&a), kind(&d)],
            spells: [
                match &with {
                    Equipped::Spell(spell) => Some(spell.clone()),
                    Equipped::Weapon(_) => None,
                },
                counter_spell,
            ],
        };
        Ok(Step::Attack(Box::new(AttackStep {
            target,
            with,
            forecast,
            arms,
            active,
            art,
            pierce,
            move_after: post_move,
        })))
    }

    /// The forecast of `unit` having moved `moved` tiles to `dest` and
    /// attacking `target` with `with`, with no art or active: the numbers an
    /// `Act` would fight with. Unlike [`Self::preview_attack`] it doesn't
    /// check the move, the phase or that `with` is usable, so the AI
    /// ([`crate::ai`]) can search many attacks cheaply; it checks those
    /// itself. `None` if `target` is out of range, `with` can't strike or a
    /// table entry is missing.
    pub(crate) fn plain_forecast(
        &self,
        unit: &Unit,
        dest: Pos,
        moved: u32,
        target: &Unit,
        with: &Equipped,
    ) -> Option<Forecast> {
        let fight = Fight {
            dest,
            moved,
            with,
            active: None,
            art: None,
        };
        let (a, d, _) = self.fighters(unit, target, &fight).ok()?;
        let distance = Pos::manhattan(dest, target.pos);
        forecast(&self.tables.items.combat_rules(), &a, &d, distance)
    }

    /// The spell `spell` if `unit` has learned it and has a use left.
    fn castable(&self, unit: &Unit, spell: &SpellId) -> Result<&SpellDef, CommandError> {
        let def = self.known_spell(unit, spell)?;
        if unit.spells.uses_left(spell) == 0 {
            return Err(CommandError::NoUsesLeft {
                unit: unit.id,
                spell: spell.clone(),
            });
        }
        Ok(def)
    }

    /// Validates `unit` casting `spell` on unit `plan.target` from
    /// `plan.dest` (`plan.with` is the spell).
    fn plan_cast(
        &self,
        unit: &Unit,
        spell: &SpellId,
        plan: AttackPlan,
    ) -> Result<Step, CommandError> {
        let def = self.castable(unit, spell)?;
        let heal_power = match def.kind {
            SpellKind::Attack { .. } => return self.plan_attack(unit, plan),
            SpellKind::Heal { heal_power } => heal_power,
        };
        if let Some(skill) = plan.active {
            return Err(CommandError::WrongSkillKind(skill.clone()));
        }
        let (dest, target) = (plan.dest, plan.target);
        let other = self.living(target)?;
        if target == unit.id || !unit.faction.is_allied_to(other.faction) {
            return Err(CommandError::BadHealTarget(target));
        }
        let distance = Pos::manhattan(dest, other.pos);
        if !def.in_range(distance) {
            return Err(CommandError::OutOfRange { target, distance });
        }
        let missing = other.stats.hp - other.hp;
        if missing <= 0 {
            return Err(CommandError::FullHp(target));
        }
        let mag = unit
            .effective_stats(&self.tables.classes, &self.tables.items)
            .mag;
        let bonus = heal_bonus(&unit.usable_skills(&self.tables.classes, &self.tables.skills));
        Ok(Step::Heal {
            spell: spell.clone(),
            target,
            amount: heal_power
                .saturating_add(mag)
                .saturating_add(bonus)
                .clamp(0, missing),
        })
    }

    /// Validates `unit` casting `spell` on the tile `pos` from `dest`.
    fn plan_tile_cast(
        &self,
        unit: &Unit,
        dest: Pos,
        spell: &SpellId,
        pos: Pos,
    ) -> Result<Step, CommandError> {
        let def = self.castable(unit, spell)?;
        let effect = def
            .terrain_effect
            .as_ref()
            .ok_or_else(|| CommandError::NoTerrainEffect(spell.clone()))?;
        let from = *self
            .map
            .tiles
            .get(pos)
            .ok_or(CommandError::TileOffMap(pos))?;
        let distance = Pos::manhattan(dest, pos);
        if !def.in_range(distance) {
            return Err(CommandError::TileOutOfRange { pos, distance });
        }
        // The caster stands on `dest` once it has moved, not on its old tile.
        let occupied = pos == dest || self.units.iter().any(|u| u.pos == pos && u.id != unit.id);
        if occupied {
            return Err(CommandError::TileOccupied(pos));
        }
        if !effect.from.contains(&from) {
            return Err(CommandError::WrongTerrain(pos));
        }
        Ok(Step::Terrain {
            spell: spell.clone(),
            pos,
            from,
            effect: effect.clone(),
        })
    }

    /// Validates `unit` using item `index` on `target` from `dest`.
    fn plan_item(
        &self,
        unit: &Unit,
        dest: Pos,
        index: usize,
        target: UnitId,
    ) -> Result<Step, CommandError> {
        if unit.faction != Faction::Player {
            return Err(CommandError::PlayerOnly(unit.id));
        }
        let item = self.pack.items.get(index).ok_or(CommandError::NoItem {
            unit: unit.id,
            index,
        })?;
        let effect = match self.tables.items.get(item) {
            None => return Err(CommandError::UnknownItem(item.clone())),
            Some(ItemDef::Consumable(c)) => c.effect,
            Some(_) => return Err(CommandError::NotConsumable(item.clone())),
        };
        if target != unit.id {
            let other = self.living(target)?;
            if unit.faction.is_hostile_to(other.faction) || Pos::manhattan(dest, other.pos) != 1 {
                return Err(CommandError::BadItemTarget(target));
            }
        }
        Ok(Step::UseItem {
            index,
            item: item.clone(),
            effect,
            target,
        })
    }

    /// Works out `unit`'s shop visit at `dest` on copies.
    fn plan_shop(&self, unit: &Unit, dest: Pos, txns: &[ShopTxn]) -> Result<Step, CommandError> {
        if unit.faction != Faction::Player {
            return Err(CommandError::PlayerOnly(unit.id));
        }
        let shop = self.map.shop(dest).ok_or(CommandError::NoShop(dest))?;
        if txns.is_empty() {
            return Err(CommandError::NoTransactions);
        }
        let class = self.class_of(unit)?;
        let mut till = Till {
            unit: Unit {
                pos: dest,
                ..unit.clone()
            },
            gold: self.gold,
            pack: self.pack.clone(),
            stock: self.stock.clone(),
            events: Vec::new(),
        };
        for (i, txn) in txns.iter().enumerate() {
            till.apply(shop, class, &self.tables, txn)
                .map_err(|error| CommandError::Shop { txn: i, error })?;
        }
        Ok(Step::Shop(Box::new(till)))
    }

    /// Validates `unit` opening the chest at `dest`.
    fn plan_open(&self, unit: &Unit, dest: Pos) -> Result<Step, CommandError> {
        if unit.faction != Faction::Player {
            return Err(CommandError::PlayerOnly(unit.id));
        }
        let loot = self.map.chest(dest).ok_or(CommandError::NoChest(dest))?;
        if self.opened.contains(&dest) {
            return Err(CommandError::AlreadyOpened(dest));
        }
        if let Loot::Item(item) = loot
            && self.tables.items.get(item).is_none()
        {
            return Err(CommandError::UnknownItem(item.clone()));
        }
        Ok(Step::Open {
            pos: dest,
            loot: loot.clone(),
        })
    }

    /// The class of `unit`.
    fn class_of(&self, unit: &Unit) -> Result<&ClassDef, CommandError> {
        self.tables
            .classes
            .get(&unit.class)
            .ok_or_else(|| CommandError::UnknownClass(unit.class.clone()))
    }

    /// `unit` as the combat maths sees it, standing on `pos`, fighting with
    /// `with` (`None`: its equipped attack).
    fn combatant(
        &self,
        unit: &Unit,
        pos: Pos,
        with: Option<&Equipped>,
    ) -> Result<CombatantInput<'_>, CommandError> {
        let class = self.class_of(unit)?;
        let tile = *self
            .map
            .tiles
            .get(pos)
            .ok_or(CommandError::OffMap(unit.id))?;
        let terrain = self
            .tables
            .terrain
            .get(tile)
            .ok_or(CommandError::UnknownTerrain(pos))?;
        Ok(unit.combat_input(
            class,
            &self.tables.classes,
            &self.tables.items,
            &self.tables.spells,
            with,
            terrain,
        ))
    }

    /// Whether `unit` may seize from `dest`.
    fn check_seize(&self, unit: &Unit, dest: Pos) -> Result<(), CommandError> {
        match self.objective {
            Objective::Seize { pos, by_lord, .. }
                if pos == dest && unit.faction == Faction::Player && (unit.is_lord || !by_lord) =>
            {
                Ok(())
            }
            _ => Err(CommandError::CannotSeize),
        }
    }

    /// The living unit `id`, or why there is none.
    fn living(&self, id: UnitId) -> Result<&Unit, CommandError> {
        if let Some(unit) = self.unit(id) {
            Ok(unit)
        } else if self.fallen.iter().any(|u| u.id == id) {
            Err(CommandError::UnitFallen(id))
        } else {
            Err(CommandError::UnknownUnit(id))
        }
    }

    fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    /// Carries out a validated `Act`: `path` ends at the unit's `dest`.
    fn act(&mut self, id: UnitId, path: Vec<Pos>, step: Step, events: &mut Vec<Event>) {
        let dest = path.last().copied();
        if let Some(dest) = dest
            && path.len() > 1
        {
            if let Some(u) = self.unit_mut(id) {
                u.pos = dest;
            }
            events.push(Event::UnitMoved { unit: id, path });
        }
        let mut seized = false;
        let mut move_after = 0;
        match step {
            Step::Wait => {}
            Step::Attack(attack) => move_after = self.attack(id, &attack, events),
            Step::Skill { active, effect } => {
                let award = SkillAward::of(&effect);
                self.use_skill(id, &active, effect, events);
                self.award_skill(id, &award, events);
            }
            Step::Heal {
                spell,
                target,
                amount,
            } => {
                events.push(Event::SpellCast {
                    unit: id,
                    spell: spell.clone(),
                    target: CastTarget::Unit(target),
                });
                if let Some(t) = self.unit_mut(target) {
                    t.hp += amount;
                    events.push(Event::Healed { target, amount });
                }
                self.spend_spell(id, &spell, events);
                self.award(
                    id,
                    progression::exp_for_heal(),
                    progression::ACTION_CP,
                    events,
                );
            }
            Step::Terrain {
                spell,
                pos,
                from,
                effect,
            } => {
                self.cast_on_tile(id, &spell, pos, from, &effect, events);
                let exp = progression::exp_for_tile_cast();
                self.award(id, exp, progression::ACTION_CP, events);
            }
            Step::UseItem {
                index,
                item,
                effect,
                target,
            } => self.use_item(id, index, item, effect, target, events),
            Step::Seize => {
                if let Some(pos) = dest {
                    events.push(Event::Seized { unit: id, pos });
                }
                seized = true;
            }
            Step::Shop(till) => {
                let till = *till;
                if let Some(u) = self.unit_mut(id) {
                    *u = till.unit;
                }
                self.gold = till.gold;
                self.pack = till.pack;
                self.stock = till.stock;
                events.extend(till.events);
            }
            Step::Open { pos, loot } => self.open(id, pos, loot, events),
        }
        let outcome = if seized {
            Some(Outcome::Victory)
        } else {
            self.judge()
        };
        if outcome.is_none() {
            self.offer_move_after(id, move_after, events);
        }
        let waits = self.pending_move.is_some();
        if let Some(u) = self.unit_mut(id) {
            u.acted = true;
            if !waits {
                events.push(Event::UnitActed { unit: id });
            }
        }
        if let Some(outcome) = outcome {
            self.finish(outcome, events);
        }
    }

    /// Carries out unit `id`'s validated combat (see the module docs for the
    /// order of events). Returns how far it may move after it.
    fn attack(&mut self, id: UnitId, attack: &AttackStep, events: &mut Vec<Event>) -> u32 {
        let target = attack.target;
        let with = attack.with.clone();
        if self
            .unit(id)
            .is_some_and(|u| u.loadout.equipped.as_ref() != Some(&with))
        {
            self.equip(id, with.clone(), events);
        }
        let broke = match (&attack.active, &attack.art) {
            (Some(active), _) => self.pay(id, active, events),
            (None, Some(art)) => self.commit_art(id, art, events),
            (None, None) => None,
        };
        if let Equipped::Spell(spell) = with {
            events.push(Event::SpellCast {
                unit: id,
                spell,
                target: CastTarget::Unit(target),
            });
        }
        let (dealt, clashes) = self.fight(id, attack, events);
        if let Some(active) = &attack.active {
            self.after_strike(id, active, dealt, events);
        }
        events.extend(broke);
        let victim = attack.pierce.map(|(victim, _)| victim);
        let order: Vec<UnitId> = std::iter::once(target).chain(victim).chain([id]).collect();
        self.remove_fallen(&order, events);
        for clash in &clashes {
            self.award_combat(clash, events);
        }
        attack.move_after
    }

    /// Opens the chest at `pos` (validated) for unit `id`.
    fn open(&mut self, id: UnitId, pos: Pos, loot: Loot, events: &mut Vec<Event>) {
        self.opened.insert(pos);
        events.push(Event::ChestOpened {
            unit: id,
            pos,
            loot: loot.clone(),
        });
        match loot {
            Loot::Gold(n) => {
                self.gold = self.gold.saturating_add(n);
                events.push(Event::GoldChanged { gold: self.gold });
            }
            Loot::Item(item) => {
                if self.tables.items.consumable(&item).is_some() {
                    self.pack.gain(item);
                } else {
                    // Known (checked by `plan_open`).
                    let _ = shop::add_to_stock(&mut self.stock, &self.tables.items, &item);
                }
            }
        }
    }

    /// Equips `equipped` on unit `id` (already validated).
    fn equip(&mut self, id: UnitId, equipped: Equipped, events: &mut Vec<Event>) {
        if let Some(u) = self.unit_mut(id) {
            u.loadout.equipped = Some(equipped.clone());
            events.push(Event::Equipped { unit: id, equipped });
        }
    }

    /// Carries out unit `id`'s validated tile cast of `spell` on `pos` (now
    /// `from`).
    fn cast_on_tile(
        &mut self,
        id: UnitId,
        spell: &SpellId,
        pos: Pos,
        from: TerrainId,
        effect: &TerrainEffect,
        events: &mut Vec<Event>,
    ) {
        let with = Equipped::Spell(spell.clone());
        let equip = self
            .tables
            .spells
            .get(spell)
            .is_some_and(SpellDef::is_attack)
            && self
                .unit(id)
                .is_some_and(|u| u.loadout.equipped.as_ref() != Some(&with));
        if equip {
            self.equip(id, with, events);
        }
        events.push(Event::SpellCast {
            unit: id,
            spell: spell.clone(),
            target: CastTarget::Tile(pos),
        });
        self.set_terrain(pos, from, effect.to, events);
        if let EffectDuration::UntilCastersNextPhase { then, damage } = effect.lasts
            && let Some(u) = self.unit(id)
        {
            self.burning.push(Burning {
                pos,
                phase: Phase::of(u.faction),
                then,
                damage,
            });
        }
        self.spend_spell(id, spell, events);
    }

    /// Changes the terrain at `pos` (on the map, now `from`) to `to`, with its
    /// event.
    fn set_terrain(&mut self, pos: Pos, from: TerrainId, to: TerrainId, events: &mut Vec<Event>) {
        if let Some(tile) = self.map.tiles.get_mut(pos) {
            *tile = to;
            events.push(Event::TerrainChanged { pos, from, to });
        }
    }

    /// Burns out the tiles the current phase's side set burning, in the
    /// order they were set, burning whoever stands on them first.
    fn burn_out(&mut self, events: &mut Vec<Event>) {
        let phase = self.phase;
        let (done, left) = std::mem::take(&mut self.burning)
            .into_iter()
            .partition(|b| b.phase == phase);
        self.burning = left;
        for b in done {
            if let Some(u) = self.units.iter_mut().find(|u| u.pos == b.pos) {
                let amount = b.damage.clamp(0, (u.hp - 1).max(0));
                u.hp -= amount;
                events.push(Event::BurnDamage {
                    unit: u.id,
                    pos: b.pos,
                    amount,
                });
            }
            if let Some(&from) = self.map.tiles.get(b.pos) {
                self.set_terrain(b.pos, from, b.then, events);
            }
        }
    }

    /// Spends one use of `spell` by unit `id` (on the map), with its event.
    fn spend_spell(&mut self, id: UnitId, spell: &SpellId, events: &mut Vec<Event>) {
        if let Some(uses_left) = self.unit_mut(id).and_then(|u| u.spells.spend(spell)) {
            events.push(Event::SpellUsesChanged {
                unit: id,
                spell: spell.clone(),
                uses_left,
            });
        }
    }

    /// Uses up item `index` of unit `id`'s source (validated) on `target`.
    fn use_item(
        &mut self,
        id: UnitId,
        index: usize,
        item: ItemId,
        effect: ConsumableEffect,
        target: UnitId,
        events: &mut Vec<Event>,
    ) {
        self.pack.items.remove(index);
        events.push(Event::ItemUsed {
            unit: id,
            item,
            target,
        });
        if let Some(t) = self.unit_mut(target) {
            let max = t.stats.hp;
            let healed = match effect {
                ConsumableEffect::Heal(amount) => t.hp.saturating_add(amount.max(0)).min(max),
                ConsumableEffect::HealFull => max,
            }
            .max(t.hp);
            let amount = healed - t.hp;
            t.hp = healed;
            events.push(Event::Healed { target, amount });
        }
    }

    /// Plays out `attacker`'s validated combat: the combat, spell uses, an
    /// art's debuff, Line Pierce's strike, then weapon EXP. Returns the HP
    /// each side's strikes removed: `[attacker, defender]` (the pierce's
    /// damage included), and each combat's tally (the pierce's second).
    /// Whoever fell is still on the map (see [`Self::remove_fallen`]).
    fn fight(
        &mut self,
        attacker: UnitId,
        step: &AttackStep,
        events: &mut Vec<Event>,
    ) -> ([StatValue; 2], Vec<Tally>) {
        let defender = step.target;
        let first = self.clash(attacker, defender, step.forecast, events);
        let mut clashes = vec![first];
        let mut tally = first;
        for ((id, spell), struck) in [attacker, defender]
            .into_iter()
            .zip(&step.arms.spells)
            .zip(tally.struck)
        {
            if let Some(spell) = spell.as_ref().filter(|_| struck > 0) {
                self.spend_spell(id, spell, events);
            }
        }
        if let Some(art) = &step.art {
            if tally.hits[0] > 0 {
                self.debuff(defender, art, events);
            }
            let standing = self.unit(attacker).is_some_and(|u| u.hp > 0);
            if let Some((victim, pierce)) = step.pierce.filter(|_| standing) {
                // The attacker always struck in the combat, so only its hits
                // and damage change.
                let t = self.clash(attacker, victim, pierce, events);
                tally.hits[0] += t.hits[0];
                tally.dealt[0] += t.dealt[0];
                clashes.push(t);
            }
        }
        let used_art = [step.art.is_some(), false];
        let exp = [0, 1].map(|i| {
            self.tables.items.rules.weapon_exp(
                tally.struck[i],
                tally.hits[i],
                tally.dealt[i],
                used_art[i],
            )
        });
        for ((id, kind), amount) in [attacker, defender]
            .into_iter()
            .zip(step.arms.kinds)
            .zip(exp)
        {
            if let Some(kind) = kind {
                self.gain_weapon_exp(id, kind, amount, events);
            }
        }
        (tally.dealt, clashes)
    }

    /// Resolves one combat of `attacker` against `defender` with `forecast`
    /// and the battle's RNG, sets both units' HP and emits
    /// [`Event::CombatResolved`]. Returns what each side's strikes did.
    fn clash(
        &mut self,
        attacker: UnitId,
        defender: UnitId,
        forecast: Forecast,
        events: &mut Vec<Event>,
    ) -> Tally {
        let hp = |s: &Self, id| {
            s.unit(id)
                .map_or(CombatHp { current: 0, max: 0 }, |u| CombatHp {
                    current: u.hp,
                    max: u.stats.hp,
                })
        };
        let (a, d) = (hp(self, attacker), hp(self, defender));
        let outcome = resolve(
            &self.tables.items.combat_rules(),
            &forecast,
            a,
            d,
            &mut self.rng,
        );
        for (id, left) in [
            (attacker, outcome.attacker_hp),
            (defender, outcome.defender_hp),
        ] {
            if let Some(u) = self.unit_mut(id) {
                u.hp = left;
            }
        }
        let tally = Tally::of(&outcome, [attacker, defender], [a.current, d.current]);
        events.push(Event::CombatResolved {
            attacker,
            defender,
            forecast,
            outcome,
        });
        tally
    }

    /// Moves the units of `ids` at 0 HP to the fallen, in that order.
    fn remove_fallen(&mut self, ids: &[UnitId], events: &mut Vec<Event>) {
        for &id in ids {
            if let Some(i) = self.units.iter().position(|u| u.id == id && u.hp <= 0) {
                self.fallen.push(self.units.remove(i));
                events.push(Event::UnitFell { unit: id });
            }
        }
    }

    /// Gives unit `id`, if still standing, `amount` weapon EXP in `kind`.
    fn gain_weapon_exp(
        &mut self,
        id: UnitId,
        kind: WeaponKind,
        amount: u32,
        events: &mut Vec<Event>,
    ) {
        if amount == 0 {
            return;
        }
        let tables = self.tables.clone();
        let Some(unit) = self.unit_mut(id).filter(|u| u.hp > 0) else {
            return;
        };
        let Some(max) = tables
            .classes
            .get(&unit.class)
            .and_then(|c| c.weapon(kind))
            .map(|w| w.max)
        else {
            return;
        };
        let rank_up = unit.gain_weapon_exp(kind, amount, max, &tables.items.rules);
        events.push(Event::WeaponExpGained {
            unit: id,
            kind,
            amount,
        });
        if let Some(rank) = rank_up {
            events.push(Event::WeaponRankUp {
                unit: id,
                kind,
                rank,
            });
        }
    }

    /// Ends the current phase and starts the next one that isn't skipped,
    /// ending turns on the way. A turn can't pass without a phase starting:
    /// the Player phase always has units (no player units is a defeat), so
    /// at most [`Phase::ALL`]`.len()` slots are visited.
    fn end_phase(&mut self, events: &mut Vec<Event>) {
        for _ in Phase::ALL {
            if self.phase == Phase::Other {
                if let Some(outcome) = self.end_of_turn() {
                    self.finish(outcome, events);
                    return;
                }
                self.turn += 1;
            }
            self.phase = self.phase.next();
            if self.start_phase(events) {
                return;
            }
        }
    }

    /// Starts the current phase (see the module docs). Returns `false` if it
    /// is skipped: no units of its factions, even after arrivals. A skipped
    /// phase still burns out its tiles (their only events).
    fn start_phase(&mut self, events: &mut Vec<Event>) -> bool {
        self.burn_out(events);
        self.expire_effects(events);
        let phase = self.phase;
        for u in self
            .units
            .iter_mut()
            .filter(|u| Phase::of(u.faction) == phase)
        {
            u.acted = false;
        }
        let arrived = self.arrive();
        if !self.units.iter().any(|u| Phase::of(u.faction) == phase) {
            return false;
        }
        if !arrived.is_empty() {
            events.push(Event::UnitsArrived { units: arrived });
        }
        events.push(Event::PhaseStarted {
            turn: self.turn,
            phase,
        });
        true
    }

    /// Places the current phase's due reinforcements whose tiles are free,
    /// already done. Returns their ids.
    fn arrive(&mut self) -> Vec<UnitId> {
        let mut arrived = Vec::new();
        let mut waiting = Vec::new();
        for r in std::mem::take(&mut self.pending) {
            let due = r.turn <= self.turn && Phase::of(r.unit.faction) == self.phase;
            if due && !self.units.iter().any(|u| u.pos == r.unit.pos) {
                let mut unit = r.unit;
                unit.acted = true;
                arrived.push(unit.id);
                self.units.push(unit);
            } else {
                waiting.push(r);
            }
        }
        self.pending = waiting;
        arrived
    }

    /// The outcome decided by the units alone: defeat first, then the
    /// objective. Turn-based results are [`Self::end_of_turn`]'s.
    fn judge(&self) -> Option<Outcome> {
        let lord_fell = self
            .fallen
            .iter()
            .any(|u| u.faction == Faction::Player && u.is_lord);
        let side_left = |f| self.units.iter().any(|u| u.faction == f);
        if lord_fell || !side_left(Faction::Player) {
            return Some(Outcome::Defeat);
        }
        let won = match self.objective {
            Objective::Rout { .. } => !side_left(Faction::Enemy),
            Objective::DefeatUnit { unit, .. } => self.fallen.iter().any(|u| u.id == unit),
            Objective::Seize { .. } | Objective::Survive { .. } => false,
        };
        won.then_some(Outcome::Victory)
    }

    /// The outcome decided as the current turn's last phase ends.
    fn end_of_turn(&self) -> Option<Outcome> {
        match self.objective {
            Objective::Survive { turns } => (self.turn == turns).then_some(Outcome::Victory),
            other => (other.turn_limit() == Some(self.turn)).then_some(Outcome::Defeat),
        }
    }

    fn finish(&mut self, outcome: Outcome, events: &mut Vec<Event>) {
        if outcome == Outcome::Victory {
            self.share_exp_pool(events);
        }
        self.outcome = Some(outcome);
        events.push(Event::BattleEnded { outcome });
    }

    /// Gives unit `id`, if on the map (the fallen are gone by now), `exp`
    /// EXP and `cp` class points if it is a player unit, or puts `exp` in
    /// the EXP pool if it is an ally.
    fn award(&mut self, id: UnitId, exp: u32, cp: ClassPoints, events: &mut Vec<Event>) {
        let Some(unit) = self.units.iter_mut().find(|u| u.id == id) else {
            return;
        };
        match unit.faction {
            Faction::Player => {
                let classes = &self.tables.classes;
                events.extend(progression::grant_exp(unit, exp, classes, &mut self.rng));
                let skills = &self.tables.skills;
                events.extend(progression::grant_class_points(unit, cp, classes, skills));
            }
            Faction::Ally => self.exp_pool = self.exp_pool.saturating_add(exp),
            Faction::Enemy | Faction::Neutral => {}
        }
    }

    /// Unit EXP and class points for both sides of one combat: each side
    /// still standing that struck, attacker first.
    fn award_combat(&mut self, tally: &Tally, events: &mut Vec<Event>) {
        for me in 0..2 {
            let (id, other) = (tally.units[me], tally.units[1 - me]);
            let (Some(unit), Some(target)) = (self.unit(id), self.any_unit(other)) else {
                continue;
            };
            if tally.struck[me] == 0 {
                continue;
            }
            let result = if tally.killed[me] {
                CombatResult::Killed
            } else if tally.dealt[me] > 0 {
                CombatResult::Damaged
            } else {
                CombatResult::NoDamage
            };
            let boss = target.role == Role::Boss;
            let exp = progression::exp_for_combat(unit.level, target.level, result, boss);
            let cp = progression::cp_for_combat(result == CombatResult::Killed);
            self.award(id, exp, cp, events);
        }
    }

    /// Unit EXP and class points for unit `id`'s non-combat active: the
    /// higher of the active's award, a heal's (if it healed) and a kill's
    /// for each hostile unit it felled; CP for the action, plus a kill's if
    /// it felled one (Nick).
    fn award_skill(&mut self, id: UnitId, award: &SkillAward, events: &mut Vec<Event>) {
        let Some(user) = self.unit(id) else {
            return;
        };
        let mut exp = progression::exp_for_active_skill();
        if award.healed {
            exp = exp.max(progression::exp_for_heal());
        }
        let mut felled = false;
        for &victim in &award.pushed {
            let Some(v) = self.fallen.iter().find(|u| u.id == victim) else {
                continue;
            };
            if !user.faction.is_hostile_to(v.faction) {
                continue;
            }
            let boss = v.role == Role::Boss;
            let kill = progression::exp_for_combat(user.level, v.level, CombatResult::Killed, boss);
            exp = exp.max(kill);
            felled = true;
        }
        let kill_cp = if felled { progression::KILL_CP } else { 0 };
        self.award(id, exp, progression::ACTION_CP + kill_cp, events);
    }

    /// The unit `id`, on the map or fallen.
    fn any_unit(&self, id: UnitId) -> Option<&Unit> {
        self.unit(id)
            .or_else(|| self.fallen.iter().find(|u| u.id == id))
    }

    /// Shares the EXP pool out among the player units on the map below the
    /// level cap: `pool / count` each (remainder lost), in unit order.
    fn share_exp_pool(&mut self, events: &mut Vec<Event>) {
        let pool = std::mem::take(&mut self.exp_pool);
        let cap = self.tables.classes.level_cap;
        let eligible = |u: &Unit| u.faction == Faction::Player && u.level < cap;
        let count = self.units.iter().filter(|u| eligible(u)).count();
        // No one to share with: the pool is lost. A share of 0 gives nothing.
        let Some(share) = u32::try_from(count).ok().and_then(|n| pool.checked_div(n)) else {
            return;
        };
        let classes = &self.tables.classes;
        for unit in self.units.iter_mut().filter(|u| eligible(u)) {
            events.extend(progression::grant_exp(unit, share, classes, &mut self.rng));
        }
    }
}

/// What a non-combat active did that its award depends on.
struct SkillAward {
    /// It healed someone (Sanctuary).
    healed: bool,
    /// Units a push may have felled (Shove): the pushed unit and the unit
    /// it hit.
    pushed: Vec<UnitId>,
}

impl SkillAward {
    fn of(effect: &SkillStep) -> SkillAward {
        match effect {
            SkillStep::Buff { .. } => SkillAward {
                healed: false,
                pushed: vec![],
            },
            SkillStep::Heal { heals } => SkillAward {
                healed: !heals.is_empty(),
                pushed: vec![],
            },
            SkillStep::Push { target, struck, .. } => SkillAward {
                healed: false,
                pushed: std::iter::once(*target).chain(*struck).collect(),
            },
        }
    }
}

/// What each side's strikes did in a combat: `[attacker, defender]`.
#[derive(Clone, Copy)]
struct Tally {
    /// The two units.
    units: [UnitId; 2],
    /// Strikes made.
    struck: [usize; 2],
    /// Strikes that hit.
    hits: [usize; 2],
    /// HP removed (Absorb healing doesn't count).
    dealt: [StatValue; 2],
    /// Whether each side's strike took the other to 0 HP.
    killed: [bool; 2],
}

impl Tally {
    /// The tally of `outcome` between `units`, given their HP going in.
    fn of(outcome: &CombatOutcome, units: [UnitId; 2], hp_before: [StatValue; 2]) -> Tally {
        // HP before each strike: [attacker, defender].
        let mut hp = hp_before;
        let mut t = Tally {
            units,
            struck: [0; 2],
            hits: [0; 2],
            dealt: [0; 2],
            killed: [false; 2],
        };
        for s in &outcome.strikes {
            let (me, target) = match s.by {
                Side::Attacker => (0, 1),
                Side::Defender => (1, 0),
            };
            t.struck[me] += 1;
            t.hits[me] += usize::from(s.hit);
            if !s.healed {
                t.dealt[me] += (hp[target] - s.target_hp_after).max(0);
            }
            hp[target] = s.target_hp_after;
            if hp[target] <= 0 {
                t.killed[me] = true;
            }
        }
        t
    }
}

mod arts;
mod skills;

pub use arts::AttackPreview;
use arts::{ArtUse, check_arts_allowed};
use skills::{ActiveUse, AttackPlan, Fight, SkillStep};

#[cfg(test)]
pub(crate) mod tests;
