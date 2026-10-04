use super::*;
use crate::audio::AudioRequest;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::input::Key;
use crate::screen::tests::ctx;

use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Info};

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap()
}

fn act(actions: &[Action]) -> FrameInput {
    FrameInput::new(actions.to_vec(), 0.0, vec![])
}

/// A frame where `c` was pressed (and did nothing as an action).
fn press(c: &str) -> FrameInput {
    act(&[]).with_typing(vec![chord(c)], vec![])
}

/// The screen with the Debug key reserved whatever the build profile.
fn screen(c: &Ctx) -> KeyBindingsScreen {
    let reserved = LayoutBindings::from_def(&c.content.keymap, Layout::RightHanded, true);
    KeyBindingsScreen::new(c).with_bindings(reserved)
}

/// Moves the focus to `action`'s slot `slot`.
fn focus_on(s: &mut KeyBindingsScreen, c: &mut Ctx, action: Action, slot: usize) {
    for _ in 0..=ROWS.len() {
        if s.focus().map(|(a, _)| a) == Some(action) {
            break;
        }
        s.update(c, &act(&[CursorDown]));
    }
    s.update(c, &act(&[CursorLeft, CursorLeft]));
    for _ in 0..slot {
        s.update(c, &act(&[CursorRight]));
    }
    assert_eq!(s.focus(), Some((action, slot)));
}

/// Moves the focus down to Restore defaults. Bounded, so a screen that
/// never gets there fails the test instead of looping for ever.
fn focus_on_restore(s: &mut KeyBindingsScreen, c: &mut Ctx) {
    for _ in 0..=ROWS.len() {
        if s.focus().is_none() {
            return;
        }
        s.update(c, &act(&[CursorDown]));
    }
    panic!("Restore defaults was never reached");
}

/// Focuses `action`'s slot `slot`, confirms, and presses `key`.
fn bind(s: &mut KeyBindingsScreen, c: &mut Ctx, action: Action, slot: usize, key: &str) {
    focus_on(s, c, action, slot);
    s.update(c, &act(&[Confirm]));
    assert!(s.is_capturing());
    s.update(c, &press(key));
}

fn sounds(c: &mut Ctx) -> Vec<String> {
    c.audio
        .take()
        .into_iter()
        .filter_map(|r| match r {
            AudioRequest::PlaySound { cue, .. } => Some(cue),
            _ => None,
        })
        .collect()
}

fn cue(sound: MenuSound) -> String {
    let mut c = ctx();
    c.audio.menu(sound);
    sounds(&mut c).remove(0)
}

fn drawn(s: &KeyBindingsScreen, c: &Ctx) -> GlyphBuffer {
    let stale = Cell::new(
        'x',
        c.palette.get(UiColor::Enemy),
        c.palette.get(UiColor::Enemy),
    );
    let mut buf = GlyphBuffer::new(CONSOLE_W, CONSOLE_H, stale);
    s.draw(c, &mut buf);
    buf
}

fn row_text(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// The console row showing `label`'s action.
fn row_of(buf: &GlyphBuffer, label: &str) -> String {
    (0..i32::from(buf.height()))
        .map(|y| row_text(buf, y))
        .find(|row| row.contains(&format!(" {label}  ")))
        .unwrap_or_else(|| panic!("no row for {label}"))
}

#[test]
fn rows_list_every_rebindable_action_once_required_first() {
    let mut listed: Vec<Action> = ROWS.iter().map(|r| r.0).collect();
    let required = listed.iter().take_while(|a| a.is_required()).count();
    assert_eq!(required, 7);
    assert!(listed[required..].iter().all(|a| !a.is_required()));
    listed.sort();
    let mut rebindable: Vec<Action> = Action::ALL
        .into_iter()
        .filter(|a| a.is_rebindable())
        .collect();
    rebindable.sort();
    assert_eq!(listed, rebindable);
    assert_eq!(label(Action::Info), "Unit info");
    assert_eq!(label(Action::Debug), "Debug");
    assert_eq!(
        blocked_message(Side::Keyboard, Cancel),
        "Give Cancel a key first"
    );
    assert_eq!(
        blocked_message(Side::Controller, Cancel),
        "Give Cancel a button first"
    );
}

#[test]
fn starts_on_the_first_slot_of_the_layout_in_use() {
    let c = ctx().with_layout(Layout::LeftHanded);
    let s = KeyBindingsScreen::new(&c);
    assert_eq!(s.name(), "key_bindings");
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert_eq!(s.bindings(), &c.layout_bindings(Layout::LeftHanded));
    assert!(!s.is_capturing());
    assert_eq!(s.message(), None);
    assert_eq!(s.side(), Side::Keyboard);
    let buf = drawn(&s, &c);
    assert!(row_text(&buf, 1).contains("─ Key bindings ─"));
    assert!(row_text(&buf, 2).contains("  Keyboard · Left-handed    Controller  "));
    // Before a layout is chosen (debug menu over the picker): right-handed.
    let none = Ctx::embedded().unwrap();
    let s = KeyBindingsScreen::new(&none);
    assert_eq!(s.bindings(), &none.layout_bindings(Layout::RightHanded));
}

#[test]
fn up_and_down_wrap_through_the_rows_and_restore_defaults() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorDown, 0)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    s.update(&mut c, &act(&[CursorUp, CursorUp]));
    assert_eq!(s.focus(), None);
    assert!(s.is_on_switch(), "up from the first row is the switch");
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), None, "and up again wraps to Restore");
    assert!(!s.is_on_switch());
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), Some((Action::Menu, 0)));
    s.update(&mut c, &act(&[CursorDown, CursorDown]));
    assert!(s.is_on_switch());
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
}

