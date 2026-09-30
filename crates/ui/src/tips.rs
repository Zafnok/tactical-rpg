//! One-time contextual tips (ticket 0406): which have been seen (kept in
//! [`Storage`] under [`TIPS_SEEN_KEY`]), key names in tip text, and the box
//! that shows one. The battle screen decides when a trigger has happened.

use std::collections::BTreeSet;

use trpg_content::tip::{CURSOR_PLACEHOLDER, placeholders};

use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::{Action, Keymap};
use crate::screens::battle::layout::MAP_VIEW;
use crate::storage::{Storage, StorageError};
use crate::widgets::help::{cursor_keys_name, key_name};

/// [`Storage`] key under which the ids of the tips already shown are saved,
/// one per line.
pub const TIPS_SEEN_KEY: &str = "tips_seen";

/// The tips already shown to this profile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TipsSeen {
    ids: BTreeSet<String>,
}

impl TipsSeen {
    /// The ids saved in `storage`; none if nothing is saved or it can't be
    /// read (the tips then show again).
    pub fn load(storage: &dyn Storage) -> Self {
        let saved = storage.read(TIPS_SEEN_KEY).ok().flatten();
        let ids = saved
            .iter()
            .flat_map(|s| s.lines())
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect();
        Self { ids }
    }

    /// Whether the tip `id` was shown.
    pub fn contains(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    /// Records that the tip `id` was shown and saves the list. The tip
    /// counts as seen even if saving fails (it then shows again next
    /// launch).
    pub fn mark(&mut self, id: &str, storage: &mut dyn Storage) {
        if self.ids.insert(id.to_owned()) {
            let text: Vec<&str> = self.ids.iter().map(String::as_str).collect();
            storage.write(TIPS_SEEN_KEY, &text.join("\n")).ok();
        }
    }
}

/// Forgets every tip seen, so they all show again (for the Options menu's
/// "reset tips", 0805).
pub fn reset_tips(storage: &mut dyn Storage) -> Result<(), StorageError> {
    storage.delete(TIPS_SEEN_KEY)
}

/// `text` with each `{Action}` replaced by the key `keymap` binds to that
/// action (its primary key), `{Cursor}` by the cursor keys, and
/// [`NOT_MAPPED`](crate::widgets::help::NOT_MAPPED) for one with no key.
/// Anything else in braces is left alone.
pub fn fill_placeholders(text: &str, keymap: &Keymap) -> String {
    let mut out = text.to_owned();
    for name in placeholders(text) {
        let keys = if name == CURSOR_PLACEHOLDER {
            cursor_keys_name(keymap)
        } else if let Some(action) = Action::from_name(name) {
            key_name(keymap, action)
        } else {
            continue;
        };
        out = out.replace(&format!("{{{name}}}"), &keys);
    }
    out
}

/// Draws a tip: a double-line box with `title` over its top border and the
/// `body` lines inside, near the top of the map view, and `close` (e.g.
/// `f close`) under them.
pub fn draw_tip(buf: &mut GlyphBuffer, palette: &Palette, title: &str, body: &str, close: &str) {
    let c = |u| palette.get(u);
    let bg = c(UiColor::PanelBg);
    let lines: Vec<&str> = body.lines().collect();
    let widest = lines
        .iter()
        .map(|l| l.chars().count())
        .chain([close.chars().count(), title.chars().count() + 2])
        .max()
        .unwrap_or(0);
    let cells = |n: usize| i32::try_from(n).unwrap_or(0);
    let w = cells(widest) + 4;
    // Border, blank row, the lines, blank row, the close hint, border.
    let h = cells(lines.len()) + 5;
    let rect = Rect::new(MAP_VIEW.x + (MAP_VIEW.w - w) / 2, MAP_VIEW.y + 1, w, h);
    buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
    buf.draw_box(rect, BoxStyle::Double, c(UiColor::PanelBorder), bg);
    buf.print(
        rect.x + 2,
        rect.y,
        &format!(" {title} "),
        c(UiColor::TextHighlight),
        bg,
    );
    let mut y = rect.y + 2;
    for line in lines {
        buf.print(rect.x + 2, y, line, c(UiColor::Text), bg);
        y += 1;
    }
    buf.print(rect.x + 2, y + 1, close, c(UiColor::TextDim), bg);
}

#[cfg(test)]
mod tests {
    use trpg_content::{Chord, RepeatDef};

