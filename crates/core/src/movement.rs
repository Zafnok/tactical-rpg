//! Movement: where a unit can go, the path it takes, and what it can attack.
//! Powers the blue move overlay, the red attack overlay, the path arrow, the
//! enemy danger zone and the AI.
//!
//! # Rules
//!
//! - **Budget.** A unit has its Mov (`stats.mov`, set by its class) in move
//!   points per move. A negative Mov counts as 0.
//! - **Step cost.** Entering a tile costs the terrain's `move_cost` for the
//!   mover's class movement type (`docs/design/terrain.md`). `None` means the
//!   tile can't be entered. The start tile costs nothing. Only the 4 grid
//!   directions are steps.
//! - **Units.** A unit can move *through* tiles holding units that are not
//!   hostile to it ([`Faction::is_hostile_to`]) but never *into* a tile
//!   holding a hostile unit. It can't *end* its move on a tile holding another
//!   unit. So [`Reach`] has two sets: *passable* tiles (reached within budget,
//!   with their cheapest cost) and *stoppable* tiles (passable and empty,
//!   plus the start tile, always).
//! - **No Canto, split movement, zones of control or flying over units**
//!   (`docs/design/turn-structure.md`: FE standard only). Movement skills are
//!   separate, later rules.
//! - **Determinism.** Dijkstra with the priority key `(cost, y, x)`;
//!   neighbours are visited in [`Dir::ALL`](crate::Dir::ALL) order and a tile's recorded
//!   predecessor only changes for a strictly cheaper cost. So among equally
//!   cheap paths the one found first (smallest key) wins, every time.
//! - **Attack tiles** ([`attack_tiles`]): every tile at Manhattan distance
//!   `min..=max` from any stoppable tile, except the stoppable tiles
//!   themselves (the red overlay drawn around the blue one, as in Fire
//!   Emblem). Walls and other impassable tiles are included: range ignores
//!   terrain.
//! - **Threat** ([`threat_area`]): stoppable tiles plus attack tiles for each
//!   of the unit's weapon ranges. A unit with no ranges (no usable weapon)
//!   threatens nothing. The **danger zone** ([`danger_zone`]) is the union of
//!   the threat areas of every unit hostile to a faction.
//! - Every unit in the `units` slice occupies its tile; callers pass the
//!   living units of the battle.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt;

use crate::class::{ClassId, ClassTable};
use crate::geom::{Grid, Pos};
use crate::map::BattleMap;
use crate::terrain::{MovementTypeId, TerrainTable};
use crate::unit::{Faction, Unit, UnitId};

/// A weapon range: `(min, max)` Manhattan distance, inclusive.
pub type AttackRange = (u32, u32);

/// A set of tiles of a `width × height` map, stored as a bitset. Iterates in
/// row-major order. Positions outside the map are never members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSet {
    width: u16,
    height: u16,
    words: Vec<u64>,
}

impl TileSet {
    /// An empty set for a `width × height` map.
    pub fn new(width: u16, height: u16) -> Self {
        let bits = usize::from(width) * usize::from(height);
        Self {
            width,
            height,
            words: vec![0; bits.div_ceil(64)],
        }
    }

    /// Map width.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Whether `pos` is in the set.
    pub fn contains(&self, pos: Pos) -> bool {
        self.bit(pos)
            .is_some_and(|(w, mask)| self.words.get(w).is_some_and(|word| word & mask != 0))
    }

    /// Adds `pos`; returns whether it was newly added (`false` if already a
    /// member or outside the map).
    pub fn insert(&mut self, pos: Pos) -> bool {
        let Some((w, mask)) = self.bit(pos) else {
            return false;
        };
        let Some(word) = self.words.get_mut(w) else {
            return false;
        };
        let added = *word & mask == 0;
        *word |= mask;
        added
    }

    /// Adds every member of `other` that lies inside this set's map.
    pub fn union_with(&mut self, other: &TileSet) {
        for pos in other.iter() {
            self.insert(pos);
        }
    }

