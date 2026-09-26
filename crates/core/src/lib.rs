//! Pure, deterministic game rules. See ADR-0004.

pub mod class;
pub mod combat;
pub mod geom;
pub mod magic;
pub mod map;
pub mod movement;
pub mod rng;
pub mod stats;
pub mod terrain;
pub mod unit;
pub mod weapon;

pub use class::{
    ArmourWeight, ClassDef, ClassId, ClassLevel, ClassPoints, ClassTable, SkillId, Tier, UnitTag,
    UnitTags, WeaponProficiency,
};
pub use combat::{
    CombatHp, CombatOutcome, CombatRules, CombatantInput, DamageType, Forecast, Side, SideForecast,
    Strike, WeaponStats, WeaponTrait, forecast, resolve, roll_hit,
};
pub use geom::{Dir, Grid, Pos};
pub use magic::{Affinity, Element, SpellId};
pub use map::BattleMap;
pub use movement::{
    AttackRange, MoveError, PathError, Reach, TileSet, attack_tiles, danger_zone, path_cost,
    reachable, threat_area,
};
pub use rng::{RandomSource, ScriptedRng, SimRng};
pub use stats::{GrowthValue, Growths, StatKind, StatValue, Stats};
pub use terrain::{MovementTypeId, TerrainId, TerrainRules, TerrainTable};
pub use unit::{
    CharacterDef, CharacterId, ClassRecord, Faction, Level, MAP_LABEL_LEN, Unit, UnitError, UnitId,
    default_map_label, is_valid_map_label,
};
pub use weapon::{WeaponKind, WeaponRank};
