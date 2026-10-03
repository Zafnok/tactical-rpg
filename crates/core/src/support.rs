//! Supports (ticket 1002): pairs of characters who build points by
//! fighting together, gain ranks by talking at camp, and help each other
//! hit and dodge.
//!
//! # Rules
//!
//! Source: `docs/design/supports.md`. Every number is data
//! ([`SupportRules`], from `assets/data/supports.ron`).
//!
//! - **Pairs.** Only a pair listed in the [`SupportTable`] has a support.
//!   A pair is symmetric ([`SupportPair`]: `(a, b)` is `(b, a)`) and has one
//!   rank track.
//! - **Ranks** are C → B → A ([`SupportRank`]). Points are per pair and
//!   cumulative. Reaching a rank's threshold ([`Thresholds`]; a pair may
//!   override them) **unlocks** its conversation; the rank is **gained when
//!   the conversation is viewed** ([`SupportState::view`]), at camp.
//! - **Points stop at an unlocked, unviewed threshold**: extra points are
//!   lost, so a pair gains at most one rank per camp visit. At rank A a
//!   pair gains nothing more.
//! - **Starting points** (a pair's data) count like earned ones: a pair
//!   that starts at or past its C threshold starts with C unlocked, held at
//!   that threshold.
//! - **Battle bonus** ([`SupportRules::best_bonus`]): Hit and Avoid of the
//!   **single best-ranked** partner in range. Bonuses never combine, so
//!   the most a unit ever gets is the A bonus.
//! - **Death (Classic)**: a dead character's supports end
//!   ([`SupportBook::end_for`]). Their unlocked but unviewed conversations
//!   are removed, they gain no more points, and the ranks already viewed
//!   stay (the "seen" list, [`SupportState::seen`]).
//!
//! The battle gives the points ([`crate::battle`], *Supports*) and the
//! campaign carries the [`SupportBook`] from battle to battle
//! ([`crate::campaign`]).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::stats::StatValue;
use crate::unit::CharacterId;

/// A support rank, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SupportRank {
    /// The first rank.
    C,
    /// The second rank.
    B,
    /// The highest rank.
    A,
}

impl SupportRank {
    /// Every rank, lowest first.
    pub const ALL: [SupportRank; 3] = [SupportRank::C, SupportRank::B, SupportRank::A];

    /// The rank after `rank` (`None`: no rank yet), or `None` after A.
    pub fn after(rank: Option<SupportRank>) -> Option<SupportRank> {
        match rank {
            None => Some(SupportRank::C),
            Some(SupportRank::C) => Some(SupportRank::B),
            Some(SupportRank::B) => Some(SupportRank::A),
            Some(SupportRank::A) => None,
        }
    }
}

/// One value per rank, written in data as `(c: …, b: …, a: …)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ByRank<T> {
    /// Rank C's.
    pub c: T,
    /// Rank B's.
    pub b: T,
    /// Rank A's.
    pub a: T,
}

impl<T> ByRank<T> {
    /// The value of `rank`.
    pub fn get(&self, rank: SupportRank) -> &T {
        match rank {
            SupportRank::C => &self.c,
            SupportRank::B => &self.b,
            SupportRank::A => &self.a,
        }
    }
}

/// The points a pair needs for each rank.
pub type Thresholds = ByRank<u32>;

impl Thresholds {
    /// Whether each rank needs more points than the one before.
    pub fn is_increasing(&self) -> bool {
        self.c < self.b && self.b < self.a
    }
}

/// What a supported partner nearby adds in a combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportBonus {
    /// Added to hit.
    pub hit: StatValue,
    /// Added to avoid.
    pub avoid: StatValue,
}

/// The points each event gives a pair (`supports.md`, "Gaining points").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointValues {
    /// The two are adjacent when the player phase ends.
    pub adjacent_at_phase_end: u32,
    /// One fights (attacks or is attacked) with the other adjacent to it.
    pub fight_beside: u32,
    /// One heals the other with a spell or a skill.
    pub heal: u32,
    /// One buffs the other with a skill.
    pub buff: u32,
    /// One uses an item on the other.
    pub item: u32,
}

/// The support numbers, all tunable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportRules {
    /// Points needed for each rank, unless the pair has its own.
    pub thresholds: Thresholds,
    /// Points per event.
    pub points: PointValues,
    /// How near (Manhattan distance) a partner must be for its bonus.
    pub bonus_range: u32,
    /// The bonus of a partner of each rank. Only the best partner in range
    /// counts: bonuses never combine.
    pub bonus: ByRank<SupportBonus>,
}