    /// Number of members.
    pub fn len(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Whether the set has no members.
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// Every member, row-major.
    pub fn iter(&self) -> impl Iterator<Item = Pos> + '_ {
        let width = usize::from(self.width);
        self.words
            .iter()
            .enumerate()
            .filter(|&(_, &word)| word != 0)
            .flat_map(move |(w, &word)| {
                (0..64)
                    .filter(move |bit| word >> bit & 1 == 1)
                    .filter_map(move |bit| {
                        let index = w * 64 + bit;
                        // Both fit: index < width * height ≤ u16::MAX².
                        let x = i32::try_from(index % width).ok()?;
                        let y = i32::try_from(index / width).ok()?;
                        Some(Pos::new(x, y))
                    })
            })
    }

    /// The word index and bit mask of `pos`, or `None` outside the map.
    fn bit(&self, pos: Pos) -> Option<(usize, u64)> {
        let x = u16::try_from(pos.x).ok().filter(|&x| x < self.width)?;
        let y = u16::try_from(pos.y).ok().filter(|&y| y < self.height)?;
        let index = usize::from(y) * usize::from(self.width) + usize::from(x);
        Some((index / 64, 1 << (index % 64)))
    }
}

/// Why movement could not be worked out for a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveError {
    /// No unit with this id is in the `units` slice.
    UnknownUnit(UnitId),
    /// The unit's class is not in the class table.
    UnknownClass(ClassId),
    /// The unit stands outside the map.
    OffMap(UnitId),
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveError::UnknownUnit(id) => write!(f, "no unit with id {}", id.0),
            MoveError::UnknownClass(c) => write!(f, "unknown class \"{}\"", c.0),
            MoveError::OffMap(id) => write!(f, "unit {} is outside the map", id.0),
        }
    }
}

impl std::error::Error for MoveError {}

/// Why a drawn path is not a legal move. `index` is the position in the path
/// of the offending tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// The mover itself is invalid.
    Mover(MoveError),
    /// The path has no tiles.
    Empty,
    /// The path does not start on the mover's tile.
    WrongStart,
    /// A step is not to a 4-way neighbour.
    NotAdjacent {
        /// Index of the tile stepped to.
        index: usize,
    },
    /// A step enters a tile the mover can't enter (or leaves the map).
    Impassable {
        /// Index of the tile stepped to.
        index: usize,
    },
    /// A step enters a tile holding a hostile unit.
    Hostile {
        /// Index of the tile stepped to.
        index: usize,
    },
    /// The path costs more than the mover's Mov.
    OverBudget {
        /// Total cost of the path.
        cost: u32,
        /// The mover's move points.
        budget: u32,
    },
}

impl From<MoveError> for PathError {
    fn from(e: MoveError) -> Self {
        PathError::Mover(e)
    }
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::Mover(e) => e.fmt(f),
            PathError::Empty => f.write_str("the path is empty"),
            PathError::WrongStart => f.write_str("the path doesn't start at the unit"),
            PathError::NotAdjacent { index } => {
                write!(f, "step {index} isn't to a neighbouring tile")
            }
            PathError::Impassable { index } => write!(f, "step {index} is impassable"),
            PathError::Hostile { index } => write!(f, "step {index} enters an enemy's tile"),
            PathError::OverBudget { cost, budget } => {
                write!(f, "the path costs {cost} but the unit has {budget} move")
            }
        }
    }
}

impl std::error::Error for PathError {}

/// Where one unit can move this turn. See the module docs for the rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    origin: Pos,
    budget: u32,
    cost: Grid<Option<u32>>,
    prev: Grid<Option<Pos>>,
    stoppable: TileSet,
}

impl Reach {
    /// The mover's tile.
    pub fn origin(&self) -> Pos {
        self.origin
    }

    /// The mover's move points.
    pub fn budget(&self) -> u32 {
        self.budget
    }

    /// The cheapest cost to reach `pos`, or `None` if it is not passable.
    pub fn cost(&self, pos: Pos) -> Option<u32> {
        self.cost.get(pos).copied().flatten()
    }

    /// Whether the unit can reach `pos` (possibly without stopping there).
    pub fn is_passable(&self, pos: Pos) -> bool {
        self.cost(pos).is_some()
    }

    /// Whether the unit can end its move on `pos`.
    pub fn is_stoppable(&self, pos: Pos) -> bool {
        self.stoppable.contains(pos)
    }

    /// Every passable tile.
    pub fn passable(&self) -> TileSet {
        let mut set = TileSet::new(self.cost.width(), self.cost.height());
        for (pos, cost) in self.cost.positions().zip(self.cost.cells()) {
            if cost.is_some() {
                set.insert(pos);
            }
        }
        set
    }

