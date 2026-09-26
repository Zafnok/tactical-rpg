//! Weapon kinds and ranks (`docs/design/weapons-and-items.md`). Weapons
//! themselves (might, hit, durability…) are ticket 0306.

use serde::{Deserialize, Serialize};

/// A weapon kind. Magic is not a weapon (see [`crate::magic`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WeaponKind {
    /// Stronger follow-up strikes.
    Sword,
    /// Effective against mounted units.
    Spear,
    /// Minimum damage, lower accuracy.
    Axe,
    /// Effective against fliers, range 2.
    Bow,
    /// Evasive and light.
    Gauntlet,
}

/// Weapon skill rank, lowest first: `E < D < C < B < A < S`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeaponRank {
    /// Lowest rank.
    E,
    /// Second rank.
    D,
    /// Third rank.
    C,
    /// Fourth rank.
    B,
    /// Fifth rank.
    A,
    /// Highest rank.
    S,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_are_ordered() {
        use WeaponRank::{A, B, C, D, E, S};
        let ranks = [E, D, C, B, A, S];
        assert!(ranks.windows(2).all(|w| w[0] < w[1]));
    }
}
