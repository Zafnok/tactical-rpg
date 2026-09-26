//! The first-launch "Pick your layout" screen (`docs/design/controls.md`):
//! one panel per [`Layout`] with a small keyboard diagram and a legend, both
//! read from that layout's bindings.

use super::print_centred;
use crate::color::UiColor;
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::{Action, Chord, Key, Keymap, Layout};
use crate::screen::{Ctx, FrameInput, Screen, Transition};
use crate::widgets::help::{all_key_names, cursor_keys_name, help_line, key_name};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// Title text.
pub const TITLE: &str = "Pick your layout";

/// Row of the title.
const TITLE_ROW: i32 = 1;
/// Top row of the first panel.
const FIRST_PANEL_ROW: i32 = 3;
/// Panel size in cells, border included.
const PANEL_W: i32 = 76;
const PANEL_H: i32 = 13;
/// Blank rows between panels.
const PANEL_GAP: i32 = 1;

/// Offsets inside a panel: the keyboard's left edge, its three rows, and
/// the legend's column and first row.
const KEYS_X: i32 = 3;
const TOP_ROW_Y: i32 = 4;
const HOME_ROW_Y: i32 = 5;
const SPACE_ROW_Y: i32 = 7;
const LEGEND_X: i32 = 50;
const LEGEND_Y: i32 = 2;
/// Width of the key column in the legend.
const LEGEND_KEY_W: usize = 12;

/// Width of one key cap, `[w]`.
const CAP_W: i32 = 3;
/// The two letter rows drawn, left to right as on a QWERTY keyboard.
const TOP_ROW: [Key; 10] = [
    Key::Q,
    Key::W,
    Key::E,
    Key::R,
    Key::T,
    Key::Y,
    Key::U,
    Key::I,
    Key::O,
    Key::P,
];
const HOME_ROW: [Key; 10] = [
    Key::A,
    Key::S,
    Key::D,
    Key::F,
    Key::G,
    Key::H,
    Key::J,
    Key::K,
    Key::L,
    Key::Semicolon,
];
/// Left edge of the arrow-key cluster, right of the letter rows.
const ARROWS_X: i32 = KEYS_X + 10 * CAP_W + 4;
/// The space bar: left edge (under `d`) and width.
const SPACE_X: i32 = KEYS_X + 1 + 2 * CAP_W;
const SPACE_W: usize = 19;

/// The name shown for a layout.
pub fn label(layout: Layout) -> &'static str {
    match layout {
        Layout::RightHanded => "Right-handed",
        Layout::LeftHanded => "Left-handed",
    }
}

/// Shows both layouts and saves the one picked. It can't be cancelled: the
/// game needs a layout. Pops itself once a layout is picked.
#[derive(Debug, Clone)]
pub struct LayoutPickerScreen {
    menu: Menu,
}

impl LayoutPickerScreen {
    /// The picker with the first layout (right-handed) focused.
    pub fn new() -> Self {
        Self {
            menu: Menu::new(
                Layout::ALL
                    .iter()
                    .map(|&l| MenuItem::new(label(l)))
                    .collect(),
            ),
        }
    }

    /// The focused layout.
    pub fn focused(&self) -> Layout {
        Layout::ALL
            .get(self.menu.focus())
            .copied()
            .unwrap_or(Layout::RightHanded)
    }

    /// The bottom help line. Before any layout is chosen that is
    /// `w/Up s/Down choose · f/j/Enter/Space pick`.
    pub fn help(ctx: &Ctx) -> String {
        let km = &ctx.keymap;
        let choose = all_key_names(km, Action::CursorUp)
            .zip(all_key_names(km, Action::CursorDown))
            .map(|(up, down)| format!("{up} {down}"));
        help_line(&[
            (choose, "choose"),
            (all_key_names(km, Action::Confirm), "pick"),
        ])
    }

    /// The legend for `km`: `(keys, what they do)`, unbound actions left out.
    pub fn legend(km: &Keymap) -> Vec<(String, &'static str)> {
        let rows = [
            (cursor_keys_name(km), "move"),
            (all_key_names(km, Action::Confirm), "select"),
            (all_key_names(km, Action::Cancel), "back"),
            (key_name(km, Action::PrevUnit), "prev unit"),
            (key_name(km, Action::NextUnit), "next unit"),
            (key_name(km, Action::Info), "unit info"),
            (key_name(km, Action::DangerZone), "danger zone"),
            (key_name(km, Action::EndTurn), "end turn"),
            (key_name(km, Action::ToggleAutoEnd), "auto-end"),
        ];
        rows.into_iter()
            .filter_map(|(keys, what)| keys.map(|k| (k, what)))
            .collect()
    }

