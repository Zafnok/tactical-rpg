//! The credits screen (ticket 0808; `docs/design/audio.md` rule 3): every
//! third-party work the game uses, grouped by kind, in a scrolling panel.
//! Each entry shows its title, author, license and source link, which is
//! what CC BY asks for. The list is [`trpg_content::Credits`]: the audio
//! manifest's credits and `assets/data/credits.ron`, merged.
//!
//! The list rolls by itself (Nick, PR 148: "credits should auto scroll …
//! manual paging/scrolling can be toggled"), and the title music plays.
//! Confirm stops and restarts the rolling; the cursor keys stop it and
//! scroll by hand.

use trpg_content::{CreditGroup, Credits};

use super::print_centred;
use super::title::TITLE_MUSIC;
use crate::audio::MenuSound;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{SEPARATOR, cursor_keys_name, help_line, key_name};
use crate::widgets::wrap::word_wrap;

/// Screen title, on the panel's border.
pub const TITLE: &str = "Credits";

/// The panel, in cells.
const PANEL: Rect = Rect::new(2, 1, 96, 29);
/// Column of the group headings: one blank cell in from the panel's left
/// border (a test pins it to the panel).
const HEADING_X: i32 = 4;
/// Column of an entry's first line, two cells right of the headings.
const ENTRY_X: i32 = HEADING_X + 2;
/// Column of an entry's license and link, two cells right of its title.
const DETAIL_X: i32 = ENTRY_X + 2;
/// First row of the list: a blank row under the border.
const LIST_Y: i32 = PANEL.y + 2;
/// Rows of the list on screen at once.
const LIST_H: usize = 25;
/// Last column text may use; the scroll marks' column, one further right,
/// stays clear.
const TEXT_END_X: i32 = PANEL.x + PANEL.w - 4;
/// Shown at the list's top right when there is more above.
const MORE_ABOVE: char = '▲';
/// Shown at the list's bottom right when there is more below.
const MORE_BELOW: char = '▼';

/// Seconds between rows while the list rolls by itself (*tunable*): an
/// entry is three rows, so one goes by every second and a half.
pub const ROW_SECS: f32 = 0.5;
/// Seconds the list rests at its top before it starts to roll, and at its
/// bottom before it starts again from the top (*tunable*).
pub const REST_SECS: f32 = 2.0;

/// The heading a group's credits go under.
pub fn heading(group: CreditGroup) -> &'static str {
    match group {
        CreditGroup::Music => "Music",
        CreditGroup::SoundEffects => "Sound effects",
        CreditGroup::Fonts => "Fonts",
        CreditGroup::Software => "Software",
    }
}

/// What a row of the list is, which sets its column and colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    /// A group heading.
    Heading,
    /// An entry's title and author.
    Entry,
    /// An entry's license and source link.
    Detail,
}

impl Row {
    /// The column the row starts at.
    fn x(self) -> i32 {
        match self {
            Self::Heading => HEADING_X,
            Self::Entry => ENTRY_X,
            Self::Detail => DETAIL_X,
        }
    }

    /// The characters that fit on the row.
    fn width(self) -> usize {
        usize::try_from(TEXT_END_X - self.x() + 1).unwrap_or(1)
    }
}

/// The whole list as rows of text, wrapped to the panel: each group that
/// has credits under its [`heading`], then a blank row, then its entries
/// (`"Title" by Author`, then `License · link`) with a blank row after
/// each. No blank row at the end.
fn rows(credits: &Credits) -> Vec<(Row, String)> {
    let mut rows = Vec::new();
    let mut add = |kind: Row, text: &str| {
        if text.is_empty() {
            rows.push((kind, String::new()));
        }
        for line in word_wrap(text, kind.width()) {
            rows.push((kind, line));
        }
    };
    for group in CreditGroup::ALL {
        let mut entries = credits.in_group(group).peekable();
        if entries.peek().is_none() {
            continue;
        }
        add(Row::Heading, heading(group));
        add(Row::Heading, "");
        for e in entries {
            add(Row::Entry, &format!("\"{}\" by {}", e.title, e.author));
            add(
                Row::Detail,
                &format!("{}{SEPARATOR}{}", e.license, e.source),
            );
            add(Row::Entry, "");
        }
    }
    rows.pop();
    rows
}

