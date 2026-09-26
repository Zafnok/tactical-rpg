//! Items and item rules (`assets/data/items.ron`), from
//! `docs/design/weapons-and-items.md`.

use std::collections::BTreeMap;

use serde::Deserialize;
use trpg_core::{
    AccessoryDef, ArmourDef, ConsumableDef, ConsumableEffect, DamageType, ItemDef, ItemId,
    ItemTable, StatValue, Stats, UnitTag, WeaponDef, WeaponKind, WeaponRank, WeaponRules,
    WeaponTrait,
};

use crate::bundle;
use crate::enums::RawArmourWeight;
use crate::error::ContentError;
use crate::ron_loader::parse_ron;
use crate::terrain::line_of;

/// Path of the item file inside the asset bundle.
pub const ITEMS_PATH: &str = "data/items.ron";

const ALL_KINDS: [WeaponKind; 5] = [
    WeaponKind::Sword,
    WeaponKind::Spear,
    WeaponKind::Axe,
    WeaponKind::Bow,
    WeaponKind::Gauntlet,
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    rules: RawRules,
    kinds: Vec<RawKind>,
    weapons: Vec<RawWeapon>,
    armour: Vec<RawArmour>,
    accessories: Vec<RawAccessory>,
    consumables: Vec<RawConsumable>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRules {
    rank_speed: [StatValue; 6],
    rank_exp: [u32; 5],
    broken_might_divisor: StatValue,
    broken_hit_penalty: StatValue,
    exp_hit: u32,
    exp_miss: u32,
    exp_damage_divisor: u32,
    exp_art_multiplier: u32,
    default_pack_cap: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawKind {
    kind: WeaponKind,
    #[serde(rename = "trait")]
    trait_: WeaponTrait,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWeapon {
    id: String,
    name: String,
    kind: WeaponKind,
    rank: WeaponRank,
    might: StatValue,
    hit: StatValue,
    crit: StatValue,
    weight: StatValue,
    range: (u32, u32),
    #[serde(default = "physical")]
    damage_type: DamageType,
    durability: u32,
    #[serde(default)]
    effective: Vec<(UnitTag, u8)>,
    price: u32,
}

fn physical() -> DamageType {
    DamageType::Physical
}

/// Stat bonuses: only the stats given are raised.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
struct RawBonus {
    hp: StatValue,
    str: StatValue,
    mag: StatValue,
    dex: StatValue,
    spd: StatValue,
    def: StatValue,
    res: StatValue,
    mov: StatValue,
}

impl From<RawBonus> for Stats {
    fn from(b: RawBonus) -> Self {
        Stats::from_growable([b.hp, b.str, b.mag, b.dex, b.spd, b.def, b.res], b.mov)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArmour {
    id: String,
    name: String,
    weight_class: RawArmourWeight,
    bonus: RawBonus,
    weight: StatValue,
    price: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAccessory {
    id: String,
    name: String,
    bonus: RawBonus,
    price: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConsumable {
    id: String,
    name: String,
    effect: ConsumableEffect,
    price: u32,
}

/// Loads and validates the embedded item file.
pub fn load() -> Result<ItemTable, Vec<ContentError>> {
    let display = bundle::display_path(ITEMS_PATH);
    let source = bundle::file(ITEMS_PATH).ok_or_else(|| {
        vec![ContentError::new(
            &display,
            "file not found in asset bundle",
        )]
    })?;
    from_source(&display, source)
}

/// Parses and validates item `source`, attributing errors to `file`.
/// Reports every problem found.
pub fn from_source(file: &str, source: &str) -> Result<ItemTable, Vec<ContentError>> {
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut v = Validator {
        file,
        source,
        errors: Vec::new(),
        items: BTreeMap::new(),
    };
    let rules = v.rules(&raw.rules);
    let traits = v.traits(&raw.kinds);
    for w in raw.weapons {
        v.weapon(w);
    }
    for a in raw.armour {
        v.armour(a);
    }
    for a in raw.accessories {
        let def = AccessoryDef {
            name: a.name,
            bonus: a.bonus.into(),
            price: a.price,
        };
        v.add(&a.id, ItemDef::Accessory(def));
    }
    for c in raw.consumables {
        v.consumable(c);
    }
    if v.errors.is_empty() {
        Ok(ItemTable {
            items: v.items,
            traits,
            rules,
        })
    } else {
        Err(v.errors)
    }
}

/// Collects the items and the validation errors of one item file.
struct Validator<'a> {
    file: &'a str,
    source: &'a str,
    errors: Vec<ContentError>,
    items: BTreeMap<ItemId, ItemDef>,
}

impl Validator<'_> {
    /// Records an error, positioned at the first line containing `needle`.
    fn err(&mut self, needle: &str, message: String) {
        let e = ContentError::new(self.file, message);
        self.errors.push(match line_of(self.source, needle) {
            Some(l) => e.at(l, None),
            None => e,
        });
    }

    /// Records an error about item `id`.
    fn err_at_item(&mut self, id: &str, message: String) {
        self.err(&format!("id: \"{id}\""), message);
    }

    fn rules(&mut self, r: &RawRules) -> WeaponRules {
        if !r.rank_exp.windows(2).all(|w| w[0] < w[1]) || r.rank_exp[0] == 0 {
            self.err(
                "rank_exp",
                "rank_exp must be above 0 and strictly increasing".into(),
            );
        }
        let exp_divisor = StatValue::try_from(r.exp_damage_divisor).unwrap_or(1);
        for (name, value) in [
            ("broken_might_divisor", r.broken_might_divisor),
            ("exp_damage_divisor", exp_divisor),
        ] {
            if value < 1 {
                self.err(name, format!("{name} must be at least 1"));
            }
        }
        WeaponRules {
            rank_speed: r.rank_speed,
            rank_exp: r.rank_exp,
            broken_might_divisor: r.broken_might_divisor,
            broken_hit_penalty: r.broken_hit_penalty,
            exp_hit: r.exp_hit,
            exp_miss: r.exp_miss,
            exp_damage_divisor: r.exp_damage_divisor,
            exp_art_multiplier: r.exp_art_multiplier,
            default_pack_cap: r.default_pack_cap,
        }
    }

    /// Each kind's trait: every kind exactly once.
    fn traits(&mut self, kinds: &[RawKind]) -> BTreeMap<WeaponKind, WeaponTrait> {
        let mut traits = BTreeMap::new();
        for k in kinds {
            if traits.insert(k.kind, k.trait_).is_some() {
                self.err(
                    &format!("kind: {:?}", k.kind),
                    format!("kind {:?} is listed twice", k.kind),
                );
            }
        }
        for kind in ALL_KINDS.iter().filter(|k| !traits.contains_key(k)) {
            self.err("kinds", format!("kind {kind:?} has no trait entry"));
        }
        traits
    }

    /// Adds an item; ids must be non-empty and unique across all lists.
    fn add(&mut self, id: &str, def: ItemDef) {
        if id.is_empty() {
            self.err("id: \"\"", "an item id is empty".into());
        }
        if self.items.insert(ItemId::new(id), def).is_some() {
            self.err_at_item(id, format!("duplicate item id \"{id}\""));
        }
    }

    fn weapon(&mut self, w: RawWeapon) {
        let (min_range, max_range) = w.range;
        if min_range < 1 || min_range > max_range {
            self.err_at_item(
                &w.id,
                format!(
                    "weapon \"{}\": range ({min_range}, {max_range}) must have 1 <= min <= max",
                    w.id
                ),
            );
        }
        if w.durability == 0 {
            self.err_at_item(
                &w.id,
                format!("weapon \"{}\": durability must be at least 1", w.id),
            );
        }
        if [w.might, w.hit, w.crit, w.weight].iter().any(|&v| v < 0) {
            self.err_at_item(
                &w.id,
                format!(
                    "weapon \"{}\": might, hit, crit and weight can't be negative",
                    w.id
                ),
            );
        }
        let def = WeaponDef {
            name: w.name,
            kind: w.kind,
            rank: w.rank,
            might: w.might,
            hit: w.hit,
            crit: w.crit,
            weight: w.weight,
            min_range,
            max_range,
            damage_type: w.damage_type,
            durability: w.durability,
            effective: w.effective,
            price: w.price,
        };
        self.add(&w.id, ItemDef::Weapon(def));
    }

    fn armour(&mut self, a: RawArmour) {
        if a.weight < 0 {
            self.err_at_item(
                &a.id,
                format!("armour \"{}\": weight can't be negative", a.id),
            );
        }
        let def = ArmourDef {
            name: a.name,
            weight_class: a.weight_class.into(),
            bonus: a.bonus.into(),
            weight: a.weight,
            price: a.price,
        };
        self.add(&a.id, ItemDef::Armour(def));
    }

    fn consumable(&mut self, c: RawConsumable) {
        if let ConsumableEffect::Heal(n) = c.effect
            && n < 1
        {
            self.err_at_item(
                &c.id,
                format!("consumable \"{}\": Heal must be at least 1", c.id),
            );
        }
        let def = ConsumableDef {
            name: c.name,
            effect: c.effect,
            price: c.price,
        };
        self.add(&c.id, ItemDef::Consumable(def));
    }
}

#[cfg(test)]
mod tests;
