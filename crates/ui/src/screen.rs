//! Screens and the screen stack: the UI architecture every screen uses.
//! See `crates/ui/README.md` for how to add a screen.
//!
//! A [`Screen`] reads the frame's [`Action`]s in [`update`](Screen::update)
//! and answers with a [`Transition`]; it paints itself into a
//! [`GlyphBuffer`] in [`draw`](Screen::draw). The [`ScreenStack`] routes
//! input to the top screen only and draws from the top-most opaque screen up,
//! so overlays (menus, dialogs) show the screen below them.

use std::any::Any;
use std::fmt::{self, Display};
use std::rc::Rc;

use trpg_content::lang::TEST;
use trpg_content::{Content, ContentErrors, LangCode};
use trpg_core::lead::DEFAULT_NAME;
use trpg_core::{LeadGender, LeadProfile};

use crate::audio::{AudioQueue, MusicClock, pick_from_pool};
use crate::color::Palette;
use crate::glyph_buffer::GlyphBuffer;
use crate::input::{
    Action, Button, Chord, Device, Keymap, Layout, LayoutBindings, PadBindings, PlayerKeys,
};
use crate::map_view::{CursorStyle, MapSkin};
use crate::storage::{MemoryStorage, Storage, StorageError};
use crate::tips::fill_text;
use crate::widgets::help::HelpKeys;

/// [`Storage`] key under which the chosen [`Layout`] is saved (its name,
/// e.g. `LeftHanded`, which is also valid RON for the enum).
pub const LAYOUT_KEY: &str = "layout";

/// [`Storage`] key under which the player's key and controller-button
/// bindings are saved ([`PlayerKeys::to_ron`], ADR-0031, ADR-0050).
pub const KEYBINDINGS_KEY: &str = "keybindings";

/// Whether this build offers debug tools: debug builds, and release builds
/// with the `debug-tools` feature (ADR-0023).
pub const DEBUG_TOOLS: bool = cfg!(any(debug_assertions, feature = "debug-tools"));

/// Default dialogue [`Ctx::text_speed`], in characters per second.
pub const DEFAULT_TEXT_SPEED: f32 = 60.0;

/// One screen of the game: title, battle map, a menu overlay, …
pub trait Screen {
    /// A stable, unique `snake_case` name, for tests and debugging.
    fn name(&self) -> &'static str;

    /// Handles one frame: `input` holds this frame's actions (often none)
    /// and elapsed time. Called only while this screen is on top.
    fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition;

    /// Paints the screen. An opaque screen must cover the whole buffer.
    fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer);

    /// Whether the screen below shows through (drawn first, then this one).
    fn is_overlay(&self) -> bool {
        false
    }

    /// This screen as [`Any`], for tests that look inside a screen on the
    /// stack ([`ScreenStack::find`]). `None` unless the screen opts in.
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }

    /// [`as_any`](Self::as_any), mutable ([`ScreenStack::find_mut`]).
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

/// What a screen asks the stack to do after an update.
pub enum Transition {
    /// Stay on this screen.
    None,
    /// Put a new screen on top of this one.
    Push(Box<dyn Screen>),
    /// Remove this screen, returning to the one below (or quitting if it was
    /// the last).
    Pop,
    /// Swap this screen for another.
    Replace(Box<dyn Screen>),
    /// Quit the game.
    Quit,
}

impl fmt::Debug for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Push(s) => write!(f, "Push({})", s.name()),
            Self::Pop => f.write_str("Pop"),
            Self::Replace(s) => write!(f, "Replace({})", s.name()),
            Self::Quit => f.write_str("Quit"),
        }
    }
}

/// One frame's input as a screen sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameInput {
    /// Actions this frame, in order: key presses, then key repeats.
    pub actions: Vec<Action>,
    /// Seconds since the previous frame.
    pub dt: f32,
    /// Actions whose key is currently held down.
    held: Vec<Action>,
    /// The chords pressed this frame, in order.
    pressed: Vec<Chord>,
    /// The characters typed this frame, in order.
    text: Vec<char>,
    /// Whether a controller button went down this frame.
    pad: bool,
    /// The controller buttons that went down this frame, in order.
    buttons_down: Vec<Button>,
    /// The controller buttons that went up this frame, in order.
    buttons_up: Vec<Button>,
}

impl FrameInput {
    /// Input for one frame; `held` lists the actions whose keys are down.
    /// No [`pressed_chords`](Self::pressed_chords).
    pub fn new(actions: Vec<Action>, dt: f32, held: Vec<Action>) -> Self {
        Self {
            actions,
            dt,
            held,
            pressed: Vec::new(),
            text: Vec::new(),
            pad: false,
            buttons_down: Vec::new(),
            buttons_up: Vec::new(),
        }
    }

    /// The same input with the controller buttons that went `down` and
    /// `up` this frame (and so [`pad_pressed`](Self::pad_pressed) if any
    /// went down).
    #[must_use]
    pub fn with_buttons(mut self, down: Vec<Button>, up: Vec<Button>) -> Self {
        self.pad = !down.is_empty();
        self.buttons_down = down;
        self.buttons_up = up;
        self
    }

    /// The controller buttons that went down this frame (bound or not; by
    /// binding position, stick directions included), for the Key bindings
    /// screen, which captures the button for a slot. Anything else reacts
    /// to [`actions`](Self::actions).
    pub fn pressed_buttons(&self) -> &[Button] {
        &self.buttons_down
    }

