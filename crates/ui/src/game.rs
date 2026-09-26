//! [`Game`]: the whole UI behind one call per frame. `app` feeds it raw key
//! events and the frame time and blits the buffer it returns; the test
//! `Harness` drives it the same way without a window.

use crate::color::UiColor;
use crate::console::{CONSOLE_H, CONSOLE_W};
use crate::debug::GlyphSamplerScreen;
use crate::glyph_buffer::{Cell, GlyphBuffer};
use crate::input::{Action, Chord, InputState, Key, Layout};
use crate::screen::{Ctx, FrameInput, Screen, ScreenStack};
use crate::screens::{LayoutPickerScreen, TitleScreen};

/// A keyboard event as `app` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawKeyEvent {
    /// A key went down, with the Shift state at the time.
    Down(Chord),
    /// A key went up.
    Up(Key),
}

/// The result of one frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameOutput<'a> {
    /// What to show.
    pub buffer: &'a GlyphBuffer,
    /// Whether the game has asked to quit (it then ignores further input).
    pub quit: bool,
}

/// Owns the screens, input state, shared context and the console buffer.
pub struct Game {
    stack: ScreenStack,
    input: InputState,
    /// The layout whose bindings `input` uses, to notice when a screen
    /// switches layout in `ctx`.
    input_layout: Option<Layout>,
    ctx: Ctx,
    buffer: GlyphBuffer,
    quit: bool,
}

impl Game {
    /// A game showing `root`. Debug screens (F12) are on in debug builds.
    pub fn new(ctx: Ctx, root: Box<dyn Screen>) -> Self {
        Self::with_stack(ctx, ScreenStack::new(root))
    }

    /// A game starting at the title screen. If no layout is in use yet, the
    /// saved one is loaded from `ctx.storage`; if none is saved (first
    /// launch), the layout picker opens on top of the title.
    pub fn start(mut ctx: Ctx) -> Self {
        if ctx.layout().is_none()
            && let Some(layout) = ctx.saved_layout()
        {
            ctx.use_layout(layout);
        }
        let title = if ctx.debug_tools {
            TitleScreen::with_quick_battle()
        } else {
            TitleScreen::new()
        };
        let mut stack = ScreenStack::new(Box::new(title));
        if ctx.layout().is_none() {
            stack.push(Box::new(LayoutPickerScreen::new()));
        }
        Self::with_stack(ctx, stack)
    }

    fn with_stack(ctx: Ctx, stack: ScreenStack) -> Self {
        let input = InputState::new(ctx.keymap.clone());
        let input_layout = ctx.layout();
        let blank = Cell::new(
            ' ',
            ctx.palette.get(UiColor::Text),
            ctx.palette.get(UiColor::Black),
        );
        let mut game = Self {
            stack,
            input,
            input_layout,
            ctx,
            buffer: GlyphBuffer::new(CONSOLE_W, CONSOLE_H, blank),
            quit: false,
        };
        game.redraw();
        game
    }

    /// Turns the debug screens (the [`Action::Debug`] key) on or off
    /// ([`Ctx::debug_tools`]).
    #[must_use]
    pub fn with_debug_screens(mut self, on: bool) -> Self {
        self.ctx.debug_tools = on;
        self
    }

