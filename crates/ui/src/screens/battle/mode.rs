//! What the player is doing on the battle screen (ticket 0403), the Fire
//! Emblem move loop: browse ([`Mode::Idle`]), select a unit and steer its
//! path ([`Mode::Selected`]), watch it walk ([`Mode::Moving`]), pick an
//! action ([`Mode::ActionMenu`]); to attack (0404), maybe pick a weapon
//! ([`Mode::WeaponMenu`]), pick a target with the forecast
//! ([`Mode::Targeting`]) and watch the combat ([`Mode::Combat`]). The move
//! is only sent, as one
//! [`Command::Act`], when an action is chosen, so cancelling is free: until
//! then the unit's new tile is a drawing override ([`Mode::drawn_pos`]) and
//! the [`BattleState`] is untouched.
//!
//! Pointing the cursor at an enemy the selected unit can attack after moving
//! (0427) aims the path at the tile it will attack from; Confirm walks
//! there and goes straight to the weapon list / forecast on that enemy.
//!
//! Around it (0405): the map menu ([`Mode::MapMenu`], [`Mode::UnitList`],
//! [`Mode::Objective`]), the end-turn prompt ([`Mode::EndTurnPrompt`]) and
//! the unit info screen ([`Mode::Info`]).
//!
//! Transitions are pure ([`step`], [`Mode::tick`]); the screen owns the
//! cursor and applies the [`Effect`]s.

use trpg_core::{
    BattleState, Command, Faction, Phase, Pos, Reach, TileSet, UnitAction, UnitId, attack_tiles,
    path_cost, reachable, threat_area,
};

use super::attack::{Targeting, WeaponChoice, attack_tile, weapon_choices, weapon_menu};
use super::items::{
    EquipChoice, ItemTargeting, PackGroup, can_equip, can_use_item, equip_choices, equip_command,
    equip_menu, pack_groups, pack_menu,
};
use super::map_menu::{MapEntry, map_menu, ready_players, unit_list};
use super::path::steer;
use super::playback::Playback;
use super::skills::{
    SkillChoice, SkillTargeting, can_use_skill, has_skill_menu, skill_choices, skill_menu,
};
use crate::input::Action;
use crate::widgets::menu::{Menu, MenuEvent, MenuItem};

/// Walking speed, in tiles per second. *Tunable.*
pub const WALK_TILES_PER_S: f32 = 12.0;

/// How long Confirm must be held during a walk to skip to its end, in
/// seconds (so the tap that started the walk doesn't skip it). *Tunable.*
pub const HOLD_SKIP_S: f32 = 0.2;

/// A selected unit, its ranges and the path the player is steering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The unit.
    pub unit: UnitId,
    /// Where it can move.
    pub reach: Reach,
    /// Tiles it can move through (drawn in `move_range`).
    pub moves: TileSet,
    /// Tiles it could attack after moving, other than `moves` (drawn in
    /// `attack_range`).
    pub attack: TileSet,
    /// The path arrow: the unit's tile first, never empty.
    pub path: Vec<Pos>,
    /// The enemy under the cursor the path is aimed at (0427), if any.
    pub target: Option<UnitId>,
}

impl Selection {
    /// Selects unit `id`: its reach and ranges in `state`, and a path of
    /// just its tile. `None` if it can't be found or placed.
    pub fn new(state: &BattleState, id: UnitId) -> Option<Self> {
        let unit = state.unit(id)?;
        let reach = reachable(
            state.map(),
            state.terrain(),
            state.classes(),
            state.units(),
            id,
        )
        .ok()?;
        let moves = reach.passable();
        let mut attack = TileSet::new(moves.width(), moves.height());
        for (min, max) in unit.attack_ranges(state.classes(), state.items(), state.spells()) {
            for pos in attack_tiles(&reach, min, max).iter() {
                if !moves.contains(pos) {
                    attack.insert(pos);
                }
            }
        }
        let path = vec![reach.origin()];
        Some(Self {
            unit: id,
            reach,
            moves,
            attack,
            path,
            target: None,
        })
    }

    /// The unit's tile.
    pub fn origin(&self) -> Pos {
        self.reach.origin()
    }

    /// Where the path ends.
    pub fn dest(&self) -> Pos {
        self.path.last().copied().unwrap_or_else(|| self.origin())
    }

    /// Steers the path to `to` ([`steer`]).
    pub fn steer(&mut self, to: Pos, state: &BattleState) {
        let valid = |path: &[Pos]| {
            path_cost(
                state.map(),
                state.terrain(),
                state.classes(),
                state.units(),
                self.unit,
                path,
            )
            .is_ok()
        };
        self.path = steer(&self.path, to, &self.reach, valid);
    }
}

/// One entry of the action menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEntry {
    /// Attack: enabled when a weapon can attack someone from there.
    Attack,
    /// Seize the objective tile, when legal there.
    Seize,
    /// Use a non-combat active skill (0412): shown when the unit knows one.
    Skill,
    /// Use a consumable from the battle pack (0407).
    Item,
    /// Change the equipped weapon (0407).
    Equip,
    /// End the unit's action.
    Wait,
}

impl MenuEntry {
    /// The text shown.
    pub const fn label(self) -> &'static str {
        match self {
            MenuEntry::Attack => "Attack",
            MenuEntry::Seize => "Seize",
            MenuEntry::Skill => "Skill",
            MenuEntry::Item => "Item",
            MenuEntry::Equip => "Equip",
            MenuEntry::Wait => "Wait",
        }
    }

    /// The menu item, disabled if not `enabled`.
    fn item(self, enabled: bool) -> MenuItem {
        if enabled {
            MenuItem::new(self.label())
        } else {
            MenuItem::disabled(self.label())
        }
    }

    /// Whether the entry can be chosen for `sel`'s unit: `Attack` if a
    /// weapon reaches someone, `Item` if the pack has an item usable on
    /// someone, `Equip` with two usable weapons.
    fn enabled(self, sel: &Selection, weapons: &[WeaponChoice], state: &BattleState) -> bool {
        match self {
            MenuEntry::Attack => !weapons.is_empty(),
            MenuEntry::Skill => can_use_skill(&skill_choices(state, sel)),
            MenuEntry::Item => can_use_item(&pack_groups(state, sel.unit, sel.dest())),
            MenuEntry::Equip => can_equip(&equip_choices(state, sel.unit)),
            MenuEntry::Seize | MenuEntry::Wait => true,
        }
    }
}

/// The action menu for `sel`'s unit at its path's end: `Attack` (enabled if
/// some weapon can attack someone there), `Seize` if legal there, `Skill`
/// if the unit knows a non-combat active, `Item` and `Equip` (enabled when
/// they can do something), `Wait`.
pub fn menu_entries(sel: &Selection, state: &BattleState) -> Vec<MenuEntry> {
    let mut entries = vec![MenuEntry::Attack];
    if state.can_seize(sel.unit, sel.dest()) {
        entries.push(MenuEntry::Seize);
    }
    if has_skill_menu(state, sel.unit) {
        entries.push(MenuEntry::Skill);
    }
    entries.extend([MenuEntry::Item, MenuEntry::Equip, MenuEntry::Wait]);
    entries
}

/// An other-faction unit's threat area shown in the browse mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Threat {
    /// The unit.
    pub unit: UnitId,
    /// Every tile it could attack this turn (drawn in `attack_range`).
    pub area: TileSet,
}

/// What the player is doing.
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// Browsing the map, maybe with a unit's threat area shown.
    Idle {
        /// The threat area shown, if any.
        threat: Option<Threat>,
    },
    /// A unit is selected; the cursor steers its path.
    Selected(Selection),
    /// The unit walks its path.
    Moving {
        /// The selection being carried out.
        sel: Selection,
        /// Seconds since the walk started.
        t: f32,
        /// Seconds Confirm has been held without a break.
        held: f32,
    },
    /// The unit stands at its path's end; the player picks an action.
    ActionMenu {
        /// The selection (restored by Cancel).
        sel: Selection,
        /// The menu widget.
        menu: Menu,
        /// What each menu item does.
        entries: Vec<MenuEntry>,
        /// The weapons that can attack from there (`Attack` is enabled if
        /// there are any).
        weapons: Vec<WeaponChoice>,
    },
    /// Several weapons can attack: the player picks one.
    WeaponMenu {
        /// The selection.
        sel: Selection,
        /// The weapon list.
        menu: Menu,
        /// What each item is.
        weapons: Vec<WeaponChoice>,
    },
    /// The player picks a target and reads the forecast.
    Targeting(Box<Targeting>),
    /// The unit's non-combat active skills (0412).
    SkillMenu {
        /// The selection.
        sel: Selection,
        /// The skill list.
        menu: Menu,
        /// What each line is.
        choices: Vec<SkillChoice>,
    },
    /// The player picks who a skill (Shove) is used on.
    SkillTarget(Box<SkillTargeting>),
    /// The battle pack's items, grouped (0407).
    ItemMenu {
        /// The selection.
        sel: Selection,
        /// The pack list.
        menu: Menu,
        /// What each line is.
        groups: Vec<PackGroup>,
    },
    /// The player picks who an item is used on.
    ItemTarget(Box<ItemTargeting>),
    /// The unit's weapons, to equip one (0407).
    EquipMenu {
        /// The selection.
        sel: Selection,
        /// The weapon list.
        menu: Menu,
        /// What each line is.
        choices: Vec<EquipChoice>,
    },
    /// An attack's combat plays out.
    Combat(Box<Playback>),
    /// A unit waits to move after its attack ([`BattleState::pending_move`]):
    /// Confirm on one of `tiles` moves it there, Confirm on the unit stays.
    MoveAfter {
        /// The unit.
        unit: UnitId,
        /// Where it may go ([`BattleState::move_after_tiles`]).
        tiles: Vec<Pos>,
    },
    /// The map menu is open.
    MapMenu {
        /// The menu widget.
        menu: Menu,
        /// What each item is.
        entries: Vec<MapEntry>,
    },
    /// The map menu's list of player units.
    UnitList {
        /// The list.
        menu: Menu,
        /// Which unit each item is.
        units: Vec<UnitId>,
    },
    /// The objective and turn are shown.
    Objective,
    /// `End turn with N units ready?`: `EndTurn` again or Confirm ends the
    /// phase, Cancel backs out.
    EndTurnPrompt {
        /// Units still ready.
        ready: usize,
    },
    /// The info screen for a unit.
    Info {
        /// The unit shown.
        unit: UnitId,
    },
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Idle { threat: None }
    }
}

