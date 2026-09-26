//! Named characters and generic unit templates
//! (`assets/data/characters.ron`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::{
    CharacterDef, CharacterId, ClassDef, ClassId, ClassTable, Faction, ItemId, ItemTable, Level,
    LoadoutDef, Pos, SpellId, StatKind, StatValue, Stats, Unit, UnitError, UnitId,
    is_valid_map_label,
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
    /// Overrides the default map label (the class name's first two letters).
    pub map_label: Option<String>,
    /// Starting loadout.
    pub loadout: LoadoutDef,
}

impl GenericTemplate {
    /// A unit made from this template (see [`Unit::generic`]), with the
    /// template's map label override and loadout applied.
    pub fn unit(
        &self,
        id: UnitId,
        classes: &ClassTable,
        items: &ItemTable,
        faction: Faction,
        pos: Pos,
    ) -> Result<Unit, UnitError> {
        let mut unit = Unit::generic(id, &self.class, classes, self.level, faction, pos)?;
        if let Some(label) = &self.map_label {
            unit.map_label.clone_from(label);
        }
        Ok(unit.with_loadout(&self.loadout, classes, items)?)
    }
}

/// The unit of named character `def` (see [`Unit::from_character`]) with its
/// starting loadout.
pub fn character_unit(
    def: &CharacterDef,
    id: UnitId,
    classes: &ClassTable,
    items: &ItemTable,
    faction: Faction,
    pos: Pos,
) -> Result<Unit, UnitError> {
    let unit = Unit::from_character(id, def, classes, faction, pos)?;
    Ok(unit.with_loadout(&def.loadout, classes, items)?)
}

