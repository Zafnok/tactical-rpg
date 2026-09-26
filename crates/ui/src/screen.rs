//! Screens and the screen stack: the UI architecture every screen uses.
//! See `crates/ui/README.md` for how to add a screen.
//!
//! A [`Screen`] reads the frame's [`Action`]s in [`update`](Screen::update)
//! and answers with a [`Transition`]; it paints itself into a
//! [`GlyphBuffer`] in [`draw`](Screen::draw). The [`ScreenStack`] routes
//! input to the top screen only and draws from the top-most opaque screen up,
//! so overlays (menus, dialogs) show the screen below them.

use std::fmt;

use trpg_content::{Content, ContentErrors};

use crate::color::Palette;
use crate::glyph_buffer::GlyphBuffer;
use crate::input::{Action, Keymap, Layout};
use crate::storage::{MemoryStorage, Storage, StorageError};

/// [`Storage`] key under which the chosen [`Layout`] is saved (its name,
/// e.g. `LeftHanded`, which is also valid RON for the enum).
pub const LAYOUT_KEY: &str = "layout";

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
}

impl FrameInput {
    /// Input for one frame; `held` lists the actions whose keys are down.
    pub fn new(actions: Vec<Action>, dt: f32, held: Vec<Action>) -> Self {
        Self { actions, dt, held }
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
    /// layout's, or [`Keymap::layout_picker`] until one is chosen. Change it
    /// with [`use_layout`](Self::use_layout) or
    /// [`choose_layout`](Self::choose_layout), never directly.
    pub keymap: Keymap,
    /// The layout in use; `None` until the player has picked one.
    layout: Option<Layout>,
    /// Where saves and settings persist (0207): files on native,
    /// `localStorage` on web. Defaults to [`MemoryStorage`]; `app` swaps in
    /// the platform implementation with [`Ctx::with_storage`].
    pub storage: Box<dyn Storage>,
    /// Whether debug tools are offered: the glyph sampler key and the title
    /// screen's Quick Battle. On in debug builds; the test harness turns it
    /// on everywhere so tests don't depend on the build profile.
    pub debug_tools: bool,
}

impl Ctx {
    /// Builds the shared context from loaded content. Fails with the names
    /// of any UI colours the palette lacks. Starts with [`MemoryStorage`]
    /// and no layout chosen (so [`Keymap::layout_picker`] is active).
    pub fn new(content: Content) -> Result<Self, LoadError> {
        let palette = Palette::new(&content.palette).map_err(LoadError::Palette)?;
        let keymap = Keymap::layout_picker(content.keymap.repeat);
        Ok(Self {
            content,
            palette,
            keymap,
            layout: None,
            storage: Box::new(MemoryStorage::new()),
            debug_tools: cfg!(debug_assertions),
        })
    }

    /// The context for the content embedded in the binary.
    pub fn embedded() -> Result<Self, LoadError> {
        Self::new(trpg_content::load_embedded().map_err(LoadError::Content)?)
    }

    /// The layout in use, or `None` if the player hasn't picked one yet.
    pub fn layout(&self) -> Option<Layout> {
        self.layout
    }

    /// Switches to `layout`'s bindings for this session, without saving.
    pub fn use_layout(&mut self, layout: Layout) {
        self.keymap = Keymap::for_layout(&self.content.keymap, layout);
        self.layout = Some(layout);
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
    /// [`MemoryStorage`]; `app` installs the platform implementation).
    #[must_use]
    pub fn with_storage(mut self, storage: Box<dyn Storage>) -> Self {
        self.storage = storage;
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
    fn a_new_ctx_has_no_layout_and_the_picker_keys() {
        let c = Ctx::embedded().unwrap();
        assert_eq!(c.layout(), None);
        assert_eq!(c.keymap, Keymap::layout_picker(c.content.keymap.repeat));
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
