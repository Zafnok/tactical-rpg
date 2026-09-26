//! Spell ids, elements and affinities (`docs/design/magic.md`). Spell
//! definitions and uses are ticket 0309.

use serde::{Deserialize, Serialize};

/// String id of a spell, e.g. `"fire"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpellId(pub String);

/// A spell's element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Element {
    /// Fire: burns forests.
    Fire,
    /// Ice: freezes water.
    Ice,
    /// No element.
    None,
}

/// How a unit takes damage from one element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Affinity {
    /// Spell might ×3 (counts as effectiveness).
    Weak,
    /// Damage halved.
    Resist,
    /// A hit heals instead of hurting.
    Absorb,
}