impl SupportRules {
    /// The design's starting values (`supports.md`).
    pub const STARTING: SupportRules = SupportRules {
        thresholds: ByRank {
            c: 20,
            b: 80,
            a: 180,
        },
        points: PointValues {
            adjacent_at_phase_end: 1,
            fight_beside: 3,
            heal: 3,
            buff: 3,
            item: 2,
        },
        bonus_range: 3,
        bonus: ByRank {
            c: SupportBonus { hit: 5, avoid: 5 },
            b: SupportBonus { hit: 10, avoid: 10 },
            a: SupportBonus { hit: 15, avoid: 15 },
        },
    };

    /// The bonus a unit gets from partners of these `ranks` in range: the
    /// highest rank's alone, or nothing with no partner.
    pub fn best_bonus(&self, ranks: impl IntoIterator<Item = SupportRank>) -> SupportBonus {
        ranks
            .into_iter()
            .max()
            .map(|rank| *self.bonus.get(rank))
            .unwrap_or_default()
    }
}

/// Two characters, in either order: `new(a, b) == new(b, a)`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(from = "(CharacterId, CharacterId)")]
pub struct SupportPair(CharacterId, CharacterId);

impl SupportPair {
    /// The pair of `a` and `b`.
    pub fn new(a: CharacterId, b: CharacterId) -> Self {
        let mut members = [a, b];
        members.sort();
        let [first, second] = members;
        Self(first, second)
    }

    /// Its two characters, in id order.
    pub fn members(&self) -> [&CharacterId; 2] {
        [&self.0, &self.1]
    }

    /// Whether `character` is one of the two.
    pub fn has(&self, character: &CharacterId) -> bool {
        self.0 == *character || self.1 == *character
    }
}

impl From<(CharacterId, CharacterId)> for SupportPair {
    fn from((a, b): (CharacterId, CharacterId)) -> Self {
        Self::new(a, b)
    }
}

/// A pair that can build support, from the content data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairDef {
    /// The two characters.
    pub pair: SupportPair,
    /// Its own thresholds, instead of the rules'.
    pub thresholds: Option<Thresholds>,
    /// The points it starts the game with.
    pub starting_points: u32,
    /// The dialogue scene of each rank's conversation.
    pub conversations: ByRank<String>,
}

/// The support rules and every pair that has a support. Shared content:
/// not saved (ADR-0020).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SupportTable {
    /// The numbers.
    pub rules: SupportRules,
    pairs: BTreeMap<SupportPair, PairDef>,
}

impl SupportTable {
    /// A table of `pairs`. A character paired with itself is left out, and
    /// a pair listed twice keeps its last entry (`content` refuses both).
    pub fn new(rules: SupportRules, pairs: impl IntoIterator<Item = PairDef>) -> Self {
        let pairs = pairs.into_iter().filter(|d| d.pair.0 != d.pair.1);
        Self {
            rules,
            pairs: pairs.map(|d| (d.pair.clone(), d)).collect(),
        }
    }

    /// The pair's entry, if it has a support.
    pub fn get(&self, pair: &SupportPair) -> Option<&PairDef> {
        self.pairs.get(pair)
    }

    /// Every pair, in pair order.
    pub fn pairs(&self) -> impl Iterator<Item = &PairDef> {
        self.pairs.values()
    }

    /// The thresholds of `def`: its own, or the rules'.
    pub fn thresholds(&self, def: &PairDef) -> Thresholds {
        def.thresholds.unwrap_or(self.rules.thresholds)
    }
}

/// How far one pair's support has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SupportState {
    points: u32,
    rank: Option<SupportRank>,
    unlocked: Option<SupportRank>,
    ended: bool,
}

impl SupportState {
    /// A pair starting with `points`, under `thresholds`.
    pub fn start(points: u32, thresholds: &Thresholds) -> Self {
        let mut state = Self::default();
        state.gain(points, thresholds);
        state
    }

    /// Its points so far.
    pub fn points(&self) -> u32 {
        self.points
    }

    /// The rank gained (its conversation was viewed), if any.
    pub fn rank(&self) -> Option<SupportRank> {
        self.rank
    }

    /// The rank whose conversation is unlocked and waiting to be viewed.
    pub fn unlocked(&self) -> Option<SupportRank> {
        self.unlocked
    }

    /// Whether the support has ended (one of the two died).
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// The ranks whose conversations were viewed, lowest first.
    pub fn seen(&self) -> impl Iterator<Item = SupportRank> {
        let rank = self.rank;
        SupportRank::ALL
            .into_iter()
            .filter(move |r| Some(*r) <= rank)
    }