/// What the screen must do after a [`step`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Apply this command to the battle, then continue from
    /// [`Mode::after_command`].
    Apply(Command),
    /// Apply this command, which doesn't end the unit's action (`Equip`),
    /// then reopen the action menu for the unit, still at the end of the
    /// path.
    ApplyStay(Command, Box<Selection>),
    /// Put the cursor on this tile.
    Cursor(Pos),
}

impl Mode {
    /// The mode after the battle changed: [`Mode::MoveAfter`] if a unit
    /// waits to move after its attack, else browsing.
    pub fn after_command(state: &BattleState) -> Mode {
        match state.pending_move() {
            Some(pending) => Mode::MoveAfter {
                unit: pending.unit,
                tiles: state.move_after_tiles(),
            },
            None => Mode::default(),
        }
    }

    /// Whether the cursor keys move the cursor (in the action menu they
    /// move the menu's focus; during a walk they do nothing).
    pub fn cursor_free(&self) -> bool {
        matches!(
            self,
            Mode::Idle { .. } | Mode::Selected(_) | Mode::MoveAfter { .. }
        )
    }

    /// The cursor moved to `to`: a selected unit's path follows it.
    pub fn cursor_moved(&mut self, to: Pos, state: &BattleState) {
        if let Mode::Selected(sel) = self {
            let enemy = state.units().iter().find(|u| u.pos == to).map(|u| u.id);
            let aim = enemy.and_then(|id| attack_tile(state, sel, id).map(|t| (id, t)));
            if let Some((id, tile)) = aim {
                if tile != sel.dest() {
                    sel.path = sel.reach.path_to(tile).unwrap_or_else(|| sel.path.clone());
                }
                sel.target = Some(id);
            } else {
                sel.target = None;
                sel.steer(to, state);
            }
        }
    }

    /// The selected unit, while one is selected, walking or choosing.
    pub fn selection(&self) -> Option<&Selection> {
        match self {
            Mode::Selected(sel)
            | Mode::Moving { sel, .. }
            | Mode::ActionMenu { sel, .. }
            | Mode::WeaponMenu { sel, .. }
            | Mode::SkillMenu { sel, .. }
            | Mode::ItemMenu { sel, .. }
            | Mode::EquipMenu { sel, .. } => Some(sel),
            Mode::Targeting(t) => Some(&t.sel),
            Mode::SkillTarget(t) => Some(&t.sel),
            Mode::ItemTarget(t) => Some(&t.sel),
            Mode::Idle { .. }
            | Mode::MoveAfter { .. }
            | Mode::Combat(_)
            | Mode::MapMenu { .. }
            | Mode::UnitList { .. }
            | Mode::Objective
            | Mode::EndTurnPrompt { .. }
            | Mode::Info { .. } => None,
        }
    }

    /// Where unit `id` is drawn, if not on its own tile: along its path
    /// while it walks, at the path's end while it chooses an action, a
    /// weapon or a target.
    pub fn drawn_pos(&self, id: UnitId) -> Option<Pos> {
        match self {
            Mode::Moving { sel, t, .. } if sel.unit == id => Some(walk_pos(sel, *t)),
            Mode::ActionMenu { sel, .. }
            | Mode::WeaponMenu { sel, .. }
            | Mode::SkillMenu { sel, .. }
            | Mode::ItemMenu { sel, .. }
            | Mode::EquipMenu { sel, .. }
                if sel.unit == id =>
            {
                Some(sel.dest())
            }
            Mode::Targeting(t) if t.sel.unit == id => Some(t.sel.dest()),
            Mode::SkillTarget(t) if t.sel.unit == id => Some(t.sel.dest()),
            Mode::ItemTarget(t) if t.sel.unit == id => Some(t.sel.dest()),
            _ => None,
        }
    }

    /// Advances a walk by `dt` seconds (`confirm_held`: Confirm is down this
    /// frame). The walk ends, opening the action menu, once the unit
    /// reaches the path's end or Confirm has been held [`HOLD_SKIP_S`].
    /// Other modes are unchanged.
    #[must_use]
    pub fn tick(self, dt: f32, confirm_held: bool, state: &BattleState) -> Mode {
        if let Mode::Combat(mut playback) = self {
            playback.tick(dt, confirm_held);
            return if playback.done() {
                Mode::after_command(state)
            } else {
                Mode::Combat(playback)
            };
        }
        let Mode::Moving { sel, t, held } = self else {
            return self;
        };
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let (t, held) = (t + dt, if confirm_held { held + dt } else { 0.0 });
        if held >= HOLD_SKIP_S || walk_done(&sel, t) {
            walk_ended(sel, state).0
        } else {
            Mode::Moving { sel, t, held }
        }
    }
}

/// How many of `sel`'s path steps a walk has taken after `t` seconds (at
/// most all of them; none for a negative or NaN time).
fn walk_steps(sel: &Selection, t: f32) -> usize {
    let walked = t * WALK_TILES_PER_S;
    let steps = sel.path.len().saturating_sub(1);
    (1..=u16::try_from(steps).unwrap_or(u16::MAX))
        .take_while(|&k| f32::from(k) <= walked)
        .count()
}

/// Whether the walk along `sel`'s path is over after `t` seconds.
fn walk_done(sel: &Selection, t: f32) -> bool {
    walk_steps(sel, t) + 1 >= sel.path.len()
}

/// The tile a walk along `sel`'s path is on after `t` seconds.
fn walk_pos(sel: &Selection, t: f32) -> Pos {
    sel.path
        .get(walk_steps(sel, t))
        .copied()
        .unwrap_or_else(|| sel.origin())
}

/// The target of `sel`'s aimed path, if the cursor is still on it.
fn aimed_at(sel: &Selection, cursor: Pos, state: &BattleState) -> Option<UnitId> {
    let t = sel.target?;
    (state.unit(t)?.pos == cursor).then_some(t)
}

/// A walk ended: the attack on the aimed enemy (0427), else the action menu.
fn walk_ended(sel: Selection, state: &BattleState) -> (Mode, Effect) {
    if sel.target.is_some() {
        open_attack(sel, state)
    } else {
        (open_menu(sel, state), Effect::None)
    }
}

/// The unit stands at the path's end, aimed at an enemy (0427): the weapons
/// that reach it (a list if several, its target first), then the forecast on
/// it. The action menu if none does.
fn open_attack(sel: Selection, state: &BattleState) -> (Mode, Effect) {
    let Some(target) = sel.target else {
        return (open_menu(sel, state), Effect::None);
    };
    let mut kept: Vec<WeaponChoice> = weapon_choices(state, &sel)
        .into_iter()
        .filter(|c| c.targets.contains(&target))
        .collect();
    for c in &mut kept {
        if let Some(i) = c.targets.iter().position(|&t| t == target) {
            c.targets.rotate_left(i);
        }
    }
    match kept.as_slice() {
        [] => (open_menu(sel, state), Effect::None),
        [only] => {
            let only = only.clone();
            target_with(state, sel, &only, None)
        }
        _ => {
            let menu = weapon_menu(state, &sel, &kept);
            let weapons = kept;
            (Mode::WeaponMenu { sel, menu, weapons }, Effect::None)
        }
    }
}

