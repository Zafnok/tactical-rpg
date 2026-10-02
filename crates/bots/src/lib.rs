//! Playtest bots (ADR-0033, ticket 0505): players made of code, which play
//! a battle through `core`'s [`Command`](trpg_core::Command)s, and the
//! measures of how a try went. Dev tooling: used by `cargo xtask playtest`
//! and tests, never by the game.
//!
//! Pure like `core` (ADR-0004): no I/O and no clock. The caller loads the
//! battle, times the tries and writes the report. Bots know nothing about
//! the look of the game (ADR-0038).
//!
//! - [`PlayerBot`]: what plays the Player phase. [`BaselineBot`] is the
//!   enemy AI on the player's side; the Casual, Normal and Hardcore bots
//!   (0506) implement the same trait.
//! - [`play_battle`]: one try, from the state given to the battle's end or
//!   a turn cap. Bots never rewind: it builds no `BattleHistory`.
//! - [`BattleMeasures`]: what happened in the try.

mod bot;
mod driver;
mod measures;

pub use bot::{BaselineBot, PlayerBot};
pub use driver::{PlayError, play_battle};
pub use measures::BattleMeasures;

#[cfg(test)]
mod testkit;
