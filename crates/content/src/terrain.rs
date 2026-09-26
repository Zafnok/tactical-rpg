//! Terrain types (`assets/data/terrain.ron`): the rules half becomes a
//! [`trpg_core::TerrainTable`], the look half a [`TerrainDisplayTable`].

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::{TerrainId, TerrainRules, TerrainTable};

use crate::bundle;
use crate::error::ContentError;
use crate::font::REQUIRED_GLYPHS;
use crate::palette::PaletteDef;
use crate::ron_loader::parse_ron;

/// Path of the terrain table inside the asset bundle.
pub const TERRAIN_PATH: &str = "data/terrain.ron";

/// How one terrain is drawn. Index = [`TerrainId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainDisplay {
    /// String id used by map legends, e.g. `"forest"`.
    pub id: String,
    /// The two glyphs of a map tile (ADR-0012).
    pub glyphs: [char; 2],
    /// Foreground palette colour name.
    pub fg: String,
    /// Background palette colour name.
    pub bg: String,
}

/// Every terrain's string id and look, indexed by [`TerrainId`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerrainDisplayTable {
    /// Display data; index = [`TerrainId`].
    pub terrains: Vec<TerrainDisplay>,
}

impl TerrainDisplayTable {
    /// The id of the terrain whose string id is `id`.
    pub fn id_of(&self, id: &str) -> Option<TerrainId> {
        let index = self.terrains.iter().position(|t| t.id == id)?;
        u16::try_from(index).ok().map(TerrainId)
    }

    /// The display data of terrain `id`.
    pub fn get(&self, id: TerrainId) -> Option<&TerrainDisplay> {
        self.terrains.get(usize::from(id.0))
    }
}

/// The validated terrain file: rules for `core`, looks for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerrainDef {
    /// Movement types and terrain rules.
    pub rules: TerrainTable,
    /// String ids, glyphs and colours, in the same order as `rules`.
    pub display: TerrainDisplayTable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    movement_types: Vec<String>,
    terrains: Vec<RawTerrain>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTerrain {
    id: String,
    name: String,
    glyphs: String,
    fg: String,
    bg: String,
    move_cost: BTreeMap<String, Option<u8>>,
    defense: i8,
    avoid: i8,
    heal_percent: u8,
}

impl TerrainDef {
    /// Loads and validates the embedded terrain table. Colour names are
    /// checked against `palette` when it is given (skipped if the palette
    /// itself failed to load, to avoid a flood of follow-on errors).
    pub fn load(palette: Option<&PaletteDef>) -> Result<Self, Vec<ContentError>> {
        let display = bundle::display_path(TERRAIN_PATH);
        let source = bundle::file(TERRAIN_PATH).ok_or_else(|| {
            vec![ContentError::new(
                &display,
                "file not found in asset bundle",
            )]
        })?;
        Self::from_source(&display, source, palette)
    }

    /// Parses and validates terrain `source`, attributing errors to `file`.
    /// Reports every problem found.
    pub fn from_source(
        file: &str,
        source: &str,
        palette: Option<&PaletteDef>,
    ) -> Result<Self, Vec<ContentError>> {
        let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
        let mut v = Validator {
            file,
            source,
            palette,
            errors: Vec::new(),
        };
        v.check_movement_types(&raw.movement_types);
        if raw.terrains.len() > usize::from(u16::MAX) + 1 {
            v.err(
                line_of(source, "terrains"),
                "more than 65536 terrains".into(),
            );
        }
        let mut seen_ids = BTreeSet::new();
        let mut rules = Vec::new();
        let mut display = Vec::new();
        for t in &raw.terrains {
            if !seen_ids.insert(t.id.as_str()) {
                v.err(
                    v.line_of_terrain(&t.id),
                    format!("duplicate terrain id \"{}\"", t.id),
                );
            }
            let (r, d) = v.terrain(t, &raw.movement_types);
            rules.push(r);
            display.push(d);
        }
        if v.errors.is_empty() {
            Ok(Self {
                rules: TerrainTable {
                    movement_types: raw.movement_types,
                    terrains: rules,
                },
                display: TerrainDisplayTable { terrains: display },
            })
        } else {
            Err(v.errors)
        }
    }
}

/// Collects validation errors for one terrain file.
struct Validator<'a> {
    file: &'a str,
    source: &'a str,
    palette: Option<&'a PaletteDef>,
    errors: Vec<ContentError>,
}

