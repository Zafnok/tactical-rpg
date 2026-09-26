//! Pure, deterministic game rules. See ADR-0004.

pub mod geom;
pub mod map;
pub mod terrain;

pub use geom::{Dir, Grid, Pos};
pub use map::BattleMap;
pub use terrain::{MovementTypeId, TerrainId, TerrainRules, TerrainTable};
