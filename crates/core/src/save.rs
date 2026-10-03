//! What a save holds (ticket 0802, ADR-0039): the campaign and where in it
//! the player stopped. `ui` turns it into text and stores it; this module
//! is only the data.
//!
//! # Rules
//!
//! Source: `docs/design/death-and-difficulty.md`, *Saving*.
//!
//! - **A chapter save** ([`SavePoint::ChapterCleared`]) is made after a
//!   chapter's victory, into one of the save slots. Its campaign's
//!   [`chapter`](Campaign::chapter) is the chapter just won, already applied
//!   to the army; loading it goes on with the chapter after that one.
//! - **The suspend save** ([`SavePoint::Battle`]) is made from the map menu
//!   mid-battle. It holds the battle's whole [`BattleHistory`] (its first
//!   state, every command since and the rewind charges left), so the battle
//!   continues exactly where it was, rewinds included. The campaign is the
//!   one the battle started with.
//! - **Versions.** Every save names the [`SAVE_VERSION`] that wrote it. A
//!   save of another version is refused, not migrated ([`SaveHeader`] reads
//!   the version of a save whose other fields no longer parse).

use serde::{Deserialize, Serialize};

use crate::campaign::Campaign;
use crate::history::BattleHistory;

/// The save format this build reads and writes. Raise it whenever a saved
/// type changes shape or meaning.
///
/// The golden saves of `crates/core/tests/it/save_format.rs` fail when a
/// saved type changed. Then raise `SAVE_VERSION` here, regenerate the
/// fixtures as `save_v<N>_*.ron` and delete the old ones (ADR-0039).
pub const SAVE_VERSION: u32 = 1;

/// One save: a slot's chapter save, or the suspend save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveFile {
    /// The [`SAVE_VERSION`] that wrote it.
    pub version: u32,
    /// The player's game (its playtime included).
    pub campaign: Campaign,
    /// Where in the campaign the save was made.
    pub point: SavePoint,
}

/// Where in the campaign a save was made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePoint {
    /// Between chapters: the campaign's chapter is won, and play goes on
    /// with the chapter after it.
    ChapterCleared,
    /// In the campaign's chapter's battle (the suspend save).
    Battle(Box<BattleHistory>),
}

/// Just a save's version: parses any save, whatever the rest of it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename = "SaveFile")]
pub struct SaveHeader {
    /// The [`SAVE_VERSION`] that wrote the save.
    pub version: u32,
}

impl SaveFile {
    /// A chapter save of `campaign`, whose chapter was just won.
    pub fn chapter_cleared(campaign: Campaign) -> Self {
        Self {
            version: SAVE_VERSION,
            campaign,
            point: SavePoint::ChapterCleared,
        }
    }

    /// A suspend save: `campaign` as the battle started with it, and the
    /// battle's `history`.
    pub fn suspended(campaign: Campaign, history: BattleHistory) -> Self {
        Self {
            version: SAVE_VERSION,
            campaign,
            point: SavePoint::Battle(Box::new(history)),
        }
    }
}

impl SaveHeader {
    /// Whether this build can read the save.
    pub fn is_current(self) -> bool {
        self.version == SAVE_VERSION
    }
}

#[cfg(test)]
mod tests;