impl Validator<'_> {
    fn err(&mut self, line: Option<u32>, message: String) {
        let e = ContentError::new(self.file, message);
        self.errors.push(match line {
            Some(l) => e.at(l, None),
            None => e,
        });
    }

    fn line_of_terrain(&self, id: &str) -> Option<u32> {
        line_of(self.source, &format!("id: \"{id}\""))
    }

    fn check_movement_types(&mut self, types: &[String]) {
        let line = line_of(self.source, "movement_types");
        if types.is_empty() {
            self.err(line, "movement_types must not be empty".into());
        }
        if types.len() > usize::from(u8::MAX) + 1 {
            self.err(line, "more than 256 movement types".into());
        }
        let mut seen = BTreeSet::new();
        for mt in types {
            if !seen.insert(mt.as_str()) {
                self.err(line, format!("duplicate movement type \"{mt}\""));
            }
        }
    }

    /// Validates one terrain entry and converts it (placeholder values stand
    /// in for invalid fields; the caller discards them if any error exists).
    fn terrain(&mut self, t: &RawTerrain, types: &[String]) -> (TerrainRules, TerrainDisplay) {
        let line = self.line_of_terrain(&t.id);
        let what = format!("terrain \"{}\"", t.id);
        let glyphs = self.glyphs(line, &what, &t.glyphs);
        if let Some(palette) = self.palette {
            for (field, colour) in [("fg", &t.fg), ("bg", &t.bg)] {
                if palette.get(colour).is_none() {
                    self.err(
                        line,
                        format!("{what}: {field} colour \"{colour}\" is not in palette.ron"),
                    );
                }
            }
        }
        let move_cost = self.move_cost(line, &what, &t.move_cost, types);
        if t.heal_percent > 100 {
            self.err(
                line,
                format!("{what}: heal_percent {} is over 100", t.heal_percent),
            );
        }
        let rules = TerrainRules {
            name: t.name.clone(),
            move_cost,
            defense: t.defense,
            avoid: t.avoid,
            heal_percent: t.heal_percent,
        };
        let display = TerrainDisplay {
            id: t.id.clone(),
            glyphs,
            fg: t.fg.clone(),
            bg: t.bg.clone(),
        };
        (rules, display)
    }

    fn glyphs(&mut self, line: Option<u32>, what: &str, text: &str) -> [char; 2] {
        for g in text.chars().filter(|&g| !REQUIRED_GLYPHS.contains(g)) {
            self.err(
                line,
                format!("{what}: glyph '{g}' is not in the font atlas"),
            );
        }
        let chars: Vec<char> = text.chars().collect();
        if let [a, b] = chars[..] {
            [a, b]
        } else {
            self.err(
                line,
                format!(
                    "{what}: glyphs \"{text}\" must be exactly 2 characters, found {}",
                    chars.len()
                ),
            );
            [' ', ' ']
        }
    }

    fn move_cost(
        &mut self,
        line: Option<u32>,
        what: &str,
        costs: &BTreeMap<String, Option<u8>>,
        types: &[String],
    ) -> Vec<Option<u8>> {
        for name in costs.keys().filter(|&k| !types.contains(k)) {
            self.err(
                line,
                format!("{what}: move_cost has unknown movement type \"{name}\""),
            );
        }
        types
            .iter()
            .map(|mt| {
                let cost = costs.get(mt);
                match cost {
                    None => self.err(
                        line,
                        format!("{what}: move_cost is missing movement type \"{mt}\""),
                    ),
                    Some(Some(0)) => self.err(
                        line,
                        format!(
                            "{what}: move_cost for \"{mt}\" must be at least 1 (use None for impassable)"
                        ),
                    ),
                    Some(_) => {}
                }
                cost.copied().flatten()
            })
            .collect()
    }
}

/// 1-based line of the first line containing `needle`.
fn line_of(source: &str, needle: &str) -> Option<u32> {
    let index = source.lines().position(|l| l.contains(needle))?;
    u32::try_from(index + 1).ok()
}

#[cfg(test)]
mod tests {
    use trpg_core::MovementTypeId;

    use super::*;

    const COSTS: &str = r#"{ "foot": Some(1), "flying": Some(1) }"#;

