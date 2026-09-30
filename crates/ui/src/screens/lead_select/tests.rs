use insta::assert_snapshot;

use super::*;
use crate::audio::AudioRequest;
use crate::harness::Harness;
use crate::screen::tests::ctx;

use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};

/// One update with `actions`: the transition, as text.
fn update(s: &mut LeadSelectScreen, c: &mut Ctx, actions: &[Action]) -> String {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    format!("{:?}", s.update(c, &input))
}

/// The sounds `actions` play.
fn sounds(s: &mut LeadSelectScreen, c: &mut Ctx, actions: &[Action]) -> Vec<String> {
    c.audio.take();
    update(s, c, actions);
    c.audio
        .take()
        .iter()
        .filter(|r| matches!(r, AudioRequest::PlaySound { .. }))
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect()
}

#[test]
fn starts_with_the_male_lead_and_the_default_name() {
    let s = LeadSelectScreen::default();
    assert_eq!(s.name(), "lead_select");
    assert_eq!(s.gender(), LeadGender::Male);
    assert_eq!(s.first_name(), "Ellery");
    assert_eq!(s.row(), Row::Gender);
    assert!(s.entry().is_none());
    assert!(s.result().is_none());
}

#[test]
fn pick_the_female_lead_and_start() {
    let mut c = ctx();
    let mut s = LeadSelectScreen::new();
    assert_eq!(update(&mut s, &mut c, &[CursorRight]), "None");
    assert_eq!(s.gender(), LeadGender::Female);
    assert_eq!(update(&mut s, &mut c, &[CursorLeft, CursorLeft]), "None");
    assert_eq!(s.gender(), LeadGender::Female);
    // Confirm on the portraits moves on to the name; Down to Start.
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.row(), Row::Name);
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.row(), Row::Start);
    // Left and Right only pick the gender on the portraits' row.
    update(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(s.gender(), LeadGender::Female);
    assert_eq!(update(&mut s, &mut c, &[Confirm, CursorUp]), "Pop");
    assert_eq!(
        s.result(),
        Some(&LeadProfile::new("Ellery", LeadGender::Female))
    );
}

#[test]
fn rows_wrap_and_cancel_goes_back() {
    let mut c = ctx();
    let mut s = LeadSelectScreen::new();
    update(&mut s, &mut c, &[CursorUp]);
    assert_eq!(s.row(), Row::Start);
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.row(), Row::Gender);
    assert_eq!(update(&mut s, &mut c, &[Cancel]), "Pop");
    assert!(s.result().is_none());
}

/// The screen with the name grid (controller players) open on "Ellery".
fn spelling(c: &mut Ctx) -> LeadSelectScreen {
    let mut s = LeadSelectScreen::new();
    update(&mut s, c, &[CursorDown]);
    s.entry = Some(NameEntry::new(&s.name));
    c.audio.take();
    s
}

// ---- Typing the name ----------------------------------------------------

/// One frame of typing: `keys` pressed (with their actions, as the game
/// sends them) and `text` typed.
fn typed(
    s: &mut LeadSelectScreen,
    c: &mut Ctx,
    actions: &[Action],
    keys: &[Key],
    text: &str,
) -> String {
    let pressed = keys
        .iter()
        .map(|&k| crate::input::Chord::plain(k))
        .collect();
    let input =
        FrameInput::new(actions.to_vec(), 0.0, vec![]).with_typing(pressed, text.chars().collect());
    format!("{:?}", s.update(c, &input))
}

/// The sounds of one frame of typing.
fn typed_sounds(s: &mut LeadSelectScreen, c: &mut Ctx, keys: &[Key], text: &str) -> Vec<String> {
    c.audio.take();
    typed(s, c, &[], keys, text);
    c.audio
        .take()
        .iter()
        .filter(|r| matches!(r, AudioRequest::PlaySound { .. }))
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect()
}

