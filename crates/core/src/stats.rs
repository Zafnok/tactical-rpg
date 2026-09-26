//! The stat list from `docs/design/stats-and-combat.md` and class growth
//! rates from `docs/design/progression.md`.

use serde::{Deserialize, Serialize};

/// The integer type of every stat, HP and damage value. The number scale is
/// undecided (ticket 0013) and may grow to huge values; changing this alias
/// must be the only edit needed.
pub type StatValue = i32;

/// A growth rate in percent. Above 100 gives more than +1 per level up.
pub type GrowthValue = u16;

/// One of the unit stats. Order is the fixed stat order of the design docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatKind {
    /// Max hit points.
    Hp,
    /// Physical attack power.
    Str,
    /// Magical attack power.
    Mag,
    /// Precision: hit and crit.
    Dex,
    /// Quickness: avoid and extra strikes.
    Spd,
    /// Reduces physical damage.
    Def,
    /// Reduces magical damage.
    Res,
    /// Movement points; set by the class, never grows.
    Mov,
}

impl StatKind {
    /// Every stat, in stat order.
    pub const ALL: [StatKind; 8] = [
        StatKind::Hp,
        StatKind::Str,
        StatKind::Mag,
        StatKind::Dex,
        StatKind::Spd,
        StatKind::Def,
        StatKind::Res,
        StatKind::Mov,
    ];

    /// The stats that grow on level up (all but Mov), in stat order.
    pub const GROWABLE: [StatKind; 7] = [
        StatKind::Hp,
        StatKind::Str,
        StatKind::Mag,
        StatKind::Dex,
        StatKind::Spd,
        StatKind::Def,
        StatKind::Res,
    ];
}

/// One value per stat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct Stats {
    /// Max HP.
    pub hp: StatValue,
    /// Strength.
    pub str: StatValue,
    /// Magic.
    pub mag: StatValue,
    /// Dexterity.
    pub dex: StatValue,
    /// Speed.
    pub spd: StatValue,
    /// Defence.
    pub def: StatValue,
    /// Resistance.
    pub res: StatValue,
    /// Movement points.
    pub mov: StatValue,
}

impl Stats {
    /// Builds stats from the seven growable values (HP, Str, Mag, Dex, Spd,
    /// Def, Res) and Mov.
    pub fn from_growable(values: [StatValue; 7], mov: StatValue) -> Self {
        let [hp, str, mag, dex, spd, def, res] = values;
        Self {
            hp,
            str,
            mag,
            dex,
            spd,
            def,
            res,
            mov,
        }
    }

    /// The value of `kind`.
    pub fn get(&self, kind: StatKind) -> StatValue {
        match kind {
            StatKind::Hp => self.hp,
            StatKind::Str => self.str,
            StatKind::Mag => self.mag,
            StatKind::Dex => self.dex,
            StatKind::Spd => self.spd,
            StatKind::Def => self.def,
            StatKind::Res => self.res,
            StatKind::Mov => self.mov,
        }
    }

    /// Sets the value of `kind`.
    pub fn set(&mut self, kind: StatKind, value: StatValue) {
        *self.field(kind) = value;
    }

    fn field(&mut self, kind: StatKind) -> &mut StatValue {
        match kind {
            StatKind::Hp => &mut self.hp,
            StatKind::Str => &mut self.str,
            StatKind::Mag => &mut self.mag,
            StatKind::Dex => &mut self.dex,
            StatKind::Spd => &mut self.spd,
            StatKind::Def => &mut self.def,
            StatKind::Res => &mut self.res,
            StatKind::Mov => &mut self.mov,
        }
    }
}

/// Growth rates in percent for the growable stats. Mov never grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Growths(pub [GrowthValue; 7]);

impl Growths {
    /// The growth rate of `kind`; always 0 for Mov.
    pub fn get(&self, kind: StatKind) -> GrowthValue {
        StatKind::GROWABLE
            .iter()
            .position(|&k| k == kind)
            .map_or(0, |i| self.0[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Stats {
        Stats::from_growable([1, 2, 3, 4, 5, 6, 7], 8)
    }

    #[test]
    fn from_growable_keeps_stat_order() {
        let s = sample();
        assert_eq!(
            (s.hp, s.str, s.mag, s.dex, s.spd, s.def, s.res, s.mov),
            (1, 2, 3, 4, 5, 6, 7, 8)
        );
    }

    #[test]
    fn get_reads_each_stat() {
        let s = sample();
        let values: Vec<StatValue> = StatKind::ALL.iter().map(|&k| s.get(k)).collect();
        assert_eq!(values, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn set_writes_only_that_stat() {
        for (i, &kind) in StatKind::ALL.iter().enumerate() {
            let mut s = sample();
            s.set(kind, 100);
            for (j, &other) in StatKind::ALL.iter().enumerate() {
                let expected = if i == j { 100 } else { sample().get(other) };
                assert_eq!(s.get(other), expected, "{kind:?} / {other:?}");
            }
        }
    }

    #[test]
    fn growable_is_all_but_mov() {
        assert_eq!(StatKind::GROWABLE[..], StatKind::ALL[..7]);
        assert_eq!(StatKind::ALL[7], StatKind::Mov);
    }

    #[test]
    fn growths_by_kind() {
        let g = Growths([70, 40, 10, 55, 60, 25, 20]);
        let values: Vec<GrowthValue> = StatKind::ALL.iter().map(|&k| g.get(k)).collect();
        assert_eq!(values, [70, 40, 10, 55, 60, 25, 20, 0]);
    }
}
