//! Pure, deterministic game rules. See ADR-0004.

pub mod class;
pub mod geom;
pub mod magic;
pub mod map;
pub mod stats;
pub mod terrain;
pub mod unit;
pub mod weapon;

pub use class::{
    ArmourWeight, ClassDef, ClassId, ClassLevel, ClassPoints, ClassTable, SkillId, Tier, UnitTag,
    UnitTags, WeaponProficiency,
};
pub use geom::{Dir, Grid, Pos};
pub use magic::{Affinity, Element, SpellId};
pub use map::BattleMap;
pub use stats::{GrowthValue, Growths, StatKind, StatValue, Stats};
pub use terrain::{MovementTypeId, TerrainId, TerrainRules, TerrainTable};
pub use unit::{CharacterDef, CharacterId, ClassRecord, Faction, Level, Unit, UnitError, UnitId};
pub use weapon::{WeaponKind, WeaponRank};