/// The screen with the typing box open on "Ellery": Confirm on the name
/// (its key's `f` isn't typed).
fn typing(c: &mut Ctx) -> LeadSelectScreen {
    let mut s = LeadSelectScreen::new();
    update(&mut s, c, &[CursorDown]);
    typed(&mut s, c, &[Confirm], &[Key::F], "f");
    assert!(s.typing().is_some());
    assert_eq!(s.first_name(), "Ellery");
    s
}

use crate::input::Key;

#[test]
fn type_a_new_name() {
    let mut c = ctx();
    let mut s = typing(&mut c);
    typed(&mut s, &mut c, &[], &[Key::Backspace; 6], "");
    assert_eq!(s.first_name(), "");
    // The game's keys do nothing while typing: `d` (Cancel) and `f`
    // (Confirm) are letters.
    let out = typed(
        &mut s,
        &mut c,
        &[Cancel, Confirm],
        &[Key::D, Key::F],
        "Mara d-f'",
    );
    assert_eq!(out, "None");
    assert_eq!(s.first_name(), "Mara d-f'");
    assert_eq!(s.row(), Row::Name);
    typed(&mut s, &mut c, &[], &[Key::Backspace; 5], "");
    // Enter keeps it (trimmed); Start then uses it.
    assert_eq!(
        typed_sounds(&mut s, &mut c, &[Key::Enter], ""),
        ["menu_select"]
    );
    assert!(s.typing().is_none());
    assert_eq!(s.first_name(), "Mara");
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(update(&mut s, &mut c, &[Confirm]), "Pop");
    assert_eq!(
        s.result(),
        Some(&LeadProfile::new("Mara", LeadGender::Male))
    );
}

#[test]
fn typed_names_have_rules() {
    let mut c = ctx();
    let mut s = typing(&mut c);
    // Digits and letters outside A-Z are refused; typing is silent.
    assert_eq!(typed_sounds(&mut s, &mut c, &[], "1é"), ["menu_cancel"; 2]);
    assert!(typed_sounds(&mut s, &mut c, &[], "abcdef").is_empty());
    assert_eq!(s.first_name(), "Elleryabcdef");
    assert_eq!(typed_sounds(&mut s, &mut c, &[], "g"), ["menu_cancel"]);
    assert_eq!(s.first_name().chars().count(), MAX_NAME_LEN);
    // Backspace sounds; with nothing left it does nothing.
    let mut s = typing(&mut c);
    typed(&mut s, &mut c, &[], &[Key::Backspace; 5], "");
    assert_eq!(
        typed_sounds(&mut s, &mut c, &[Key::Backspace], ""),
        ["menu_cancel"]
    );
    assert!(typed_sounds(&mut s, &mut c, &[Key::Backspace], "").is_empty());
    // No space first, or twice; Enter on a blank name is refused.
    assert_eq!(typed_sounds(&mut s, &mut c, &[], " "), ["menu_cancel"]);
    assert_eq!(
        typed_sounds(&mut s, &mut c, &[Key::Enter], ""),
        ["menu_cancel"]
    );
    assert!(s.typing().is_some());
    typed(&mut s, &mut c, &[], &[], "A ");
    assert_eq!(typed_sounds(&mut s, &mut c, &[], " "), ["menu_cancel"]);
    assert_eq!(s.first_name(), "A ");
}

#[test]
fn escape_closes_the_box_and_keeps_the_old_name() {
    let mut c = ctx();
    let mut s = typing(&mut c);
    typed(&mut s, &mut c, &[], &[], "xyz");
    // Escape is also Cancel: the screen stays, only the box closes.
    let out = typed(&mut s, &mut c, &[Cancel], &[Key::Escape], "");
    assert_eq!(out, "None");
    assert!(s.typing().is_none());
    assert_eq!(s.first_name(), "Ellery");
}

#[test]
fn typing_help_names_the_text_keys() {
    let mut c = ctx();
    let s = typing(&mut c);
    assert_eq!(s.help(&c), "Enter done · Backspace delete · Escape cancel");
}

