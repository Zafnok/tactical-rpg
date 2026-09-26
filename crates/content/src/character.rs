//! Named characters and generic unit templates
//! (`assets/data/characters.ron`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::{
    CharacterDef, CharacterId, ClassDef, ClassId, ClassTable, Level, SpellId, StatKind, StatValue,
    Stats,
};

use crate::bundle;
use crate::enums::{RawStatKind, RawWeaponKind, RawWeaponRank};
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the character file inside the asset bundle.
pub const CHARACTERS_PATH: &str = "data/characters.ron";

/// Most personal spells a character may have.
pub const MAX_PERSONAL_SPELLS: usize = 2;

/// A generic unit template: a class at a character level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericTemplate {
    /// String id, e.g. `"test_brigand"`.
    pub id: String,
    /// The unit's class.
    pub class: ClassId,
    /// The unit's character level.
    pub level: Level,
}

/// Every named character and generic template, by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CharacterTable {
    /// Named characters.
    pub characters: BTreeMap<CharacterId, CharacterDef>,
    /// Generic unit templates.
    pub generics: BTreeMap<String, GenericTemplate>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    characters: Vec<RawCharacter>,
    generics: Vec<RawGeneric>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCharacter {
    id: String,
    name: String,
    class: String,
    level: Level,
    #[serde(default)]
    is_lord: bool,
    talent: RawStatKind,
    base: [StatValue; 7],
    weapon_ranks: Vec<(RawWeaponKind, RawWeaponRank)>,
    #[serde(default)]
    personal_spells: Vec<(Level, String)>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeneric {
    id: String,
    class: String,
    level: Level,
}

/// Loads and validates the embedded character file. Class references are
/// checked against `classes` when given (skipped if the class file failed to
/// load).
pub fn load(classes: Option<&ClassTable>) -> Result<CharacterTable, Vec<ContentError>> {
    let display = bundle::display_path(CHARACTERS_PATH);
    let source = bundle::file(CHARACTERS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source, classes)
}

/// Parses and validates character `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(
    file: &str,
    source: &str,
    classes: Option<&ClassTable>,
) -> Result<CharacterTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut v = Validator {
        file,
        source,
        classes,
        errors: Vec::new(),
    };
    let mut table = CharacterTable::default();
    for c in &raw.characters {
        let def = v.character(c);
        if table.characters.insert(def.id.clone(), def).is_some() {
            v.err(&c.id, format!("duplicate character id \"{}\"", c.id));
        }
    }
    let mut seen = BTreeSet::new();
    for g in &raw.generics {
        if !seen.insert(g.id.as_str()) {
            v.err(&g.id, format!("duplicate generic id \"{}\"", g.id));
        }
        let what = format!("generic \"{}\"", g.id);
        if let Some(class) = v.class(&g.id, &what, &g.class)
            && class.lord_only
        {
            v.err(&g.id, format!("{what}: class \"{}\" is lord-only", g.class));
        }
        v.level(&g.id, &what, g.level);
        table.generics.insert(
            g.id.clone(),
            GenericTemplate {
                id: g.id.clone(),
                class: ClassId(g.class.clone()),
                level: g.level,
            },
        );
    }
    if v.errors.is_empty() {
        Ok(table)
    } else {
        Err(v.errors)
    }
}

/// Collects validation errors for one character file.
struct Validator<'a> {
    file: &'a str,
    source: &'a str,
    classes: Option<&'a ClassTable>,
    errors: Vec<ContentError>,
}

impl<'a> Validator<'a> {
    /// Records an error about entry `id` (positioned at its `id:` line).
    fn err(&mut self, id: &str, message: String) {
        let e = ContentError::new(self.file, message);
        let line = line_of(self.source, &format!("id: \"{id}\""));
        self.errors.push(match line {
            Some(l) => e.at(l, None),
            None => e,
        });
    }

