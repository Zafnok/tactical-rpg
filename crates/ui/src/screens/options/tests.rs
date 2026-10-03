use super::*;
use crate::audio::AudioRequest;
use crate::input::Layout;
use crate::screen::tests::ctx;
use crate::settings::SETTINGS_KEY;
use crate::storage::{Storage, StorageError};
use crate::tips::{TIPS_SEEN_KEY, TipsSeen};

fn press(s: &mut OptionsScreen, c: &mut Ctx, actions: &[Action]) -> String {
    let input = FrameInput::new(actions.to_vec(), 0.0, vec![]);
    format!("{:?}", s.update(c, &input))
}

/// The screen focused on `row`.
fn on(row: Row) -> OptionsScreen {
    let mut s = OptionsScreen::new();
    s.row = row;
    s
}

/// The sounds asked for since the last call.
fn sounds(c: &mut Ctx) -> Vec<String> {
    let cue = |r: AudioRequest| r.cue().map(str::to_owned);
    c.audio.take().into_iter().filter_map(cue).collect()
}

fn saved(c: &Ctx) -> Settings {
    let text = c.storage.read(SETTINGS_KEY).unwrap().unwrap();
    Settings::from_ron(&text).unwrap()
}

#[test]
fn game_mode_shows_only_with_a_campaign() {
    let mut c = ctx();
    let rows = OptionsScreen::rows(&c);
    assert_eq!(rows.len(), Row::ALL.len() - 1);
    assert!(!rows.contains(&Row::GameMode));
    c.campaign_mode = Some(GameMode::Casual);
    assert_eq!(OptionsScreen::rows(&c), Row::ALL);
}

#[test]
fn up_and_down_move_the_focus_and_wrap() {
    use Action::{CursorDown, CursorUp};
    let mut c = ctx();
    let mut s = OptionsScreen::new();
    assert_eq!(s.name(), "options");
    assert!(!s.is_overlay());
    assert_eq!(s.focus(), Row::TextSpeed);
    press(&mut s, &mut c, &[CursorDown]);
    assert_eq!(s.focus(), Row::AnimSpeed);
    assert_eq!(sounds(&mut c), ["menu_move"]);
    press(&mut s, &mut c, &[CursorUp, CursorUp]);
    assert_eq!(s.focus(), Row::RestoreDefaults);
    press(&mut s, &mut c, &[CursorUp, CursorUp]);
    // No campaign: Game mode is skipped.
    assert_eq!(s.focus(), Row::KeyBindings);
    press(&mut s, &mut c, &[CursorDown, CursorDown, CursorDown]);
    assert_eq!(s.focus(), Row::TextSpeed);
}

#[test]
fn left_and_right_step_a_setting_and_stop_at_the_ends() {
    use Action::{CursorLeft, CursorRight};
    let mut c = ctx();
    let mut s = OptionsScreen::new();
    press(&mut s, &mut c, &[CursorRight]);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
    assert_eq!(saved(&c).text_speed, TextSpeed::Fast);
    assert_eq!(sounds(&mut c), ["menu_move"]);
    press(&mut s, &mut c, &[CursorRight, CursorRight, CursorRight]);
    assert_eq!(c.settings().text_speed, TextSpeed::Instant);
    // Only the step that changed something sounds.
    assert_eq!(sounds(&mut c), ["menu_move"]);
    press(&mut s, &mut c, &[CursorLeft; 5]);
    assert_eq!(c.settings().text_speed, TextSpeed::Slow);
    assert_eq!(sounds(&mut c).len(), 3);
}