#[test]
fn left_and_right_move_between_the_slots_and_stop_at_the_ends() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert!(sounds(&mut c).is_empty(), "nowhere to go: no sound");
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    s.update(&mut c, &act(&[CursorRight, CursorRight, CursorRight]));
    assert_eq!(s.focus(), Some((CursorUp, SLOTS - 1)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
    // Restore defaults has no slots: Left/Right do nothing there, and the
    // slot is kept for the way back.
    s.update(&mut c, &act(&[CursorUp, CursorUp]));
    sounds(&mut c);
    s.update(&mut c, &act(&[CursorRight, CursorLeft]));
    assert!(sounds(&mut c).is_empty());
    s.update(&mut c, &act(&[CursorDown, CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
}

#[test]
fn confirm_captures_the_next_key_into_the_slot() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Confirm, 1);
    sounds(&mut c);
    // The key that confirms is not the key captured, even in one frame.
    let confirm = act(&[Confirm, CursorDown]).with_typing(vec![chord("f")], vec![]);
    s.update(&mut c, &confirm);
    assert!(s.is_capturing());
    assert_eq!(s.focus(), Some((Confirm, 1)), "later actions are dropped");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // A frame with no press keeps waiting.
    s.update(&mut c, &act(&[]));
    assert!(s.is_capturing());
    s.update(&mut c, &press("g"));
    assert!(!s.is_capturing());
    assert_eq!(
        s.bindings().slots(Confirm),
        [Some(chord("f")), Some(chord("g")), None]
    );
    assert_eq!(s.moved, None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // Shifted chords are their own keys.
    bind(&mut s, &mut c, Confirm, 2, "Shift+g");
    assert_eq!(s.bindings().slots(Confirm)[2], Some(chord("Shift+g")));
}

#[test]
fn actions_and_repeats_are_ignored_while_capturing() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[Confirm]));
    let t = s.update(&mut c, &act(&[CursorDown, Cancel, Confirm]));
    assert_eq!(format!("{t:?}"), "None");
    assert!(s.is_capturing());
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    // The pressed key's own action that frame is ignored too: `d` is bound,
    // not treated as Cancel.
    let d = act(&[Cancel]).with_typing(vec![chord("d")], vec![]);
    let t = s.update(&mut c, &d);
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("d")));
}

#[test]
fn a_cursor_key_captured_and_still_held_doesnt_move_the_focus() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Info, 1);
    s.update(&mut c, &act(&[Confirm]));
    // `Down` is pressed for the slot and kept down: it repeats as Cursor
    // down under the keys the screen was opened with.
    let held = |actions: &[Action]| FrameInput::new(actions.to_vec(), 0.0, vec![CursorDown]);
    s.update(
        &mut c,
        &held(&[CursorDown]).with_typing(vec![chord("Down")], vec![]),
    );
    assert_eq!(s.bindings().slots(Info)[1], Some(chord("Down")));
    sounds(&mut c);
    s.update(&mut c, &held(&[CursorDown, CursorDown, CursorRight]));
    assert_eq!(s.focus(), Some((Info, 1)));
    assert!(sounds(&mut c).is_empty());
    // Other actions still work meanwhile.
    s.update(&mut c, &held(&[Confirm]));
    assert!(s.is_capturing());
    s.update(
        &mut c,
        &held(&[]).with_typing(vec![chord("Escape")], vec![]),
    );
    s.update(&mut c, &held(&[CursorDown]));
    assert_eq!(s.focus(), Some((Info, 1)), "backing out waits for it too");
    // Once every cursor key is up, the next press moves again.
    s.update(&mut c, &act(&[]));
    s.update(&mut c, &held(&[CursorDown]));
    assert_eq!(s.focus(), Some((Action::DangerZone, 1)));
    // A key that isn't held never blocks: released in the capture frame.
    bind(&mut s, &mut c, Info, 2, "g");
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), Some((Action::NextUnit, 2)));
    // The ignored clear-slot key doesn't start the wait.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &press("Delete"));
    assert!(!s.await_release);
}

#[test]
fn the_abort_key_leaves_capture_with_nothing_changed() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    // Escape is also Cancel in every keymap: it must not close the screen.
    let escape = act(&[Cancel]).with_typing(vec![chord("Escape")], vec![]);
    let t = s.update(&mut c, &escape);
    assert_eq!(format!("{t:?}"), "None");
    assert!(!s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Keys pressed after it in the same frame aren't captured.
    s.update(&mut c, &act(&[Confirm]));
    let both = act(&[]).with_typing(vec![chord("Escape"), chord("g")], vec![]);
    s.update(&mut c, &both);
    assert_eq!(s.bindings(), &before);
    // Shift+Escape is an ordinary key.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &press("Shift+Escape"));
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("Shift+Escape")));
}

#[test]
fn the_clear_key_is_ignored_while_capturing() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    s.update(&mut c, &press("Delete"));
    assert!(s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(s.message(), None);
    assert!(sounds(&mut c).is_empty());
    // A key pressed after it in the same frame is still captured.
    let both = act(&[]).with_typing(vec![chord("Delete"), chord("g"), chord("h")], vec![]);
    s.update(&mut c, &both);
    assert!(!s.is_capturing());
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("g")));
    assert_eq!(s.bindings().find(chord("h")), None);
}

#[test]
fn a_reserved_key_is_refused_and_capture_goes_on() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    // The Debug key, in a build with debug tools.
    s.update(&mut c, &press("F2"));
    assert!(s.is_capturing());
    assert_eq!(s.message(), Some("That key can't be used"));
    assert_eq!(s.bindings(), &before);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Denied)]);
    let buf = drawn(&s, &c);
    assert!(row_text(&buf, MESSAGE_ROW).contains(RESERVED_MESSAGE));
    // The next key binds and clears the message.
    s.update(&mut c, &press("g"));
    assert!(!s.is_capturing());
    assert_eq!(s.message(), None);
    assert_eq!(s.bindings().slots(CursorUp)[0], Some(chord("g")));
    // Backing out clears it too.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &press("F2"));
    s.update(&mut c, &press("Escape"));
    assert_eq!(s.message(), None);
}

#[test]
fn a_key_used_elsewhere_moves_and_its_old_row_is_highlighted_briefly() {
    let mut c = ctx();
    let mut s = screen(&c);
    bind(&mut s, &mut c, Confirm, 1, "e");
    assert_eq!(
        s.bindings().slots(Confirm),
        [Some(chord("f")), Some(chord("e")), None]
    );
    assert!(s.bindings().is_unmapped(Info));
    assert_eq!(s.moved, Some((Info, MOVED_FLASH_SECS)));
    let label_fg = |s: &KeyBindingsScreen, c: &Ctx| {
        let buf = drawn(s, c);
        let y = (0..i32::from(CONSOLE_H))
            .find(|&y| row_text(&buf, y).contains(" Unit info  "))
            .unwrap();
        buf.get(LABEL_X, y).unwrap().fg
    };
    assert_eq!(label_fg(&s, &c), c.palette.get(UiColor::White));
    let buf = drawn(&s, &c);
    assert!(row_of(&buf, "Unit info").contains(NOT_MAPPED));
    assert!(!row_of(&buf, "Confirm").contains(NOT_MAPPED));
    // The highlight lasts MOVED_FLASH_SECS.
    s.update(
        &mut c,
        &FrameInput::new(vec![], MOVED_FLASH_SECS - 0.5, vec![]),
    );
    assert_eq!(label_fg(&s, &c), c.palette.get(UiColor::White));
    s.update(&mut c, &FrameInput::new(vec![], 0.5, vec![]));
    assert_eq!(s.moved, None);
    assert_eq!(label_fg(&s, &c), c.palette.get(UiColor::Text));
    // A NaN frame time ends it rather than leaving it on for good.
    bind(&mut s, &mut c, Confirm, 2, "w");
    assert!(s.moved.is_some());
    s.update(&mut c, &FrameInput::new(vec![], f32::NAN, vec![]));
    assert_eq!(s.moved, None);
    // Its own row, when focused, keeps the focus colour.
    bind(&mut s, &mut c, Info, 0, "s");
    focus_on(&mut s, &mut c, Action::NextUnit, 0);
    let buf = drawn(&s, &c);
    let y = (0..i32::from(CONSOLE_H))
        .find(|&y| row_text(&buf, y).contains(" Next ready unit  "))
        .unwrap();
    assert_eq!(
        buf.get(LABEL_X, y).unwrap().fg,
        c.palette.get(UiColor::TextHighlight)
    );
}

