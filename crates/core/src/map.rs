//! The battle map: a named grid of terrain, plus shops and chests on some
//! tiles. Units live elsewhere (0302).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::geom::{Grid, Pos};
use crate::shop::{Loot, Shop};
use crate::terrain::TerrainId;

/// Something on a map tile that a unit standing there can use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileFeature {
    /// A shop (the `Shop` action).
    Shop(Shop),
    /// A chest (the `Open` action); whether it was opened is battle state.
    Chest(Loot),
}

/// A battle map: its name, the terrain of every tile and its features.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleMap {
    /// Human-readable map name, e.g. `"Test Field"`.
    pub name: String,
    /// The terrain of each tile, indexing a [`crate::TerrainTable`].
    pub tiles: Grid<TerrainId>,
    /// Shops and chests, by tile.
    #[serde(default)]
    pub features: BTreeMap<Pos, TileFeature>,
}

impl BattleMap {
    /// A map with no features.
    pub fn new(name: impl Into<String>, tiles: Grid<TerrainId>) -> Self {
        Self {
            name: name.into(),
            tiles,
            features: BTreeMap::new(),
        }
    }

    /// The shop at `pos`, if any.
    pub fn shop(&self, pos: Pos) -> Option<&Shop> {
        match self.features.get(&pos)? {
            TileFeature::Shop(s) => Some(s),
            TileFeature::Chest(_) => None,
        }
    }

    /// The chest at `pos` (opened or not), if any.
    pub fn chest(&self, pos: Pos) -> Option<&Loot> {
        match self.features.get(&pos)? {
            TileFeature::Chest(l) => Some(l),
            TileFeature::Shop(_) => None,
        }
    }
}
