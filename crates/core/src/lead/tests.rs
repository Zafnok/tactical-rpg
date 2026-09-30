//! Tests of the lead profile: pronoun table, portraits and token
//! substitution.

use proptest::prelude::*;

use super::*;

fn male(name: &str) -> LeadProfile {
    LeadProfile::new(name, LeadGender::Male)
}

fn female(name: &str) -> LeadProfile {
    LeadProfile::new(name, LeadGender::Female)
}

#[test]
fn pronoun_table() {
    assert_eq!(
        LeadGender::Male.pronouns(),
        Pronouns {
            they: "he",
            them: "him",
            their: "his",
            theirs: "his",
            themself: "himself",
        }
    );
    assert_eq!(
        LeadGender::Female.pronouns(),
        Pronouns {
            they: "she",
            them: "her",
            their: "her",
            theirs: "hers",
            themself: "herself",
        }
    );
    assert_eq!(female("A").pronouns(), LeadGender::Female.pronouns());
    let p = LeadGender::Female.pronouns();
    let got: Vec<_> = PRONOUN_TOKENS.iter().map(|t| p.get(t)).collect();
    assert_eq!(
        got,
        [
            Some("she"),
            Some("her"),
            Some("her"),
            Some("hers"),
            Some("herself")
        ]
    );
    assert_eq!(p.get("lead"), None);
    assert_eq!(p.get("They"), None);
}

#[test]
fn portraits_follow_gender() {
    assert_eq!(male("A").portrait_id(), "lead_m");
    assert_eq!(female("A").portrait_id(), "lead_f");
    assert_eq!(male("A").portrait_for("lead"), "lead_m");
    assert_eq!(female("A").portrait_for("lead"), "lead_f");
    assert_eq!(female("A").portrait_for("bors"), "bors");
}

#[test]
fn substitutes_for_both_genders() {
    let text = "{lead} drew {their} sword. {They} swore {they} would \
                keep it; it was {theirs}. Ask {them}: {they} did it {themself}.";
    assert_eq!(
        male("Ellery").substitute(text),
        "Ellery drew his sword. He swore he would keep it; it was his. \
         Ask him: he did it himself."
    );
    assert_eq!(
        female("Ellery").substitute(text),
        "Ellery drew her sword. She swore she would keep it; it was hers. \
         Ask her: she did it herself."
    );
}

#[test]
fn substitutes_a_custom_name() {
    let p = female("Isolde");
    assert_eq!(p.substitute("{lead}? {lead}!"), "Isolde? Isolde!");
    assert_eq!(
        p.substitute("{Them} {Their} {Theirs} {Themself}"),
        "Her Her Hers Herself"
    );
}

#[test]
fn leaves_text_without_tokens_alone() {
    let p = male("Ellery");
    assert!(matches!(
        p.substitute("No tokens."),
        Cow::Borrowed("No tokens.")
    ));
    assert_eq!(p.substitute("{Lead} {THEY} {x} {"), "{Lead} {THEY} {x} {");
    assert_eq!(p.substitute("a } b"), "a } b");
    assert_eq!(p.substitute(""), "");
}

#[test]
fn tokens_and_their_longest_forms() {
    for t in ["lead", "they", "They", "themself", "Themself", "theirs"] {
        assert!(is_token(t), "{t}");
    }
    for t in ["Lead", "THEY", "x", "", "he", "n:king"] {
        assert!(!is_token(t), "{t}");
    }
    assert_eq!(longest("lead"), Some(MAX_NAME_LEN));
    assert_eq!(longest("they"), Some(3));
    assert_eq!(longest("They"), Some(3));
    assert_eq!(longest("them"), Some(3));
    assert_eq!(longest("their"), Some(3));
    assert_eq!(longest("theirs"), Some(4));
    assert_eq!(longest("themself"), Some(7));
    assert_eq!(longest("bogus"), None);
    assert_eq!(
        longest_len("Hi {lead}, {they} {x} {"),
        3 + 12 + 2 + 3 + 1 + 3 + 1 + 1
    );
}

#[test]
fn splits_tokens() {
    let parts: Vec<_> = split_tokens("a{b}c{d").collect();
    assert_eq!(
        parts,
        [
            Part::Text("a"),
            Part::Token("b"),
            Part::Text("c"),
            Part::Unclosed("{d")
        ]
    );
    assert_eq!(split_tokens("").count(), 0);
    assert_eq!(split_tokens("{}").collect::<Vec<_>>(), [Part::Token("")]);
}

#[test]
fn round_trips_through_serde() {
    let p = female("Ellery");
    let text = ron::to_string(&p).unwrap_or_default();
    let back: LeadProfile = ron::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, p);
}

proptest! {
    /// Substituting never gets longer than [`longest_len`] promises, for
    /// any name up to the limit.
    #[test]
    fn longest_len_bounds_substitution(
        name in "[A-Za-z]{1,12}",
        female_lead: bool,
        words in proptest::collection::vec(
            prop_oneof![
                "[a-z ]{0,6}".prop_map(|s| s),
                proptest::sample::select(vec![
                    "{lead}", "{they}", "{They}", "{them}", "{their}", "{theirs}",
                    "{themself}", "{Themself}", "{x}",
                ]).prop_map(str::to_owned),
            ],
            0..8,
        ),
    ) {
        let gender = if female_lead { LeadGender::Female } else { LeadGender::Male };
        let text = words.concat();
        let out = LeadProfile::new(name, gender).substitute(&text);
        prop_assert!(out.chars().count() <= longest_len(&text));
        let left = ["{they}", "{lead}"].iter().any(|t| out.contains(t));
        prop_assert!(!left, "tokens left in {}", out);
    }
}
