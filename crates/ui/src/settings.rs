//! The player's settings (ticket 0805): text and animation speeds, combat
//! animations, auto-end, fullscreen, the layout picked, the cursor style
//! and the volumes. Saved as RON under [`SETTINGS_KEY`]; [`Ctx`] loads them
//! with its storage and saves them on every change
//! ([`Ctx::change_settings`]). Key bindings are saved apart (ADR-0031).
//!
//! [`Ctx`]: crate::screen::Ctx
//! [`Ctx::change_settings`]: crate::screen::Ctx::change_settings

use serde::{Deserialize, Serialize};

use crate::input::Layout;
use crate::map_view::CursorStyle;

/// [`Storage`](crate::storage::Storage) key under which the settings are
/// saved ([`Settings::to_ron`]).
pub const SETTINGS_KEY: &str = "settings";

/// The saved format's version.
pub const SETTINGS_VERSION: u32 = 1;

/// The loudest volume setting; 0 is silent.
pub const MAX_VOLUME: u8 = 10;

/// The volume settings' default. *Tunable.*
pub const DEFAULT_VOLUME: u8 = 8;

/// How much faster animations play at [`AnimSpeed::Fast`]. *Tunable.*
pub const FAST_ANIM: f32 = 2.0;

/// How much faster the Enemy and Other phases play at
/// [`EnemyPhaseSpeed::Fast`], on top of the animation speed. *Tunable.*
pub const FAST_ENEMY_PHASE: f32 = 2.0;

/// How fast dialogue text is revealed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextSpeed {
    /// Half the normal speed.
    Slow,
    /// The speed the game always had.
    #[default]
    Normal,
    /// Twice the normal speed.
    Fast,
    /// A whole page at once.
    Instant,
}

impl TextSpeed {
    /// Every speed, slowest first.
    pub const ALL: [Self; 4] = [Self::Slow, Self::Normal, Self::Fast, Self::Instant];

    /// Characters revealed per second (*tunable*); `None` shows a page at
    /// once.
    pub const fn chars_per_s(self) -> Option<f32> {
        match self {
            Self::Slow => Some(30.0),
            Self::Normal => Some(60.0),
            Self::Fast => Some(120.0),
            Self::Instant => None,
        }
    }

    /// The name the Options screen shows.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Slow => "Slow",
            Self::Normal => "Normal",
            Self::Fast => "Fast",
            Self::Instant => "Instant",
        }
    }
}

/// How fast battle animations play: walks, a fight's playback, the EXP bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnimSpeed {
    /// The speed the game always had.
    #[default]
    Normal,
    /// [`FAST_ANIM`] times as fast.
    Fast,
}

impl AnimSpeed {
    /// Every speed, slowest first.
    pub const ALL: [Self; 2] = [Self::Normal, Self::Fast];

    /// The multiplier on animation time.
    pub const fn factor(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Fast => FAST_ANIM,
        }
    }

    /// The name the Options screen shows.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Fast => "Fast",
        }
    }
}

/// How fast the Enemy and Other phases play.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyPhaseSpeed {
    /// The speed the game always had.
    #[default]
    Normal,
    /// [`FAST_ENEMY_PHASE`] times as fast.
    Fast,
}

impl EnemyPhaseSpeed {
    /// Every speed, slowest first.
    pub const ALL: [Self; 2] = [Self::Normal, Self::Fast];

    /// The multiplier on animation time in the AI's phases.
    pub const fn factor(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Fast => FAST_ENEMY_PHASE,
        }
    }

    /// The name the Options screen shows.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Fast => "Fast",
        }
    }
}

/// Everything the Options screen sets. The defaults are how the game
/// behaved before it had options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// How fast dialogue text is revealed.
    pub text_speed: TextSpeed,
    /// How fast battle animations play.
    pub anim_speed: AnimSpeed,
    /// Whether a fight's playback is shown; off, every fight is skipped as
    /// the Cancel key skips one.
    pub combat_animations: bool,
    /// How fast the Enemy and Other phases play.
    pub enemy_phase_speed: EnemyPhaseSpeed,
    /// Auto-end: the player phase ends when its last unit has acted
    /// (`docs/design/turn-structure.md`; off by default). The Auto-end key
    /// in battle flips it too.
    pub auto_end_turn: bool,
    /// Whether the game fills the screen.
    pub fullscreen: bool,
    /// How the battle cursor is drawn (`docs/design/look-and-feel.md`).
    pub cursor_style: CursorStyle,
    /// Music volume, 0 (silent) to [`MAX_VOLUME`].
    pub music_volume: u8,
    /// Sound volume, 0 (silent) to [`MAX_VOLUME`].
    pub sound_volume: u8,
    /// The layout the player picked; `None` until they have. Changed only
    /// by [`Ctx::choose_layout`](crate::screen::Ctx::choose_layout), which
    /// also switches the keys.
    pub(crate) layout: Option<Layout>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            text_speed: TextSpeed::default(),
            anim_speed: AnimSpeed::default(),
            combat_animations: true,
            enemy_phase_speed: EnemyPhaseSpeed::default(),
            auto_end_turn: false,
            fullscreen: false,
            cursor_style: CursorStyle::default(),
            music_volume: DEFAULT_VOLUME,
            sound_volume: DEFAULT_VOLUME,
            layout: None,
        }
    }
}