#[test]
fn rebinding_a_slot_to_its_own_key_changes_nothing() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    bind(&mut s, &mut c, Confirm, 0, "f");
    assert!(!s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(s.moved, None);
}

#[test]
fn the_clear_key_empties_the_focused_slot() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Info, 0);
    sounds(&mut c);
    s.update(&mut c, &press("Shift+Delete"));
    assert!(!s.bindings().is_unmapped(Info));
    s.update(&mut c, &press("Delete"));
    assert!(s.bindings().is_unmapped(Info));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // An empty slot: nothing to do, no sound.
    s.update(&mut c, &press("Delete"));
    assert!(sounds(&mut c).is_empty());
    // A required action's last key may be emptied too.
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    assert!(s.bindings().is_unmapped(Confirm));
    // On Restore defaults there is no slot.
    let before = s.bindings().clone();
    s.update(
        &mut c,
        &act(&[CursorUp, CursorUp, CursorUp, CursorUp, CursorUp]),
    );
    assert_eq!(s.focus(), None);
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.bindings(), &before);
}

#[test]
fn clearing_a_slot_clears_the_message() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    s.update(&mut c, &act(&[Cancel]));
    assert!(s.message().is_some());
    focus_on(&mut s, &mut c, Info, 0);
    s.update(&mut c, &act(&[Cancel]));
    assert!(s.message().is_some());
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.message(), None);
}

#[test]
fn leaving_saves_the_edit_and_pops() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c);
    bind(&mut s, &mut c, Confirm, 1, "g");
    sounds(&mut c);
    let before = c.keymap.clone();
    assert_eq!(c.keymap, before, "nothing applies until the screen closes");
    let t = s.update(&mut c, &act(&[Cancel, CursorDown]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    assert_eq!(c.keymap.action(chord("g")), Some(Confirm));
    assert_eq!(&c.layout_bindings(Layout::RightHanded), s.bindings());
    assert!(c.player_keys().is_custom(Layout::RightHanded));
    assert!(!c.player_keys().is_custom(Layout::LeftHanded));
    assert_eq!(
        c.storage.read(crate::screen::KEYBINDINGS_KEY).unwrap(),
        Some(c.player_keys().to_ron())
    );
}

#[test]
fn leaving_without_a_change_writes_nothing() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.storage.read(crate::screen::KEYBINDINGS_KEY), Ok(None));
}

#[test]
fn leaving_is_blocked_while_a_required_action_has_no_key() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c);
    // Cancel and Cursor down lose their keys; the message names the one
    // listed first on the screen.
    bind(&mut s, &mut c, Info, 1, "d");
    bind(&mut s, &mut c, Info, 2, "Down");
    sounds(&mut c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.message(), Some("Give Cursor down a key first"));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Denied)]);
    assert!(!c.player_keys().is_custom(Layout::RightHanded));
    let buf = drawn(&s, &c);
    let warn = c.palette.get(UiColor::HpLow);
    let message = row_text(&buf, MESSAGE_ROW);
    let x = i32::try_from(message.find("Give").unwrap()).unwrap();
    assert_eq!(buf.get(x, MESSAGE_ROW).unwrap().fg, warn);
    // Moving on clears the message; the next try names what is left.
    bind(&mut s, &mut c, CursorDown, 0, "j");
    assert_eq!(s.message(), None);
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.message(), Some("Give Cancel a key first"));
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.message(), None);
    // Escape doesn't count as Cancel's key, but still works as Cancel:
    // with a key of its own again, leaving works.
    bind(&mut s, &mut c, Cancel, 0, "k");
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.keymap.action(chord("k")), Some(Cancel));
    // An optional action with no key never blocks.
    let mut s = KeyBindingsScreen::new(&c);
    focus_on(&mut s, &mut c, Action::Rewind, 0);
    s.update(&mut c, &press("Delete"));
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(c.keymap.primary(Action::Rewind), None);
}

