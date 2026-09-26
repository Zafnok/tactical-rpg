//! Units on the battle map, the named-character data they are made from, and
//! factions (`docs/design/progression.md`).

use std::collections::BTreeMap;
use std::fmt;

use crate::class::{ClassDef, ClassId, ClassLevel, ClassPoints, ClassTable};
use crate::geom::Pos;
use crate::magic::SpellId;
use crate::stats::{StatKind, StatValue, Stats};
use crate::weapon::{WeaponKind, WeaponRank};

/// A character level. Never resets; the cap is data and may become huge.
pub type Level = u32;

/// Which side a unit fights for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Faction {
    /// The player's army.
    Player,
    /// Enemies of the player and allies.
    Enemy,
    /// Friendly, AI-controlled (green) units.
    Ally,
    /// Hostile to nobody.
    Neutral,
}

impl Faction {
    /// Whether units of `self` and `other` fight each other. Player and Ally
    /// are friends; Enemy is hostile to both; Neutral is hostile to nobody.
    pub fn is_hostile_to(self, other: Faction) -> bool {
        use Faction::{Ally, Enemy, Player};
        matches!(
            (self, other),
            (Enemy, Player | Ally) | (Player | Ally, Enemy)
        )
    }
}

/// Identifies a unit within a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(pub u32);

/// String id of a named character, e.g. `"test_lord"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterId(pub String);

/// A unit's progress in one class it has unlocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassRecord {
    /// Class level, from 1.
    pub class_level: ClassLevel,
    /// Class points earned in this class.
    pub class_points: ClassPoints,
}

impl ClassRecord {
    /// The record of a class just unlocked: class level 1, no CP.
    pub const UNLOCKED: ClassRecord = ClassRecord {
        class_level: 1,
        class_points: 0,
    };
}

/// A named character's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDef {
    /// String id.
    pub id: CharacterId,
    /// Display name.
    pub name: String,
    /// Starting class.
    pub class: ClassId,
    /// Starting character level.
    pub level: Level,
    /// Whether this is the lord (game over on death; lord-only classes).
    pub is_lord: bool,
    /// The personal +20% growth stat (never Mov).
    pub talent: StatKind,
    /// Starting stats (≤ the class caps). Mov comes from the class.
    pub base: Stats,
    /// Starting weapon ranks.
    pub weapon_ranks: BTreeMap<WeaponKind, WeaponRank>,
    /// Personal spells learned at a character level (0–2).
    pub personal_spells: Vec<(Level, SpellId)>,
}

/// A unit on the battle map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Battle-unique id.
    pub id: UnitId,
    /// The named character, or `None` for a generic unit.
    pub character: Option<CharacterId>,
    /// Display name.
    pub name: String,
    /// Current class.
    pub class: ClassId,
    /// Character level (never resets).
    pub level: Level,
    /// EXP towards the next character level.
    pub exp: u32,
    /// Progress in every unlocked class.
    pub class_records: BTreeMap<ClassId, ClassRecord>,
    /// Current permanent stats. ≤ the class caps, except stats kept above a
    /// new class's caps after a reclass.
    pub stats: Stats,
    /// Current HP, `0..=stats.hp`.
    pub hp: StatValue,
    /// Side.
    pub faction: Faction,
    /// Map position.
    pub pos: Pos,
    /// Whether the unit has acted this phase.
    pub acted: bool,
    /// Whether this is the lord.
    pub is_lord: bool,
    /// Weapon rank per kind (kept for kinds the current class can't use).
    pub weapon_ranks: BTreeMap<WeaponKind, WeaponRank>,
}

/// Why a unit could not be created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitError {
    /// The class id is not in the class table.
    UnknownClass(ClassId),
    /// A unit that isn't the lord was put in a lord-only class.
    LordOnlyClass(ClassId),
}

impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitError::UnknownClass(c) => write!(f, "unknown class \"{}\"", c.0),
            UnitError::LordOnlyClass(c) => {
                write!(f, "class \"{}\" is only for the lord", c.0)
            }
        }
    }
}

impl std::error::Error for UnitError {}