    fn terrain(id: &str, glyphs: &str, fg: &str, costs: &str, heal: u8) -> String {
        format!(
            "        (id: \"{id}\", name: \"N\", glyphs: \"{glyphs}\", fg: \"{fg}\", bg: \"black\",\n            move_cost: {costs}, defense: 1, avoid: -5, heal_percent: {heal}),\n"
        )
    }

    fn file(types: &str, terrains: &[String]) -> String {
        format!(
            "(\n    movement_types: [{types}],\n    terrains: [\n{}    ],\n)",
            terrains.concat()
        )
    }

    fn palette() -> PaletteDef {
        let mut p = PaletteDef::default();
        p.colors.insert("black".into(), [0, 0, 0]);
        p.colors.insert("grass".into(), [0, 255, 0]);
        p
    }

    fn errors(src: &str) -> Vec<ContentError> {
        TerrainDef::from_source("t.ron", src, Some(&palette()))
            .err()
            .unwrap_or_default()
    }

    const TYPES: &str = r#""foot", "flying""#;

    #[test]
    fn valid_file_loads() {
        let src = file(
            TYPES,
            &[
                terrain("plain", "..", "grass", COSTS, 0),
                terrain(
                    "sea",
                    "≈≈",
                    "grass",
                    r#"{ "flying": Some(1), "foot": None }"#,
                    100,
                ),
            ],
        );
        let t = TerrainDef::from_source("t.ron", &src, Some(&palette())).unwrap_or_default();
        assert_eq!(t.rules.movement_types, ["foot", "flying"]);
        assert_eq!(t.display.id_of("sea"), Some(TerrainId(1)));
        assert_eq!(t.display.id_of("plain"), Some(TerrainId(0)));
        assert_eq!(t.display.id_of("lava"), None);
        let sea = t.rules.get(TerrainId(1));
        assert_eq!(sea.map(|r| r.move_cost.clone()), Some(vec![None, Some(1)]));
        assert_eq!(
            sea.map(|r| (r.defense, r.avoid, r.heal_percent)),
            Some((1, -5, 100))
        );
        assert_eq!(sea.map(|r| r.name.as_str()), Some("N"));
        assert_eq!(t.rules.move_cost(TerrainId(1), MovementTypeId(0)), None);
        let d = t.display.get(TerrainId(1));
        assert_eq!(d.map(|d| d.glyphs), Some(['≈', '≈']));
        assert_eq!(
            d.map(|d| (d.fg.as_str(), d.bg.as_str())),
            Some(("grass", "black"))
        );
        assert_eq!(t.display.get(TerrainId(2)), None);
    }