#[test]
fn restore_defaults_resets_only_this_layout() {
    let mut c = ctx();
    let mut left = c.layout_bindings(Layout::LeftHanded);
    left.bind(Info, 0, chord("g")).unwrap();
    c.set_layout_bindings(Layout::LeftHanded, left.clone())
        .unwrap();
    let mut s = KeyBindingsScreen::new(&c);
    bind(&mut s, &mut c, Confirm, 1, "e");
    assert!(s.moved.is_some());
    s.update(&mut c, &act(&[Cancel]));
    assert!(c.player_keys().is_custom(Layout::RightHanded));

    let mut s = KeyBindingsScreen::new(&c);
    focus_on_restore(&mut s, &mut c);
    s.moved = Some((Info, 1.0));
    sounds(&mut c);
    // It asks first; a second Confirm in the same frame isn't the answer.
    let custom = s.bindings().clone();
    s.update(&mut c, &act(&[Confirm, Confirm]));
    assert!(!s.is_capturing());
    assert!(s.is_asking_restore());
    assert_eq!(s.bindings(), &custom);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    s.update(&mut c, &act(&[Confirm, Cancel]));
    assert!(!s.is_asking_restore());
    assert_eq!(s.moved, None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    let defaults = LayoutBindings::defaults(&c.content.keymap, Layout::RightHanded);
    assert_eq!(s.bindings(), &defaults);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert!(!c.player_keys().is_custom(Layout::RightHanded));
    assert_eq!(c.layout_bindings(Layout::LeftHanded), left);
}

#[test]
fn the_restore_question_can_be_backed_out_of() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c);
    bind(&mut s, &mut c, Confirm, 1, "e");
    let custom = s.bindings().clone();
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(
        s.restore_question(),
        "Restore the default keys for Right-handed?"
    );
    assert_eq!(s.help(&c), "f yes · d no");
    sounds(&mut c);
    // Nothing but Confirm and Cancel answers; the clear-slot key and the
    // cursor do nothing.
    let other = act(&[CursorUp, Info, CursorLeft]).with_typing(vec![chord("Delete")], vec![]);
    s.update(&mut c, &other);
    assert!(s.is_asking_restore());
    assert_eq!(s.focus(), None);
    assert!(sounds(&mut c).is_empty());
    let buf = drawn(&s, &c);
    let asked = (0..i32::from(CONSOLE_H))
        .map(|y| row_text(&buf, y))
        .collect::<Vec<_>>();
    assert!(
        asked
            .iter()
            .any(|r| r.contains("║ Restore the default keys for Right-handed? ║"))
    );
    assert!(asked.iter().any(|r| r.contains("║ f yes / d no ")));
    // Cancel closes the question, not the screen, and changes nothing;
    // what follows it that frame is dropped.
    let t = s.update(&mut c, &act(&[Cancel, Confirm, Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert!(!s.is_asking_restore());
    assert_eq!(s.bindings(), &custom);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    assert_eq!(s.help(&c), "arrows move · f restore · d back");
    let buf = drawn(&s, &c);
    assert!(!(0..i32::from(CONSOLE_H)).any(|y| row_text(&buf, y).contains('║')));
    // The left-handed question names its layout and keys.
    let left = ctx().with_layout(Layout::LeftHanded);
    let mut s = KeyBindingsScreen::new(&left);
    s.asking_restore = true;
    assert_eq!(
        s.restore_question(),
        "Restore the default keys for Left-handed?"
    );
    assert_eq!(s.help(&c), "j yes · k no");
}

#[test]
fn confirm_clears_the_message() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c);
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    s.update(&mut c, &act(&[Cancel]));
    assert!(s.message().is_some());
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.message(), None);
}

#[test]
fn other_actions_do_nothing() {
    let mut c = ctx();
    let mut s = screen(&c);
    let before = s.bindings().clone();
    let t = s.update(
        &mut c,
        &act(&[Info, Action::EndTurn, Action::Debug, Action::Menu]),
    );
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert_eq!(s.bindings(), &before);
    assert!(sounds(&mut c).is_empty());
    // An unbound key pressed while not capturing does nothing either.
    s.update(&mut c, &press("g"));
    assert_eq!(s.bindings(), &before);
}

#[test]
fn help_names_the_keys_the_screen_was_opened_with() {
    let mut c = ctx();
    let mut s = screen(&c);
    assert_eq!(s.help(&c), "arrows move · f bind · Delete clear · d back");
    // Edits don't change the help: the old keys still steer the screen.
    bind(&mut s, &mut c, Info, 0, "f");
    bind(&mut s, &mut c, Info, 1, "d");
    assert_eq!(s.help(&c), "arrows move · f bind · Delete clear · d back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "Press the key to put here · Escape back");
    s.update(&mut c, &press("Escape"));
    s.update(&mut c, &act(&[CursorUp, CursorUp, CursorUp, CursorUp]));
    focus_on_restore(&mut s, &mut c);
    assert_eq!(s.help(&c), "arrows move · f restore · d back");
    let left = ctx().with_layout(Layout::LeftHanded);
    assert_eq!(
        KeyBindingsScreen::new(&left).help(&left),
        "wasd move · j bind · Delete clear · k back"
    );
}

#[test]
fn draws_slots_fixed_keys_and_not_mapped_notes() {
    let mut c = ctx();
    let mut s = screen(&c);
    let buf = drawn(&s, &c);
    let stale = c.palette.get(UiColor::Enemy);
    let left = (0..i32::from(CONSOLE_H))
        .flat_map(|y| (0..i32::from(CONSOLE_W)).map(move |x| (x, y)))
        .filter(|&(x, y)| buf.get(x, y).is_some_and(|c| c.bg == stale))
        .count();
    assert_eq!(left, 0, "an opaque screen covers the whole buffer");
    assert!(row_text(&buf, 4).contains("Key 1"));
    assert!(row_text(&buf, 4).contains("Key 3"));
    assert!(row_text(&buf, 5).contains(REQUIRED_HEADING));
    assert!(row_of(&buf, "Cancel").contains(" Cancel  + Escape "));
    assert!(row_of(&buf, "End turn").contains(" Space "));
    assert!(row_of(&buf, "Auto-end on/off").contains(" Shift+Space "));
    // Optional and unmapped: dim. Required and unmapped: the warning colour.
    let note_fg = |buf: &GlyphBuffer, label: &str| {
        let y = (0..i32::from(CONSOLE_H))
            .find(|&y| row_text(buf, y).contains(&format!(" {label}  ")))
            .unwrap();
        let x = i32::try_from(row_text(buf, y).find(NOT_MAPPED).unwrap()).unwrap();
        buf.get(x, y).unwrap().fg
    };
    assert_eq!(note_fg(&buf, "Select"), c.palette.get(UiColor::TextDim));
    focus_on(&mut s, &mut c, Confirm, 0);
    s.update(&mut c, &press("Delete"));
    let buf = drawn(&s, &c);
    assert_eq!(note_fg(&buf, "Confirm"), c.palette.get(UiColor::HpLow));
    // The focused slot is a bar; while capturing it shows the prompt.
    let bar = c.palette.get(UiColor::PanelBorderFocus);
    let y = (0..i32::from(CONSOLE_H))
        .find(|&y| row_text(&buf, y).contains(" Confirm  "))
        .unwrap();
    assert_eq!(buf.get(slot_x(0), y).unwrap().bg, bar);
    assert_eq!(buf.get(slot_x(0) + 15, y).unwrap().bg, bar);
    assert_ne!(buf.get(slot_x(0) + 16, y).unwrap().bg, bar);
    assert_ne!(buf.get(slot_x(1), y).unwrap().bg, bar);
    s.update(&mut c, &act(&[CursorRight, Confirm]));
    let buf = drawn(&s, &c);
    assert!(row_text(&buf, y).contains(CAPTURE_PROMPT));
    assert_eq!(buf.get(slot_x(1), y).unwrap().bg, bar);
    assert_ne!(buf.get(slot_x(0), y).unwrap().bg, bar);
}

#[test]
fn every_chord_name_fits_a_slot() {
    for &key in Key::ALL {
        let name = Chord::shifted(key).to_string();
        assert!(name.chars().count() <= SLOT_W, "{name}");
    }
    assert!(slot_x(SLOTS) + 1 + i32::try_from(NOT_MAPPED.len()).unwrap() < PANEL.x + PANEL.w);
    assert_eq!(slot_x(0), SLOTS_X);
    assert_eq!(slot_x(2), SLOTS_X + 2 * SLOT_PITCH);
}

// --- The controller side (ticket 0816) --------------------------------------

const XBOX: Device = Device::Pad(PadKind::Xbox);

/// A context whose player pressed a controller button last.
fn pad_ctx() -> Ctx {
    let mut c = ctx();
    c.device = XBOX;
    c
}

/// A frame of `dt` seconds where the buttons `down` went down and `up` went
/// up (and did nothing as actions).
fn buttons(down: &[Button], up: &[Button], dt: f32) -> FrameInput {
    FrameInput::new(vec![], dt, vec![]).with_buttons(down.to_vec(), up.to_vec())
}

/// Presses and lets go of `button` in two frames.
fn tap(s: &mut KeyBindingsScreen, c: &mut Ctx, button: Button) {
    s.update(c, &buttons(&[button], &[], 0.0));
    s.update(c, &buttons(&[], &[button], 0.0));
}

/// Focuses `action`'s slot `slot` and starts capturing through the choices.
fn change(s: &mut KeyBindingsScreen, c: &mut Ctx, action: Action, slot: usize) {
    focus_on(s, c, action, slot);
    s.update(c, &act(&[Confirm]));
    assert_eq!(s.choice(), Some(0));
    s.update(c, &act(&[Confirm]));
    assert!(s.is_capturing());
}

fn pad_names(s: &KeyBindingsScreen, action: Action) -> [&'static str; SLOTS] {
    s.pad_bindings()
        .slots(action)
        .map(|b| b.map_or("-", Button::name))
}

#[test]
fn opens_on_the_side_pressed_last() {
    let c = ctx();
    assert_eq!(KeyBindingsScreen::new(&c).side(), Side::Keyboard);
    let c = pad_ctx();
    let s = KeyBindingsScreen::new(&c);
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    assert_eq!(s.choice(), None);
    assert!(!s.is_on_switch());
}

#[test]
fn the_switch_row_shows_the_keyboard_or_the_controller() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp]));
    assert!(s.is_on_switch());
    s.message = Some("old".to_owned());
    s.moved = Some((Info, 1.0));
    sounds(&mut c);
    // Left on the keyboard side: already there.
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.side(), Side::Keyboard);
    assert!(sounds(&mut c).is_empty());
    assert_eq!(s.message(), Some("old"));
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Move)]);
    assert_eq!((s.message(), s.moved), (None, None));
    s.update(&mut c, &act(&[CursorRight]));
    assert_eq!(s.side(), Side::Controller);
    assert!(sounds(&mut c).is_empty());
    s.update(&mut c, &act(&[CursorLeft]));
    assert_eq!(s.side(), Side::Keyboard);
    // Confirm flips it, either way, and opens nothing.
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(sounds(&mut c), vec![cue(MenuSound::Move); 2]);
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.side(), Side::Keyboard);
    assert!(!s.is_capturing() && !s.is_asking_restore() && s.choice().is_none());
    // The slot is kept for the way back down.
    assert_eq!(s.slot, 0);
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 0)));
    // On the controller too, where Confirm doesn't open the choices.
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    s.update(&mut c, &act(&[CursorUp, Confirm]));
    assert_eq!((s.side(), s.choice()), (Side::Keyboard, None));
    // The clear-slot key does nothing there.
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.bindings(), &c.layout_bindings(Layout::RightHanded));
}

