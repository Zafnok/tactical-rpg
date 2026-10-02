//! The map menu (ticket 0405), opened by Cancel with nothing to cancel or
//! Confirm on an empty tile (`docs/design/controls.md`): `Units` (jump the
//! cursor to one), `Objective` (what to do and the turn), `Options` and
//! `Suspend` (placeholders until 0805 and 0802), `Restart Battle` (0801,
//! with a confirm) and `End Turn`; and the end-turn prompt
//! (`docs/design/turn-structure.md`).

use trpg_core::{BattleState, Faction, Objective, Phase, Turn, UnitId};

use super::layout::MAP_VIEW;
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::widgets::menu::{Menu, MenuItem};

/// One entry of the map menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapEntry {
    /// The player's units: choosing one jumps the cursor to it.
    Units,
    /// The objective and the turn.
    Objective,
    /// Options (ticket 0805): disabled for now.
    Options,
    /// Suspend the battle (ticket 0802): disabled for now.
    Suspend,
    /// Start the battle again from its first turn, with every rewind
    /// charge back (`death-and-difficulty.md`), after a confirm.
    Restart,
    /// End the player phase.
    EndTurn,
}

impl MapEntry {
    /// Every entry, in menu order.
    pub const ALL: [MapEntry; 6] = [
        MapEntry::Units,
        MapEntry::Objective,
        MapEntry::Options,
        MapEntry::Suspend,
        MapEntry::Restart,
        MapEntry::EndTurn,
    ];

    /// The text shown.
    pub const fn label(self) -> &'static str {
        match self {
            MapEntry::Units => "Units",
            MapEntry::Objective => "Objective",
            MapEntry::Options => "Options",
            MapEntry::Suspend => "Suspend",
            MapEntry::Restart => "Restart Battle",
            MapEntry::EndTurn => "End Turn",
        }
    }

    /// Whether it can be chosen in `state`: `Options` and `Suspend` never
    /// (placeholders), `End Turn` only in the player phase of a battle still
    /// going, the others (`Restart Battle` too) always.
    pub fn enabled(self, state: &BattleState) -> bool {
        match self {
            MapEntry::Units | MapEntry::Objective | MapEntry::Restart => true,
            MapEntry::Options | MapEntry::Suspend => false,
            MapEntry::EndTurn => state.phase() == Phase::Player && state.outcome().is_none(),
        }
    }
}

/// The map menu for `state`, focused on `focus` (if enabled), and what
/// each item is.
pub fn map_menu(state: &BattleState, focus: MapEntry) -> (Menu, Vec<MapEntry>) {
    let entries = MapEntry::ALL.to_vec();
    let items = entries
        .iter()
        .map(|&e| {
            if e.enabled(state) {
                MenuItem::new(e.label())
            } else {
                MenuItem::disabled(e.label())
            }
        })
        .collect();
    let index = entries.iter().position(|&e| e == focus).unwrap_or(0);
    (Menu::new(items).focused(index), entries)
}

/// How many player units can still act this phase.
pub fn ready_players(state: &BattleState) -> usize {
    state
        .units()
        .iter()
        .filter(|u| u.faction == Faction::Player && Phase::of(u.faction) == state.phase())
        .filter(|u| !u.acted)
        .count()
}

/// The `Units` list: every player unit, as `name  HP hp/max  ready` (or
/// `acted`), in the battle's unit order, and their ids.
pub fn unit_list(state: &BattleState) -> (Menu, Vec<UnitId>) {
    let players: Vec<_> = state
        .units()
        .iter()
        .filter(|u| u.faction == Faction::Player)
        .collect();
    let name_w = players
        .iter()
        .map(|u| u.name.chars().count())
        .max()
        .unwrap_or(0);
    let items = players
        .iter()
        .map(|u| {
            let status = if u.acted { "acted" } else { "ready" };
            let hp = format!("{}/{}", u.hp, u.stats.hp);
            MenuItem::new(format!("{:name_w$}  HP {hp:>5}  {status}", u.name))
        })
        .collect();
    (Menu::new(items), players.iter().map(|u| u.id).collect())
}

