//! Languages (ADR-0045 §1–2): English screen text by key
//! (`assets/lang/en/ui.ron`), and the packs that overlay it
//! (`assets/lang/<code>/`). A pack entry carries the English it was made
//! from; when that is no longer today's English the entry is **stale**, and
//! a key with no entry is **missing**. Both show in English and fail
//! nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};

use crate::bundle;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::tip::placeholders;

/// Directory of the languages inside the asset bundle.
pub const LANG_DIR: &str = "lang";

/// The source language's code.
pub const ENGLISH: &str = "en";

/// The code of the pack the tests use. It is in the bundle, but never
/// offered to players.
pub const TEST: &str = "test";

/// File name of a language's screen text.
const UI_FILE: &str = "ui.ron";

/// File name of a pack's description.
const INFO_FILE: &str = "lang.ron";

/// Longest [`LangCode`], in characters.
const MAX_CODE_CHARS: usize = 8;

/// A language's id: 2 to 8 lowercase letters (`en`, `ja`), the name of its
/// directory under `assets/lang/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LangCode(String);

impl LangCode {
    /// `code` as a language id, or `None` if it isn't 2 to 8 lowercase
    /// letters.
    pub fn new(code: &str) -> Option<Self> {
        let ok = (2..=MAX_CODE_CHARS).contains(&code.len())
            && code.bytes().all(|b| b.is_ascii_lowercase());
        ok.then(|| Self(code.to_owned()))
    }

    /// English, the source language.
    pub fn english() -> Self {
        Self(ENGLISH.to_owned())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LangCode {
    fn default() -> Self {
        Self::english()
    }
}

impl fmt::Display for LangCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Who made a pack's text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum MadeBy {
    /// Machine-translated.
    Machine,
    /// Translated by a person or team: who to credit.
    Human(String),
}

/// A pack's `lang.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LangInfo {
    /// The language's own name for itself.
    pub name: String,
    /// Who made the text.
    pub made_by: MadeBy,
}

/// One piece of a pack's text.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Which text this is.
    pub key: String,
    /// The English `text` was made from.
    pub source: String,
    /// The text in the pack's language.
    pub text: String,
}

/// One language other than English.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangPack {
    /// Its name and who made it.
    pub info: LangInfo,
    /// Its screen text, by key.
    ui: BTreeMap<String, Entry>,
}

impl LangPack {
    /// The pack's screen-text entry for `key`, if it has one.
    pub fn entry(&self, key: &str) -> Option<&Entry> {
        self.ui.get(key)
    }
}

/// What a pack lacks ([`Lang::status`]), each list in key order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LangStatus {
    /// Keys the pack has no entry for.
    pub missing: Vec<String>,
    /// Keys whose entry was made from English that has since changed.
    pub stale: Vec<String>,
}

/// English's screen text and every pack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lang {
    /// English screen text, by key.
    english: BTreeMap<String, String>,
    /// The other languages, by code.
    packs: BTreeMap<LangCode, LangPack>,
}

impl Lang {
    /// English with `packs` over it.
    pub fn new(english: BTreeMap<String, String>, packs: BTreeMap<LangCode, LangPack>) -> Self {
        Self { english, packs }
    }

    /// Whether English has `key`.
    pub fn has(&self, key: &str) -> bool {
        self.english.contains_key(key)
    }

    /// Today's English for `key`, if there is such a key.
    pub fn english(&self, key: &str) -> Option<&str> {
        self.english.get(key).map(String::as_str)
    }

    /// The pack for `code`; `None` for English or an unknown code.
    pub fn pack(&self, code: &LangCode) -> Option<&LangPack> {
        self.packs.get(code)
    }

    /// The codes of the packs, in order (English isn't one).
    pub fn codes(&self) -> impl Iterator<Item = &LangCode> {
        self.packs.keys()
    }

    /// The text for `key` in language `code`: the pack's, when it has an
    /// entry made from today's English; otherwise English. A key English
    /// lacks comes back as it is.
    pub fn text<'a>(&'a self, code: &LangCode, key: &'a str) -> &'a str {
        let Some(english) = self.english.get(key) else {
            return key;
        };
        let entry = self.packs.get(code).and_then(|pack| pack.ui.get(key));
        match entry {
            Some(entry) if entry.source == *english => &entry.text,
            _ => english,
        }
    }

