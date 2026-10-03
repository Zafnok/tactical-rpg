//! Scripted tests of the Options screen through the real game (ticket
//! 0805): opened from the title and from the map menu; every setting
//! changes what the game does and is saved; the layout switches from it;
//! a Classic campaign switches to Casual, one way.
//!
//! Rows, top to bottom: Text speed, Animation speed, Combat animations,
//! Enemy phase speed, Auto-end turn, Fullscreen, Cursor, Music volume,
//! Sound volume, Layout, Key bindings, (Game mode, with a campaign), Reset
//! tips, Restore defaults. `key_bindings_screen.rs` opens the Key bindings
//! row.

use insta::assert_snapshot;
use trpg_core::GameMode;
use trpg_ui::harness::{FRAME_DT, Harness};
use trpg_ui::input::{Action, Chord, Layout};
use trpg_ui::map_view::CursorStyle;
use trpg_ui::settings::{AnimSpeed, EnemyPhaseSpeed, Settings, TextSpeed};
use trpg_ui::tips::TIPS_SEEN_KEY;

/// From the first row down to each row.
const TO_AUTO_END: &str = "Down Down Down Down";
const TO_FULLSCREEN: &str = "Down Down Down Down Down";
const TO_CURSOR: &str = "Down Down Down Down Down Down";
const TO_MUSIC: &str = "Down Down Down Down Down Down Down";
const TO_LAYOUT: &str = "Down Down Down Down Down Down Down Down Down";

/// A launch after the right-handed layout was picked.
fn title() -> Harness {
    Harness::with_layout(Layout::RightHanded)
}

/// Options, from the title: two rows under New Game (past Quick Battle).
fn options() -> Harness {
    let mut h = title();
    h.keys("Down Down f");
    assert_eq!(h.screens(), ["title", "options"]);
    h
}

/// From Options back at the title (still on its Options item), the Quick
/// Battle with its `PLAYER PHASE` banner closed.
fn into_quick_battle(h: &mut Harness) {
    h.keys("d Up f f");
    assert_eq!(h.screens(), ["title", "battle"]);
}

/// The Quick Battle, then Options from its map menu (Units, Objective,
/// Options).
fn options_in_battle() -> Harness {
    let mut h = title();
    h.keys("Down f f d Down Down f");
    assert_eq!(h.screens(), ["title", "battle", "options"]);
    h
}

fn settings(h: &Harness) -> Settings {
    h.game().ctx().settings().clone()
}

/// The next launch with what this one saved.
fn relaunch(h: Harness) -> Harness {
    let mut h = Harness::with_storage(h.into_storage());
    h.wait(FRAME_DT);
    h
}

fn shows(h: &Harness, text: &str) -> bool {
    let buf = h.game().buffer();
    (0..32).any(|y| {
        (0..100)
            .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
            .collect::<String>()
            .contains(text)
    })
}

fn mode(h: &Harness) -> Option<GameMode> {
    h.flow().and_then(|f| f.campaign()).map(|c| c.mode)
}