/// The credits screen. The list rolls up a row at a time by itself, rests
/// at the bottom and starts again from the top. Confirm stops or restarts
/// the rolling; cursor up and down stop it and scroll a row at a time;
/// Cancel closes the screen.
#[derive(Debug, Clone)]
pub struct CreditsScreen {
    /// The list, wrapped ([`rows`]).
    rows: Vec<(Row, String)>,
    /// Index in `rows` of the first row on screen.
    top: usize,
    /// Whether the list is rolling by itself.
    rolling: bool,
    /// Seconds until the list rolls its next row.
    wait: f32,
    /// Whether the title music has been asked for.
    music_on: bool,
}

impl CreditsScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "credits";

    /// The screen showing the top of `ctx`'s credits, about to roll.
    pub fn new(ctx: &Ctx) -> Self {
        Self {
            rows: rows(&ctx.content.credits),
            top: 0,
            rolling: true,
            wait: REST_SECS,
            music_on: false,
        }
    }

    /// Whether the list is rolling by itself.
    pub fn is_rolling(&self) -> bool {
        self.rolling
    }

    /// The highest `top`: the last row at the bottom of the list.
    fn max_top(&self) -> usize {
        self.rows.len().saturating_sub(LIST_H)
    }

    /// Whether there are rows above the ones on screen.
    pub fn more_above(&self) -> bool {
        self.top > 0
    }

    /// Whether there are rows below the ones on screen.
    pub fn more_below(&self) -> bool {
        self.top < self.max_top()
    }

    /// Scrolls one row down or up, with the menu's tick; nothing (and no
    /// sound) at either end.
    fn scroll(&mut self, ctx: &mut Ctx, down: bool) {
        let to = if down {
            (self.top + 1).min(self.max_top())
        } else {
            self.top.saturating_sub(1)
        };
        if to != self.top {
            self.top = to;
            ctx.audio.menu(MenuSound::Move);
        }
    }

    /// Lets `dt` seconds pass while the list rolls: a row every
    /// [`ROW_SECS`], a rest of [`REST_SECS`] at the bottom, then the top
    /// again and the same rest there. Silent. A list that fits on screen
    /// stays put.
    ///
    /// One frame rolls at most once round the list (every row and the
    /// jump back to the top), so a huge frame time can't keep it busy; a
    /// frame longer than that puts the list back at its top, resting.
    fn roll(&mut self, dt: f32) {
        if !self.rolling || self.max_top() == 0 || !dt.is_finite() {
            return;
        }
        self.wait -= dt;
        for _ in 0..=self.max_top() {
            if self.wait > 0.0 {
                return;
            }
            if self.more_below() {
                self.top += 1;
                self.wait += if self.more_below() {
                    ROW_SECS
                } else {
                    REST_SECS
                };
            } else {
                self.top = 0;
                self.wait += REST_SECS;
            }
        }
        if self.wait <= 0.0 {
            self.top = 0;
            self.wait = REST_SECS;
        }
    }

    /// Confirm: stops the rolling, or starts it again from the rows on
    /// screen after one [`ROW_SECS`].
    fn toggle_rolling(&mut self, ctx: &mut Ctx) {
        self.rolling = !self.rolling;
        self.wait = ROW_SECS;
        ctx.audio.menu(MenuSound::Select);
    }

    /// The bottom help line: the cursor keys `scroll`, the Confirm key
    /// `pause` while the list rolls and `auto-scroll` while it doesn't,
    /// the Cancel key `back`, named from the active keymap.
    pub fn help(&self, ctx: &Ctx) -> String {
        let km = ctx.help_keys();
        let confirm = if self.rolling { "pause" } else { "auto-scroll" };
        help_line(&[
            (Some(cursor_keys_name(km)), "scroll"),
            (Some(key_name(km, Action::Confirm)), confirm),
            (Some(key_name(km, Action::Cancel)), "back"),
        ])
    }
}

