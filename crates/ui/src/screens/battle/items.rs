//! The `Item` and `Equip` menus (ticket 0407): the battle pack grouped by
//! item, who each item can be used on, the equip list of a unit's weapons
//! and attack spells (0410), and the item target mode with its `HP 12 → 22` preview. Legality is the
//! core's ([`Command`]s it refuses change nothing); this module only decides
//! what to offer, so the player is never shown a use that heals nothing.

use trpg_core::{
    BattleState, Command, ConsumableEffect, Equipped, ItemId, Pos, SpellDef, StatValue, UnitAction,
    UnitId, WEAPON_SLOTS,
};

use super::attack::{spell_uses, weapon_name};
use super::info::range_text;
use super::mode::Selection;
use crate::color::UiColor;
use crate::widgets::menu::{Menu, MenuItem};

/// One line of the pack list: every copy of one consumable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackGroup {
    /// The item.
    pub item: ItemId,
    /// Index in the battle pack of its first copy (what `UseItem` sends).
    pub index: usize,
    /// How many copies the pack holds.
    pub count: usize,
    /// Who it can be used on: the unit itself and adjacent allies that are
    /// hurt, in `(y, x)` order of their tiles. Empty: the line is disabled.
    pub targets: Vec<UnitId>,
}

/// HP `effect` restores on a unit with `hp` of `max` HP.
pub fn heal_amount(effect: ConsumableEffect, hp: StatValue, max: StatValue) -> StatValue {
    let healed = match effect {
        ConsumableEffect::Heal(amount) => hp.saturating_add(amount.max(0)).min(max),
        ConsumableEffect::HealFull => max,
    };
    healed.max(hp) - hp
}

/// The units `unit` could use an item on from `dest`: itself and allies on
/// the four tiles around `dest`, that are hurt, in `(y, x)` order.
pub fn item_targets(state: &BattleState, unit: UnitId, dest: Pos) -> Vec<UnitId> {
    let Some(user) = state.unit(unit) else {
        return vec![];
    };
    let mut found: Vec<(Pos, UnitId)> = state
        .units()
        .iter()
        .filter(|u| {
            let near = u.id == unit || Pos::manhattan(dest, u.pos) == 1;
            near && !user.faction.is_hostile_to(u.faction) && u.hp < u.stats.hp
        })
        .map(|u| (if u.id == unit { dest } else { u.pos }, u.id))
        .collect();
    found.sort_by_key(|(p, _)| (p.y, p.x));
    found.into_iter().map(|(_, id)| id).collect()
}

/// The consumables of the pack grouped by id, in the order each first
/// appears, with who each could be used on from `dest`.
pub fn pack_groups(state: &BattleState, unit: UnitId, dest: Pos) -> Vec<PackGroup> {
    let mut groups: Vec<PackGroup> = vec![];
    for (index, id) in state.pack().items.iter().enumerate() {
        if state.items().consumable(id).is_none() {
            continue;
        }
        match groups.iter_mut().find(|g| &g.item == id) {
            Some(g) => g.count += 1,
            None => groups.push(PackGroup {
                item: id.clone(),
                index,
                count: 1,
                targets: vec![],
            }),
        }
    }
    let targets = item_targets(state, unit, dest);
    for g in &mut groups {
        // Every consumable so far heals, and `targets` are hurt units.
        g.targets.clone_from(&targets);
    }
    groups
}

/// Whether `Item` is enabled: some item in the pack can be used on someone.
pub fn can_use_item(groups: &[PackGroup]) -> bool {
    groups.iter().any(|g| !g.targets.is_empty())
}

/// `Pack 4/6`: how many items the pack holds of how many it could bring.
pub fn pack_header(state: &BattleState) -> String {
    format!("Pack {}/{}", state.pack().items.len(), state.pack().cap)
}

/// What a consumable does, in a few words.
pub fn effect_text(effect: ConsumableEffect) -> String {
    match effect {
        ConsumableEffect::Heal(n) => format!("Restore {n} HP"),
        ConsumableEffect::HealFull => "Restore all HP".to_owned(),
    }
}

