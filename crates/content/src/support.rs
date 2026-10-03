//! Supports (`assets/data/supports.ron`, ticket 1002): the support numbers
//! and every pair of characters that has a support, from
//! `docs/design/supports.md`. The rules are in `trpg_core::support`.
//!
//! Loading checks that thresholds rise from C to B to A (the rules' and
//! each pair's own), that a higher rank's bonus is never smaller, that
//! both characters of a pair exist and differ, that no pair is listed
//! twice (either way round), and that each rank's conversation is a
//! dialogue scene.

use std::collections::BTreeSet;

use serde::Deserialize;
use trpg_core::{
    ByRank, CharacterId, PairDef, SupportBonus, SupportPair, SupportRules, SupportTable, Thresholds,
};

use crate::bundle;
use crate::character::CharacterTable;
use crate::dialogue::DialogueTable;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the support file inside the asset bundle.
pub const SUPPORTS_PATH: &str = "data/supports.ron";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    rules: SupportRules,
    pairs: Vec<RawPair>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPair {
    a: String,
    b: String,
    #[serde(default)]
    thresholds: Option<Thresholds>,
    #[serde(default)]
    starting_points: u32,
    conversations: ByRank<String>,
}

/// Loads and validates the embedded support file. Characters and
/// conversations are checked against `characters` and `dialogue` when
/// given (each is skipped if its files failed to load).
pub fn load(
    characters: Option<&CharacterTable>,
    dialogue: Option<&DialogueTable>,
) -> Result<SupportTable, Vec<ContentError>> {
    let display = bundle::display_path(SUPPORTS_PATH);
    let source = bundle::file(SUPPORTS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source, characters, dialogue)
}

/// Parses and validates support `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(
    file: &str,
    source: &str,
    characters: Option<&CharacterTable>,
    dialogue: Option<&DialogueTable>,
) -> Result<SupportTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors: Vec<ContentError> = rule_problems(&raw.rules)
        .into_iter()
        .map(|p| ContentError::new(file, p))
        .collect();
    let mut seen = BTreeSet::new();
    let mut pairs = Vec::new();
    for p in raw.pairs {
        let what = format!("pair \"{}\" and \"{}\"", p.a, p.b);
        let line = line_of(source, &format!("a: \"{}\", b: \"{}\"", p.a, p.b));
        let mut at = |message: String| {
            let e = ContentError::new(file, format!("{what}: {message}"));
            errors.push(match line {
                Some(l) => e.at(l, None),
                None => e,
            });
        };
        let pair = SupportPair::new(CharacterId(p.a.clone()), CharacterId(p.b.clone()));
        if p.a == p.b {
            at("a character can't be paired with itself".into());
        } else if !seen.insert(pair.clone()) {
            at("the pair is listed twice".into());
        }
        if let Some(characters) = characters {
            for id in pair.members() {
                if !characters.characters.contains_key(id) {
                    at(format!("unknown character \"{}\"", id.0));
                }
            }
        }
        if let Some(t) = p.thresholds.filter(|t| !t.is_increasing()) {
            at(rising(&t));
        }
        if let Some(dialogue) = dialogue {
            for scene in [&p.conversations.c, &p.conversations.b, &p.conversations.a] {
                if dialogue.get(scene).is_none() {
                    at(format!("unknown dialogue scene \"{scene}\""));
                }
            }
        }
        pairs.push(PairDef {
            pair,
            thresholds: p.thresholds,
            starting_points: p.starting_points,
            conversations: p.conversations,
        });
    }
    if errors.is_empty() {
        Ok(SupportTable::new(raw.rules, pairs))
    } else {
        Err(errors)
    }
}

/// The message for thresholds that don't rise.
fn rising(t: &Thresholds) -> String {
    format!(
        "thresholds must rise from C to B to A (got {}, {}, {})",
        t.c, t.b, t.a
    )
}

/// What is wrong with the rules.
fn rule_problems(rules: &SupportRules) -> Vec<String> {
    let mut problems = Vec::new();
    if !rules.thresholds.is_increasing() {
        problems.push(rising(&rules.thresholds));
    }
    let ByRank { c, b, a } = rules.bonus;
    let falls =
        |low: SupportBonus, high: SupportBonus| high.hit < low.hit || high.avoid < low.avoid;
    let none = SupportBonus::default();
    if falls(none, c) || falls(c, b) || falls(b, a) {
        problems.push(
            "bonus: a higher rank's hit and avoid can't be smaller than a lower rank's, \
             or below 0"
                .into(),
        );
    }
    problems
}

#[cfg(test)]
mod tests {
    use trpg_core::SupportRank;

    use super::*;
    use crate::{character, class, dialogue, item, names, terrain::TerrainDef};

