# trpg-ui

Everything between raw key events and the glyphs on screen, with no macroquad,
clock or files (ADR-0004). `app` calls `Game::frame` once per frame and blits
the buffer it returns; tests drive the same `Game` headlessly with the
`Harness`.

| Module | What |
| ------ | ---- |
| `glyph_buffer`, `color`, `snapshot`, `console` | The 100×32 `GlyphBuffer` virtual console, palette colours, the text snapshot format |
| `input` | `Action`s, `Keymap`, `InputState` (key repeat) |
| `screen` | `Screen` trait, `Transition`, `FrameInput`, `Ctx` (shared resources), `ScreenStack` |
| `game` | `Game`: owns the stack, input state, `Ctx` and buffer; `frame(events, dt)` |
| `widgets` | `Menu` (vertical list in a box), `help` (help text that names keys) |
| `screens` | Game screens: `TitleScreen`, `PlaceholderScreen` |
| `debug` | Glyph sampler and its screen (F12 in debug builds) |
| `harness` | Headless test driver (tests, or the `harness` feature) |

## How a frame runs

1. `app` collects key presses/releases as `RawKeyEvent`s and calls
   `game.frame(&events, dt)`.
2. `Game` feeds them to `InputState`, which turns them into this frame's
   `Action`s (presses, then key repeats).
3. Only the **top** screen's `update(ctx, input)` runs. It returns a
   `Transition`: `None`, `Push(screen)`, `Pop`, `Replace(screen)` or `Quit`.
   Popping the last screen also quits.
4. The buffer is cleared and the stack draws: from the top-most screen whose
   `is_overlay()` is `false`, up through every overlay above it.
5. `FrameOutput { buffer, quit }` goes back to `app`. After a quit, frames do
   nothing.

In debug builds `Game` handles the `Debug` action (F12) itself and pushes the
glyph sampler.

## Adding a screen

1. Create `src/screens/<name>.rs` (or a sub-module for a big screen) and add it
   to `src/screens/mod.rs`.
2. Implement `Screen`:

   ```rust
   pub struct InfoScreen { menu: Menu }

   impl Screen for InfoScreen {
       fn name(&self) -> &'static str { "info" }   // unique, snake_case

       fn update(&mut self, ctx: &mut Ctx, input: &FrameInput) -> Transition {
           for &action in &input.actions {
               match self.menu.handle(action) {
                   Some(MenuEvent::Chosen(i)) => return Transition::Push(/* … */),
                   Some(MenuEvent::Cancelled) => return Transition::Pop,
                   None => {}
               }
           }
           Transition::None
       }

       fn draw(&self, ctx: &Ctx, buf: &mut GlyphBuffer) {
           // Opaque screens must paint every cell; overlays only their part.
       }

       fn is_overlay(&self) -> bool { false }        // true for menus/dialogs
   }
   ```

3. Rules:
   - React to `Action`s only, never keys. Stop at the first action that
     transitions; the rest of that frame's actions are dropped.
   - Anything animated advances by `input.dt`. Use `input.is_held(action)`
     for hold-to-fast-forward.
   - Colours come from `ctx.palette` (`UiColor` names), never raw RGB.
   - Text that names a key reads it from `ctx.keymap` via `widgets::help`
     (`key_name`, `cursor_keys_name`, `help_line`): each layout binds actions
     to different keys.
   - Never compute game rules here; send `core` commands and animate events.
   - Shared state that several screens need goes in `Ctx` (a plain struct).
4. Ship at least one snapshot test and one Harness test (ADR-0007).

## Writing a Harness test

Integration tests live in `crates/ui/tests/` (the crate's dev-dependency on
itself turns on the `harness` feature for them). Unit tests inside the crate
can use `crate::harness::Harness` directly.

```rust
use insta::assert_snapshot;
use trpg_ui::harness::Harness;

#[test]
fn select_opens_the_placeholder() {
    let mut h = Harness::new();          // embedded content, at the title
    h.keys("f");                         // press + release, one frame each
    assert_eq!(h.top_screen(), "placeholder");
    assert_snapshot!(h.snapshot());      // GlyphBuffer text snapshot
    h.keys("d");
    assert_eq!(h.top_screen(), "title");
}
```

- `keys("Down Down f")`: whitespace-separated chords as written in
  `keymap.ron` (`f`, `Left`, `Shift+Space`, `Enter`, `F12`). Each is pressed
  in one frame and released in the next; frames are `FRAME_DT` (1/60 s).
- `hold("Right", 0.5)`: holds a key for 0.5 simulated seconds (so it
  repeats), then releases it. `wait(0.2)`: time passes with no input. One
  call runs at most `MAX_FRAMES` (2 000, ~33 s); longer ones panic.
- `top_screen()`, `screens()`, `quit_requested()`, `snapshot()`, `game()`.
- `Harness::with_screen(Box::new(MyScreen::new()))` tests a screen on its
  own.
- Scripts use the default right-handed layout (`docs/design/controls.md`):
  arrows move, `f` confirms, `d` cancels.
- Debug screens are always on in the Harness, so `F12` works in any build.
- Snapshots: read every `.snap.new` before `cargo insta accept`.
