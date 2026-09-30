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

/// The screen with the name grid open on "Ellery".
fn spelling(c: &mut Ctx) -> LeadSelectScreen {
    let mut s = LeadSelectScreen::new();
    update(&mut s, c, &[CursorDown, Confirm]);
    assert!(s.entry().is_some());
    s
}

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
    assert_eq!(sounds(&mut s, &mut c, &[CursorRight]), ["menu_move"]);
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
    let mut h = Harness::with_screen(Box::new(LeadSelectScreen::new()));
    h.keys("Right Down f d d d Down");
    assert_snapshot!(h.snapshot());
}