    /// What `code`'s pack lacks. English lacks nothing; `None` for a code
    /// with no pack.
    pub fn status(&self, code: &LangCode) -> Option<LangStatus> {
        if code.as_str() == ENGLISH {
            return Some(LangStatus::default());
        }
        let pack = self.packs.get(code)?;
        let mut status = LangStatus::default();
        for (key, english) in &self.english {
            match pack.ui.get(key) {
                None => status.missing.push(key.clone()),
                Some(entry) if entry.source != *english => status.stale.push(key.clone()),
                Some(_) => {}
            }
        }
        Some(status)
    }
}

/// A RON map read as its pairs in file order, so a key written twice is
/// seen twice.
struct Pairs(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Pairs {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PairsVisitor;

        impl<'de> Visitor<'de> for PairsVisitor {
            type Value = Pairs;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of key to text")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Pairs, A::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Pairs(pairs))
            }
        }

        deserializer.deserialize_map(PairsVisitor)
    }
}

/// Whether `key` is made of lowercase letters, digits, `_` and `.` only
/// (and isn't empty).
fn key_ok(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
}

/// Parses and validates English screen text `source` (a map of key to
/// text), attributing errors to `file`. Reports every problem found.
pub fn english_from_source(
    file: &str,
    source: &str,
) -> Result<BTreeMap<String, String>, Vec<ContentError>> {
    let Pairs(pairs) = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut english = BTreeMap::new();
    for (key, text) in pairs {
        if !key_ok(&key) {
            errors.push(ContentError::new(
                file,
                format!("\"{key}\": a key must be lowercase letters, digits, `_` and `.`"),
            ));
        }
        if english.contains_key(&key) {
            errors.push(ContentError::new(
                file,
                format!("\"{key}\": the key is used twice"),
            ));
        }
        english.insert(key, text);
    }
    if errors.is_empty() {
        Ok(english)
    } else {
        Err(errors)
    }
}

/// Parses and validates a pack: its `lang.ron` (`info_source`, errors
/// attributed to `info_file`) and its `ui.ron` (`ui_source`, `ui_file`),
/// checked against `english`. Reports every problem found.
///
/// An entry's key must be one of English's and be there once. Its text
/// must have the same placeholders as its `source`: the English it was
/// made from, not today's, so that changing English can make an entry
/// stale but never an error.
pub fn pack_from_sources(
    (info_file, info_source): (&str, &str),
    (ui_file, ui_source): (&str, &str),
    english: &BTreeMap<String, String>,
) -> Result<LangPack, Vec<ContentError>> {
    let mut errors = Vec::new();
    let info = parse_ron::<LangInfo>(info_file, info_source)
        .map_err(|e| errors.push(e))
        .ok();
    if info.as_ref().is_some_and(|i| i.name.trim().is_empty()) {
        errors.push(ContentError::new(info_file, "the name must not be empty"));
    }
    let entries = parse_ron::<Vec<Entry>>(ui_file, ui_source)
        .map_err(|e| errors.push(e))
        .unwrap_or_default();
    let mut ui = BTreeMap::new();
    for entry in entries {
        let mut err = |message: &str| {
            errors.push(ContentError::new(
                ui_file,
                format!("\"{}\": {message}", entry.key),
            ));
        };
        if !english.contains_key(&entry.key) {
            err("no such key in English");
        }
        if ui.contains_key(&entry.key) {
            err("the key is used twice");
        }
        let names = |text| placeholders(text).into_iter().collect::<BTreeSet<_>>();
        if names(&entry.text) != names(&entry.source) {
            err("the text and its source must have the same placeholders");
        }
        ui.insert(entry.key.clone(), entry);
    }
    match info {
        Some(info) if errors.is_empty() => Ok(LangPack { info, ui }),
        _ => Err(errors),
    }
}

/// One file under `assets/lang/`.
#[derive(Debug, Clone, Copy)]
pub struct LangFile<'a> {
    /// Its path under the languages' directory (`en/ui.ron`).
    pub path: &'a str,
    /// Its text; `None` if it isn't UTF-8.
    pub source: Option<&'a str>,
}

