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
    act(&[]).with_pressed_chords(vec![chord(c)])
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
    assert_eq!(blocked_message(Cancel), "Give Cancel a key first");
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
    assert!(row_text(&drawn(&s, &c), 1).contains(" Key bindings · Left-handed "));
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
    assert_eq!(s.focus(), None, "up from the first row wraps to Restore");
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), Some((Action::Menu, 0)));
    s.update(&mut c, &act(&[CursorDown, CursorDown]));
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
    s.update(&mut c, &act(&[CursorUp]));
    sounds(&mut c);
    s.update(&mut c, &act(&[CursorRight, CursorLeft]));
    assert!(sounds(&mut c).is_empty());
    s.update(&mut c, &act(&[CursorDown]));
    assert_eq!(s.focus(), Some((CursorUp, 1)));
}

#[test]
fn confirm_captures_the_next_key_into_the_slot() {
    let mut c = ctx();
    let mut s = screen(&c);
    focus_on(&mut s, &mut c, Confirm, 1);
    sounds(&mut c);
    // The key that confirms is not the key captured, even in one frame.
    let confirm = act(&[Confirm, CursorDown]).with_pressed_chords(vec![chord("f")]);
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
    let d = act(&[Cancel]).with_pressed_chords(vec![chord("d")]);
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
        &held(&[CursorDown]).with_pressed_chords(vec![chord("Down")]),
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
        &held(&[]).with_pressed_chords(vec![chord("Escape")]),
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
    let escape = act(&[Cancel]).with_pressed_chords(vec![chord("Escape")]);
    let t = s.update(&mut c, &escape);
    assert_eq!(format!("{t:?}"), "None");
    assert!(!s.is_capturing());
    assert_eq!(s.bindings(), &before);
    assert_eq!(sounds(&mut c), [cue(MenuSound::Cancel)]);
    // Keys pressed after it in the same frame aren't captured.
    s.update(&mut c, &act(&[Confirm]));
    let both = act(&[]).with_pressed_chords(vec![chord("Escape"), chord("g")]);
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
    let both = act(&[]).with_pressed_chords(vec![chord("Delete"), chord("g"), chord("h")]);
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
    s.update(&mut c, &act(&[CursorUp]));
    assert_eq!(s.focus(), None);
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
    while s.focus().is_some() {
        s.update(&mut c, &act(&[CursorDown]));
    }
    s.update(&mut c, &act(&[Confirm]));
    assert!(s.is_asking_restore());
    assert_eq!(
        s.restore_question(),
        "Restore the default keys for Right-handed?"
    );
    assert_eq!(s.help(), "f yes · d no");
    sounds(&mut c);
    // Nothing but Confirm and Cancel answers; the clear-slot key and the
    // cursor do nothing.
    let other = act(&[CursorUp, Info, CursorLeft]).with_pressed_chords(vec![chord("Delete")]);
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
    assert_eq!(s.help(), "arrows move · f restore · d back");
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
    assert_eq!(s.help(), "j yes · k no");
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
    assert_eq!(s.help(), "arrows move · f bind · Delete clear · d back");
    // Edits don't change the help: the old keys still steer the screen.
    bind(&mut s, &mut c, Info, 0, "f");
    bind(&mut s, &mut c, Info, 1, "d");
    assert_eq!(s.help(), "arrows move · f bind · Delete clear · d back");
    s.update(&mut c, &act(&[Confirm]));
    assert_eq!(s.help(), "Press the key to put here · Escape back");
    s.update(&mut c, &press("Escape"));
    s.update(&mut c, &act(&[CursorUp, CursorUp, CursorUp, CursorUp]));
    while s.focus().is_some() {
        s.update(&mut c, &act(&[CursorDown]));
    }
    assert_eq!(s.help(), "arrows move · f restore · d back");
    let left = ctx().with_layout(Layout::LeftHanded);
    assert_eq!(
        KeyBindingsScreen::new(&left).help(),
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
    assert!(row_text(&buf, 3).contains("Key 1"));
    assert!(row_text(&buf, 3).contains("Key 3"));
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
