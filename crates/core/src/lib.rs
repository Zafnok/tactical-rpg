//! Pure, deterministic game rules. See ADR-0004.

pub mod art;
pub mod battle;
pub mod class;
pub mod combat;
pub mod geom;
pub mod item;
pub mod magic;
pub mod map;
pub mod movement;
pub mod rng;
pub mod shop;
pub mod skill;
pub mod spell;
pub mod stats;
pub mod terrain;
pub mod unit;
pub mod weapon;

pub use art::{ArtDef, ArtEffect, ArtId, ArtNote, ArtTable, Debuff};
pub use battle::{
    AttackPreview, BattleSetup, BattleState, Burning, CastTarget, Command, CommandError,
    Destination, Event, Objective, Outcome, PendingMove, Phase, Reinforcement, SellFrom, ShopTxn,
    Turn, UnitAction,
};
pub use class::{
    ArmourWeight, ClassDef, ClassId, ClassLevel, ClassPoints, ClassTable, Tier, UnitTag, UnitTags,
    WeaponProficiency,
};
pub use combat::{
    CombatHp, CombatMods, CombatOutcome, CombatRules, CombatantInput, DamageType, Forecast, Side,
    SideForecast, Strike, WeaponStats, WeaponTrait, forecast, resolve, roll_hit,
};
pub use geom::{Dir, Grid, GridSizeError, Pos};
pub use item::{
    AccessoryDef, ArmourDef, BattlePack, ConsumableDef, ConsumableEffect, Equipped, ItemDef,
    ItemId, ItemTable, Loadout, LoadoutDef, LoadoutError, Stock, WEAPON_SLOTS, WeaponDef,
    WeaponInstance, WeaponRules,
};
pub use magic::{Affinity, Element};
pub use map::{BattleMap, TileFeature};
pub use movement::{
    AttackRange, MoveError, PathError, Reach, TileSet, attack_tiles, danger_zone, path_cost,
    reachable, threat_area,
};
pub use rng::{RandomSource, ScriptedRng, SimRng};
pub use shop::{Gold, Loot, Shop, ShopError, ShopKind, ShopSession, repair_cost, sell_price};
pub use skill::{
    ActiveEffect, Area, Bonuses, Condition, CostError, CostSource, EffectSource, Paid,
    PassiveEffect, SkillContext, SkillCost, SkillDef, SkillId, SkillKind, SkillTable, Stance,
    TimedEffect, TimedMods, WeaponReq, check_cost, pay_cost,
};
pub use spell::{
    EffectDuration, SpellChanges, SpellDef, SpellId, SpellKind, SpellState, SpellTable,
    TerrainEffect,
};
pub use stats::{GrowthValue, Growths, StatKind, StatValue, Stats};
pub use terrain::{MovementTypeId, TerrainId, TerrainRules, TerrainTable};
pub use unit::{
    CharacterDef, CharacterId, ClassRecord, Faction, Level, MAP_LABEL_LEN, Unit, UnitError, UnitId,
    default_map_label, is_valid_map_label,
};
pub use weapon::{WeaponKind, WeaponRank};
