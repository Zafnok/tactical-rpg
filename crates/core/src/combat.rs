//! Combat maths: the forecast and the resolution of one combat.
//!
//! Sources: `docs/design/stats-and-combat.md` (formulas, 2RN hit rolls,
//! strike counts and order), `docs/design/weapons-and-items.md` (weapon-type
//! traits, effectiveness, attack speed, broken weapons; no weapon triangle)
//! and `docs/design/magic.md` (attack spells, elemental affinities).
//!
//! # Rules
//!
//! For a side `A` striking `B` (both sides use the same formulas):
//!
//! ```text
//! eff_mult   = largest multiplier among A's effective tags that B has, and
//!              the Weak affinity multiplier (magic only); else 1
//! might      = broken ? weapon.might / 2 : weapon.might
//! power      = (Physical ? A.Str : A.Mag) + might * eff_mult
//! mitigation = (Physical ? B.Def : B.Res) + terrain_B.defense
//! damage     = max(0, power - mitigation)
//!              → Resist: / 2   → Axe: max(damage, 5)
//! follow-up  = Sword ? damage * 6 / 5 : damage          (strikes 2..N)
//! crit dmg   = that strike's damage * 3
//! avoid_B    = B.Spd * 2 + terrain_B.avoid + (B wields a gauntlet ? 15 : 0)
//! hit        = clamp(weapon.hit - (broken ? 20 : 0) + A.Dex * 2 - avoid_B, 0, 100)
//! crit       = clamp(weapon.crit + A.Dex / 2 - B.Dex / 4, 0, 100)   (Absorb: 0)
//! burden     = max(0, weapon.weight + armour.weight - A.Str / 5)
//! AS         = A.Spd + rank_speed[rank] - burden
//! strikes    = 1 + number of thresholds [4, 14, 24] that AS_A - AS_B reaches
//! ```
//!
//! A flying `B` gets no terrain Def or Avoid. Physical attacks ignore
//! affinities. An Absorb hit heals `B` by `damage` (never above max HP). All
//! the numbers (`2`, `5`, `6/5`, `15`, the thresholds…) are fields of
//! [`CombatRules`]; its [`Default`] holds the design values.
//!
//! Strike order: attacker, defender (if it can counter), then the faster
//! side's strikes 2..N. Each strike rolls `r1`, `r2` and hits iff
//! `r1 + r2 < 2 * hit`; only a hit rolls `r3` and crits iff `r3 < crit`.
//! Combat stops as soon as a unit reaches 0 HP.

use serde::{Deserialize, Serialize};

use crate::class::{UnitTag, UnitTags};
use crate::magic::{Affinity, Element};
use crate::rng::RandomSource;
use crate::stats::{StatValue, Stats};
use crate::terrain::TerrainRules;
use crate::weapon::{WeaponKind, WeaponRank};

/// Whether an attack uses Str against Def or Mag against Res.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageType {
    /// Str + might against Def.
    Physical,
    /// Mag + might against Res.
    Magical,
}

/// A weapon type's special rule (`weapons-and-items.md`). Build one with
/// [`CombatRules::type_trait`] so its numbers come from the rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponTrait {
    /// No trait: spells and typeless test weapons.
    None,
    /// Strikes 2..N deal `damage * 6 / 5` ([`CombatRules::sword_followup`]).
    SwordFollowUp,
    /// A hit deals at least this much damage.
    AxeMinDamage(StatValue),
    /// The wielder's avoid rises by this much while it is equipped.
    GauntletAvoid(StatValue),
    /// Might is multiplied against targets with this tag.
    Effective(UnitTag, u8),
}

