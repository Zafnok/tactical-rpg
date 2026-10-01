//! Scripted tests of the player's key bindings through the real game
//! (ticket 0217, `docs/design/controls.md` *Rebinding keys*): rebinding
//! takes effect at once, `Esc` always cancels, and custom keys persist per
//! layout across restarts.

use trpg_ui::harness::Harness;
use trpg_ui::input::{Action, Chord, Layout, LayoutBindings};
use trpg_ui::screen::KEYBINDINGS_KEY;
use trpg_ui::{MemoryStorage, Storage};

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|e| panic!("{e}"))
}

/// `layout`'s current bindings with `edit` applied.
fn edited(h: &Harness, layout: Layout, edit: impl FnOnce(&mut LayoutBindings)) -> LayoutBindings {
    let mut b = h.game().ctx().layout_bindings(layout);
    edit(&mut b);
    b
}

/// Rebinds `layout` as the Key bindings screen would.
fn rebind(h: &mut Harness, layout: Layout, edit: impl FnOnce(&mut LayoutBindings)) {
    let b = edited(h, layout, edit);
    if let Err(e) = h.ctx_mut().set_layout_bindings(layout, b) {
        panic!("saving key bindings: {e}");
    }
}

/// Confirm's first slot on `g`.
fn confirm_on_g(b: &mut LayoutBindings) {
    assert_eq!(b.bind(Action::Confirm, 0, chord("g")), Ok(None));
}

#[test]
fn a_rebound_key_works_at_once_and_the_old_one_doesnt() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    rebind(&mut h, Layout::RightHanded, confirm_on_g);
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
    h.keys("g");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("d");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn escape_cancels_in_both_layouts() {
    for layout in Layout::ALL {
        let mut h = Harness::with_layout(layout);
        h.keys("Enter"); // Not bound: nothing happens.
        assert_eq!(h.screens(), ["title"]);
        let confirm = h.game().ctx().keymap.chords_for(Action::Confirm)[0];
        h.keys(&confirm.to_string());
        assert_eq!(h.screens(), ["title", "mode_select"], "{layout}");
        h.keys("Escape");
        assert_eq!(h.screens(), ["title"], "{layout}");
    }
}

#[test]
fn escape_cancels_even_with_cancel_unbound() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    rebind(&mut h, Layout::RightHanded, |b| b.clear(Action::Cancel, 0));
    h.keys("f d");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("Escape");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn shift_escape_can_be_bound_and_plain_escape_still_cancels() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    rebind(&mut h, Layout::RightHanded, |b| {
        assert_eq!(b.bind(Action::Confirm, 1, chord("Shift+Escape")), Ok(None));
    });
    h.keys("Shift+Escape");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("Escape");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn the_layout_picker_has_escape_as_cancel() {
    let h = Harness::new();
    assert_eq!(h.top_screen(), "layout_picker");
    assert_eq!(
        h.game().ctx().keymap.action(chord("Escape")),
        Some(Action::Cancel)
    );
}

#[test]
fn custom_keys_survive_a_restart() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    rebind(&mut h, Layout::RightHanded, confirm_on_g);
    let mut h = Harness::with_storage(h.into_storage());
    assert_eq!(h.screens(), ["title"]);
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
    h.keys("g");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

#[test]
fn each_layout_keeps_its_own_keys() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    rebind(&mut h, Layout::RightHanded, confirm_on_g);
    // Switch to left-handed: its own default keys.
    h.ctx_mut().choose_layout(Layout::LeftHanded).ok();
    h.keys("g");
    assert_eq!(h.screens(), ["title"]);
    h.keys("j");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("k");
    // And back: the right-handed edit is still there, across a restart too.
    h.ctx_mut().choose_layout(Layout::RightHanded).ok();
    let mut h = Harness::with_storage(h.into_storage());
    h.keys("f");
    assert_eq!(h.screens(), ["title"]);
    h.keys("g");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

#[test]
fn a_left_handed_edit_leaves_right_handed_alone() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    rebind(&mut h, Layout::LeftHanded, |b| {
        assert_eq!(b.bind(Action::Confirm, 0, chord("h")), Ok(None));
    });
    h.keys("f");
    assert_eq!(h.screens(), ["title", "mode_select"]);
    h.keys("d");
    h.ctx_mut().choose_layout(Layout::LeftHanded).ok();
    h.keys("j");
    assert_eq!(h.screens(), ["title"]);
    h.keys("h");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

#[test]
fn corrupt_old_or_clashing_saved_keys_start_repaired() {
    let clash = "PlayerKeys(version: 1, layouts: {\"RightHanded\": {\"Info\": [Some(\"f\")]}})";
    for saved in [
        "not ron at all",
        "PlayerKeys(version: 0, layouts: {})",
        clash,
    ] {
        let mut storage = MemoryStorage::new();
        storage.write("layout", "RightHanded").ok();
        storage.write(KEYBINDINGS_KEY, saved).ok();
        let mut h = Harness::with_storage(Box::new(storage));
        // The defaults: `f` still confirms.
        h.keys("f");
        assert_eq!(h.screens(), ["title", "mode_select"], "{saved}");
        assert_eq!(
            h.game().ctx().layout_bindings(Layout::RightHanded),
            LayoutBindings::defaults(&h.game().ctx().content.keymap, Layout::RightHanded),
            "{saved}"
        );
    }
}