/// The action menu for `sel`.
pub(super) fn open_menu(sel: Selection, state: &BattleState) -> Mode {
    let entries = menu_entries(&sel, state);
    let weapons = weapon_choices(state, &sel);
    let items: Vec<MenuItem> = entries
        .iter()
        .map(|e| e.item(e.enabled(&sel, &weapons, state)))
        .collect();
    // It opens on the first of Attack and Seize that is enabled, else Wait:
    // Item and Equip are never the default.
    let first = entries
        .iter()
        .position(|e| {
            matches!(e, MenuEntry::Attack | MenuEntry::Seize) && e.enabled(&sel, &weapons, state)
        })
        .or_else(|| entries.iter().position(|&e| e == MenuEntry::Wait));
    let menu = Menu::new(items).focused(first.unwrap_or(0));
    Mode::ActionMenu {
        sel,
        menu,
        entries,
        weapons,
    }
}

/// The action menu for `sel`, focused on `Attack` (back from choosing an
/// attack).
fn back_to_menu(sel: Selection, state: &BattleState) -> Mode {
    back_to_entry(sel, state, MenuEntry::Attack)
}

/// The action menu for `sel`, focused on `entry` (back from what it opened).
pub(super) fn back_to_entry(sel: Selection, state: &BattleState, entry: MenuEntry) -> Mode {
    let mode = open_menu(sel, state);
    let Mode::ActionMenu {
        sel,
        menu,
        entries,
        weapons,
    } = mode
    else {
        return mode;
    };
    let at = entries.iter().position(|&e| e == entry);
    Mode::ActionMenu {
        sel,
        menu: at.map_or(menu.clone(), |i| menu.focused(i)),
        entries,
        weapons,
    }
}

/// Targeting with `choice` (from the weapon list `weapons`, if any), the
/// cursor on its first target; the action menu if no forecast can be made
/// (only if the battle changed under the menu, which it doesn't).
fn target_with(
    state: &BattleState,
    sel: Selection,
    choice: &WeaponChoice,
    weapons: Option<(Menu, Vec<WeaponChoice>)>,
) -> (Mode, Effect) {
    match Targeting::new(state, sel.clone(), choice, weapons) {
        Some(t) => {
            let at = state.unit(t.target()).map_or(sel.dest(), |u| u.pos);
            (Mode::Targeting(Box::new(t)), Effect::Cursor(at))
        }
        None => (back_to_menu(sel, state), Effect::None),
    }
}

/// Whether `unit` belongs to the player and can still act this phase.
fn is_ready_player(state: &BattleState, id: UnitId) -> bool {
    state.unit(id).is_some_and(|u| {
        u.faction == Faction::Player && Phase::of(u.faction) == state.phase() && !u.acted
    })
}

/// The threat area of unit `id` ([`threat_area`] with its attack ranges).
fn threat_of(state: &BattleState, id: UnitId) -> Option<Threat> {
    let unit = state.unit(id)?;
    let ranges = unit.attack_ranges(state.classes(), state.items(), state.spells());
    let area = threat_area(
        state.map(),
        state.terrain(),
        state.classes(),
        state.units(),
        id,
        &ranges,
    )
    .ok()?;
    Some(Threat { unit: id, area })
}

/// One non-cursor action in `mode`, with the cursor on `cursor`. Cursor
/// movement and unit cycling are the screen's; see [`Mode::cursor_moved`].
pub fn step(mode: Mode, action: Action, cursor: Pos, state: &BattleState) -> (Mode, Effect) {
    match mode {
        Mode::Idle { threat } => step_idle(threat, action, cursor, state),
        Mode::Selected(sel) => match action {
            Action::Confirm if aimed_at(&sel, cursor, state).is_some() => {
                if sel.path.len() == 1 {
                    open_attack(sel, state)
                } else {
                    let walk = Mode::Moving {
                        sel,
                        t: 0.0,
                        held: 0.0,
                    };
                    (walk, Effect::None)
                }
            }
            Action::Confirm if cursor == sel.dest() && sel.reach.is_stoppable(cursor) => {
                if sel.path.len() == 1 {
                    (open_menu(sel, state), Effect::None)
                } else {
                    let walk = Mode::Moving {
                        sel,
                        t: 0.0,
                        held: 0.0,
                    };
                    (walk, Effect::None)
                }
            }
            Action::Cancel => {
                let origin = sel.origin();
                (Mode::default(), Effect::Cursor(origin))
            }
            _ => (Mode::Selected(sel), Effect::None),
        },
        Mode::Moving { sel, .. } if action == Action::Confirm => walk_ended(sel, state),
        moving @ Mode::Moving { .. } => (moving, Effect::None),
        Mode::ActionMenu {
            sel,
            mut menu,
            entries,
            weapons,
        } => match menu.handle(action) {
            Some(MenuEvent::Chosen(i)) => {
                let action = match entries.get(i) {
                    Some(MenuEntry::Wait) => UnitAction::Wait,
                    Some(MenuEntry::Seize) => UnitAction::Seize,
                    Some(MenuEntry::Attack) => return choose_attack(sel, weapons, state),
                    Some(MenuEntry::Skill) => return open_skills(sel, state),
                    Some(MenuEntry::Item) => return open_pack(sel, state),
                    Some(MenuEntry::Equip) => return open_equip(sel, state),
                    None => {
                        let menu = Mode::ActionMenu {
                            sel,
                            menu,
                            entries,
                            weapons,
                        };
                        return (menu, Effect::None);
                    }
                };
                let cmd = Command::Act {
                    unit: sel.unit,
                    dest: sel.dest(),
                    action,
                };
                (Mode::default(), Effect::Apply(cmd))
            }
            Some(MenuEvent::Cancelled) => {
                let mut sel = sel;
                sel.target = None;
                (Mode::Selected(sel), Effect::None)
            }
            None => {
                let menu = Mode::ActionMenu {
                    sel,
                    menu,
                    entries,
                    weapons,
                };
                (menu, Effect::None)
            }
        },
        Mode::WeaponMenu { sel, menu, weapons } => {
            step_weapon_menu(sel, menu, weapons, action, state)
        }
        Mode::Targeting(t) => step_targeting(*t, action, state),
        Mode::SkillMenu { sel, menu, choices } => step_skills(sel, menu, choices, action, state),
        Mode::SkillTarget(t) => step_skill_target(*t, action, state),
        Mode::ItemMenu { sel, menu, groups } => step_pack(sel, menu, groups, action, state),
        Mode::ItemTarget(t) => step_item_target(*t, action, state),
        Mode::EquipMenu { sel, menu, choices } => step_equip(sel, menu, choices, action, state),
        Mode::Combat(mut playback) => {
            if action == Action::Cancel {
                playback.skip();
            }
            (Mode::Combat(playback), Effect::None)
        }
        Mode::MoveAfter { unit, tiles } => step_move_after(unit, tiles, action, cursor, state),
        other => step_around(other, action, state),
    }
}

/// [`step`] while a unit waits to move after its attack: Confirm on one of
/// `tiles` moves it there, Confirm on the unit stays.
fn step_move_after(
    unit: UnitId,
    tiles: Vec<Pos>,
    action: Action,
    cursor: Pos,
    state: &BattleState,
) -> (Mode, Effect) {
    let here = state.unit(unit).map(|u| u.pos);
    let to = if Some(cursor) == here {
        Some(None)
    } else if tiles.contains(&cursor) {
        Some(Some(cursor))
    } else {
        None
    };
    match to {
        Some(to) if action == Action::Confirm => (
            Mode::default(),
            Effect::Apply(Command::MoveAfter { unit, to }),
        ),
        _ => (Mode::MoveAfter { unit, tiles }, Effect::None),
    }
}

/// [`step`] in the modes around the move loop: the map menu and what it
/// opens, the end-turn prompt, the info screen.
fn step_around(mode: Mode, action: Action, state: &BattleState) -> (Mode, Effect) {
    match mode {
        Mode::MapMenu { menu, entries } => step_map_menu(menu, entries, action, state),
        Mode::UnitList { mut menu, units } => match menu.handle(action) {
            Some(MenuEvent::Chosen(i)) => {
                let at = units.get(i).and_then(|&id| state.unit(id)).map(|u| u.pos);
                (Mode::default(), at.map_or(Effect::None, Effect::Cursor))
            }
            Some(MenuEvent::Cancelled) => (open_map_menu(state, MapEntry::Units), Effect::None),
            None => (Mode::UnitList { menu, units }, Effect::None),
        },
        Mode::Objective => match action {
            Action::Confirm | Action::Cancel => {
                (open_map_menu(state, MapEntry::Objective), Effect::None)
            }
            _ => (Mode::Objective, Effect::None),
        },
        Mode::EndTurnPrompt { ready } => match action {
            Action::EndTurn | Action::Confirm => {
                (Mode::default(), Effect::Apply(Command::EndPhase))
            }
            Action::Cancel => (Mode::default(), Effect::None),
            _ => (Mode::EndTurnPrompt { ready }, Effect::None),
        },
        Mode::Info { unit } => step_info(unit, action, state),
        other => (other, Effect::None),
    }
}

/// The map menu, focused on `focus`.
fn open_map_menu(state: &BattleState, focus: MapEntry) -> Mode {
    let (menu, entries) = map_menu(state, focus);
    Mode::MapMenu { menu, entries }
}