    /// Every tile the unit can end its move on (always includes the origin).
    pub fn stoppable(&self) -> &TileSet {
        &self.stoppable
    }

    /// The cheapest path from the origin to `dest`, both included, or `None`
    /// if `dest` is not passable. Deterministic (see the module docs).
    pub fn path_to(&self, dest: Pos) -> Option<Vec<Pos>> {
        self.cost(dest)?;
        let mut path = vec![dest];
        let mut cur = dest;
        while let Some(&Some(p)) = self.prev.get(cur) {
            path.push(p);
            cur = p;
        }
        path.reverse();
        Some(path)
    }
}

/// The unit `id`, its movement type and move points.
fn mover_info<'a>(
    map: &BattleMap,
    classes: &ClassTable,
    units: &'a [Unit],
    id: UnitId,
) -> Result<(&'a Unit, MovementTypeId, u32), MoveError> {
    let unit = units
        .iter()
        .find(|u| u.id == id)
        .ok_or(MoveError::UnknownUnit(id))?;
    let class = classes
        .get(&unit.class)
        .ok_or_else(|| MoveError::UnknownClass(unit.class.clone()))?;
    if !map.tiles.in_bounds(unit.pos) {
        return Err(MoveError::OffMap(id));
    }
    let budget = u32::try_from(unit.stats.mov).unwrap_or(0);
    Ok((unit, class.movement_type, budget))
}

/// Whether a unit hostile to `faction` stands on `pos`.
fn hostile_at(units: &[Unit], faction: Faction, pos: Pos) -> bool {
    units
        .iter()
        .any(|u| u.pos == pos && u.faction.is_hostile_to(faction))
}

/// Where unit `mover` can move: Dijkstra from its tile with its Mov as
/// budget.
pub fn reachable(
    map: &BattleMap,
    terrain: &TerrainTable,
    classes: &ClassTable,
    units: &[Unit],
    mover: UnitId,
) -> Result<Reach, MoveError> {
    let (unit, movement_type, budget) = mover_info(map, classes, units, mover)?;
    let tiles = &map.tiles;
    let (width, height) = (tiles.width(), tiles.height());
    let mut occupied = TileSet::new(width, height);
    let mut hostile = TileSet::new(width, height);
    for other in units.iter().filter(|u| u.id != mover) {
        occupied.insert(other.pos);
        if other.faction.is_hostile_to(unit.faction) {
            hostile.insert(other.pos);
        }
    }

    let mut cost = Grid::filled(width, height, None);
    let mut prev = Grid::filled(width, height, None);
    if let Some(c) = cost.get_mut(unit.pos) {
        *c = Some(0);
    }
    // Tiles already expanded: their cost and predecessor are final.
    let mut done = TileSet::new(width, height);
    let mut queue = BinaryHeap::from([Reverse((0, unit.pos.y, unit.pos.x))]);
    while let Some(Reverse((c, y, x))) = queue.pop() {
        let pos = Pos::new(x, y);
        if !done.insert(pos) {
            continue; // A stale entry: `pos` was expanded at a lower cost.
        }
        for next in tiles.neighbors4(pos) {
            if done.contains(next) || hostile.contains(next) {
                continue;
            }
            let step = tiles
                .get(next)
                .and_then(|&t| terrain.move_cost(t, movement_type));
            let Some(step) = step else {
                continue;
            };
            let next_cost = c + u32::from(step);
            if next_cost > budget {
                continue;
            }
            let Some(slot) = cost.get_mut(next) else {
                continue;
            };
            if slot.is_none_or(|old| next_cost < old) {
                *slot = Some(next_cost);
                if let Some(p) = prev.get_mut(next) {
                    *p = Some(pos);
                }
                queue.push(Reverse((next_cost, next.y, next.x)));
            }
        }
    }

    let mut stoppable = TileSet::new(width, height);
    for (pos, c) in tiles.positions().zip(cost.cells()) {
        if c.is_some() && (pos == unit.pos || !occupied.contains(pos)) {
            stoppable.insert(pos);
        }
    }
    Ok(Reach {
        origin: unit.pos,
        budget,
        cost,
        prev,
        stoppable,
    })
}

