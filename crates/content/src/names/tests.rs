//! Tests of the names table, name tokens and the literal-name check.

use proptest::prelude::*;

use super::*;

/// A table from `(id, name)` pairs.
fn table(entries: &[(&str, &str)]) -> Names {
    Names {
        names: entries
            .iter()
            .map(|&(id, name)| (id.to_owned(), name.to_owned()))
            .collect(),
    }
}

fn story() -> Names {
    table(&[
        ("king", "Emeric"),
        ("place.thornmarch", "the Thornmarch"),
        ("faction.crown", "the Crown of Ardeval"),
        ("term.vow", "a vow"),
        ("heretic", "Rue"),
    ])
}

/// Every error message for names `src`, as `file:line: message`.
fn errors(src: &str) -> Vec<String> {
    from_source("n.ron", src)
        .err()
        .unwrap_or_default()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn embedded_table_loads() {
    let names = load();
    assert!(names.is_ok(), "{names:?}");
    let names = names.unwrap_or_default();
    assert_eq!(names.get("king"), Some("Emeric"));
    assert_eq!(names.get("faction.crown"), Some("the Crown of Ardeval"));
}

/// The ids in the first column of every table of `docs/story/names.md`.
fn registry_ids() -> std::collections::BTreeSet<String> {
    include_str!("../../../../docs/story/names.md")
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split_once('`'))
        .map(|(id, _)| id.to_owned())
        .collect()
}

#[test]
fn table_holds_every_registered_id() {
    let ids = registry_ids();
    assert!(ids.len() > 60, "parsed only {} ids", ids.len());
    let names = load().unwrap_or_default();
    let missing: Vec<&String> = ids.iter().filter(|id| names.get(id).is_none()).collect();
    assert_eq!(missing, Vec::<&String>::new(), "ids missing from names.ron");
    // Every extra entry is a placeholder test character.
    let extra: Vec<&String> = names
        .names
        .keys()
        .filter(|id| !ids.contains(*id))
        .filter(|id| !id.starts_with("test_"))
        .collect();
    assert_eq!(
        extra,
        Vec::<&String>::new(),
        "ids not in docs/story/names.md"
    );
}

#[test]
fn lead_default_matches_core() {
    let names = load().unwrap_or_default();
    assert_eq!(names.get(lead::LEAD_ID), Some(lead::DEFAULT_NAME));
}

#[test]
fn parses_name_tokens() {
    assert_eq!(
        name_token("n:place.thornmarch"),
        Some(NameToken {
            id: "place.thornmarch",
            capital: false
        })
    );
    assert_eq!(
        name_token("N:king"),
        Some(NameToken {
            id: "king",
            capital: true
        })
    );
    assert_eq!(
        name_token("n:"),
        Some(NameToken {
            id: "",
            capital: false
        })
    );
    assert_eq!(name_token("lead"), None);
    assert_eq!(name_token("they"), None);
    assert_eq!(name_token("x:king"), None);
    assert!(is_name_id("place.kells_ford"));
    assert!(is_name_id("a1"));
    assert!(!is_name_id(""));
    assert!(!is_name_id("King"));
    assert!(!is_name_id("king-2"));
}

#[test]
fn substitutes_every_use() {
    let text = "{n:king} rides. {n:king} never stops.";
    assert_eq!(
        story().substitute(text),
        "Emeric rides. Emeric never stops."
    );
    // A rename is one entry.
    let renamed = table(&[("king", "Osric")]);
    assert_eq!(renamed.substitute(text), "Osric rides. Osric never stops.");
}

#[test]
fn capital_token_capitalises() {
    let names = story();
    assert_eq!(
        names.substitute("{N:place.thornmarch} is cold. We left {n:place.thornmarch}."),
        "The Thornmarch is cold. We left the Thornmarch."
    );
    assert_eq!(names.substitute("{N:king}"), "Emeric");
}

