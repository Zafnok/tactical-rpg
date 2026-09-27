//! Combat Arts (`assets/data/arts.ron`), from `docs/design/combat-arts.md`.
//! The rules are in `trpg_core::art`.

use std::collections::BTreeMap;

use serde::Deserialize;
use trpg_core::{ArtDef, ArtEffect, ArtId, ArtTable, ItemDef, ItemTable, WeaponKind, WeaponRank};

use crate::bundle;
use crate::error::ContentError;
use crate::item::ITEMS_PATH;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the art file inside the asset bundle.
pub const ARTS_PATH: &str = "data/arts.ron";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    arts: Vec<RawArt>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArt {
    id: String,
    name: String,
    kind: WeaponKind,
    rank: Option<WeaponRank>,
    cost: u32,
    #[serde(default)]
    effect: ArtEffect,
}

/// Loads and validates the embedded art file.
pub fn load() -> Result<ArtTable, Vec<ContentError>> {
    let display = bundle::display_path(ARTS_PATH);
    let source = bundle::file(ARTS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Parses and validates art `source`, attributing errors to `file`. Reports
/// every problem found.
pub fn from_source(file: &str, source: &str) -> Result<ArtTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut errors = Vec::new();
    let mut arts = BTreeMap::new();
    for a in raw.arts {
        let at = |message: String| {
            let e = ContentError::new(file, message);
            match line_of(source, &format!("id: \"{}\"", a.id)) {
                Some(l) => e.at(l, None),
                None => e,
            }
        };
        let what = format!("art \"{}\"", a.id);
        if a.id.is_empty() {
            errors.push(at("an art id is empty".into()));
        }
        if a.cost == 0 {
            errors.push(at(format!("{what}: cost must be at least 1")));
        }
        errors.extend(
            problems(a.kind, &a.effect)
                .into_iter()
                .map(|p| at(format!("{what}: {p}"))),
        );
        let id = ArtId(a.id.clone());
        let def = ArtDef {
            id: id.clone(),
            name: a.name,
            kind: a.kind,
            rank: a.rank,
            cost: a.cost,
            effect: a.effect,
        };
        if arts.insert(id, def).is_some() {
            errors.push(at(format!("duplicate art id \"{}\"", a.id)));
        }
    }
    if errors.is_empty() {
        Ok(ArtTable { arts })
    } else {
        Err(errors)
    }
}

/// What is wrong with an art of `kind`'s effect.
fn problems(kind: WeaponKind, effect: &ArtEffect) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((_, den)) = effect.sword_followup {
        if kind != WeaponKind::Sword {
            out.push("sword_followup is for Sword arts only".into());
        }
        if den < 1 {
            out.push("sword_followup's denominator must be at least 1".into());
        }
    }
    if effect.axe_min_damage.is_some() && kind != WeaponKind::Axe {
        out.push("axe_min_damage is for Axe arts only".into());
    }
    if effect.min_range == Some(0) {
        out.push("min_range must be at least 1".into());
    }
    if effect.on_first_hit.is_some_and(|d| d.amount < 1) {
        out.push("a debuff's amount must be at least 1".into());
    }
    if effect.effective.iter().any(|&(_, mult)| mult < 1) {
        out.push("an effectiveness multiplier must be at least 1".into());
    }
    out
}

/// Checks that every weapon's `arts` name weapon arts (`rank: None`) in
/// `arts` of the weapon's own kind. Errors point at the weapon in the item
/// file.
pub fn check_references(arts: &ArtTable, items: &ItemTable) -> Vec<ContentError> {
    let mut errors = Vec::new();
    let display = bundle::display_path(ITEMS_PATH);
    for (id, item) in &items.items {
        let ItemDef::Weapon(weapon) = item else {
            continue;
        };
        for art in &weapon.arts {
            let problem = match arts.get(art) {
                None => format!("unknown art \"{}\"", art.0),
                Some(def) if def.kind != weapon.kind => format!(
                    "\"{}\" is a {:?} art, not a {:?} one",
                    art.0, def.kind, weapon.kind
                ),
                Some(def) if def.rank.is_some() => format!(
                    "\"{}\" is a rank art; a weapon lists only weapon arts (rank: None)",
                    art.0
                ),
                Some(_) => continue,
            };
            let e = ContentError::new(&display, format!("weapon \"{}\": {problem}", id.0));
            let line =
                bundle::file(ITEMS_PATH).and_then(|src| line_of(src, &format!("id: \"{}\"", id.0)));
            errors.push(match line {
                Some(l) => e.at(l, None),
                None => e,
            });
        }
    }
    errors
}

#[cfg(test)]
mod tests;
