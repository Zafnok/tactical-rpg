//! The rewind screen (ticket 0307, `docs/design/death-and-difficulty.md`):
//! an overlay in the side panel listing the battle's past actions, newest
//! first. The map behind shows the battle as it was just before the
//! highlighted action; confirming (after a "Use 1 of N charges?" prompt)
//! goes back there for one charge.
//!
//! It lives inside the battle screen rather than on the screen stack: it
//! draws the battle screen's map with another state and hands the chosen
//! point back, which a pushed screen can't.

use trpg_core::{
    BattleHistory, BattleState, Command, Event, Phase, Replayed, Side, UnitAction, UnitId,
};

use super::layout::SIDE_PANEL;
use super::panel::{TEXT_W, TEXT_X};
use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;

/// Row of the title.
pub const TITLE_ROW: i32 = SIDE_PANEL.y + 1;

/// Row of the charges line.
pub const CHARGES_ROW: i32 = TITLE_ROW + 1;

/// First row of the list, after a blank row.
pub const LIST_ROW: i32 = TITLE_ROW + 3;

/// Row of the confirm prompt (the list stops above it).
pub const PROMPT_ROW: i32 = SIDE_PANEL.y + SIDE_PANEL.h - 3;

/// Indent of an entry's wrapped lines.
const WRAP_INDENT: &str = "  ";

/// One past action.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// What happened, e.g. `Turn 2 · Ana attacked Brigand (hit, 7 dmg)`.
    pub line: String,
    /// The point a rewind to it goes back to (the command's index).
    pub point: usize,
    /// The battle just before it.
    pub before: BattleState,
}

/// What the battle screen should do after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewindEffect {
    /// Nothing more.
    None,
    /// Close the screen, battle unchanged.
    Close,
    /// Rewind to this point (the player confirmed).
    Rewind(usize),
}

/// The open rewind screen.
#[derive(Debug, Clone, PartialEq)]
pub struct RewindScreen {
    entries: Vec<Entry>,
    focus: usize,
    confirming: bool,
    charges_left: u8,
    charges: u8,
}

impl RewindScreen {
    /// The screen for `history`, focused on the newest action.
    pub fn new(history: &BattleHistory, charges: u8) -> Self {
        let mut entries: Vec<Entry> = history
            .replay()
            .into_iter()
            .enumerate()
            .map(|(point, r)| Entry {
                line: describe(&r),
                point,
                before: r.before,
            })
            .collect();
        entries.reverse();
        Self {
            entries,
            focus: 0,
            confirming: false,
            charges_left: history.charges_left(),
            charges,
        }
    }

    /// The past actions, newest first.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The highlighted entry, if there are any.
    pub fn focused(&self) -> Option<&Entry> {
        self.entries.get(self.focus)
    }

    /// Whether the "Use 1 of N charges?" prompt is up.
    pub fn is_confirming(&self) -> bool {
        self.confirming
    }

    /// Whether Confirm can rewind: an action to go back to and a charge.
    pub fn can_rewind(&self) -> bool {
        self.charges_left > 0 && !self.entries.is_empty()
    }

    /// Handles one action: up/down choose (not wrapping), Confirm asks and
    /// then rewinds, Cancel backs out of the prompt or closes, Rewind
    /// closes.
    pub fn step(&mut self, action: Action) -> RewindEffect {
        match action {
            Action::Cancel if self.confirming => self.confirming = false,
            Action::Cancel | Action::Rewind => return RewindEffect::Close,
            Action::Confirm if self.confirming => {
                if let Some(e) = self.focused() {
                    return RewindEffect::Rewind(e.point);
                }
            }
            Action::Confirm => self.confirming = self.can_rewind(),
            Action::CursorDown if !self.confirming => {
                self.focus = (self.focus + 1).min(self.entries.len().saturating_sub(1));
            }
            Action::CursorUp if !self.confirming => self.focus = self.focus.saturating_sub(1),
            _ => {}
        }
        RewindEffect::None
    }