/// The combat numbers of an equipped weapon or attack spell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponStats {
    /// The weapon kind; `None` for spells (no rank, no rank speed).
    pub kind: Option<WeaponKind>,
    /// The type trait.
    pub trait_: WeaponTrait,
    /// Might.
    pub might: StatValue,
    /// Base hit.
    pub hit: StatValue,
    /// Base crit.
    pub crit: StatValue,
    /// Weight; feeds the burden. 0 for spells.
    pub weight: StatValue,
    /// Smallest range, in tiles (Manhattan).
    pub min_range: u32,
    /// Largest range, in tiles (Manhattan).
    pub max_range: u32,
    /// Str vs Def or Mag vs Res.
    pub damage_type: DamageType,
    /// Extra `(tag, multiplier)` pairs beyond the type trait.
    pub effective: Vec<(UnitTag, u8)>,
    /// At 0 durability: might halved, −20 hit. Never true for spells.
    pub broken: bool,
    /// The spell's element; `None` for weapons.
    pub element: Element,
}

impl WeaponStats {
    /// Whether a unit `distance` tiles away is in range.
    pub fn in_range(&self, distance: u32) -> bool {
        (self.min_range..=self.max_range).contains(&distance)
    }
}

/// Every combat constant. [`Default`] is the design's values; balance tickets
/// change them here (or load them as data), never in the formulas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatRules {
    /// Speed gaps that give strikes 2, 3, 4… (`[4, 14, 24]`).
    pub strike_thresholds: Vec<StatValue>,
    /// Attack speed bonus per weapon rank, E to S.
    pub rank_speed: [StatValue; 6],
    /// Str that carries 1 weight (5).
    pub str_per_weight: StatValue,
    /// Hit gained per Dex (2).
    pub hit_per_dex: StatValue,
    /// Avoid gained per Spd (2).
    pub avoid_per_spd: StatValue,
    /// Attacker's Dex is divided by this for crit (2).
    pub crit_dex_divisor: StatValue,
    /// Defender's Dex is divided by this for crit avoid (4).
    pub crit_avoid_dex_divisor: StatValue,
    /// Crit damage multiplier (3).
    pub crit_multiplier: StatValue,
    /// Sword follow-up damage ratio `(numerator, denominator)`, (6, 5).
    pub sword_followup: (StatValue, StatValue),
    /// Axe minimum damage on a hit (5).
    pub axe_min_damage: StatValue,
    /// Gauntlet avoid bonus (15).
    pub gauntlet_avoid: StatValue,
    /// Spear effectiveness (Mounted, ×2).
    pub spear_effective: (UnitTag, u8),
    /// Bow effectiveness (Flying, ×3).
    pub bow_effective: (UnitTag, u8),
    /// Broken weapon might divisor (2).
    pub broken_might_divisor: StatValue,
    /// Broken weapon hit penalty (20).
    pub broken_hit_penalty: StatValue,
    /// Weak affinity might multiplier (3).
    pub weak_multiplier: u8,
    /// Resist affinity damage divisor (2).
    pub resist_divisor: StatValue,
}

impl Default for CombatRules {
    fn default() -> Self {
        Self {
            strike_thresholds: vec![4, 14, 24],
            rank_speed: [0, 0, 1, 2, 3, 4],
            str_per_weight: 5,
            hit_per_dex: 2,
            avoid_per_spd: 2,
            crit_dex_divisor: 2,
            crit_avoid_dex_divisor: 4,
            crit_multiplier: 3,
            sword_followup: (6, 5),
            axe_min_damage: 5,
            gauntlet_avoid: 15,
            spear_effective: (UnitTag::Mounted, 2),
            bow_effective: (UnitTag::Flying, 3),
            broken_might_divisor: 2,
            broken_hit_penalty: 20,
            weak_multiplier: 3,
            resist_divisor: 2,
        }
    }
}