/// What the objective asks, e.g. `Rout the enemy` or `Seize the Fort`.
/// *Claude's starting wording* until maps carry their own text.
pub fn objective_text(state: &BattleState) -> String {
    match state.objective() {
        Objective::Rout { .. } => "Rout the enemy".to_owned(),
        Objective::DefeatUnit { unit, .. } => {
            let name = state
                .unit(unit)
                .or_else(|| state.fallen().iter().find(|u| u.id == unit))
                .map_or("the boss", |u| u.name.as_str());
            format!("Defeat {name}")
        }
        Objective::Seize { pos, .. } => {
            let place = state
                .map()
                .tiles
                .get(pos)
                .and_then(|&id| state.terrain().get(id))
                .map_or("objective", |t| t.name.as_str());
            format!("Seize the {place}")
        }
        Objective::Survive { turns } => format!("Survive {turns} turns"),
    }
}

/// The turn, with the map's limit if it has one: `Turn 3` or `Turn 3/8`
/// (for Survive, the turn that wins).
pub fn turn_text(state: &BattleState) -> String {
    match shown_limit(state) {
        Some(limit) => format!("Turn {}/{limit}", state.turn()),
        None => format!("Turn {}", state.turn()),
    }
}

/// The turn shown after the current one as its limit: the objective's
/// turn limit, or for Survive the turn that wins.
pub fn shown_limit(state: &BattleState) -> Option<Turn> {
    match state.objective() {
        Objective::Survive { turns } => Some(turns),
        other => other.turn_limit(),
    }
}

/// The restart question.
pub const RESTART_QUESTION: &str = "Restart the battle from turn 1?";

/// The end-turn question for `ready` units still ready.
pub fn end_turn_question(ready: usize) -> String {
    let units = if ready == 1 { "unit" } else { "units" };
    format!("End turn with {ready} {units} ready?")
}

/// Draws a box of `lines` (text and colour) centred on the map view, one
/// blank column and row inside a double border; `title` over the top
/// border.
pub fn draw_dialog(
    buf: &mut GlyphBuffer,
    palette: &Palette,
    title: &str,
    lines: &[(String, UiColor)],
) {
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    let widest = lines
        .iter()
        .map(|(l, _)| l.chars().count())
        .chain(std::iter::once(title.chars().count() + 2))
        .max()
        .unwrap_or(0);
    let w = i32::try_from(widest).unwrap_or(0) + 4;
    let h = i32::try_from(lines.len()).unwrap_or(0) + 4;
    let rect = Rect::new(
        MAP_VIEW.x + (MAP_VIEW.w - w) / 2,
        MAP_VIEW.y + (MAP_VIEW.h - h) / 2,
        w,
        h,
    );
    buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorder), bg);
    if !title.is_empty() {
        buf.print(
            rect.x + 2,
            rect.y,
            &format!(" {title} "),
            c(UiColor::TextHighlight),
            bg,
        );
    }
    for (y, (line, color)) in (rect.y + 2..).zip(lines) {
        buf.print(rect.x + 2, y, line, c(*color), bg);
    }
}

/// Draws `menu` centred on the map view.
pub fn draw_centred_menu(buf: &mut GlyphBuffer, palette: &Palette, menu: &Menu) {
    let (w, h) = menu.size();
    let x = MAP_VIEW.x + (MAP_VIEW.w - w) / 2;
    let y = MAP_VIEW.y + (MAP_VIEW.h - h) / 2;
    menu.draw(palette, buf, x, y);
}

#[cfg(test)]
mod tests {
    use trpg_core::Pos;

    use super::*;
    use crate::screen::tests::ctx;
    use crate::screens::battle::quick_battle;
    use crate::screens::battle::testing::{battle_with, wait};

    fn labels(menu: &Menu) -> Vec<(&str, bool)> {
        menu.items()
            .iter()
            .map(|i| (i.label.as_str(), i.enabled))
            .collect()
    }

    #[test]
    fn options_and_suspend_are_disabled_and_end_turn_needs_the_player_phase() {
        let mut s = quick_battle(&ctx().content).unwrap();
        let (menu, entries) = map_menu(&s, MapEntry::Units);
        assert_eq!(entries, MapEntry::ALL);
        assert_eq!(
            labels(&menu),
            [
                ("Units", true),
                ("Objective", true),
                ("Options", false),
                ("Suspend", false),
                ("Restart Battle", true),
                ("End Turn", true),
            ]
        );
        assert_eq!(menu.focus(), 0);
        assert_eq!(map_menu(&s, MapEntry::Restart).0.focus(), 4);
        assert_eq!(map_menu(&s, MapEntry::EndTurn).0.focus(), 5);
        // A disabled entry can't take the focus.
        assert_eq!(map_menu(&s, MapEntry::Suspend).0.focus(), 0);
        // In the enemy phase, End Turn is disabled.
        s.apply(&trpg_core::Command::EndPhase).unwrap();
        assert_eq!(s.phase(), Phase::Enemy);
        assert!(!MapEntry::EndTurn.enabled(&s));
        // Nobody acted, but it isn't the player's phase: none ready.
        assert_eq!(ready_players(&s), 0);
        assert!(MapEntry::Units.enabled(&s));
        assert!(MapEntry::Restart.enabled(&s));
    }