    /// The controller buttons that went up this frame, as
    /// [`pressed_buttons`](Self::pressed_buttons).
    pub fn released_buttons(&self) -> &[Button] {
        &self.buttons_up
    }

    /// The same input with whether a controller button went down this
    /// frame.
    #[must_use]
    pub fn with_pad(mut self, pad: bool) -> Self {
        self.pad = pad;
        self
    }

    /// Whether a controller button went down this frame: the player is on
    /// a controller, so a screen that needs text shows a letter grid
    /// instead of asking them to type.
    pub fn pad_pressed(&self) -> bool {
        self.pad
    }

    /// The same input with the chords `pressed` and the characters `text`
    /// typed this frame.
    #[must_use]
    pub fn with_typing(mut self, pressed: Vec<Chord>, text: Vec<char>) -> Self {
        self.pressed = pressed;
        self.text = text;
        self
    }

    /// The chords pressed this frame (bound or not; presses only, no
    /// repeats), for the few screens that read keys themselves: a text
    /// box's fixed keys ([`crate::input::text_key`]) and the Key bindings
    /// screen, which captures the key for a slot.
    /// Anything else reacts to [`actions`](Self::actions).
    pub fn pressed_chords(&self) -> &[Chord] {
        &self.pressed
    }

    /// The characters typed this frame (printable only), for text boxes.
    pub fn text(&self) -> &[char] {
        &self.text
    }

    /// Whether a key bound to `action` is held (e.g. hold Confirm to
    /// fast-forward).
    pub fn is_held(&self, action: Action) -> bool {
        self.held.contains(&action)
    }
}

/// Resources shared by every screen. A plain struct: add fields as later
/// tickets need them (settings, …).
#[derive(Debug)]
pub struct Ctx {
    /// All validated game content.
    pub content: Content,
    /// Named colours, from `content.palette`.
    pub palette: Palette,
    /// The active key bindings, for help text that names keys: the chosen
    /// layout's with the player's changes, or [`Keymap::layout_picker`]
    /// until one is chosen. Change it with [`use_layout`](Self::use_layout),
    /// [`choose_layout`](Self::choose_layout) or
    /// [`set_layout_bindings`](Self::set_layout_bindings), never directly.
    pub keymap: Keymap,
    /// What the player pressed last, the keyboard or a controller, so help
    /// text names keys or that pad's buttons ([`help_keys`]). `Game` keeps
    /// it up to date from the input.
    ///
    /// [`help_keys`]: Self::help_keys
    pub device: Device,
    /// The layout in use; `None` until the player has picked one.
    layout: Option<Layout>,
    /// The player's key bindings for every layout and their controller
    /// buttons, loaded from `storage`.
    player_keys: PlayerKeys,
    /// Problems found while loading saved data, for `app` to log.
    warnings: Vec<String>,
    /// Where saves and settings persist (0207): files on native,
    /// `localStorage` on web. Defaults to [`MemoryStorage`]; `app` swaps in
    /// the platform implementation with [`Ctx::with_storage`].
    pub storage: Box<dyn Storage>,
    /// Whether debug tools are offered: the debug menu key and the title
    /// screen's Quick Battle. On in debug builds and with the `debug-tools`
    /// feature (the Pages build, ADR-0023); the test harness turns it on
    /// everywhere so tests don't depend on the build profile.
    pub debug_tools: bool,
    /// How the battle cursor is drawn (corner marks unless the player picks
    /// an accessibility style). Lives here until the Options menu (0805)
    /// moves it into the saved settings.
    pub cursor_style: CursorStyle,
    /// How battle maps look (ADR-0038): the glyph skin, unless a debug tool
    /// swaps in another. Screens build a `MapScene` and this paints it.
    pub map_skin: Rc<dyn MapSkin>,
    /// Whether battles show their one-time tips (0406). Off here, so
    /// screen tests aren't interrupted by them; `app` turns it on, and the
    /// Options menu (0805) will let the player switch it.
    pub tips_enabled: bool,
    /// How fast dialogue text is revealed, in characters per second. Lives
    /// here until the Options menu (0805) moves it into the saved settings.
    pub text_speed: f32,
    /// Sounds and music screens ask for this frame (ADR-0026), e.g.
    /// `ctx.audio.play_sound("menu_move")`. `Game` passes them to `app`.
    pub audio: AudioQueue,
    /// Where the music that is sounding is in its track, for a screen that
    /// keeps time with it (ADR-0037). `None` in silence: nothing asked
    /// for, the track still loading (a fraction of a second to several
    /// seconds), its file missing, or a track played once that has ended.
    /// During a fade it is the track fading out. `app` (or the test
    /// `Harness`) reports it each frame through
    /// [`Game::set_music_playing`](crate::Game::set_music_playing).
    pub music_clock: Option<MusicClock>,
    /// Seeds random music picks (e.g. a track from a pool), kept apart
    /// from core's simulation RNG (ADR-0019). A fixed
    /// [`DEFAULT_MUSIC_SEED`] here, so tests are repeatable; `app` sets it
    /// from the clock at startup so each launch picks differently.
    pub music_seed: u64,
    /// Random music picks made so far, mixed into
    /// [`music_seed`](Self::music_seed) so each pick rolls afresh
    /// ([`Ctx::pick_music`]).
    music_picks: u64,
    /// Who the player made the lead, for dialogue's name and pronoun
    /// tokens and the lead's portrait: a placeholder until a campaign
    /// starts (New Game asks the player), then the campaign's
    /// ([`crate::flow`]).
    pub lead: LeadProfile,
    /// Seconds the game has been running: the sum of every frame's time
    /// (`Game` adds it), for the campaign's playtime.
    pub clock_s: f64,
    /// Whether the title waits for a key press before showing its menu
    /// and playing its music (`docs/design/title-screen.md`). Off here;
    /// `app` sets [`KeyPrompt::Waiting`] for the web build, and `Game`
    /// moves it on to [`KeyPrompt::Pressed`] at the first key press.
    pub key_prompt: KeyPrompt,
    /// The language screen text is shown in (ADR-0045): English until the
    /// player picks another (0825). The test pack counts only with
    /// [`debug_tools`](Self::debug_tools) on; anywhere else it is English.
    pub lang: LangCode,
}