    /// Adds up to `amount` points, stopping at the next rank's threshold
    /// (which unlocks its conversation). Returns the points gained: 0 for
    /// an ended support, at rank A, or while a conversation waits.
    pub fn gain(&mut self, amount: u32, thresholds: &Thresholds) -> u32 {
        let Some(next) = SupportRank::after(self.rank).filter(|_| !self.ended) else {
            return 0;
        };
        let cap = *thresholds.get(next);
        let before = self.points;
        // Never down: a threshold lowered by a data change takes no points.
        self.points = before.saturating_add(amount).min(cap).max(before);
        if self.points >= cap {
            self.unlocked = Some(next);
        }
        self.points - before
    }

    /// Views the unlocked conversation: its rank is gained. Returns that
    /// rank, or `None` if nothing was unlocked.
    pub fn view(&mut self) -> Option<SupportRank> {
        let rank = self.unlocked.take()?;
        self.rank = Some(rank);
        Some(rank)
    }

    /// Ends the support: no more points, and an unlocked but unviewed
    /// conversation is removed.
    pub fn end(&mut self) {
        self.ended = true;
        self.unlocked = None;
    }
}

/// Why a support conversation can't be viewed. Nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupportError {
    /// The two characters have no support.
    NoSuchPair,
    /// This character isn't in the army.
    NotInArmy(CharacterId),
    /// The pair has no conversation unlocked and unviewed.
    NothingUnlocked,
}

impl std::fmt::Display for SupportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchPair => f.write_str("these two have no support"),
            Self::NotInArmy(c) => write!(f, "{} isn't in the army", c.0),
            Self::NothingUnlocked => f.write_str("no support conversation is unlocked"),
        }
    }
}

impl std::error::Error for SupportError {}

/// A support conversation that was just viewed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportViewed {
    /// The rank the pair gained.
    pub rank: SupportRank,
    /// The conversation's dialogue scene.
    pub conversation: String,
}

/// Every pair's support so far: saved with the campaign, and with a battle
/// while it runs. A listed pair without an entry is at its
/// [starting state](SupportBook::state).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SupportBook {
    pairs: BTreeMap<SupportPair, SupportState>,
}

impl SupportBook {
    /// The pair's support: as recorded, or its starting state; `None` if
    /// `table` doesn't list the pair.
    pub fn state(&self, pair: &SupportPair, table: &SupportTable) -> Option<SupportState> {
        let def = table.get(pair)?;
        let start = || SupportState::start(def.starting_points, &table.thresholds(def));
        Some(self.pairs.get(pair).copied().unwrap_or_else(start))
    }

    /// The rank the pair has gained, if any.
    pub fn rank(&self, pair: &SupportPair) -> Option<SupportRank> {
        self.pairs.get(pair).and_then(SupportState::rank)
    }

    /// Gives the pair up to `amount` points ([`SupportState::gain`]).
    /// Returns the points gained: 0 for a pair `table` doesn't list.
    pub fn gain(&mut self, pair: &SupportPair, amount: u32, table: &SupportTable) -> u32 {
        self.change(pair, table, |state, thresholds| {
            state.gain(amount, thresholds)
        })
        .unwrap_or(0)
    }

    /// Views the pair's unlocked conversation ([`SupportState::view`]).
    pub fn view(
        &mut self,
        pair: &SupportPair,
        table: &SupportTable,
    ) -> Result<SupportViewed, SupportError> {
        let def = table.get(pair).ok_or(SupportError::NoSuchPair)?;
        let rank = self
            .change(pair, table, |state, _| state.view())
            .flatten()
            .ok_or(SupportError::NothingUnlocked)?;
        Ok(SupportViewed {
            rank,
            conversation: def.conversations.get(rank).clone(),
        })
    }

    /// Ends every support of `character` ([`SupportState::end`]).
    pub fn end_for(&mut self, character: &CharacterId, table: &SupportTable) {
        for def in table.pairs().filter(|d| d.pair.has(character)) {
            self.change(&def.pair, table, |state, _| state.end());
        }
    }

    /// Every conversation unlocked and waiting to be viewed, in pair order.
    pub fn unlocked(&self, table: &SupportTable) -> Vec<(SupportPair, SupportRank)> {
        table
            .pairs()
            .filter_map(|d| {
                let rank = self.state(&d.pair, table)?.unlocked()?;
                Some((d.pair.clone(), rank))
            })
            .collect()
    }

    /// Changes a listed pair's state (from its starting state if it has no
    /// entry yet) and records it. `None` for a pair `table` doesn't list.
    fn change<T>(
        &mut self,
        pair: &SupportPair,
        table: &SupportTable,
        f: impl FnOnce(&mut SupportState, &Thresholds) -> T,
    ) -> Option<T> {
        let def = table.get(pair)?;
        let mut state = self.state(pair, table)?;
        let out = f(&mut state, &table.thresholds(def));
        self.pairs.insert(pair.clone(), state);
        Some(out)
    }
}

#[cfg(test)]
mod tests;