/// Loads and validates English and every pack in the embedded bundle.
pub fn load() -> Result<Lang, Vec<ContentError>> {
    let prefix = format!("{LANG_DIR}/");
    let paths = bundle::files_under(LANG_DIR);
    let files: Vec<LangFile<'_>> = paths
        .iter()
        .map(|path| LangFile {
            path: path.strip_prefix(&prefix).unwrap_or(path),
            source: bundle::file(path),
        })
        .collect();
    from_files(&files, |path| {
        bundle::display_path(&format!("{LANG_DIR}/{path}"))
    })
}

/// Validates the languages made of `files`: English's `en/ui.ron`, and a
/// pack for every other directory, which needs its `lang.ron` and
/// `ui.ron`. Any other file is an error. `display` gives the name a path
/// is reported under. Reports every problem found; with English broken
/// the packs are not checked (each entry would be an unknown key).
pub fn from_files(
    files: &[LangFile<'_>],
    display: impl Fn(&str) -> String,
) -> Result<Lang, Vec<ContentError>> {
    let mut errors = Vec::new();
    let read = |path: &str, errors: &mut Vec<ContentError>| {
        let source = files.iter().find(|f| f.path == path).and_then(|f| f.source);
        if source.is_none() {
            errors.push(ContentError::new(
                display(path),
                "file not found in asset bundle",
            ));
        }
        source
    };
    let mut dirs = BTreeSet::new();
    for file in files {
        let known = match file.path.split_once('/') {
            Some((ENGLISH, name)) => name == UI_FILE,
            Some((dir, name)) => {
                dirs.insert(dir);
                name == UI_FILE || name == INFO_FILE
            }
            None => false,
        };
        if !known {
            errors.push(ContentError::new(
                display(file.path),
                "not a file a language has",
            ));
        }
    }
    let english_path = format!("{ENGLISH}/{UI_FILE}");
    let english = read(&english_path, &mut errors)
        .map(|source| english_from_source(&display(&english_path), source));
    let english = match english {
        Some(Ok(english)) => Some(english),
        Some(Err(e)) => {
            errors.extend(e);
            None
        }
        None => None,
    };
    let mut packs = BTreeMap::new();
    for dir in dirs {
        let Some(code) = LangCode::new(dir) else {
            errors.push(ContentError::new(
                display(dir),
                "a language's directory must be named with 2 to 8 lowercase letters",
            ));
            continue;
        };
        let info_path = format!("{dir}/{INFO_FILE}");
        let ui_path = format!("{dir}/{UI_FILE}");
        let info = read(&info_path, &mut errors);
        let ui = read(&ui_path, &mut errors);
        let (Some(info), Some(ui), Some(english)) = (info, ui, &english) else {
            continue;
        };
        let info = (display(&info_path), info);
        let ui = (display(&ui_path), ui);
        match pack_from_sources((&info.0, info.1), (&ui.0, ui.1), english) {
            Ok(pack) => {
                packs.insert(code, pack);
            }
            Err(e) => errors.extend(e),
        }
    }
    match english {
        Some(english) if errors.is_empty() => Ok(Lang::new(english, packs)),
        _ => Err(errors),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGLISH_SOURCE: &str = r#"{
        "a.one": "One",
        "a.two": "Two {count}",
        "a.three": "Three",
    }"#;

    const INFO: &str = r#"(name: "Test", made_by: Machine)"#;

    fn english() -> BTreeMap<String, String> {
        english_from_source("en/ui.ron", ENGLISH_SOURCE).unwrap()
    }

    fn pack(ui: &str) -> Result<LangPack, Vec<String>> {
        pack_from_sources(("xx/lang.ron", INFO), ("xx/ui.ron", ui), &english())
            .map_err(|errors| errors.iter().map(ToString::to_string).collect())
    }

    fn code(code: &str) -> LangCode {
        LangCode::new(code).unwrap()
    }

    /// English with one pack `xx`: `a.one` translated, `a.two` stale,
    /// `a.three` missing.
    fn lang() -> Lang {
        let ui = r#"[
            (key: "a.one", source: "One", text: "Un"),
            (key: "a.two", source: "Two of {count}", text: "{count} deux"),
        ]"#;
        let packs = BTreeMap::from([(code("xx"), pack(ui).unwrap())]);
        Lang::new(english(), packs)
    }

    #[test]
    fn codes_are_two_to_eight_lowercase_letters() {
        for good in ["en", "ja", "test", "abcdefgh"] {
            assert_eq!(LangCode::new(good).unwrap().as_str(), good);
        }
        for bad in ["", "e", "EN", "e1", "pt-br", "abcdefghi", "日本"] {
            assert_eq!(LangCode::new(bad), None, "{bad}");
        }
        assert_eq!(LangCode::default(), LangCode::english());
        assert_eq!(LangCode::english().to_string(), "en");
    }

    #[test]
    fn english_loads_as_a_map() {
        let english = english();
        assert_eq!(english.len(), 3);
        assert_eq!(english["a.two"], "Two {count}");
    }

    #[test]
    fn bad_english_keys_are_reported_with_file_and_key() {
        let source = r#"{"A.b": "x", "": "y", "a b": "z", "ok_1.x": "fine", "ok_1.x": "again"}"#;
        let errors = english_from_source("en/ui.ron", source).unwrap_err();
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        let must = "a key must be lowercase letters, digits, `_` and `.`";
        assert_eq!(
            errors,
            [
                format!("en/ui.ron: \"A.b\": {must}"),
                format!("en/ui.ron: \"\": {must}"),
                format!("en/ui.ron: \"a b\": {must}"),
                "en/ui.ron: \"ok_1.x\": the key is used twice".to_owned(),
            ]
        );
    }

    #[test]
    fn english_that_is_not_a_map_is_a_syntax_error() {
        let errors = english_from_source("en/ui.ron", "[1, 2]").unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, Some(1));
        let errors = english_from_source("en/ui.ron", "{\"a\": \"b\", \"c\": 3}").unwrap_err();
        assert_eq!(errors.len(), 1);
    }

    /// What a wrong value is told the file should be.
    #[test]
    fn english_that_is_not_a_map_says_what_was_expected() {
        use serde::de::IntoDeserializer;
        use serde::de::value::{BoolDeserializer, Error};
        let wrong: BoolDeserializer<Error> = true.into_deserializer();
        let error = Pairs::deserialize(wrong).err().unwrap();
        assert_eq!(
            error.to_string(),
            "invalid type: boolean `true`, expected a map of key to text"
        );
    }

    #[test]
    fn a_pack_loads_its_info_and_entries() {
        let p = pack(r#"[(key: "a.one", source: "One", text: "Un")]"#).unwrap();
        assert_eq!(p.info.name, "Test");
        assert_eq!(p.info.made_by, MadeBy::Machine);
        assert_eq!(p.entry("a.one").unwrap().text, "Un");
        assert_eq!(p.entry("a.two"), None);
        let human = r#"(name: "Test", made_by: Human("A. Translator"))"#;
        let p = pack_from_sources(("l", human), ("u", "[]"), &english()).unwrap();
        assert_eq!(p.info.made_by, MadeBy::Human("A. Translator".to_owned()));
    }

    #[test]
    fn an_unknown_key_fails_with_file_and_key() {
        let errors = pack(r#"[(key: "a.nope", source: "x", text: "y")]"#).unwrap_err();
        assert_eq!(errors, ["xx/ui.ron: \"a.nope\": no such key in English"]);
    }

    #[test]
    fn different_placeholders_fail_with_file_and_key() {
        let message = "the text and its source must have the same placeholders";
        for text in ["deux", "{n} deux", "{count} {n}"] {
            let ui = format!(r#"[(key: "a.two", source: "Two {{count}}", text: "{text}")]"#);
            assert_eq!(
                pack(&ui).unwrap_err(),
                [format!("xx/ui.ron: \"a.two\": {message}")],
                "{text}"
            );
        }
        // The same set, in any order and any number of times, is fine.
        let ui = r#"[(key: "a.two", source: "{a} {b}", text: "{b} {a} {b}")]"#;
        assert!(pack(ui).is_ok());
    }

    #[test]
    fn placeholders_are_checked_against_the_source_not_todays_english() {
        // English gained `{count}` since: the entry is stale, not an error.
        let ui = r#"[(key: "a.two", source: "Two", text: "Deux")]"#;
        assert!(pack(ui).is_ok());
    }

    #[test]
    fn a_key_used_twice_in_a_pack_fails() {
        let ui = r#"[
            (key: "a.one", source: "One", text: "Un"),
            (key: "a.one", source: "One", text: "Une"),
        ]"#;
        assert_eq!(
            pack(ui).unwrap_err(),
            ["xx/ui.ron: \"a.one\": the key is used twice"]
        );
    }

    #[test]
    fn every_problem_in_a_pack_is_reported() {
        let errors = pack_from_sources(
            ("xx/lang.ron", r#"(name: " ", made_by: Machine)"#),
            (
                "xx/ui.ron",
                r#"[(key: "a.nope", source: "{a}", text: "b")]"#,
            ),
            &english(),
        )
        .unwrap_err();
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        assert_eq!(errors.len(), 3, "{errors:?}");
        assert_eq!(errors[0], "xx/lang.ron: the name must not be empty");
        // A file that doesn't parse is one error, attributed to that file.
        let errors = pack_from_sources(("l.ron", "("), ("u.ron", "["), &english()).unwrap_err();
        let files: Vec<&str> = errors.iter().map(|e| e.file.as_str()).collect();
        assert_eq!(files, ["l.ron", "u.ron"]);
        // A good `ui.ron` doesn't make up for a bad `lang.ron`.
        assert!(pack_from_sources(("l.ron", "("), ("u.ron", "[]"), &english()).is_err());
    }

    #[test]
    fn lookup_uses_the_pack_and_falls_back_to_english() {
        let lang = lang();
        let xx = code("xx");
        assert_eq!(lang.text(&xx, "a.one"), "Un");
        // Stale and missing entries show English.
        assert_eq!(lang.text(&xx, "a.two"), "Two {count}");
        assert_eq!(lang.text(&xx, "a.three"), "Three");
        // English, and a code with no pack, are English.
        assert_eq!(lang.text(&LangCode::english(), "a.one"), "One");
        assert_eq!(lang.text(&code("zz"), "a.one"), "One");
        // An unknown key comes back as it is.
        assert_eq!(lang.text(&xx, "a.nope"), "a.nope");
        assert!(lang.has("a.one"));
        assert!(!lang.has("a.nope"));
        assert_eq!(lang.english("a.one"), Some("One"));
        assert_eq!(lang.english("a.nope"), None);
    }

    #[test]
    fn status_lists_missing_and_stale_keys() {
        let lang = lang();
        let status = lang.status(&code("xx")).unwrap();
        assert_eq!(status.missing, ["a.three"]);
        assert_eq!(status.stale, ["a.two"]);
        assert_eq!(
            lang.status(&LangCode::english()),
            Some(LangStatus::default())
        );
        assert_eq!(lang.status(&code("zz")), None);
        assert_eq!(lang.codes().collect::<Vec<_>>(), [&code("xx")]);
        assert!(lang.pack(&code("xx")).is_some());
        assert!(lang.pack(&LangCode::english()).is_none());
    }

    /// The languages made of `files` (path, text), or the errors as text.
    fn files(files: &[(&str, &str)]) -> Result<Lang, Vec<String>> {
        let files: Vec<LangFile<'_>> = files
            .iter()
            .map(|&(path, source)| LangFile {
                path,
                source: Some(source),
            })
            .collect();
        from_files(&files, |path| format!("lang/{path}"))
            .map_err(|errors| errors.iter().map(ToString::to_string).collect())
    }

    const UI: &str = r#"[(key: "a.one", source: "One", text: "Un")]"#;

    #[test]
    fn files_make_english_and_a_pack_per_directory() {
        let lang = files(&[
            ("en/ui.ron", ENGLISH_SOURCE),
            ("xx/lang.ron", INFO),
            ("xx/ui.ron", UI),
            ("yy/ui.ron", "[]"),
            ("yy/lang.ron", INFO),
        ])
        .unwrap();
        assert_eq!(lang.codes().collect::<Vec<_>>(), [&code("xx"), &code("yy")]);
        assert_eq!(lang.text(&code("xx"), "a.one"), "Un");
        assert_eq!(lang.text(&code("yy"), "a.one"), "One");
        // English alone is enough.
        let lang = files(&[("en/ui.ron", ENGLISH_SOURCE)]).unwrap();
        assert_eq!(lang.codes().count(), 0);
        assert!(lang.has("a.one"));
    }

    #[test]
    fn a_missing_file_is_reported() {
        assert_eq!(
            files(&[]).unwrap_err(),
            ["lang/en/ui.ron: file not found in asset bundle"]
        );
        assert_eq!(
            files(&[("en/ui.ron", ENGLISH_SOURCE), ("xx/ui.ron", UI)]).unwrap_err(),
            ["lang/xx/lang.ron: file not found in asset bundle"]
        );
        assert_eq!(
            files(&[("en/ui.ron", ENGLISH_SOURCE), ("xx/lang.ron", INFO)]).unwrap_err(),
            ["lang/xx/ui.ron: file not found in asset bundle"]
        );
        // A file that isn't text counts as missing.
        let binary = [LangFile {
            path: "en/ui.ron",
            source: None,
        }];
        let errors = from_files(&binary, str::to_owned).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, "en/ui.ron");
    }

    #[test]
    fn a_file_no_language_has_is_reported() {
        let stray = "not a file a language has";
        assert_eq!(
            files(&[
                ("en/ui.ron", ENGLISH_SOURCE),
                ("en/lang.ron", INFO),
                ("notes.txt", ""),
                ("xx/lang.ron", INFO),
                ("xx/ui.ron", UI),
                ("xx/data.ron", "[]"),
                ("xx/dialogue/ch01.ron", "[]"),
            ])
            .unwrap_err(),
            [
                format!("lang/en/lang.ron: {stray}"),
                format!("lang/notes.txt: {stray}"),
                format!("lang/xx/data.ron: {stray}"),
                format!("lang/xx/dialogue/ch01.ron: {stray}"),
            ]
        );
    }

    #[test]
    fn a_badly_named_directory_is_reported() {
        assert_eq!(
            files(&[
                ("en/ui.ron", ENGLISH_SOURCE),
                ("JA/lang.ron", INFO),
                ("JA/ui.ron", UI),
            ])
            .unwrap_err(),
            ["lang/JA: a language's directory must be named with 2 to 8 lowercase letters"]
        );
    }

    #[test]
    fn pack_errors_are_reported_for_every_pack() {
        let bad = r#"[(key: "a.nope", source: "x", text: "y")]"#;
        assert_eq!(
            files(&[
                ("en/ui.ron", ENGLISH_SOURCE),
                ("xx/lang.ron", INFO),
                ("xx/ui.ron", bad),
                ("yy/lang.ron", INFO),
                ("yy/ui.ron", bad),
            ])
            .unwrap_err(),
            [
                "lang/xx/ui.ron: \"a.nope\": no such key in English",
                "lang/yy/ui.ron: \"a.nope\": no such key in English",
            ]
        );
    }

    #[test]
    fn with_english_broken_the_packs_are_not_checked() {
        let errors = files(&[
            ("en/ui.ron", r#"{"Bad Key": "x"}"#),
            ("xx/lang.ron", INFO),
            ("xx/ui.ron", UI),
        ])
        .unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("lang/en/ui.ron: \"Bad Key\""));
    }

    #[test]
    fn the_embedded_languages_load() {
        let lang = load().unwrap();
        assert_eq!(lang.english("title.new_game"), Some("New Game"));
        let test = code(TEST);
        assert_eq!(lang.codes().collect::<Vec<_>>(), [&test]);
        assert_eq!(lang.text(&test, "title.new_game"), "NEW GAME");
        // The test pack keeps one stale and one missing entry.
        let status = lang.status(&test).unwrap();
        assert_eq!(status.missing, ["title.credits"]);
        assert_eq!(status.stale, ["title.subtitle"]);
    }
}
