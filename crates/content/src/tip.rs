//! One-time contextual tips (`assets/data/tips.ron`, ticket 0406): what
//! triggers each and what it says. The screens decide when a trigger has
//! happened and remember which tips were seen.

use std::collections::BTreeSet;

use serde::Deserialize;

use crate::bundle;
use crate::error::ContentError;
use crate::keymap::Action;
use crate::ron_loader::parse_ron;

/// Path of the tips file inside the asset bundle.
pub const TIPS_PATH: &str = "data/tips.ron";

/// Most text lines a tip may have.
pub const MAX_TEXT_LINES: usize = 3;

/// Longest text line, in characters, before placeholders are filled in.
pub const MAX_LINE_CHARS: usize = 60;

/// Longest tip title, in characters.
pub const MAX_TITLE_CHARS: usize = 30;

/// The placeholder for the keys that move the cursor (`{Cursor}`); every
/// other placeholder is an [`Action`] name, e.g. `{Confirm}`.
pub const CURSOR_PLACEHOLDER: &str = "Cursor";

/// What makes a tip appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum TipTrigger {
    /// A battle starts.
    FirstBattleStart,
    /// The player selects a unit.
    FirstUnitSelected,
    /// A selected unit could reach an enemy.
    FirstEnemyInRange,
    /// The attack forecast opens.
    FirstForecast,
    /// A player unit is badly hurt.
    FirstLowHp,
    /// A player unit levels up.
    FirstLevelUp,
    /// The enemy phase begins.
    FirstEnemyPhase,
    /// The cursor rests on an enemy, so its danger zone is worth knowing.
    DangerZoneAvailable,
}

/// One tip.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tip {
    /// Stable id (`snake_case`); what is remembered once the tip was seen.
    pub id: String,
    /// What makes it appear.
    pub trigger: TipTrigger,
    /// Shown in the box's border.
    pub title: String,
    /// The body: lines separated by `\n`, with `{Action}` placeholders
    /// for key names.
    pub text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    tips: Vec<Tip>,
}

/// Every tip, in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TipTable {
    /// The tips.
    pub tips: Vec<Tip>,
}

impl TipTable {
    /// The tip that `trigger` shows, if the file has one.
    pub fn for_trigger(&self, trigger: TipTrigger) -> Option<&Tip> {
        self.tips.iter().find(|t| t.trigger == trigger)
    }
}

/// Loads and validates the embedded tips file.
pub fn load() -> Result<TipTable, Vec<ContentError>> {
    let display = bundle::display_path(TIPS_PATH);
    let source = bundle::file(TIPS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Parses and validates tips `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<TipTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut ids = BTreeSet::new();
    let mut triggers = BTreeSet::new();
    for tip in &raw.tips {
        let mut err = |message: String| {
            errors.push(ContentError::new(
                file,
                format!("tip \"{}\": {message}", tip.id),
            ));
        };
        let id_ok = !tip.id.is_empty()
            && tip
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            err("the id must be lowercase letters, digits and `_`".into());
        }
        if !ids.insert(tip.id.as_str()) {
            err("the id is used twice".into());
        }
        if !triggers.insert(tip.trigger) {
            err(format!("{:?} already has a tip", tip.trigger));
        }
        if tip.title.is_empty() || tip.title.chars().count() > MAX_TITLE_CHARS {
            err(format!(
                "the title must be 1 to {MAX_TITLE_CHARS} characters"
            ));
        }
        let lines: Vec<&str> = tip.text.lines().collect();
        if lines.is_empty() || lines.len() > MAX_TEXT_LINES {
            err(format!("the text must have 1 to {MAX_TEXT_LINES} lines"));
        }
        for line in lines {
            if line.chars().count() > MAX_LINE_CHARS {
                err(format!(
                    "a text line is over {MAX_LINE_CHARS} characters: {line:?}"
                ));
            }
        }
        for name in placeholders(&tip.text) {
            if name != CURSOR_PLACEHOLDER && Action::from_name(name).is_none() {
                err(format!("unknown placeholder {{{name}}}"));
            }
        }
    }
    if errors.is_empty() {
        Ok(TipTable { tips: raw.tips })
    } else {
        Err(errors)
    }
}

/// The names between braces in `text`, in order (`{Confirm}` gives
/// `Confirm`). An unclosed `{` is ignored.
pub fn placeholders(text: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        names.push(&after[..close]);
        rest = &after[close + 1..];
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn errors(source: &str) -> Vec<String> {
        from_source("t.ron", source)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    #[test]
    fn the_shipped_file_loads_and_covers_every_trigger() {
        let table = load().unwrap();
        for trigger in [
            TipTrigger::FirstBattleStart,
            TipTrigger::FirstUnitSelected,
            TipTrigger::FirstEnemyInRange,
            TipTrigger::FirstForecast,
            TipTrigger::FirstLowHp,
            TipTrigger::FirstLevelUp,
            TipTrigger::FirstEnemyPhase,
            TipTrigger::DangerZoneAvailable,
        ] {
            assert!(table.for_trigger(trigger).is_some(), "{trigger:?}");
        }
    }

    #[test]
    fn placeholders_are_listed_in_order() {
        assert_eq!(placeholders("{A} x {B}{C} {"), ["A", "B", "C"]);
        assert!(placeholders("none {open").is_empty());
    }

    #[test]
    fn a_good_tip_loads() {
        let source = r#"(tips: [(id: "a", trigger: FirstForecast, title: "T",
            text: "Press {Confirm}.\n{Cursor}")])"#;
        let table = from_source("t.ron", source).unwrap();
        assert_eq!(table.tips.len(), 1);
        assert_eq!(
            table.for_trigger(TipTrigger::FirstForecast),
            Some(&table.tips[0])
        );
        assert_eq!(table.for_trigger(TipTrigger::FirstLowHp), None);
    }

    #[test]
    fn bad_tips_are_reported() {
        let long = "x".repeat(61);
        let source = format!(
            r#"(tips: [
            (id: "A", trigger: FirstForecast, title: "", text: "{{Nope}}"),
            (id: "A", trigger: FirstForecast, title: "T", text: "1\n2\n3\n4"),
            (id: "c", trigger: FirstLowHp, title: "T", text: "{long}"),
            (id: "d", trigger: FirstLowHp, title: "T", text: ""),
            ])"#
        );
        let all = errors(&source);
        let has = |part: &str| all.iter().any(|m| m.contains(part));
        assert!(has("lowercase letters"), "{all:?}");
        assert!(has("used twice"), "{all:?}");
        assert!(has("already has a tip"), "{all:?}");
        assert!(has("the title must be"), "{all:?}");
        assert!(has("unknown placeholder {Nope}"), "{all:?}");
        assert!(has("1 to 3 lines"), "{all:?}");
        assert!(has("over 60 characters"), "{all:?}");
    }

    #[test]
    fn a_syntax_error_is_reported() {
        assert_eq!(errors("(tips: [").len(), 1);
    }
}
