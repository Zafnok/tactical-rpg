//! The lead: the Persona-style main character the player shapes
//! (`docs/design/setting-and-tone.md`, "The lead"). The player picks the
//! lead's gender and first name at New Game (0801), so dialogue refers to
//! the lead with tokens: `{lead}` for the name and `{they}`, `{them}`,
//! `{their}`, `{theirs}`, `{themself}` (and `{They}` etc.) for pronouns,
//! filled in by [`LeadProfile::substitute`].

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// The lead's character id: in dialogue (`lead: Let's move.`) and
/// `characters.ron`. Its portrait depends on the gender
/// ([`LeadProfile::portrait_id`]).
pub const LEAD_ID: &str = "lead";
/// The lead's default first name (a placeholder; the family name Veyne is
/// fixed), which the player can change at New Game.
pub const DEFAULT_NAME: &str = "Rowan";
/// Longest lead name, in characters.
pub const MAX_NAME_LEN: usize = 12;
/// Portrait of the male lead.
pub const PORTRAIT_MALE: &str = "lead_m";
/// Portrait of the female lead.
pub const PORTRAIT_FEMALE: &str = "lead_f";

/// The lead's gender, picked at New Game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeadGender {
    /// He / him.
    Male,
    /// She / her.
    Female,
}

impl LeadGender {
    /// Both genders.
    pub const ALL: [LeadGender; 2] = [LeadGender::Male, LeadGender::Female];

    /// This gender's pronouns.
    pub const fn pronouns(self) -> Pronouns {
        match self {
            LeadGender::Male => Pronouns {
                they: "he",
                them: "him",
                their: "his",
                theirs: "his",
                themself: "himself",
            },
            LeadGender::Female => Pronouns {
                they: "she",
                them: "her",
                their: "her",
                theirs: "hers",
                themself: "herself",
            },
        }
    }
}

/// A set of pronouns, named after the gender-neutral forms the script's
/// tokens use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pronouns {
    /// Subject: he / she (`{they}`).
    pub they: &'static str,
    /// Object: him / her (`{them}`).
    pub them: &'static str,
    /// Possessive determiner: his / her (`{their}`).
    pub their: &'static str,
    /// Possessive pronoun: his / hers (`{theirs}`).
    pub theirs: &'static str,
    /// Reflexive: himself / herself (`{themself}`).
    pub themself: &'static str,
}

impl Pronouns {
    /// The pronoun for the lower-case token name `token` (`they`, `them`,
    /// `their`, `theirs`, `themself`).
    pub fn get(&self, token: &str) -> Option<&'static str> {
        match token {
            "they" => Some(self.they),
            "them" => Some(self.them),
            "their" => Some(self.their),
            "theirs" => Some(self.theirs),
            "themself" => Some(self.themself),
            _ => None,
        }
    }
}

/// The lower-case pronoun token names.
pub const PRONOUN_TOKENS: [&str; 5] = ["they", "them", "their", "theirs", "themself"];
/// The name token.
pub const NAME_TOKEN: &str = "lead";

/// Who the player made the lead: stored in the campaign (0801).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeadProfile {
    /// The lead's first name, at most [`MAX_NAME_LEN`] characters.
    pub name: String,
    /// The lead's gender.
    pub gender: LeadGender,
}

impl LeadProfile {
    /// A profile.
    pub fn new(name: impl Into<String>, gender: LeadGender) -> Self {
        Self {
            name: name.into(),
            gender,
        }
    }

    /// The lead's pronouns.
    pub const fn pronouns(&self) -> Pronouns {
        self.gender.pronouns()
    }