#[test]
fn the_controller_side_draws_buttons_as_the_pad_names_them() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    let buf = drawn(&s, &c);
    assert!(row_text(&buf, 4).contains("Button 1"));
    assert!(row_text(&buf, 4).contains("Button 3"));
    assert!(!row_text(&buf, 4).contains("Key 1"));
    assert!(row_text(&buf, 5).contains(REQUIRED_BUTTON_HEADING));
    assert!(row_text(&buf, 14).contains(OPTIONAL_HEADING));
    assert!(row_of(&buf, "Cursor up").contains(" ↑                 L-stick ↑         · "));
    assert!(row_of(&buf, "Confirm").contains(" A  "));
    assert!(!row_of(&buf, "Cancel").contains('+'), "no fixed button");
    assert!(row_of(&buf, "End turn").contains(" Start "));
    assert!(row_of(&buf, "Select").contains(NOT_MAPPED));
    assert!(!row_of(&buf, "Rewind").contains(NOT_MAPPED));
    // The shown side is highlighted in the switch row, the other dim.
    // The column `word` starts at in the switch row.
    let col = |buf: &GlyphBuffer, word: &str| {
        let row = row_text(buf, 2);
        let cells = row[..row.find(word).unwrap()].chars().count();
        i32::try_from(cells).unwrap()
    };
    let fg = |buf: &GlyphBuffer, word: &str| buf.get(col(buf, word), 2).unwrap().fg;
    assert_eq!(fg(&buf, CONTROLLER_SIDE), c.palette.get(UiColor::TextHighlight));
    assert_eq!(fg(&buf, KEYBOARD_SIDE), c.palette.get(UiColor::TextDim));
    // A Sony pad's names; on the keyboard, an Xbox pad's.
    c.device = Device::Pad(PadKind::PlayStation);
    assert!(row_of(&drawn(&s, &c), "End turn").contains(" Options "));
    c.device = Device::Keyboard;
    assert!(row_of(&drawn(&s, &c), "End turn").contains(" Start "));
    // On the switch, the shown side is a bar.
    s.update(&mut c, &act(&[CursorUp]));
    let buf = drawn(&s, &c);
    let bar = c.palette.get(UiColor::PanelBorderFocus);
    let x = col(&buf, CONTROLLER_SIDE);
    assert_eq!(buf.get(x - 1, 2).unwrap().bg, bar);
    assert_eq!(buf.get(x + 10, 2).unwrap().bg, bar);
    assert_ne!(buf.get(x + 11, 2).unwrap().bg, bar);
    assert_ne!(buf.get(x - 2, 2).unwrap().bg, bar);
    s.update(&mut c, &act(&[CursorLeft]));
    let buf = drawn(&s, &c);
    assert_eq!(buf.get(LABEL_X - 1, 2).unwrap().bg, bar);
    assert_ne!(buf.get(x, 2).unwrap().bg, bar);
    assert!(row_text(&buf, 4).contains("Key 1"));
    assert!(row_of(&buf, "Cancel").contains(" Cancel  + Escape "));
}

