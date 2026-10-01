//! Class skills (`assets/data/skills.ron`), from `docs/design/progression.md`
//! and `docs/design/combat-arts.md`. The rules are in `trpg_core::skill`.
//!
//! Besides each skill's own shape, loading checks its cost (durability for
//! combat actives, an extra spell use for spell actives, uses per battle for
//! the others) and that every higher rank of a family is the rank below it
//! with bigger numbers, nothing else (`ranks`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use trpg_core::{
    ActiveEffect, Area, ClassTable, PassiveEffect, SkillCost, SkillDef, SkillId, SkillKind,
    SkillTable, WeaponReq,
};

use crate::bundle;
use crate::class::CLASSES_PATH;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the skill file inside the asset bundle.
pub const SKILLS_PATH: &str = "data/skills.ron";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    skills: Vec<RawSkill>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSkill {
    id: String,
    name: String,
    /// Empty (the default) means the id.
    #[serde(default)]
    family: String,
    #[serde(default = "first_rank")]
    rank: u8,
    kind: SkillKind,
}

fn first_rank() -> u8 {
    1
}

/// Loads and validates the embedded skill file.
pub fn load() -> Result<SkillTable, Vec<ContentError>> {
    let display = bundle::display_path(SKILLS_PATH);
    let source = bundle::file(SKILLS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Parses and validates skill `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<SkillTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut skills = BTreeMap::new();
    let mut ranks = BTreeSet::new();
    // An error at the line of skill `id`.
    let at_skill = |id: &str, message: String| {
        let e = ContentError::new(file, message);
        match line_of(source, &format!("id: \"{id}\"")) {
            Some(l) => e.at(l, None),
            None => e,
        }
    };
    for s in raw.skills {
        let at = |message: String| at_skill(&s.id, message);
        let what = format!("skill \"{}\"", s.id);
        if s.id.is_empty() {
            errors.push(at("a skill id is empty".into()));
        }
        if s.rank == 0 {
            errors.push(at(format!("{what}: rank must be at least 1")));
        }
        let family = if s.family.is_empty() {
            s.id.clone()
        } else {
            s.family.clone()
        };
        if !ranks.insert((family.clone(), s.rank)) {
            errors.push(at(format!(
                "{what}: family \"{family}\" already has a rank {}",
                s.rank
            )));
        }
        errors.extend(
            problems(&s.kind)
                .into_iter()
                .map(|p| at(format!("{what}: {p}"))),
        );
        let id = SkillId(s.id.clone());
        let def = SkillDef {
            id: id.clone(),
            name: s.name,
            family,
            rank: s.rank,
            kind: s.kind,
        };
        if skills.insert(id, def).is_some() {
            errors.push(at(format!("duplicate skill id \"{}\"", s.id)));
        }
    }
    errors.extend(
        ranks::rank_problems(&skills)
            .into_iter()
            .map(|(id, problem)| at_skill(&id.0, problem)),
    );
    if errors.is_empty() {
        Ok(SkillTable { skills })
    } else {
        Err(errors)
    }
}

/// What is wrong with a skill's kind.
fn problems(kind: &SkillKind) -> Vec<String> {
    let mut out = Vec::new();
    let mut radius = |r: u32, what: &str| {
        if r == 0 {
            out.push(format!("{what} radius must be at least 1"));
        }
    };
    match kind {
        SkillKind::Passive(effects) => {
            for e in effects {
                if let PassiveEffect::AllyAura { radius: r, .. } = e {
                    radius(*r, "an aura's");
                }
            }
        }
        SkillKind::Active { cost, effect } => {
            match effect {
                ActiveEffect::Buff {
                    area: Area::Allies { radius: r },
                    ..
                } => radius(*r, "a buff's"),
                ActiveEffect::Heal { radius: r, .. } => radius(*r, "a heal's"),
                ActiveEffect::Push { collision } if *collision < 0 => {
                    out.push("collision damage can't be negative".into());
                }
                _ => {}
            }
            let spell = matches!(
                effect,
                ActiveEffect::Strike {
                    with: WeaponReq::Spell,
                    ..
                }
            );
            let combat = matches!(effect, ActiveEffect::Strike { .. });
            match cost {
                SkillCost::Uses(_) if combat => out.push(
                    "only non-attack actives (Buff, Heal, Push) cost Uses; a combat active costs durability and a spell active ExtraSpellUse"
                        .into(),
                ),
                SkillCost::Uses(0) => out.push("uses per battle must be at least 1".into()),
                SkillCost::Durability(0) => out.push("a durability cost must be at least 1".into()),
                SkillCost::Durability(_) if spell => {
                    out.push("a spell active costs ExtraSpellUse, not durability".into());
                }
                SkillCost::ExtraSpellUse if !spell => {
                    out.push("only spell actives (Strike with Spell) cost ExtraSpellUse".into());
                }
                _ => {}
            }
        }
    }
    out
}

/// Checks that every class's active and passives name skills in `skills`,
/// of the right kind. Errors point at the class in its own file.
pub fn check_references(skills: &SkillTable, classes: &ClassTable) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let display = bundle::display_path(CLASSES_PATH);
    for (id, class) in &classes.classes {
        let refs = class
            .active
            .iter()
            .map(|s| (s, true))
            .chain(class.passives.iter().map(|s| (s, false)));
        for (skill, active) in refs {
            let problem = match skills.get(skill) {
                None => format!("unknown skill \"{}\"", skill.0),
                Some(def) if def.is_active() != active => format!(
                    "\"{}\" is {}",
                    skill.0,
                    if active {
                        "not an active"
                    } else {
                        "not a passive"
                    }
                ),
                Some(_) => continue,
            };
            let e = ContentError::new(&display, format!("class \"{}\": {problem}", id.0));
            let line = bundle::file(CLASSES_PATH)
                .and_then(|src| line_of(src, &format!("id: \"{}\"", id.0)));
            errors.push(match line {
                Some(l) => e.at(l, None),
                None => e,
            });
        }
    }
    errors
}

mod ranks;

#[cfg(test)]
mod tests;
