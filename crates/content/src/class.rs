//! The class tree (`assets/data/classes.ron`) → [`trpg_core::ClassTable`].

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::{
    ClassDef, ClassId, ClassPoints, ClassTable, Element, GrowthValue, Growths, MovementTypeId,
    SkillId, SpellId, StatKind, StatValue, Stats, Tier, UnitTags, WeaponProficiency,
};

use crate::bundle;
use crate::enums::{
    RawAffinity, RawArmourWeight, RawElement, RawUnitTag, RawWeaponKind, RawWeaponRank,
};
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the class table inside the asset bundle.
pub const CLASSES_PATH: &str = "data/classes.ron";

/// Most weapons a class may carry.
pub const MAX_WEAPON_SLOTS: u8 = 3;

/// Highest allowed growth rate, in percent.
pub const MAX_GROWTH: GrowthValue = 255;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    level_cap: u32,
    class_level_cap: u8,
    hard_ceilings: [StatValue; 7],
    mov_ceiling: StatValue,
    min_gains: Vec<u8>,
    cp_per_class_level: Vec<ClassPoints>,
    classes: Vec<RawClass>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClass {
    id: String,
    name: String,
    tier: Tier,
    movement_type: String,
    mov: StatValue,
    #[serde(default)]
    tags: Vec<RawUnitTag>,
    weapons: Vec<(RawWeaponKind, RawWeaponRank, RawWeaponRank)>,
    armour: Vec<RawArmourWeight>,
    weapon_slots: u8,
    base: [StatValue; 7],
    caps: [StatValue; 7],
    growths: [GrowthValue; 7],
    #[serde(default)]
    spells: Vec<(u8, String)>,
    #[serde(default)]
    affinities: Vec<(RawElement, RawAffinity)>,
    #[serde(default)]
    active: Option<String>,
    #[serde(default)]
    passives: Vec<String>,
    #[serde(default)]
    promotes_to: Vec<String>,
    #[serde(default)]
    enemy_only: bool,
    #[serde(default)]
    lord_only: bool,
}

/// Loads and validates the embedded class table. Movement type names are
/// checked against `movement_types` when given (skipped if the terrain file
/// failed to load).
pub fn load(movement_types: Option<&[String]>) -> Result<ClassTable, Vec<ContentError>> {
    let display = bundle::display_path(CLASSES_PATH);
    let source = bundle::file(CLASSES_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source, movement_types)
}

/// Parses and validates class-table `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(
    file: &str,
    source: &str,
    movement_types: Option<&[String]>,
) -> Result<ClassTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut v = Validator {
        file,
        source,
        errors: Vec::new(),
    };
    let hard_ceilings = Stats::from_growable(raw.hard_ceilings, raw.mov_ceiling);
    let mut classes = BTreeMap::new();
    for c in &raw.classes {
        let class = v.class(c, movement_types, &hard_ceilings);
        if classes.insert(class.id.clone(), class).is_some() {
            v.err(&c.id, format!("duplicate class id \"{}\"", c.id));
        }
    }
    v.promotions(&classes);
    v.tier_table(&classes, "min_gains", &raw.min_gains);
    v.tier_table(&classes, "cp_per_class_level", &raw.cp_per_class_level);
    v.reachability(&classes);
    if v.errors.is_empty() {
        Ok(ClassTable {
            classes,
            min_gains: raw.min_gains,
            cp_per_class_level: raw.cp_per_class_level,
            class_level_cap: raw.class_level_cap,
            level_cap: raw.level_cap,
            hard_ceilings,
        })
    } else {
        Err(v.errors)
    }
}

/// Collects validation errors for one class file.
struct Validator<'a> {
    file: &'a str,
    source: &'a str,
    errors: Vec<ContentError>,
}