#[test]
fn with_the_keyboard_confirm_captures_a_button_let_go() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp, CursorRight]));
    focus_on(&mut s, &mut c, Info, 1);
    sounds(&mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_capturing());
    assert_eq!(s.choice(), None);
    let y = row_y(11);
    assert!(row_text(&drawn(&s, &c), y).contains(CAPTURE_BUTTON_PROMPT));
    sounds(&mut c);
    // Going down doesn't bind; nor does a key.
    s.update(&mut c, &buttons(&[Button::RightTrigger], &[], 0.0));
    s.update(&mut c, &press("g"));
    assert!(s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "-", "-"]);
    assert_eq!(s.bindings(), &screen(&c).bindings);
    assert!(sounds(&mut c).is_empty());
    // Letting go does.
    s.update(&mut c, &buttons(&[], &[Button::RightTrigger], 0.0));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "RightTrigger", "-"]);
    assert_eq!((s.moved, s.held), (None, None));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // The keys are untouched.
    assert_eq!(s.bindings(), &screen(&c).bindings);
}

#[test]
fn a_button_taken_from_another_action_moves_and_that_row_lights_up() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 0);
    s.message = Some("old".to_owned());
    tap(&mut s, &mut c, Button::South);
    assert_eq!(pad_names(&s, Info), ["South", "-", "-"]);
    assert_eq!(pad_names(&s, Confirm), ["-", "-", "-"]);
    assert_eq!(s.moved, Some((Confirm, MOVED_FLASH_SECS)));
    assert_eq!(s.message(), None);
    let buf = drawn(&s, &c);
    assert!(row_of(&buf, "Confirm").contains(NOT_MAPPED));
    // Its own button again: nothing moves.
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::South);
    assert_eq!(s.moved, None);
    assert!(!s.is_capturing());
}

#[test]
fn a_button_pressed_and_let_go_in_one_frame_binds() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 2);
    s.update(&mut c, &buttons(&[Button::West], &[Button::West], 5.0));
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["North", "-", "West"]);
}

#[test]
fn holding_a_button_backs_out_with_nothing_changed() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 0);
    sounds(&mut c);
    // The frame it goes down in doesn't count, however long.
    s.update(&mut c, &buttons(&[Button::West], &[], 5.0));
    assert_eq!(s.held, Some((Button::West, 0.0)));
    s.update(&mut c, &buttons(&[], &[], 0.5));
    s.update(&mut c, &buttons(&[], &[], 0.25));
    assert!(s.is_capturing(), "not held long enough yet");
    assert_eq!(s.held, Some((Button::West, 0.75)));
    assert!(sounds(&mut c).is_empty());
    s.update(&mut c, &buttons(&[], &[], 0.25));
    assert!(!s.is_capturing(), "held for exactly the time");
    assert_eq!(s.held, None);
    assert!(!s.await_release, "no cursor button is held");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Letting go afterwards does nothing.
    s.update(&mut c, &buttons(&[], &[Button::West], 0.0));
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    assert_eq!(s.focus(), Some((Info, 0)));
    assert!(sounds(&mut c).is_empty());
    assert_eq!(HOLD_TO_CANCEL_SECS.to_bits(), 1.0_f32.to_bits());

    // Let go just before the time: it binds.
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], 0.75));
    s.update(&mut c, &buttons(&[], &[Button::West], 0.5));
    assert_eq!(pad_names(&s, Info), ["West", "-", "-"]);
}

#[test]
fn a_bad_frame_time_neither_backs_out_nor_breaks_the_hold() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], f32::NAN));
    s.update(&mut c, &buttons(&[], &[], -3.0));
    assert_eq!(s.held, Some((Button::West, 0.0)));
    s.update(&mut c, &buttons(&[], &[], 1.5));
    assert!(!s.is_capturing());
}

#[test]
fn only_the_first_button_down_is_watched() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 1);
    // Two go down in one frame, then a third: the first is the one.
    s.update(&mut c, &buttons(&[Button::West, Button::Start], &[], 0.0));
    s.update(&mut c, &buttons(&[Button::RightTrigger], &[], 0.25));
    // The others let go: still waiting.
    let others = [Button::Start, Button::RightTrigger];
    s.update(&mut c, &buttons(&[], &others, 0.25));
    assert!(s.is_capturing());
    assert_eq!(s.held, Some((Button::West, 0.5)));
    s.update(&mut c, &buttons(&[], &[Button::West], 0.0));
    assert_eq!(pad_names(&s, Info), ["North", "West", "-"]);
}

#[test]
fn a_button_held_since_before_the_capture_is_ignored() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    // Confirm's button went down to pick Change, and is still down.
    change(&mut s, &mut c, Info, 1);
    s.update(&mut c, &buttons(&[], &[], 5.0));
    assert!(s.is_capturing(), "a hold from before doesn't back out");
    s.update(&mut c, &buttons(&[], &[Button::South], 0.0));
    assert!(s.is_capturing(), "letting go of it doesn't bind it");
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
}

#[test]
fn on_a_controller_confirm_offers_change_or_clear() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    focus_on(&mut s, &mut c, Info, 0);
    s.message = Some("old".to_owned());
    sounds(&mut c);
    // A second Confirm in the same frame isn't the answer.
    s.update(&mut c, &act(&[Confirm, Confirm, CursorDown]));
    assert_eq!(s.choice(), Some(0));
    assert!(!s.is_capturing());
    assert_eq!(s.message(), None);
    assert_eq!(s.focus(), Some((Info, 0)));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // Up and down move between the two; left, right and the rest don't.
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.choice(), Some(1));
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.choice(), Some(0));
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.choice(), Some(1));
    assert_eq!(sounds(&mut c), vec![cue(MenuSound::Move); 3]);
    s.update(&mut c, &act(&[CursorLeft, CursorRight, Info, Action::EndTurn]));
    s.update(&mut c, &press("Delete"));
    assert_eq!(s.choice(), Some(1));
    assert_eq!(s.focus(), Some((Info, 0)));
    assert!(sounds(&mut c).is_empty());
    // A button going down while they are open isn't captured.
    tap(&mut s, &mut c, Button::West);
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    // Cancel closes them and stays on the screen.
    let t = s.update(&mut c, &act(&[Cancel, Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    assert_eq!(s.choice(), None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Clear empties the slot.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm, Confirm]));
    assert_eq!(s.choice(), None);
    assert!(!s.is_capturing());
    assert_eq!(pad_names(&s, Info), ["-", "-", "-"]);
    sounds(&mut c);
    // Clear on an empty slot closes them too, with one sound.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm]));
    assert_eq!(s.choice(), None);
    assert_eq!(
        sounds(&mut c),
        [
            cue(MenuSound::Select),
            cue(MenuSound::Move),
            cue(MenuSound::Select)
        ]
    );
    // Change waits for a button.
    s.update(&mut c, &act(&[Confirm]));
    sounds(&mut c);
    s.update(&mut c, &act(&[Confirm, CursorDown]));
    assert!(s.is_capturing());
    assert_eq!(s.choice(), None);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    // On Restore defaults Confirm still asks; no choices there.
    tap(&mut s, &mut c, Button::West);
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(s.choice(), None);
}

