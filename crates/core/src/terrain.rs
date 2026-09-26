//! Terrain rules: movement costs and combat bonuses per terrain type. How a
//! terrain looks lives in `trpg-content`, not here.

use serde::{Deserialize, Serialize};

/// Index of a movement type (`foot`, `mounted`, …) in [`TerrainTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MovementTypeId(pub u8);

/// Index of a terrain in [`TerrainTable`].
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct TerrainId(pub u16);

/// The game rules of one terrain type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainRules {
    /// Human-readable name, e.g. `"Forest"`.
    pub name: String,
    /// Cost to enter, indexed by [`MovementTypeId`]; `None` = impassable.
    pub move_cost: Vec<Option<u8>>,
    /// Added to Def and Res of a unit standing here.
    pub defense: i8,
    /// Added to the avoid of a unit standing here.
    pub avoid: i8,
    /// Percent of max HP restored at the start of the unit's phase.
    pub heal_percent: u8,
}

/// Every movement type and terrain, looked up by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerrainTable {
    /// Movement type names; index = [`MovementTypeId`].
    pub movement_types: Vec<String>,
    /// Terrain rules; index = [`TerrainId`].
    pub terrains: Vec<TerrainRules>,
}

impl TerrainTable {
    /// The rules of terrain `id`, if it exists.
    pub fn get(&self, id: TerrainId) -> Option<&TerrainRules> {
        self.terrains.get(usize::from(id.0))
    }

    /// The id of the movement type called `name`.
    pub fn movement_type(&self, name: &str) -> Option<MovementTypeId> {
        let index = self.movement_types.iter().position(|m| m == name)?;
        u8::try_from(index).ok().map(MovementTypeId)
    }

    /// The cost for movement type `mt` to enter terrain `id`; `None` if
    /// impassable (or either id is unknown).
    pub fn move_cost(&self, id: TerrainId, mt: MovementTypeId) -> Option<u8> {
        *self.get(id)?.move_cost.get(usize::from(mt.0))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> TerrainTable {
        let rules = |name: &str, move_cost: Vec<Option<u8>>| TerrainRules {
            name: name.to_owned(),
            move_cost,
            defense: 0,
            avoid: 0,
            heal_percent: 0,
        };
        TerrainTable {
            movement_types: vec!["foot".into(), "flying".into()],
            terrains: vec![
                rules("Plain", vec![Some(1), Some(1)]),
                rules("Forest", vec![Some(2), Some(1)]),
                rules("Sea", vec![None, Some(1)]),
            ],
        }
    }

    #[test]
    fn get_by_id() {
        let t = table();
        assert_eq!(t.get(TerrainId(1)).map(|r| r.name.as_str()), Some("Forest"));
        assert_eq!(t.get(TerrainId(3)), None);
    }

    #[test]
    fn movement_type_by_name() {
        let t = table();
        assert_eq!(t.movement_type("foot"), Some(MovementTypeId(0)));
        assert_eq!(t.movement_type("flying"), Some(MovementTypeId(1)));
        assert_eq!(t.movement_type("boat"), None);
    }

    #[test]
    fn move_costs() {
        let t = table();
        let (foot, flying) = (MovementTypeId(0), MovementTypeId(1));
        assert_eq!(t.move_cost(TerrainId(1), foot), Some(2));
        assert_eq!(t.move_cost(TerrainId(1), flying), Some(1));
        assert_eq!(t.move_cost(TerrainId(2), foot), None);
        assert_eq!(t.move_cost(TerrainId(2), flying), Some(1));
        assert_eq!(t.move_cost(TerrainId(9), foot), None);
        assert_eq!(t.move_cost(TerrainId(0), MovementTypeId(2)), None);
    }
}