    /// Runs one frame: applies `events` (in order), advances input by `dt`
    /// seconds, updates the top screen with the resulting actions and
    /// redraws. After a quit, frames do nothing.
    pub fn frame(&mut self, events: &[RawKeyEvent], dt: f32) -> FrameOutput<'_> {
        if !self.quit {
            self.step(events, dt);
        }
        FrameOutput {
            buffer: &self.buffer,
            quit: self.quit,
        }
    }

    fn step(&mut self, events: &[RawKeyEvent], dt: f32) {
        for &event in events {
            match event {
                RawKeyEvent::Down(chord) => self.input.key_down(chord),
                RawKeyEvent::Up(key) => self.input.key_up(key),
            }
        }
        let actions = self.input.update(dt);
        let held = Action::ALL
            .into_iter()
            .filter(|&a| self.input.is_held(a))
            .collect();
        let opens_sampler = self.ctx.debug_tools
            && actions.contains(&Action::Debug)
            && self.stack.top_name() != Some(GlyphSamplerScreen::NAME);
        if opens_sampler {
            self.stack
                .push(Box::new(GlyphSamplerScreen::new(&self.ctx)));
        } else {
            let input = FrameInput::new(actions, dt, held);
            self.quit = self.stack.update(&mut self.ctx, &input);
            if self.ctx.layout() != self.input_layout {
                self.input_layout = self.ctx.layout();
                self.input.set_keymap(self.ctx.keymap.clone());
            }
        }
        self.redraw();
    }

    /// Clears the buffer and draws the stack into it.
    fn redraw(&mut self) {
        let blank = Cell::new(
            ' ',
            self.ctx.palette.get(UiColor::Text),
            self.ctx.palette.get(UiColor::Black),
        );
        self.buffer.fill_rect(self.buffer.bounds(), blank);
        self.stack.draw(&self.ctx, &mut self.buffer);
    }

    /// The last frame drawn.
    pub fn buffer(&self) -> &GlyphBuffer {
        &self.buffer
    }

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// The shared context.
    pub fn ctx(&self) -> &Ctx {
        &self.ctx
    }

    /// Ends the game, handing back the shared context (and so its storage).
    pub fn into_ctx(self) -> Ctx {
        self.ctx
    }

    /// Name of the top screen, or `None` once the last one has closed.
    pub fn top_screen(&self) -> Option<&'static str> {
        self.stack.top_name()
    }

    /// Names of the open screens, bottom first.
    pub fn screens(&self) -> Vec<&'static str> {
        self.stack.names()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::tests::ctx;

    fn down(key: Key) -> RawKeyEvent {
        RawKeyEvent::Down(Chord::plain(key))
    }

    fn tap(game: &mut Game, key: Key) -> bool {
        let quit = game.frame(&[down(key)], 0.0).quit;
        game.frame(&[RawKeyEvent::Up(key)], 0.0);
        quit
    }

    #[test]
    fn starts_at_the_title_already_drawn() {
        let game = Game::start(ctx());
        assert_eq!(game.top_screen(), Some("title"));
        assert!(!game.quit_requested());
        let expected = {
            let mut g = Game::start(ctx());
            g.frame(&[], 0.0);
            g.buffer().clone()
        };
        assert_eq!(game.buffer(), &expected);
        assert_eq!(
            (game.buffer().width(), game.buffer().height()),
            (CONSOLE_W, CONSOLE_H)
        );
    }

    fn first_launch() -> Ctx {
        Ctx::embedded().unwrap()
    }

    #[test]
    fn first_launch_opens_the_picker_over_the_title() {
        let game = Game::start(first_launch());
        assert_eq!(game.screens(), ["title", "layout_picker"]);
        assert_eq!(game.ctx().layout(), None);
    }

    #[test]
    fn a_saved_layout_skips_the_picker() {
        let mut ctx = first_launch();
        ctx.storage.write("layout", "LeftHanded").unwrap();
        let game = Game::start(ctx);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.ctx().layout(), Some(Layout::LeftHanded));
        assert_eq!(game.input.keymap(), &game.ctx().keymap);
    }

    #[test]
    fn a_layout_in_use_beats_the_saved_one() {
        let mut ctx = ctx();
        ctx.storage.write("layout", "LeftHanded").unwrap();
        let game = Game::start(ctx);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.ctx().layout(), Some(Layout::RightHanded));
    }

    #[test]
    fn picking_a_layout_switches_the_input_keys() {
        let mut game = Game::start(first_launch());
        // Picker keys: `s` moves down, `j` picks.
        tap(&mut game, Key::S);
        tap(&mut game, Key::J);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.ctx().layout(), Some(Layout::LeftHanded));
        assert_eq!(game.input.keymap(), &game.ctx().keymap);
        // Left-handed: `j` confirms, `f` does nothing, `k` backs out.
        tap(&mut game, Key::F);
        assert_eq!(game.screens(), ["title"]);
        tap(&mut game, Key::J);
        assert_eq!(game.screens(), ["title", "placeholder"]);
        tap(&mut game, Key::K);
        assert_eq!(game.screens(), ["title"]);
        let ctx = game.into_ctx();
        assert_eq!(ctx.saved_layout(), Some(Layout::LeftHanded));
    }

    #[test]
    fn events_reach_the_top_screen() {
        let mut game = Game::start(ctx());
        assert!(!tap(&mut game, Key::F));
        assert_eq!(game.screens(), ["title", "placeholder"]);
        tap(&mut game, Key::D);
        assert_eq!(game.screens(), ["title"]);
    }

    #[test]
    fn quit_stops_further_frames() {
        let mut game = Game::start(ctx());
        game.frame(&[down(Key::Up)], 0.0);
        let out = game.frame(&[RawKeyEvent::Up(Key::Up), down(Key::F)], 0.0);
        assert!(out.quit);
        assert!(game.quit_requested());
        let before = game.buffer().clone();
        // Would open the placeholder if the game were still running.
        game.frame(&[RawKeyEvent::Up(Key::F), down(Key::Up)], 0.0);
        game.frame(&[RawKeyEvent::Up(Key::Up), down(Key::F)], 0.0);
        assert_eq!(game.screens(), ["title"]);
        assert_eq!(game.buffer(), &before);
        assert!(game.frame(&[], 0.0).quit);
    }

    #[test]
    fn popping_the_last_screen_quits() {
        let mut game = Game::new(ctx(), Box::new(crate::screens::PlaceholderScreen));
        assert!(tap(&mut game, Key::D));
        assert_eq!(game.top_screen(), None);
    }

    #[test]
    fn debug_key_opens_the_sampler_once() {
        let mut game = Game::start(ctx()).with_debug_screens(true);
        tap(&mut game, Key::F12);
        assert_eq!(game.screens(), ["title", "glyph_sampler"]);
        tap(&mut game, Key::F12);
        assert_eq!(game.screens(), ["title", "glyph_sampler"]);
        tap(&mut game, Key::D);
        assert_eq!(game.screens(), ["title"]);
    }

    #[test]
    fn debug_key_does_nothing_without_debug_screens() {
        let mut game = Game::start(ctx()).with_debug_screens(false);
        tap(&mut game, Key::F12);
        assert_eq!(game.screens(), ["title"]);
    }

    #[test]
    fn debug_screens_follow_the_build() {
        let game = Game::start(Ctx::embedded().unwrap());
        assert_eq!(game.ctx.debug_tools, cfg!(debug_assertions));
        let names = |ctx: Ctx| {
            let mut game = Game::start(ctx);
            tap(&mut game, Key::Down);
            tap(&mut game, Key::F);
            game.screens()
        };
        let mut release = ctx();
        release.debug_tools = false;
        assert_eq!(names(release), ["title"]); // Down + f chose Quit.
        let mut debug = ctx();
        debug.debug_tools = true;
        assert_eq!(names(debug), ["title", "battle"]);
    }

    /// Records what the screen saw.
    struct Spy(std::rc::Rc<std::cell::RefCell<Vec<FrameInput>>>);

    impl Screen for Spy {
        fn name(&self) -> &'static str {
            "spy"
        }
        fn update(&mut self, _: &mut Ctx, input: &FrameInput) -> crate::screen::Transition {
            self.0.borrow_mut().push(input.clone());
            crate::screen::Transition::None
        }
        fn draw(&self, _: &Ctx, _: &mut GlyphBuffer) {}
    }

    #[test]
    fn screens_see_actions_dt_and_held_keys() {
        let seen = std::rc::Rc::default();
        let mut game = Game::new(ctx(), Box::new(Spy(std::rc::Rc::clone(&seen))));
        assert_eq!(game.ctx().palette, ctx().palette);
        game.frame(&[down(Key::Right), down(Key::F)], 0.1);
        game.frame(&[RawKeyEvent::Up(Key::F)], 0.0);
        let seen = seen.borrow();
        assert_eq!(seen[0].actions, [Action::CursorRight, Action::Confirm]);
        assert!((seen[0].dt - 0.1).abs() < f32::EPSILON);
        assert!(seen[0].is_held(Action::CursorRight));
        assert!(seen[0].is_held(Action::Confirm));
        assert!(seen[1].actions.is_empty());
        assert!(seen[1].is_held(Action::CursorRight));
        assert!(!seen[1].is_held(Action::Confirm));
    }
}