/// The web build's "press any key" title prompt ([`Ctx::key_prompt`]):
/// browsers block sound until the player presses a key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyPrompt {
    /// No prompt (native builds).
    #[default]
    Off,
    /// No key pressed yet: the title shows the prompt.
    Waiting,
    /// A key has been pressed (on any screen).
    Pressed,
}

/// [`Ctx::music_seed`] until `app` sets it.
pub const DEFAULT_MUSIC_SEED: u64 = 0;

impl Ctx {
    /// Builds the shared context from loaded content. Fails with the names
    /// of any UI colours the palette lacks. Starts with [`MemoryStorage`]
    /// and no layout chosen (so [`Keymap::layout_picker`] is active).
    pub fn new(content: Content) -> Result<Self, LoadError> {
        let palette = Palette::new(&content.palette).map_err(LoadError::Palette)?;
        let keymap = Keymap::layout_picker(&content.keymap);
        Ok(Self {
            content,
            palette,
            keymap,
            device: Device::default(),
            layout: None,
            player_keys: PlayerKeys::default(),
            warnings: Vec::new(),
            storage: Box::new(MemoryStorage::new()),
            debug_tools: DEBUG_TOOLS,
            cursor_style: CursorStyle::default(),
            map_skin: crate::map_view::default_skin(),
            tips_enabled: false,
            text_speed: DEFAULT_TEXT_SPEED,
            audio: AudioQueue::default(),
            music_clock: None,
            music_seed: DEFAULT_MUSIC_SEED,
            music_picks: 0,
            lead: LeadProfile::new(DEFAULT_NAME, LeadGender::Male),
            clock_s: 0.0,
            key_prompt: KeyPrompt::Off,
            lang: LangCode::english(),
        })
    }

    /// The screen text for `key` (`title.new_game`) in the player's
    /// language, as written in `assets/lang/`, placeholders and all: the
    /// pack's text if it has one made from today's English, else English.
    ///
    /// A key English lacks is a bug in the screen: it panics in debug
    /// builds, and shows as the key itself in release builds.
    pub fn text<'a>(&'a self, key: &'a str) -> &'a str {
        let lang = &self.content.lang;
        debug_assert!(
            lang.has(key),
            "text key not in assets/lang/en/ui.ron: {key:?}"
        );
        if self.lang.as_str() == TEST && !self.debug_tools {
            return lang.text(&LangCode::english(), key);
        }
        lang.text(&self.lang, key)
    }

    /// [`text`](Self::text) with its placeholders filled in: each `{name}`
    /// that `args` names by its value, and each `{Action}` (and `{Cursor}`)
    /// by the key the player has for it ([`help_keys`](Self::help_keys)).
    pub fn text_with(&self, key: &str, args: &[(&str, &dyn Display)]) -> String {
        fill_text(self.text(key), self.help_keys(), args)
    }

    /// The context for the content embedded in the binary.
    pub fn embedded() -> Result<Self, LoadError> {
        Self::new(trpg_content::load_embedded().map_err(LoadError::Content)?)
    }

    /// A music cue picked at random from the audio manifest's `pool`
    /// ([`pick_from_pool`]): the n-th pick of this run uses
    /// [`music_seed`](Self::music_seed) mixed with n, so a battle started
    /// again may get another track. `None` if there is no such pool or it
    /// is empty.
    pub fn pick_music(&mut self, pool: &str) -> Option<String> {
        let seed = self.music_seed ^ self.music_picks;
        self.music_picks = self.music_picks.wrapping_add(1);
        pick_from_pool(&self.content.audio, pool, seed).map(str::to_owned)
    }

