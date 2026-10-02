//! Battle notes (ticket 0411, `docs/design/magic.md`): the battle's
//! strategy hints, shown in a `BATTLE NOTES` box at the start of the battle
//! and again under the objective on the map menu's `Objective` page. While
//! either is up, the units the notes are about blink on the map, and the
//! box keeps off their rows.

use trpg_core::{BattleNote, UnitId};

use super::layout::MAP_VIEW;
use super::map_menu::centred_top;
use crate::widgets::wrap::word_wrap;

/// The title of the notes box.
pub const NOTES_TITLE: &str = "BATTLE NOTES";

/// The heading over the notes on the `Objective` page.
pub const NOTES_HEADING: &str = "Battle notes";

/// The widest a line of a note is, in cells, its marker included.
pub const NOTE_WIDTH: usize = 48;

/// Starts each note; the lines a long note wraps onto are indented by as
/// much.
pub const MARKER: &str = "• ";

/// How long a noted unit stays highlighted, then plain, in seconds.
/// *Tunable.*
pub const BLINK_S: f32 = 0.4;

/// The lines of `notes`: each starts with [`MARKER`] and wraps to
/// [`NOTE_WIDTH`], its further lines indented under its text.
pub fn note_lines(notes: &[BattleNote]) -> Vec<String> {
    let indent = MARKER.chars().count();
    notes
        .iter()
        .flat_map(|note| {
            let lines = word_wrap(&note.text, NOTE_WIDTH - indent);
            lines.into_iter().enumerate().map(move |(i, line)| {
                if i == 0 {
                    format!("{MARKER}{line}")
                } else {
                    format!("{}{line}", " ".repeat(indent))
                }
            })
        })
        .collect()
}

/// Whether a note of `notes` is about `unit`.
pub fn is_noted(notes: &[BattleNote], unit: UnitId) -> bool {
    notes.iter().any(|n| n.units.contains(&unit))
}

/// The top row of a box `h` rows tall that lists the notes: centred on the
/// map view, unless it would then cover one of `rows` (the screen rows of
/// the units the notes are about); then one row from the top of the view,
/// or one row from its bottom, whichever covers none of them. Centred if
/// every place covers one.
pub fn box_top(h: i32, rows: &[i32]) -> i32 {
    let centred = centred_top(h);
    let top = MAP_VIEW.y + 1;
    let bottom = MAP_VIEW.y + MAP_VIEW.h - h - 1;
    let clear = |y: i32| !rows.iter().any(|&r| r >= y && r < y + h);
    [centred, top, bottom]
        .into_iter()
        .find(|&y| clear(y))
        .unwrap_or(centred)
}

/// Whether noted units are highlighted `t` seconds after the notes came
/// up: on for the first [`BLINK_S`], off for the next, and so on.
pub fn blink_on(t: f32) -> bool {
    !t.is_finite() || t.rem_euclid(2.0 * BLINK_S) < BLINK_S
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(text: &str, units: &[u32]) -> BattleNote {
        BattleNote {
            text: text.into(),
            units: units.iter().map(|&u| UnitId(u)).collect(),
        }
    }

    #[test]
    fn notes_wrap_under_their_marker() {
        let notes = [
            note("Frost Elemental: weak to Fire, absorbs Ice.", &[4]),
            note(
                "Burn the forest to cut off the ambush before the riders reach the bridge.",
                &[],
            ),
        ];
        assert_eq!(
            note_lines(&notes),
            [
                "• Frost Elemental: weak to Fire, absorbs Ice.",
                "• Burn the forest to cut off the ambush before",
                "  the riders reach the bridge.",
            ]
        );
        assert_eq!(note_lines(&[]), Vec::<String>::new());
        // A line as wide as it may be isn't wrapped.
        let full = "x".repeat(NOTE_WIDTH - 2);
        assert_eq!(note_lines(&[note(&full, &[])]), [format!("• {full}")]);
        let over = format!("{full} y");
        assert_eq!(note_lines(&[note(&over, &[])]).len(), 2);
    }

    #[test]
    fn noted_units_are_those_a_note_names() {
        let notes = [note("A.", &[4, 6]), note("B.", &[]), note("C.", &[2])];
        let about: Vec<u32> = (1..=7).filter(|&u| is_noted(&notes, UnitId(u))).collect();
        assert_eq!(about, [2, 4, 6]);
        assert!(!is_noted(&[], UnitId(1)));
    }

    #[test]
    fn the_blink_starts_on_and_alternates() {
        assert!(blink_on(0.0));
        assert!(blink_on(BLINK_S * 0.99));
        assert!(!blink_on(BLINK_S));
        assert!(!blink_on(BLINK_S * 1.99));
        assert!(blink_on(BLINK_S * 2.0));
        assert!(!blink_on(BLINK_S * 3.5));
        assert!(blink_on(f32::NAN));
    }

    #[test]
    fn the_box_keeps_off_the_noted_units_rows() {
        // A box of 8 rows is centred on rows 11..19 of the 30-row view.
        assert_eq!(centred_top(8), 11);
        assert_eq!(box_top(8, &[]), 11);
        assert_eq!(box_top(8, &[10, 19]), 11, "just above and below it");
        // Covered: the top (rows 1..9), unless a unit is there too; then
        // the bottom (rows 21..29).
        assert_eq!(box_top(8, &[11]), 1);
        assert_eq!(box_top(8, &[18, 9, 0]), 1);
        assert_eq!(box_top(8, &[18, 1]), 21);
        assert_eq!(box_top(8, &[11, 8]), 21);
        assert_eq!(box_top(8, &[15, 8, 20, 29]), 21);
        // Nowhere is clear: centred.
        assert_eq!(box_top(8, &[15, 8, 21]), 11);
        assert_eq!(box_top(8, &[15, 5, 28]), 11);
    }

    /// Every battle's notes fit the map view, on the `Objective` page too
    /// (two lines, a blank and the heading over them, in a box with a
    /// blank row inside its border).
    #[test]
    fn every_battles_notes_fit_the_map_view() {
        let content = trpg_content::load_embedded().unwrap_or_else(|e| panic!("{e}"));
        for (id, battle) in &content.battles {
            let lines = note_lines(&battle.battle_notes);
            let rows = i32::try_from(lines.len()).unwrap_or(i32::MAX);
            assert!(rows + 4 + 4 <= MAP_VIEW.h, "{id}: {rows} lines");
            assert!(
                lines.iter().all(|l| l.chars().count() <= NOTE_WIDTH),
                "{id}"
            );
        }
    }
}