/// Ending the turn: the prompt if player units are still ready, else
/// `EndPhase` at once.
fn end_turn(state: &BattleState) -> (Mode, Effect) {
    match ready_players(state) {
        0 => (Mode::default(), Effect::Apply(Command::EndPhase)),
        ready => (Mode::EndTurnPrompt { ready }, Effect::None),
    }
}

/// [`step`] in the map menu.
fn step_map_menu(
    mut menu: Menu,
    entries: Vec<MapEntry>,
    action: Action,
    state: &BattleState,
) -> (Mode, Effect) {
    match menu.handle(action) {
        Some(MenuEvent::Chosen(i)) => match entries.get(i) {
            Some(MapEntry::Units) => {
                let (menu, units) = unit_list(state);
                (Mode::UnitList { menu, units }, Effect::None)
            }
            Some(MapEntry::Objective) => (Mode::Objective, Effect::None),
            Some(MapEntry::EndTurn) => end_turn(state),
            // Disabled: the menu never chooses them.
            Some(MapEntry::Options | MapEntry::Suspend) | None => {
                (Mode::MapMenu { menu, entries }, Effect::None)
            }
        },
        Some(MenuEvent::Cancelled) => (Mode::default(), Effect::None),
        None => (Mode::MapMenu { menu, entries }, Effect::None),
    }
}

/// The units the info screen cycles through from `id`: its faction's, in
/// reading order `(y, x)`.
fn same_faction(state: &BattleState, id: UnitId) -> Vec<UnitId> {
    let Some(faction) = state.unit(id).map(|u| u.faction) else {
        return vec![];
    };
    let mut units: Vec<_> = state
        .units()
        .iter()
        .filter(|u| u.faction == faction)
        .collect();
    units.sort_by_key(|u| (u.pos.y, u.pos.x));
    units.iter().map(|u| u.id).collect()
}

/// [`step`] on the info screen: Down or `NextUnit` shows the next unit of
/// the same faction, Up or `PrevUnit` the previous one (wrapping); Cancel
/// or Info closes it with the cursor on the unit last shown.
fn step_info(unit: UnitId, action: Action, state: &BattleState) -> (Mode, Effect) {
    let forward = match action {
        Action::CursorDown | Action::NextUnit => true,
        Action::CursorUp | Action::PrevUnit => false,
        Action::Cancel | Action::Info => {
            let at = state.unit(unit).map(|u| u.pos);
            return (Mode::default(), at.map_or(Effect::None, Effect::Cursor));
        }
        _ => return (Mode::Info { unit }, Effect::None),
    };
    let units = same_faction(state, unit);
    let n = units.len();
    let next = units.iter().position(|&u| u == unit).map_or(unit, |i| {
        let j = if forward {
            (i + 1) % n
        } else {
            (i + n - 1) % n
        };
        units[j]
    });
    (Mode::Info { unit: next }, Effect::None)
}

/// Attack chosen in the action menu: the weapon list if several weapons
/// can attack, else straight to targeting.
fn choose_attack(
    sel: Selection,
    weapons: Vec<WeaponChoice>,
    state: &BattleState,
) -> (Mode, Effect) {
    match weapons.as_slice() {
        [] => (back_to_menu(sel, state), Effect::None),
        [only] => {
            let only = only.clone();
            target_with(state, sel, &only, None)
        }
        _ => {
            let menu = weapon_menu(state, &sel, &weapons);
            (Mode::WeaponMenu { sel, menu, weapons }, Effect::None)
        }
    }
}

/// [`step`] in the weapon list: Confirm targets with the chosen weapon,
/// Cancel goes back to the action menu.
fn step_weapon_menu(
    sel: Selection,
    mut menu: Menu,
    weapons: Vec<WeaponChoice>,
    action: Action,
    state: &BattleState,
) -> (Mode, Effect) {
    match menu.handle(action) {
        Some(MenuEvent::Chosen(i)) => match weapons.get(i).cloned() {
            Some(choice) => target_with(state, sel, &choice, Some((menu, weapons))),
            None => (Mode::WeaponMenu { sel, menu, weapons }, Effect::None),
        },
        Some(MenuEvent::Cancelled) => (back_to_menu(sel, state), Effect::None),
        None => (Mode::WeaponMenu { sel, menu, weapons }, Effect::None),
    }
}

/// `Skill` chosen: the list of non-combat actives (the action menu again if
/// none can be used, which the enabled entry rules out).
fn open_skills(sel: Selection, state: &BattleState) -> (Mode, Effect) {
    let choices = skill_choices(state, &sel);
    if !can_use_skill(&choices) {
        return (back_to_entry(sel, state, MenuEntry::Skill), Effect::None);
    }
    let menu = skill_menu(state, sel.unit, &choices);
    (Mode::SkillMenu { sel, menu, choices }, Effect::None)
}

/// [`step`] in the skill list: Confirm uses a skill that needs no target
/// at once and picks who to use Shove on otherwise, Cancel goes back to the
/// action menu.
fn step_skills(
    sel: Selection,
    mut menu: Menu,
    choices: Vec<SkillChoice>,
    action: Action,
    state: &BattleState,
) -> (Mode, Effect) {
    match menu.handle(action) {
        Some(MenuEvent::Chosen(i)) => {
            let Some(choice) = choices.get(i).filter(|c| c.usable) else {
                return (Mode::SkillMenu { sel, menu, choices }, Effect::None);
            };
            if !choice.needs_target {
                let cmd = Command::Act {
                    unit: sel.unit,
                    dest: sel.dest(),
                    action: UnitAction::UseSkill {
                        skill: choice.skill.clone(),
                        target: None,
                    },
                };
                return (Mode::default(), Effect::Apply(cmd));
            }
            match SkillTargeting::new(sel.clone(), menu.clone(), choices.clone(), i) {
                Some(t) => {
                    let at = state.unit(t.target()).map(|u| u.pos);
                    (
                        Mode::SkillTarget(Box::new(t)),
                        at.map_or(Effect::None, Effect::Cursor),
                    )
                }
                None => (Mode::SkillMenu { sel, menu, choices }, Effect::None),
            }
        }
        Some(MenuEvent::Cancelled) => (back_to_entry(sel, state, MenuEntry::Skill), Effect::None),
        None => (Mode::SkillMenu { sel, menu, choices }, Effect::None),
    }
}

/// [`step`] while picking a skill's target: the cursor keys and
/// `NextUnit`/`PrevUnit` cycle the targets, Confirm uses the skill, Cancel
/// goes back to the skill list with the cursor on the unit.
fn step_skill_target(mut t: SkillTargeting, action: Action, state: &BattleState) -> (Mode, Effect) {
    let forward = match action {
        Action::CursorRight | Action::CursorDown | Action::NextUnit => true,
        Action::CursorLeft | Action::CursorUp | Action::PrevUnit => false,
        Action::Confirm => return (Mode::default(), Effect::Apply(t.command())),
        Action::Cancel => {
            let dest = t.sel.dest();
            let back = Mode::SkillMenu {
                sel: t.sel,
                menu: t.menu,
                choices: t.choices,
            };
            return (back, Effect::Cursor(dest));
        }
        _ => return (Mode::SkillTarget(Box::new(t)), Effect::None),
    };
    t.cycle(forward);
    let at = state.unit(t.target()).map(|u| u.pos);
    (
        Mode::SkillTarget(Box::new(t)),
        at.map_or(Effect::None, Effect::Cursor),
    )
}

/// `Item` chosen: the pack list (the action menu again if it has nothing
/// usable, which the enabled entry rules out).
fn open_pack(sel: Selection, state: &BattleState) -> (Mode, Effect) {
    let groups = pack_groups(state, sel.unit, sel.dest());
    if !can_use_item(&groups) {
        return (back_to_entry(sel, state, MenuEntry::Item), Effect::None);
    }
    let menu = pack_menu(state, &groups);
    (Mode::ItemMenu { sel, menu, groups }, Effect::None)
}

/// `Equip` chosen: the weapon list.
fn open_equip(sel: Selection, state: &BattleState) -> (Mode, Effect) {
    let choices = equip_choices(state, sel.unit);
    if !can_equip(&choices) {
        return (back_to_entry(sel, state, MenuEntry::Equip), Effect::None);
    }
    let menu = equip_menu(state, sel.unit, &choices);
    (Mode::EquipMenu { sel, menu, choices }, Effect::None)
}

/// Where the cursor goes to show `target`, the user of an item at `sel`'s
/// path end or another unit.
fn target_tile(state: &BattleState, sel: &Selection, target: UnitId) -> Option<Pos> {
    if target == sel.unit {
        Some(sel.dest())
    } else {
        state.unit(target).map(|u| u.pos)
    }
}

