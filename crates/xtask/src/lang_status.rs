//! `cargo xtask lang-status <code>`: what a language pack lacks (ADR-0045):
//! the **missing** keys (no entry) and the **stale** ones (an entry made
//! from English that has since changed). Both show in English in the game
//! and fail no gate; this lists them for whoever translates.

use std::fmt::Write as _;

use trpg_content::{Lang, LangCode, MadeBy};

/// The report for the pack `code` of `lang`, or why there is none.
pub fn report(lang: &Lang, code: &str) -> Result<String, String> {
    let known = || {
        let codes: Vec<&str> = lang.codes().map(LangCode::as_str).collect();
        format!("the packs are: {}", codes.join(", "))
    };
    let code = LangCode::new(code).ok_or_else(|| format!("no language \"{code}\"; {}", known()))?;
    let (Some(pack), Some(status)) = (lang.pack(&code), lang.status(&code)) else {
        return Err(format!("no language pack \"{code}\"; {}", known()));
    };
    let made_by = match &pack.info.made_by {
        MadeBy::Machine => "machine-made".to_owned(),
        MadeBy::Human(credit) => format!("by {credit}"),
    };
    let mut out = format!(
        "lang-status {code} ({}, {made_by}): {} missing, {} stale\n",
        pack.info.name,
        status.missing.len(),
        status.stale.len()
    );
    if !status.missing.is_empty() {
        out.push_str("missing:\n");
    }
    for key in &status.missing {
        let english = lang.english(key).unwrap_or_default();
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "  {key}\n    english: {english:?}");
    }
    if !status.stale.is_empty() {
        out.push_str("stale:\n");
    }
    for key in &status.stale {
        let was = pack.entry(key).map_or("", |e| e.source.as_str());
        let now = lang.english(key).unwrap_or_default();
        let _ = writeln!(out, "  {key}\n    was: {was:?}\n    now: {now:?}");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use trpg_content::lang::{english_from_source, pack_from_sources};

    use super::*;

    fn lang(info: &str, ui: &str) -> Lang {
        let english = r#"{"a.one": "One", "a.two": "Two", "a.three": "Three"}"#;
        let english = english_from_source("en/ui.ron", english).unwrap();
        let pack = pack_from_sources(("l", info), ("u", ui), &english).unwrap();
        let packs = BTreeMap::from([(LangCode::new("xx").unwrap(), pack)]);
        Lang::new(english, packs)
    }

    #[test]
    fn lists_missing_and_stale_keys() {
        let ui = r#"[
            (key: "a.one", source: "One", text: "Un"),
            (key: "a.two", source: "Too", text: "Deux"),
        ]"#;
        let lang = lang(r#"(name: "Xx", made_by: Machine)"#, ui);
        assert_eq!(
            report(&lang, "xx").unwrap(),
            "lang-status xx (Xx, machine-made): 1 missing, 1 stale\n\
             missing:\n  a.three\n    english: \"Three\"\n\
             stale:\n  a.two\n    was: \"Too\"\n    now: \"Two\"\n"
        );
    }

    #[test]
    fn a_complete_pack_is_one_line() {
        let ui = r#"[
            (key: "a.one", source: "One", text: "Un"),
            (key: "a.two", source: "Two", text: "Deux"),
            (key: "a.three", source: "Three", text: "Trois"),
        ]"#;
        let lang = lang(r#"(name: "Xx", made_by: Human("A. Translator"))"#, ui);
        assert_eq!(
            report(&lang, "xx").unwrap(),
            "lang-status xx (Xx, by A. Translator): 0 missing, 0 stale\n"
        );
    }

    #[test]
    fn an_unknown_language_names_the_packs() {
        let lang = lang(r#"(name: "Xx", made_by: Machine)"#, "[]");
        assert_eq!(
            report(&lang, "zz").unwrap_err(),
            "no language pack \"zz\"; the packs are: xx"
        );
        // English is the source, not a pack.
        assert!(report(&lang, "en").is_err());
        assert_eq!(
            report(&lang, "Not A Code").unwrap_err(),
            "no language \"Not A Code\"; the packs are: xx"
        );
    }

    /// The test pack keeps one of each on purpose (ticket 0233).
    #[test]
    fn the_test_pack_has_one_stale_and_one_missing_key() {
        let content = trpg_content::load_embedded().unwrap();
        let report = report(&content.lang, "test").unwrap();
        assert!(
            report.starts_with("lang-status test (TEST, machine-made): 1 missing, 1 stale\n"),
            "{report}"
        );
        assert!(report.contains("missing:\n  title.credits\n"), "{report}");
        assert!(report.contains("stale:\n  title.subtitle\n"), "{report}");
    }
}
