//! Choosing an attack (ticket 0404): which weapons can attack someone from
//! where the unit stands, the targets of each, in `(y, x)` order, and the
//! targeting state with its forecast. Every check is
//! [`BattleState::preview_attack`], the same validation the attack command
//! gets, so the UI never decides what is legal itself (ADR-0004).

use trpg_core::{AttackPreview, BattleState, Pos, UnitAction, UnitId, WEAPON_SLOTS};

use super::art_list::{ArtChoice, Technique, art_choices, art_menu, durability_text};
use super::mode::Selection;
use crate::color::UiColor;
use crate::input::Action;
use crate::widgets::menu::{Menu, MenuItem};

/// The weapon attack with the weapon in `slot` on `target`: no art, no
/// active.
pub fn attack(target: UnitId, slot: usize) -> UnitAction {
    Technique::Attack.action(target, slot)
}

/// The units `unit` could attack from `dest` with the weapon in `slot`,
/// ordered by `(y, x)` of their tiles.
pub fn targets(state: &BattleState, unit: UnitId, dest: Pos, slot: usize) -> Vec<UnitId> {
    let mut found: Vec<(Pos, UnitId)> = state
        .units()
        .iter()
        .filter(|u| {
            state
                .preview_attack(unit, dest, &attack(u.id, slot))
                .is_ok()
        })
        .map(|u| (u.pos, u.id))
        .collect();
    found.sort_by_key(|(p, _)| (p.y, p.x));
    found.into_iter().map(|(_, id)| id).collect()
}

/// A weapon that can attack someone from the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponChoice {
    /// Its loadout slot.
    pub slot: usize,
    /// Who it can attack, in `(y, x)` order (never empty).
    pub targets: Vec<UnitId>,
}

/// The weapons of `sel`'s unit that can attack someone from its path's
/// end, in slot order.
pub fn weapon_choices(state: &BattleState, sel: &Selection) -> Vec<WeaponChoice> {
    (0..WEAPON_SLOTS)
        .filter_map(|slot| {
            let targets = targets(state, sel.unit, sel.dest(), slot);
            (!targets.is_empty()).then_some(WeaponChoice { slot, targets })
        })
        .collect()
}

/// The weapon list's line for `slot`: name and stats, e.g.
/// `Iron Sword   Mt 5  Hit 90  Crit 0  Rng 1`, names padded to `name_w`.
pub fn weapon_label(state: &BattleState, unit: UnitId, slot: usize, name_w: usize) -> String {
    let Some(def) = state
        .unit(unit)
        .and_then(|u| u.loadout.weapon(slot))
        .and_then(|w| state.items().weapon(&w.def))
    else {
        return String::new();
    };
    let range = if def.min_range == def.max_range {
        def.min_range.to_string()
    } else {
        format!("{}-{}", def.min_range, def.max_range)
    };
    format!(
        "{:<name_w$}  Mt {:>2}  Hit {:>3}  Crit {:>2}  Rng {range}",
        def.name, def.might, def.hit, def.crit
    )
}

/// The weapon in `unit`'s `slot`: its name, durability left and max.
pub fn weapon_durability(
    state: &BattleState,
    unit: UnitId,
    slot: usize,
) -> Option<(String, u32, u32)> {
    let copy = state.unit(unit)?.loadout.weapon(slot)?;
    let def = state.items().weapon(&copy.def)?;
    Some((def.name.clone(), copy.durability_left, def.durability))
}

/// The name of the weapon in `unit`'s `slot` (empty if none).
pub fn weapon_name(state: &BattleState, unit: UnitId, slot: usize) -> String {
    state
        .unit(unit)
        .and_then(|u| u.loadout.weapon(slot))
        .and_then(|w| state.items().weapon(&w.def))
        .map(|d| d.name.clone())
        .unwrap_or_default()
}

/// The weapon list: one line per choice with the weapon's durability after
/// it (`20/20`, `broken` at 0), focused on the equipped weapon if it is one
/// of them.
pub fn weapon_menu(state: &BattleState, sel: &Selection, choices: &[WeaponChoice]) -> Menu {
    let name_w = choices
        .iter()
        .map(|c| weapon_name(state, sel.unit, c.slot).chars().count())
        .max()
        .unwrap_or(0);
    let items = choices
        .iter()
        .map(|c| {
            let item = MenuItem::new(weapon_label(state, sel.unit, c.slot, name_w));
            match weapon_durability(state, sel.unit, c.slot) {
                Some((_, left, max)) => {
                    let color = if left == 0 {
                        UiColor::HpLow
                    } else {
                        UiColor::TextDim
                    };
                    item.with_suffix(format!(" {}", durability_text(left, max)), color)
                }
                None => item,
            }
        })
        .collect();
    let equipped = state.unit(sel.unit).and_then(|u| u.loadout.equipped_slot());
    let focus = choices
        .iter()
        .position(|c| Some(c.slot) == equipped)
        .unwrap_or(0);
    Menu::new(items).focused(focus)
}