/// [`step`] in the pack list: Confirm picks who to use the item on, Cancel
/// goes back to the action menu.
fn step_pack(
    sel: Selection,
    mut menu: Menu,
    groups: Vec<PackGroup>,
    action: Action,
    state: &BattleState,
) -> (Mode, Effect) {
    match menu.handle(action) {
        Some(MenuEvent::Chosen(i)) => {
            match ItemTargeting::new(sel.clone(), menu.clone(), groups.clone(), i) {
                Some(t) => {
                    let at = target_tile(state, &sel, t.target());
                    (
                        Mode::ItemTarget(Box::new(t)),
                        at.map_or(Effect::None, Effect::Cursor),
                    )
                }
                None => (Mode::ItemMenu { sel, menu, groups }, Effect::None),
            }
        }
        Some(MenuEvent::Cancelled) => (back_to_entry(sel, state, MenuEntry::Item), Effect::None),
        None => (Mode::ItemMenu { sel, menu, groups }, Effect::None),
    }
}

/// [`step`] while picking an item's target: the cursor keys and
/// `NextUnit`/`PrevUnit` cycle the targets, Confirm uses the item, Cancel
/// goes back to the pack list with the cursor on the unit.
fn step_item_target(mut t: ItemTargeting, action: Action, state: &BattleState) -> (Mode, Effect) {
    let forward = match action {
        Action::CursorRight | Action::CursorDown | Action::NextUnit => true,
        Action::CursorLeft | Action::CursorUp | Action::PrevUnit => false,
        Action::Confirm => return (Mode::default(), Effect::Apply(t.command())),
        Action::Cancel => {
            let dest = t.sel.dest();
            let back = Mode::ItemMenu {
                sel: t.sel,
                menu: t.menu,
                groups: t.groups,
            };
            return (back, Effect::Cursor(dest));
        }
        _ => return (Mode::ItemTarget(Box::new(t)), Effect::None),
    };
    t.cycle(forward);
    let at = target_tile(state, &t.sel, t.target());
    (
        Mode::ItemTarget(Box::new(t)),
        at.map_or(Effect::None, Effect::Cursor),
    )
}

/// [`step`] in the weapon list: Confirm equips the weapon and reopens the
/// action menu (the unit hasn't acted), Cancel goes back without changing
/// anything.
fn step_equip(
    sel: Selection,
    mut menu: Menu,
    choices: Vec<EquipChoice>,
    action: Action,
    state: &BattleState,
) -> (Mode, Effect) {
    match menu.handle(action) {
        Some(MenuEvent::Chosen(i)) => match choices.get(i) {
            // (The menu never chooses a dimmed, unusable weapon.)
            Some(c) => {
                let cmd = equip_command(sel.unit, c.slot);
                (Mode::default(), Effect::ApplyStay(cmd, Box::new(sel)))
            }
            None => (Mode::EquipMenu { sel, menu, choices }, Effect::None),
        },
        Some(MenuEvent::Cancelled) => (back_to_entry(sel, state, MenuEntry::Equip), Effect::None),
        None => (Mode::EquipMenu { sel, menu, choices }, Effect::None),
    }
}

/// [`step`] while targeting: left, right and `NextUnit`/`PrevUnit` cycle the
/// targets (right and next go forward, left and previous go back); up and
/// down cycle the combat actives (0412) when there are any, else the
/// targets too; Confirm attacks, Cancel goes back to the weapon list or the
/// action menu with the cursor on the unit.
fn step_targeting(mut t: Targeting, action: Action, state: &BattleState) -> (Mode, Effect) {
    // Up and Down move a menu-style cursor over the skills, when the unit
    // has any to choose from; else they cycle the targets like Left/Right.
    let skills = !t.actives(state).is_empty();
    let forward = match action {
        Action::CursorUp | Action::CursorDown if skills => {
            t.cycle_skill(action == Action::CursorDown, state);
            return (Mode::Targeting(Box::new(t)), Effect::None);
        }
        Action::CursorRight | Action::CursorDown | Action::NextUnit => true,
        Action::CursorLeft | Action::CursorUp | Action::PrevUnit => false,
        Action::Confirm => return (Mode::default(), Effect::Apply(t.command())),
        Action::Cancel => {
            let dest = t.sel.dest();
            let back = match t.weapons {
                Some((menu, weapons)) => Mode::WeaponMenu {
                    sel: t.sel,
                    menu,
                    weapons,
                },
                None => back_to_menu(t.sel, state),
            };
            return (back, Effect::Cursor(dest));
        }
        _ => return (Mode::Targeting(Box::new(t)), Effect::None),
    };
    t.cycle(forward, state);
    let effect = state
        .unit(t.target())
        .map_or(Effect::None, |u| Effect::Cursor(u.pos));
    (Mode::Targeting(Box::new(t)), effect)
}

