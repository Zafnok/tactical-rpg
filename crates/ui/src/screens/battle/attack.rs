//! Choosing an attack (ticket 0404): which weapons can attack someone from
//! where the unit stands, the targets of each, in `(y, x)` order, and the
//! targeting state with its forecast. Every check is
//! [`BattleState::preview_attack`], the same validation the attack command
//! gets, so the UI never decides what is legal itself (ADR-0004).

use trpg_core::{AttackPreview, BattleState, Pos, SkillId, UnitAction, UnitId, WEAPON_SLOTS};

use super::mode::Selection;
use super::skills::{combat_active_ids, combat_actives};
use crate::widgets::menu::{Menu, MenuItem};

/// The weapon attack with the weapon in `slot` on `target`: no art, no
/// active (arts come with 0414).
pub fn attack(target: UnitId, slot: usize) -> UnitAction {
    attack_with(target, slot, None)
}

/// [`attack`] with the combat active `active` (0412).
pub fn attack_with(target: UnitId, slot: usize, active: Option<SkillId>) -> UnitAction {
    UnitAction::Attack {
        target,
        slot,
        active,
        art: None,
    }
}

/// The units `unit` could attack from `dest` with the weapon in `slot`,
/// ordered by `(y, x)` of their tiles.
pub fn targets(state: &BattleState, unit: UnitId, dest: Pos, slot: usize) -> Vec<UnitId> {
    targets_with(state, unit, dest, slot, None)
}

/// [`targets`] with the combat active `active` chosen: an active that adds
/// range (Long Shot) reaches further (0426).
pub fn targets_with(
    state: &BattleState,
    unit: UnitId,
    dest: Pos,
    slot: usize,
    active: Option<&SkillId>,
) -> Vec<UnitId> {
    let mut found: Vec<(Pos, UnitId)> = state
        .units()
        .iter()
        .filter(|u| {
            let action = attack_with(u.id, slot, active.cloned());
            state.preview_attack(unit, dest, &action).is_ok()
        })
        .map(|u| (u.pos, u.id))
        .collect();
    found.sort_by_key(|(p, _)| (p.y, p.x));
    found.into_iter().map(|(_, id)| id).collect()
}

/// Everyone the weapon in `slot` could attack plain or with any combat
/// active, in `(y, x)` order.
fn reachable(state: &BattleState, unit: UnitId, dest: Pos, slot: usize) -> Vec<UnitId> {
    let mut found = targets(state, unit, dest, slot);
    for id in combat_active_ids(state, unit) {
        for t in targets_with(state, unit, dest, slot, Some(&id)) {
            if !found.contains(&t) {
                found.push(t);
            }
        }
    }
    found.sort_by_key(|&id| state.unit(id).map(|u| (u.pos.y, u.pos.x)));
    found
}

/// A weapon that can attack someone from the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponChoice {
    /// Its loadout slot.
    pub slot: usize,
    /// Who it can attack, plain or with a combat active, in `(y, x)` order
    /// (never empty).
    pub targets: Vec<UnitId>,
}