impl Unit {
    /// Creates the unit of named character `def` at its data level, in its
    /// starting class. Stats are clamped to `0..=cap`; Mov comes from the
    /// class; weapon ranks are raised to the class's start ranks.
    pub fn from_character(
        id: UnitId,
        def: &CharacterDef,
        classes: &ClassTable,
        faction: Faction,
        pos: Pos,
    ) -> Result<Unit, UnitError> {
        let class = lookup(classes, &def.class, def.is_lord)?;
        let mut stats = def.base;
        stats.mov = class.move_points;
        for kind in StatKind::GROWABLE {
            stats.set(kind, stats.get(kind).min(class.caps.get(kind)).max(0));
        }
        let mut weapon_ranks = def.weapon_ranks.clone();
        raise_to_start_ranks(&mut weapon_ranks, class);
        Ok(Unit {
            id,
            character: Some(def.id.clone()),
            name: def.name.clone(),
            level: def.level,
            is_lord: def.is_lord,
            ..Self::fresh(id, class, stats, faction, pos, weapon_ranks)
        })
    }

    /// Creates a generic unit of class `class` at character level `level`,
    /// with the fixed average stats of `progression.md`:
    /// `stat = min(cap, base + growth × (level − 1) / 100)`.
    pub fn generic(
        id: UnitId,
        class: &ClassId,
        classes: &ClassTable,
        level: Level,
        faction: Faction,
        pos: Pos,
    ) -> Result<Unit, UnitError> {
        let class = lookup(classes, class, false)?;
        let levels = StatValue::try_from(level.saturating_sub(1)).unwrap_or(StatValue::MAX);
        let mut stats = class.base;
        stats.mov = class.move_points;
        for kind in StatKind::GROWABLE {
            let gain = StatValue::from(class.growths.get(kind)).saturating_mul(levels) / 100;
            let value = class.base.get(kind).saturating_add(gain);
            stats.set(kind, value.min(class.caps.get(kind)));
        }
        let mut weapon_ranks = BTreeMap::new();
        raise_to_start_ranks(&mut weapon_ranks, class);
        Ok(Unit {
            level,
            ..Self::fresh(id, class, stats, faction, pos, weapon_ranks)
        })
    }

    /// A generic, level-1 unit of `class` with the given stats.
    fn fresh(
        id: UnitId,
        class: &ClassDef,
        stats: Stats,
        faction: Faction,
        pos: Pos,
        weapon_ranks: BTreeMap<WeaponKind, WeaponRank>,
    ) -> Unit {
        Unit {
            id,
            character: None,
            name: class.name.clone(),
            class: class.id.clone(),
            level: 1,
            exp: 0,
            class_records: BTreeMap::from([(class.id.clone(), ClassRecord::UNLOCKED)]),
            stats,
            hp: stats.hp,
            faction,
            pos,
            acted: false,
            is_lord: false,
            weapon_ranks,
        }
    }
}

fn lookup<'a>(
    classes: &'a ClassTable,
    id: &ClassId,
    is_lord: bool,
) -> Result<&'a ClassDef, UnitError> {
    let class = classes
        .get(id)
        .ok_or_else(|| UnitError::UnknownClass(id.clone()))?;
    if class.lord_only && !is_lord {
        return Err(UnitError::LordOnlyClass(id.clone()));
    }
    Ok(class)
}