/// The pack list: `Potion ×3  Restore 10 HP` per group.
pub fn pack_menu(state: &BattleState, groups: &[PackGroup]) -> Menu {
    let name = |g: &PackGroup| {
        state
            .items()
            .consumable(&g.item)
            .map_or_else(|| g.item.0.clone(), |c| c.name.clone())
    };
    let name_w = groups.iter().map(|g| name(g).chars().count()).max();
    let items = groups
        .iter()
        .map(|g| {
            let effect = state
                .items()
                .consumable(&g.item)
                .map(|c| effect_text(c.effect))
                .unwrap_or_default();
            let text = format!(
                "{:<w$} ×{}  {effect}",
                name(g),
                g.count,
                w = name_w.unwrap_or(0)
            );
            if g.targets.is_empty() {
                MenuItem::disabled(text)
            } else {
                MenuItem::new(text)
            }
        })
        .collect();
    Menu::new(items)
}

/// Picking who an item is used on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTargeting {
    /// The unit using the item and its path (it stands at the path's end).
    pub sel: Selection,
    /// The pack list it was opened from (Cancel goes back to it).
    pub menu: Menu,
    /// The pack's groups, as listed.
    pub groups: Vec<PackGroup>,
    /// Which group is being used.
    pub group: usize,
    /// The target under the cursor, an index of the group's targets.
    pub index: usize,
}

impl ItemTargeting {
    /// Targeting for `groups[group]`, starting on the user if it is a
    /// target. `None` if the group has no target.
    pub fn new(sel: Selection, menu: Menu, groups: Vec<PackGroup>, group: usize) -> Option<Self> {
        let g = groups.get(group)?;
        if g.targets.is_empty() {
            return None;
        }
        let index = g.targets.iter().position(|&t| t == sel.unit).unwrap_or(0);
        Some(Self {
            sel,
            menu,
            groups,
            group,
            index,
        })
    }

    /// The group being used.
    fn used(&self) -> &PackGroup {
        // `group` is valid and its targets are never empty ([`Self::new`]).
        &self.groups[self.group]
    }

    /// The target under the cursor.
    pub fn target(&self) -> UnitId {
        self.used().targets[self.index]
    }

    /// Who the item can be used on.
    pub fn targets(&self) -> &[UnitId] {
        &self.used().targets
    }

    /// Moves to the next target (or the previous one), wrapping.
    pub fn cycle(&mut self, forward: bool) {
        let n = self.used().targets.len();
        self.index = if forward {
            (self.index + 1) % n
        } else {
            (self.index + n - 1) % n
        };
    }

    /// The command that uses the item.
    pub fn command(&self) -> Command {
        Command::Act {
            unit: self.sel.unit,
            dest: self.sel.dest(),
            action: UnitAction::UseItem {
                pack_index: self.used().index,
                target: self.target(),
            },
        }
    }

    /// The preview line, e.g. `Potion on Rex: HP 12 → 22`.
    pub fn preview(&self, state: &BattleState) -> String {
        let item = &self.used().item;
        let name = state
            .items()
            .consumable(item)
            .map_or(item.0.as_str(), |c| c.name.as_str());
        let Some(target) = state.unit(self.target()) else {
            return name.to_owned();
        };
        let effect = state.items().consumable(item).map(|c| c.effect);
        let gain = effect.map_or(0, |e| heal_amount(e, target.hp, target.stats.hp));
        format!(
            "{name} on {}: HP {} → {}",
            target.name,
            target.hp,
            target.hp + gain
        )
    }
}

/// A line of the equip list: a weapon slot or an attack spell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipChoice {
    /// What it equips.
    pub what: Equipped,
    /// Whether the unit can fight with it: a weapon it can wield, a spell
    /// with a use left (the others are dimmed and can't be chosen).
    pub usable: bool,
}