#[test]
fn leaves_other_tokens_and_unknown_ids() {
    let names = story();
    assert_eq!(
        names.substitute("{lead} and {n:nobody}, {they} {n:king} {"),
        "{lead} and {n:nobody}, {they} Emeric {"
    );
    assert!(matches!(names.substitute("No tokens."), Cow::Borrowed(_)));
    assert_eq!(names.substitute(""), "");
}

#[test]
fn longest_is_the_longest_name() {
    assert_eq!(story().longest(), "the Crown of Ardeval".len());
    assert_eq!(Names::default().longest(), 0);
}

#[test]
fn literal_form_drops_articles_and_skips_lowercase() {
    assert_eq!(literal_form("the Thornmarch"), Some("Thornmarch"));
    assert_eq!(literal_form("an Echo"), Some("Echo"));
    assert_eq!(literal_form("a Vow"), Some("Vow"));
    assert_eq!(literal_form("Emeric"), Some("Emeric"));
    assert_eq!(literal_form("Ama, the Mother"), Some("Ama, the Mother"));
    assert_eq!(literal_form("a vow"), None);
    assert_eq!(literal_form("breath"), None);
}

#[test]
fn literal_check_hits() {
    let names = story();
    assert_eq!(
        names.literal_in("Long live Emeric!"),
        Some(("king", "Emeric"))
    );
    assert_eq!(
        names.literal_in("Back to Thornmarch."),
        Some(("place.thornmarch", "Thornmarch"))
    );
    assert_eq!(
        names.literal_in("For the Crown of Ardeval, {lead}."),
        Some(("faction.crown", "Crown of Ardeval"))
    );
    assert_eq!(names.literal_in("\"Rue\"."), Some(("heretic", "Rue")));
}

#[test]
fn literal_check_reports_the_longest_name() {
    let names = table(&[
        ("family.marr", "Marr"),
        ("retainer", "Hollis Marr"),
        ("retainer.first", "Hollis"),
        ("rival.first", "Dace"),
        ("sister.first", "Wren"),
    ]);
    // A full name is itself, not the short forms inside it.
    assert_eq!(
        names.literal_in("Hollis Marr rode out."),
        Some(("retainer", "Hollis Marr"))
    );
    assert_eq!(
        names.literal_in("Hollis, wait."),
        Some(("retainer.first", "Hollis"))
    );
    assert_eq!(
        names.literal_in("A Marr never runs."),
        Some(("family.marr", "Marr"))
    );
    // Of equally long names, the first by id.
    assert_eq!(
        names.literal_in("Wren and Dace."),
        Some(("rival.first", "Dace"))
    );
}

#[test]
fn every_two_word_name_has_short_forms() {
    let names = load().unwrap_or_default();
    let story = || {
        names
            .names
            .iter()
            .filter(|(id, _)| !id.starts_with("test_"))
    };
    let mut people = 0;
    let mut gods = 0;
    let mut lasts = 0;
    for (id, name) in story() {
        if let Some((god, title)) = name.split_once(", ") {
            // A god: "Ama, the Mother".
            gods += 1;
            assert_eq!(names.get(&format!("{id}.name")), Some(god), "{id}");
            assert_eq!(names.get(&format!("{id}.title")), Some(title), "{id}");
        } else if !id.contains('.') && name.contains(' ') {
            // A person: "Hollis Marr".
            people += 1;
            let (first, last) = name.split_once(' ').unwrap_or_default();
            assert_eq!(names.get(&format!("{id}.first")), Some(first), "{id}");
            // The surname is the character's own, or a family's.
            let shared = story().any(|(i, n)| i.starts_with("family.") && n == last);
            let own = names.get(&format!("{id}.last"));
            assert_eq!(own, (!shared).then_some(last), "{id}");
            lasts += usize::from(own.is_some());
        }
    }
    assert_eq!((people, gods, lasts), (12, 5, 9));
    assert_eq!(names.get("family.marr"), Some("Marr"));
    assert_eq!(names.get("family.veyne"), Some("Veyne"));
    assert_eq!(names.get("red_captain.nickname"), Some("Red Harl"));
}