    #[test]
    fn end_turn_is_disabled_once_the_battle_is_over() {
        let c = ctx();
        let q = quick_battle(&c.content).unwrap();
        let players: Vec<_> = q
            .units()
            .iter()
            .filter(|u| u.faction == Faction::Player)
            .cloned()
            .collect();
        let rout = Objective::Rout { turn_limit: None };
        // No enemies: won from the start.
        let s = battle_with(&c, q.map().clone(), players, rout);
        assert!(s.outcome().is_some());
        assert!(!MapEntry::EndTurn.enabled(&s));
    }

    #[test]
    fn the_unit_list_shows_hp_and_who_is_ready() {
        let mut s = quick_battle(&ctx().content).unwrap();
        wait(&mut s, 2);
        assert_eq!(ready_players(&s), 3);
        let (menu, ids) = unit_list(&s);
        assert_eq!(ids, [UnitId(1), UnitId(2), UnitId(3), UnitId(8)]);
        let rows: Vec<&str> = menu.items().iter().map(|i| i.label.as_str()).collect();
        assert_eq!(
            rows,
            [
                "Test Lord    HP 19/19  ready",
                "Test Knight  HP 20/20  ready",
                "Test Archer  HP 17/17  acted",
                "Test Mage    HP 16/16  ready",
            ]
        );
    }

    #[test]
    fn objective_and_turn_texts() {
        let c = ctx();
        let q = quick_battle(&c.content).unwrap();
        assert_eq!(objective_text(&q), "Rout the enemy");
        assert_eq!(turn_text(&q), "Turn 1");
        let with = |o| battle_with(&c, q.map().clone(), q.units().to_vec(), o);
        let s = with(Objective::Rout {
            turn_limit: Some(8),
        });
        assert_eq!(turn_text(&s), "Turn 1/8");
        let s = with(Objective::DefeatUnit {
            unit: UnitId(6),
            turn_limit: None,
        });
        assert_eq!(objective_text(&s), "Defeat Raider");
        let s = with(Objective::DefeatUnit {
            unit: UnitId(99),
            turn_limit: None,
        });
        assert_eq!(objective_text(&s), "Defeat the boss");
        // A boss that has fallen keeps its name.
        let mut s = battle_with(
            &c,
            q.map().clone(),
            q.units().to_vec(),
            Objective::DefeatUnit {
                unit: UnitId(4),
                turn_limit: None,
            },
        );
        let mut units = s.units().to_vec();
        units[0].pos = Pos::new(7, 2);
        units[3].hp = 1;
        s = battle_with(&c, q.map().clone(), units, s.objective());
        s.apply(&trpg_core::Command::Act {
            unit: UnitId(1),
            dest: Pos::new(7, 2),
            action: trpg_core::UnitAction::Attack {
                target: UnitId(4),
                slot: 0,
                active: None,
                art: None,
            },
        })
        .unwrap();
        assert!(s.unit(UnitId(4)).is_none());
        assert_eq!(objective_text(&s), "Defeat Brigand");
        let s = with(Objective::Seize {
            pos: Pos::new(5, 5),
            by_lord: true,
            turn_limit: Some(3),
        });
        assert_eq!(
            (objective_text(&s), turn_text(&s)),
            ("Seize the Fort".into(), "Turn 1/3".into())
        );
        let s = with(Objective::Seize {
            pos: Pos::new(99, 99),
            by_lord: true,
            turn_limit: None,
        });
        assert_eq!(objective_text(&s), "Seize the objective");
        let s = with(Objective::Survive { turns: 5 });
        assert_eq!(
            (objective_text(&s), turn_text(&s)),
            ("Survive 5 turns".into(), "Turn 1/5".into())
        );
    }

    #[test]
    fn the_question_counts_units() {
        assert_eq!(end_turn_question(1), "End turn with 1 unit ready?");
        assert_eq!(end_turn_question(3), "End turn with 3 units ready?");
    }
}
