//! Text dump of a [`GlyphBuffer`] for snapshot tests (ADR-0007):
//!
//! ```text
//! <glyph rows, exactly width chars each, trailing spaces kept>
//! --- colours ---
//! <rows of single-char keys, one per cell>
//! --- legend ---
//! a = fg:text bg:panel_bg
//! b = fg:player bg:#1c3a79
//! ```
//!
//! Each distinct (fg, bg) pair gets a key in first-seen order (row-major):
//! `a-z`, `A-Z`, `0-9`, then further Unicode letters should a screen ever
//! use more than 62 pairs. Colours print as their palette name when one
//! matches exactly, else as `#rrggbb`. Lines end in `\n` on every OS.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::color::{Palette, Rgb};
use crate::glyph_buffer::GlyphBuffer;

/// Shown instead of control characters, which would break the row layout.
const CONTROL_GLYPH: char = '\u{fffd}';

impl GlyphBuffer {
    /// Renders the buffer in the snapshot format described in [`crate::snapshot`].
    pub fn to_snapshot(&self, palette: &Palette) -> String {
        let (w, h) = (i32::from(self.width()), i32::from(self.height()));
        let cells = || (0..h).flat_map(move |y| (0..w).filter_map(move |x| self.get(x, y)));
        let mut keys: HashMap<(Rgb, Rgb), char> = HashMap::new();
        let mut legend: Vec<(char, Rgb, Rgb)> = Vec::new();
        for c in cells() {
            keys.entry((c.fg, c.bg)).or_insert_with(|| {
                let k = key(legend.len());
                legend.push((k, c.fg, c.bg));
                k
            });
        }
        let per_row = usize::from(self.width()).max(1);
        let mut out = String::new();
        for (i, c) in cells().enumerate() {
            out.push(if c.glyph.is_control() {
                CONTROL_GLYPH
            } else {
                c.glyph
            });
            if (i + 1) % per_row == 0 {
                out.push('\n');
            }
        }
        out.push_str("--- colours ---\n");
        for (i, c) in cells().enumerate() {
            out.push(keys[&(c.fg, c.bg)]);
            if (i + 1) % per_row == 0 {
                out.push('\n');
            }
        }
        out.push_str("--- legend ---\n");
        let name = |rgb: Rgb| {
            palette
                .name_of(rgb)
                .map_or_else(|| rgb.to_hex(), str::to_owned)
        };
        for (k, fg, bg) in legend {
            let _ = writeln!(out, "{k} = fg:{} bg:{}", name(fg), name(bg));
        }
        out
    }
}

/// The `n`th legend key.
fn key(n: usize) -> char {
    const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    if let Some(&k) = KEYS.get(n) {
        return char::from(k);
    }
    // Beyond 62 pairs: CJK ideographs, a contiguous run of 20k+ printable
    // chars. Past that (never reached: a buffer holds at most 65535² cells
    // but a real screen far fewer colours) the key repeats as '?'.
    u32::try_from(n - KEYS.len())
        .ok()
        .and_then(|i| i.checked_add(0x4e00))
        .filter(|&c| c <= 0x9fff)
        .and_then(char::from_u32)
        .unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use super::*;
    use crate::color::UiColor;
    use crate::color::tests::game_palette;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::glyph_buffer::{BoxStyle, Cell, Rect};

    #[test]
    fn keys_sequence() {
        assert_eq!(key(0), 'a');
        assert_eq!(key(25), 'z');
        assert_eq!(key(26), 'A');
        assert_eq!(key(51), 'Z');
        assert_eq!(key(52), '0');
        assert_eq!(key(61), '9');
        assert_eq!(key(62), '\u{4e00}');
        assert_eq!(key(63), '\u{4e01}');
        assert_eq!(key(62 + 0x9fff - 0x4e00), '\u{9fff}');
        assert_eq!(key(62 + 0x9fff - 0x4e00 + 1), '?');
        assert_eq!(key(usize::MAX), '?');
    }

    #[test]
    fn exact_format_small() {
        let p = game_palette();
        let text = p.get(UiColor::Text);
        let bg = p.get(UiColor::PanelBg);
        let odd = Rgb::new(1, 2, 3);
        let mut b = GlyphBuffer::new(3, 2, Cell::new(' ', text, bg));
        b.print(0, 0, "hi", odd, bg);
        b.set(2, 1, Cell::new('\n', text, odd));
        assert_eq!(
            b.to_snapshot(&p),
            "hi \n  \u{fffd}\n\
             --- colours ---\n\
             aab\nbbc\n\
             --- legend ---\n\
             a = fg:#010203 bg:panel_bg\n\
             b = fg:text bg:panel_bg\n\
             c = fg:text bg:#010203\n"
        );
    }

    #[test]
    fn empty_buffers() {
        let p = game_palette();
        let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        let expected = "--- colours ---\n--- legend ---\n";
        assert_eq!(GlyphBuffer::new(0, 0, blank).to_snapshot(&p), expected);
        assert_eq!(GlyphBuffer::new(0, 3, blank).to_snapshot(&p), expected);
        assert_eq!(GlyphBuffer::new(3, 0, blank).to_snapshot(&p), expected);
    }

    #[test]
    fn many_pairs_get_distinct_keys() {
        let p = game_palette();
        let mut b = GlyphBuffer::new(70, 1, Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)));
        for x in 0..70u8 {
            b.set(
                i32::from(x),
                0,
                Cell::new('.', Rgb::new(x, 0, 7), Rgb::new(0, 0, 7)),
            );
        }
        let snap = b.to_snapshot(&p);
        let keys_row = snap.lines().nth(2).unwrap();
        let keys: std::collections::HashSet<char> = keys_row.chars().collect();
        assert_eq!(keys.len(), 70);
        assert!(keys_row.starts_with("abc"));
        assert_eq!(snap.lines().count(), 1 + 1 + 1 + 1 + 70);
    }

    #[test]
    fn sample_box() {
        let p = game_palette();
        let c = |u| p.get(u);
        let mut b = GlyphBuffer::new(
            CONSOLE_W / 4,
            CONSOLE_H / 4,
            Cell::new(' ', c(UiColor::Text), c(UiColor::Black)),
        );
        let panel = Rect::new(1, 1, 22, 6);
        b.fill_rect(panel, Cell::new(' ', c(UiColor::Text), c(UiColor::PanelBg)));
        b.draw_box(
            panel,
            BoxStyle::Double,
            c(UiColor::PanelBorderFocus),
            c(UiColor::PanelBg),
        );
        b.print(
            3,
            1,
            " Battle ",
            c(UiColor::TextHighlight),
            c(UiColor::PanelBg),
        );
        b.print(3, 3, "Aldo", c(UiColor::Player), c(UiColor::PanelBg));
        b.print_fg(8, 3, "vs", c(UiColor::TextDim));
        b.print(11, 3, "Brigand", c(UiColor::Enemy), c(UiColor::PanelBg));
        b.print(3, 4, "HP", c(UiColor::Text), c(UiColor::PanelBg));
        b.print(6, 4, "■■■■", c(UiColor::HpHigh), c(UiColor::PanelBg));
        b.blend_bg(Rect::new(3, 5, 4, 1), c(UiColor::MoveRange), 1.0);
        b.print_fg(3, 5, "move", c(UiColor::Text));
        b.blend_bg(Rect::new(20, 5, 2, 1), Rgb::new(255, 255, 255), 0.5);
        let snap = b.to_snapshot(&p);
        assert_eq!(snap, b.to_snapshot(&p), "deterministic");
        assert_snapshot!(snap);
    }
}