/// The saved form. A field missing from the text takes its default, so an
/// older save loads.
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "Settings")]
struct SettingsFile {
    version: u32,
    text_speed: TextSpeed,
    anim_speed: AnimSpeed,
    combat_animations: bool,
    enemy_phase_speed: EnemyPhaseSpeed,
    auto_end_turn: bool,
    fullscreen: bool,
    /// [`Layout::name`].
    layout: Option<String>,
    cursor_style: CursorStyle,
    music_volume: u8,
    sound_volume: u8,
}

impl Default for SettingsFile {
    fn default() -> Self {
        Settings::default().file()
    }
}

/// A volume setting as the multiplier `app` applies to every cue's own
/// volume: 0 at 0, 1 at [`MAX_VOLUME`].
fn volume_factor(volume: u8) -> f32 {
    f32::from(volume.min(MAX_VOLUME)) / f32::from(MAX_VOLUME)
}

impl Settings {
    /// The layout the player picked, if they have.
    pub fn layout(&self) -> Option<Layout> {
        self.layout
    }

    /// The same settings with `layout` as the one picked.
    #[must_use]
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// The defaults, keeping what isn't a preference with a default: the
    /// layout picked (the Options screen's "Restore defaults").
    #[must_use]
    pub fn restored(&self) -> Self {
        Self {
            layout: self.layout,
            ..Self::default()
        }
    }

    /// The multiplier on the music's volume, 0–1.
    pub fn music_factor(&self) -> f32 {
        volume_factor(self.music_volume)
    }

    /// The multiplier on every sound's volume, 0–1.
    pub fn sound_factor(&self) -> f32 {
        volume_factor(self.sound_volume)
    }

    /// The multiplier on a battle's animation time: the animation speed,
    /// and in the AI's phases (`ai_phase`) the enemy phase speed too.
    pub fn battle_speed(&self, ai_phase: bool) -> f32 {
        let phase = if ai_phase {
            self.enemy_phase_speed.factor()
        } else {
            1.0
        };
        self.anim_speed.factor() * phase
    }

    fn file(&self) -> SettingsFile {
        SettingsFile {
            version: SETTINGS_VERSION,
            text_speed: self.text_speed,
            anim_speed: self.anim_speed,
            combat_animations: self.combat_animations,
            enemy_phase_speed: self.enemy_phase_speed,
            auto_end_turn: self.auto_end_turn,
            fullscreen: self.fullscreen,
            layout: self.layout.map(|l| l.name().to_owned()),
            cursor_style: self.cursor_style,
            music_volume: self.music_volume,
            sound_volume: self.sound_volume,
        }
    }

    /// The saved form (RON).
    pub fn to_ron(&self) -> String {
        let config = ron::ser::PrettyConfig::new().struct_names(true);
        // Numbers, booleans, names and options always serialise.
        ron::ser::to_string_pretty(&self.file(), config).unwrap_or_default()
    }