    #[test]
    fn syntax_error_is_positioned() {
        let errs = errors("(\n movement_types: [\"foot\"]\n terrains: [],\n)");
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].line, Some(3));
    }

    #[test]
    fn bad_movement_types() {
        let errs = errors(&file("", &[]));
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].to_string(),
            "t.ron:2: movement_types must not be empty"
        );
        let errs = errors(&file(r#""foot", "foot""#, &[]));
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].to_string(),
            "t.ron:2: duplicate movement type \"foot\""
        );
        let many: Vec<String> = (0..=256).map(|i| format!("\"m{i}\"")).collect();
        let errs = errors(&file(&many.join(", "), &[]));
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("more than 256"));
        let max: Vec<String> = (0..256).map(|i| format!("\"m{i}\"")).collect();
        assert!(errors(&file(&max.join(", "), &[])).is_empty());
    }

    #[test]
    fn duplicate_terrain_id() {
        let src = file(
            TYPES,
            &[
                terrain("plain", "..", "grass", COSTS, 0),
                terrain("plain", "..", "grass", COSTS, 0),
            ],
        );
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].to_string(),
            "t.ron:4: duplicate terrain id \"plain\""
        );
    }

    #[test]
    fn glyphs_must_be_two_atlas_characters() {
        let src = file(
            TYPES,
            &[
                terrain("a", ".", "grass", COSTS, 0),
                terrain("b", "...", "grass", COSTS, 0),
                terrain("c", ".€", "grass", COSTS, 0),
            ],
        );
        let errs = errors(&src);
        let text: Vec<String> = errs.iter().map(ToString::to_string).collect();
        assert_eq!(
            text,
            [
                "t.ron:4: terrain \"a\": glyphs \".\" must be exactly 2 characters, found 1",
                "t.ron:6: terrain \"b\": glyphs \"...\" must be exactly 2 characters, found 3",
                "t.ron:8: terrain \"c\": glyph '€' is not in the font atlas",
            ]
        );
    }

    #[test]
    fn colours_must_be_in_palette() {
        let src = file(TYPES, &[terrain("a", "..", "lava", COSTS, 0)]);
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].to_string(),
            "t.ron:4: terrain \"a\": fg colour \"lava\" is not in palette.ron"
        );
        let bg = src.replace("bg: \"black\"", "bg: \"void\"");
        let bg_errs = errors(&bg);
        assert_eq!(bg_errs.len(), 2);
        assert!(bg_errs[1].message.contains("bg colour \"void\""));
        // Without a palette the colour names are not checked.
        assert!(TerrainDef::from_source("t.ron", &src, None).is_ok());
    }

    #[test]
    fn cost_table_must_match_movement_types() {
        let src = file(
            TYPES,
            &[
                terrain("a", "..", "grass", r#"{ "foot": Some(1) }"#, 0),
                terrain(
                    "b",
                    "..",
                    "grass",
                    r#"{ "foot": Some(1), "flying": Some(1), "boat": Some(1) }"#,
                    0,
                ),
                terrain(
                    "c",
                    "..",
                    "grass",
                    r#"{ "foot": Some(0), "flying": None }"#,
                    0,
                ),
            ],
        );
        let text: Vec<String> = errors(&src).iter().map(ToString::to_string).collect();
        assert_eq!(
            text,
            [
                "t.ron:4: terrain \"a\": move_cost is missing movement type \"flying\"",
                "t.ron:6: terrain \"b\": move_cost has unknown movement type \"boat\"",
                "t.ron:8: terrain \"c\": move_cost for \"foot\" must be at least 1 (use None for impassable)",
            ]
        );
    }

    #[test]
    fn heal_percent_at_most_100() {
        let src = file(TYPES, &[terrain("a", "..", "grass", COSTS, 101)]);
        let errs = errors(&src);
        assert_eq!(errs.len(), 1);
        assert_eq!(
            errs[0].to_string(),
            "t.ron:4: terrain \"a\": heal_percent 101 is over 100"
        );
    }

    #[test]
    fn all_errors_reported_together() {
        let src = file(
            r#""foot", "foot""#,
            &[terrain("a", ".", "lava", r#"{ "foot": Some(1) }"#, 101)],
        );
        assert_eq!(errors(&src).len(), 4);
    }

    #[test]
    fn line_of_finds_first_match() {
        assert_eq!(line_of("a\nbb\nb", "b"), Some(2));
        assert_eq!(line_of("a", "z"), None);
    }

    /// Acceptance: the embedded terrain values match the table in
    /// `docs/design/stats-and-combat.md` ("Terrain combat effects").
    #[test]
    fn embedded_terrain_matches_design_doc() {
        let t = TerrainDef::load(PaletteDef::load().ok().as_ref());
        assert!(t.is_ok(), "{t:?}");
        let t = t.unwrap_or_default();
        assert_eq!(
            t.rules.movement_types,
            ["foot", "mounted", "armored", "flying"]
        );
        let expected: &[(&str, i8, i8, u8)] = &[
            ("plain", 0, 0, 0),
            ("road", 0, 0, 0),
            ("bridge", 0, 0, 0),
            ("floor", 0, 0, 0),
            ("forest", 1, 20, 0),
            ("mountain", 2, 30, 0),
            ("peak", 2, 40, 0),
            ("water", 0, 10, 0),
            ("sea", 0, 10, 0),
            ("village", 0, 10, 0),
            ("fort", 2, 20, 20),
            ("gate", 3, 20, 10),
            ("throne", 3, 30, 10),
        ];
        for &(id, def, avoid, heal) in expected {
            let rules = t.display.id_of(id).and_then(|i| t.rules.get(i));
            assert_eq!(
                rules.map(|r| (r.defense, r.avoid, r.heal_percent)),
                Some((def, avoid, heal)),
                "{id}"
            );
            assert!(
                rules.is_some_and(|r| r.move_cost.iter().any(Option::is_some)),
                "{id}"
            );
        }
        for id in ["thicket", "door", "wall"] {
            let rules = t.display.id_of(id).and_then(|i| t.rules.get(i));
            assert!(
                rules.is_some_and(|r| r.move_cost.iter().all(Option::is_none)),
                "{id}"
            );
        }
    }
}