    /// Draws the panel: title, charges, the list (scrolled to keep the
    /// highlighted entry in view) and the prompt.
    pub fn draw(&self, palette: &Palette, buf: &mut GlyphBuffer) {
        let c = |u| palette.get(u);
        let bg = c(UiColor::PanelBg);
        buf.fill_rect(SIDE_PANEL, Cell::new(' ', c(UiColor::Text), bg));
        buf.draw_box(SIDE_PANEL, BoxStyle::Double, c(UiColor::PanelBorder), bg);
        let print = |buf: &mut GlyphBuffer, y, text: &str, fg, bg| {
            buf.print(TEXT_X, y, text, fg, bg);
        };
        print(buf, TITLE_ROW, "Rewind", c(UiColor::TextHighlight), bg);
        let charges = if self.charges_left == 0 {
            ("No charges left".to_owned(), c(UiColor::HpLow))
        } else {
            let s = if self.charges == 1 { "" } else { "s" };
            let text = format!("{} of {} charge{s} left", self.charges_left, self.charges);
            (text, c(UiColor::Text))
        };
        print(buf, CHARGES_ROW, &charges.0, charges.1, bg);
        if self.entries.is_empty() {
            print(
                buf,
                LIST_ROW,
                "Nothing to rewind yet",
                c(UiColor::TextDim),
                bg,
            );
            return;
        }
        let blocks: Vec<Vec<String>> = self.entries.iter().map(|e| wrap(&e.line)).collect();
        let first = first_shown(&blocks, self.focus, PROMPT_ROW - 1 - LIST_ROW);
        let mut y = LIST_ROW;
        for (i, block) in blocks.iter().enumerate().skip(first) {
            let rows = i32::try_from(block.len()).unwrap_or(i32::MAX);
            if y + rows > PROMPT_ROW - 1 {
                break;
            }
            let (fg, row_bg) = if i == self.focus {
                (bg, c(UiColor::PanelBorderFocus))
            } else {
                (c(UiColor::Text), bg)
            };
            for text in block {
                buf.fill_rect(
                    Rect::new(TEXT_X - 1, y, SIDE_PANEL.w - 2, 1),
                    Cell::new(' ', fg, row_bg),
                );
                print(buf, y, text, fg, row_bg);
                y += 1;
            }
        }
        if self.confirming {
            let s = if self.charges_left == 1 { "" } else { "s" };
            let ask = format!("Use 1 of {} charge{s}?", self.charges_left);
            print(buf, PROMPT_ROW, &ask, c(UiColor::TextHighlight), bg);
        }
    }
}