/// [`step`] while browsing: Confirm selects a ready player unit or toggles
/// another faction's unit's threat area, and opens the map menu on an empty
/// tile; Cancel hides the threat area, or opens the map menu if none is
/// shown; `Info` opens the info screen of the unit under the cursor;
/// `EndTurn` ends the phase (asking first if units are still ready).
fn step_idle(
    threat: Option<Threat>,
    action: Action,
    cursor: Pos,
    state: &BattleState,
) -> (Mode, Effect) {
    let idle = |threat| (Mode::Idle { threat }, Effect::None);
    match action {
        Action::Confirm => {
            let Some(unit) = state.units().iter().find(|u| u.pos == cursor) else {
                return (open_map_menu(state, MapEntry::Units), Effect::None);
            };
            if is_ready_player(state, unit.id) {
                return match Selection::new(state, unit.id) {
                    Some(sel) => (Mode::Selected(sel), Effect::None),
                    None => idle(threat),
                };
            }
            if unit.faction == Faction::Player {
                return idle(threat);
            }
            if threat.as_ref().is_some_and(|t| t.unit == unit.id) {
                idle(None)
            } else {
                idle(threat_of(state, unit.id))
            }
        }
        Action::Cancel if threat.is_some() => idle(None),
        Action::Cancel | Action::Menu => (open_map_menu(state, MapEntry::Units), Effect::None),
        Action::Info => match state.units().iter().find(|u| u.pos == cursor) {
            Some(unit) => (Mode::Info { unit: unit.id }, Effect::None),
            None => idle(threat),
        },
        Action::EndTurn => end_turn(state),
        _ => idle(threat),
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::Objective;

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::attack::can_hit;
    use crate::screens::battle::playback::TIMINGS;
    use crate::screens::battle::quick_battle;
    use crate::screens::battle::testing::{battle_with, skirmish, vaulted, wait};

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// The Quick Battle: lord 1 at (3, 5), knight 2 at (4, 6), archer 3 at
    /// (2, 4); brigands 4 at (8, 2) and 5 at (7, 4), raider 6 at (7, 1).
    fn quick() -> BattleState {
        quick_battle(&ctx().content).unwrap()
    }

    /// The Quick Battle with `objective`.
    fn with_objective(objective: Objective) -> BattleState {
        let c = ctx();
        let q = quick();
        battle_with(&c, q.map().clone(), q.units().to_vec(), objective)
    }

    /// `actions` from `mode` with the cursor on `cursor`, stopping at the
    /// first effect other than `None`.
    fn run(mode: Mode, actions: &[Action], cursor: Pos, s: &BattleState) -> (Mode, Effect) {
        let mut mode = mode;
        for &a in actions {
            let (m, effect) = step(mode, a, cursor, s);
            if effect != Effect::None {
                return (m, effect);
            }
            mode = m;
        }
        (mode, Effect::None)
    }

    /// The lord selected, with the cursor (and path) steered through
    /// `cursor`.
    fn selected(s: &BattleState, cursor: &[Pos]) -> Selection {
        let (Mode::Selected(mut sel), Effect::None) =
            step(Mode::default(), Action::Confirm, p(3, 5), s)
        else {
            panic!("the lord isn't selected");
        };
        for &to in cursor {
            sel.steer(to, s);
        }
        sel
    }

    #[test]
    fn confirm_on_a_ready_player_unit_selects_it_with_its_ranges() {
        let s = quick();
        let sel = selected(&s, &[]);
        assert_eq!((sel.unit, sel.path.clone()), (UnitId(1), vec![p(3, 5)]));
        assert_eq!((sel.origin(), sel.dest()), (p(3, 5), p(3, 5)));
        // Mov 5, through the knight's tile.
        assert!(sel.moves.contains(p(3, 5)) && sel.moves.contains(p(4, 6)));
        assert!(sel.moves.contains(p(5, 5)) && !sel.moves.contains(p(10, 5)));
        assert!(!sel.attack.is_empty());
        for pos in sel.attack.iter() {
            assert!(!sel.moves.contains(pos), "{pos:?} is in both");
        }
        // The knight's tile is passable but not a place to stop.
        assert!(!sel.reach.is_stoppable(p(4, 6)));
    }

    #[test]
    fn confirm_elsewhere_while_browsing_does_nothing() {
        let mut s = quick();
        // A player unit that has acted.
        wait(&mut s, 2);
        assert_eq!(
            step(Mode::default(), Action::Confirm, p(2, 4), &s),
            (Mode::default(), Effect::None)
        );
        // Other keys.
        for a in [Action::Info, Action::DangerZone, Action::EndTurn] {
            assert_eq!(step(Mode::default(), a, p(3, 5), &s).1, Effect::None);
        }
    }

    #[test]
    fn confirm_moves_only_to_a_stoppable_path_end() {
        let s = quick();
        // Onto the knight: passable, not stoppable. Ignored.
        let sel = selected(&s, &[p(4, 5), p(4, 6)]);
        assert_eq!(sel.dest(), p(4, 6));
        let (mode, effect) = step(Mode::Selected(sel.clone()), Action::Confirm, p(4, 6), &s);
        assert_eq!((mode, effect), (Mode::Selected(sel), Effect::None));
        // The cursor off the path's end (on an unreachable tile). Ignored.
        let sel = selected(&s, &[p(4, 5)]);
        let (mode, _) = step(Mode::Selected(sel.clone()), Action::Confirm, p(13, 0), &s);
        assert_eq!(mode, Mode::Selected(sel));
        // On an empty reachable tile: the walk starts.
        let sel = selected(&s, &[p(4, 5), p(5, 5)]);
        let (mode, effect) = step(Mode::Selected(sel.clone()), Action::Confirm, p(5, 5), &s);
        assert_eq!(effect, Effect::None);
        assert_eq!(
            mode,
            Mode::Moving {
                sel,
                t: 0.0,
                held: 0.0
            }
        );
        // On the unit itself: straight to the menu.
        let sel = selected(&s, &[]);
        let (mode, _) = step(Mode::Selected(sel), Action::Confirm, p(3, 5), &s);
        assert!(matches!(mode, Mode::ActionMenu { .. }), "{mode:?}");
    }

    #[test]
    fn cancel_while_selected_puts_the_cursor_back_on_the_unit() {
        let s = quick();
        let sel = selected(&s, &[p(4, 5), p(5, 5)]);
        assert_eq!(
            step(Mode::Selected(sel.clone()), Action::Cancel, p(5, 5), &s),
            (Mode::default(), Effect::Cursor(p(3, 5)))
        );
        // Other keys keep the selection.
        assert_eq!(
            step(Mode::Selected(sel.clone()), Action::Info, p(5, 5), &s),
            (Mode::Selected(sel), Effect::None)
        );
    }

    #[test]
    fn the_walk_steps_at_twelve_tiles_a_second_then_opens_the_menu() {
        let s = quick();
        let sel = selected(&s, &[p(4, 5), p(5, 5), p(6, 5)]);
        let mut mode = Mode::Moving {
            sel,
            t: 0.0,
            held: 0.0,
        };
        let lord = UnitId(1);
        assert_eq!(mode.drawn_pos(lord), Some(p(3, 5)));
        assert_eq!(mode.drawn_pos(UnitId(2)), None);
        assert!(!mode.cursor_free());
        let tick = 1.0 / WALK_TILES_PER_S;
        mode = mode.tick(tick * 1.01, false, &s);
        assert_eq!(mode.drawn_pos(lord), Some(p(4, 5)));
        mode = mode.tick(tick, false, &s);
        assert_eq!(mode.drawn_pos(lord), Some(p(5, 5)));
        // A bad frame time changes nothing.
        mode = mode.tick(f32::NAN, false, &s);
        mode = mode.tick(-1.0, false, &s);
        assert_eq!(mode.drawn_pos(lord), Some(p(5, 5)));
        // Keys other than Confirm are ignored while walking.
        for a in [Action::Cancel, Action::CursorLeft, Action::Info] {
            let (m, effect) = step(mode.clone(), a, p(6, 5), &s);
            assert_eq!((&m, effect), (&mode, Effect::None));
        }
        mode = mode.tick(tick, false, &s);
        // At the end: the menu, the unit drawn there.
        assert!(matches!(mode, Mode::ActionMenu { .. }), "{mode:?}");
        assert_eq!(mode.drawn_pos(lord), Some(p(6, 5)));
        assert_eq!(mode.drawn_pos(UnitId(2)), None);
        assert!(!mode.cursor_free());
        assert_eq!(mode.selection().map(|s| s.unit), Some(lord));
        // Browsing: nothing is drawn elsewhere, nothing is selected.
        assert_eq!(Mode::default().drawn_pos(lord), None);
        assert_eq!(Mode::default().selection(), None);
        assert_eq!(Mode::default().tick(1.0, true, &s), Mode::default());
    }

    #[test]
    fn holding_or_pressing_confirm_skips_the_walk() {
        let s = quick();
        let sel = selected(&s, &[p(4, 5), p(5, 5), p(6, 5), p(7, 5)]);
        let walk = Mode::Moving {
            sel,
            t: 0.0,
            held: 0.0,
        };
        let menu = |m: &Mode| matches!(m, Mode::ActionMenu { .. });
        // Held, but not long enough; released; held again long enough.
        let short = HOLD_SKIP_S * 0.6;
        let mut mode = walk.clone().tick(short, true, &s);
        assert!(!menu(&mode));
        mode = mode.tick(0.0, false, &s).tick(short, true, &s);
        assert!(!menu(&mode), "the hold restarted when released");
        mode = mode.tick(short, true, &s);
        assert!(menu(&mode));
        // The hold alone skips: early in the walk, held just past the limit.
        let Mode::Moving { sel, .. } = walk.clone() else {
            unreachable!()
        };
        let nearly = Mode::Moving {
            sel,
            t: 0.0,
            held: short,
        };
        assert!(menu(&nearly.clone().tick(short, true, &s)));
        assert!(!menu(&nearly.tick(short, false, &s)));
        // A new press skips at once.
        let (mode, _) = step(walk, Action::Confirm, p(7, 5), &s);
        assert!(menu(&mode));
    }

    /// The lord walked to (5, 5), its menu open.
    fn lord_menu(s: &BattleState) -> Mode {
        let sel = selected(s, &[p(4, 5), p(5, 5)]);
        let walk = Mode::Moving {
            sel,
            t: 0.0,
            held: 0.0,
        };
        walk.tick(1.0, false, s)
    }

    #[test]
    fn the_menu_offers_a_disabled_attack_and_wait_then_waits() {
        let s = quick();
        let mode = lord_menu(&s);
        let Mode::ActionMenu { menu, entries, .. } = &mode else {
            panic!("{mode:?}");
        };
        assert_eq!(
            entries,
            &[
                MenuEntry::Attack,
                MenuEntry::Skill,
                MenuEntry::Item,
                MenuEntry::Equip,
                MenuEntry::Wait
            ]
        );
        let labels: Vec<(&str, bool)> = menu
            .items()
            .iter()
            .map(|i| (i.label.as_str(), i.enabled))
            .collect();
        // Nobody is hurt (no item to use) but the lord has two weapons and
        // its Inspire has allies near to inspire.
        let want = [
            ("Attack", false),
            ("Skill", true),
            ("Item", false),
            ("Equip", true),
            ("Wait", true),
        ];
        assert_eq!(labels, want);
        // Up and Down stay on Wait; Confirm sends the move and the wait.
        let (mode, effect) = run(
            mode,
            &[Action::CursorUp, Action::CursorDown, Action::Confirm],
            p(5, 5),
            &s,
        );
        assert_eq!(mode, Mode::default());
        assert_eq!(
            effect,
            Effect::Apply(Command::Act {
                unit: UnitId(1),
                dest: p(5, 5),
                action: UnitAction::Wait,
            })
        );
    }

    #[test]
    fn cancel_in_the_menu_goes_back_to_the_same_path() {
        let s = quick();
        let mode = lord_menu(&s);
        let sel = mode.selection().cloned().unwrap();
        assert_eq!(sel.path, [p(3, 5), p(4, 5), p(5, 5)]);
        assert_eq!(
            step(mode, Action::Cancel, p(5, 5), &s),
            (Mode::Selected(sel), Effect::None)
        );
    }

    #[test]
    fn seize_is_offered_only_where_it_is_legal() {
        let seize = |by_lord| Objective::Seize {
            pos: p(5, 5),
            by_lord,
            turn_limit: None,
        };
        let s = with_objective(seize(true));
        let Mode::ActionMenu { entries, menu, .. } = lord_menu(&s) else {
            panic!("no menu");
        };
        let all = [
            MenuEntry::Attack,
            MenuEntry::Seize,
            MenuEntry::Skill,
            MenuEntry::Item,
            MenuEntry::Equip,
            MenuEntry::Wait,
        ];
        assert_eq!(entries, all);
        assert_eq!(menu.focus(), 1, "focus on Seize");
        let (_, effect) = run(lord_menu(&s), &[Action::Confirm], p(5, 5), &s);
        assert_eq!(
            effect,
            Effect::Apply(Command::Act {
                unit: UnitId(1),
                dest: p(5, 5),
                action: UnitAction::Seize,
            })
        );
        // Elsewhere, or by a non-lord where the lord must seize: no Seize.
        let sel = selected(&s, &[p(4, 5)]);
        let plain = [
            MenuEntry::Attack,
            MenuEntry::Skill,
            MenuEntry::Item,
            MenuEntry::Equip,
            MenuEntry::Wait,
        ];
        assert_eq!(menu_entries(&sel, &s), plain);
        let mut knight = sel;
        knight.unit = UnitId(2);
        knight.path = vec![p(5, 5)];
        assert_eq!(menu_entries(&knight, &s), plain);
        let s = with_objective(seize(false));
        assert_eq!(menu_entries(&knight, &s), all);
        assert_eq!(
            all.map(MenuEntry::label),
            ["Attack", "Seize", "Skill", "Item", "Equip", "Wait"]
        );
    }

    /// The unit whose threat area `mode` shows, if any.
    fn shown(mode: &Mode) -> Option<UnitId> {
        match mode {
            Mode::Idle { threat: Some(t) } => Some(t.unit),
            _ => None,
        }
    }

    #[test]
    fn confirm_on_an_enemy_toggles_its_threat_area() {
        let s = quick();
        let (mode, effect) = step(Mode::default(), Action::Confirm, p(8, 2), &s);
        assert_eq!((shown(&mode), effect), (Some(UnitId(4)), Effect::None));
        let Mode::Idle {
            threat: Some(threat),
        } = &mode
        else {
            panic!("{mode:?}");
        };
        // Its own tile, a tile it can walk to and hit from, not a far one.
        assert!(threat.area.contains(p(8, 2)));
        assert!(threat.area.contains(p(8, 7)));
        assert!(!threat.area.contains(p(0, 7)));
        // Browsing on with it shown: the cursor may move, cycling doesn't
        // hide it.
        assert!(mode.cursor_free());
        let (mode, _) = step(mode, Action::NextUnit, p(8, 2), &s);
        assert_eq!(shown(&mode), Some(UnitId(4)));
        // Another enemy: its area instead.
        let (mode, _) = step(mode, Action::Confirm, p(7, 1), &s);
        assert_eq!(shown(&mode), Some(UnitId(6)));
        // The same enemy again hides it.
        let (mode, _) = step(mode, Action::Confirm, p(7, 1), &s);
        assert_eq!(mode, Mode::default());
        // Cancel hides it rather than leaving.
        let (mode, _) = step(Mode::default(), Action::Confirm, p(8, 2), &s);
        assert_eq!(
            step(mode, Action::Cancel, p(8, 2), &s),
            (Mode::default(), Effect::None)
        );
        let (menu, effect) = step(Mode::default(), Action::Cancel, p(8, 2), &s);
        assert!(matches!(menu, Mode::MapMenu { .. }), "{menu:?}");
        assert_eq!(effect, Effect::None);
    }

    #[test]
    fn a_player_unit_that_acted_keeps_the_threat_area_and_a_ready_one_hides_it() {
        let mut s = quick();
        wait(&mut s, 2);
        let (mode, _) = step(Mode::default(), Action::Confirm, p(8, 2), &s);
        let (mode, _) = step(mode, Action::Confirm, p(2, 4), &s);
        assert_eq!(shown(&mode), Some(UnitId(4)));
        let (mode, _) = step(mode, Action::Confirm, p(3, 5), &s);
        assert!(matches!(mode, Mode::Selected(_)), "{mode:?}");
    }

    #[test]
    fn a_move_after_an_attack_waits_for_a_tile_or_the_unit() {
        let s = vaulted(&ctx());
        let mode = Mode::after_command(&s);
        let Mode::MoveAfter { unit, tiles } = &mode else {
            panic!("{mode:?}");
        };
        assert_eq!(*unit, UnitId(3));
        assert_eq!(tiles, &s.move_after_tiles());
        assert!(!tiles.is_empty());
        assert!(mode.cursor_free());
        assert_eq!(mode.selection(), None);
        let to = tiles[0];
        // Anything but Confirm on a highlighted tile or on the unit: waits.
        for (a, at) in [
            (Action::Cancel, to),
            (Action::Confirm, p(0, 0)),
            (Action::Info, p(8, 4)),
        ] {
            let (m, effect) = step(mode.clone(), a, at, &s);
            assert_eq!((&m, effect), (&mode, Effect::None));
        }
        let moved = |at| step(mode.clone(), Action::Confirm, at, &s);
        assert_eq!(
            moved(to),
            (
                Mode::default(),
                Effect::Apply(Command::MoveAfter {
                    unit: UnitId(3),
                    to: Some(to),
                })
            )
        );
        assert_eq!(
            moved(p(8, 4)).1,
            Effect::Apply(Command::MoveAfter {
                unit: UnitId(3),
                to: None,
            })
        );
        // Without a pending move: browsing.
        assert_eq!(Mode::after_command(&quick()), Mode::default());
    }

    /// The skirmish's lord walked to (7, 2), its menu open.
    fn skirmish_menu(s: &BattleState) -> Mode {
        let (Mode::Selected(mut sel), _) = step(Mode::default(), Action::Confirm, p(6, 2), s)
        else {
            panic!("the lord isn't selected");
        };
        sel.steer(p(7, 2), s);
        let (walk, _) = step(Mode::Selected(sel), Action::Confirm, p(7, 2), s);
        walk.tick(1.0, false, s)
    }

    #[test]
    fn attack_is_enabled_when_a_weapon_reaches_and_lists_the_weapons() {
        let s = skirmish(&ctx(), 20);
        let mode = skirmish_menu(&s);
        let Mode::ActionMenu { menu, weapons, .. } = &mode else {
            panic!("{mode:?}");
        };
        assert!(menu.items()[0].enabled);
        assert_eq!(menu.focus(), 0, "focus on Attack");
        assert_eq!(weapons.len(), 2);
        // Attack: the weapon list (two swords reach).
        let (mode, effect) = step(mode, Action::Confirm, p(7, 2), &s);
        assert_eq!(effect, Effect::None);
        let Mode::WeaponMenu { weapons, .. } = &mode else {
            panic!("{mode:?}");
        };
        assert_eq!(weapons.len(), 2);
        assert_eq!(mode.drawn_pos(UnitId(1)), Some(p(7, 2)));
        assert!(!mode.cursor_free());
        // Cancel: back to the menu, on Attack.
        let (back, _) = step(mode.clone(), Action::CursorDown, p(7, 2), &s);
        let (back, _) = step(back, Action::Cancel, p(7, 2), &s);
        let Mode::ActionMenu { menu, .. } = &back else {
            panic!("{back:?}");
        };
        assert_eq!(menu.focus(), 0);
        // The steel sword: targeting the first target, the cursor on it.
        let (list, _) = step(mode, Action::CursorDown, p(7, 2), &s);
        let (mode, effect) = step(list.clone(), Action::Confirm, p(7, 2), &s);
        assert_eq!(effect, Effect::Cursor(p(7, 1)));
        let Mode::Targeting(t) = &mode else {
            panic!("{mode:?}");
        };
        assert_eq!((t.slot, t.target()), (1, UnitId(6)));
        assert_eq!(mode.selection().map(|s| s.unit), Some(UnitId(1)));
        assert_eq!(mode.drawn_pos(UnitId(1)), Some(p(7, 2)));
        // Cancel: back to the list as it was, the cursor on the lord.
        let (back, effect) = step(mode, Action::Cancel, p(7, 1), &s);
        assert_eq!(effect, Effect::Cursor(p(7, 2)));
        assert_eq!(back, list);
    }

    #[test]
    fn targeting_cycles_with_every_direction_and_confirm_attacks() {
        let s = skirmish(&ctx(), 20);
        let (list, _) = step(skirmish_menu(&s), Action::Confirm, p(7, 2), &s);
        let (mut mode, _) = step(list, Action::Confirm, p(7, 2), &s);
        let target = |m: &Mode| match m {
            Mode::Targeting(t) => t.target(),
            _ => panic!("{m:?}"),
        };
        for (a, id, at) in [
            (Action::CursorRight, 4, p(8, 2)),
            (Action::CursorDown, 6, p(7, 1)),
            (Action::NextUnit, 4, p(8, 2)),
            (Action::CursorLeft, 6, p(7, 1)),
            (Action::CursorUp, 4, p(8, 2)),
            (Action::PrevUnit, 6, p(7, 1)),
        ] {
            let (m, effect) = step(mode, a, p(0, 0), &s);
            assert_eq!(
                (target(&m), effect),
                (UnitId(id), Effect::Cursor(at)),
                "{a:?}"
            );
            mode = m;
        }
        // Other keys do nothing.
        let (same, effect) = step(mode.clone(), Action::Info, p(7, 1), &s);
        assert_eq!((&same, effect), (&mode, Effect::None));
        let (after, effect) = step(mode, Action::Confirm, p(7, 1), &s);
        assert_eq!(after, Mode::default());
        assert_eq!(
            effect,
            Effect::Apply(Command::Act {
                unit: UnitId(1),
                dest: p(7, 2),
                action: UnitAction::Attack {
                    target: UnitId(6),
                    slot: 0,
                    active: None,
                    art: None,
                },
            })
        );
    }

    #[test]
    fn attack_with_no_weapon_in_reach_stays_in_the_menu() {
        let s = skirmish(&ctx(), 20);
        let sel = Selection::new(&s, UnitId(1)).unwrap();
        let (mode, effect) = choose_attack(sel, vec![], &s);
        assert_eq!(effect, Effect::None);
        let Mode::ActionMenu { menu, entries, .. } = &mode else {
            panic!("{mode:?}");
        };
        assert_eq!(entries[menu.focus()], MenuEntry::Wait, "Attack is disabled");
    }

    #[test]
    fn one_weapon_goes_straight_to_targeting_and_back_to_the_menu() {
        let s = skirmish(&ctx(), 20);
        // The archer at (8, 4) attacks from where it stands.
        let (Mode::Selected(sel), _) = step(Mode::default(), Action::Confirm, p(8, 4), &s) else {
            panic!("the archer isn't selected");
        };
        let (menu, _) = step(Mode::Selected(sel), Action::Confirm, p(8, 4), &s);
        let (mode, effect) = step(menu, Action::Confirm, p(8, 4), &s);
        assert_eq!(effect, Effect::Cursor(p(8, 2)));
        let Mode::Targeting(t) = &mode else {
            panic!("{mode:?}");
        };
        assert_eq!(
            (t.targets.clone(), t.weapons.is_none()),
            (vec![UnitId(4)], true)
        );
        let (back, effect) = step(mode, Action::Cancel, p(8, 2), &s);
        assert_eq!(effect, Effect::Cursor(p(8, 4)));
        let Mode::ActionMenu { menu, entries, .. } = &back else {
            panic!("{back:?}");
        };
        assert_eq!(entries[menu.focus()], MenuEntry::Attack);
    }

    #[test]
    fn during_a_combat_only_cancel_counts_and_the_end_goes_back_to_browsing() {
        let c = ctx();
        let before = skirmish(&c, 20);
        let mut after = before.clone();
        let cmd = Command::Act {
            unit: UnitId(1),
            dest: p(7, 2),
            action: UnitAction::Attack {
                target: UnitId(4),
                slot: 0,
                active: None,
                art: None,
            },
        };
        let events = after.apply(&cmd).unwrap();
        let playback = Playback::new(&events, before.units(), after.fallen(), TIMINGS).unwrap();
        let total = playback.total();
        let combat = Mode::Combat(Box::new(playback));
        assert!(!combat.cursor_free());
        assert_eq!(
            (combat.selection(), combat.drawn_pos(UnitId(1))),
            (None, None)
        );
        for a in [Action::Confirm, Action::CursorLeft, Action::NextUnit] {
            let (m, effect) = step(combat.clone(), a, p(8, 2), &after);
            assert_eq!((&m, effect), (&combat, Effect::None));
        }
        // Halfway, still playing; at the end, browsing.
        let half = combat.clone().tick(total / 2.0, false, &after);
        assert!(matches!(half, Mode::Combat(_)));
        assert_eq!(half.tick(total, false, &after), Mode::default());
        // Cancel skips: the frame's tick ends it.
        let (skipped, effect) = step(combat, Action::Cancel, p(8, 2), &after);
        assert_eq!(effect, Effect::None);
        assert_eq!(skipped.tick(1.0 / 60.0, false, &after), Mode::default());
    }

    /// The unit at `at` selected, its path at its origin.
    fn pick(s: &BattleState, at: Pos) -> Selection {
        let (Mode::Selected(sel), _) = step(Mode::default(), Action::Confirm, at, s) else {
            panic!("no unit at {at:?}");
        };
        sel
    }

    /// `sel` with the cursor moved onto `at`.
    fn aim(sel: Selection, at: Pos, s: &BattleState) -> Selection {
        let mut mode = Mode::Selected(sel);
        mode.cursor_moved(at, s);
        let Mode::Selected(sel) = mode else {
            panic!("not selected");
        };
        sel
    }

    #[test]
    fn pointing_at_an_enemy_aims_the_path_at_the_cheapest_tile_that_can_hit() {
        let s = skirmish(&ctx(), 20);
        // The lord at (6, 2), the raider (unit 6) at (7, 1): not adjacent.
        let start = pick(&s, p(6, 2));
        assert!(!can_hit(&s, UnitId(1), p(6, 2), UnitId(6)));
        let sel = aim(start.clone(), p(7, 1), &s);
        assert_eq!(sel.target, Some(UnitId(6)));
        let dest = sel.dest();
        assert!(start.reach.is_stoppable(dest) && can_hit(&s, UnitId(1), dest, UnitId(6)));
        for t in start.reach.stoppable().iter() {
            if can_hit(&s, UnitId(1), t, UnitId(6)) {
                assert!(start.reach.cost(dest) <= start.reach.cost(t), "{t:?}");
            }
        }
        assert_eq!(Some(sel.path), start.reach.path_to(dest));
    }

    #[test]
    fn pointing_at_an_enemy_keeps_a_path_that_already_hits() {
        let s = skirmish(&ctx(), 20);
        let mut start = pick(&s, p(6, 2));
        start.steer(p(7, 2), &s);
        for enemy in [p(7, 1), p(8, 2)] {
            let sel = aim(start.clone(), enemy, &s);
            assert_eq!(sel.path, start.path);
            assert!(sel.target.is_some());
        }
    }

    #[test]
    fn an_archer_points_from_two_tiles_away() {
        let s = quick();
        let enemy = s.unit(UnitId(5)).unwrap().pos;
        let sel = aim(pick(&s, p(2, 4)), enemy, &s);
        assert_eq!(sel.target, Some(UnitId(5)));
        let d = sel.dest();
        assert_eq!((d.x - enemy.x).abs() + (d.y - enemy.y).abs(), 2);
    }

    #[test]
    fn confirm_on_the_aimed_enemy_walks_then_opens_the_forecast_and_cancel_backs_out() {
        let s = skirmish(&ctx(), 20);
        let sel = aim(pick(&s, p(6, 2)), p(7, 1), &s);
        let dest = sel.dest();
        let (walk, effect) = step(Mode::Selected(sel), Action::Confirm, p(7, 1), &s);
        assert_eq!(effect, Effect::None);
        assert!(matches!(walk, Mode::Moving { .. }), "{walk:?}");
        let mode = walk.tick(1.0, false, &s);
        // Two swords reach the raider: the weapon list, then the forecast.
        let Mode::WeaponMenu { weapons, .. } = &mode else {
            panic!("{mode:?}");
        };
        assert!(weapons.iter().all(|c| c.targets[0] == UnitId(6)));
        let (targeting, effect) = step(mode, Action::Confirm, p(7, 1), &s);
        assert_eq!(effect, Effect::Cursor(p(7, 1)));
        let Mode::Targeting(t) = &targeting else {
            panic!("{targeting:?}");
        };
        assert_eq!(t.target(), UnitId(6));
        // Cancel: the weapon list, the action menu at the tile, the path.
        let (list, _) = step(targeting, Action::Cancel, p(7, 1), &s);
        assert!(matches!(list, Mode::WeaponMenu { .. }));
        let (menu, _) = step(list, Action::Cancel, dest, &s);
        let Mode::ActionMenu { sel, .. } = &menu else {
            panic!("{menu:?}");
        };
        assert_eq!(sel.dest(), dest);
        let (back, _) = step(menu, Action::Cancel, dest, &s);
        let Mode::Selected(sel) = back else {
            panic!("{back:?}");
        };
        assert_eq!((sel.target, sel.dest()), (None, dest));
    }

    #[test]
    fn a_single_weapon_goes_straight_to_the_forecast_without_walking() {
        let s = skirmish(&ctx(), 20);
        // The archer already hits the brigand from where it stands.
        let sel = aim(pick(&s, p(8, 4)), p(8, 2), &s);
        assert_eq!((sel.path.len(), sel.target), (1, Some(UnitId(4))));
        let (mode, effect) = step(Mode::Selected(sel), Action::Confirm, p(8, 2), &s);
        assert_eq!(effect, Effect::Cursor(p(8, 2)));
        assert!(matches!(mode, Mode::Targeting(_)), "{mode:?}");
    }

    #[test]
    fn an_enemy_no_tile_can_hit_leaves_the_path_and_confirm_does_nothing() {
        let s = quick();
        // The lord at (3, 5) can't reach the raider at (7, 1) this turn.
        let start = pick(&s, p(3, 5));
        let far = s.unit(UnitId(6)).unwrap().pos;
        let sel = aim(start.clone(), far, &s);
        assert_eq!((sel.target, &sel.path), (None, &start.path));
        let (mode, effect) = step(Mode::Selected(sel.clone()), Action::Confirm, far, &s);
        assert_eq!((mode, effect), (Mode::Selected(sel), Effect::None));
    }

    #[test]
    fn an_aim_no_weapon_reaches_from_the_path_end_falls_back_to_the_action_menu() {
        let s = skirmish(&ctx(), 20);
        // Aimed at the raider but still standing at (6, 2), out of reach.
        let mut sel = pick(&s, p(6, 2));
        sel.target = Some(UnitId(6));
        let (mode, effect) = open_attack(sel, &s);
        assert_eq!(effect, Effect::None);
        assert!(matches!(mode, Mode::ActionMenu { .. }), "{mode:?}");
    }
}
