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
use crate::input::{Action, Keymap};

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
/// tickets need them (settings, storage, …).
#[derive(Debug, Clone)]
pub struct Ctx {
    /// All validated game content.
    pub content: Content,
    /// Named colours, from `content.palette`.
    pub palette: Palette,
    /// The active key bindings, for help text that names keys.
    pub keymap: Keymap,
}

impl Ctx {
    /// Builds the shared context from loaded content. Fails with the names
    /// of any UI colours the palette lacks.
    pub fn new(content: Content) -> Result<Self, LoadError> {
        let palette = Palette::new(&content.palette).map_err(LoadError::Palette)?;
        let keymap = Keymap::from_def(&content.keymap);
        Ok(Self {
            content,
            palette,
            keymap,
        })
    }

    /// The context for the content embedded in the binary.
    pub fn embedded() -> Result<Self, LoadError> {
        Self::new(trpg_content::load_embedded().map_err(LoadError::Content)?)
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

    /// The context for the embedded content.
    pub(crate) fn ctx() -> Ctx {
        Ctx::embedded().unwrap()
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
        assert_eq!(c.keymap, Keymap::from_def(&c.content.keymap));
        assert_eq!(c.palette.get(UiColor::Black), Rgb::new(0, 0, 0));
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