impl Screen for CreditsScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        if !self.music_on {
            // The music state ignores a request for the track already on
            // (the title's, when the screen is opened from there).
            ctx.audio.play_music(TITLE_MUSIC);
            self.music_on = true;
        }
        self.roll(input.dt);
        for &action in &input.actions {
            match action {
                Action::CursorUp | Action::CursorDown => {
                    // Scrolling by hand takes over from the rolling.
                    self.rolling = false;
                    self.scroll(ctx, action == Action::CursorDown);
                }
                Action::Confirm => self.toggle_rolling(ctx),
                Action::Cancel => {
                    ctx.audio.menu(MenuSound::Cancel);
                    return Transition::Pop;
                }
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let (black, bg) = (c(UiColor::Black), c(UiColor::PanelBg));
        let (text, dim) = (c(UiColor::Text), c(UiColor::TextDim));
        buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
        buf.fill_rect(PANEL, Cell::new(' ', text, bg));
        buf.draw_box(PANEL, BoxStyle::Single, c(UiColor::PanelBorder), bg);
        let title = format!(" {TITLE} ");
        buf.print(HEADING_X, PANEL.y, &title, c(UiColor::TextHighlight), bg);

        let on_screen = self.rows.iter().skip(self.top).take(LIST_H);
        for (y, (kind, line)) in (LIST_Y..).zip(on_screen) {
            let fg = match kind {
                Row::Heading => c(UiColor::PanelBorderFocus),
                Row::Entry => text,
                Row::Detail => dim,
            };
            buf.print(kind.x(), y, line, fg, bg);
        }
        let last_y = LIST_Y + i32::try_from(LIST_H).unwrap_or(0) - 1;
        let mark_x = TEXT_END_X + 1;
        if self.more_above() {
            buf.print(mark_x, LIST_Y, &MORE_ABOVE.to_string(), dim, bg);
        }
        if self.more_below() {
            buf.print(mark_x, last_y, &MORE_BELOW.to_string(), dim, bg);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &self.help(ctx), dim, black);
    }
}

#[cfg(test)]
mod tests {
    use trpg_content::CreditEntry;

    use super::*;
    use crate::audio::AudioRequest;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::screen::tests::ctx;

    fn entry(group: CreditGroup, title: &str) -> CreditEntry {
        CreditEntry {
            group,
            title: title.into(),
            author: "Ann".into(),
            license: "CC0-1.0".into(),
            source: format!("https://example.org/{title}"),
        }
    }

    /// A context whose credits are `entries`.
    fn ctx_with(entries: Vec<CreditEntry>) -> Ctx {
        let mut c = ctx();
        c.content.credits = Credits { entries };
        c
    }

    fn texts(s: &CreditsScreen) -> Vec<&str> {
        s.rows.iter().map(|(_, t)| t.as_str()).collect()
    }

    fn input(actions: &[Action]) -> FrameInput {
        FrameInput::new(actions.to_vec(), 0.0, vec![])
    }

    /// The sounds an update with `actions` plays, as cue names.
    fn sounds_of(s: &mut CreditsScreen, c: &mut Ctx, actions: &[Action]) -> Vec<String> {
        s.update(c, &input(actions));
        c.audio
            .take()
            .iter()
            .filter(|r| matches!(r, AudioRequest::PlaySound { .. }))
            .filter_map(|r| r.cue().map(str::to_owned))
            .collect()
    }

    fn draw(s: &CreditsScreen, c: &Ctx) -> GlyphBuffer {
        let stale = Cell::new(
            'x',
            c.palette.get(UiColor::Enemy),
            c.palette.get(UiColor::Enemy),
        );
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
        s.draw(c, &mut buf);
        buf
    }

    fn row(buf: &GlyphBuffer, y: i32) -> String {
        (0..i32::from(buf.width()))
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect()
    }