/// The weapons of `sel`'s unit that can attack someone from its path's
/// end, in slot order.
pub fn weapon_choices(state: &BattleState, sel: &Selection) -> Vec<WeaponChoice> {
    (0..WEAPON_SLOTS)
        .filter_map(|slot| {
            let targets = reachable(state, sel.unit, sel.dest(), slot);
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

/// The name of the weapon in `unit`'s `slot` (empty if none).
pub fn weapon_name(state: &BattleState, unit: UnitId, slot: usize) -> String {
    state
        .unit(unit)
        .and_then(|u| u.loadout.weapon(slot))
        .and_then(|w| state.items().weapon(&w.def))
        .map(|d| d.name.clone())
        .unwrap_or_default()
}

/// The weapon list: one line per choice, focused on the equipped weapon if
/// it is one of them.
pub fn weapon_menu(state: &BattleState, sel: &Selection, choices: &[WeaponChoice]) -> Menu {
    let name_w = choices
        .iter()
        .map(|c| weapon_name(state, sel.unit, c.slot).chars().count())
        .max()
        .unwrap_or(0);
    let items = choices
        .iter()
        .map(|c| MenuItem::new(weapon_label(state, sel.unit, c.slot, name_w)))
        .collect();
    let equipped = state.unit(sel.unit).and_then(|u| u.loadout.equipped_slot());
    let focus = choices
        .iter()
        .position(|c| Some(c.slot) == equipped)
        .unwrap_or(0);
    Menu::new(items).focused(focus)
}

/// Picking a target for an attack with one weapon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Targeting {
    /// The attacker and its path (it stands at the path's end).
    pub sel: Selection,
    /// The attacking weapon's loadout slot.
    pub slot: usize,
    /// Who it can attack with the chosen active, in `(y, x)` order (never
    /// empty).
    pub targets: Vec<UnitId>,
    /// The target under the cursor.
    pub index: usize,
    /// The combat active chosen for this attack, if any (0412).
    pub active: Option<SkillId>,
    /// The forecast against it, with the active applied.
    pub preview: AttackPreview,
    /// The weapon list it was opened from (Cancel goes back to it), if the
    /// unit had several weapons to choose from.
    pub weapons: Option<(Menu, Vec<WeaponChoice>)>,
}

impl Targeting {
    /// Targeting the first of `choice`'s targets. `None` if its forecast
    /// can't be made (the battle changed; never while choosing).
    pub fn new(
        state: &BattleState,
        sel: Selection,
        choice: &WeaponChoice,
        weapons: Option<(Menu, Vec<WeaponChoice>)>,
    ) -> Option<Self> {
        choice.targets.first()?;
        let (unit, dest) = (sel.unit, sel.dest());
        // Plain if the weapon reaches anyone; else the first active that
        // extends its reach to someone.
        let plain = targets(state, unit, dest, choice.slot);
        let (active, targets) = if plain.is_empty() {
            combat_active_ids(state, unit).into_iter().find_map(|id| {
                let t = targets_with(state, unit, dest, choice.slot, Some(&id));
                (!t.is_empty()).then_some((Some(id), t))
            })?
        } else {
            (None, plain)
        };
        let first = *targets.first()?;
        let preview = state
            .preview_attack(unit, dest, &attack_with(first, choice.slot, active.clone()))
            .ok()?;
        Some(Self {
            sel,
            slot: choice.slot,
            targets,
            index: 0,
            active,
            preview,
            weapons,
        })
    }

    /// The target under the cursor.
    pub fn target(&self) -> UnitId {
        // `index` is always a valid index of `targets`, which is never empty.
        self.targets[self.index]
    }

    /// Moves to the next target (or the previous one), wrapping, and
    /// updates the forecast.
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
        // The active stays if the core still accepts it against the new
        // target, else the attack is plain.
        let with = |active: Option<SkillId>| {
            let action = attack_with(self.target(), self.slot, active);
            state
                .preview_attack(self.sel.unit, self.sel.dest(), &action)
                .ok()
        };
        if let Some(p) = with(self.active.clone()) {
            self.preview = p;
        } else if let Some(p) = with(None) {
            self.active = None;
            self.preview = p;
        }
    }

    /// The combat actives usable against the target under the cursor.
    pub fn actives(&self, state: &BattleState) -> Vec<SkillId> {
        combat_actives(
            state,
            self.sel.unit,
            self.sel.dest(),
            self.slot,
            self.target(),
        )
    }

    /// Moves to the next combat active (or the previous one) in the ring
    /// `none → Keen Edge → … → none` and updates the forecast. Actives that
    /// can't be paid for are skipped.
    pub fn cycle_skill(&mut self, forward: bool, state: &BattleState) {
        let mut ring: Vec<Option<SkillId>> = vec![None];
        ring.extend(self.actives(state).into_iter().map(Some));
        let n = ring.len();
        let at = ring.iter().position(|a| *a == self.active).unwrap_or(0);
        // The next entry the core accepts against the current target (plain
        // is refused when only an active reaches it).
        for step in 1..n {
            let next = ring[if forward {
                (at + step) % n
            } else {
                (at + n - step) % n
            }]
            .clone();
            let action = attack_with(self.target(), self.slot, next.clone());
            let Ok(p) = state.preview_attack(self.sel.unit, self.sel.dest(), &action) else {
                continue;
            };
            // An active that adds range reaches other targets than plain.
            let (unit, dest) = (self.sel.unit, self.sel.dest());
            let found = targets_with(state, unit, dest, self.slot, next.as_ref());
            let current = self.target();
            self.index = found.iter().position(|&t| t == current).unwrap_or(0);
            self.targets = found;
            self.active = next;
            self.preview = p;
            return;
        }
    }

    /// The command that makes the attack.
    pub fn command(&self) -> trpg_core::Command {
        trpg_core::Command::Act {
            unit: self.sel.unit,
            dest: self.sel.dest(),
            action: attack_with(self.target(), self.slot, self.active.clone()),
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

    /// The skirmish with the archer (unit 3) knowing Long Shot at (8, 4) and
    /// brigand 4 moved to `brigand`.
    fn archer_vs(c: &crate::screen::Ctx, brigand: Pos) -> BattleState {
        let s = skirmish(c, 20);
        let mut units = s.units().to_vec();
        units[2].pos = p(8, 4);
        units[3].pos = brigand;
        assert!(units[2].learn_skill(&SkillId::new("long_shot"), &c.content.skills));
        battle_with(
            c,
            s.map().clone(),
            units,
            Objective::Rout { turn_limit: None },
        )
    }

    #[test]
    fn long_shot_offers_a_target_just_beyond_the_bows_range() {
        let c = ctx();
        // Three tiles up: past the bow's two, within Long Shot's extra one.
        let s = archer_vs(&c, p(8, 1));
        let (archer, brigand) = (UnitId(3), UnitId(4));
        let long = SkillId::new("long_shot");
        assert!(!targets(&s, archer, p(8, 4), 0).contains(&brigand));
        assert!(targets_with(&s, archer, p(8, 4), 0, Some(&long)).contains(&brigand));
        // The weapon is a choice only because of the active; targeting
        // starts with Long Shot on and can attack with it.
        let mut sel = Selection::new(&s, archer).unwrap();
        sel.path = vec![sel.origin()];
        let choices = weapon_choices(&s, &sel);
        assert!(choices[0].targets.contains(&brigand));
        let mut t = Targeting::new(&s, sel, &choices[0], None).unwrap();
        assert_eq!(t.active, Some(long.clone()));
        assert!(t.targets.contains(&brigand));
        while t.target() != brigand {
            t.cycle(true, &s);
        }
        // The only way to attack it is with the active: cycling skills
        // never lands on the plain attack.
        for _ in 0..4 {
            t.cycle_skill(true, &s);
            assert_eq!(t.target(), brigand);
            assert!(t.active.is_some());
        }
        while t.active != Some(long.clone()) {
            t.cycle_skill(true, &s);
        }
        let mut after = s.clone();
        let events = after.apply(&t.command()).unwrap();
        assert!(!events.is_empty());
    }

    #[test]
    fn choosing_long_shot_adds_targets_and_dropping_it_removes_them() {
        let c = ctx();
        // Brigand 4 is two tiles up (in plain reach); Long Shot adds reach
        // without dropping it.
        let s = archer_vs(&c, p(8, 2));
        let mut sel = Selection::new(&s, UnitId(3)).unwrap();
        sel.path = vec![sel.origin()];
        let choice = &weapon_choices(&s, &sel)[0];
        let mut t = Targeting::new(&s, sel, choice, None).unwrap();
        assert_eq!((t.target(), t.active.clone()), (UnitId(4), None));
        let plain = t.targets.clone();
        let long = Some(SkillId::new("long_shot"));
        while t.active != long {
            t.cycle_skill(true, &s);
        }
        assert_eq!(t.target(), UnitId(4));
        assert!(t.targets.len() > plain.len());
        while t.active.is_some() {
            t.cycle_skill(true, &s);
        }
        assert_eq!(t.targets, plain);
        // Without Long Shot known, a target beyond range isn't offered.
        let s = skirmish(&c, 20);
        let mut units = s.units().to_vec();
        units[2].pos = p(8, 4);
        units[3].pos = p(8, 1);
        let s = battle_with(
            &c,
            s.map().clone(),
            units,
            Objective::Rout { turn_limit: None },
        );
        assert!(!targets(&s, UnitId(3), p(8, 4), 0).contains(&UnitId(4)));
    }
}