/// `line` word-wrapped to the panel's text width, wrapped lines indented.
fn wrap(line: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in line.split(' ') {
        let len = current.chars().count();
        if len > 0 && len + 1 + word.chars().count() > TEXT_W {
            lines.push(std::mem::replace(
                &mut current,
                format!("{WRAP_INDENT}{word}"),
            ));
        } else {
            if len > 0 {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    lines.push(current);
    lines
}

/// The first block to show so that blocks `first..=focus` fit in `rows`.
/// (`focus` itself if even it alone doesn't fit).
fn first_shown(blocks: &[Vec<String>], focus: usize, rows: i32) -> usize {
    let height = |b: &Vec<String>| i32::try_from(b.len()).unwrap_or(i32::MAX);
    (0..focus)
        .find(|&first| blocks[first..=focus].iter().map(height).sum::<i32>() <= rows)
        .unwrap_or(focus)
}

/// A unit's name in `state`, or `?` (never shown: a command only names
/// units on the map just before it).
fn name(state: &BattleState, id: UnitId) -> &str {
    state.unit(id).map_or("?", |u| u.name.as_str())
}

/// How a combat went for the attacker: `hit, 7 dmg`, `2 hits, 14 dmg` or
/// `missed`, plus `defeated` if the defender fell.
fn combat_summary(events: &[Event]) -> Option<String> {
    let (outcome, defender_hp) = events.iter().find_map(|e| match e {
        Event::CombatResolved { outcome, .. } => Some((outcome, outcome.defender_hp)),
        _ => None,
    })?;
    let hits: Vec<_> = outcome
        .strikes
        .iter()
        .filter(|s| s.by == Side::Attacker && s.hit && !s.healed)
        .collect();
    let dmg: i32 = hits.iter().map(|s| s.damage).sum();
    let mut text = match hits.len() {
        0 => "missed".to_owned(),
        1 => format!("hit, {dmg} dmg"),
        n => format!("{n} hits, {dmg} dmg"),
    };
    if defender_hp == 0 {
        text.push_str(", defeated");
    }
    Some(text)
}

/// A readable line for a past command, e.g. `Turn 2 · Ana attacked Brigand
/// (hit, 7 dmg)`.
pub fn describe(r: &Replayed) -> String {
    let s = &r.before;
    let what = match &r.command {
        Command::Act { unit, action, .. } => {
            let who = name(s, *unit);
            let combat = combat_summary(&r.events).map(|c| format!(" ({c})"));
            let combat = combat.as_deref().unwrap_or("");
            match action {
                UnitAction::Wait => format!("{who} waited"),
                UnitAction::Attack { target, .. } => {
                    format!("{who} attacked {}{combat}", name(s, *target))
                }
                UnitAction::UseItem { pack_index, target } => {
                    let item = s
                        .pack()
                        .items
                        .get(*pack_index)
                        .and_then(|id| s.items().get(id))
                        .map_or("an item", |d| d.name());
                    if target == unit {
                        format!("{who} used {item}")
                    } else {
                        format!("{who} used {item} on {}", name(s, *target))
                    }
                }
                UnitAction::Seize => format!("{who} seized the objective"),
                UnitAction::Shop { .. } => format!("{who} visited a shop"),
                UnitAction::Open => format!("{who} opened a chest"),
                UnitAction::Cast { spell, target, .. } => {
                    let spell = s.spells().get(spell).map_or("a spell", |d| &d.name);
                    match target {
                        trpg_core::CastTarget::Unit(t) => {
                            format!("{who} cast {spell} on {}{combat}", name(s, *t))
                        }
                        trpg_core::CastTarget::Tile(_) => format!("{who} cast {spell}"),
                    }
                }
                UnitAction::UseSkill { skill, .. } => {
                    let skill = s.skills().get(skill).map_or("a skill", |d| &d.name);
                    format!("{who} used {skill}")
                }
                UnitAction::Talk { target } => format!("{who} talked to {}", name(s, *target)),
            }
        }
        Command::Equip { unit, .. } => format!("{} changed weapons", name(s, *unit)),
        Command::MoveAfter { unit, to } => {
            let who = name(s, *unit);
            if to.is_some() {
                format!("{who} moved after attacking")
            } else {
                format!("{who} stayed after attacking")
            }
        }
        Command::EndPhase => format!("{} phase ended", phase_name(s.phase())),
    };
    format!("Turn {} · {what}", s.turn())
}

/// The name players see for `phase`.
const fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Player => "Player",
        Phase::Enemy => "Enemy",
        // Ally and neutral units (FE calls it the ally phase).
        Phase::Other => "Ally",
    }
}

#[cfg(test)]
mod tests {
    use trpg_core::Pos;

    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::screen::tests::ctx;
    use crate::screens::battle::testing::skirmish;

    #[test]
    fn wrap_keeps_lines_within_the_panel_and_indents_the_rest() {
        let lines = wrap("Turn 12 · Test Lord attacked Brigand (2 hits, 14 dmg, defeated)");
        assert_eq!(
            lines,
            [
                "Turn 12 · Test Lord",
                "  attacked Brigand (2",
                "  hits, 14 dmg, defeated)"
            ]
        );
        assert!(lines.iter().all(|l| l.chars().count() <= TEXT_W));
        assert_eq!(wrap("short"), ["short"]);
        // Exactly the panel's width fits on one line.
        let full = format!("{} {}", "a".repeat(12), "b".repeat(TEXT_W - 13));
        assert_eq!(wrap(&full), std::slice::from_ref(&full));
        // A word longer than the panel isn't preceded by an empty line.
        let long = "c".repeat(TEXT_W + 4);
        assert_eq!(wrap(&long), std::slice::from_ref(&long));
    }

    #[test]
    fn first_shown_scrolls_just_enough_to_show_the_focus() {
        let b = |n| vec![String::new(); n];
        let blocks = [b(2), b(2), b(3), b(1)];
        assert_eq!(first_shown(&blocks, 0, 4), 0);
        assert_eq!(first_shown(&blocks, 1, 4), 0);
        assert_eq!(first_shown(&blocks, 2, 4), 2);
        assert_eq!(first_shown(&blocks, 2, 5), 1);
        assert_eq!(first_shown(&blocks, 3, 6), 1);
        assert_eq!(first_shown(&blocks, 3, 4), 2);
        // A block taller than the rows: shown from itself.
        assert_eq!(first_shown(&[b(5)], 0, 4), 0);
        assert_eq!(first_shown(&blocks, 2, 2), 2);
    }

    #[test]
    fn panel_rows() {
        assert_eq!(
            (TITLE_ROW, CHARGES_ROW, LIST_ROW, PROMPT_ROW),
            (1, 2, 4, 27)
        );
    }

    fn screen(lines: usize, focus: usize) -> RewindScreen {
        let before = skirmish(&ctx(), 20);
        let entries = (0..lines)
            .map(|i| Entry {
                line: format!("e{i}"),
                point: lines - 1 - i,
                before: before.clone(),
            })
            .collect();
        RewindScreen {
            entries,
            focus,
            confirming: false,
            charges_left: 3,
            charges: 3,
        }
    }

    /// Row `y` of the panel drawn for `r`, inside its border, trimmed.
    fn drawn(r: &RewindScreen) -> impl Fn(i32) -> String + use<> {
        let c = ctx();
        let black = c.palette.get(UiColor::Black);
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, Cell::new(' ', black, black));
        r.draw(&c.palette, &mut buf);
        move |y| {
            (TEXT_X..SIDE_PANEL.x + SIDE_PANEL.w - 1)
                .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
                .collect::<String>()
                .trim()
                .to_owned()
        }
    }

    #[test]
    fn a_long_list_fills_the_rows_above_the_prompt_and_scrolls_to_the_focus() {
        // 22 rows from LIST_ROW to the blank row above the prompt.
        let row = drawn(&screen(30, 0));
        assert_eq!(row(LIST_ROW), "e0");
        assert_eq!(row(PROMPT_ROW - 2), "e21");
        assert_eq!(row(PROMPT_ROW - 1), "");
        assert_eq!(row(PROMPT_ROW), "");
        let row = drawn(&screen(30, 29));
        assert_eq!(row(LIST_ROW), "e8");
        assert_eq!(row(PROMPT_ROW - 2), "e29");
        assert_eq!(row(PROMPT_ROW - 1), "");
    }

    #[test]
    fn the_prompt_freezes_the_choice() {
        let mut r = screen(3, 0);
        assert_eq!(r.step(Action::CursorDown), RewindEffect::None);
        assert_eq!(r.step(Action::Confirm), RewindEffect::None);
        assert!(r.is_confirming());
        // Up and down do nothing while it's up.
        r.step(Action::CursorDown);
        assert_eq!(r.focused().map(|e| e.point), Some(1));
        r.step(Action::CursorUp);
        assert_eq!(r.focused().map(|e| e.point), Some(1));
        assert_eq!(r.step(Action::Confirm), RewindEffect::Rewind(1));
    }

    #[test]
    fn using_an_item_names_the_target_unless_it_is_the_user() {
        let before = skirmish(&ctx(), 20);
        let used = |target| Replayed {
            before: before.clone(),
            command: Command::Act {
                unit: UnitId(1),
                dest: Pos::new(6, 2),
                action: UnitAction::UseItem {
                    pack_index: 0,
                    target: UnitId(target),
                },
            },
            events: vec![],
        };
        assert_eq!(describe(&used(1)), "Turn 1 · Test Lord used an item");
        assert_eq!(
            describe(&used(2)),
            "Turn 1 · Test Lord used an item on Test Knight"
        );
    }

    #[test]
    fn phase_names() {
        assert_eq!(phase_name(Phase::Player), "Player");
        assert_eq!(phase_name(Phase::Enemy), "Enemy");
        assert_eq!(phase_name(Phase::Other), "Ally");
    }
}