#[test]
fn literal_check_misses() {
    let names = story();
    // Tokens, other case, and ordinary words aren't hits.
    assert_eq!(names.literal_in("{n:king} and {N:place.thornmarch}."), None);
    assert_eq!(names.literal_in("You'll rue the day. emeric."), None);
    assert_eq!(names.literal_in("I swore a vow."), None);
}

#[test]
fn literal_check_is_whole_word() {
    let names = story();
    assert_eq!(names.literal_in("Emerics and Emeric2 and XEmeric"), None);
    assert_eq!(names.literal_in("Ruel. Rue's"), Some(("heretic", "Rue")));
}

#[test]
fn valid_source_loads() {
    let t = from_source(
        "n.ron",
        "{\n    \"king\": \"Emeric\",\n    \"place.a_b\": \"A\",\n}",
    );
    assert_eq!(t, Ok(table(&[("king", "Emeric"), ("place.a_b", "A")])));
}

#[test]
fn bad_ids_are_reported() {
    assert_eq!(
        errors("{\n    \"King\": \"Emeric\",\n}"),
        ["n.ron:2: \"King\" is not a valid name id; use lowercase letters, digits, _ and ."]
    );
}

#[test]
fn duplicate_ids_are_reported() {
    assert_eq!(
        errors("{\n    \"king\": \"Emeric\",\n    \"king\": \"Osric\",\n}"),
        ["n.ron:2: name id \"king\" is listed more than once"]
    );
}

#[test]
fn bad_values_are_reported() {
    assert_eq!(
        errors("{\n    \"a\": \" \",\n    \"b\": \"{n:a}\",\n    \"c\": \"Zo\u{eb}\",\n}"),
        [
            "n.ron:2: name \"a\" is empty",
            "n.ron:3: name \"b\" contains a brace; names can't hold tokens",
            "n.ron:4: name \"c\": unsupported character '\u{eb}'; dialogue text is plain ASCII",
        ]
    );
}

#[test]
fn duplicate_values_are_reported() {
    assert_eq!(
        errors("{\n    \"a\": \"Tor\",\n    \"b\": \"Tor\",\n}"),
        ["n.ron:3: \"Tor\" is the name of both \"a\" and \"b\"; names must be unique"]
    );
}

/// The token for name `i` of the property tests' tables.
fn marker(i: usize) -> String {
    format!("q{i}q")
}

/// `text` with every table name turned back into its token (the property
/// tables' names are `q<i>q`, capitalised `Q<i>q`, which plain text can't
/// contain).
fn retokenise(text: &str, count: usize) -> String {
    let mut out = text.to_owned();
    for i in 0..count {
        out = out
            .replace(&marker(i), &format!("{{n:id{i}}}"))
            .replace(&format!("Q{i}q"), &format!("{{N:id{i}}}"));
    }
    out
}

proptest! {
    /// Substituting then turning names back into tokens gives the text
    /// back, and leaves no name token.
    #[test]
    fn substitution_round_trips(
        count in 1..6usize,
        pieces in proptest::collection::vec(
            prop_oneof![
                "[a-pr-z ,.!]{0,6}".prop_map(|s| (s, None)),
                (0..6usize, any::<bool>()).prop_map(|(i, cap)| (String::new(), Some((i, cap)))),
                Just(("{lead}".to_owned(), None)),
            ],
            0..10,
        ),
    ) {
        let names = Names {
            names: (0..count).map(|i| (format!("id{i}"), marker(i))).collect(),
        };
        let text: String = pieces
            .iter()
            .map(|(s, token)| match token {
                Some((i, cap)) => format!("{{{}:id{}}}", if *cap { 'N' } else { 'n' }, i % count),
                None => s.clone(),
            })
            .collect();
        let out = names.substitute(&text);
        prop_assert!(!out.contains("{n:") && !out.contains("{N:"), "tokens left in {}", out);
        prop_assert_eq!(retokenise(&out, count), text);
    }
}