/// Picking a target for an attack with one weapon, and what to attack it
/// with: `Attack`, a Combat Art or a combat active (the arts list, 0414).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targeting {
    /// The attacker and its path (it stands at the path's end).
    pub sel: Selection,
    /// The attacking weapon's loadout slot.
    pub slot: usize,
    /// Who it can attack, in `(y, x)` order (never empty).
    pub targets: Vec<UnitId>,
    /// The target under the cursor.
    pub index: usize,
    /// The arts list's lines against the target (`Attack` first, so never
    /// empty).
    pub choices: Vec<ArtChoice>,
    /// The arts list; its focus is the line the attack uses.
    pub list: Menu,
    /// The forecast against the target, with the chosen line applied.
    pub preview: AttackPreview,
    /// The weapon list it was opened from (Cancel goes back to it), if the
    /// unit had several weapons to choose from.
    pub weapons: Option<(Menu, Vec<WeaponChoice>)>,
}

impl Targeting {
    /// Targeting the first of `choice`'s targets with a plain attack.
    /// `None` if its forecast can't be made (the battle changed; never
    /// while choosing).
    pub fn new(
        state: &BattleState,
        sel: Selection,
        choice: &WeaponChoice,
        weapons: Option<(Menu, Vec<WeaponChoice>)>,
    ) -> Option<Self> {
        let first = *choice.targets.first()?;
        let preview = state
            .preview_attack(sel.unit, sel.dest(), &attack(first, choice.slot))
            .ok()?;
        let choices = art_choices(state, sel.unit, sel.dest(), choice.slot, first);
        let list = art_menu(state, &choices, 0);
        Some(Self {
            sel,
            slot: choice.slot,
            targets: choice.targets.clone(),
            index: 0,
            choices,
            list,
            preview,
            weapons,
        })
    }

    /// The target under the cursor.
    pub fn target(&self) -> UnitId {
        // `index` is always a valid index of `targets`, which is never empty.
        self.targets[self.index]
    }

    /// What the attack is made with: the list's focused line.
    pub fn technique(&self) -> Technique {
        self.choices
            .get(self.list.focus())
            .map(|c| c.technique.clone())
            .unwrap_or_default()
    }

    /// Whether the list has a line besides `Attack` (usable or not); if
    /// not, it isn't shown and Up/Down pick targets.
    pub fn has_list(&self) -> bool {
        self.choices.len() > 1
    }

    /// Moves to the next target (or the previous one), wrapping, and
    /// updates the list and the forecast. The chosen line stays if it can
    /// still be chosen against the new target, else the attack is plain.
    pub fn cycle(&mut self, forward: bool, state: &BattleState) {
        let n = self.targets.len();
        if n == 0 {
            return;
        }
        self.index = if forward {
            (self.index + 1) % n
        } else {
            (self.index + n - 1) % n
        };
        let kept = self.technique();
        self.choices = art_choices(
            state,
            self.sel.unit,
            self.sel.dest(),
            self.slot,
            self.target(),
        );
        let at = self
            .choices
            .iter()
            .position(|c| c.technique == kept && c.usable())
            .unwrap_or(0);
        self.list = art_menu(state, &self.choices, at);
        if !self.refresh(state) {
            self.list = art_menu(state, &self.choices, 0);
            self.refresh(state);
        }
    }

    /// Moves the list's focus down (or up) to the next line that can be
    /// chosen, wrapping, and updates the forecast.
    pub fn move_list(&mut self, down: bool, state: &BattleState) {
        let before = self.list.clone();
        let key = if down {
            Action::CursorDown
        } else {
            Action::CursorUp
        };
        self.list.handle(key);
        if !self.refresh(state) {
            self.list = before;
        }
    }

    /// The forecast for the target with the chosen line; `false` (and the
    /// forecast unchanged) if the core refuses it.
    fn refresh(&mut self, state: &BattleState) -> bool {
        let action = self.technique().action(self.target(), self.slot);
        match state.preview_attack(self.sel.unit, self.sel.dest(), &action) {
            Ok(p) => {
                self.preview = p;
                true
            }
            Err(_) => false,
        }
    }