#[test]
fn confirm_steps_a_setting_and_goes_round() {
    let mut c = ctx();
    let mut s = on(Row::Cursor);
    let styles: Vec<_> = (0..3)
        .map(|_| {
            press(&mut s, &mut c, &[Action::Confirm]);
            c.settings().cursor_style
        })
        .collect();
    assert_eq!(
        styles,
        [
            CursorStyle::LargeCorners,
            CursorStyle::TileGlow,
            CursorStyle::Corners
        ]
    );
    let mut s = on(Row::SoundVolume);
    press(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    assert_eq!(c.settings().sound_volume, 10);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(c.settings().sound_volume, 0);
}

#[test]
fn every_setting_row_changes_its_own_setting() {
    let mut c = ctx();
    for row in Row::ALL.into_iter().filter(|r| r.is_setting()) {
        let before = c.settings().clone();
        // Right, or left for a setting already at its last value.
        press(&mut on(row), &mut c, &[Action::CursorRight]);
        if *c.settings() == before {
            press(&mut on(row), &mut c, &[Action::CursorLeft]);
        }
        let s = c.settings().clone();
        let changed = [
            (Row::TextSpeed, s.text_speed != before.text_speed),
            (Row::AnimSpeed, s.anim_speed != before.anim_speed),
            (
                Row::CombatAnimations,
                s.combat_animations != before.combat_animations,
            ),
            (
                Row::EnemyPhaseSpeed,
                s.enemy_phase_speed != before.enemy_phase_speed,
            ),
            (Row::AutoEnd, s.auto_end_turn != before.auto_end_turn),
            (Row::Fullscreen, s.fullscreen != before.fullscreen),
            (Row::Cursor, s.cursor_style != before.cursor_style),
            (Row::MusicVolume, s.music_volume != before.music_volume),
            (Row::SoundVolume, s.sound_volume != before.sound_volume),
        ];
        let rows: Vec<Row> = changed.iter().filter(|c| c.1).map(|c| c.0).collect();
        assert_eq!(rows, [row]);
        assert_eq!(saved(&c), s);
    }
}

#[test]
fn rows_show_their_values() {
    let mut c = ctx();
    let value = |c: &Ctx, row| OptionsScreen::value(c, row);
    let shown: Vec<String> = Row::ALL.iter().map(|&r| value(&c, r)).collect();
    assert_eq!(
        shown,
        [
            "Normal",
            "Normal",
            "On",
            "Normal",
            "Off",
            "Off",
            "Corners",
            "████████░░  8",
            "████████░░  8",
            "Right-handed",
            "",
            "",
            "",
            ""
        ]
    );
    c.change_settings(|s| {
        s.cursor_style = CursorStyle::LargeCorners;
        s.music_volume = 0;
        s.sound_volume = 10;
        s.auto_end_turn = true;
    })
    .unwrap();
    c.campaign_mode = Some(GameMode::Classic);
    c.use_layout(Layout::LeftHanded);
    assert_eq!(value(&c, Row::Cursor), "Large corners");
    assert_eq!(value(&c, Row::MusicVolume), "░░░░░░░░░░  0");
    assert_eq!(value(&c, Row::SoundVolume), "██████████ 10");
    assert_eq!(value(&c, Row::AutoEnd), "On");
    assert_eq!(value(&c, Row::GameMode), "Classic");
    assert_eq!(value(&c, Row::Layout), "Left-handed");
    assert_eq!(cursor_label(CursorStyle::TileGlow), "Tile glow");
    c.campaign_mode = Some(GameMode::Casual);
    assert_eq!(value(&c, Row::GameMode), "Casual");
}

#[test]
fn layout_and_key_bindings_open_their_screens() {
    let mut c = ctx();
    assert_eq!(
        press(&mut on(Row::Layout), &mut c, &[Action::Confirm]),
        "Push(layout_picker)"
    );
    assert_eq!(
        press(&mut on(Row::KeyBindings), &mut c, &[Action::Confirm]),
        "Push(key_bindings)"
    );
    assert_eq!(sounds(&mut c), ["menu_select", "menu_select"]);
    // Left and right do nothing on them.
    let before = c.settings().clone();
    for row in [Row::Layout, Row::KeyBindings, Row::GameMode, Row::ResetTips] {
        let keys = [Action::CursorLeft, Action::CursorRight];
        assert_eq!(press(&mut on(row), &mut c, &keys), "None");
    }
    assert_eq!(*c.settings(), before);
    assert!(sounds(&mut c).is_empty());
}

#[test]
fn cancel_closes_the_screen() {
    let mut c = ctx();
    let mut s = OptionsScreen::new();
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "Pop");
    assert_eq!(sounds(&mut c), ["menu_cancel"]);
}

#[test]
fn classic_switches_to_casual_after_a_confirm_and_never_back() {
    let mut c = ctx();
    c.campaign_mode = Some(GameMode::Classic);
    let mut s = on(Row::GameMode);
    assert_eq!(s.help(&c), "arrows move · f switch to Casual · d back");
    press(&mut s, &mut c, &[Action::Confirm, Action::Confirm]);
    // The key that asked isn't the answer.
    assert_eq!(s.asking(), Some(Question::SwitchToCasual));
    assert_eq!(c.campaign_mode, Some(GameMode::Classic));
    assert_eq!(s.help(&c), "f yes · d no");
    // Nothing but the answers does anything.
    press(&mut s, &mut c, &[Action::CursorDown, Action::CursorRight]);
    assert_eq!((s.focus(), s.asking().is_some()), (Row::GameMode, true));
    // No: still Classic.
    assert_eq!(press(&mut s, &mut c, &[Action::Cancel]), "None");
    assert_eq!((s.asking(), s.message()), (None, None));
    assert_eq!(c.campaign_mode, Some(GameMode::Classic));
    // Yes.
    press(&mut s, &mut c, &[Action::Confirm]);
    sounds(&mut c);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(c.campaign_mode, Some(GameMode::Casual));
    assert_eq!(s.message(), Some(CASUAL_MESSAGE));
    assert_eq!(sounds(&mut c), ["menu_select"]);
    // Casual offers nothing.
    assert_eq!(s.help(&c), "arrows move · d back");
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.asking(), None);
    assert_eq!(c.campaign_mode, Some(GameMode::Casual));
    assert_eq!(sounds(&mut c), ["menu_cancel"]);
}