    #[test]
    fn rows_group_the_entries_under_headings() {
        use Row::{Detail, Entry, Heading};
        let c = ctx_with(vec![
            entry(CreditGroup::Music, "One"),
            entry(CreditGroup::Music, "Two"),
            entry(CreditGroup::Fonts, "Font"),
        ]);
        let s = CreditsScreen::new(&c);
        assert_eq!(
            texts(&s),
            [
                "Music",
                "",
                "\"One\" by Ann",
                "CC0-1.0 · https://example.org/One",
                "",
                "\"Two\" by Ann",
                "CC0-1.0 · https://example.org/Two",
                "",
                "Fonts",
                "",
                "\"Font\" by Ann",
                "CC0-1.0 · https://example.org/Font",
            ]
        );
        let kinds: Vec<Row> = s.rows.iter().map(|&(k, _)| k).collect();
        assert_eq!(
            kinds,
            [
                Heading, Heading, Entry, Detail, Entry, Entry, Detail, Entry, Heading, Heading,
                Entry, Detail
            ]
        );
        assert!(CreditsScreen::new(&ctx_with(vec![])).rows.is_empty());
    }

    #[test]
    fn every_group_has_its_heading() {
        let headings: Vec<&str> = CreditGroup::ALL.iter().map(|&g| heading(g)).collect();
        assert_eq!(headings, ["Music", "Sound effects", "Fonts", "Software"]);
    }

    #[test]
    fn long_lines_wrap_inside_the_panel() {
        let mut long = entry(CreditGroup::Music, &"word ".repeat(30));
        long.source = format!("https://example.org/{}", "x".repeat(120));
        let c = ctx_with(vec![long]);
        let s = CreditsScreen::new(&c);
        // The title wraps at a space; the link is cut at the edge.
        assert_eq!(s.rows.len(), 2 + 2 + 3);
        for (kind, text) in &s.rows {
            assert!(text.chars().count() <= kind.width(), "{text:?}");
        }
        assert_eq!(HEADING_X, PANEL.x + 2);
        assert_eq!(Row::Heading.width(), 91);
        assert_eq!(Row::Entry.width(), 89);
        assert_eq!(Row::Detail.width(), 87);
        assert_eq!(s.rows[4].1, "CC0-1.0 ·");
        assert_eq!(s.rows[5].1.chars().count(), 87);
        // Nothing is drawn on or past the panel's border, or in the scroll
        // marks' column.
        let buf = draw(&s, &c);
        for y in LIST_Y..LIST_Y + 7 {
            let line = row(&buf, y);
            assert_eq!(line.chars().nth(95), Some(' '), "{line}");
            assert_eq!(line.chars().nth(96), Some(' '), "{line}");
            assert_eq!(line.chars().nth(97), Some('│'), "{line}");
        }
        assert_eq!(row(&buf, LIST_Y + 5).chars().nth(94), Some('x'));
    }

    /// 20 entries: 2 + 20 × 3 − 1 = 61 rows, 36 more than fit.
    fn long_list() -> Ctx {
        ctx_with(
            (0..20)
                .map(|i| entry(CreditGroup::Music, &format!("Track{i:02}")))
                .collect(),
        )
    }

    #[test]
    fn scrolling_moves_a_row_at_a_time_and_stops_at_the_ends() {
        use Action::{CursorDown, CursorUp};
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        assert_eq!(s.rows.len(), 61);
        assert_eq!((s.top, s.max_top()), (0, 36));
        assert!(!s.more_above() && s.more_below());
        // Already at the top: nothing moves, no sound.
        assert!(sounds_of(&mut s, &mut c, &[CursorUp]).is_empty());
        assert_eq!(s.top, 0);
        assert_eq!(
            sounds_of(&mut s, &mut c, &[CursorDown, CursorDown]),
            ["menu_move"; 2]
        );
        assert_eq!(s.top, 2);
        assert!(s.more_above() && s.more_below());
        assert_eq!(sounds_of(&mut s, &mut c, &[CursorUp]), ["menu_move"]);
        assert_eq!(s.top, 1);
        s.update(&mut c, &input(&[CursorDown; 34]));
        assert_eq!(s.top, 35);
        assert!(s.more_below());
        c.audio.take();
        // The last step down, then the bottom: no further, no sound.
        assert_eq!(
            sounds_of(&mut s, &mut c, &[CursorDown, CursorDown, CursorDown]),
            ["menu_move"]
        );
        assert_eq!(s.top, 36);
        assert!(s.more_above() && !s.more_below());
    }