#[test]
fn the_title_opens_options_and_cancel_closes_it() {
    let mut h = title();
    let before = h.snapshot();
    h.keys("Down Down f");
    assert_eq!(h.screens(), ["title", "options"]);
    assert_eq!(
        settings(&h),
        Settings::default().with_layout(Layout::RightHanded)
    );
    // No campaign: no Game mode row.
    assert!(!shows(&h, "Game mode"));
    assert_snapshot!(h.snapshot());
    h.keys("d");
    assert_eq!(h.screens(), ["title"]);
    // The title is as it was, still on Options.
    h.keys("Up Up");
    assert_eq!(h.snapshot(), before);
    // Escape closes it too.
    h.keys("Down Down f Escape");
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn the_map_menu_opens_options_over_the_battle() {
    let mut h = options_in_battle();
    // The Quick Battle is a Classic campaign: its mode shows.
    assert!(shows(&h, "Game mode"));
    assert!(shows(&h, "Classic"));
    assert_snapshot!(h.snapshot());
    h.keys("d");
    // Back in the map menu, still on Options.
    assert_eq!(h.screens(), ["title", "battle"]);
    assert!(shows(&h, "Restart Battle"));
    h.keys("f");
    assert_eq!(h.screens(), ["title", "battle", "options"]);
}

/// The glyphs on screen that aren't blank.
fn glyphs_shown(h: &Harness) -> usize {
    let snap = h.snapshot();
    let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
    glyphs.chars().filter(|c| !c.is_whitespace()).count()
}

/// Acceptance: the text speed changes how fast a dialogue box fills.
#[test]
fn text_speed_changes_how_fast_dialogue_appears() {
    // `keys` on the Text speed row, then the debug menu's test scene for a
    // tenth of a second.
    let shown_at = |keys: &str, speed: TextSpeed| {
        let mut h = options();
        h.keys(keys);
        assert_eq!(settings(&h).text_speed, speed);
        h.keys("d F2 Down Down f").wait(0.1);
        assert_eq!(h.top_screen(), "dialogue");
        glyphs_shown(&h)
    };
    let slow = shown_at("Left", TextSpeed::Slow);
    let normal = shown_at("", TextSpeed::Normal);
    let fast = shown_at("Right", TextSpeed::Fast);
    let instant = shown_at("Right Right", TextSpeed::Instant);
    assert!(slow < normal && normal < fast, "{slow} {normal} {fast}");
    assert!(fast < instant, "{fast} {instant}");
    // Instant shows the whole box: more time adds nothing.
    let mut h = options();
    h.keys("Right Right d F2 Down Down f");
    let at_once = glyphs_shown(&h);
    h.wait(0.5);
    assert!(glyphs_shown(&h) <= at_once + 1, "only the prompt to go on");
}

/// Acceptance: the speeds and the combat animations switch are the
/// settings the battle reads (`screens/battle/settings_tests.rs` times
/// them).
#[test]
fn the_battle_speed_rows_set_their_settings() {
    let mut h = options();
    h.keys("Down Right");
    assert_eq!(settings(&h).anim_speed, AnimSpeed::Fast);
    h.keys("Down Left");
    assert!(!settings(&h).combat_animations);
    h.keys("Down Right");
    assert_eq!(settings(&h).enemy_phase_speed, EnemyPhaseSpeed::Fast);
    assert!(shows(&h, "Combat animations"));
    let h = relaunch(h);
    let saved = settings(&h);
    assert_eq!(saved.anim_speed, AnimSpeed::Fast);
    assert_eq!(saved.enemy_phase_speed, EnemyPhaseSpeed::Fast);
    assert!(!saved.combat_animations);
    assert!((saved.battle_speed(true) - 4.0).abs() < f32::EPSILON);
}

/// Acceptance: auto-end set in Options is on in battle; the battle's
/// Auto-end key flips the same setting, and that is saved too.
#[test]
fn auto_end_is_one_setting_in_options_and_in_battle() {
    let mut h = options();
    h.keys(TO_AUTO_END).keys("Right");
    assert!(settings(&h).auto_end_turn);
    into_quick_battle(&mut h);
    assert!(shows(&h, "auto-end: ON"));
    h.keys("Shift+Space");
    assert!(shows(&h, "auto-end: OFF"));
    assert!(!settings(&h).auto_end_turn);
    // Options over the battle shows it off, and turns it on again.
    h.keys("d Down Down f").keys(TO_AUTO_END);
    let row = format!("{:<24}◄ Off ►", "Auto-end turn");
    assert!(shows(&h, &row), "{}", h.snapshot());
    h.keys("f d d");
    assert!(shows(&h, "auto-end: ON"));
    let h = relaunch(h);
    assert!(settings(&h).auto_end_turn);
}

/// Acceptance: the Fullscreen row tells `app` to fill the screen.
#[test]
fn fullscreen_is_asked_of_the_app_and_saved() {
    let mut h = options();
    assert!(!h.fullscreen());
    h.keys(TO_FULLSCREEN).keys("Right");
    assert!(h.fullscreen());
    let mut h = relaunch(h);
    assert!(h.fullscreen(), "from the first frame of the next launch");
    h.keys("Down Down f").keys(TO_FULLSCREEN).keys("Left");
    assert!(!h.fullscreen());
}

/// Acceptance: the Cursor row changes the battle cursor.
#[test]
fn the_cursor_row_changes_the_battle_cursor() {
    let style = |h: &Harness| h.map_scene().and_then(|s| s.cursor).map(|c| c.style);
    let mut h = options();
    h.keys(TO_CURSOR).keys("Right");
    assert!(shows(&h, "Large corners"));
    into_quick_battle(&mut h);
    assert_eq!(style(&h), Some(CursorStyle::LargeCorners));
    // Changed over the battle, it shows as soon as the battle is back.
    h.keys("d Down Down f").keys(TO_CURSOR).keys("Right");
    assert!(shows(&h, "Tile glow"));
    h.keys("d d");
    assert_eq!(style(&h), Some(CursorStyle::TileGlow));
    let mut h = relaunch(h);
    h.keys("Down f f");
    assert_eq!(style(&h), Some(CursorStyle::TileGlow));
}

/// Acceptance: the volumes change what `app` is told to play at, 0
/// silences, and they persist across a restart.
#[test]
fn volumes_reach_the_mixer_and_persist() {
    let close = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6;
    let mut h = options();
    // The defaults: 8 of 10 each.
    assert!(close(h.volumes(), (0.8, 0.8)), "{:?}", h.volumes());
    h.keys(TO_MUSIC).keys("Left Left Left");
    assert!(close(h.volumes(), (0.5, 0.8)), "{:?}", h.volumes());
    // Past 0 it stays silent.
    h.keys("Left Left Left Left Left Left Left");
    assert!(close(h.volumes(), (0.0, 0.8)), "{:?}", h.volumes());
    assert_eq!(settings(&h).music_volume, 0);
    // The sound volume, up to its loudest and no further.
    h.keys("Down Right Right Right");
    assert!(close(h.volumes(), (0.0, 1.0)), "{:?}", h.volumes());
    assert_eq!(settings(&h).sound_volume, 10);
    // Each step that changed it ticks (at the new volume); the third
    // Right changed nothing.
    h.clear_audio().keys("Left Right Right");
    assert_eq!(h.sounds(), ["menu_move", "menu_move"]);
    let h = relaunch(h);
    assert!(close(h.volumes(), (0.0, 1.0)), "{:?}", h.volumes());
}

/// Acceptance: the layout switches from Options, both ways, as often as
/// the player likes; the new keys work at once and persist.
#[test]
fn the_layout_switches_from_options_both_ways_and_persists() {
    let layout = |h: &Harness| h.game().ctx().layout();
    let mut h = options();
    h.keys(TO_LAYOUT);
    assert!(shows(&h, "Right-handed"));
    h.keys("f");
    assert_eq!(h.screens(), ["title", "options", "layout_picker"]);
    // The layout in use is focused, and Cancel backs out unchanged.
    assert!(shows(&h, "► Right-handed"));
    assert!(shows(&h, "d back"));
    assert_snapshot!(h.snapshot());
    h.keys("d");
    assert_eq!(h.screens(), ["title", "options"]);
    assert_eq!(layout(&h), Some(Layout::RightHanded));
    // Left-handed: its keys work at once (`f` and the arrows don't).
    h.keys("f Down f");
    assert_eq!(h.screens(), ["title", "options"]);
    assert_eq!(layout(&h), Some(Layout::LeftHanded));
    assert!(shows(&h, "Left-handed"));
    assert!(shows(&h, "wasd move · j open · k back"));
    h.keys("f Up d");
    assert_eq!(h.screens(), ["title", "options"]);
    // And back, with the left-handed keys: the picker opens on
    // Left-handed, one up is Right-handed.
    h.keys("j");
    assert!(shows(&h, "► Left-handed"));
    h.keys("w j");
    assert_eq!(layout(&h), Some(Layout::RightHanded));
    assert!(shows(&h, "arrows move · f open · d back"));
    // And once more.
    h.keys("f Down f");
    assert_eq!(layout(&h), Some(Layout::LeftHanded));
    h.keys("k");
    assert_eq!(h.screens(), ["title"]);
    let mut h = relaunch(h);
    assert_eq!(h.screens(), ["title"], "no picker: the layout was saved");
    assert_eq!(layout(&h), Some(Layout::LeftHanded));
    h.keys("j");
    assert_eq!(h.screens(), ["title", "mode_select"]);
}

/// Acceptance: switching layout loads that layout's own custom keys.
#[test]
fn switching_layout_loads_that_layouts_own_keys() {
    let chord = |s: &str| Chord::parse(s).unwrap_or_else(|e| panic!("{e}"));
    let mut h = title();
    // Left-handed has `h` as a second Confirm key; right-handed has `g`.
    let mut left = h.game().ctx().layout_bindings(Layout::LeftHanded);
    assert_eq!(left.bind(Action::Confirm, 1, chord("h")), Ok(None));
    let mut right = h.game().ctx().layout_bindings(Layout::RightHanded);
    assert_eq!(right.bind(Action::Confirm, 1, chord("g")), Ok(None));
    let ctx = h.ctx_mut();
    assert_eq!(ctx.set_layout_bindings(Layout::LeftHanded, left), Ok(()));
    assert_eq!(ctx.set_layout_bindings(Layout::RightHanded, right), Ok(()));
    h.keys("Down Down g");
    assert_eq!(h.screens(), ["title", "options"]);
    // `h` does nothing right-handed.
    h.keys(TO_LAYOUT).keys("h");
    assert_eq!(h.screens(), ["title", "options"]);
    h.keys("g Down g");
    assert_eq!(h.game().ctx().layout(), Some(Layout::LeftHanded));
    // Now `h` confirms (the Layout row opens the picker) and `g` doesn't.
    h.keys("g");
    assert_eq!(h.screens(), ["title", "options"]);
    h.keys("h");
    assert_eq!(h.screens(), ["title", "options", "layout_picker"]);
    // Back to right-handed: its `g` is back.
    h.keys("w h g");
    assert_eq!(h.game().ctx().layout(), Some(Layout::RightHanded));
    assert_eq!(h.screens(), ["title", "options", "layout_picker"]);
}

/// Acceptance: Classic switches to Casual after a confirm, for the
/// campaign and for the battle when it is restarted; Casual never offers
/// Classic.
#[test]
fn classic_switches_to_casual_with_a_confirm_and_never_back() {
    let mut h = options_in_battle();
    assert_eq!(mode(&h), Some(GameMode::Classic));
    // Game mode is three rows up from the first (past Restore defaults
    // and Reset tips).
    h.keys("Up Up Up");
    assert!(shows(&h, "f switch to Casual"));
    h.keys("f");
    assert!(shows(&h, "Switch to Casual? This can't be undone."));
    assert!(shows(&h, "f yes / d no"));
    assert_snapshot!(h.snapshot());
    // No: still Classic, still in Options.
    h.keys("d");
    assert_eq!(h.screens(), ["title", "battle", "options"]);
    assert!(shows(&h, "Classic"));
    // Yes.
    h.keys("f f");
    assert!(shows(&h, "Game mode is now Casual"));
    assert!(!shows(&h, "Classic"));
    // Casual offers nothing: Confirm does nothing, and the help says so.
    assert!(!shows(&h, "switch to Casual"));
    h.keys("f");
    assert!(!shows(&h, "Switch to Casual?"));
    // Back in the battle, the campaign is Casual.
    h.keys("d");
    assert_eq!(mode(&h), Some(GameMode::Casual));
    // The map menu is still open on Options: Restart Battle is two down,
    // and the battle starts again in Casual.
    h.keys("Down Down f f");
    assert_eq!(h.screens(), ["title", "battle"]);
    let battle = h.battle().map(|b| b.state().mode());
    assert_eq!(battle, Some(GameMode::Casual));
    // Options again: Casual, with nothing to switch to.
    h.keys("f d Down Down f Up Up Up");
    assert_eq!(h.screens(), ["title", "battle", "options"]);
    assert!(shows(&h, "Casual") && !shows(&h, "switch to Casual"));
    h.keys("f");
    assert_eq!(mode(&h), Some(GameMode::Casual));
}

/// A campaign started in Casual never offers Classic, and the title's
/// Options has no Game mode row once the campaign is left.
#[test]
fn a_casual_campaign_shows_its_mode_and_the_title_shows_none() {
    let mut h = title();
    // New Game, Casual, the default lead; skip the intro scene; close the
    // battle notes and the banner.
    h.keys("f Down f Up f");
    assert_eq!(h.top_screen(), "dialogue");
    h.keys("d f f f");
    assert_eq!(h.screens(), ["title", "battle"]);
    assert_eq!(mode(&h), Some(GameMode::Casual));
    h.keys("d Down Down f Up Up Up");
    assert_eq!(h.screens(), ["title", "battle", "options"]);
    assert!(shows(&h, "Game mode") && shows(&h, "Casual"));
    h.keys("f");
    assert!(!shows(&h, "Switch to Casual?"));
    // Suspend (the next row of the map menu) back to the title.
    h.keys("d Down f f");
    assert_eq!(h.screens(), ["title"]);
    h.wait(FRAME_DT);
    assert_eq!(h.game().ctx().campaign_mode, None);
}

#[test]
fn reset_tips_forgets_the_tips_seen() {
    let mut h = title();
    h.ctx_mut().storage.write(TIPS_SEEN_KEY, "first_move").ok();
    // Reset tips is two rows up from the first.
    h.keys("Down Down f Up Up");
    assert!(shows(&h, "f reset"));
    h.keys("f");
    assert!(shows(&h, "Tips will show again"));
    assert_eq!(h.game().ctx().storage.read(TIPS_SEEN_KEY), Ok(None));
}

#[test]
fn restore_defaults_asks_first_and_keeps_the_layout_and_the_keys() {
    let chord = |s: &str| Chord::parse(s).unwrap_or_else(|e| panic!("{e}"));
    let mut h = options();
    let mut keys = h.game().ctx().layout_bindings(Layout::RightHanded);
    assert_eq!(keys.bind(Action::Confirm, 1, chord("g")), Ok(None));
    let ctx = h.ctx_mut();
    assert_eq!(
        ctx.set_layout_bindings(Layout::RightHanded, keys.clone()),
        Ok(())
    );
    h.keys("Right Down Right").keys(TO_CURSOR).keys("f");
    let changed = settings(&h);
    assert_ne!(
        changed,
        Settings::default().with_layout(Layout::RightHanded)
    );
    // Restore defaults is the last row: round from Music volume's
    // neighbour by going up from the first.
    h.keys("Up Up Up Up Up Up Up Up f");
    assert!(
        shows(&h, "Restore every option to its default?"),
        "{}",
        h.snapshot()
    );
    assert_snapshot!(h.snapshot());
    h.keys("d");
    assert_eq!(settings(&h), changed);
    h.keys("f f");
    assert!(shows(&h, "Defaults restored"));
    assert_eq!(
        settings(&h),
        Settings::default().with_layout(Layout::RightHanded)
    );
    // Custom keys are reset on the Key bindings screen, not here.
    assert_eq!(h.game().ctx().layout_bindings(Layout::RightHanded), keys);
    let h = relaunch(h);
    assert_eq!(
        settings(&h),
        Settings::default().with_layout(Layout::RightHanded)
    );
}

/// A layout picked before the settings held it (its own storage key) is
/// still the layout on the next launch.
#[test]
fn a_layout_saved_by_an_older_build_is_kept() {
    let mut storage = trpg_ui::MemoryStorage::new();
    trpg_ui::Storage::write(&mut storage, "layout", "LeftHanded").ok();
    let mut h = Harness::with_storage(Box::new(storage));
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(h.game().ctx().layout(), Some(Layout::LeftHanded));
    // Changing an option keeps it.
    h.keys("s s j d k");
    let h = relaunch(h);
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(settings(&h).layout(), Some(Layout::LeftHanded));
    assert_eq!(settings(&h).text_speed, TextSpeed::Fast);
}

#[test]
fn every_glyph_drawn_is_in_the_font() {
    let font = trpg_content::FontAtlasDef::load().unwrap_or_default();
    let mut h = options_in_battle();
    for script in ["", TO_MUSIC, "Up Up Up Up f"] {
        h.keys(script);
        let snap = h.snapshot();
        let glyphs = snap.split("\n--- colours ---").next().unwrap_or("");
        for g in glyphs.chars().filter(|&c| c != '\n') {
            assert!(font.glyph_rect(g).is_some(), "{g:?} missing from the font");
        }
    }
}

/// The help line fits its row on every controller too (button names are
/// longer than keys).
#[test]
fn the_help_line_fits_on_every_pad() {
    use trpg_ui::input::PadKind;
    for kind in [
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::Nintendo,
        PadKind::default(),
    ] {
        let mut h = options_in_battle();
        h.use_pad(kind);
        // Onto Game mode, whose hint is the longest.
        h.pad("DpadUp DpadUp DpadUp");
        let help = h.snapshot().lines().nth(31).unwrap_or("").trim().to_owned();
        assert!(help.contains("switch to Casual"), "{kind:?}: {help}");
        assert!(help.contains("back"), "{kind:?}: {help}");
        assert!(help.chars().count() <= 100, "{kind:?}: {help}");
    }
}
