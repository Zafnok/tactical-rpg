//! Saves in [`Storage`] (ticket 0802, ADR-0036): [`SaveFile`]s as RON text
//! under the keys `slot_01` … `slot_30` (the chapter saves) and `suspend`
//! (the one-time battle save).
//!
//! Reading a save never panics: text that isn't a save is
//! [`SaveError::Corrupt`], a save written by another
//! [`SAVE_VERSION`](trpg_core::SAVE_VERSION) is
//! [`SaveError::Incompatible`]. The player sees the error's text.

use std::fmt;

use trpg_content::Content;
use trpg_core::{Campaign, GameMode, SaveFile, SaveHeader, SavePoint};

use crate::storage::{Storage, StorageError};

/// How many save slots there are (`death-and-difficulty.md`).
pub const SLOTS: usize = 30;

/// [`Storage`] key of the suspend save.
pub const SUSPEND_KEY: &str = "suspend";

/// [`Storage`] key of save slot `slot` (1 to [`SLOTS`]): `slot_01` …
pub fn slot_key(slot: usize) -> String {
    format!("slot_{slot:02}")
}

/// Why a save couldn't be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The save was written by another [`SAVE_VERSION`](trpg_core::SAVE_VERSION).
    Incompatible,
    /// The text isn't a save, or names things this game doesn't have.
    Corrupt,
    /// There is no save there.
    Missing,
    /// The storage failed.
    Storage(StorageError),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incompatible => f.write_str("This save is from an incompatible version"),
            Self::Corrupt => f.write_str("This save can't be read"),
            Self::Missing => f.write_str("There is no save here"),
            Self::Storage(e) => write!(f, "Save {e}"),
        }
    }
}

impl std::error::Error for SaveError {}

/// `save` as text.
pub fn encode(save: &SaveFile) -> Result<String, SaveError> {
    ron::to_string(save).map_err(|e| SaveError::Storage(StorageError::Backend(e.to_string())))
}

/// The save in `text`, if it is one of this build's
/// [`SAVE_VERSION`](trpg_core::SAVE_VERSION).
pub fn decode(text: &str) -> Result<SaveFile, SaveError> {
    let header: SaveHeader = ron::from_str(text).map_err(|_| SaveError::Corrupt)?;
    if !header.is_current() {
        return Err(SaveError::Incompatible);
    }
    ron::from_str(text).map_err(|_| SaveError::Corrupt)
}

/// The save stored at `key`; `None` if there is none.
pub fn read(storage: &dyn Storage, key: &str) -> Result<Option<SaveFile>, SaveError> {
    match storage.read(key).map_err(SaveError::Storage)? {
        Some(text) => decode(&text).map(Some),
        None => Ok(None),
    }
}

/// Stores `save` at `key`, replacing what was there.
pub fn write(storage: &mut dyn Storage, key: &str, save: &SaveFile) -> Result<(), SaveError> {
    let text = encode(save)?;
    storage.write(key, &text).map_err(SaveError::Storage)
}

/// Whether there is a suspend save (readable or not).
pub fn has_suspend(storage: &dyn Storage) -> bool {
    matches!(storage.read(SUSPEND_KEY), Ok(Some(_)))
}

/// Whether any save slot holds something (readable or not).
pub fn any_slot(storage: &dyn Storage) -> bool {
    (1..=SLOTS).any(|slot| matches!(storage.read(&slot_key(slot)), Ok(Some(_))))
}

/// What a save slot holds, as the slot picker shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    /// Nothing.
    Empty,
    /// A chapter save.
    Saved(Box<SlotSummary>),
    /// Something that can't be loaded, and why.
    Unreadable(SaveError),
}

/// A chapter save's line in the slot picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotSummary {
    /// The title of the chapter the save goes on with
    /// ([`next_chapter_title`]).
    pub chapter: String,
    /// Classic or Casual.
    pub mode: GameMode,
    /// How many characters are in the army.
    pub roster: usize,
    /// Seconds played.
    pub playtime_s: u64,
    /// The save itself.
    pub save: SaveFile,
}

/// The title a chapter save of `campaign` shows: the chapter it goes on
/// with (the one after the chapter it cleared); `<title> (cleared)` when no
/// chapter follows yet; the chapter's id if the content no longer has it.
pub fn next_chapter_title(content: &Content, campaign: &Campaign) -> String {
    let Some(cleared) = content.chapters.get(&campaign.chapter) else {
        return campaign.chapter.clone();
    };
    let next = cleared
        .next
        .as_ref()
        .and_then(|id| content.chapters.get(id));
    match next {
        Some(next) => next.title.clone(),
        None => format!("{} (cleared)", cleared.title),
    }
}

/// What save slot `slot` holds. A slot holding a battle save (never
/// written there) is unreadable.
pub fn slot(storage: &dyn Storage, content: &Content, slot: usize) -> Slot {
    match read(storage, &slot_key(slot)) {
        Ok(None) => Slot::Empty,
        Ok(Some(save)) if save.point == SavePoint::ChapterCleared => {
            Slot::Saved(Box::new(SlotSummary {
                chapter: next_chapter_title(content, &save.campaign),
                mode: save.campaign.mode,
                roster: save.campaign.roster.len(),
                playtime_s: save.campaign.playtime_s,
                save,
            }))
        }
        Ok(Some(_)) => Slot::Unreadable(SaveError::Corrupt),
        Err(e) => Slot::Unreadable(e),
    }
}

/// Every save slot, slot 1 first.
pub fn slots(storage: &dyn Storage, content: &Content) -> Vec<Slot> {
    (1..=SLOTS).map(|n| slot(storage, content, n)).collect()
}

/// `seconds` as `h:mm:ss`.
pub fn playtime_text(seconds: u64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    format!("{h}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests;