/// The cost of moving unit `mover` along `path` (its own tile first), or why
/// the path is illegal: every step must be to a 4-way neighbour the mover can
/// enter, not holding a hostile unit, and the total must be within its Mov.
/// Where the path ends is not checked (the UI's arrow may point at any
/// passable tile). The UI's "arrow follows the cursor" path uses this.
pub fn path_cost(
    map: &BattleMap,
    terrain: &TerrainTable,
    classes: &ClassTable,
    units: &[Unit],
    mover: UnitId,
    path: &[Pos],
) -> Result<u32, PathError> {
    let (unit, movement_type, budget) = mover_info(map, classes, units, mover)?;
    let Some(&first) = path.first() else {
        return Err(PathError::Empty);
    };
    if first != unit.pos {
        return Err(PathError::WrongStart);
    }
    let mut total: u32 = 0;
    for (i, pair) in path.windows(2).enumerate() {
        let (from, to) = (pair[0], pair[1]);
        let index = i + 1;
        if Pos::manhattan(from, to) != 1 {
            return Err(PathError::NotAdjacent { index });
        }
        if hostile_at(units, unit.faction, to) {
            return Err(PathError::Hostile { index });
        }
        let step = map
            .tiles
            .get(to)
            .and_then(|&t| terrain.move_cost(t, movement_type))
            .ok_or(PathError::Impassable { index })?;
        total = total.saturating_add(u32::from(step));
    }
    if total > budget {
        return Err(PathError::OverBudget {
            cost: total,
            budget,
        });
    }
    Ok(total)
}

/// Every tile at Manhattan distance `min..=max` from a stoppable tile of
/// `reach`, except the stoppable tiles themselves. Empty if `min > max`.
pub fn attack_tiles(reach: &Reach, min: u32, max: u32) -> TileSet {
    let stoppable = reach.stoppable();
    let mut out = TileSet::new(stoppable.width(), stoppable.height());
    let (width, height) = (i64::from(stoppable.width()), i64::from(stoppable.height()));
    let max = i64::from(max);
    for from in stoppable.iter() {
        let (fx, fy) = (i64::from(from.x), i64::from(from.y));
        // The diamond of radius `max`, clipped to the map's rows and columns.
        for y in (fy - max).max(0)..(fy + max + 1).min(height) {
            let rest = max - (y - fy).abs();
            for x in (fx - rest).max(0)..(fx + rest + 1).min(width) {
                // Inside the map, so both fit.
                let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
                    continue;
                };
                let to = Pos::new(x, y);
                if Pos::manhattan(from, to) >= min && !stoppable.contains(to) {
                    out.insert(to);
                }
            }
        }
    }
    out
}

/// The tiles unit `unit` threatens this turn with weapons of the given
/// ranges (in a battle: [`Unit::attack_ranges`]): its stoppable tiles plus
/// their attack tiles. Empty if `ranges` is empty.
pub fn threat_area(
    map: &BattleMap,
    terrain: &TerrainTable,
    classes: &ClassTable,
    units: &[Unit],
    unit: UnitId,
    ranges: &[AttackRange],
) -> Result<TileSet, MoveError> {
    let reach = reachable(map, terrain, classes, units, unit)?;
    if ranges.is_empty() {
        return Ok(TileSet::new(map.tiles.width(), map.tiles.height()));
    }
    let mut set = reach.stoppable().clone();
    for &(min, max) in ranges {
        set.union_with(&attack_tiles(&reach, min, max));
    }
    Ok(set)
}

/// The union of the threat areas of every unit hostile to `faction`, with
/// each unit's weapon ranges given by `ranges` (in a battle:
/// [`Unit::attack_ranges`], the ranges of the weapons it can wield).
pub fn danger_zone(
    map: &BattleMap,
    terrain: &TerrainTable,
    classes: &ClassTable,
    units: &[Unit],
    faction: Faction,
    ranges: impl Fn(&Unit) -> Vec<AttackRange>,
) -> Result<TileSet, MoveError> {
    let mut zone = TileSet::new(map.tiles.width(), map.tiles.height());
    for u in units.iter().filter(|u| u.faction.is_hostile_to(faction)) {
        zone.union_with(&threat_area(
            map,
            terrain,
            classes,
            units,
            u.id,
            &ranges(u),
        )?);
    }
    Ok(zone)
}

#[cfg(test)]
mod tests;