impl CombatRules {
    /// The trait every weapon of `kind` has.
    pub fn type_trait(&self, kind: WeaponKind) -> WeaponTrait {
        match kind {
            WeaponKind::Sword => WeaponTrait::SwordFollowUp,
            WeaponKind::Spear => {
                WeaponTrait::Effective(self.spear_effective.0, self.spear_effective.1)
            }
            WeaponKind::Axe => WeaponTrait::AxeMinDamage(self.axe_min_damage),
            WeaponKind::Bow => WeaponTrait::Effective(self.bow_effective.0, self.bow_effective.1),
            WeaponKind::Gauntlet => WeaponTrait::GauntletAvoid(self.gauntlet_avoid),
        }
    }

    /// The attack speed bonus of `rank`.
    pub fn rank_speed(&self, rank: WeaponRank) -> StatValue {
        self.rank_speed[rank as usize]
    }

    /// The largest number of strikes one side can get.
    pub fn max_strikes(&self) -> u8 {
        u8::try_from(self.strike_thresholds.len() + 1).unwrap_or(u8::MAX)
    }

    /// Strikes for a side whose attack speed is `diff` above the other's.
    pub fn strikes(&self, diff: StatValue) -> u8 {
        let reached = self
            .strike_thresholds
            .iter()
            .filter(|&&t| diff >= t)
            .count();
        u8::try_from(reached + 1).unwrap_or(u8::MAX)
    }
}

/// One side of a combat, as the combat maths sees it. Built by the battle
/// state from a unit (tickets 0305/0306/0309).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatantInput<'a> {
    /// Gear-adjusted stats (`stats.hp` is max HP).
    pub stats: Stats,
    /// Class tags, for effectiveness and the flier terrain rule.
    pub tags: UnitTags,
    /// Elemental affinities.
    pub affinities: Vec<(Element, Affinity)>,
    /// The equipped weapon or attack spell; `None` = can't attack or counter.
    pub weapon: Option<WeaponStats>,
    /// Rank in the equipped weapon's kind (ignored for spells).
    pub weapon_rank: WeaponRank,
    /// Weight of the equipped armour (0 without armour).
    pub armour_weight: StatValue,
    /// The terrain the unit stands on.
    pub terrain: &'a TerrainRules,
}

impl CombatantInput<'_> {
    /// Attack speed: `Spd + rank_speed - burden`.
    pub fn attack_speed(&self, rules: &CombatRules) -> StatValue {
        let (weight, rank_bonus) = match &self.weapon {
            Some(w) => (
                w.weight,
                w.kind.map_or(0, |_| rules.rank_speed(self.weapon_rank)),
            ),
            None => (0, 0),
        };
        let carried = self.stats.str / rules.str_per_weight;
        let burden = (weight + self.armour_weight - carried).max(0);
        self.stats.spd + rank_bonus - burden
    }

    /// Terrain Def (0 for fliers).
    fn terrain_defense(&self) -> StatValue {
        if self.tags.flying {
            0
        } else {
            StatValue::from(self.terrain.defense)
        }
    }

    /// Avoid against any attack.
    fn avoid(&self, rules: &CombatRules) -> StatValue {
        let terrain = if self.tags.flying {
            0
        } else {
            StatValue::from(self.terrain.avoid)
        };
        let gauntlet = match self.weapon.as_ref().map(|w| w.trait_) {
            Some(WeaponTrait::GauntletAvoid(v)) => v,
            _ => 0,
        };
        self.stats.spd * rules.avoid_per_spd + terrain + gauntlet
    }
}

/// One side's numbers in the forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SideForecast {
    /// First-strike damage (for Absorb: HP healed per hit).
    pub damage: StatValue,
    /// Damage of strikes 2..N (differs from `damage` only for swords).
    pub followup_damage: StatValue,
    /// Displayed hit, `0..=100`.
    pub hit: u8,
    /// Displayed crit, `0..=100`.
    pub crit: u8,
    /// Number of strikes, `1..=max_strikes`.
    pub strikes: u8,
    /// Whether an effectiveness multiplier (tag or Weak) applies.
    pub effective: bool,
    /// Whether the weapon is broken.
    pub broken: bool,
    /// The target's affinity to this attack's element, if any applies.
    pub affinity: Option<Affinity>,
}