    const RULES: &str = "rules: (
        thresholds: (c: 20, b: 80, a: 180),
        points: (adjacent_at_phase_end: 1, fight_beside: 3, heal: 3, buff: 3, item: 2),
        bonus_range: 3,
        bonus: (c: (hit: 5, avoid: 5), b: (hit: 10, avoid: 10), a: (hit: 15, avoid: 15)),
    )";

    const TALKS: &str = "conversations: (c: \"test\", b: \"test\", a: \"test\")";

    /// A support file with the design's rules and these pair entries.
    fn file(pairs: &[&str]) -> String {
        format!("(\n{RULES},\npairs: [\n{}\n],\n)", pairs.join(",\n"))
    }

    fn characters() -> CharacterTable {
        let terrain = TerrainDef::load(None).unwrap();
        let classes = class::load(Some(&terrain.rules.movement_types)).unwrap();
        let items = item::load().unwrap();
        let names = names::load().unwrap();
        character::load(Some(&classes), Some(&items), Some(&names)).unwrap()
    }

    fn scenes() -> DialogueTable {
        dialogue::load(None, None, None, None).unwrap()
    }

    /// Loads `source` checked against the embedded characters and scenes.
    fn checked(source: &str) -> Result<SupportTable, Vec<String>> {
        from_source("s.ron", source, Some(&characters()), Some(&scenes()))
            .map_err(|errors| errors.iter().map(ToString::to_string).collect())
    }

    fn pair(a: &str, b: &str) -> SupportPair {
        SupportPair::new(CharacterId(a.into()), CharacterId(b.into()))
    }

    #[test]
    fn valid_source_loads() {
        let knight = format!("(a: \"test_lord\", b: \"test_knight\", {TALKS})");
        let mage = "(
            a: \"test_mage\", b: \"test_lord\",
            thresholds: Some((c: 0, b: 50, a: 100)),
            starting_points: 4,
            conversations: (c: \"test\", b: \"test_fort\", a: \"test_talk\"),
        )";
        let table = checked(&file(&[&knight, mage])).unwrap();
        assert_eq!(table.rules, SupportRules::STARTING);
        assert_eq!(table.pairs().count(), 2);
        let def = table.get(&pair("test_knight", "test_lord")).unwrap();
        assert_eq!((def.thresholds, def.starting_points), (None, 0));
        assert_eq!(table.thresholds(def), SupportRules::STARTING.thresholds);
        // Either way round is the same pair.
        let def = table.get(&pair("test_lord", "test_mage")).unwrap();
        assert_eq!(def.starting_points, 4);
        assert_eq!(
            table.thresholds(def),
            ByRank {
                c: 0,
                b: 50,
                a: 100
            }
        );
        assert_eq!(
            SupportRank::ALL.map(|r| def.conversations.get(r).as_str()),
            ["test", "test_fort", "test_talk"]
        );
        // No pairs at all is fine.
        assert_eq!(checked(&file(&[])).map(|t| t.pairs().count()), Ok(0));
    }

    #[test]
    fn bad_sources_are_refused_with_their_position() {
        let errors = from_source("s.ron", "(", None, None).err().unwrap();
        assert_eq!(errors.len(), 1);
        let extra = file(&[]).replace("bonus_range: 3", "bonus_range: 3, stacking: true");
        let errors = from_source("s.ron", &extra, None, None).err().unwrap();
        assert_eq!(errors[0].line, Some(5));
        let unknown_field = format!("(a: \"test_lord\", b: \"test_knight\", rank: C, {TALKS})");
        assert!(from_source("s.ron", &file(&[&unknown_field]), None, None).is_err());
    }

    #[test]
    fn a_pair_needs_two_different_characters_that_exist() {
        let own = format!("(a: \"test_lord\", b: \"test_lord\", {TALKS})");
        assert_eq!(
            checked(&file(&[&own])),
            Err(vec![
                "s.ron:9: pair \"test_lord\" and \"test_lord\": a character can't be paired \
                 with itself"
                    .to_owned()
            ])
        );
        // A generic unit or a speaker isn't a character.
        let ghost = format!("(a: \"test_lord\", b: \"nobody\", {TALKS})");
        let generic = format!("(a: \"test_brigand\", b: \"test_knight\", {TALKS})");
        assert_eq!(
            checked(&file(&[&ghost, &generic])),
            Err(vec![
                "s.ron:9: pair \"test_lord\" and \"nobody\": unknown character \"nobody\""
                    .to_owned(),
                "s.ron:10: pair \"test_brigand\" and \"test_knight\": unknown character \
                 \"test_brigand\""
                    .to_owned(),
            ])
        );
        // Skipped when the characters failed to load.
        let table = from_source("s.ron", &file(&[&ghost]), None, Some(&scenes()));
        assert_eq!(table.map(|t| t.pairs().count()), Ok(1));
    }

    #[test]
    fn a_pair_is_listed_once_whichever_way_round() {
        let once = format!("(a: \"test_lord\", b: \"test_knight\", {TALKS})");
        let again = format!("(a: \"test_knight\", b: \"test_lord\", {TALKS})");
        assert_eq!(
            checked(&file(&[&once, &again])),
            Err(vec![
                "s.ron:10: pair \"test_knight\" and \"test_lord\": the pair is listed twice"
                    .to_owned()
            ])
        );
        assert_eq!(checked(&file(&[&once, &once])).map_err(|e| e.len()), Err(1));
    }

    #[test]
    fn thresholds_must_rise() {
        let flat = file(&[]).replace("(c: 20, b: 80, a: 180)", "(c: 20, b: 20, a: 180)");
        assert_eq!(
            checked(&flat),
            Err(vec![
                "s.ron: thresholds must rise from C to B to A (got 20, 20, 180)".to_owned()
            ])
        );
        let own = format!(
            "(a: \"test_lord\", b: \"test_knight\", thresholds: Some((c: 5, b: 9, a: 9)), {TALKS})"
        );
        assert_eq!(
            checked(&file(&[&own])),
            Err(vec![
                "s.ron:9: pair \"test_lord\" and \"test_knight\": thresholds must rise from C \
                 to B to A (got 5, 9, 9)"
                    .to_owned()
            ])
        );
    }

    #[test]
    fn a_higher_ranks_bonus_is_never_smaller() {
        let message = "s.ron: bonus: a higher rank's hit and avoid can't be smaller than a \
                       lower rank's, or below 0";
        for (from, to) in [
            ("c: (hit: 5, avoid: 5)", "c: (hit: 11, avoid: 5)"),
            ("c: (hit: 5, avoid: 5)", "c: (hit: 5, avoid: 11)"),
            ("c: (hit: 5, avoid: 5)", "c: (hit: -1, avoid: 5)"),
            ("c: (hit: 5, avoid: 5)", "c: (hit: 5, avoid: -1)"),
            ("a: (hit: 15, avoid: 15)", "a: (hit: 9, avoid: 15)"),
            ("a: (hit: 15, avoid: 15)", "a: (hit: 15, avoid: 9)"),
        ] {
            let source = file(&[]).replace(from, to);
            assert_eq!(checked(&source), Err(vec![message.to_owned()]), "{to}");
        }
        // Equal bonuses, or none at all, are fine.
        let flat = file(&[]).replace("a: (hit: 15, avoid: 15)", "a: (hit: 10, avoid: 10)");
        assert!(checked(&flat).is_ok());
        let zero = file(&[]).replace("c: (hit: 5, avoid: 5)", "c: (hit: 0, avoid: 0)");
        assert!(checked(&zero).is_ok());
    }

    #[test]
    fn every_conversation_is_a_dialogue_scene() {
        let lost = "(
            a: \"test_lord\", b: \"test_knight\",
            conversations: (c: \"test\", b: \"no_b\", a: \"no_a\"),
        )";
        assert_eq!(
            checked(&file(&[lost])),
            Err(vec![
                "s.ron:10: pair \"test_lord\" and \"test_knight\": unknown dialogue scene \"no_b\""
                    .to_owned(),
                "s.ron:10: pair \"test_lord\" and \"test_knight\": unknown dialogue scene \"no_a\""
                    .to_owned(),
            ])
        );
        let no_c = lost.replace("c: \"test\"", "c: \"no_c\"");
        assert_eq!(checked(&file(&[&no_c])).map_err(|e| e.len()), Err(3));
        // Skipped when the dialogue failed to load.
        let table = from_source("s.ron", &file(&[lost]), Some(&characters()), None);
        assert_eq!(table.map(|t| t.pairs().count()), Ok(1));
    }

    #[test]
    fn every_problem_is_reported() {
        let bad = "(
            a: \"nobody\", b: \"nobody\",
            thresholds: Some((c: 9, b: 5, a: 1)),
            conversations: (c: \"no_c\", b: \"test\", a: \"test\"),
        )";
        let source = file(&[bad]).replace("(c: 20, b: 80, a: 180)", "(c: 2, b: 1, a: 3)");
        assert_eq!(checked(&source).map_err(|e| e.len()), Err(6));
    }

    #[test]
    fn embedded_file_holds_the_starting_values() {
        let table = load(Some(&characters()), Some(&scenes())).unwrap();
        assert_eq!(table.rules, SupportRules::STARTING);
        assert!(table.get(&pair("test_lord", "test_knight")).is_some());
        assert!(table.get(&pair("test_lord", "test_mage")).is_some());
    }
}
