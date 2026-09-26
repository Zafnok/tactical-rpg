//! Classes and the per-tier progression tables (`docs/design/progression.md`).
//! Skills and spells are referenced by id only; their effects are tickets
//! 0311 and 0309.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::magic::{Affinity, Element, SpellId};
use crate::stats::{Growths, StatValue, Stats};
use crate::terrain::MovementTypeId;
use crate::weapon::{WeaponKind, WeaponRank};

/// String id of a class, e.g. `"swordsman"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassId(pub String);

/// String id of a skill, e.g. `"keen_edge"`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillId(pub String);

/// A class tier: a plain number from 1. Nothing assumes 3 is the top.
pub type Tier = u8;

/// A class level (1 to the class level cap).
pub type ClassLevel = u8;

/// Class points.
pub type ClassPoints = u32;

/// A unit tag, used by weapon effectiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UnitTag {
    /// Land cavalry (spears are effective).
    Mounted,
    /// Fliers (bows are effective). Not `Mounted`.
    Flying,
    /// Armoured units.
    Armored,
}

/// The set of tags a class gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct UnitTags {
    /// Has [`UnitTag::Mounted`].
    pub mounted: bool,
    /// Has [`UnitTag::Flying`].
    pub flying: bool,
    /// Has [`UnitTag::Armored`].
    pub armored: bool,
}

impl UnitTags {
    /// Tags from a list (duplicates are harmless).
    pub fn from_tags(tags: &[UnitTag]) -> Self {
        Self {
            mounted: tags.contains(&UnitTag::Mounted),
            flying: tags.contains(&UnitTag::Flying),
            armored: tags.contains(&UnitTag::Armored),
        }
    }

    /// Whether `tag` is in the set.
    pub fn has(self, tag: UnitTag) -> bool {
        match tag {
            UnitTag::Mounted => self.mounted,
            UnitTag::Flying => self.flying,
            UnitTag::Armored => self.armored,
        }
    }
}

/// An armour weight a class may wear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArmourWeight {
    /// Light armour.
    Light,
    /// Medium armour.
    Medium,
    /// Heavy armour.
    Heavy,
}

/// A weapon kind a class can use, with the rank a unit is raised to on
/// entering the class and the highest rank it can reach in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeaponProficiency {
    /// The weapon kind.
    pub kind: WeaponKind,
    /// Rank given on entering the class (never lowers a rank).
    pub start: WeaponRank,
    /// Highest rank reachable while in the class.
    pub max: WeaponRank,
}

/// One class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDef {
    /// String id.
    pub id: ClassId,
    /// Display name.
    pub name: String,
    /// Tier, from 1.
    pub tier: Tier,
    /// How terrain costs are looked up.
    pub movement_type: MovementTypeId,
    /// Mov of a unit in this class.
    pub move_points: StatValue,
    /// Stats of a level-1 generic unit; also used for the promotion bonus.
    /// `base.mov == move_points`.
    pub base: Stats,
    /// Per-stat caps. `caps.mov == move_points`.
    pub caps: Stats,
    /// Growth rates in percent.
    pub growths: Growths,
    /// Weapon kinds this class can use.
    pub weapons: Vec<WeaponProficiency>,
    /// Armour weights this class can wear.
    pub armour: Vec<ArmourWeight>,
    /// Tags for weapon effectiveness.
    pub tags: UnitTags,
    /// Classes one tier up this class promotes to.
    pub promotes_to: Vec<ClassId>,
    /// Usable from unlock; permanent on mastery.
    pub active: Option<SkillId>,
    /// Learned on mastery.
    pub passives: Vec<SkillId>,
    /// Only enemies use this class.
    pub enemy_only: bool,
    /// Only the lord can be in this class.
    pub lord_only: bool,
    /// Weapons a unit can carry in this class (3, or 0 for tier-3+ magic).
    pub weapon_slots: u8,
    /// Spells learned at a class level.
    pub spells: Vec<(ClassLevel, SpellId)>,
    /// Elemental affinities.
    pub affinities: Vec<(Element, Affinity)>,
}

impl ClassDef {
    /// The proficiency for `kind`, if this class can use it.
    pub fn weapon(&self, kind: WeaponKind) -> Option<&WeaponProficiency> {
        self.weapons.iter().find(|w| w.kind == kind)
    }
}

/// Every class plus the progression tables that go with the class tree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClassTable {
    /// Classes by id.
    pub classes: BTreeMap<ClassId, ClassDef>,
    /// Minimum stat gains per level up; index = tier − 1.
    pub min_gains: Vec<u8>,
    /// Class points per class level; index = tier − 1.
    pub cp_per_class_level: Vec<ClassPoints>,
    /// Highest class level (mastery).
    pub class_level_cap: ClassLevel,
    /// Highest character level.
    pub level_cap: u32,
    /// Absolute limits no stat may exceed.
    pub hard_ceilings: Stats,
}

impl ClassTable {
    /// The class with id `id`.
    pub fn get(&self, id: &ClassId) -> Option<&ClassDef> {
        self.classes.get(id)
    }

    /// Minimum stat gains per level up for classes of `tier`.
    pub fn min_gains(&self, tier: Tier) -> Option<u8> {
        tier_entry(&self.min_gains, tier)
    }

    /// Class points per class level for classes of `tier`.
    pub fn cp_per_class_level(&self, tier: Tier) -> Option<ClassPoints> {
        tier_entry(&self.cp_per_class_level, tier)
    }
}

fn tier_entry<T: Copy>(table: &[T], tier: Tier) -> Option<T> {
    let index = usize::from(tier).checked_sub(1)?;
    table.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_from_list() {
        let t = UnitTags::from_tags(&[UnitTag::Armored, UnitTag::Mounted]);
        assert!(t.has(UnitTag::Mounted));
        assert!(t.has(UnitTag::Armored));
        assert!(!t.has(UnitTag::Flying));
        let f = UnitTags::from_tags(&[UnitTag::Flying]);
        assert_eq!(
            f,
            UnitTags {
                mounted: false,
                flying: true,
                armored: false
            }
        );
        assert!(f.has(UnitTag::Flying));
        assert!(!f.has(UnitTag::Mounted));
        assert!(!f.has(UnitTag::Armored));
    }

    #[test]
    fn tier_tables_are_one_based() {
        let t = ClassTable {
            min_gains: vec![2, 2, 3],
            cp_per_class_level: vec![10, 17, 25],
            ..ClassTable::default()
        };
        assert_eq!(t.min_gains(0), None);
        assert_eq!(t.min_gains(1), Some(2));
        assert_eq!(t.min_gains(3), Some(3));
        assert_eq!(t.min_gains(4), None);
        assert_eq!(t.cp_per_class_level(0), None);
        assert_eq!(t.cp_per_class_level(2), Some(17));
        assert_eq!(t.cp_per_class_level(4), None);
    }

    #[test]
    fn weapon_lookup() {
        let prof = |kind| WeaponProficiency {
            kind,
            start: WeaponRank::E,
            max: WeaponRank::C,
        };
        let class = ClassDef {
            id: ClassId("rider".into()),
            name: "Rider".into(),
            tier: 1,
            movement_type: MovementTypeId(1),
            move_points: 7,
            base: Stats::default(),
            caps: Stats::default(),
            growths: Growths::default(),
            weapons: vec![prof(WeaponKind::Sword), prof(WeaponKind::Spear)],
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
        };
        assert_eq!(
            class.weapon(WeaponKind::Spear).map(|w| w.kind),
            Some(WeaponKind::Spear)
        );
        assert_eq!(class.weapon(WeaponKind::Axe), None);
    }
}