/// Raises each rank in a kind `class` can use to at least its start rank.
fn raise_to_start_ranks(ranks: &mut BTreeMap<WeaponKind, WeaponRank>, class: &ClassDef) {
    for w in &class.weapons {
        let rank = ranks.entry(w.kind).or_insert(w.start);
        *rank = (*rank).max(w.start);
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::class::{UnitTags, WeaponProficiency};
    use crate::stats::Growths;
    use crate::terrain::MovementTypeId;

    fn class(id: &str) -> ClassDef {
        ClassDef {
            id: ClassId(id.into()),
            name: "Brigand".into(),
            tier: 1,
            movement_type: MovementTypeId(0),
            move_points: 5,
            base: Stats::from_growable([20, 6, 0, 2, 4, 3, 0], 5),
            caps: Stats::from_growable([45, 24, 8, 16, 18, 18, 10], 5),
            growths: Growths([80, 50, 0, 30, 30, 25, 5]),
            weapons: vec![
                WeaponProficiency {
                    kind: WeaponKind::Axe,
                    start: WeaponRank::D,
                    max: WeaponRank::C,
                },
                WeaponProficiency {
                    kind: WeaponKind::Sword,
                    start: WeaponRank::E,
                    max: WeaponRank::C,
                },
            ],
            armour: vec![],
            tags: UnitTags::default(),
            promotes_to: vec![],
            active: None,
            passives: vec![],
            enemy_only: false,
            lord_only: false,
            weapon_slots: 3,
            spells: vec![],
            affinities: vec![],
        }
    }

    fn table(classes: Vec<ClassDef>) -> ClassTable {
        ClassTable {
            classes: classes.into_iter().map(|c| (c.id.clone(), c)).collect(),
            ..ClassTable::default()
        }
    }

    fn character(class: &str) -> CharacterDef {
        CharacterDef {
            id: CharacterId("hero".into()),
            name: "Hero".into(),
            class: ClassId(class.into()),
            level: 3,
            is_lord: false,
            talent: StatKind::Spd,
            base: Stats::from_growable([22, 7, 1, 5, 6, 4, 2], 0),
            weapon_ranks: BTreeMap::from([
                (WeaponKind::Axe, WeaponRank::C),
                (WeaponKind::Bow, WeaponRank::B),
            ]),
            personal_spells: vec![],
        }
    }

    const POS: Pos = Pos::new(3, 4);

    #[test]
    fn hostility_truth_table() {
        use Faction::{Ally, Enemy, Neutral, Player};
        let all = [Player, Enemy, Ally, Neutral];
        let hostile = [
            // Player, Enemy, Ally, Neutral
            [false, true, false, false],  // Player
            [true, false, true, false],   // Enemy
            [false, true, false, false],  // Ally
            [false, false, false, false], // Neutral
        ];
        for (i, &a) in all.iter().enumerate() {
            for (j, &b) in all.iter().enumerate() {
                assert_eq!(a.is_hostile_to(b), hostile[i][j], "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn from_character_copies_the_data() {
        let classes = table(vec![class("brigand")]);
        let unit = Unit::from_character(
            UnitId(7),
            &character("brigand"),
            &classes,
            Faction::Player,
            POS,
        );
        let expected = Unit {
            id: UnitId(7),
            character: Some(CharacterId("hero".into())),
            name: "Hero".into(),
            class: ClassId("brigand".into()),
            level: 3,
            exp: 0,
            class_records: BTreeMap::from([(ClassId("brigand".into()), ClassRecord::UNLOCKED)]),
            stats: Stats::from_growable([22, 7, 1, 5, 6, 4, 2], 5),
            hp: 22,
            faction: Faction::Player,
            pos: POS,
            acted: false,
            is_lord: false,
            weapon_ranks: BTreeMap::from([
                // Kept: already above the class start rank.
                (WeaponKind::Axe, WeaponRank::C),
                // Kept although the class can't use bows.
                (WeaponKind::Bow, WeaponRank::B),
                // Raised to the class start rank.
                (WeaponKind::Sword, WeaponRank::E),
            ]),
        };
        assert_eq!(unit, Ok(expected));
    }

    #[test]
    fn from_character_raises_ranks_below_start() {
        let classes = table(vec![class("brigand")]);
        let mut def = character("brigand");
        def.weapon_ranks = BTreeMap::from([(WeaponKind::Axe, WeaponRank::E)]);
        let unit = Unit::from_character(UnitId(0), &def, &classes, Faction::Player, POS);
        assert_eq!(
            unit.map(|u| u.weapon_ranks.get(&WeaponKind::Axe).copied()),
            Ok(Some(WeaponRank::D))
        );
    }

    #[test]
    fn from_character_clamps_stats_to_caps() {
        let classes = table(vec![class("brigand")]);
        let mut def = character("brigand");
        def.base = Stats::from_growable([99, 7, -3, 17, 6, 4, 2], 12);
        let unit = Unit::from_character(UnitId(0), &def, &classes, Faction::Enemy, POS);
        let stats = Stats::from_growable([45, 7, 0, 16, 6, 4, 2], 5);
        assert_eq!(unit.as_ref().map(|u| u.stats), Ok(stats));
        assert_eq!(unit.map(|u| u.hp), Ok(45));
    }

    #[test]
    fn from_character_lord_only_classes() {
        let mut exile = class("exile");
        exile.lord_only = true;
        let classes = table(vec![exile]);
        let mut def = character("exile");
        assert_eq!(
            Unit::from_character(UnitId(0), &def, &classes, Faction::Player, POS),
            Err(UnitError::LordOnlyClass(ClassId("exile".into())))
        );
        def.is_lord = true;
        let unit = Unit::from_character(UnitId(0), &def, &classes, Faction::Player, POS);
        assert_eq!(unit.map(|u| u.is_lord), Ok(true));
    }

    #[test]
    fn from_character_unknown_class() {
        let classes = table(vec![]);
        assert_eq!(
            Unit::from_character(UnitId(0), &character("x"), &classes, Faction::Player, POS),
            Err(UnitError::UnknownClass(ClassId("x".into())))
        );
    }

    /// Hand-worked example: a level-5 Brigand (4 levels of growth).
    /// HP 20 + 80×4/100 = 23, Str 6 + 2 = 8, Mag 0, Dex 2 + 1 = 3,
    /// Spd 4 + 1 = 5, Def 3 + 1 = 4, Res 0 + 20/100 = 0, Mov 5.
    #[test]
    fn generic_matches_hand_worked_example() {
        let classes = table(vec![class("brigand")]);
        let unit = Unit::generic(
            UnitId(9),
            &ClassId("brigand".into()),
            &classes,
            5,
            Faction::Enemy,
            POS,
        );
        let expected = Unit {
            id: UnitId(9),
            character: None,
            name: "Brigand".into(),
            class: ClassId("brigand".into()),
            level: 5,
            exp: 0,
            class_records: BTreeMap::from([(ClassId("brigand".into()), ClassRecord::UNLOCKED)]),
            stats: Stats::from_growable([23, 8, 0, 3, 5, 4, 0], 5),
            hp: 23,
            faction: Faction::Enemy,
            pos: POS,
            acted: false,
            is_lord: false,
            weapon_ranks: BTreeMap::from([
                (WeaponKind::Axe, WeaponRank::D),
                (WeaponKind::Sword, WeaponRank::E),
            ]),
        };
        assert_eq!(unit, Ok(expected));
    }

    #[test]
    fn generic_level_one_is_base_and_high_levels_cap() {
        let classes = table(vec![class("brigand")]);
        let id = ClassId("brigand".into());
        let at = |level| {
            Unit::generic(UnitId(0), &id, &classes, level, Faction::Enemy, POS).map(|u| u.stats)
        };
        let base = Stats::from_growable([20, 6, 0, 2, 4, 3, 0], 5);
        assert_eq!(at(1), Ok(base));
        assert_eq!(at(0), Ok(base));
        // Level 99: HP 20 + 78 capped at 45, Res 0 + 4 = 4.
        let capped = Stats::from_growable([45, 24, 0, 16, 18, 18, 4], 5);
        assert_eq!(at(99), Ok(capped));
        // Huge levels saturate instead of overflowing.
        let max = Stats::from_growable([45, 24, 0, 16, 18, 18, 10], 5);
        assert_eq!(at(Level::MAX), Ok(max));
    }

    #[test]
    fn generic_rejects_lord_only_and_unknown_classes() {
        let mut exile = class("exile");
        exile.lord_only = true;
        let classes = table(vec![exile]);
        let generic = |id: &str| {
            Unit::generic(
                UnitId(0),
                &ClassId(id.into()),
                &classes,
                1,
                Faction::Ally,
                POS,
            )
        };
        assert_eq!(
            generic("exile"),
            Err(UnitError::LordOnlyClass(ClassId("exile".into())))
        );
        assert_eq!(
            generic("nope"),
            Err(UnitError::UnknownClass(ClassId("nope".into())))
        );
    }

    #[test]
    fn error_messages() {
        let c = ClassId("exile".into());
        assert_eq!(
            UnitError::UnknownClass(c.clone()).to_string(),
            "unknown class \"exile\""
        );
        assert_eq!(
            UnitError::LordOnlyClass(c).to_string(),
            "class \"exile\" is only for the lord"
        );
    }

    fn arb_stats(lo: StatValue) -> impl Strategy<Value = Stats> {
        (prop::array::uniform7(lo..100), lo..20).prop_map(|(v, mov)| Stats::from_growable(v, mov))
    }

    proptest! {
        #[test]
        fn from_character_respects_caps(
            base in arb_stats(-10),
            caps in arb_stats(0),
            mov in 0..15,
        ) {
            let mut c = class("c");
            c.move_points = mov;
            let caps = Stats { mov, ..caps };
            c.caps = caps;
            let classes = table(vec![c]);
            let mut def = character("c");
            def.base = base;
            let unit = Unit::from_character(UnitId(0), &def, &classes, Faction::Player, POS);
            prop_assert!(unit.is_ok());
            if let Ok(u) = unit {
                for kind in StatKind::ALL {
                    prop_assert!(u.stats.get(kind) <= caps.get(kind), "{:?}", kind);
                    prop_assert!(u.stats.get(kind) >= 0, "{:?}", kind);
                }
                prop_assert_eq!(u.stats.mov, mov);
                prop_assert_eq!(u.hp, u.stats.hp);
            }
        }
    }
}