/// The filled weapon slots of `unit`, in slot order, then its attack spells
/// in id order.
pub fn equip_choices(state: &BattleState, unit: UnitId) -> Vec<EquipChoice> {
    let Some(u) = state.unit(unit) else {
        return vec![];
    };
    let Some(class) = state.classes().get(&u.class) else {
        return vec![];
    };
    let weapons = (0..WEAPON_SLOTS)
        .filter(|&slot| u.loadout.weapon(slot).is_some())
        .map(|slot| EquipChoice {
            what: Equipped::Weapon(slot),
            usable: u.usable_weapon(slot, class, state.items()).is_some(),
        });
    let attack = |id: &&trpg_core::SpellId| state.spells().get(id).is_some_and(SpellDef::is_attack);
    let spells = u.learned.iter().filter(attack).map(|id| EquipChoice {
        what: Equipped::Spell(id.clone()),
        usable: u.castable_attack(id, state.spells()).is_some(),
    });
    weapons.chain(spells).collect()
}

/// Whether `Equip` is enabled: at least two usable weapons or spells.
pub fn can_equip(choices: &[EquipChoice]) -> bool {
    choices.iter().filter(|c| c.usable).count() >= 2
}

/// The name of what a line equips (empty if the unit lacks it).
fn equip_name(state: &BattleState, unit: UnitId, what: &Equipped) -> String {
    match what {
        Equipped::Weapon(slot) => weapon_name(state, unit, *slot),
        Equipped::Spell(spell) => {
            spell_uses(state, unit, spell).map_or_else(String::new, |(name, ..)| name)
        }
    }
}

/// The line of a weapon or spell: a marker if equipped, its name and stats
/// and durability (a spell's uses), e.g. `* Iron Sword  Mt 5 Hit 90 Crit 0
/// Wt 2 Rng1 20/20`.
fn equip_label(state: &BattleState, unit: UnitId, what: &Equipped, name_w: usize) -> String {
    let Some(u) = state.unit(unit) else {
        return String::new();
    };
    // Name, combat numbers, and what is left of it.
    let found = match what {
        Equipped::Weapon(slot) => u.loadout.weapon(*slot).and_then(|copy| {
            let def = state.items().weapon(&copy.def)?;
            let left = (copy.durability_left, def.durability);
            Some((def.name.clone(), state.items().weapon_stats(copy)?, left))
        }),
        Equipped::Spell(spell) => state.spells().get(spell).and_then(|def| {
            let left = u32::from(u.spells.uses_left(spell));
            Some((
                def.name.clone(),
                def.weapon_stats()?,
                (left, u32::from(def.uses)),
            ))
        }),
    };
    let Some((name, numbers, (left, max))) = found else {
        return String::new();
    };
    let marker = if u.loadout.equipped.as_ref() == Some(what) {
        '*'
    } else {
        ' '
    };
    format!(
        "{marker} {name:<name_w$}  Mt{:>2} Hit{:>3} Crit{:>2} Wt{:>2} Rng{} {left:>2}/{max}",
        numbers.might,
        numbers.hit,
        numbers.crit,
        numbers.weight,
        range_text(numbers.min_range, numbers.max_range)
    )
}

/// The equip list: one line per weapon or attack spell, the equipped one
/// marked and focused, unusable ones dimmed, broken weapons tagged in the
/// warning colour.
pub fn equip_menu(state: &BattleState, unit: UnitId, choices: &[EquipChoice]) -> Menu {
    let name_w = choices
        .iter()
        .map(|c| equip_name(state, unit, &c.what).chars().count())
        .max()
        .unwrap_or(0);
    let broken = |what: &Equipped| match what {
        Equipped::Weapon(slot) => state
            .unit(unit)
            .and_then(|u| u.loadout.weapon(*slot))
            .is_some_and(trpg_core::WeaponInstance::is_broken),
        Equipped::Spell(_) => false,
    };
    let items = choices
        .iter()
        .map(|c| {
            let label = equip_label(state, unit, &c.what, name_w);
            let item = if c.usable {
                MenuItem::new(label)
            } else {
                MenuItem::disabled(label)
            };
            if broken(&c.what) {
                item.with_suffix("(broken)", UiColor::HpLow)
            } else {
                item
            }
        })
        .collect();
    let equipped = state.unit(unit).and_then(|u| u.loadout.equipped.as_ref());
    let focus = choices
        .iter()
        .position(|c| Some(&c.what) == equipped)
        .unwrap_or(0);
    Menu::new(items).focused(focus)
}

/// The command that equips `what`.
pub fn equip_command(unit: UnitId, what: Equipped) -> Command {
    Command::Equip {
        unit,
        equipped: what,
    }
}