    /// The class `class`, or `None` (with an error if it is unknown, and
    /// silently if the class table isn't available).
    fn class(&mut self, id: &str, what: &str, class: &str) -> Option<&'a ClassDef> {
        let classes = self.classes?;
        let def = classes.get(&ClassId(class.to_owned()));
        if def.is_none() {
            self.err(id, format!("{what}: unknown class \"{class}\""));
        }
        def
    }

    /// Checks `level` is in `1..=level_cap`.
    fn level(&mut self, id: &str, what: &str, level: Level) {
        let Some(cap) = self.classes.map(|c| c.level_cap) else {
            return;
        };
        if level < 1 || level > cap {
            self.err(id, format!("{what}: level {level} is outside 1..={cap}"));
        }
    }

    fn character(&mut self, c: &RawCharacter) -> CharacterDef {
        let what = format!("character \"{}\"", c.id);
        let talent = StatKind::from(c.talent);
        if talent == StatKind::Mov {
            self.err(&c.id, format!("{what}: talent can't be Mov"));
        }
        if c.personal_spells.len() > MAX_PERSONAL_SPELLS {
            self.err(
                &c.id,
                format!(
                    "{what}: {} personal spells, at most {MAX_PERSONAL_SPELLS}",
                    c.personal_spells.len()
                ),
            );
        }
        if c.is_lord && !c.personal_spells.is_empty() {
            self.err(&c.id, format!("{what}: the lord has no personal spells"));
        }
        self.level(&c.id, &what, c.level);
        let class = self.class(&c.id, &what, &c.class);
        let mov = class.map_or(0, |k| k.move_points);
        let base = Stats::from_growable(c.base, mov);
        let weapon_ranks = c
            .weapon_ranks
            .iter()
            .map(|&(k, r)| (k.into(), r.into()))
            .collect();
        let def = CharacterDef {
            id: CharacterId(c.id.clone()),
            name: c.name.clone(),
            class: ClassId(c.class.clone()),
            level: c.level,
            is_lord: c.is_lord,
            talent,
            base,
            weapon_ranks,
            personal_spells: c
                .personal_spells
                .iter()
                .map(|(level, s)| (*level, SpellId(s.clone())))
                .collect(),
        };
        if let Some(class) = class {
            self.against_class(&def, &what, class);
        }
        def
    }

    /// Checks the character fits its starting class.
    fn against_class(&mut self, def: &CharacterDef, what: &str, class: &ClassDef) {
        let id = &def.id.0;
        if class.lord_only && !def.is_lord {
            self.err(
                id,
                format!(
                    "{what}: class \"{}\" is lord-only but is_lord is false",
                    class.id.0
                ),
            );
        }
        for kind in StatKind::GROWABLE {
            let (b, cap) = (def.base.get(kind), class.caps.get(kind));
            if b < 0 {
                self.err(id, format!("{what}: base {kind:?} {b} is negative"));
            }
            if b > cap {
                self.err(
                    id,
                    format!(
                        "{what}: base {kind:?} {b} is over the \"{}\" cap {cap}",
                        class.id.0
                    ),
                );
            }
        }
        for (&kind, &rank) in &def.weapon_ranks {
            if let Some(w) = class.weapon(kind).filter(|w| rank > w.max) {
                self.err(
                    id,
                    format!(
                        "{what}: {kind:?} rank {rank:?} is above the \"{}\" max rank {:?}",
                        class.id.0, w.max
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{Faction, Pos, Unit, UnitId, WeaponKind, WeaponRank};

    use super::*;

    fn classes() -> ClassTable {
        let types: Vec<String> = ["foot", "mounted", "armored", "flying"]
            .iter()
            .map(|&t| t.to_owned())
            .collect();
        crate::class::load(Some(&types)).unwrap_or_default()
    }

    fn character(id: &str, class: &str, extra: &str) -> String {
        format!(
            "        (\n            id: \"{id}\", name: \"N\", class: \"{class}\", level: 1, talent: Spd,\n            base: (18, 5, 0, 7, 8, 3, 1), weapon_ranks: [(Sword, D)], {extra}\n        ),\n"
        )
    }

    fn file(characters: &[String], generics: &str) -> String {
        format!(
            "(\n    characters: [\n{}    ],\n    generics: [{generics}],\n)",
            characters.concat()
        )
    }

    fn load_src(src: &str) -> Result<CharacterTable, Vec<ContentError>> {
        from_source("ch.ron", src, Some(&classes()))
    }

    fn errors(src: &str) -> Vec<String> {
        load_src(src)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn valid_file_loads() {
        let src = file(
            &[character(
                "hero",
                "swordsman",
                "personal_spells: [(1, \"fire\"), (10, \"force\")]",
            )],
            "(id: \"thug\", class: \"brigand\", level: 99)",
        );
        let t = load_src(&src);
        assert!(t.is_ok(), "{t:?}");
        let t = t.unwrap_or_default();
        let expected = CharacterDef {
            id: CharacterId("hero".into()),
            name: "N".into(),
            class: ClassId("swordsman".into()),
            level: 1,
            is_lord: false,
            talent: StatKind::Spd,
            base: Stats::from_growable([18, 5, 0, 7, 8, 3, 1], 5),
            weapon_ranks: BTreeMap::from([(WeaponKind::Sword, WeaponRank::D)]),
            personal_spells: vec![(1, SpellId("fire".into())), (10, SpellId("force".into()))],
        };
        assert_eq!(t.characters.get(&expected.id), Some(&expected));
        assert_eq!(
            t.generics.get("thug"),
            Some(&GenericTemplate {
                id: "thug".into(),
                class: ClassId("brigand".into()),
                level: 99,
            })
        );
    }

    #[test]
    fn without_classes_references_are_not_checked() {
        let src = file(
            &[character("hero", "nope", "level: 0")].map(|c| c.replace("level: 1, ", "")),
            "(id: \"g\", class: \"nope\", level: 0)",
        );
        let t = from_source("ch.ron", &src, None);
        assert!(t.is_ok(), "{t:?}");
        assert_eq!(
            t.ok()
                .and_then(|t| t.characters.values().next().map(|c| c.base.mov)),
            Some(0)
        );
    }

    #[test]
    fn syntax_and_unknown_names_are_positioned() {
        let src = file(&[character("hero", "swordsman", "")], "").replace("Spd", "Luck");
        let errs = load_src(&src).err().unwrap_or_default();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(4));
        assert!(errs[0].message.contains("Luck"));
    }

    #[test]
    fn unknown_classes() {
        let src = file(
            &[character("hero", "myrmidon", "")],
            "(id: \"g\", class: \"pirate\", level: 1)",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:4: character \"hero\": unknown class \"myrmidon\"",
                "ch.ron:8: generic \"g\": unknown class \"pirate\"",
            ]
        );
    }

    #[test]
    fn duplicate_ids() {
        let src = file(
            &[
                character("hero", "swordsman", ""),
                character("hero", "swordsman", ""),
            ],
            "(id: \"g\", class: \"brigand\", level: 1), (id: \"g\", class: \"brigand\", level: 1)",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:4: duplicate character id \"hero\"",
                "ch.ron:12: duplicate generic id \"g\"",
            ]
        );
    }

    #[test]
    fn level_range() {
        let src = file(
            &[character("a", "swordsman", "").replace("level: 1", "level: 0")],
            "(id: \"g\", class: \"brigand\", level: 100), (id: \"h\", class: \"brigand\", level: 99)",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:4: character \"a\": level 0 is outside 1..=99",
                "ch.ron:8: generic \"g\": level 100 is outside 1..=99",
            ]
        );
    }

    #[test]
    fn talent_is_not_mov() {
        let src = file(&[character("a", "swordsman", "")], "").replace("Spd", "Mov");
        assert_eq!(
            errors(&src),
            ["ch.ron:4: character \"a\": talent can't be Mov"]
        );
    }

    #[test]
    fn personal_spell_limits() {
        let src = file(
            &[
                character(
                    "a",
                    "swordsman",
                    "personal_spells: [(1, \"x\"), (2, \"y\"), (3, \"z\")]",
                ),
                character(
                    "lord",
                    "exile",
                    "is_lord: true, personal_spells: [(1, \"x\")]",
                ),
            ],
            "",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:4: character \"a\": 3 personal spells, at most 2",
                "ch.ron:8: character \"lord\": the lord has no personal spells",
            ]
        );
    }

    #[test]
    fn lord_only_classes() {
        let src = file(
            &[
                character("a", "exile", ""),
                character("lord", "exile", "is_lord: true"),
                // The lord may start in an ordinary class too.
                character("lord2", "swordsman", "is_lord: true"),
            ],
            "(id: \"g\", class: \"exile\", level: 1)",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:4: character \"a\": class \"exile\" is lord-only but is_lord is false",
                "ch.ron:16: generic \"g\": class \"exile\" is lord-only",
            ]
        );
    }

    #[test]
    fn base_within_class_caps() {
        let src = file(
            &[character("a", "swordsman", "").replace("base: (18, 5, 0,", "base: (41, 20, -1,")],
            "",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:4: character \"a\": base Hp 41 is over the \"swordsman\" cap 40",
                "ch.ron:4: character \"a\": base Mag -1 is negative",
            ]
        );
    }

    #[test]
    fn weapon_ranks_within_class_max() {
        let src = file(
            &[character("a", "swordsman", "").replace("[(Sword, D)]", "[(Sword, B), (Axe, S)]")],
            "",
        );
        // Axe isn't a swordsman kind, so its rank is kept for later.
        assert_eq!(
            errors(&src),
            ["ch.ron:4: character \"a\": Sword rank B is above the \"swordsman\" max rank C"]
        );
        let src = file(
            &[character("a", "swordsman", "").replace("[(Sword, D)]", "[(Sword, C)]")],
            "",
        );
        assert!(load_src(&src).is_ok());
    }

    #[test]
    fn empty_source_is_a_syntax_error() {
        assert_eq!(load_src("").err().map(|e| e.len()), Some(1));
    }

    /// The placeholder file loads, and every entry makes a valid unit.
    #[test]
    fn embedded_characters_load_and_make_units() {
        let classes = classes();
        let t = load(Some(&classes));
        assert!(t.is_ok(), "{t:?}");
        let t = t.unwrap_or_default();
        let ids: Vec<&str> = t.characters.keys().map(|c| c.0.as_str()).collect();
        assert_eq!(ids, ["test_archer", "test_knight", "test_lord"]);
        assert_eq!(t.generics.len(), 2);
        let lords: Vec<&str> = t
            .characters
            .values()
            .filter(|c| c.is_lord)
            .map(|c| c.id.0.as_str())
            .collect();
        assert_eq!(lords, ["test_lord"]);
        for def in t.characters.values() {
            let unit =
                Unit::from_character(UnitId(0), def, &classes, Faction::Player, Pos::new(0, 0));
            assert!(unit.is_ok(), "{}", def.id.0);
        }
        for g in t.generics.values() {
            let unit = Unit::generic(
                UnitId(0),
                &g.class,
                &classes,
                g.level,
                Faction::Enemy,
                Pos::new(0, 0),
            );
            assert!(unit.is_ok(), "{}", g.id);
        }
    }
}