    /// What help text names keys from: the active bindings on the device
    /// the player pressed last.
    pub fn help_keys(&self) -> HelpKeys<'_> {
        HelpKeys::new(&self.keymap, self.device)
    }

    /// The layout in use, or `None` if the player hasn't picked one yet.
    pub fn layout(&self) -> Option<Layout> {
        self.layout
    }

    /// Switches to `layout`'s bindings (the player's own for that layout,
    /// else its defaults) for this session, without saving the choice.
    pub fn use_layout(&mut self, layout: Layout) {
        self.layout = Some(layout);
        self.refresh_keymap();
    }

    /// Rebuilds [`keymap`](Self::keymap) from the layout in use (the
    /// layout picker's keys while there is none) and the player's
    /// bindings.
    fn refresh_keymap(&mut self) {
        self.keymap = match self.layout {
            Some(layout) => self.keymap_for(layout),
            None => self.player_keys.layout_picker_keymap(&self.content.keymap),
        };
    }

    /// The keymap `layout` would have: the player's bindings for it, else
    /// its defaults. (The layout picker draws both layouts with this.)
    pub fn keymap_for(&self, layout: Layout) -> Keymap {
        self.player_keys.keymap(&self.content.keymap, layout)
    }

    /// The player's key bindings for every layout.
    pub fn player_keys(&self) -> &PlayerKeys {
        &self.player_keys
    }

    /// `layout`'s current bindings, for the Key bindings screen (0815) to
    /// edit and hand back to [`set_layout_bindings`](Self::set_layout_bindings).
    pub fn layout_bindings(&self, layout: Layout) -> LayoutBindings {
        self.player_keys.bindings(&self.content.keymap, layout)
    }

    /// Replaces `layout`'s bindings and saves every layout's under
    /// [`KEYBINDINGS_KEY`]. If `layout` is in use, its keys work from the
    /// next key press (`Game` hands the new keymap to its input at once).
    /// The change applies even if saving fails (it is then lost on quit).
    pub fn set_layout_bindings(
        &mut self,
        layout: Layout,
        bindings: LayoutBindings,
    ) -> Result<(), StorageError> {
        self.player_keys.set(&self.content.keymap, layout, bindings);
        if self.layout == Some(layout) {
            self.use_layout(layout);
        }
        self.storage
            .write(KEYBINDINGS_KEY, &self.player_keys.to_ron())
    }

    /// The player's controller buttons (the defaults until they change
    /// them), for the Key bindings screen (0816) to edit and hand back to
    /// [`set_pad_bindings`](Self::set_pad_bindings). The same in both
    /// layouts.
    pub fn pad_bindings(&self) -> PadBindings {
        self.player_keys.pad_bindings(&self.content.keymap)
    }

    /// Replaces the controller buttons and saves them under
    /// [`KEYBINDINGS_KEY`]. They work from the next press, whatever the
    /// layout (and before one is chosen). The change applies even if
    /// saving fails (it is then lost on quit).
    pub fn set_pad_bindings(&mut self, bindings: PadBindings) -> Result<(), StorageError> {
        self.player_keys.set_pad(&self.content.keymap, bindings);
        self.refresh_keymap();
        self.storage
            .write(KEYBINDINGS_KEY, &self.player_keys.to_ron())
    }

    /// Loads the player's key bindings from `storage` (repairing what it
    /// must, with a warning per fix) and re-applies the layout in use.
    fn load_player_keys(&mut self) {
        let (keys, warnings) = match self.storage.read(KEYBINDINGS_KEY) {
            Ok(Some(text)) => PlayerKeys::from_ron(&text, &self.content.keymap),
            Ok(None) => (PlayerKeys::default(), Vec::new()),
            Err(e) => (
                PlayerKeys::default(),
                vec![format!("can't be read, using the default keys: {e}")],
            ),
        };
        self.player_keys = keys;
        self.warnings.extend(
            warnings
                .into_iter()
                .map(|w| format!("{KEYBINDINGS_KEY}: {w}")),
        );
        self.refresh_keymap();
    }

    /// Takes the warnings gathered while loading saved data (e.g. repaired
    /// key bindings), for `app` to log.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Builder form of [`use_layout`](Self::use_layout).
    #[must_use]
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.use_layout(layout);
        self
    }

    /// The player picked `layout`: switches to it and saves it under
    /// [`LAYOUT_KEY`]. The switch happens even if saving fails (the player
    /// is then asked again next launch).
    pub fn choose_layout(&mut self, layout: Layout) -> Result<(), StorageError> {
        self.use_layout(layout);
        self.storage.write(LAYOUT_KEY, layout.name())
    }

    /// The layout saved by an earlier [`choose_layout`](Self::choose_layout).
    /// `None` if nothing is saved, or if the saved value can't be read or
    /// isn't a known layout (the player is simply asked again).
    pub fn saved_layout(&self) -> Option<Layout> {
        let saved = self.storage.read(LAYOUT_KEY).ok()??;
        Layout::from_name(saved.trim())
    }

    /// Replaces the storage backend (the harness and tests keep
    /// [`MemoryStorage`]; `app` installs the platform implementation) and
    /// loads the player's key bindings from it.
    #[must_use]
    pub fn with_storage(mut self, storage: Box<dyn Storage>) -> Self {
        self.storage = storage;
        self.load_player_keys();
        self
    }
}

/// Why the game's content could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// One or more assets failed to load or validate.
    Content(ContentErrors),
    /// The palette lacks these UI colours.
    Palette(Vec<&'static str>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Content(errors) => write!(f, "{errors}"),
            Self::Palette(missing) => write!(f, "palette lacks {}", missing.join(", ")),
        }
    }
}

impl std::error::Error for LoadError {}

/// The stack of open screens. Only the top screen is updated; drawing starts
/// at the top-most opaque screen and goes up through the overlays above it.
#[derive(Default)]
pub struct ScreenStack {
    screens: Vec<Box<dyn Screen>>,
}

impl ScreenStack {
    /// A stack holding just `root`.
    pub fn new(root: Box<dyn Screen>) -> Self {
        Self {
            screens: vec![root],
        }
    }