    #[test]
    fn a_short_list_does_not_scroll() {
        let mut c = ctx_with(vec![entry(CreditGroup::Fonts, "Font")]);
        let mut s = CreditsScreen::new(&c);
        assert!(!s.more_above() && !s.more_below());
        assert!(sounds_of(&mut s, &mut c, &[Action::CursorDown]).is_empty());
        assert_eq!(s.top, 0);
        let buf = draw(&s, &c);
        let all: String = (0..i32::from(CONSOLE_H)).map(|y| row(&buf, y)).collect();
        assert!(!all.contains(MORE_ABOVE) && !all.contains(MORE_BELOW));
        // A list of exactly the rows that fit doesn't scroll either:
        // 2 + 8 × 3 − 1 = 25.
        let fits = ctx_with(
            (0..8)
                .map(|i| entry(CreditGroup::Music, &format!("Track {i}")))
                .collect(),
        );
        let s = CreditsScreen::new(&fits);
        assert_eq!(s.rows.len(), LIST_H);
        assert!(!s.more_below());
    }

    #[test]
    fn cancel_closes_with_the_cancel_sound_and_other_actions_do_nothing() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        assert_eq!(s.name(), "credits");
        assert!(!s.is_overlay());
        let others = [Action::CursorLeft, Action::CursorRight, Action::Info];
        assert!(sounds_of(&mut s, &mut c, &others).is_empty());
        assert!(s.is_rolling());
        assert_eq!(s.top, 0);
        // Actions after Cancel are dropped.
        let actions = [Action::CursorDown, Action::Cancel, Action::CursorDown];
        assert_eq!(format!("{:?}", s.update(&mut c, &input(&actions))), "Pop");
        assert_eq!(s.top, 1);
        let sounds: Vec<String> = c
            .audio
            .take()
            .iter()
            .filter_map(|r| r.cue().map(str::to_owned))
            .collect();
        assert_eq!(sounds, ["menu_move", "menu_cancel"]);
    }

    #[test]
    fn draws_the_rows_from_the_top_one_with_scroll_marks() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        let buf = draw(&s, &c);
        assert!(row(&buf, 1).starts_with("  ┌─ Credits ─"));
        assert!(row(&buf, 3).starts_with("  │ Music "));
        assert!(row(&buf, 5).starts_with("  │   \"Track00\" by Ann "));
        assert!(row(&buf, 6).starts_with("  │     CC0-1.0 · https://example.org/Track00 "));
        // 25 rows: the last is the eighth entry's second line, with the
        // "more below" mark at the right.
        assert!(row(&buf, 27).starts_with("  │     CC0-1.0 · https://example.org/Track07 "));
        assert_eq!(row(&buf, 27).chars().nth(95), Some(MORE_BELOW));
        assert_eq!(row(&buf, 3).chars().nth(95), Some(' '));
        // The row above the bottom border stays blank.
        assert_eq!(row(&buf, 28).replace(' ', ""), "││");
        assert!(row(&buf, 29).starts_with("  └─"));
        assert_eq!(row(&buf, 31).trim(), s.help(&c));

        s.update(&mut c, &input(&[Action::CursorDown; 3]));
        let buf = draw(&s, &c);
        assert!(row(&buf, 3).starts_with("  │     CC0-1.0 · https://example.org/Track00 "));
        assert_eq!(row(&buf, 3).chars().nth(95), Some(MORE_ABOVE));
        assert_eq!(row(&buf, 27).chars().nth(95), Some(MORE_BELOW));

        s.update(&mut c, &input(&[Action::CursorDown; 40]));
        let buf = draw(&s, &c);
        assert!(row(&buf, 27).starts_with("  │     CC0-1.0 · https://example.org/Track19 "));
        assert_eq!(row(&buf, 27).chars().nth(95), Some(' '));
        assert_eq!(row(&buf, 3).chars().nth(95), Some(MORE_ABOVE));
    }

    #[test]
    fn rows_are_coloured_by_kind() {
        let c = long_list();
        let buf = draw(&CreditsScreen::new(&c), &c);
        let fg = |x, y| buf.get(x, y).map(|cell| cell.fg);
        let p = |u| Some(c.palette.get(u));
        assert_eq!(fg(HEADING_X + 1, 1), p(UiColor::TextHighlight));
        assert_eq!(fg(HEADING_X, 3), p(UiColor::PanelBorderFocus));
        assert_eq!(fg(ENTRY_X, 5), p(UiColor::Text));
        assert_eq!(fg(DETAIL_X, 6), p(UiColor::TextDim));
        assert_eq!(fg(TEXT_END_X + 1, 27), p(UiColor::TextDim));
        assert_eq!(fg(50, 31), p(UiColor::TextDim));
    }

    #[test]
    fn help_names_the_layout_keys_and_what_confirm_does() {
        let mut c = ctx();
        let mut s = CreditsScreen::new(&c);
        assert_eq!(s.help(&c), "arrows scroll · f pause · d back");
        s.update(&mut c, &input(&[Action::Confirm]));
        assert_eq!(s.help(&c), "arrows scroll · f auto-scroll · d back");
        c.use_layout(crate::input::Layout::LeftHanded);
        assert_eq!(s.help(&c), "wasd scroll · j auto-scroll · k back");
    }

    /// An update with no actions after `dt` seconds.
    fn pass(s: &mut CreditsScreen, c: &mut Ctx, dt: f32) {
        s.update(c, &FrameInput::new(vec![], dt, vec![]));
    }

    #[test]
    fn the_list_rolls_a_row_every_half_second_after_a_rest() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        assert!(s.is_rolling());
        pass(&mut s, &mut c, 1.5);
        assert_eq!(s.top, 0, "resting at the top");
        pass(&mut s, &mut c, 0.5);
        assert_eq!(s.top, 1, "the rest is over at exactly 2 s");
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 1);
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 2);
        // A long frame rolls every row it covers.
        pass(&mut s, &mut c, 2.0);
        assert_eq!(s.top, 6);
        // Rolling is silent (only the music was asked for).
        let sounds = c.audio.take();
        assert!(
            sounds
                .iter()
                .all(|r| !matches!(r, AudioRequest::PlaySound { .. })),
            "{sounds:?}"
        );
        // A frame time that isn't a number changes nothing.
        pass(&mut s, &mut c, f32::NAN);
        pass(&mut s, &mut c, f32::INFINITY);
        assert_eq!(s.top, 6);
        pass(&mut s, &mut c, 0.5);
        assert_eq!(s.top, 7);
    }

    #[test]
    fn at_the_bottom_the_list_rests_then_starts_again_from_the_top() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        // 2 s rest, then 36 rows: the last lands at 2 + 35 × 0.5 = 19.5 s.
        pass(&mut s, &mut c, 19.25);
        assert_eq!(s.top, 35);
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 36);
        assert!(!s.more_below());
        pass(&mut s, &mut c, 1.75);
        assert_eq!(s.top, 36, "resting at the bottom");
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 0, "back to the top after 2 s");
        pass(&mut s, &mut c, 1.75);
        assert_eq!(s.top, 0, "resting at the top again");
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 1);
    }

    #[test]
    fn a_frame_rolls_at_most_once_round_the_list() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        pass(&mut s, &mut c, 2.0);
        assert_eq!(s.top, 1);
        // From row 1 with half a second to go, once round the list is 37
        // steps: 35 rows down, the jump to the top, and row 1 again, due
        // 21.5 s from now. A frame of 21.75 s does exactly that.
        pass(&mut s, &mut c, 21.75);
        assert_eq!(s.top, 1);
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 2, "the next row was a quarter second away");

        // A frame that reaches past the 37th step (here, to the moment the
        // 38th is due) puts the list back at the top, resting.
        let mut s = CreditsScreen::new(&c);
        pass(&mut s, &mut c, 2.0);
        pass(&mut s, &mut c, 22.0);
        assert_eq!(s.top, 0);
        pass(&mut s, &mut c, 1.75);
        assert_eq!(s.top, 0, "resting");
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 1);

        // So does a frame time far too large to count rows by.
        let mut s = CreditsScreen::new(&c);
        pass(&mut s, &mut c, 5.0);
        assert_eq!(s.top, 7);
        pass(&mut s, &mut c, 1.0e30);
        assert_eq!(s.top, 0);
        pass(&mut s, &mut c, 1.75);
        assert_eq!(s.top, 0, "resting");
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 1);
    }

    #[test]
    fn a_list_that_fits_does_not_roll() {
        let mut c = ctx_with(vec![entry(CreditGroup::Fonts, "Font")]);
        let mut s = CreditsScreen::new(&c);
        pass(&mut s, &mut c, 60.0);
        assert_eq!(s.top, 0);
        assert!((s.wait - REST_SECS).abs() < f32::EPSILON);
    }

    #[test]
    fn confirm_stops_the_rolling_and_starts_it_again() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        pass(&mut s, &mut c, 3.0);
        assert_eq!(s.top, 3);
        assert_eq!(
            sounds_of(&mut s, &mut c, &[Action::Confirm]),
            ["menu_select"]
        );
        assert!(!s.is_rolling());
        pass(&mut s, &mut c, 10.0);
        assert_eq!(s.top, 3, "stopped");
        assert_eq!(
            sounds_of(&mut s, &mut c, &[Action::Confirm]),
            ["menu_select"]
        );
        assert!(s.is_rolling());
        // It goes on from where it is, half a second later.
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 3);
        pass(&mut s, &mut c, 0.25);
        assert_eq!(s.top, 4);
    }

    #[test]
    fn scrolling_by_hand_stops_the_rolling() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        pass(&mut s, &mut c, 3.0);
        assert_eq!(
            sounds_of(&mut s, &mut c, &[Action::CursorUp]),
            ["menu_move"]
        );
        assert_eq!(s.top, 2);
        assert!(!s.is_rolling());
        pass(&mut s, &mut c, 10.0);
        assert_eq!(s.top, 2);
        // Down stops it too, and so does Up with nowhere to go.
        let mut s = CreditsScreen::new(&c);
        s.update(&mut c, &input(&[Action::CursorDown]));
        assert_eq!((s.top, s.is_rolling()), (1, false));
        c.audio.take();
        let mut s = CreditsScreen::new(&c);
        assert!(sounds_of(&mut s, &mut c, &[Action::CursorUp]).is_empty());
        assert_eq!((s.top, s.is_rolling()), (0, false));
        // The frame's time passes before its keys: the row that was due
        // rolls, then Down scrolls one more.
        let mut s = CreditsScreen::new(&c);
        s.update(
            &mut c,
            &FrameInput::new(vec![Action::CursorDown], 2.0, vec![]),
        );
        assert_eq!(s.top, 2);
    }

    #[test]
    fn the_screen_asks_for_the_title_music_once() {
        let mut c = long_list();
        let mut s = CreditsScreen::new(&c);
        let music = |c: &mut Ctx| -> Vec<String> {
            c.audio
                .take()
                .iter()
                .filter(|r| !matches!(r, AudioRequest::PlaySound { .. }))
                .map(|r| r.cue().unwrap_or("-").to_owned())
                .collect()
        };
        pass(&mut s, &mut c, 0.0);
        assert_eq!(music(&mut c), [TITLE_MUSIC]);
        pass(&mut s, &mut c, 1.0);
        s.update(&mut c, &input(&[Action::Confirm, Action::CursorDown]));
        assert!(music(&mut c).is_empty());
    }

    /// The real list: every entry's four parts are on a row.
    #[test]
    fn the_games_credits_show_title_author_license_and_link() {
        let c = ctx();
        let s = CreditsScreen::new(&c);
        // Joined, so a wrapped line reads on.
        let all = texts(&s).join(" ");
        assert!(c.content.credits.entries.len() > 30);
        for e in &c.content.credits.entries {
            assert!(all.contains(&format!("\"{}\" by {}", e.title, e.author)));
            // The link may be wrapped; its start follows the license.
            let start: String = e.source.chars().take(40).collect();
            assert!(
                all.contains(&format!("{}{SEPARATOR}{start}", e.license)),
                "{e:?}"
            );
        }
        assert_eq!(texts(&s)[0], "Music");
        assert!(texts(&s).contains(&"Sound effects"));
        assert!(texts(&s).contains(&"Fonts"));
        assert!(!texts(&s).contains(&"Software"), "hidden for now");
    }
}