/// The typing box open over the screen with "Mara" typed.
#[test]
fn typing_snapshot() {
    let mut h = Harness::with_screen(Box::new(LeadSelectScreen::new()));
    h.keys("Down f Backspace Backspace Backspace Backspace Backspace Backspace");
    h.type_text("Mara");
    assert_snapshot!(h.snapshot());
}

// ---- The letter grid (controller players) --------------------------------

#[test]
fn spell_a_new_name() {
    let mut c = ctx();
    let mut s = spelling(&mut c);
    // Cancel deletes letters: "Ellery" → "".
    update(&mut s, &mut c, &[Cancel; 6]);
    assert_eq!(s.first_name(), "");
    // "M" (0, 12), then "a" (2, 0).
    update(&mut s, &mut c, &[CursorLeft, Confirm]);
    update(
        &mut s,
        &mut c,
        &[CursorDown, CursorDown, CursorRight, Confirm],
    );
    assert_eq!(s.first_name(), "Ma");
    // Space, then Delete it: (4, 2) and (4, 3).
    update(
        &mut s,
        &mut c,
        &[CursorDown, CursorDown, CursorRight, CursorRight],
    );

    assert_eq!(s.entry().map(NameEntry::focus), Some((4, 2)));
    update(&mut s, &mut c, &[Confirm]);
    assert_eq!(s.first_name(), "Ma ");
    update(&mut s, &mut c, &[CursorRight, Confirm]);
    assert_eq!(s.first_name(), "Ma");
    // Done keeps the name.
    update(&mut s, &mut c, &[CursorRight, Confirm]);
    assert!(s.entry().is_none());
    assert_eq!(s.first_name(), "Ma");
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(update(&mut s, &mut c, &[Confirm]), "Pop");
    assert_eq!(s.result(), Some(&LeadProfile::new("Ma", LeadGender::Male)));
}