    /// Reads the saved form. Text that can't be read, or of another
    /// version, gives `Err` with why (the caller then uses the defaults);
    /// a missing field takes its default, an unknown layout counts as none
    /// picked and a volume over [`MAX_VOLUME`] is brought down to it.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let file: SettingsFile = ron::from_str(text).map_err(|e| format!("unreadable: {e}"))?;
        if file.version != SETTINGS_VERSION {
            let version = file.version;
            return Err(format!("version {version} isn't {SETTINGS_VERSION}"));
        }
        Ok(Self {
            text_speed: file.text_speed,
            anim_speed: file.anim_speed,
            combat_animations: file.combat_animations,
            enemy_phase_speed: file.enemy_phase_speed,
            auto_end_turn: file.auto_end_turn,
            fullscreen: file.fullscreen,
            cursor_style: file.cursor_style,
            music_volume: file.music_volume.min(MAX_VOLUME),
            sound_volume: file.sound_volume.min(MAX_VOLUME),
            layout: file.layout.as_deref().and_then(Layout::from_name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed() -> Settings {
        Settings {
            text_speed: TextSpeed::Instant,
            anim_speed: AnimSpeed::Fast,
            combat_animations: false,
            enemy_phase_speed: EnemyPhaseSpeed::Fast,
            auto_end_turn: true,
            fullscreen: true,
            cursor_style: CursorStyle::TileGlow,
            music_volume: 0,
            sound_volume: 10,
            layout: Some(Layout::LeftHanded),
        }
    }

    #[test]
    fn defaults_are_how_the_game_behaved_before() {
        let s = Settings::default();
        assert_eq!(s.text_speed.chars_per_s(), Some(60.0));
        assert!((s.battle_speed(false) - 1.0).abs() < f32::EPSILON);
        assert!((s.battle_speed(true) - 1.0).abs() < f32::EPSILON);
        assert!(s.combat_animations && !s.auto_end_turn && !s.fullscreen);
        assert_eq!(s.cursor_style, CursorStyle::Corners);
        assert_eq!((s.music_volume, s.sound_volume), (8, 8));
        assert_eq!(s.layout(), None);
    }

    #[test]
    fn settings_round_trip_through_ron() {
        for s in [Settings::default(), changed()] {
            assert_eq!(Settings::from_ron(&s.to_ron()), Ok(s));
        }
        let text = changed().to_ron();
        assert!(text.starts_with("Settings("), "{text}");
        assert!(text.contains("layout: Some(\"LeftHanded\")"), "{text}");
    }

    #[test]
    fn missing_fields_take_their_defaults() {
        let s = Settings::from_ron("Settings(version: 1, sound_volume: 3)").unwrap();
        let expected = Settings {
            sound_volume: 3,
            ..Settings::default()
        };
        assert_eq!(s, expected);
    }

    #[test]
    fn bad_text_and_other_versions_are_refused() {
        let unreadable = Settings::from_ron("nonsense").unwrap_err();
        assert!(unreadable.starts_with("unreadable: "), "{unreadable}");
        assert_eq!(
            Settings::from_ron("Settings(version: 2)"),
            Err("version 2 isn't 1".to_owned())
        );
    }

    #[test]
    fn odd_values_are_repaired() {
        let text =
            "Settings(version: 1, music_volume: 200, sound_volume: 11, layout: Some(\"Vim\"))";
        let s = Settings::from_ron(text).unwrap();
        assert_eq!((s.music_volume, s.sound_volume), (10, 10));
        assert_eq!(s.layout(), None);
    }

    #[test]
    fn volumes_are_factors_from_silent_to_full() {
        let mut s = Settings::default();
        assert!((s.music_factor() - 0.8).abs() < 1e-6);
        assert!((s.sound_factor() - 0.8).abs() < 1e-6);
        s.music_volume = 0;
        s.sound_volume = 10;
        assert!(s.music_factor().abs() < f32::EPSILON);
        assert!((s.sound_factor() - 1.0).abs() < f32::EPSILON);
        // Out of range (set in code) is still at most full volume.
        s.sound_volume = 200;
        assert!((s.sound_factor() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn speeds_multiply_in_the_ai_phases_only() {
        let mut s = Settings {
            anim_speed: AnimSpeed::Fast,
            ..Settings::default()
        };
        assert!((s.battle_speed(false) - 2.0).abs() < f32::EPSILON);
        assert!((s.battle_speed(true) - 2.0).abs() < f32::EPSILON);
        s.enemy_phase_speed = EnemyPhaseSpeed::Fast;
        assert!((s.battle_speed(false) - 2.0).abs() < f32::EPSILON);
        assert!((s.battle_speed(true) - 4.0).abs() < f32::EPSILON);
        s.anim_speed = AnimSpeed::Normal;
        assert!((s.battle_speed(true) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn text_speeds_double_each_step_and_instant_has_none() {
        let speeds: Vec<_> = TextSpeed::ALL.iter().map(|s| s.chars_per_s()).collect();
        assert_eq!(speeds, [Some(30.0), Some(60.0), Some(120.0), None]);
        let labels: Vec<_> = TextSpeed::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels, ["Slow", "Normal", "Fast", "Instant"]);
        assert_eq!(AnimSpeed::ALL.map(AnimSpeed::label), ["Normal", "Fast"]);
        assert_eq!(
            EnemyPhaseSpeed::ALL.map(EnemyPhaseSpeed::label),
            ["Normal", "Fast"]
        );
    }

    #[test]
    fn restoring_keeps_the_layout() {
        let restored = changed().restored();
        assert_eq!(
            restored,
            Settings::default().with_layout(Layout::LeftHanded)
        );
    }
}