    /// The lead's portrait: [`PORTRAIT_MALE`] or [`PORTRAIT_FEMALE`].
    pub const fn portrait_id(&self) -> &'static str {
        match self.gender {
            LeadGender::Male => PORTRAIT_MALE,
            LeadGender::Female => PORTRAIT_FEMALE,
        }
    }

    /// The portrait of `character`: the gendered one for [`LEAD_ID`],
    /// otherwise the character's own.
    pub fn portrait_for<'a>(&self, character: &'a str) -> &'a str {
        if character == LEAD_ID {
            self.portrait_id()
        } else {
            character
        }
    }

    /// What the token `{token}` stands for, or `None` if it isn't one.
    pub fn token(&self, token: &str) -> Option<Cow<'_, str>> {
        if token == NAME_TOKEN {
            return Some(Cow::Borrowed(&self.name));
        }
        if let Some(p) = self.pronouns().get(token) {
            return Some(Cow::Borrowed(p));
        }
        let lower = uncapitalised(token)?;
        let p = self.pronouns().get(&lower)?;
        Some(Cow::Owned(capitalise(p)))
    }

    /// `text` with every lead token replaced. Braces that don't form a
    /// token are left as they are (the dialogue validator rejects them).
    pub fn substitute<'t>(&self, text: &'t str) -> Cow<'t, str> {
        if !text.contains('{') {
            return Cow::Borrowed(text);
        }
        let mut out = String::with_capacity(text.len());
        for part in split_tokens(text) {
            let value = match part {
                Part::Token(t) => self.token(t),
                Part::Text(_) | Part::Unclosed(_) => None,
            };
            match (part, value) {
                (_, Some(value)) => out.push_str(&value),
                (Part::Token(t), None) => {
                    out.push('{');
                    out.push_str(t);
                    out.push('}');
                }
                (Part::Text(t) | Part::Unclosed(t), None) => out.push_str(t),
            }
        }
        Cow::Owned(out)
    }
}

/// Whether `token` (the text between the braces) is a lead token.
pub fn is_token(token: &str) -> bool {
    longest(token).is_some()
}

/// The most characters token `token` can become, over every name and
/// gender, or `None` if it isn't a token.
pub fn longest(token: &str) -> Option<usize> {
    if token == NAME_TOKEN {
        return Some(MAX_NAME_LEN);
    }
    // A capitalised form is as long as the lower-case one.
    let lower = uncapitalised(token).unwrap_or_else(|| token.to_owned());
    LeadGender::ALL
        .iter()
        .filter_map(|g| g.pronouns().get(&lower).map(str::len))
        .max()
}

/// Characters `text` can take once every lead token is filled in, at most
/// (unknown tokens count as written).
pub fn longest_len(text: &str) -> usize {
    split_tokens(text)
        .map(|part| match part {
            Part::Text(t) | Part::Unclosed(t) => t.chars().count(),
            Part::Token(t) => longest(t).unwrap_or(t.chars().count() + 2),
        })
        .sum()
}

/// A piece of dialogue text: plain text, a `{token}`, or a `{` with no
/// `}` after it (with the rest of the text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part<'a> {
    /// Text outside braces.
    Text(&'a str),
    /// The inside of a `{...}`.
    Token(&'a str),
    /// A `{` with no closing `}`, and everything after it.
    Unclosed(&'a str),
}

/// Splits `text` into plain text and `{tokens}`.
pub fn split_tokens(text: &str) -> impl Iterator<Item = Part<'_>> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let Some(open) = rest.find('{') else {
            let t = rest;
            rest = "";
            return Some(Part::Text(t));
        };
        if open > 0 {
            let t = &rest[..open];
            rest = &rest[open..];
            return Some(Part::Text(t));
        }
        let Some(close) = rest.find('}') else {
            let t = rest;
            rest = "";
            return Some(Part::Unclosed(t));
        };
        let t = &rest[1..close];
        rest = &rest[close + 1..];
        Some(Part::Token(t))
    })
}

/// `token` with its first letter lower-cased, if it starts with a capital
/// letter (`They` → `they`).
fn uncapitalised(token: &str) -> Option<String> {
    let mut chars = token.chars();
    let first = chars.next().filter(char::is_ascii_uppercase)?;
    Some(first.to_ascii_lowercase().to_string() + chars.as_str())
}

/// `word` with its first letter capitalised.
fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str()
    })
}

#[cfg(test)]
mod tests;