#[test]
fn the_choices_are_drawn_under_the_slot_or_over_it_on_the_last_rows() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    focus_on(&mut s, &mut c, Info, 1);
    s.update(&mut c, &act(&[Confirm]));
    let buf = drawn(&s, &c);
    let (x, y) = (slot_x(1), row_y(11));
    assert_eq!(buf.get(x, y + 1).unwrap().glyph, '╔');
    assert_eq!(buf.get(x + CHOICE_W - 1, y + CHOICE_H).unwrap().glyph, '╝');
    assert!(row_text(&buf, y + 2).contains("║ Change   ║"));
    assert!(row_text(&buf, y + 3).contains("║ Clear    ║"));
    let bar = c.palette.get(UiColor::PanelBorderFocus);
    assert_eq!(buf.get(x + 1, y + 2).unwrap().bg, bar);
    assert_eq!(buf.get(x + CHOICE_W - 2, y + 2).unwrap().bg, bar);
    assert_ne!(buf.get(x + 1, y + 3).unwrap().bg, bar);
    s.update(&mut c, &act(&[CursorDown]));
    let buf = drawn(&s, &c);
    assert_eq!(buf.get(x + 1, y + 3).unwrap().bg, bar);
    assert_ne!(buf.get(x + 1, y + 2).unwrap().bg, bar);
    assert!(row_text(&buf, y + 2).contains("║ Change   ║"));
    // Only under the last row it doesn't fit.
    for (row, over) in [(13, false), (14, false), (15, true)] {
        let mut s = KeyBindingsScreen::new(&c);
        focus_on(&mut s, &mut c, ROWS[row].0, 0);
        s.update(&mut c, &act(&[Confirm]));
        let buf = drawn(&s, &c);
        let y = row_y(row);
        let top = if over { y - CHOICE_H } else { y + 1 };
        assert_eq!(buf.get(slot_x(0), top).unwrap().glyph, '╔', "row {row}");
        let bottom = top + CHOICE_H - 1;
        assert_eq!(buf.get(slot_x(0), bottom).unwrap().glyph, '╚', "row {row}");
        assert!(bottom < PANEL.y + PANEL.h - 1, "inside the panel");
        // The slot itself stays visible.
        assert_eq!(buf.get(slot_x(0), y).unwrap().bg, bar);
    }
}

#[test]
fn on_a_controller_the_keyboard_side_still_takes_only_keys() {
    let mut c = pad_ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp, CursorLeft]));
    assert_eq!(s.side(), Side::Keyboard);
    change(&mut s, &mut c, Info, 1);
    assert!(row_text(&drawn(&s, &c), row_y(11)).contains(CAPTURE_PROMPT));
    // A button let go isn't a key: still waiting.
    tap(&mut s, &mut c, Button::West);
    assert!(s.is_capturing());
    assert_eq!(s.held, None);
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
    // Holding one backs out.
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &buttons(&[], &[], 1.0));
    assert!(!s.is_capturing());
    // And a key goes in.
    change(&mut s, &mut c, Info, 1);
    s.update(&mut c, &press("g"));
    assert_eq!(s.bindings().slots(Info)[1], Some(chord("g")));
    // Clear empties the key slot, not a button.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm]));
    assert_eq!(s.bindings().slots(Info)[1], None);
    assert_eq!(s.pad_bindings(), &c.pad_bindings());
}

#[test]
fn on_the_controller_side_the_keyboards_fixed_keys_still_work() {
    let mut c = ctx();
    let mut s = screen(&c);
    s.update(&mut c, &act(&[CursorUp, CursorRight]));
    focus_on(&mut s, &mut c, Info, 0);
    // The clear-slot key empties the button slot.
    sounds(&mut c);
    s.update(&mut c, &press("Delete"));
    assert_eq!(pad_names(&s, Info), ["-", "-", "-"]);
    assert_eq!(s.bindings(), &screen(&c).bindings);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Select)]);
    s.update(&mut c, &press("Delete"));
    assert!(sounds(&mut c).is_empty(), "already empty");
    // The back-out key ends a capture, even with a button held; the
    // clear-slot key does nothing during one.
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &buttons(&[Button::West], &[], 0.0));
    s.update(&mut c, &press("Delete"));
    assert!(s.is_capturing());
    let escape = press("Escape").with_buttons(vec![], vec![Button::West]);
    s.update(&mut c, &escape);
    assert!(!s.is_capturing());
    assert_eq!(s.held, None);
    assert_eq!(pad_names(&s, Info), ["-", "-", "-"]);
}

#[test]
fn cursor_buttons_held_after_a_capture_dont_move_the_focus() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 0);
    s.update(&mut c, &buttons(&[Button::DpadDown], &[], 0.0));
    s.update(&mut c, &FrameInput::new(vec![], 1.0, vec![CursorDown]));
    assert!(!s.is_capturing());
    assert!(s.await_release);
    // Still held: its repeats are ignored until it is let go.
    let held = FrameInput::new(vec![CursorDown], 0.1, vec![CursorDown]);
    s.update(&mut c, &held);
    assert_eq!(s.focus(), Some((Info, 0)));
    s.update(&mut c, &act(&[]));
    s.update(&mut c, &held);
    assert_eq!(s.focus(), Some((Action::DangerZone, 0)));
    // A button tapped into a slot is up again: the next press moves.
    change(&mut s, &mut c, Action::DangerZone, 0);
    tap(&mut s, &mut c, Button::RightTrigger);
    s.update(&mut c, &held);
    assert_eq!(s.focus(), Some((Action::ToggleAutoEnd, 0)));
}