    use super::*;
    use crate::input::Layout;
    use crate::storage::MemoryStorage;
    use crate::widgets::help::NOT_MAPPED;

    fn keymap(pairs: &[(&str, Action)]) -> Keymap {
        Keymap::new(
            pairs.iter().map(|&(c, a)| (Chord::parse(c).unwrap(), a)),
            RepeatDef::default(),
        )
    }

    #[test]
    fn the_box_is_as_wide_as_its_widest_line_or_title() {
        let content = trpg_content::load_embedded().unwrap();
        let palette = Palette::new(&content.palette).unwrap();
        let blank = Cell::new(' ', palette.get(UiColor::Text), palette.get(UiColor::Black));
        let mut buf = GlyphBuffer::new(100, 32, blank);
        // A title of 20 cells gives 22 inside the borders, plus 2 margins.
        draw_tip(
            &mut buf,
            &palette,
            &"T".repeat(20),
            "a
bb",
            "x",
        );
        let (w, h) = (26, 2 + 5);
        let x = MAP_VIEW.x + (MAP_VIEW.w - w) / 2;
        let y = MAP_VIEW.y + 1;
        let glyph = |x, y| buf.get(x, y).map(|c| c.glyph);
        assert_eq!(glyph(x, y), Some('╔'));
        assert_eq!(glyph(x + w - 1, y), Some('╗'));
        assert_eq!(glyph(x + w, y), Some(' '));
        assert_eq!(glyph(x, y + h - 1), Some('╚'));
        assert_eq!(glyph(x + 2, y + 2), Some('a'));
        assert_eq!(glyph(x + 3, y + 3), Some('b'));
        assert_eq!(glyph(x + 2, y + 5), Some('x'));
    }

    #[test]
    fn seen_tips_are_saved_and_loaded() {
        let mut storage = MemoryStorage::new();
        let mut seen = TipsSeen::load(&storage);
        assert!(!seen.contains("a"));
        seen.mark("a", &mut storage);
        seen.mark("b", &mut storage);
        seen.mark("a", &mut storage);
        assert_eq!(
            storage.read(TIPS_SEEN_KEY).unwrap().as_deref(),
            Some("a\nb")
        );
        let again = TipsSeen::load(&storage);
        assert!(again.contains("a") && again.contains("b") && !again.contains("c"));
        assert_eq!(again, seen);
    }

    #[test]
    fn resetting_forgets_every_tip() {
        let mut storage = MemoryStorage::new();
        TipsSeen::default().mark("a", &mut storage);
        let mut seen = TipsSeen::load(&storage);
        seen.mark("b", &mut storage);
        reset_tips(&mut storage).unwrap();
        assert_eq!(TipsSeen::load(&storage), TipsSeen::default());
        // Nothing to reset is fine.
        reset_tips(&mut storage).unwrap();
    }

    #[test]
    fn placeholders_show_the_bound_keys() {
        let km = keymap(&[
            ("f", Action::Confirm),
            ("d", Action::Cancel),
            ("Up", Action::CursorUp),
            ("Left", Action::CursorLeft),
            ("Down", Action::CursorDown),
            ("Right", Action::CursorRight),
        ]);
        assert_eq!(
            fill_placeholders("{Confirm}/{Cancel} {Cursor} {Confirm}", &km),
            "f/d arrows f"
        );
        // Unbound and unknown placeholders.
        assert_eq!(
            fill_placeholders("{Rewind} {Nope} {", &km),
            format!("{NOT_MAPPED} {{Nope}} {{")
        );
        // A cursor action with no key: the whole cursor placeholder.
        let no_cursor = keymap(&[("f", Action::Confirm)]);
        assert_eq!(
            fill_placeholders("{Cursor} {Confirm}", &no_cursor),
            "! not mapped f"
        );
    }

    #[test]
    fn placeholders_follow_the_layout() {
        let content = trpg_content::load_embedded().unwrap();
        let text = "{Confirm} {Cursor} {Rewind}";
        let right = Keymap::for_layout(&content.keymap, Layout::RightHanded);
        let left = Keymap::for_layout(&content.keymap, Layout::LeftHanded);
        assert_eq!(fill_placeholders(text, &right), "f arrows r");
        assert_eq!(fill_placeholders(text, &left), "j wasd u");
    }
}
