//! What the player is doing on the battle screen (ticket 0403), the Fire
//! Emblem move loop: browse ([`Mode::Idle`]), select a unit and steer its
//! path ([`Mode::Selected`]), watch it walk ([`Mode::Moving`]), pick an
//! action ([`Mode::ActionMenu`]). The move is only sent, as one
//! [`Command::Act`], when an action is chosen, so cancelling is free: until
//! then the unit's new tile is a drawing override ([`Mode::drawn_pos`]) and
//! the [`BattleState`] is untouched.
//!
//! Transitions are pure ([`step`], [`Mode::tick`]); the screen owns the
//! cursor and applies the [`Effect`]s.

use trpg_core::{
    BattleState, Command, Faction, Phase, Pos, Reach, TileSet, UnitAction, UnitId, attack_tiles,
    path_cost, reachable, threat_area,
};

use super::path::steer;
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
    /// Attack (targeting comes with ticket 0404; shown disabled until then).
    Attack,
    /// Seize the objective tile, when legal there.
    Seize,
    /// End the unit's action.
    Wait,
}

impl MenuEntry {
    /// The text shown.
    pub const fn label(self) -> &'static str {
        match self {
            MenuEntry::Attack => "Attack",
            MenuEntry::Seize => "Seize",
            MenuEntry::Wait => "Wait",
        }
    }

    /// The menu item: `Attack` is disabled until ticket 0404.
    fn item(self) -> MenuItem {
        match self {
            MenuEntry::Attack => MenuItem::disabled(self.label()),
            _ => MenuItem::new(self.label()),
        }
    }
}

