//! The battle map: a named grid of terrain. Units live elsewhere (0302).

use crate::geom::Grid;
use crate::terrain::TerrainId;

/// A battle map: its name and the terrain of every tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleMap {
    /// Human-readable map name, e.g. `"Test Field"`.
    pub name: String,
    /// The terrain of each tile, indexing a [`crate::TerrainTable`].
    pub tiles: Grid<TerrainId>,
}
