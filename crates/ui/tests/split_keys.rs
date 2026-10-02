//! Scripted tests of the optional split keys (ticket 0218,
//! `docs/design/controls.md` *Optional split keys*): once Select has a key
//! it picks on the map instead of Confirm, and once Confirm end turn has a
//! key it accepts the end-turn prompt instead of End turn pressed again.
//! With neither bound the battle plays as before (see `battle.rs`).

use trpg_ui::harness::Harness;
use trpg_ui::input::{Action, Chord, Layout};

/// At the title with the right-handed layout, `action`'s first slot on
/// `key`, then Quick Battle, its `PLAYER PHASE` banner closed.
fn quick_battle_with(action: Action, key: &str) -> Harness {
    let mut h = Harness::with_layout(Layout::RightHanded);
    let mut b = h.game().ctx().layout_bindings(Layout::RightHanded);
    let chord = Chord::parse(key).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(b.bind(action, 0, chord), Ok(None));
    if let Err(e) = h.ctx_mut().set_layout_bindings(Layout::RightHanded, b) {
        panic!("saving key bindings: {e}");
    }
    h.keys("Down f Left f f");
    assert_eq!(h.screens(), ["title", "battle"]);
    h
}

/// The key-help line, left of the right-aligned debug hint.
fn help(h: &Harness) -> String {
    let buf = h.game().buffer();
    (0..90)
        .map(|x| buf.get(x, 31).map_or(' ', |c| c.glyph))
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Whether some row of the screen contains `text`.
fn shows(h: &Harness, text: &str) -> bool {
    let buf = h.game().buffer();
    (0..32).any(|y| {
        (0..100)
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect::<String>()
            .contains(text)
    })
}

#[test]
fn select_picks_on_the_map_and_confirm_keeps_menus_and_the_forecast() {
    let mut h = quick_battle_with(Action::Select, "g");
    assert_eq!(
        help(&h),
        "g select · e info · s next unit · r rewind · d menu · Space end turn"
    );
    // Confirm no longer selects the lord under the cursor.
    h.keys("f");
    assert_eq!(
        help(&h),
        "g select · e info · s next unit · r rewind · d menu · Space end turn"
    );
    // Select does; steer beside the brigand at (7, 4).
    h.keys("g Right Right Right Up");
    assert_eq!(help(&h), "arrows move · g move here · d cancel");
    // Confirm doesn't pick the tile; Select does.
    h.keys("f");
    assert_eq!(help(&h), "arrows move · g move here · d cancel");
    h.keys("g").wait(0.5);
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    // Select does nothing in the menu; Confirm picks Attack, then the iron
    // sword, and the forecast opens on Confirm.
    h.keys("g");
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    h.keys("f f");
    assert_eq!(
        help(&h),
        "Left/Right target · Up/Down art · f attack · d back"
    );
    h.keys("g");
    assert_eq!(
        help(&h),
        "Left/Right target · Up/Down art · f attack · d back"
    );
    // Confirm accepts the forecast: the attack plays.
    h.keys("f");
    assert_eq!(help(&h), "d skip · hold f fast");
}

#[test]
fn select_on_an_empty_tile_opens_the_map_menu_and_on_an_enemy_its_range() {
    let mut h = quick_battle_with(Action::Select, "g");
    // To an empty tile left of the lord: Confirm does nothing there.
    h.keys("Left f");
    assert!(help(&h).starts_with("arrows move · g menu"), "{}", help(&h));
    h.keys("g");
    assert_eq!(help(&h), "arrows choose · f confirm · d back");
    h.keys("d");
    // To the brigand at (8, 2): Select shows its range.
    h.keys("Right Right Right Right Right Right Up Up Up");
    assert!(help(&h).contains("g range"), "{}", help(&h));
    h.keys("f");
    assert!(!help(&h).contains("hide range"), "{}", help(&h));
    h.keys("g");
    assert!(help(&h).contains("d hide range"), "{}", help(&h));
}

/// The lord waits where it stands, so three units stay ready.
fn one_wait(h: &mut Harness) {
    h.keys("f f");
    assert_eq!(help(h), "arrows choose · f confirm · d back");
    h.keys("f");
}

#[test]
fn confirm_end_turn_accepts_the_prompt_instead_of_end_turn_again() {
    let mut h = quick_battle_with(Action::ConfirmEndTurn, "Enter");
    one_wait(&mut h);
    // End turn opens the prompt; End turn again does nothing.
    h.keys("Space");
    assert!(shows(&h, "End turn with 3 units ready?"));
    assert_eq!(help(&h), "Enter yes · f yes · d no");
    h.keys("Space");
    assert!(shows(&h, "End turn with 3 units ready?"));
    assert!(!shows(&h, "ENEMY PHASE"));
    // Cancel backs out.
    h.keys("d");
    assert!(!shows(&h, "units ready?"));
    // Confirm end turn ends the turn.
    h.keys("Space Enter");
    assert!(shows(&h, "ENEMY PHASE"), "{}", h.snapshot());
}

#[test]
fn confirm_still_accepts_the_prompt_with_confirm_end_turn_bound() {
    let mut h = quick_battle_with(Action::ConfirmEndTurn, "Enter");
    one_wait(&mut h);
    h.keys("Space f");
    assert!(shows(&h, "ENEMY PHASE"), "{}", h.snapshot());
}

#[test]
fn without_split_keys_end_turn_again_accepts_the_prompt() {
    let mut h = Harness::with_layout(Layout::RightHanded);
    h.keys("Down f Left f f");
    one_wait(&mut h);
    h.keys("Space");
    assert_eq!(help(&h), "Space yes · f yes · d no");
    h.keys("Space");
    assert!(shows(&h, "ENEMY PHASE"));
}