/// Checks the map labels of the units placed on one map (`map`, a file
/// name for the error): two named characters of the same faction must not
/// share a label (ADR-0018). Generic units may: a map with three Brigands
/// shows three `Br`s. Returns one error per clash.
pub fn check_map_labels(map: &str, units: &[Unit]) -> Vec<ContentError> {
    let mut seen: BTreeMap<(Faction, &str), &str> = BTreeMap::new();
    let mut errors = Vec::new();
    for u in units.iter().filter(|u| u.character.is_some()) {
        match seen.get(&(u.faction, u.map_label.as_str())) {
            Some(first) => errors.push(ContentError::new(
                map,
                format!(
                    "{first} and {} are both labelled \"{}\" on the map; give one a map_label",
                    u.name, u.map_label
                ),
            )),
            None => {
                seen.insert((u.faction, u.map_label.as_str()), u.name.as_str());
            }
        }
    }
    errors
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
    #[serde(default)]
    map_label: Option<String>,
    #[serde(default)]
    loadout: RawLoadout,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawLoadout {
    #[serde(default)]
    weapons: Vec<String>,
    #[serde(default)]
    armour: Option<String>,
    #[serde(default)]
    accessory: Option<String>,
}

impl RawLoadout {
    fn to_def(&self) -> LoadoutDef {
        LoadoutDef {
            weapons: self.weapons.iter().map(|w| ItemId::new(w)).collect(),
            armour: self.armour.as_deref().map(ItemId::new),
            accessory: self.accessory.as_deref().map(ItemId::new),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeneric {
    id: String,
    class: String,
    level: Level,
    #[serde(default)]
    map_label: Option<String>,
    #[serde(default)]
    loadout: RawLoadout,
}

/// Loads and validates the embedded character file. Class references are
/// checked against `classes` when given (skipped if the class file failed to
/// load); loadouts against `classes` and `items` when both are given.
pub fn load(
    classes: Option<&ClassTable>,
    items: Option<&ItemTable>,
) -> Result<CharacterTable, Vec<ContentError>> {
    let display = bundle::display_path(CHARACTERS_PATH);
    let source = bundle::file(CHARACTERS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source, classes, items)
}

/// Parses and validates character `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(
    file: &str,
    source: &str,
    classes: Option<&ClassTable>,
    items: Option<&ItemTable>,
) -> Result<CharacterTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut v = Validator {
        file,
        source,
        classes,
        items,
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
        v.map_label(&g.id, &what, g.map_label.as_deref());
        let template = GenericTemplate {
            id: g.id.clone(),
            class: ClassId(g.class.clone()),
            level: g.level,
            map_label: g.map_label.clone(),
            loadout: g.loadout.to_def(),
        };
        if let (Some(classes), Some(items)) = (v.classes, v.items)
            && let Err(UnitError::Loadout(e)) =
                template.unit(UnitId(0), classes, items, Faction::Enemy, Pos::new(0, 0))
        {
            v.err(&g.id, format!("{what}: loadout: {e}"));
        }
        table.generics.insert(g.id.clone(), template);
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
    items: Option<&'a ItemTable>,
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

    /// Checks a map label override is exactly two letters.
    fn map_label(&mut self, id: &str, what: &str, label: Option<&str>) {
        if let Some(label) = label.filter(|l| !is_valid_map_label(l)) {
            self.err(
                id,
                format!("{what}: map_label \"{label}\" must be exactly two letters"),
            );
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
        self.map_label(&c.id, &what, c.map_label.as_deref());
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
            map_label: c.map_label.clone(),
            loadout: c.loadout.to_def(),
        };
        if let Some(class) = class {
            self.against_class(&def, &what, class);
        }
        if let (Some(classes), Some(items)) = (self.classes, self.items)
            && let Err(UnitError::Loadout(e)) = character_unit(
                &def,
                UnitId(0),
                classes,
                items,
                Faction::Player,
                Pos::new(0, 0),
            )
        {
            self.err(&c.id, format!("{what}: loadout: {e}"));
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

    fn items() -> ItemTable {
        crate::item::load().unwrap_or_default()
    }

    fn load_src(src: &str) -> Result<CharacterTable, Vec<ContentError>> {
        from_source("ch.ron", src, Some(&classes()), Some(&items()))
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
            map_label: None,
            loadout: LoadoutDef::default(),
        };
        assert_eq!(t.characters.get(&expected.id), Some(&expected));
        assert_eq!(
            t.generics.get("thug"),
            Some(&GenericTemplate {
                id: "thug".into(),
                class: ClassId("brigand".into()),
                level: 99,
                map_label: None,
                loadout: LoadoutDef::default(),
            })
        );
    }

    #[test]
    fn without_classes_references_are_not_checked() {
        let src = file(
            &[character("hero", "nope", "level: 0")].map(|c| c.replace("level: 1, ", "")),
            "(id: \"g\", class: \"nope\", level: 0)",
        );
        let t = from_source("ch.ron", &src, None, Some(&items()));
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
    fn map_label_overrides() {
        let src = file(
            &[
                character("a", "swordsman", "map_label: Some(\"Xy\")"),
                character("b", "swordsman", "map_label: Some(\"X\")"),
                character("c", "swordsman", "map_label: Some(\"X1\")"),
            ],
            "(id: \"g\", class: \"brigand\", level: 1, map_label: Some(\"Bg\")), \
             (id: \"h\", class: \"brigand\", level: 1, map_label: Some(\"Bgg\"))",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:8: character \"b\": map_label \"X\" must be exactly two letters",
                "ch.ron:12: character \"c\": map_label \"X1\" must be exactly two letters",
                "ch.ron:16: generic \"h\": map_label \"Bgg\" must be exactly two letters",
            ]
        );
        let src = file(
            &[character("a", "swordsman", "map_label: Some(\"Xy\")")],
            "(id: \"g\", class: \"brigand\", level: 1, map_label: Some(\"Bg\"))",
        );
        let t = load_src(&src).unwrap_or_default();
        let def = t.characters.get(&CharacterId("a".into()));
        assert_eq!(def.and_then(|d| d.map_label.as_deref()), Some("Xy"));
        let g = t.generics.get("g");
        assert_eq!(g.and_then(|g| g.map_label.as_deref()), Some("Bg"));
    }

    #[test]
    fn generic_template_units_use_the_label_override() {
        let classes = classes();
        let items = items();
        let mut t = GenericTemplate {
            id: "g".into(),
            class: ClassId("brigand".into()),
            level: 3,
            map_label: None,
            loadout: LoadoutDef::default(),
        };
        let make = |t: &GenericTemplate| {
            t.unit(UnitId(4), &classes, &items, Faction::Enemy, Pos::new(1, 2))
        };
        let expected = Unit::generic(
            UnitId(4),
            &t.class,
            &classes,
            3,
            Faction::Enemy,
            Pos::new(1, 2),
        );
        assert_eq!(make(&t), expected);
        assert_eq!(make(&t).map(|u| u.map_label), Ok("Br".to_owned()));
        t.map_label = Some("Zz".into());
        assert_eq!(make(&t).map(|u| u.map_label), Ok("Zz".to_owned()));
        t.class = ClassId("nope".into());
        assert_eq!(
            make(&t),
            Err(UnitError::UnknownClass(ClassId("nope".into())))
        );
    }

    #[test]
    fn map_label_clashes_between_named_units() {
        let classes = classes();
        let items = items();
        let t = load(Some(&classes), Some(&items)).unwrap_or_default();
        let named = |label: &str, n: u32, faction| {
            let def = &t.characters[&CharacterId("test_knight".into())];
            Unit::from_character(UnitId(n), def, &classes, faction, Pos::new(0, 0))
                .ok()
                .map(|mut u| {
                    u.map_label = label.into();
                    u.name = format!("K{n}");
                    u
                })
        };
        let generic = |n: u32| {
            t.generics["test_brigand"]
                .unit(UnitId(n), &classes, &items, Faction::Enemy, Pos::new(0, 0))
                .ok()
        };
        let ok: Vec<Unit> = [
            named("Ab", 0, Faction::Player),
            named("Ab", 1, Faction::Enemy),
            named("Cd", 2, Faction::Player),
            generic(3),
            generic(4),
        ]
        .into_iter()
        .flatten()
        .collect();
        assert_eq!(ok.len(), 5);
        assert!(check_map_labels("m.map", &ok).is_empty());
        let mut clash = ok;
        clash.extend(named("Ab", 5, Faction::Player));
        clash.extend(named("Ab", 6, Faction::Player));
        let msgs: Vec<String> = check_map_labels("m.map", &clash)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            msgs,
            [
                "m.map: K0 and K5 are both labelled \"Ab\" on the map; give one a map_label",
                "m.map: K0 and K6 are both labelled \"Ab\" on the map; give one a map_label",
            ]
        );
    }

    #[test]
    fn empty_source_is_a_syntax_error() {
        assert_eq!(load_src("").err().map(|e| e.len()), Some(1));
    }

    #[test]
    fn loadouts_are_checked_against_classes_and_items() {
        let src = file(
            &[
                character("a", "swordsman", "loadout: (weapons: [\"iron_sword\"])"),
                character("b", "swordsman", "loadout: (weapons: [\"nope\"])"),
                character("c", "swordsman", "loadout: (armour: Some(\"iron_plate\"))"),
                character("d", "swordsman", "loadout: (accessory: Some(\"potion\"))"),
            ],
            "(id: \"g\", class: \"brigand\", level: 1, loadout: (weapons: [\"iron_axe\", \"iron_axe\", \"iron_axe\", \"iron_axe\"]))",
        );
        assert_eq!(
            errors(&src),
            [
                "ch.ron:8: character \"b\": loadout: unknown item \"nope\"",
                "ch.ron:12: character \"c\": loadout: the class can't wear \"iron_plate\"",
                "ch.ron:16: character \"d\": loadout: \"potion\" doesn't go in that slot",
                "ch.ron:20: generic \"g\": loadout: 4 weapons, but the class has 3 weapon slots",
            ]
        );
        // Without the item table, loadouts aren't checked.
        assert!(from_source("ch.ron", &src, Some(&classes()), None).is_ok());
        let t = load_src(&file(
            &[character(
                "a",
                "swordsman",
                "loadout: (weapons: [\"iron_sword\"], armour: Some(\"leather_vest\"), accessory: Some(\"speed_ring\"))",
            )],
            "",
        ))
        .unwrap_or_default();
        let def = &t.characters[&CharacterId("a".into())];
        assert_eq!(
            def.loadout,
            LoadoutDef {
                weapons: vec![ItemId::new("iron_sword")],
                armour: Some(ItemId::new("leather_vest")),
                accessory: Some(ItemId::new("speed_ring")),
            }
        );
        let unit = character_unit(
            def,
            UnitId(1),
            &classes(),
            &items(),
            Faction::Player,
            Pos::new(0, 0),
        );
        assert_eq!(unit.map(|u| u.loadout.equipped), Ok(Some(0)));
    }

    /// The placeholder file loads, and every entry makes a valid unit with
    /// its loadout.
    #[test]
    fn embedded_characters_load_and_make_units() {
        let classes = classes();
        let items = items();
        let t = load(Some(&classes), Some(&items));
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
            let unit = character_unit(
                def,
                UnitId(0),
                &classes,
                &items,
                Faction::Player,
                Pos::new(0, 0),
            );
            assert!(
                unit.is_ok_and(|u| u.loadout.equipped.is_some()),
                "{}",
                def.id.0
            );
        }
        for g in t.generics.values() {
            let unit = g.unit(UnitId(0), &classes, &items, Faction::Enemy, Pos::new(0, 0));
            assert!(unit.is_ok_and(|u| u.loadout.equipped.is_some()), "{}", g.id);
        }
    }
}