/// The action menu for `sel`'s unit at its path's end: `Attack` (disabled),
/// `Seize` if legal there, `Wait`.
pub fn menu_entries(sel: &Selection, state: &BattleState) -> Vec<MenuEntry> {
    let mut entries = vec![MenuEntry::Attack];
    if state.can_seize(sel.unit, sel.dest()) {
        entries.push(MenuEntry::Seize);
    }
    entries.push(MenuEntry::Wait);
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
    },
    /// A unit waits to move after its attack ([`BattleState::pending_move`]):
    /// Confirm on one of `tiles` moves it there, Confirm on the unit stays.
    MoveAfter {
        /// The unit.
        unit: UnitId,
        /// Where it may go ([`BattleState::move_after_tiles`]).
        tiles: Vec<Pos>,
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
    /// Put the cursor on this tile.
    Cursor(Pos),
    /// Leave the battle screen (until the map menu, 0405).
    Leave,
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
            sel.steer(to, state);
        }
    }

    /// The selected unit, while one is selected, walking or choosing.
    pub fn selection(&self) -> Option<&Selection> {
        match self {
            Mode::Selected(sel) | Mode::Moving { sel, .. } | Mode::ActionMenu { sel, .. } => {
                Some(sel)
            }
            Mode::Idle { .. } | Mode::MoveAfter { .. } => None,
        }
    }

    /// Where unit `id` is drawn, if not on its own tile: along its path
    /// while it walks, at the path's end while the action menu is open.
    pub fn drawn_pos(&self, id: UnitId) -> Option<Pos> {
        match self {
            Mode::Moving { sel, t, .. } if sel.unit == id => Some(walk_pos(sel, *t)),
            Mode::ActionMenu { sel, .. } if sel.unit == id => Some(sel.dest()),
            _ => None,
        }
    }

    /// Advances a walk by `dt` seconds (`confirm_held`: Confirm is down this
    /// frame). The walk ends, opening the action menu, once the unit
    /// reaches the path's end or Confirm has been held [`HOLD_SKIP_S`].
    /// Other modes are unchanged.
    #[must_use]
    pub fn tick(self, dt: f32, confirm_held: bool, state: &BattleState) -> Mode {
        let Mode::Moving { sel, t, held } = self else {
            return self;
        };
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let (t, held) = (t + dt, if confirm_held { held + dt } else { 0.0 });
        if held >= HOLD_SKIP_S || walk_done(&sel, t) {
            open_menu(sel, state)
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

/// The action menu for `sel`.
fn open_menu(sel: Selection, state: &BattleState) -> Mode {
    let entries = menu_entries(&sel, state);
    let menu = Menu::new(entries.iter().map(|e| e.item()).collect());
    Mode::ActionMenu { sel, menu, entries }
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
        Mode::Moving { sel, .. } if action == Action::Confirm => {
            (open_menu(sel, state), Effect::None)
        }
        moving @ Mode::Moving { .. } => (moving, Effect::None),
        Mode::ActionMenu {
            sel,
            mut menu,
            entries,
        } => match menu.handle(action) {
            Some(MenuEvent::Chosen(i)) => {
                let action = match entries.get(i) {
                    Some(MenuEntry::Wait) => UnitAction::Wait,
                    Some(MenuEntry::Seize) => UnitAction::Seize,
                    _ => return (Mode::ActionMenu { sel, menu, entries }, Effect::None),
                };
                let cmd = Command::Act {
                    unit: sel.unit,
                    dest: sel.dest(),
                    action,
                };
                (Mode::default(), Effect::Apply(cmd))
            }
            Some(MenuEvent::Cancelled) => (Mode::Selected(sel), Effect::None),
            None => (Mode::ActionMenu { sel, menu, entries }, Effect::None),
        },
        Mode::MoveAfter { unit, tiles } => {
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
    }
}

/// [`step`] while browsing: Confirm selects a ready player unit or toggles
/// another faction's unit's threat area; Cancel hides the threat area, or
/// leaves if none is shown.
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
                return idle(threat);
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
        Action::Cancel => (Mode::default(), Effect::Leave),
        _ => idle(threat),
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::Objective;

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::quick_battle;
    use crate::screens::battle::testing::{battle_with, vaulted, wait};

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// The Quick Battle: lord 1 at (3, 5), knight 2 at (4, 6), archer 3 at
    /// (2, 4); brigands 4 at (8, 2) and 5 at (12, 3), raider 6 at (7, 1).
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
        // An empty tile.
        assert_eq!(
            step(Mode::default(), Action::Confirm, p(6, 5), &s),
            (Mode::default(), Effect::None)
        );
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
        assert_eq!(entries, &[MenuEntry::Attack, MenuEntry::Wait]);
        let labels: Vec<(&str, bool)> = menu
            .items()
            .iter()
            .map(|i| (i.label.as_str(), i.enabled))
            .collect();
        assert_eq!(labels, [("Attack", false), ("Wait", true)]);
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
        let all = [MenuEntry::Attack, MenuEntry::Seize, MenuEntry::Wait];
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
        assert_eq!(menu_entries(&sel, &s), [MenuEntry::Attack, MenuEntry::Wait]);
        let mut knight = sel;
        knight.unit = UnitId(2);
        knight.path = vec![p(5, 5)];
        assert_eq!(
            menu_entries(&knight, &s),
            [MenuEntry::Attack, MenuEntry::Wait]
        );
        let s = with_objective(seize(false));
        assert_eq!(menu_entries(&knight, &s), all);
        assert_eq!(all.map(MenuEntry::label), ["Attack", "Seize", "Wait"]);
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
        // Browsing on with it shown: the cursor may move, Info does nothing.
        assert!(mode.cursor_free());
        let (mode, _) = step(mode, Action::Info, p(8, 2), &s);
        assert_eq!(shown(&mode), Some(UnitId(4)));
        // Another enemy: its area instead.
        let (mode, _) = step(mode, Action::Confirm, p(7, 1), &s);
        assert_eq!(shown(&mode), Some(UnitId(6)));
        // An empty tile keeps it.
        let (mode, _) = step(mode, Action::Confirm, p(6, 5), &s);
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
        assert_eq!(
            step(Mode::default(), Action::Cancel, p(8, 2), &s),
            (Mode::default(), Effect::Leave)
        );
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
}