    /// Whether no screen is open (the game should quit).
    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }

    /// Names of the open screens, bottom first.
    pub fn names(&self) -> Vec<&'static str> {
        self.screens.iter().map(|s| s.name()).collect()
    }

    /// Name of the top screen.
    pub fn top_name(&self) -> Option<&'static str> {
        self.screens.last().map(|s| s.name())
    }

    /// Puts `screen` on top.
    pub fn push(&mut self, screen: Box<dyn Screen>) {
        self.screens.push(screen);
    }

    /// The top-most screen of type `T` (one that opts in with
    /// [`Screen::as_any`]).
    pub fn find<T: Any>(&self) -> Option<&T> {
        self.screens
            .iter()
            .rev()
            .find_map(|s| s.as_any()?.downcast_ref())
    }

    /// The top-most screen of type `T`, mutable ([`Screen::as_any_mut`]).
    pub fn find_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.screens
            .iter_mut()
            .rev()
            .find_map(|s| s.as_any_mut()?.downcast_mut())
    }

    /// Updates the top screen and applies its transition. Returns `true`
    /// if the game should quit ([`Transition::Quit`], or the last screen
    /// popped). Does nothing and returns `true` on an empty stack.
    pub fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> bool {
        let Some(top) = self.screens.last_mut() else {
            return true;
        };
        let transition = top.update(ctx, input);
        self.apply(transition)
    }

    /// Applies a transition as if the top screen had returned it. Returns
    /// `true` if the game should quit.
    pub fn apply(&mut self, transition: Transition) -> bool {
        match transition {
            Transition::None => {}
            Transition::Push(screen) => self.screens.push(screen),
            Transition::Pop => {
                self.screens.pop();
            }
            Transition::Replace(screen) => {
                self.screens.pop();
                self.screens.push(screen);
            }
            Transition::Quit => return true,
        }
        self.screens.is_empty()
    }

    /// Draws the top-most opaque screen and every overlay above it, bottom
    /// up. If every screen is an overlay, all are drawn.
    pub fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
        let base = self
            .screens
            .iter()
            .rposition(|s| !s.is_overlay())
            .unwrap_or(0);
        for screen in &self.screens[base..] {
            screen.draw(ctx, buf);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::glyph_buffer::Cell;
    use crate::{Rgb, UiColor};

    /// The context for the embedded content, with the right-handed layout
    /// already chosen.
    /// Debug tools are on whatever the build profile, as in the harness.
    pub(crate) fn ctx() -> Ctx {
        let mut ctx = Ctx::embedded().unwrap().with_layout(Layout::RightHanded);
        ctx.debug_tools = true;
        ctx
    }

    /// [`ctx`] with a manifest holding these sound and music cues too.
    /// Each music cue loops and is 10 s long (an existing cue, e.g.
    /// `title`, is replaced).
    pub(crate) fn ctx_with_cues(sounds: &[&str], music: &[&str]) -> Ctx {
        use trpg_content::{CreditRef, MusicCue, SoundCue};
        let mut ctx = ctx();
        let audio = &mut ctx.content.audio;
        for &cue in sounds {
            let files = vec![format!("sfx/{cue}.wav")];
            let sound = SoundCue {
                files,
                volume: 100,
                credit: CreditRef::Own,
            };
            audio.sounds.insert(cue.to_owned(), sound);
        }
        for &cue in music {
            let track = MusicCue {
                file: format!("{cue}.ogg"),
                volume: 100,
                looped: true,
                length_ms: 10_000,
                credit: CreditRef::Own,
            };
            audio.music.insert(cue.to_owned(), track);
        }
        ctx
    }

    type Log = Rc<RefCell<Vec<String>>>;

    /// A test screen that logs calls, answers updates from a script and
    /// draws its tag in the top-left cell.
    struct Probe {
        name: &'static str,
        overlay: bool,
        log: Log,
        next: RefCell<Vec<Transition>>,
    }

    impl Probe {
        fn boxed(name: &'static str, overlay: bool, log: &Log) -> Box<dyn Screen> {
            Box::new(Self::with(name, overlay, log, vec![]))
        }

        fn with(name: &'static str, overlay: bool, log: &Log, next: Vec<Transition>) -> Self {
            Self {
                name,
                overlay,
                log: Rc::clone(log),
                next: RefCell::new(next),
            }
        }
    }

    impl Screen for Probe {
        fn name(&self) -> &'static str {
            self.name
        }

        fn update(&mut self, _: &mut Ctx, input: &FrameInput) -> Transition {
            self.log
                .borrow_mut()
                .push(format!("update {} {:?}", self.name, input.actions));
            let mut next = self.next.borrow_mut();
            if next.is_empty() {
                Transition::None
            } else {
                next.remove(0)
            }
        }

        fn draw(&self, _: &Ctx, buf: &mut GlyphBuffer) {
            self.log.borrow_mut().push(format!("draw {}", self.name));
            let black = Rgb::new(0, 0, 0);
            let glyph = self.name.chars().next().unwrap_or('?');
            buf.set(0, 0, Cell::new(glyph, black, black));
        }

        fn is_overlay(&self) -> bool {
            self.overlay
        }
    }

    fn input(actions: &[Action]) -> FrameInput {
        FrameInput::new(actions.to_vec(), 0.0, vec![])
    }

    fn draws(stack: &ScreenStack, log: &Log) -> Vec<String> {
        log.borrow_mut().clear();
        let ctx = ctx();
        let mut buf = GlyphBuffer::new(1, 1, Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)));
        stack.draw(&ctx, &mut buf);
        log.borrow().clone()
    }

    #[test]
    fn only_the_top_screen_updates() {
        let log = Log::default();
        let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
        stack.push(Probe::boxed("b", false, &log));
        assert!(!stack.update(&mut ctx(), &input(&[Action::Confirm])));
        assert_eq!(*log.borrow(), ["update b [Confirm]"]);
        assert_eq!(stack.names(), ["a", "b"]);
        assert_eq!(stack.top_name(), Some("b"));
    }

    #[test]
    fn transitions_change_the_stack() {
        let log = Log::default();
        let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
        assert!(!stack.apply(Transition::None));
        assert_eq!(stack.names(), ["a"]);
        assert!(!stack.apply(Transition::Push(Probe::boxed("b", false, &log))));
        assert_eq!(stack.names(), ["a", "b"]);
        assert!(!stack.apply(Transition::Replace(Probe::boxed("c", false, &log))));
        assert_eq!(stack.names(), ["a", "c"]);
        assert!(!stack.apply(Transition::Pop));
        assert_eq!(stack.names(), ["a"]);
        assert!(!stack.is_empty());
        assert!(
            stack.apply(Transition::Pop),
            "popping the last screen quits"
        );
        assert!(stack.is_empty());
        assert_eq!(stack.top_name(), None);
    }

    #[test]
    fn quit_keeps_the_stack() {
        let log = Log::default();
        let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
        assert!(stack.apply(Transition::Quit));
        assert_eq!(stack.names(), ["a"]);
    }

    #[test]
    fn update_applies_the_returned_transition() {
        let log = Log::default();
        let root = Probe::with(
            "a",
            false,
            &log,
            vec![Transition::Push(Probe::boxed("b", false, &log))],
        );
        let mut stack = ScreenStack::new(Box::new(root));
        assert!(!stack.update(&mut ctx(), &input(&[])));
        assert_eq!(stack.names(), ["a", "b"]);
        let quitter = Probe::with("q", false, &log, vec![Transition::Quit]);
        stack.push(Box::new(quitter));
        assert!(stack.update(&mut ctx(), &input(&[])));
    }

    #[test]
    fn empty_stack_quits_and_draws_nothing() {
        let log = Log::default();
        let mut stack = ScreenStack::default();
        assert!(stack.is_empty());
        assert!(stack.update(&mut ctx(), &input(&[])));
        assert!(draws(&stack, &log).is_empty());
    }

    #[test]
    fn draw_starts_at_the_top_most_opaque_screen() {
        let log = Log::default();
        let mut stack = ScreenStack::new(Probe::boxed("a", false, &log));
        stack.push(Probe::boxed("b", false, &log));
        assert_eq!(draws(&stack, &log), ["draw b"]);
        stack.push(Probe::boxed("o", true, &log));
        stack.push(Probe::boxed("p", true, &log));
        assert_eq!(draws(&stack, &log), ["draw b", "draw o", "draw p"]);
    }

    #[test]
    fn all_overlays_draw_everything() {
        let log = Log::default();
        let mut stack = ScreenStack::new(Probe::boxed("o", true, &log));
        stack.push(Probe::boxed("p", true, &log));
        assert_eq!(draws(&stack, &log), ["draw o", "draw p"]);
    }

    #[test]
    fn frame_input_held_query() {
        let i = FrameInput::new(vec![Action::Confirm], 0.5, vec![Action::CursorUp]);
        assert!(i.is_held(Action::CursorUp));
        assert!(!i.is_held(Action::Confirm));
        assert_eq!(i.actions, [Action::Confirm]);
        assert!((i.dt - 0.5).abs() < f32::EPSILON);
        assert!(i.pressed_chords().is_empty());
        assert!(i.pressed_buttons().is_empty() && i.released_buttons().is_empty());
        assert!(!i.pad_pressed());
    }

    #[test]
    fn frame_input_carries_the_buttons_that_went_down_and_up() {
        use crate::input::Button::{East, South, West};
        let i = FrameInput::new(vec![], 0.0, vec![]).with_buttons(vec![South, West], vec![East]);
        assert_eq!(i.pressed_buttons(), [South, West]);
        assert_eq!(i.released_buttons(), [East]);
        assert!(i.pad_pressed());
        // A release alone isn't a press.
        let i = FrameInput::new(vec![], 0.0, vec![]).with_buttons(vec![], vec![East]);
        assert!(!i.pad_pressed());
        assert_eq!(i.released_buttons(), [East]);
    }

    #[test]
    fn transition_debug_names_screens() {
        let log = Log::default();
        let dbg = |t: Transition| format!("{t:?}");
        assert_eq!(dbg(Transition::None), "None");
        assert_eq!(
            dbg(Transition::Push(Probe::boxed("a", false, &log))),
            "Push(a)"
        );
        assert_eq!(dbg(Transition::Pop), "Pop");
        assert_eq!(
            dbg(Transition::Replace(Probe::boxed("b", false, &log))),
            "Replace(b)"
        );
        assert_eq!(dbg(Transition::Quit), "Quit");
    }

    #[test]
    fn ctx_uses_the_content_palette_and_keymap() {
        let c = ctx();
        assert_eq!(c.palette, Palette::new(&c.content.palette).unwrap());
        assert_eq!(
            c.keymap,
            Keymap::for_layout(&c.content.keymap, Layout::RightHanded)
        );
        assert_eq!(c.layout(), Some(Layout::RightHanded));
        assert_eq!(c.palette.get(UiColor::Black), Rgb::new(0, 0, 0));
    }

    #[test]
    fn text_comes_from_the_language_in_use() {
        let mut c = ctx();
        assert_eq!(c.lang, LangCode::english());
        assert_eq!(c.text("title.new_game"), "New Game");
        c.lang = LangCode::new(TEST).unwrap();
        assert_eq!(c.text("title.new_game"), "NEW GAME");
        // Stale and missing entries are English.
        assert_eq!(c.text("title.subtitle"), "an ASCII tactics game");
        assert_eq!(c.text("title.credits"), "Credits");
        // The test pack is only for builds with debug tools.
        c.debug_tools = false;
        assert_eq!(c.text("title.new_game"), "New Game");
        // A language with no pack is English.
        c.lang = LangCode::new("zz").unwrap();
        assert_eq!(c.text("title.new_game"), "New Game");
    }

    #[test]
    fn text_with_fills_values_and_key_names() {
        let mut c = ctx();
        assert_eq!(
            c.text_with("title.help", &[]),
            "arrows move · f select · d back"
        );
        c.lang = LangCode::new(TEST).unwrap();
        assert_eq!(
            c.text_with("title.help", &[]),
            "arrows MOVE · f SELECT · d BACK"
        );
        // A value named like a placeholder in the text fills it.
        assert_eq!(
            c.text_with("title.help", &[("Cursor", &7)]),
            "7 MOVE · f SELECT · d BACK"
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "text key not in assets/lang/en/ui.ron: \"title.nope\"")]
    fn an_unknown_text_key_panics_in_debug_builds() {
        let c = ctx();
        let _ = c.text("title.nope");
    }

    #[test]
    fn a_new_ctx_has_no_layout_and_the_picker_keys() {
        let c = Ctx::embedded().unwrap();
        assert_eq!(c.layout(), None);
        assert_eq!(c.keymap, Keymap::layout_picker(&c.content.keymap));
        assert_eq!(c.saved_layout(), None);
    }

    #[test]
    fn use_layout_switches_without_saving() {
        let mut c = ctx();
        c.use_layout(Layout::LeftHanded);
        assert_eq!(c.layout(), Some(Layout::LeftHanded));
        assert_eq!(
            c.keymap,
            Keymap::for_layout(&c.content.keymap, Layout::LeftHanded)
        );
        assert_eq!(c.storage.read(LAYOUT_KEY), Ok(None));
    }

    #[test]
    fn choose_layout_saves_and_saved_layout_reads_it_back() {
        for layout in Layout::ALL {
            let mut c = Ctx::embedded().unwrap();
            assert_eq!(c.choose_layout(layout), Ok(()));
            assert_eq!(c.layout(), Some(layout));
            assert_eq!(
                c.storage.read(LAYOUT_KEY),
                Ok(Some(layout.name().to_owned()))
            );
            assert_eq!(c.saved_layout(), Some(layout));
        }
    }

    #[test]
    fn a_bad_saved_layout_counts_as_none() {
        let mut c = Ctx::embedded().unwrap();
        c.storage.write(LAYOUT_KEY, "Vim").unwrap();
        assert_eq!(c.saved_layout(), None);
        c.storage
            .write(
                LAYOUT_KEY,
                " LeftHanded
",
            )
            .unwrap();
        assert_eq!(c.saved_layout(), Some(Layout::LeftHanded));
    }

    /// A storage whose every operation fails.
    #[derive(Debug)]
    struct Failing;

    impl Storage for Failing {
        fn read(&self, _: &str) -> Result<Option<String>, StorageError> {
            Err(StorageError::Backend("down".into()))
        }
        fn write(&mut self, _: &str, _: &str) -> Result<(), StorageError> {
            Err(StorageError::Backend("down".into()))
        }
        fn delete(&mut self, _: &str) -> Result<(), StorageError> {
            Err(StorageError::Backend("down".into()))
        }
        fn list(&self) -> Result<Vec<String>, StorageError> {
            Err(StorageError::Backend("down".into()))
        }
    }

    #[test]
    fn a_failed_save_still_switches_layout() {
        let mut c = Ctx::embedded().unwrap().with_storage(Box::new(Failing));
        assert_eq!(c.saved_layout(), None);
        assert!(c.choose_layout(Layout::LeftHanded).is_err());
        assert_eq!(c.layout(), Some(Layout::LeftHanded));
    }

    #[test]
    fn unreadable_storage_gives_default_keys_and_a_warning() {
        let mut c = Ctx::embedded().unwrap().with_storage(Box::new(Failing));
        assert_eq!(c.player_keys(), &PlayerKeys::default());
        assert_eq!(
            c.take_warnings(),
            ["keybindings: can't be read, using the default keys: storage error: down"]
        );
        assert!(c.take_warnings().is_empty());
    }

    /// `Info` on `g` instead of its default key.
    fn info_on_g(c: &Ctx, layout: Layout) -> LayoutBindings {
        let mut b = c.layout_bindings(layout);
        let g = crate::input::Chord::plain(crate::input::Key::G);
        assert!(b.bind(Action::Info, 0, g).is_ok());
        b
    }

    #[test]
    fn set_layout_bindings_saves_and_switches_the_layout_in_use() {
        let mut c = ctx();
        let b = info_on_g(&c, Layout::RightHanded);
        assert_eq!(
            c.set_layout_bindings(Layout::RightHanded, b.clone()),
            Ok(())
        );
        assert_eq!(
            c.keymap,
            b.keymap(c.content.keymap.repeat)
                .with_default_pad(&c.content.keymap)
        );
        assert_eq!(c.layout_bindings(Layout::RightHanded), b);
        let saved = c.storage.read(KEYBINDINGS_KEY).unwrap().unwrap();
        assert_eq!(saved, c.player_keys().to_ron());
        // The other layout's keys change only the saved config.
        let before = c.keymap.clone();
        let left = info_on_g(&c, Layout::LeftHanded);
        c.set_layout_bindings(Layout::LeftHanded, left.clone())
            .unwrap();
        assert_eq!(c.keymap, before);
        c.use_layout(Layout::LeftHanded);
        assert_eq!(
            c.keymap,
            left.keymap(c.content.keymap.repeat)
                .with_default_pad(&c.content.keymap)
        );
        assert_eq!(c.keymap_for(Layout::RightHanded), before);
    }

    /// Unit info also on the right trigger.
    fn info_on_right_trigger(c: &Ctx) -> PadBindings {
        let mut b = c.pad_bindings();
        assert!(b.bind(Action::Info, 1, Button::RightTrigger).is_ok());
        b
    }

    #[test]
    fn set_pad_bindings_saves_and_applies_in_every_layout_and_before_one_is_chosen() {
        let mut c = ctx();
        assert_eq!(c.pad_bindings(), PadBindings::defaults(&c.content.keymap));
        let b = info_on_right_trigger(&c);
        assert_eq!(c.set_pad_bindings(b.clone()), Ok(()));
        assert_eq!(c.pad_bindings(), b);
        assert_eq!(
            c.keymap.pad_action(Button::RightTrigger),
            Some(Action::Info)
        );
        let saved = c.storage.read(KEYBINDINGS_KEY).unwrap().unwrap();
        assert_eq!(saved, c.player_keys().to_ron());
        // The keys are the layout's own; the other layout has the buttons too.
        assert_eq!(c.keymap, c.keymap_for(Layout::RightHanded));
        assert_eq!(
            c.keymap.chords_for(Action::Info),
            Keymap::for_layout(&c.content.keymap, Layout::RightHanded).chords_for(Action::Info)
        );
        c.use_layout(Layout::LeftHanded);
        assert_eq!(
            c.keymap.pad_action(Button::RightTrigger),
            Some(Action::Info)
        );
        // Before a layout is chosen: the layout picker's keys, these buttons.
        let mut none = Ctx::embedded().unwrap();
        assert_eq!(none.set_pad_bindings(b.clone()), Ok(()));
        assert_eq!(none.layout(), None);
        assert_eq!(
            none.keymap,
            Keymap::layout_picker(&none.content.keymap).with_pad(b.pairs())
        );
        // They load with the storage, with or without a layout.
        let storage = std::mem::replace(&mut none.storage, Box::new(MemoryStorage::new()));
        let again = Ctx::embedded().unwrap().with_storage(storage);
        assert_eq!(again.pad_bindings(), b);
        assert_eq!(
            again.keymap.pad_action(Button::RightTrigger),
            Some(Action::Info)
        );
        // A failed save still applies them.
        let mut failing = ctx().with_storage(Box::new(Failing));
        assert!(failing.set_pad_bindings(b.clone()).is_err());
        assert_eq!(failing.pad_bindings(), b);
        assert_eq!(
            failing.keymap.pad_action(Button::RightTrigger),
            Some(Action::Info)
        );
    }

    #[test]
    fn saved_keys_load_with_the_storage() {
        let mut c = ctx();
        let b = info_on_g(&c, Layout::RightHanded);
        c.set_layout_bindings(Layout::RightHanded, b.clone())
            .unwrap();
        let storage = std::mem::replace(&mut c.storage, Box::new(MemoryStorage::new()));
        // A layout already in use picks up the loaded keys.
        let mut again = ctx().with_storage(storage);
        assert_eq!(again.layout_bindings(Layout::RightHanded), b);
        assert_eq!(
            again.keymap,
            b.keymap(again.content.keymap.repeat)
                .with_default_pad(&again.content.keymap)
        );
        assert!(again.take_warnings().is_empty());
    }

    #[test]
    fn repaired_keys_leave_a_warning() {
        let mut storage = MemoryStorage::new();
        storage.write(KEYBINDINGS_KEY, "nonsense").unwrap();
        let mut c = Ctx::embedded().unwrap().with_storage(Box::new(storage));
        let warnings = c.take_warnings();
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].starts_with("keybindings: unreadable"),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_failed_save_still_switches_keys() {
        let mut c = Ctx::embedded()
            .unwrap()
            .with_storage(Box::new(Failing))
            .with_layout(Layout::LeftHanded);
        let b = info_on_g(&c, Layout::LeftHanded);
        assert!(
            c.set_layout_bindings(Layout::LeftHanded, b.clone())
                .is_err()
        );
        assert_eq!(
            c.keymap,
            b.keymap(c.content.keymap.repeat)
                .with_default_pad(&c.content.keymap)
        );
    }

    #[test]
    fn ctx_rejects_an_incomplete_palette() {
        let mut content = ctx().content;
        content.palette.colors.remove("text");
        let err = Ctx::new(content).unwrap_err();
        assert_eq!(err, LoadError::Palette(vec!["text"]));
        assert_eq!(err.to_string(), "palette lacks text");
    }

    #[test]
    fn content_error_display() {
        let errors = ContentErrors(vec![trpg_content::ContentError::new("a.ron", "bad")]);
        let err = LoadError::Content(errors.clone());
        assert_eq!(err.to_string(), errors.to_string());
    }
}