/// The combat forecast. The defender side is `None` when it can't counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Forecast {
    /// The attacker's numbers.
    pub attacker: SideForecast,
    /// The defender's numbers, if it counters.
    pub defender: Option<SideForecast>,
}

/// The forecast of `attacker` attacking `defender` from `distance` tiles, or
/// `None` if the attacker has no weapon or the defender is out of its range.
pub fn forecast(
    rules: &CombatRules,
    attacker: &CombatantInput,
    defender: &CombatantInput,
    distance: u32,
) -> Option<Forecast> {
    let can_strike = |c: &CombatantInput| c.weapon.as_ref().is_some_and(|w| w.in_range(distance));
    if !can_strike(attacker) {
        return None;
    }
    let diff = attacker.attack_speed(rules) - defender.attack_speed(rules);
    let attacker_side = side(rules, attacker, defender, diff)?;
    let defender_side = if can_strike(defender) {
        side(rules, defender, attacker, -diff)
    } else {
        None
    };
    Some(Forecast {
        attacker: attacker_side,
        defender: defender_side,
    })
}

/// `a`'s numbers striking `b`; `diff` is `AS_a - AS_b`.
fn side(
    rules: &CombatRules,
    a: &CombatantInput,
    b: &CombatantInput,
    diff: StatValue,
) -> Option<SideForecast> {
    let weapon = a.weapon.as_ref()?;
    let affinity = match weapon.damage_type {
        DamageType::Magical if weapon.element != Element::None => b
            .affinities
            .iter()
            .find(|(e, _)| *e == weapon.element)
            .map(|&(_, aff)| aff),
        _ => None,
    };

    let tag_mult = weapon
        .effective
        .iter()
        .copied()
        .chain(match weapon.trait_ {
            WeaponTrait::Effective(tag, mult) => Some((tag, mult)),
            _ => None,
        })
        .filter(|&(tag, _)| b.tags.has(tag))
        .map(|(_, mult)| mult)
        .max();
    let weak_mult = (affinity == Some(Affinity::Weak)).then_some(rules.weak_multiplier);
    let eff_mult = tag_mult.max(weak_mult).unwrap_or(1).max(1);

    let might = if weapon.broken {
        weapon.might / rules.broken_might_divisor
    } else {
        weapon.might
    };
    let (power_stat, mitigation_stat) = match weapon.damage_type {
        DamageType::Physical => (a.stats.str, b.stats.def),
        DamageType::Magical => (a.stats.mag, b.stats.res),
    };
    let power = power_stat + might * StatValue::from(eff_mult);
    let mitigation = mitigation_stat + b.terrain_defense();
    let mut damage = (power - mitigation).max(0);
    if affinity == Some(Affinity::Resist) {
        damage /= rules.resist_divisor;
    }
    if let WeaponTrait::AxeMinDamage(min) = weapon.trait_ {
        damage = damage.max(min);
    }
    let followup_damage = if weapon.trait_ == WeaponTrait::SwordFollowUp {
        let (num, den) = rules.sword_followup;
        damage * num / den
    } else {
        damage
    };

    let broken_penalty = if weapon.broken {
        rules.broken_hit_penalty
    } else {
        0
    };
    let hit = weapon.hit - broken_penalty + a.stats.dex * rules.hit_per_dex - b.avoid(rules);
    let crit = if affinity == Some(Affinity::Absorb) {
        0
    } else {
        weapon.crit + a.stats.dex / rules.crit_dex_divisor
            - b.stats.dex / rules.crit_avoid_dex_divisor
    };

    Some(SideForecast {
        damage,
        followup_damage,
        hit: percent(hit),
        crit: percent(crit),
        strikes: rules.strikes(diff),
        effective: eff_mult > 1,
        broken: weapon.broken,
        affinity,
    })
}

fn percent(value: StatValue) -> u8 {
    u8::try_from(value.clamp(0, 100)).unwrap_or(0)
}