impl Validator<'_> {
    /// Records an error about class `id` (positioned at its `id:` line).
    fn err(&mut self, id: &str, message: String) {
        let e = ContentError::new(self.file, message);
        let line = line_of(self.source, &format!("id: \"{id}\""));
        self.errors.push(match line {
            Some(l) => e.at(l, None),
            None => e,
        });
    }

    /// Validates one class entry and converts it (placeholder values stand in
    /// for invalid fields; the caller discards them if any error exists).
    fn class(
        &mut self,
        c: &RawClass,
        movement_types: Option<&[String]>,
        ceilings: &Stats,
    ) -> ClassDef {
        let what = format!("class \"{}\"", c.id);
        if c.tier == 0 {
            self.err(&c.id, format!("{what}: tier must be at least 1"));
        }
        let movement_type = self.movement_type(c, &what, movement_types);
        if c.mov < 0 || c.mov > ceilings.mov {
            self.err(
                &c.id,
                format!("{what}: mov {} is outside 0..={}", c.mov, ceilings.mov),
            );
        }
        if c.weapon_slots > MAX_WEAPON_SLOTS {
            self.err(
                &c.id,
                format!(
                    "{what}: weapon_slots {} is over {MAX_WEAPON_SLOTS}",
                    c.weapon_slots
                ),
            );
        }
        let base = Stats::from_growable(c.base, c.mov);
        let caps = Stats::from_growable(c.caps, c.mov);
        self.stats(&c.id, &what, &base, &caps, ceilings);
        for (kind, g) in StatKind::GROWABLE.iter().zip(c.growths) {
            if g > MAX_GROWTH {
                self.err(
                    &c.id,
                    format!("{what}: {kind:?} growth {g} is over {MAX_GROWTH}"),
                );
            }
        }
        let weapons = self.weapons(&c.id, &what, &c.weapons);
        let mut elements = BTreeSet::new();
        for &(element, _) in &c.affinities {
            let element = Element::from(element);
            if !elements.insert(element) {
                self.err(
                    &c.id,
                    format!("{what}: more than one affinity for {element:?}"),
                );
            }
        }
        ClassDef {
            id: ClassId(c.id.clone()),
            name: c.name.clone(),
            tier: c.tier,
            movement_type,
            move_points: c.mov,
            base,
            caps,
            growths: Growths(c.growths),
            weapons,
            armour: c.armour.iter().map(|&a| a.into()).collect(),
            tags: UnitTags::from_tags(&c.tags.iter().map(|&t| t.into()).collect::<Vec<_>>()),
            promotes_to: c.promotes_to.iter().map(|p| ClassId(p.clone())).collect(),
            active: c.active.clone().map(SkillId),
            passives: c.passives.iter().map(|p| SkillId(p.clone())).collect(),
            enemy_only: c.enemy_only,
            lord_only: c.lord_only,
            weapon_slots: c.weapon_slots,
            spells: c
                .spells
                .iter()
                .map(|(level, s)| (*level, SpellId(s.clone())))
                .collect(),
            affinities: c
                .affinities
                .iter()
                .map(|&(e, a)| (e.into(), a.into()))
                .collect(),
        }
    }

    fn movement_type(
        &mut self,
        c: &RawClass,
        what: &str,
        movement_types: Option<&[String]>,
    ) -> MovementTypeId {
        let Some(types) = movement_types else {
            return MovementTypeId(0);
        };
        let index = types.iter().position(|m| *m == c.movement_type);
        if let Some(i) = index.and_then(|i| u8::try_from(i).ok()) {
            return MovementTypeId(i);
        }
        self.err(
            &c.id,
            format!(
                "{what}: unknown movement type \"{}\" (not in terrain.ron)",
                c.movement_type
            ),
        );
        MovementTypeId(0)
    }

    /// Checks `0 ≤ base ≤ caps ≤ hard ceilings` for every growable stat.
    fn stats(&mut self, id: &str, what: &str, base: &Stats, caps: &Stats, ceilings: &Stats) {
        for kind in StatKind::GROWABLE {
            let (b, c, h) = (base.get(kind), caps.get(kind), ceilings.get(kind));
            if b < 0 {
                self.err(id, format!("{what}: base {kind:?} {b} is negative"));
            }
            if b > c {
                self.err(id, format!("{what}: base {kind:?} {b} is over its cap {c}"));
            }
            if c > h {
                self.err(
                    id,
                    format!("{what}: cap {kind:?} {c} is over the hard ceiling {h}"),
                );
            }
        }
    }

    fn weapons(
        &mut self,
        id: &str,
        what: &str,
        weapons: &[(RawWeaponKind, RawWeaponRank, RawWeaponRank)],
    ) -> Vec<WeaponProficiency> {
        weapons
            .iter()
            .map(|&(kind, start, max)| {
                let p = WeaponProficiency {
                    kind: kind.into(),
                    start: start.into(),
                    max: max.into(),
                };
                if p.start > p.max {
                    self.err(
                        id,
                        format!(
                            "{what}: {:?} start rank {:?} is above its max rank {:?}",
                            p.kind, p.start, p.max
                        ),
                    );
                }
                p
            })
            .collect()
    }

    /// Promotion targets exist, are one tier up, aren't enemy-only, and are
    /// lord-only exactly when the class promoting into them is.
    fn promotions(&mut self, classes: &BTreeMap<ClassId, ClassDef>) {
        for class in classes.values() {
            let from = &class.id.0;
            for target in &class.promotes_to {
                let to = &target.0;
                let Some(t) = classes.get(target) else {
                    self.err(
                        from,
                        format!("class \"{from}\": promotes to unknown class \"{to}\""),
                    );
                    continue;
                };
                if u16::from(t.tier) != u16::from(class.tier) + 1 {
                    self.err(
                        from,
                        format!(
                            "class \"{from}\" (tier {}): promotion \"{to}\" is tier {}, not {}",
                            class.tier,
                            t.tier,
                            u16::from(class.tier) + 1
                        ),
                    );
                }
                if t.enemy_only {
                    self.err(
                        from,
                        format!("class \"{from}\": promotes to enemy-only class \"{to}\""),
                    );
                }
                if t.lord_only != class.lord_only {
                    let message = if class.lord_only {
                        format!(
                            "lord-only class \"{from}\" promotes to \"{to}\", which isn't lord-only"
                        )
                    } else {
                        format!("class \"{from}\" promotes to lord-only class \"{to}\"")
                    };
                    self.err(from, message);
                }
            }
        }
    }

    /// A per-tier table must never decrease and must cover every tier used.
    fn tier_table<T: PartialOrd + Copy>(
        &mut self,
        classes: &BTreeMap<ClassId, ClassDef>,
        name: &str,
        table: &[T],
    ) {
        let line = line_of(self.source, &format!("{name}:"));
        let file = self.file;
        let push = |errors: &mut Vec<ContentError>, message: String| {
            let e = ContentError::new(file, message);
            errors.push(match line {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        if let Some(i) = table.windows(2).position(|w| w[1] < w[0]) {
            push(
                &mut self.errors,
                format!(
                    "{name}: tier {} is lower than tier {} (must never decrease)",
                    i + 2,
                    i + 1
                ),
            );
        }
        let top = classes.values().map(|c| c.tier).max().unwrap_or(0);
        if usize::from(top) > table.len() {
            push(
                &mut self.errors,
                format!(
                    "{name}: has {} entries but classes go up to tier {top}",
                    table.len()
                ),
            );
        }
    }

    /// Every class that isn't enemy-only must be reachable from a tier-1
    /// class by promotions.
    fn reachability(&mut self, classes: &BTreeMap<ClassId, ClassDef>) {
        let mut reached: BTreeSet<&ClassId> = classes
            .values()
            .filter(|c| c.tier == 1)
            .map(|c| &c.id)
            .collect();
        let mut frontier: Vec<&ClassId> = reached.iter().copied().collect();
        while let Some(id) = frontier.pop() {
            for next in classes.get(id).into_iter().flat_map(|c| &c.promotes_to) {
                if reached.insert(next) {
                    frontier.push(next);
                }
            }
        }
        for class in classes.values() {
            if !class.enemy_only && !reached.contains(&class.id) {
                self.err(
                    &class.id.0,
                    format!(
                        "class \"{}\" (tier {}) can't be reached by promotion from a tier-1 class",
                        class.id.0, class.tier
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{Affinity, ArmourWeight, UnitTag, WeaponKind, WeaponRank};

    use super::*;

    const TYPES: &[&str] = &["foot", "mounted", "armored", "flying"];

    fn types() -> Vec<String> {
        TYPES.iter().map(|&t| t.to_owned()).collect()
    }

    /// One class entry; `extra` is spliced in as more fields.
    fn class(id: &str, tier: u8, promotes_to: &[&str], extra: &str) -> String {
        let targets: Vec<String> = promotes_to.iter().map(|t| format!("\"{t}\"")).collect();
        format!(
            "        (\n            id: \"{id}\", name: \"N\", tier: {tier}, movement_type: \"foot\", mov: 5,\n            weapons: [(Sword, D, C)], armour: [Light], weapon_slots: 3,\n            base: (18, 5, 0, 7, 8, 3, 1), caps: (40, 20, 10, 24, 25, 18, 15),\n            growths: (70, 40, 10, 55, 60, 25, 20), promotes_to: [{}], {extra}\n        ),\n",
            targets.join(", ")
        )
    }

    fn file_with(min_gains: &str, cp: &str, classes: &[String]) -> String {
        format!(
            "(\n    level_cap: 99,\n    class_level_cap: 10,\n    hard_ceilings: (80, 50, 50, 50, 50, 50, 50),\n    mov_ceiling: 15,\n    min_gains: {min_gains},\n    cp_per_class_level: {cp},\n    classes: [\n{}    ],\n)",
            classes.concat()
        )
    }

    fn file(classes: &[String]) -> String {
        file_with("[2, 2, 3]", "[10, 17, 25]", classes)
    }

    fn load_src(src: &str) -> Result<ClassTable, Vec<ContentError>> {
        let types = types();
        from_source("c.ron", src, Some(&types))
    }

    fn errors(src: &str) -> Vec<String> {
        load_src(src)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// A small valid tree: a → b → c.
    fn tree(extra_a: &str) -> Vec<String> {
        vec![
            class("a", 1, &["b"], extra_a),
            class("b", 2, &["c"], ""),
            class("c", 3, &[], ""),
        ]
    }

    #[test]
    fn valid_file_loads() {
        let extra = "tags: [Mounted, Armored], active: Some(\"keen\"), passives: [\"p1\", \"p2\"], \
                     spells: [(1, \"fire\"), (5, \"frost\")], affinities: [(Fire, Absorb), (Ice, Weak)], \
                     enemy_only: false, lord_only: false";
        let mut classes = tree(extra);
        classes[0] = classes[0].replace(
            "weapons: [(Sword, D, C)], armour: [Light]",
            "weapons: [(Sword, E, C), (Spear, D, B)], armour: [Medium, Heavy]",
        );
        classes[0] = classes[0].replace("movement_type: \"foot\"", "movement_type: \"armored\"");
        let t = load_src(&file(&classes));
        assert!(t.is_ok(), "{t:?}");
        let t = t.unwrap_or_default();
        assert_eq!(t.level_cap, 99);
        assert_eq!(t.class_level_cap, 10);
        assert_eq!(
            t.hard_ceilings,
            Stats::from_growable([80, 50, 50, 50, 50, 50, 50], 15)
        );
        assert_eq!(t.min_gains, [2, 2, 3]);
        assert_eq!(t.cp_per_class_level, [10, 17, 25]);
        let a = t.get(&ClassId("a".into()));
        let expected = ClassDef {
            id: ClassId("a".into()),
            name: "N".into(),
            tier: 1,
            movement_type: MovementTypeId(2),
            move_points: 5,
            base: Stats::from_growable([18, 5, 0, 7, 8, 3, 1], 5),
            caps: Stats::from_growable([40, 20, 10, 24, 25, 18, 15], 5),
            growths: Growths([70, 40, 10, 55, 60, 25, 20]),
            weapons: vec![
                WeaponProficiency {
                    kind: WeaponKind::Sword,
                    start: WeaponRank::E,
                    max: WeaponRank::C,
                },
                WeaponProficiency {
                    kind: WeaponKind::Spear,
                    start: WeaponRank::D,
                    max: WeaponRank::B,
                },
            ],
            armour: vec![ArmourWeight::Medium, ArmourWeight::Heavy],
            tags: UnitTags::from_tags(&[UnitTag::Mounted, UnitTag::Armored]),
            promotes_to: vec![ClassId("b".into())],
            active: Some(SkillId("keen".into())),
            passives: vec![SkillId("p1".into()), SkillId("p2".into())],
            enemy_only: false,
            lord_only: false,
            weapon_slots: 3,
            spells: vec![(1, SpellId("fire".into())), (5, SpellId("frost".into()))],
            affinities: vec![
                (Element::Fire, Affinity::Absorb),
                (Element::Ice, Affinity::Weak),
            ],
        };
        assert_eq!(a, Some(&expected));
        let c = t.get(&ClassId("c".into()));
        assert_eq!(c.map(|c| (c.tier, c.active.is_none())), Some((3, true)));
    }

    #[test]
    fn defaults_for_optional_fields() {
        let t = load_src(&file(&tree(""))).unwrap_or_default();
        let a = t.get(&ClassId("a".into()));
        assert!(a.is_some_and(|a| a.tags == UnitTags::default()
            && a.spells.is_empty()
            && a.affinities.is_empty()
            && a.active.is_none()
            && a.passives.is_empty()
            && !a.enemy_only
            && !a.lord_only));
    }

    #[test]
    fn syntax_and_unknown_names_are_positioned() {
        let src = file(&tree("")).replace("(Sword, D, C)", "(Whip, D, C)");
        let errs = load_src(&src).err().unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(11));
        assert!(errs[0].message.contains("Whip"));
        let src = file(&tree("tags: [Swimming]"));
        assert!(errors(&src)[0].contains("Swimming"));
    }

    #[test]
    fn unknown_movement_type() {
        let src = file(&tree("")).replacen("\"foot\"", "\"boat\"", 1);
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": unknown movement type \"boat\" (not in terrain.ron)"]
        );
        // Without the terrain's movement types the names aren't checked.
        assert!(from_source("c.ron", &src, None).is_ok());
    }

    #[test]
    fn duplicate_class_id() {
        let mut classes = tree("");
        classes.push(class("c", 3, &[], ""));
        assert_eq!(
            errors(&file(&classes)),
            ["c.ron:22: duplicate class id \"c\""]
        );
    }

    #[test]
    fn tier_must_be_positive() {
        let mut classes = tree("");
        classes.push(class("z", 0, &[], "enemy_only: true"));
        assert_eq!(
            errors(&file(&classes)),
            ["c.ron:28: class \"z\": tier must be at least 1"]
        );
    }

    #[test]
    fn mov_range() {
        let src = file(&tree("")).replacen("mov: 5", "mov: 16", 1);
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": mov 16 is outside 0..=15"]
        );
        let src = file(&tree("")).replacen("mov: 5", "mov: -1", 1);
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": mov -1 is outside 0..=15"]
        );
        assert!(load_src(&file(&tree("")).replacen("mov: 5", "mov: 15", 1)).is_ok());
        assert!(load_src(&file(&tree("")).replacen("mov: 5", "mov: 0", 1)).is_ok());
    }

    #[test]
    fn weapon_slots_at_most_three() {
        let src = file(&tree("")).replacen("weapon_slots: 3", "weapon_slots: 4", 1);
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": weapon_slots 4 is over 3"]
        );
        let src = file(&tree("")).replacen("weapon_slots: 3", "weapon_slots: 0", 1);
        assert!(load_src(&src).is_ok());
    }

    #[test]
    fn start_rank_at_most_max_rank() {
        let src = file(&tree("")).replacen("(Sword, D, C)", "(Sword, B, C)", 1);
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": Sword start rank B is above its max rank C"]
        );
        let src = file(&tree("")).replacen("(Sword, D, C)", "(Sword, C, C)", 1);
        assert!(load_src(&src).is_ok());
    }

    #[test]
    fn stats_base_caps_and_ceilings() {
        let src = file(&tree(""))
            .replacen("base: (18, 5, 0,", "base: (18, 21, -1,", 1)
            .replacen(
                "caps: (40, 20, 10, 24, 25, 18, 15)",
                "caps: (81, 20, 10, 24, 25, 18, 51)",
                1,
            );
        assert_eq!(
            errors(&src),
            [
                "c.ron:10: class \"a\": cap Hp 81 is over the hard ceiling 80",
                "c.ron:10: class \"a\": base Str 21 is over its cap 20",
                "c.ron:10: class \"a\": base Mag -1 is negative",
                "c.ron:10: class \"a\": cap Res 51 is over the hard ceiling 50",
            ]
        );
        // Equal values are fine.
        let src = file(&tree(""))
            .replacen("base: (18, 5,", "base: (18, 20,", 1)
            .replacen("caps: (40,", "caps: (80,", 1);
        assert!(load_src(&src).is_ok());
    }

    #[test]
    fn growths_at_most_255() {
        let src = file(&tree("")).replacen("growths: (70, 40", "growths: (256, 255", 1);
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": Hp growth 256 is over 255"]
        );
        let src = file(&tree("")).replacen("growths: (70", "growths: (-1", 1);
        assert_eq!(load_src(&src).err().map(|e| e.len()), Some(1));
    }

    #[test]
    fn one_affinity_per_element() {
        let src = file(&tree(
            "affinities: [(Fire, Weak), (Ice, Weak), (Fire, Resist)]",
        ));
        assert_eq!(
            errors(&src),
            ["c.ron:10: class \"a\": more than one affinity for Fire"]
        );
    }

    #[test]
    fn promotion_rules() {
        let classes = vec![
            class("a", 1, &["b", "missing", "c", "e", "l2"], ""),
            class("b", 2, &[], ""),
            class("c", 3, &[], ""),
            class("e", 2, &[], "enemy_only: true"),
            class("l1", 1, &["l2", "b"], "lord_only: true"),
            class("l2", 2, &[], "lord_only: true"),
        ];
        assert_eq!(
            errors(&file(&classes)),
            [
                "c.ron:10: class \"a\": promotes to unknown class \"missing\"",
                "c.ron:10: class \"a\" (tier 1): promotion \"c\" is tier 3, not 2",
                "c.ron:10: class \"a\": promotes to enemy-only class \"e\"",
                "c.ron:10: class \"a\" promotes to lord-only class \"l2\"",
                "c.ron:34: lord-only class \"l1\" promotes to \"b\", which isn't lord-only",
            ]
        );
    }

    #[test]
    fn promotion_to_a_lower_tier() {
        let classes = vec![class("a", 1, &["b"], ""), class("b", 2, &["a"], "")];
        let errs = errors(&file_with("[2, 2]", "[10, 17]", &classes));
        assert_eq!(
            errs,
            ["c.ron:16: class \"b\" (tier 2): promotion \"a\" is tier 1, not 3"]
        );
    }

    #[test]
    fn tier_tables_never_decrease() {
        let errs = errors(&file_with("[2, 3, 2]", "[10, 9, 25]", &tree("")));
        assert_eq!(
            errs,
            [
                "c.ron:6: min_gains: tier 3 is lower than tier 2 (must never decrease)",
                "c.ron:7: cp_per_class_level: tier 2 is lower than tier 1 (must never decrease)",
            ]
        );
        assert!(load_src(&file_with("[2, 2, 2]", "[10, 10, 10, 11]", &tree(""))).is_ok());
    }

    #[test]
    fn tier_tables_cover_every_tier() {
        let errs = errors(&file_with("[2, 2]", "[]", &tree("")));
        assert_eq!(
            errs,
            [
                "c.ron:6: min_gains: has 2 entries but classes go up to tier 3",
                "c.ron:7: cp_per_class_level: has 0 entries but classes go up to tier 3",
            ]
        );
        assert!(load_src(&file_with("[]", "[]", &[])).is_ok());
    }

    #[test]
    fn every_class_reachable() {
        let mut classes = tree("");
        classes.push(class("orphan", 2, &["orphan3"], ""));
        classes.push(class("orphan3", 3, &[], ""));
        classes.push(class("boss", 3, &[], "enemy_only: true"));
        assert_eq!(
            errors(&file(&classes)),
            [
                "c.ron:28: class \"orphan\" (tier 2) can't be reached by promotion from a tier-1 class",
                "c.ron:34: class \"orphan3\" (tier 3) can't be reached by promotion from a tier-1 class",
            ]
        );
    }

    #[test]
    fn all_errors_reported_together() {
        let src = file_with(
            "[3, 2]",
            "[10, 17, 25]",
            &[
                class("a", 1, &["zz"], "affinities: [(Ice, Weak), (Ice, Weak)]"),
                class("b", 2, &[], "").replacen("weapon_slots: 3", "weapon_slots: 9", 1),
                class("c", 3, &[], ""),
            ],
        );
        // zz unknown, Ice twice, 9 slots, min_gains decreasing and too short,
        // b and c unreachable.
        assert_eq!(errors(&src).len(), 7);
    }

    #[test]
    fn empty_source_is_a_syntax_error() {
        assert_eq!(load_src("").err().map(|e| e.len()), Some(1));
    }

    /// Every class in `progression.md`'s tree (tiers 1-3), by tier.
    const TREE: &[(u8, &[&str])] = &[
        (
            1,
            &[
                "swordsman",
                "brawler",
                "raider",
                "archer",
                "guard",
                "rider",
                "mage",
                "cleric",
                "exile",
                "brigand",
                "fire_elemental",
                "frost_elemental",
            ],
        ),
        (
            2,
            &[
                "duelist",
                "shadowblade",
                "striker",
                "grappler",
                "berserker",
                "vanguard",
                "marksman",
                "outrider",
                "bulwark",
                "iron_rider",
                "lancer",
                "sorcerer",
                "mystic",
                "priest",
                "blade_heir",
                "commander",
            ],
        ),
        (
            3,
            &[
                "blade_dancer",
                "nightblade",
                "tempest_fist",
                "colossus",
                "ravager",
                "warchief",
                "deadeye",
                "windrunner",
                "bastion",
                "juggernaut",
                "high_lancer",
                "flier",
                "archmage",
                "arcanist",
                "oracle",
                "sovereign",
                "grand_marshal",
            ],
        ),
    ];

    fn embedded() -> ClassTable {
        let t = load(Some(&types()));
        assert!(t.is_ok(), "{t:?}");
        t.unwrap_or_default()
    }

    /// Acceptance: the embedded tree is the one in `progression.md` (tiers
    /// 1-3, the tier-3 Flier, the lord's line and the enemy-only classes).
    #[test]
    fn embedded_class_tree_matches_design_doc() {
        let t = embedded();
        let count: usize = TREE.iter().map(|(_, ids)| ids.len()).sum();
        assert_eq!(t.classes.len(), count);
        for (tier, ids) in TREE {
            for id in *ids {
                let class = t.get(&ClassId((*id).into()));
                assert_eq!(class.map(|c| c.tier), Some(*tier), "{id}");
            }
        }
        let flags = |id: &str| {
            t.get(&ClassId(id.into()))
                .map(|c| (c.enemy_only, c.lord_only))
        };
        for id in ["brigand", "fire_elemental", "frost_elemental"] {
            assert_eq!(flags(id), Some((true, false)), "{id}");
        }
        for id in [
            "exile",
            "blade_heir",
            "commander",
            "sovereign",
            "grand_marshal",
        ] {
            assert_eq!(flags(id), Some((false, true)), "{id}");
        }
        let promotes = |id: &str| {
            t.get(&ClassId(id.into())).map(|c| {
                c.promotes_to
                    .iter()
                    .map(|p| p.0.as_str())
                    .collect::<Vec<_>>()
            })
        };
        assert_eq!(promotes("guard"), Some(vec!["bulwark", "iron_rider"]));
        assert_eq!(promotes("rider"), Some(vec!["iron_rider", "lancer"]));
        assert_eq!(promotes("lancer"), Some(vec!["high_lancer", "flier"]));
        assert_eq!(promotes("mage"), Some(vec!["sorcerer", "mystic"]));
        assert_eq!(promotes("cleric"), Some(vec!["mystic", "priest"]));
        assert_eq!(promotes("exile"), Some(vec!["blade_heir", "commander"]));
        assert_eq!(promotes("flier"), Some(vec![]));
        assert_eq!(t.min_gains, [2, 2, 3]);
        assert_eq!(t.cp_per_class_level, [10, 17, 25]);
        assert_eq!(t.class_level_cap, 10);
        assert_eq!(t.level_cap, 99);
    }

    /// Acceptance: two classes, one a shared promotion, match hand-written
    /// expectations from `progression.md` and `magic.md`.
    #[test]
    fn embedded_class_numbers_match_design_doc() {
        let t = embedded();
        let sp = |kind, start, max| WeaponProficiency { kind, start, max };
        let iron_rider = ClassDef {
            id: ClassId("iron_rider".into()),
            name: "Iron Rider".into(),
            tier: 2,
            movement_type: MovementTypeId(1),
            move_points: 6,
            base: Stats::from_growable([27, 10, 0, 6, 6, 11, 2], 6),
            caps: Stats::from_growable([56, 30, 10, 24, 22, 32, 14], 6),
            growths: Growths([85, 50, 5, 40, 35, 50, 10]),
            weapons: vec![
                sp(WeaponKind::Spear, WeaponRank::C, WeaponRank::A),
                sp(WeaponKind::Axe, WeaponRank::D, WeaponRank::B),
                sp(WeaponKind::Sword, WeaponRank::D, WeaponRank::B),
            ],
            armour: vec![ArmourWeight::Medium, ArmourWeight::Heavy],
            tags: UnitTags::from_tags(&[UnitTag::Armored, UnitTag::Mounted]),
            promotes_to: vec![ClassId("juggernaut".into())],
            active: Some(SkillId("trample".into())),
            passives: vec![SkillId("charge_1".into()), SkillId("steadfast_1".into())],
            enemy_only: false,
            lord_only: false,
            weapon_slots: 3,
            spells: vec![],
            affinities: vec![],
        };
        assert_eq!(t.get(&iron_rider.id), Some(&iron_rider));
        let frost = ClassDef {
            id: ClassId("frost_elemental".into()),
            name: "Frost Elemental".into(),
            tier: 1,
            movement_type: MovementTypeId(0),
            move_points: 4,
            base: Stats::from_growable([30, 0, 8, 4, 5, 8, 4], 4),
            caps: Stats::from_growable([60, 10, 30, 24, 20, 30, 30], 4),
            growths: Growths([80, 0, 50, 35, 30, 40, 40]),
            weapons: vec![],
            armour: vec![],
            tags: UnitTags::default(),
            promotes_to: vec![],
            active: None,
            passives: vec![],
            enemy_only: true,
            lord_only: false,
            weapon_slots: 0,
            spells: vec![(1, SpellId("frost".into()))],
            affinities: vec![
                (Element::Ice, Affinity::Absorb),
                (Element::Fire, Affinity::Weak),
            ],
        };
        assert_eq!(t.get(&frost.id), Some(&frost));
        let archmage = t.get(&ClassId("archmage".into()));
        assert_eq!(archmage.map(|c| c.weapon_slots), Some(0));
        let flier = t.get(&ClassId("flier".into()));
        assert_eq!(
            flier.map(|c| (c.tier, c.tags.has(UnitTag::Flying), c.movement_type)),
            Some((3, true, MovementTypeId(3)))
        );
    }
}