    /// The command that makes the attack.
    pub fn command(&self) -> trpg_core::Command {
        trpg_core::Command::Act {
            unit: self.sel.unit,
            dest: self.sel.dest(),
            action: self.technique().action(self.target(), self.slot),
        }
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::{Command, Objective};

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::testing::{battle_with, skirmish};

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// The lord of `state` selected with its path to `dest`.
    fn at(state: &BattleState, dest: Pos) -> Selection {
        let mut sel = Selection::new(state, UnitId(1)).unwrap();
        sel.path = vec![sel.origin(), dest];
        sel
    }

    #[test]
    fn targets_are_the_attackable_units_in_row_then_column_order() {
        let c = ctx();
        let s = skirmish(&c, 20);
        // From (7, 2): the raider above (row 1), then the brigand right.
        assert_eq!(targets(&s, UnitId(1), p(7, 2), 0), [UnitId(6), UnitId(4)]);
        // Two brigands either side of (7, 3), in one row: left first.
        let mut units = s.units().to_vec();
        units[3].pos = p(8, 3);
        units[4].pos = p(6, 3);
        let rout = Objective::Rout { turn_limit: None };
        let s = battle_with(&c, s.map().clone(), units, rout);
        assert_eq!(targets(&s, UnitId(1), p(7, 3), 0), [UnitId(5), UnitId(4)]);
        // An empty slot, or a tile the lord can't reach: none.
        assert!(targets(&s, UnitId(1), p(7, 3), 2).is_empty());
        assert!(targets(&s, UnitId(1), p(0, 0), 0).is_empty());
    }

    #[test]
    fn every_weapon_that_reaches_is_a_choice_focused_on_the_equipped_one() {
        let c = ctx();
        let s = skirmish(&c, 20);
        let sel = at(&s, p(7, 2));
        let choices = weapon_choices(&s, &sel);
        let slots: Vec<usize> = choices.iter().map(|w| w.slot).collect();
        assert_eq!(slots, [0, 1]);
        assert!(choices.iter().all(|w| w.targets == [UnitId(6), UnitId(4)]));
        let menu = weapon_menu(&s, &sel, &choices);
        let labels: Vec<&str> = menu.items().iter().map(|i| i.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Iron Sword   Mt  5  Hit  90  Crit  0  Rng 1",
                "Steel Sword  Mt  8  Hit  75  Crit  0  Rng 1",
            ]
        );
        assert_eq!(menu.focus(), 0);
        // With the steel sword equipped, the list starts on it.
        let mut units = s.units().to_vec();
        units[0].loadout.equipped = Some(trpg_core::Equipped::Weapon(1));
        let s2 = battle_with(
            &c,
            s.map().clone(),
            units,
            Objective::Rout { turn_limit: None },
        );
        assert_eq!(weapon_menu(&s2, &sel, &choices).focus(), 1);
        // Nobody in reach: no choices.
        assert!(weapon_choices(&s, &at(&s, p(6, 3))).is_empty());
        // An empty slot has no name or line.
        assert_eq!(weapon_name(&s, UnitId(1), 2), "");
        assert_eq!(weapon_label(&s, UnitId(1), 2, 4), "");
    }

    #[test]
    fn targeting_cycles_both_ways_and_keeps_the_forecast_current() {
        let c = ctx();
        let s = skirmish(&c, 20);
        let sel = at(&s, p(7, 2));
        let choice = &weapon_choices(&s, &sel)[1];
        let mut t = Targeting::new(&s, sel, choice, None).unwrap();
        assert_eq!((t.target(), t.slot), (UnitId(6), 1));
        let forecast_on = |id| {
            s.preview_attack(UnitId(1), p(7, 2), &attack(id, 1))
                .unwrap()
        };
        assert_eq!(t.preview, forecast_on(UnitId(6)));
        t.cycle(true, &s);
        assert_eq!(t.target(), UnitId(4));
        assert_eq!(t.preview, forecast_on(UnitId(4)));
        t.cycle(true, &s);
        assert_eq!(t.target(), UnitId(6));
        t.cycle(false, &s);
        assert_eq!(t.target(), UnitId(4));
        t.cycle(false, &s);
        assert_eq!(t.preview, forecast_on(UnitId(6)));
        assert_eq!(
            t.command(),
            Command::Act {
                unit: UnitId(1),
                dest: p(7, 2),
                action: attack(UnitId(6), 1),
            }
        );
        // Three targets around (7, 3): the raider left, brigand 4 right,
        // brigand 5 below. Back from the first is the last.
        let mut units = s.units().to_vec();
        units[3].pos = p(8, 3);
        units[5].pos = p(6, 3);
        let rout = Objective::Rout { turn_limit: None };
        let s3 = battle_with(&c, s.map().clone(), units, rout);
        let sel3 = at(&s3, p(7, 3));
        let three = &weapon_choices(&s3, &sel3)[0];
        assert_eq!(three.targets, [UnitId(6), UnitId(4), UnitId(5)]);
        let mut t3 = Targeting::new(&s3, sel3, three, None).unwrap();
        t3.cycle(false, &s3);
        assert_eq!(t3.target(), UnitId(5));
        t3.cycle(false, &s3);
        assert_eq!(t3.target(), UnitId(4));
        // No target: no targeting.
        let none = WeaponChoice {
            slot: 0,
            targets: vec![],
        };
        assert_eq!(Targeting::new(&s, at(&s, p(7, 2)), &none, None), None);
    }
}