#[test]
fn leaving_is_blocked_while_a_required_action_has_no_button() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::South);
    change(&mut s, &mut c, Info, 1);
    tap(&mut s, &mut c, Button::Start);
    sounds(&mut c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "None");
    // The one listed higher on the screen is named.
    assert_eq!(s.message(), Some("Give Confirm a button first"));
    assert_eq!(sounds(&mut c), [cue(MenuSound::Denied)]);
    assert_eq!(c.pad_bindings(), PadBindings::defaults(&c.content.keymap));
    // From the keyboard side too: the screen shows the side that lacks one.
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.message(), None);
    s.side = Side::Keyboard;
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.side(), Side::Controller);
    assert_eq!(s.message(), Some("Give Confirm a button first"));
    // When both sides lack one, the shown side's is named and it stays.
    s.bindings.clear(Action::EndTurn, 0);
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.side(), Side::Controller);
    s.side = Side::Keyboard;
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(s.side(), Side::Keyboard);
    assert_eq!(s.message(), Some("Give End turn a key first"));
    // An optional action may be left with no button.
    let mut s = KeyBindingsScreen::new(&c);
    focus_on(&mut s, &mut c, Info, 0);
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[CursorDown, Confirm]));
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
}

#[test]
fn leaving_saves_the_buttons_for_both_layouts_and_leaves_the_keys() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    change(&mut s, &mut c, Info, 1);
    tap(&mut s, &mut c, Button::RightTrigger);
    // Nothing changes in the game until the screen closes.
    assert_eq!(c.keymap.pad_action(Button::RightTrigger), None);
    assert_eq!(c.storage.read(crate::screen::KEYBINDINGS_KEY), Ok(None));
    sounds(&mut c);
    let t = s.update(&mut c, &act(&[Cancel]));
    assert_eq!(format!("{t:?}"), "Pop");
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    assert_eq!(&c.pad_bindings(), s.pad_bindings());
    assert_eq!(c.keymap.pad_action(Button::RightTrigger), Some(Info));
    assert!(c.player_keys().is_custom_pad());
    for layout in Layout::ALL {
        assert!(!c.player_keys().is_custom(layout));
        let km = c.keymap_for(layout);
        assert_eq!(km.pad_action(Button::RightTrigger), Some(Info), "{layout}");
    }
    let saved = c.storage.read(crate::screen::KEYBINDINGS_KEY);
    assert_eq!(saved, Ok(Some(c.player_keys().to_ron())));
    // Leaving with nothing changed writes nothing.
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    s.update(&mut c, &act(&[Cancel]));
    assert_eq!(c.storage.read(crate::screen::KEYBINDINGS_KEY), Ok(None));
}

#[test]
fn restore_defaults_on_the_controller_side_resets_only_the_buttons() {
    let mut c = pad_ctx();
    let mut s = screen(&c);
    // A key change on the other side stays.
    s.bindings.bind(Info, 1, chord("g")).unwrap();
    let keys = s.bindings().clone();
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::South);
    assert!(s.moved.is_some());
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(s.restore_question(), "Restore the default buttons?");
    assert!(row_text(&drawn(&s, &c), 15).contains("Restore the default buttons?"));
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.pad_bindings(), &PadBindings::defaults(&c.content.keymap));
    assert_eq!(s.bindings(), &keys);
    assert_eq!(s.moved, None);
    // And on the keyboard side, only the keys.
    change(&mut s, &mut c, Info, 0);
    tap(&mut s, &mut c, Button::West);
    let pad = s.pad_bindings().clone();
    s.side = Side::Keyboard;
    assert_eq!(
        s.restore_question(),
        "Restore the default keys for Right-handed?"
    );
    focus_on_restore(&mut s, &mut c);
    s.update(&mut c, &act(&[Confirm]));
    s.update(&mut c, &act(&[Confirm]));
    assert_ne!(s.bindings(), &keys);
    assert_eq!(s.pad_bindings(), &pad);
}

#[test]
fn help_follows_the_side_and_the_device() {
    let mut c = pad_ctx();
    let mut s = KeyBindingsScreen::new(&c);
    assert_eq!(s.help(&c), "D-pad/L-stick move · A change · B back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "D-pad/L-stick move · A select · B back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "Press a button… · hold any button to cancel");
    // The keyboard used last: its back-out key is named too.
    c.device = Device::Keyboard;
    assert_eq!(
        s.help(&c),
        "Press a button… · hold any button to cancel · Escape back"
    );
    s.update(&mut c, &press("Escape"));
    assert_eq!(s.help(&c), "arrows move · f bind · Delete clear · d back");
    // A key slot waiting, on a controller and on the keyboard.
    s.side = Side::Keyboard;
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "Press the key to put here · Escape back");
    c.device = XBOX;
    assert_eq!(
        s.help(&c),
        "Press the key to put here · hold any button to cancel"
    );
    s.update(&mut c, &press("Escape"));
    // The switch and Restore defaults.
    s.update(&mut c, &act(&[CursorUp]));
    assert!(s.is_on_switch());
    assert_eq!(
        s.help(&c),
        "←/→ keyboard or controller · D-pad/L-stick move · B back"
    );
    c.device = Device::Keyboard;
    assert_eq!(
        s.help(&c),
        "Left/Right keyboard or controller · arrows move · d back"
    );
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.help(&c), "arrows move · f restore · d back");
    c.device = XBOX;
    assert_eq!(s.help(&c), "D-pad/L-stick move · A restore · B back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(&c), "A yes · B no");
    // Every line fits the console on every pad.
    for kind in [PadKind::PlayStation, PadKind::Nintendo, PadKind::Generic] {
        c.device = Device::Pad(kind);
        let mut s = KeyBindingsScreen::new(&c);
        let mut lines = vec![s.help(&c)];
        s.update(&mut c, &act(&[CursorUp]));
        lines.push(s.help(&c));
        for line in lines {
            assert!(line.chars().count() <= usize::from(CONSOLE_W), "{line}");
        }
    }
}

#[test]
fn every_button_name_fits_a_slot() {
    let kinds = [
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::PlayStation4,
        PadKind::Nintendo,
        PadKind::Generic,
    ];
    for kind in kinds {
        for &button in Button::ALL {
            let name = kind.button_name(button);
            assert!(name.chars().count() <= SLOT_W, "{name}");
        }
    }
    assert!(CAPTURE_BUTTON_PROMPT.chars().count() <= SLOT_W);
    for choice in CHOICES {
        assert!(i32::try_from(choice.chars().count()).unwrap() <= CHOICE_W - 4);
    }
}
