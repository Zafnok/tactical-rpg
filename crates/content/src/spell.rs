//! Spells (`assets/data/spells.ron`), from `docs/design/magic.md`.

use std::collections::BTreeMap;

use serde::Deserialize;
use trpg_core::{
    ClassTable, Element, SpellDef, SpellId, SpellKind, SpellTable, StatValue, TerrainEffectId,
    UnitTag,
};

use crate::bundle;
use crate::character::{CHARACTERS_PATH, CharacterTable};
use crate::class::CLASSES_PATH;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the spell file inside the asset bundle.
pub const SPELLS_PATH: &str = "data/spells.ron";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    spells: Vec<RawSpell>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpell {
    id: String,
    name: String,
    kind: RawKind,
    element: Element,
    range: (u32, u32),
    uses: u8,
    #[serde(default)]
    terrain_effect: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum RawKind {
    Attack {
        might: StatValue,
        hit: StatValue,
        crit: StatValue,
        #[serde(default)]
        effective: Vec<(UnitTag, u8)>,
    },
    Heal {
        heal_power: StatValue,
    },
}

/// Loads and validates the embedded spell file.
pub fn load() -> Result<SpellTable, Vec<ContentError>> {
    let display = bundle::display_path(SPELLS_PATH);
    let source = bundle::file(SPELLS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Parses and validates spell `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<SpellTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut spells = BTreeMap::new();
    for s in raw.spells {
        let at = |message: String| {
            let e = ContentError::new(file, message);
            match line_of(source, &format!("id: \"{}\"", s.id)) {
                Some(l) => e.at(l, None),
                None => e,
            }
        };
        let what = format!("spell \"{}\"", s.id);
        if s.id.is_empty() {
            errors.push(at("a spell id is empty".into()));
        }
        let (min_range, max_range) = s.range;
        if min_range < 1 || min_range > max_range {
            errors.push(at(format!(
                "{what}: range ({min_range}, {max_range}) must have 1 <= min <= max"
            )));
        }
        if s.uses == 0 {
            errors.push(at(format!("{what}: uses must be at least 1")));
        }
        let kind = match s.kind {
            RawKind::Attack {
                might,
                hit,
                crit,
                effective,
            } => {
                if [might, hit, crit].iter().any(|&v| v < 0) {
                    errors.push(at(format!("{what}: might, hit and crit can't be negative")));
                }
                SpellKind::Attack {
                    might,
                    hit,
                    crit,
                    effective,
                }
            }
            RawKind::Heal { heal_power } => {
                if heal_power < 1 {
                    errors.push(at(format!("{what}: heal_power must be at least 1")));
                }
                SpellKind::Heal { heal_power }
            }
        };
        let id = SpellId(s.id.clone());
        let def = SpellDef {
            id: id.clone(),
            name: s.name,
            kind,
            element: s.element,
            min_range,
            max_range,
            uses: s.uses,
            terrain_effect: s.terrain_effect.map(TerrainEffectId),
        };
        if spells.insert(id, def).is_some() {
            errors.push(at(format!("duplicate spell id \"{}\"", s.id)));
        }
    }
    if errors.is_empty() {
        Ok(SpellTable { spells })
    } else {
        Err(errors)
    }
}

/// Checks that every class spell and personal spell names a spell in
/// `spells`. Errors point at the class or character in its own file.
pub fn check_references(
    spells: &SpellTable,
    classes: &ClassTable,
    characters: &CharacterTable,
) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let mut check = |path: &str, owner: &str, what: String, spell: &SpellId| {
        if spells.get(spell).is_some() {
            return;
        }
        let display = bundle::display_path(path);
        let e = ContentError::new(&display, format!("{what}: unknown spell \"{}\"", spell.0));
        let line = bundle::file(path).and_then(|src| line_of(src, &format!("id: \"{owner}\"")));
        errors.push(match line {
            Some(l) => e.at(l, None),
            None => e,
        });
    };
    for (id, class) in &classes.classes {
        for (_, spell) in &class.spells {
            check(CLASSES_PATH, &id.0, format!("class \"{}\"", id.0), spell);
        }
    }
    for (id, character) in &characters.characters {
        for (_, spell) in &character.personal_spells {
            check(
                CHARACTERS_PATH,
                &id.0,
                format!("character \"{}\"", id.0),
                spell,
            );
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use trpg_core::{CharacterDef, CharacterId, ClassDef, ClassId};

    use super::*;

    fn spells() -> SpellTable {
        let t = load();
        assert!(t.is_ok(), "{t:?}");
        t.unwrap_or_default()
    }

    fn get(id: &str) -> Option<SpellDef> {
        spells().get(&SpellId::new(id)).cloned()
    }

    fn attack(
        id: &str,
        name: &str,
        element: Element,
        [might, hit, crit]: [StatValue; 3],
        uses: u8,
        effect: Option<&str>,
    ) -> SpellDef {
        SpellDef {
            id: SpellId::new(id),
            name: name.into(),
            kind: SpellKind::Attack {
                might,
                hit,
                crit,
                effective: vec![],
            },
            element,
            min_range: 1,
            max_range: 2,
            uses,
            terrain_effect: effect.map(|e| TerrainEffectId(e.into())),
        }
    }

    fn heal(id: &str, name: &str, heal_power: StatValue, uses: u8) -> SpellDef {
        SpellDef {
            id: SpellId::new(id),
            name: name.into(),
            kind: SpellKind::Heal { heal_power },
            element: Element::None,
            min_range: 1,
            max_range: 1,
            uses,
            terrain_effect: None,
        }
    }

    /// Every starter spell, exactly as in `magic.md`'s table.
    #[test]
    fn spells_match_the_design() {
        let design = [
            attack(
                "fire",
                "Fire",
                Element::Fire,
                [5, 90, 0],
                10,
                Some("burn_forest"),
            ),
            attack(
                "frost",
                "Frost",
                Element::Ice,
                [4, 95, 0],
                10,
                Some("freeze_water"),
            ),
            attack("force", "Force", Element::None, [6, 80, 5], 8, None),
            heal("heal", "Heal", 10, 8),
            heal("mend", "Mend", 20, 4),
        ];
        for def in &design {
            assert_eq!(get(&def.id.0).as_ref(), Some(def));
        }
        assert_eq!(spells().spells.len(), design.len());
    }

    fn check(src: &str) -> Vec<String> {
        match from_source("s.ron", src) {
            Ok(_) => vec![],
            Err(e) => e.iter().map(ToString::to_string).collect(),
        }
    }

    fn one(fields: &str) -> String {
        format!("(spells: [\n(\n{fields}\n),\n])")
    }

    const OK: &str = "id: \"x\", name: \"X\", kind: Attack(might: 1, hit: 2, crit: 3, effective: [(Flying, 2)]), element: Ice, range: (1, 2), uses: 3";

    #[test]
    fn parses_every_field() {
        let t = from_source("s.ron", &one(OK)).unwrap_or_default();
        assert_eq!(
            t.get(&SpellId::new("x")),
            Some(&SpellDef {
                id: SpellId::new("x"),
                name: "X".into(),
                kind: SpellKind::Attack {
                    might: 1,
                    hit: 2,
                    crit: 3,
                    effective: vec![(UnitTag::Flying, 2)],
                },
                element: Element::Ice,
                min_range: 1,
                max_range: 2,
                uses: 3,
                terrain_effect: None,
            })
        );
    }

    #[test]
    fn validation_errors() {
        let bad = |fields: &str, msg: &str| {
            assert_eq!(check(&one(fields)), [format!("s.ron:3: {msg}")], "{fields}");
        };
        let with = |from: &str, to: &str| OK.replace(from, to);
        bad(
            &with("range: (1, 2)", "range: (0, 2)"),
            "spell \"x\": range (0, 2) must have 1 <= min <= max",
        );
        bad(
            &with("range: (1, 2)", "range: (3, 2)"),
            "spell \"x\": range (3, 2) must have 1 <= min <= max",
        );
        bad(
            &with("uses: 3", "uses: 0"),
            "spell \"x\": uses must be at least 1",
        );
        for field in ["might: 1", "hit: 2", "crit: 3"] {
            let negative = field.replace(": ", ": -");
            bad(
                &with(field, &negative),
                "spell \"x\": might, hit and crit can't be negative",
            );
        }
        let heal = "id: \"x\", name: \"X\", kind: Heal(heal_power: 0), element: None, range: (1, 1), uses: 1";
        bad(heal, "spell \"x\": heal_power must be at least 1");
        bad(&with("id: \"x\"", "id: \"\""), "a spell id is empty");
        assert!(check(&one(&heal.replace("power: 0", "power: 1"))).is_empty());
        // Boundaries that are fine: min == max, 1 use, zeros.
        assert!(
            check(&one(
                &with("range: (1, 2)", "range: (2, 2)").replace("uses: 3", "uses: 1")
            ))
            .is_empty()
        );
        assert!(
            check(&one(&with("might: 1", "might: 0")
                .replace("hit: 2", "hit: 0")
                .replace("crit: 3", "crit: 0")))
            .is_empty()
        );
    }

    #[test]
    fn duplicate_ids_and_parse_errors() {
        let src = format!("(spells: [\n({OK}),\n({OK}),\n])");
        assert_eq!(check(&src), ["s.ron:2: duplicate spell id \"x\""]);
        let errs = check("(spells: [(id: \"x\")])");
        assert_eq!(errs.len(), 1);
        assert!(errs[0].starts_with("s.ron:1:"), "{errs:?}");
    }

    fn class(id: &str, spells: &[&str]) -> ClassDef {
        let classes = crate::class::load(None).unwrap_or_default();
        let any = classes.classes.values().next().cloned();
        ClassDef {
            id: ClassId(id.into()),
            spells: spells.iter().map(|s| (1, SpellId::new(s))).collect(),
            ..any.unwrap_or_else(|| panic!("no classes"))
        }
    }

    #[test]
    fn references_must_exist() {
        let t = spells();
        let classes = ClassTable {
            classes: [
                class("ok", &["fire", "heal"]),
                class("broken", &["fire", "zap"]),
            ]
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect(),
            ..ClassTable::default()
        };
        let mut characters = CharacterTable::default();
        let base = crate::character::load(None, None)
            .unwrap_or_default()
            .characters
            .into_values()
            .next()
            .unwrap_or_else(|| panic!("no characters"));
        for (id, spells) in [("fine", vec!["mend"]), ("odd", vec!["mend", "glow"])] {
            let def = CharacterDef {
                id: CharacterId(id.into()),
                personal_spells: spells.into_iter().map(|s| (1, SpellId::new(s))).collect(),
                ..base.clone()
            };
            characters.characters.insert(def.id.clone(), def);
        }
        let errors: Vec<String> = check_references(&t, &classes, &characters)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            errors,
            [
                "assets/data/classes.ron: class \"broken\": unknown spell \"zap\"",
                "assets/data/characters.ron: character \"odd\": unknown spell \"glow\"",
            ]
        );
        // Positioned at the owner's line when it is in the file.
        let real = crate::class::load(None).unwrap_or_default();
        let (id, mage) = real
            .classes
            .iter()
            .find(|(_, c)| !c.spells.is_empty())
            .unwrap_or_else(|| panic!("no class with spells"));
        let errors = check_references(&SpellTable::default(), &real, &CharacterTable::default());
        let first = errors.first().map(ToString::to_string).unwrap_or_default();
        let line = bundle::file(CLASSES_PATH)
            .and_then(|src| line_of(src, &format!("id: \"{}\"", id.0)))
            .unwrap_or(0);
        assert!(!mage.spells.is_empty());
        assert!(
            first.starts_with(&format!("assets/data/classes.ron:{line}: ")),
            "{first}"
        );
    }

    #[test]
    fn the_embedded_references_all_exist() {
        let classes = crate::class::load(None).unwrap_or_default();
        let characters = crate::character::load(Some(&classes), None).unwrap_or_default();
        assert!(!classes.classes.is_empty());
        assert_eq!(check_references(&spells(), &classes, &characters), vec![]);
    }
}