#[test]
fn reset_tips_forgets_the_tips_seen() {
    let mut c = ctx();
    let mut seen = TipsSeen::default();
    seen.mark("first_move", &mut *c.storage);
    assert!(TipsSeen::load(&*c.storage).contains("first_move"));
    let mut s = on(Row::ResetTips);
    assert_eq!(s.help(&c), "arrows move · f reset · d back");
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(c.storage.read(TIPS_SEEN_KEY), Ok(None));
    assert_eq!(s.message(), Some(TIPS_RESET_MESSAGE));
    // Moving on clears the message.
    press(&mut s, &mut c, &[Action::CursorDown]);
    assert_eq!(s.message(), None);
}

#[test]
fn restore_defaults_asks_first_and_keeps_the_layout() {
    let mut c = ctx();
    c.choose_layout(Layout::LeftHanded).unwrap();
    c.change_settings(|s| {
        s.text_speed = TextSpeed::Slow;
        s.fullscreen = true;
        s.music_volume = 2;
    })
    .unwrap();
    let changed = c.settings().clone();
    let mut s = on(Row::RestoreDefaults);
    assert_eq!(s.help(&c), "wasd move · j restore · k back");
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.asking(), Some(Question::RestoreDefaults));
    press(&mut s, &mut c, &[Action::Cancel]);
    assert_eq!(*c.settings(), changed);
    press(&mut s, &mut c, &[Action::Confirm]);
    press(&mut s, &mut c, &[Action::Confirm]);
    let expected = Settings::default().with_layout(Layout::LeftHanded);
    assert_eq!(*c.settings(), expected);
    assert_eq!(saved(&c), expected);
    assert_eq!(s.message(), Some(RESTORED_MESSAGE));
    assert_eq!(c.layout(), Some(Layout::LeftHanded));
}

/// A storage that reads nothing and can't be written.
#[derive(Debug)]
struct ReadOnly;

impl Storage for ReadOnly {
    fn read(&self, _: &str) -> Result<Option<String>, StorageError> {
        Ok(None)
    }
    fn write(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Backend("full".into()))
    }
    fn delete(&mut self, _: &str) -> Result<(), StorageError> {
        Err(StorageError::Backend("full".into()))
    }
    fn list(&self) -> Result<Vec<String>, StorageError> {
        Ok(Vec::new())
    }
}

#[test]
fn a_failed_save_still_changes_the_setting_and_says_so() {
    let mut c = ctx().with_storage(Box::new(ReadOnly));
    let mut s = OptionsScreen::new();
    press(&mut s, &mut c, &[Action::CursorRight]);
    assert_eq!(c.settings().text_speed, TextSpeed::Fast);
    assert_eq!(s.message(), Some(NOT_SAVED_MESSAGE));
    let mut s = on(Row::ResetTips);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(s.message(), Some(NOT_SAVED_MESSAGE));
    let mut s = on(Row::RestoreDefaults);
    press(&mut s, &mut c, &[Action::Confirm]);
    press(&mut s, &mut c, &[Action::Confirm]);
    assert_eq!(c.settings().text_speed, TextSpeed::Normal);
    assert_eq!(s.message(), Some(NOT_SAVED_MESSAGE));
}

#[test]
fn help_names_what_confirm_does_on_each_kind_of_row() {
    let c = ctx();
    assert_eq!(
        OptionsScreen::new().help(&c),
        "arrows move · f change · d back"
    );
    assert_eq!(on(Row::Layout).help(&c), "arrows move · f open · d back");
    assert_eq!(
        on(Row::KeyBindings).help(&c),
        "arrows move · f open · d back"
    );
}

#[test]
fn stepping_helpers_stop_or_wrap() {
    let all = [1, 2, 3];
    assert_eq!(stepped(&all, 3, true, false), 3);
    assert_eq!(stepped(&all, 3, true, true), 1);
    assert_eq!(stepped(&all, 1, false, false), 1);
    assert_eq!(stepped(&all, 1, false, true), 3);
    assert_eq!(stepped(&all, 2, true, false), 3);
    assert_eq!(stepped(&all, 2, false, false), 1);
    assert_eq!(stepped_volume(10, true, false), 10);
    assert_eq!(stepped_volume(10, true, true), 0);
    assert_eq!(stepped_volume(9, true, true), 10);
    assert_eq!(stepped_volume(0, false, false), 0);
    assert_eq!(stepped_volume(0, false, true), 0);
    assert_eq!(stepped_volume(5, false, false), 4);
}
