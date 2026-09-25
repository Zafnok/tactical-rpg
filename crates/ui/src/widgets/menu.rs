//! A vertical list menu: title menu now, action and map menus later.

use crate::color::{Palette, UiColor};
use crate::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::input::Action;

/// One menu entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Text shown.
    pub label: String,
    /// Disabled items are drawn dim, skipped by the cursor and can't be chosen.
    pub enabled: bool,
}

impl MenuItem {
    /// An enabled item.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
        }
    }

    /// A disabled item.
    pub fn disabled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: false,
        }
    }
}

/// What a menu reports back to its screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEvent {
    /// Confirm on the enabled item at this index.
    Chosen(usize),
    /// Cancel was pressed.
    Cancelled,
}

/// A vertical list of items in a single-line box. `CursorDown`/`CursorUp`
/// move the focus (wrapping, skipping disabled items), `Confirm` chooses the
/// focused item, `Cancel` cancels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    items: Vec<MenuItem>,
    focus: usize,
}

impl Menu {
    /// A menu focused on its first enabled item (or the first item, if none
    /// is enabled).
    pub fn new(items: Vec<MenuItem>) -> Self {
        let focus = items.iter().position(|i| i.enabled).unwrap_or(0);
        Self { items, focus }
    }

    /// The items, in order.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// Index of the focused item.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// Handles one action. Returns an event for `Confirm` on an enabled item
    /// and for `Cancel`; moves the focus for `CursorDown`/`CursorUp`;
    /// ignores everything else.
    pub fn handle(&mut self, action: Action) -> Option<MenuEvent> {
        match action {
            Action::CursorDown => self.step(true),
            Action::CursorUp => self.step(false),
            Action::Confirm => {
                return self
                    .items
                    .get(self.focus)
                    .filter(|i| i.enabled)
                    .map(|_| MenuEvent::Chosen(self.focus));
            }
            Action::Cancel => return Some(MenuEvent::Cancelled),
            _ => {}
        }
        None
    }

    /// Moves the focus to the next (or previous) enabled item, wrapping.
    /// Stays put if no other item is enabled.
    fn step(&mut self, down: bool) {
        let n = self.items.len();
        let next = (1..n)
            .map(|k| {
                if down {
                    (self.focus + k) % n
                } else {
                    (self.focus + n - k) % n
                }
            })
            .find(|&i| self.items[i].enabled);
        if let Some(i) = next {
            self.focus = i;
        }
    }

    /// Box size in cells: the widest label plus a space and a border on each
    /// side, by one row per item plus the border.
    pub fn size(&self) -> (i32, i32) {
        let widest = self
            .items
            .iter()
            .map(|i| i.label.chars().count())
            .max()
            .unwrap_or(0);
        let w = i32::try_from(widest).unwrap_or(i32::MAX).saturating_add(4);
        let h = i32::try_from(self.items.len())
            .unwrap_or(i32::MAX)
            .saturating_add(2);
        (w, h)
    }

