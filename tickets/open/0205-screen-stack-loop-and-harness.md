---
id: "0205"
title: Screen stack, game loop, menu widget, placeholder title screen, headless test Harness
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0203", "0204"]
nick_input: none
completed:
---

# 0205 — Screen stack, game loop, menu widget, test Harness

## Context

The UI architecture every later screen uses, and the **headless Harness** that
drives screens with scripted input for integration tests
([ADR-0007](../../docs/adr/0007-testing-strategy.md) layer 4). Most future
tickets depend on this; design the API carefully and document it.

## Nick input

None.

## Scope

**In:** `Screen` trait, `Transition`, `ScreenStack`, `Game` (frame driver),
`Ctx` (shared resources), `widgets::Menu`, placeholder `TitleScreen`,
debug screen for the glyph sampler, `Harness`, `app` loop using `Game`,
`crates/ui/README.md` documenting how to add a screen.

**Out:** real game flow (0801), battle screens (04xx), options (0805).

## Implementation steps

1. `trpg-ui::screen`:
   ```rust
   pub trait Screen {
       fn name(&self) -> &'static str;                  // for tests/debug
       fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition;
       fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer);
       fn is_overlay(&self) -> bool { false }            // draw screen below first
   }
   pub enum Transition { None, Push(Box<dyn Screen>), Pop, Replace(Box<dyn Screen>), Quit }
   pub struct FrameInput { pub actions: Vec<Action>, pub dt: f32, /* + held query */ }
   ```
   `Ctx` holds `&'static`/owned `Content`, `Palette`, and anything shared later
   (settings, storage handle). Keep it a plain struct.
2. `ScreenStack`: only the top screen gets `update`; drawing walks down to the
   first non-overlay screen and draws upward. Popping the last screen = quit.
3. `trpg-ui::Game`: owns `ScreenStack`, `InputState`, `Ctx`, a `GlyphBuffer`.
   `Game::frame(&mut self, events: &[RawKeyEvent], dt) -> FrameOutput { buffer: &GlyphBuffer, quit: bool }`.
   `app` becomes: collect events → `game.frame` → blit → `next_frame().await`;
   exit the loop on `quit` (on web, just stop updating).
4. `trpg-ui::widgets::Menu`: vertical list of items (label, enabled), `j/k`
   (CursorDown/Up, wraps), `Confirm` → `MenuEvent::Chosen(index)`, `Cancel` →
   `MenuEvent::Cancelled`; draws in a single-line box with the focused item
   highlighted (`text_highlight` on `panel_border_focus`) and disabled items dim.
   Reusable by action menus and map menu later.
5. `TitleScreen`: centred title text `tactical-rpg`, subtitle `an ASCII tactics game`,
   menu `New Game`, `Quit`. `New Game` pushes a `PlaceholderScreen`
   ("Coming soon — press d to go back"). `Quit` → `Transition::Quit`.
   Help line at the bottom: `arrows move · f select · d back` (key names read from the keymap).
6. Debug: `Action::Debug` (F12) in **debug builds only** (`cfg(debug_assertions)`)
   pushes `GlyphSamplerScreen` (wraps 0203's sampler; `Cancel` pops).
7. `trpg-ui::harness::Harness` (behind `#[cfg(any(test, feature = "harness"))]`
   so integration tests in `tests/` can use it via the feature):
   - `Harness::new()` → `Game` with embedded content, starting at the title.
   - `h.keys("Down Down f")`: whitespace-separated chords (`f`, `Shift+Space`, `Enter`),
     each a press + release with a 1-frame update; `h.hold("Right", 0.5)` holds for
     simulated seconds; `h.wait(0.2)`.
   - `h.top_screen() -> &'static str`, `h.snapshot() -> String`
     (0202's format), `h.quit_requested() -> bool`.
8. Integration tests `crates/ui/tests/title.rs`: title renders (snapshot);
   `"j f"` → quit requested; `"f"` → placeholder on top; `"f d"` → back to title.
9. `crates/ui/README.md`: how to add a screen + how to write a Harness test.

## Acceptance criteria

- [ ] Running the app shows the title menu; `j/k` moves, `f` selects, `d` backs out, Quit closes the window (native).
- [ ] F12 opens the glyph sampler in debug builds only.
- [ ] Harness integration tests pass; snapshots committed.
- [ ] `crates/ui/README.md` written.
- [ ] `app` contains no screen logic.

## Tests required

- Unit: stack push/pop/replace/overlay draw order; Menu wrap, disabled skip, events.
- Snapshot: title screen, menu with disabled item.
- Integration (Harness): step 8.

## Completion notes