    /// Draws `layout`'s panel with its top-left corner at `(x, y)`.
    fn draw_panel(ctx: &Ctx, buf: &mut GlyphBuffer, layout: Layout, focused: bool, x: i32, y: i32) {
        let c = |u| ctx.palette.get(u);
        let bg = c(UiColor::PanelBg);
        let rect = Rect::new(x, y, PANEL_W, PANEL_H);
        buf.fill_rect(rect, Cell::new(' ', c(UiColor::Text), bg));
        let (style, border, title) = if focused {
            (
                BoxStyle::Double,
                c(UiColor::PanelBorderFocus),
                format!(" ► {} ", label(layout)),
            )
        } else {
            (
                BoxStyle::Single,
                c(UiColor::PanelBorder),
                format!(" {} ", label(layout)),
            )
        };
        buf.draw_box(rect, style, border, bg);
        let title_fg = if focused {
            c(UiColor::TextHighlight)
        } else {
            c(UiColor::Text)
        };
        buf.print(x + 2, y, &title, title_fg, bg);

        let km = Keymap::for_layout(&ctx.content.keymap, layout);
        let cap_fg = |key| c(key_role(&km, key));
        let dim = c(UiColor::TextDim);
        let mut cap = |cx: i32, cy: i32, key: Key, glyph: &str| {
            buf.print(cx, cy, "[", dim, bg);
            buf.print(cx + 1, cy, glyph, cap_fg(key), bg);
            buf.print(cx + 2, cy, "]", dim, bg);
        };
        for (col, key) in (0..).zip(TOP_ROW) {
            cap(x + KEYS_X + col * CAP_W, y + TOP_ROW_Y, key, key.name());
        }
        for (col, key) in (0..).zip(HOME_ROW) {
            cap(
                x + KEYS_X + 1 + col * CAP_W,
                y + HOME_ROW_Y,
                key,
                key.name(),
            );
        }
        cap(x + ARROWS_X + CAP_W, y + TOP_ROW_Y, Key::Up, "↑");
        for (col, (key, glyph)) in
            (0..).zip([(Key::Left, "←"), (Key::Down, "↓"), (Key::Right, "→")])
        {
            cap(x + ARROWS_X + col * CAP_W, y + HOME_ROW_Y, key, glyph);
        }
        let space = format!("[{:^w$}]", Key::Space.name(), w = SPACE_W - 2);
        buf.print(x + SPACE_X, y + SPACE_ROW_Y, &space, dim, bg);
        let name_x = x + SPACE_X + (i32::try_from(SPACE_W).unwrap_or(0) - 5) / 2;
        buf.print(
            name_x,
            y + SPACE_ROW_Y,
            Key::Space.name(),
            cap_fg(Key::Space),
            bg,
        );

        for (row, (keys, what)) in (y + LEGEND_Y..).zip(Self::legend(&km)) {
            let fg = if what == "move" {
                c(UiColor::Player)
            } else {
                c(UiColor::TextHighlight)
            };
            buf.print(x + LEGEND_X, row, &keys, fg, bg);
            let what_x = x + LEGEND_X + i32::try_from(LEGEND_KEY_W).unwrap_or(0);
            buf.print(what_x, row, what, c(UiColor::Text), bg);
        }
    }
}

/// The colour a key cap's label gets in `km`: movement keys `player`,
/// other bound keys `text_highlight`, unbound keys `text_dim`.
fn key_role(km: &Keymap, key: Key) -> UiColor {
    match km.action(Chord::plain(key)) {
        Some(action) if action.is_repeatable() => UiColor::Player,
        Some(_) => UiColor::TextHighlight,
        None => UiColor::TextDim,
    }
}