    /// Draws the menu with its top-left corner at `(x, y)`: a `panel_bg`
    /// box with a `panel_border` single line, items in `text`, disabled ones
    /// in `text_dim`, and the focused one as a bar of `panel_bg` text on
    /// `panel_border_focus`.
    pub fn draw(&self, palette: &Palette, buf: &mut GlyphBuffer, x: i32, y: i32) {
        let color = |u| palette.get(u);
        let bg = color(UiColor::PanelBg);
        let (w, h) = self.size();
        let rect = Rect::new(x, y, w, h);
        buf.fill_rect(rect, Cell::new(' ', color(UiColor::Text), bg));
        buf.draw_box(rect, BoxStyle::Single, color(UiColor::PanelBorder), bg);
        for (row, (i, item)) in (y + 1..).zip(self.items.iter().enumerate()) {
            let (fg, row_bg) = match (i == self.focus, item.enabled) {
                (_, false) => (color(UiColor::TextDim), bg),
                (true, true) => (bg, color(UiColor::PanelBorderFocus)),
                (false, true) => (color(UiColor::Text), bg),
            };
            buf.fill_rect(Rect::new(x + 1, row, w - 2, 1), Cell::new(' ', fg, row_bg));
            buf.print(x + 2, row, &item.label, fg, row_bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use proptest::prelude::*;

    use super::*;
    use crate::color::tests::game_palette;
    use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorUp, Info};

    fn menu(enabled: &[bool]) -> Menu {
        Menu::new(
            enabled
                .iter()
                .enumerate()
                .map(|(i, &on)| MenuItem {
                    label: format!("item {i}"),
                    enabled: on,
                })
                .collect(),
        )
    }

    fn focus_after(m: &mut Menu, actions: &[Action]) -> usize {
        for &a in actions {
            m.handle(a);
        }
        m.focus()
    }

    #[test]
    fn down_and_up_wrap() {
        let mut m = menu(&[true, true, true]);
        assert_eq!(m.focus(), 0);
        assert_eq!(focus_after(&mut m, &[CursorDown]), 1);
        assert_eq!(focus_after(&mut m, &[CursorDown]), 2);
        assert_eq!(focus_after(&mut m, &[CursorDown]), 0);
        assert_eq!(focus_after(&mut m, &[CursorUp]), 2);
        assert_eq!(focus_after(&mut m, &[CursorUp]), 1);
    }

    #[test]
    fn disabled_items_are_skipped() {
        let mut m = menu(&[false, true, false, true, false]);
        assert_eq!(m.focus(), 1, "starts on the first enabled item");
        assert_eq!(focus_after(&mut m, &[CursorDown]), 3);
        assert_eq!(focus_after(&mut m, &[CursorDown]), 1);
        assert_eq!(focus_after(&mut m, &[CursorUp]), 3);
        assert_eq!(focus_after(&mut m, &[CursorUp]), 1);
    }

    #[test]
    fn a_single_enabled_item_keeps_focus() {
        let mut m = menu(&[false, true, false]);
        assert_eq!(focus_after(&mut m, &[CursorDown, CursorUp, CursorDown]), 1);
    }

    #[test]
    fn nothing_enabled_nothing_chosen() {
        let mut m = menu(&[false, false]);
        assert_eq!(m.focus(), 0);
        assert_eq!(m.handle(CursorDown), None);
        assert_eq!(m.focus(), 0);
        assert_eq!(m.handle(Confirm), None);
        assert_eq!(m.handle(Cancel), Some(MenuEvent::Cancelled));
    }

    #[test]
    fn empty_menu_is_harmless() {
        let mut m = Menu::new(vec![]);
        for a in Action::ALL {
            m.handle(a);
        }
        assert_eq!(m.handle(Confirm), None);
        assert_eq!(m.size(), (4, 2));
    }

    #[test]
    fn events() {
        let mut m = menu(&[true, true]);
        assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(0)));
        assert_eq!(m.handle(CursorDown), None);
        assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(1)));
        assert_eq!(m.handle(Cancel), Some(MenuEvent::Cancelled));
        assert_eq!(m.handle(CursorLeft), None);
        assert_eq!(m.handle(Info), None);
        assert_eq!(m.focus(), 1, "other actions don't move the focus");
    }

    #[test]
    fn item_constructors() {
        assert_eq!(
            MenuItem::new("a"),
            MenuItem {
                label: "a".into(),
                enabled: true
            }
        );
        assert!(!MenuItem::disabled("b").enabled);
        let m = Menu::new(vec![MenuItem::new("a"), MenuItem::disabled("bcd")]);
        assert_eq!(m.items()[1].label, "bcd");
        assert_eq!(m.size(), (7, 4));
    }

    fn render(m: &Menu) -> String {
        let p = game_palette();
        let (w, h) = m.size();
        let mut buf = GlyphBuffer::new(
            u16::try_from(w + 2).unwrap(),
            u16::try_from(h + 2).unwrap(),
            Cell::new(' ', p.get(UiColor::Text), p.get(UiColor::Black)),
        );
        m.draw(&p, &mut buf, 1, 1);
        buf.to_snapshot(&p)
    }

    #[test]
    fn snapshot_with_disabled_item() {
        let mut m = Menu::new(vec![
            MenuItem::new("Move"),
            MenuItem::disabled("Attack"),
            MenuItem::new("Items"),
            MenuItem::new("Wait"),
        ]);
        m.handle(CursorDown);
        assert_eq!(m.focus(), 2);
        assert_snapshot!(render(&m));
    }

    proptest! {
        #[test]
        fn focus_always_lands_on_an_enabled_item(
            enabled in prop::collection::vec(any::<bool>(), 1..8),
            moves in prop::collection::vec(any::<bool>(), 0..20),
        ) {
            let mut m = menu(&enabled);
            for down in moves {
                m.handle(if down { CursorDown } else { CursorUp });
                prop_assert!(m.focus() < enabled.len());
                if enabled.iter().any(|&e| e) {
                    prop_assert!(enabled[m.focus()]);
                    prop_assert_eq!(m.handle(Confirm), Some(MenuEvent::Chosen(m.focus())));
                }
            }
        }

        #[test]
        fn down_then_up_returns(enabled in prop::collection::vec(any::<bool>(), 1..8)) {
            let mut m = menu(&enabled);
            let start = m.focus();
            m.handle(CursorDown);
            m.handle(CursorUp);
            prop_assert_eq!(m.focus(), start);
        }
    }
}