/// Which side of the combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    /// The unit that started the combat.
    Attacker,
    /// The unit that was attacked.
    Defender,
}

/// A unit's current and max HP going into combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CombatHp {
    /// Current HP.
    pub current: StatValue,
    /// Max HP (caps Absorb healing).
    pub max: StatValue,
}

/// One strike of a resolved combat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Strike {
    /// Who struck.
    pub by: Side,
    /// Whether it hit.
    pub hit: bool,
    /// Whether it crit (only on a hit).
    pub crit: bool,
    /// The strike's damage (crit included) if it hit, else 0. For an
    /// Absorb strike, the healing before the max-HP cap.
    pub damage: StatValue,
    /// Whether it healed the target (Absorb) instead of hurting it.
    pub healed: bool,
    /// The target's HP after the strike.
    pub target_hp_after: StatValue,
}

/// The result of a resolved combat.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CombatOutcome {
    /// Every strike, in order.
    pub strikes: Vec<Strike>,
    /// The attacker's HP at the end.
    pub attacker_hp: StatValue,
    /// The defender's HP at the end.
    pub defender_hp: StatValue,
}

/// The 2RN ("true hit") roll: two rolls, a hit iff `r1 + r2 < 2 * hit`.
/// Always takes exactly two rolls.
pub fn roll_hit(rng: &mut impl RandomSource, hit: u8) -> bool {
    let r1 = u32::from(rng.roll_percent());
    let r2 = u32::from(rng.roll_percent());
    r1 + r2 < 2 * u32::from(hit)
}

/// Plays out `forecast` with `rng`: strikes in order (attacker, defender,
/// then the faster side's strikes 2..N), stopping when a unit reaches 0 HP.
pub fn resolve(
    rules: &CombatRules,
    forecast: &Forecast,
    attacker: CombatHp,
    defender: CombatHp,
    rng: &mut impl RandomSource,
) -> CombatOutcome {
    let mut order = vec![Side::Attacker];
    if forecast.defender.is_some() {
        order.push(Side::Defender);
    }
    let extra = |s: Option<SideForecast>| s.map_or(0, |s| s.strikes.saturating_sub(1));
    let (faster, extra_strikes) = if extra(forecast.defender) > 0 {
        (Side::Defender, extra(forecast.defender))
    } else {
        (Side::Attacker, extra(Some(forecast.attacker)))
    };
    order.extend(std::iter::repeat_n(faster, usize::from(extra_strikes)));

    let mut hp = [attacker, defender];
    let mut struck = [0u8; 2];
    let mut strikes = Vec::with_capacity(order.len());
    for by in order {
        if hp[0].current <= 0 || hp[1].current <= 0 {
            break;
        }
        let (me, target, numbers) = match by {
            Side::Attacker => (0, 1, forecast.attacker),
            Side::Defender => match forecast.defender {
                Some(d) => (1, 0, d),
                None => break,
            },
        };
        struck[me] += 1;
        let base = if struck[me] > 1 {
            numbers.followup_damage
        } else {
            numbers.damage
        };
        let healed = numbers.affinity == Some(Affinity::Absorb);
        let hit = roll_hit(rng, numbers.hit);
        let crit = hit && rng.roll_percent() < numbers.crit;
        let damage = match (hit, crit) {
            (false, _) => 0,
            (true, false) => base,
            (true, true) => base * rules.crit_multiplier,
        };
        let t = &mut hp[target];
        t.current = if healed {
            (t.current + damage).min(t.max)
        } else {
            (t.current - damage).max(0)
        };
        strikes.push(Strike {
            by,
            hit,
            crit,
            damage,
            healed,
            target_hp_after: t.current,
        });
    }
    CombatOutcome {
        strikes,
        attacker_hp: hp[0].current,
        defender_hp: hp[1].current,
    }
}

#[cfg(test)]
mod tests;