#[test]
fn the_grid_wraps_along_rows_and_keeps_to_short_rows() {
    let mut c = ctx();
    let mut s = spelling(&mut c);
    let at = |s: &LeadSelectScreen| s.entry().map(NameEntry::focus);
    update(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(at(&s), Some((0, 12)));
    update(&mut s, &mut c, &[CursorRight]);
    assert_eq!(at(&s), Some((0, 0)));
    // Up from the top row: the last row, whose 5 cells end at column 4.
    update(&mut s, &mut c, &[CursorLeft, CursorUp]);
    assert_eq!(at(&s), Some((4, 4)));
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(at(&s), Some((0, 4)));
    update(&mut s, &mut c, &[CursorUp, CursorUp]);
    assert_eq!(at(&s), Some((3, 4)));
    update(&mut s, &mut c, &[CursorRight]);
    assert_eq!(at(&s), Some((3, 5)));
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(at(&s), Some((4, 4)));
}

#[test]
fn names_have_rules() {
    let mut c = ctx();
    let mut s = spelling(&mut c);
    // Twelve characters at most: "Ellery" + six more; the 13th is refused.
    assert_eq!(sounds(&mut s, &mut c, &[Confirm; 6]), ["menu_select"; 6]);
    assert_eq!(s.first_name(), "ElleryAAAAAA");
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    assert_eq!(s.first_name().chars().count(), MAX_NAME_LEN);
    // No space at the start, or after another.
    let mut s = spelling(&mut c);
    update(&mut s, &mut c, &[Cancel; 6]);
    update(&mut s, &mut c, &[CursorUp, CursorRight, CursorRight]);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    assert_eq!(s.first_name(), "");
    // Done on a blank name is refused; Delete on it too.
    update(&mut s, &mut c, &[CursorRight, CursorRight]);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    update(&mut s, &mut c, &[CursorLeft]);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    // "D" (down from Delete), space, then a second space is refused; Done
    // trims the name.
    update(
        &mut s,
        &mut c,
        &[CursorDown, Confirm, CursorUp, CursorLeft, Confirm],
    );
    assert_eq!(s.first_name(), "D ");
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_cancel"]);
    update(&mut s, &mut c, &[CursorRight, CursorRight, Confirm]);
    assert_eq!(s.first_name(), "D");
    assert!(s.entry().is_none());
}

#[test]
fn cancel_on_an_empty_name_puts_the_old_one_back() {
    let mut c = ctx();
    let mut s = spelling(&mut c);
    assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_cancel"]);
    assert_eq!(s.first_name(), "Eller");
    update(&mut s, &mut c, &[Cancel; 5]);
    assert_eq!(s.first_name(), "");
    assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_cancel"]);
    assert!(s.entry().is_none());
    assert_eq!(s.first_name(), "Ellery");
    // The screen itself is still up.
    assert_eq!(update(&mut s, &mut c, &[]), "None");
}

#[test]
fn menu_sounds() {
    let mut c = ctx();
    let mut s = LeadSelectScreen::new();
    assert_eq!(sounds(&mut s, &mut c, &[CursorRight]), ["menu_move"]);
    assert_eq!(sounds(&mut s, &mut c, &[CursorDown]), ["menu_move"]);
    assert_eq!(sounds(&mut s, &mut c, &[Confirm]), ["menu_select"]);
    // The typing box is open: the game's keys play nothing.
    assert!(sounds(&mut s, &mut c, &[CursorRight, Confirm, Cancel]).is_empty());
    // An action with nothing to do plays nothing.
    let mut s = LeadSelectScreen::new();
    assert!(sounds(&mut s, &mut c, &[Action::Info]).is_empty());
    let mut s = spelling(&mut c);
    assert!(sounds(&mut s, &mut c, &[Action::Info]).is_empty());
    assert_eq!(sounds(&mut s, &mut c, &[CursorDown]), ["menu_move"]);
    let mut s = LeadSelectScreen::new();
    assert_eq!(sounds(&mut s, &mut c, &[Cancel]), ["menu_cancel"]);
}

#[test]
fn help_names_the_keys() {
    let mut c = ctx();
    let mut s = LeadSelectScreen::new();
    assert_eq!(s.help(&c), "arrows choose · f next · d back");
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.help(&c), "arrows choose · f change name · d back");
    update(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.help(&c), "arrows choose · f start · d back");
    let s = spelling(&mut c);
    assert!(s.typing().is_none());
    assert_eq!(s.help(&c), "arrows choose · f type · d delete");
    c.use_layout(crate::input::Layout::LeftHanded);
    assert_eq!(s.help(&c), "wasd choose · j type · k delete");
}

#[test]
fn grid_cells() {
    let rows = grid();
    assert_eq!(rows.len(), 5);
    assert!(rows[..4].iter().all(|r| r.len() == 13));
    assert_eq!(rows[1][12], GridCell::Char('Z'));
    assert_eq!(rows[3][0], GridCell::Char('n'));
    let labels: Vec<String> = rows[4].iter().map(|g| g.label()).collect();
    assert_eq!(labels, ["-", "'", "Blank", "Delete", "Done"]);
}

/// Both portraits, the male one chosen, the name and Start.
#[test]
fn lead_select_snapshot() {
    let h = Harness::with_screen(Box::new(LeadSelectScreen::new()));
    assert_snapshot!(h.snapshot());
}

/// The female lead chosen, and the name grid open over the screen on
/// `N`, the name cut to `Ell`.

#[test]
fn name_grid_snapshot() {
    let mut s = LeadSelectScreen::new();
    s.gender = LeadGender::Female;
    s.row = Row::Name;
    s.entry = Some(NameEntry::new("Ell"));
    let mut h = Harness::with_screen(Box::new(s));
    h.keys("Down");
    assert_snapshot!(h.snapshot());
}