impl Default for LayoutPickerScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for LayoutPickerScreen {
    fn name(&self) -> &'static str {
        "layout_picker"
    }

    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
        for &action in &input.actions {
            // Cancel does nothing: a layout must be picked.
            if let Some(MenuEvent::Chosen(i)) = self.menu.handle(action) {
                let layout = Layout::ALL.get(i).copied().unwrap_or(Layout::RightHanded);
                // If saving fails the layout is still used this session;
                // the player is just asked again next launch.
                let _ = ctx.choose_layout(layout);
                return Transition::Pop;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let c = |u| ctx.palette.get(u);
        let black = c(UiColor::Black);
        buf.fill_rect(buf.bounds(), Cell::new(' ', c(UiColor::Text), black));
        print_centred(buf, TITLE_ROW, TITLE, c(UiColor::TextHighlight), black);
        let x = (i32::from(buf.width()) - PANEL_W) / 2;
        for (i, layout) in (0..).zip(Layout::ALL) {
            let y = FIRST_PANEL_ROW + i * (PANEL_H + PANEL_GAP);
            Self::draw_panel(ctx, buf, layout, layout == self.focused(), x, y);
        }
        let bottom = i32::from(buf.height()) - 1;
        print_centred(buf, bottom, &Self::help(ctx), c(UiColor::TextDim), black);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::{CONSOLE_H, CONSOLE_W};
    use crate::screen::tests::ctx;

    fn input(actions: &[Action]) -> FrameInput {
        FrameInput::new(actions.to_vec(), 0.0, vec![])
    }

    fn first_launch_ctx() -> Ctx {
        Ctx::embedded().unwrap()
    }

    #[test]
    fn down_and_confirm_pick_left_handed_and_save_it() {
        let mut c = first_launch_ctx();
        let mut p = LayoutPickerScreen::new();
        assert_eq!(p.name(), "layout_picker");
        assert_eq!(p.focused(), Layout::RightHanded);
        let t = p.update(&mut c, &input(&[Action::CursorDown]));
        assert_eq!(format!("{t:?}"), "None");
        assert_eq!(p.focused(), Layout::LeftHanded);
        assert_eq!(c.layout(), None);
        let t = p.update(&mut c, &input(&[Action::Confirm]));
        assert_eq!(format!("{t:?}"), "Pop");
        assert_eq!(c.layout(), Some(Layout::LeftHanded));
        assert_eq!(c.saved_layout(), Some(Layout::LeftHanded));
    }

    #[test]
    fn confirm_picks_the_focused_layout() {
        let mut c = first_launch_ctx();
        let mut p = LayoutPickerScreen::new();
        let t = p.update(
            &mut c,
            &input(&[Action::CursorUp, Action::CursorUp, Action::Confirm]),
        );
        assert_eq!(format!("{t:?}"), "Pop");
        assert_eq!(c.saved_layout(), Some(Layout::RightHanded));
    }

    #[test]
    fn cancel_and_other_actions_do_nothing() {
        let mut c = first_launch_ctx();
        let mut p = LayoutPickerScreen::default();
        let t = p.update(
            &mut c,
            &input(&[Action::Cancel, Action::Info, Action::CursorLeft]),
        );
        assert_eq!(format!("{t:?}"), "None");
        assert_eq!(c.layout(), None);
        assert_eq!(p.focused(), Layout::RightHanded);
    }

    #[test]
    fn help_names_the_picker_keys() {
        assert_eq!(
            LayoutPickerScreen::help(&first_launch_ctx()),
            "w/Up s/Down choose · f/j/Enter/Space pick"
        );
        // Later (from Options) it names the chosen layout's keys.
        assert_eq!(LayoutPickerScreen::help(&ctx()), "Up Down choose · f pick");
    }

    #[test]
    fn legend_comes_from_each_layout() {
        let def = &first_launch_ctx().content.keymap;
        let legend = |l| {
            LayoutPickerScreen::legend(&Keymap::for_layout(def, l))
                .into_iter()
                .map(|(k, w)| format!("{k} {w}"))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            legend(Layout::RightHanded),
            [
                "arrows move",
                "f select",
                "d/Escape back",
                "a prev unit",
                "s next unit",
                "e unit info",
                "w danger zone",
                "Space end turn",
                "Shift+Space auto-end",
            ]
        );
        assert_eq!(
            legend(Layout::LeftHanded),
            [
                "wasd move",
                "j select",
                "k/Escape back",
                "; prev unit",
                "l next unit",
                "i unit info",
                "o danger zone",
                "Space end turn",
                "Shift+Space auto-end",
            ]
        );
        assert!(
            LayoutPickerScreen::legend(&Keymap::for_layout(
                &trpg_content::KeymapDef::default(),
                Layout::LeftHanded
            ))
            .is_empty()
        );
    }

    #[test]
    fn key_roles() {
        let def = &first_launch_ctx().content.keymap;
        let left = Keymap::for_layout(def, Layout::LeftHanded);
        assert_eq!(key_role(&left, Key::W), UiColor::Player);
        assert_eq!(key_role(&left, Key::J), UiColor::TextHighlight);
        assert_eq!(key_role(&left, Key::Up), UiColor::TextDim);
    }

    #[test]
    fn labels() {
        assert_eq!(label(Layout::RightHanded), "Right-handed");
        assert_eq!(label(Layout::LeftHanded), "Left-handed");
    }

    #[test]
    fn covers_the_whole_buffer() {
        let c = first_launch_ctx();
        let stale = Cell::new(
            'x',
            c.palette.get(UiColor::Enemy),
            c.palette.get(UiColor::Enemy),
        );
        let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
        LayoutPickerScreen::new().draw(&c, &mut buf);
        let left = (0..i32::from(CONSOLE_H))
            .flat_map(|y| (0..i32::from(CONSOLE_W)).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y) == Some(&stale))
            .count();
        assert_eq!(left, 0);
    }
}
