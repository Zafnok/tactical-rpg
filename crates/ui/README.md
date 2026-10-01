# trpg-ui

Everything between raw key events and the glyphs on screen, with no macroquad,
clock or files (ADR-0004). `app` calls `Game::frame` once per frame and blits
the buffer it returns; tests drive the same `Game` headlessly with the
`Harness`.

| Module | What |
| ------ | ---- |
| `glyph_buffer`, `color`, `snapshot`, `console` | The 100×32 `GlyphBuffer` virtual console, palette colours, the text snapshot format |
| `input` | `Action`s, `Layout`, `Keymap`, `InputState` (key repeat) |
| `screen` | `Screen` trait, `Transition`, `FrameInput`, `Ctx` (shared resources, active layout), `ScreenStack` |
| `game` | `Game`: owns the stack, input state, `Ctx`, buffer and music state; `frame(events, dt)` |
| `audio` | `AudioRequest`, the `AudioQueue` screens push to (`ctx.audio`), `MusicState` (which track plays, fades) and its `MusicCommand`s (ADR-0026), `MusicClock` (how far into its track the music is, ADR-0036) |
| `widgets` | `Menu` (vertical list in a box), `help` (help text that names keys) |
| `flow` | `FlowScreen`: the game flow (ADR-0035). One screen on the stack that owns the `Campaign` and hosts the flow's screens itself: mode, lead, a chapter's scenes, its battle, Game Over, "To be continued" |
| `screens` | Game screens: `TitleScreen`, `ModeSelectScreen`, `LeadSelectScreen` (with the name grid), `GameOverScreen`, `ToBeContinuedScreen`, `LayoutPickerScreen`, `KeyBindingsScreen` (rebinding, 0815), `DialogueScreen` (full-screen or over the map), `BattleScreen` (`screens/battle`: its `mode` state machine, `attack` targeting, `forecast` panel and combat `playback`, which runs as a mode of the battle screen, ADR-0025) |
| `portrait` | `draw_portrait`: a 32×32-pixel portrait as 32×16 half-block cells, dimmed and/or mirrored (ADR-0018) |
| `debug` | Debug menu (F2 in debug builds): glyph sampler, portrait viewer, test scene (full-screen or overlay), Key bindings (until Options, 0805, opens it) |
| `dialogue` | `DialoguePlayer`: plays a dialogue `Scene` one text box at a time and gives the `View` (portraits, speaker, text, caption) to draw |
| `harness` | Headless test driver (tests, or the `harness` feature) |

## How a frame runs

1. `app` tells the game what music is sounding
   (`game.set_music_playing(..)`, which sets `ctx.music_clock`), collects
   key and controller-button presses/releases as
   `RawInputEvent`s and calls `game.frame(&events, dt)`. Controllers go
   through `input::Pads` first, which turns each pad's raw state into
   button changes (ADR-0034).
2. `Game` feeds them to `InputState`, which turns them into this frame's
   `Action`s (presses, then repeats of the held cursor key or button). The
   raw key presses also go into `FrameInput::pressed_chords()`, which text
   boxes and the Key bindings screen read (to capture a key for a slot).
3. Only the **top** screen's `update(ctx, input)` runs. It returns a
   `Transition`: `None`, `Push(screen)`, `Pop`, `Replace(screen)` or `Quit`.
   Popping the last screen also quits.
4. The buffer is cleared and the stack draws: from the top-most screen whose
   `is_overlay()` is `false`, up through every overlay above it.
5. If the screen switched layout (`ctx.choose_layout`), `Game` gives
   `InputState` the new keymap.
6. The audio requests screens pushed to `ctx.audio` are drained; music
   requests go through `MusicState`, which turns them into `MusicCommand`s
   (load, start, gain, stop) and runs the fade.
7. `FrameOutput { buffer, quit, audio, music }` goes back to `app`, which
   plays the sounds and executes the music commands. After a quit, frames
   do nothing.

`Game::start` loads the saved layout from `ctx.storage` (key `layout`); on
first launch there is none, so it opens the layout picker over the title.
Until a layout is picked, `Keymap::layout_picker` is active (`Up`/`w`,
`Down`/`s`, and `f`/`j`/`Enter`/`Space` to pick), so either hand works.

In debug builds `Game` handles the `Debug` action (F2) itself and pushes the
debug menu (unless a debug screen is already on top).

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
   - Sounds and music: `ctx.audio.play_sound("menu_move")`,
     `play_sound_at(cue, 0.6)`, `play_music("title")`, `stop_music()`. Cue
     ids come from `assets/audio/audio.ron`; an unknown one panics in debug
     builds. Asking for the music already playing does nothing, so a screen
     may ask every time it's shown.
   - Keeping time with the music: read `ctx.music_clock` (`cue`,
     `position` and `length` in seconds; a looped track's position wraps
     at its length). It is `None` until the track really sounds (its file
     loads first, which can take seconds on the web) and stays `None` if
     the file is missing, so never count from your own `play_music` call
     and always handle `None` (ADR-0036).
   - Menu sounds (0425): use `menu.handle_with_sound(action, &mut
     ctx.audio)` for the menu widget (`Menu::without_cancel()` when there
     is nothing to back out of); elsewhere `ctx.audio.menu(MenuSound::…)`
     (`Move`, `Select` for confirming or opening, `Cancel` for backing
     out, `Denied` for confirming what can't be chosen), and `play_sound(CURSOR_MOVE)` per map tile. Play nothing when
     the key does nothing, and nothing for reading on through text.
4. Ship at least one snapshot test and one Harness test (ADR-0007).

## Writing a Harness test

Integration tests live in `crates/ui/tests/` (the crate's dev-dependency on
itself turns on the `harness` feature for them). Unit tests inside the crate
can use `crate::harness::Harness` directly.

```rust
use insta::assert_snapshot;
use trpg_ui::harness::Harness;
use trpg_ui::input::Layout;

#[test]
fn select_opens_new_game() {
    let mut h = Harness::with_layout(Layout::RightHanded); // at the title
    h.keys("f");                         // press + release, one frame each
    assert_eq!(h.top_screen(), "mode_select");
    assert_snapshot!(h.snapshot());      // GlyphBuffer text snapshot
    h.keys("d");
    assert_eq!(h.top_screen(), "title");
}
```

- `keys("Down Down f")`: whitespace-separated chords as written in
  `keymap.ron` (`f`, `Left`, `Shift+Space`, `Enter`, `F2`). Each is pressed
  in one frame and released in the next; frames are `FRAME_DT` (1/60 s).
- `hold("Right", 0.5)`: holds a key for 0.5 simulated seconds (so it
  repeats), then releases it. `wait(0.2)`: time passes with no input. One
  call runs at most `MAX_FRAMES` (2 000, ~33 s); longer ones panic.
- `top_screen()`, `screens()`, `quit_requested()`, `snapshot()`, `game()`.
- `flow()` / `flow_mut()`: the game flow on the stack; `flow_mut()` →
  `battle_mut()` → `send(&Command)` plays its battle with scripted
  commands (`crates/ui/tests/flow.rs`).
- Audio: `audio_requests()` (every request of the run), `last_frame_audio()`
  (the last frame's; a `keys` press is the frame *before* the last),
  `music_commands()`, `clear_audio()`, `sounds()` (the sound cues played,
  without music). A test of a made-up cue adds it to `ctx.content.audio`
  and builds the Harness with `Harness::from_game`.
- Music clock: the Harness plays the part of `app`. A track sounds from
  the frame after the game starts it and `music_clock()` counts up with
  the frames. `music_load_delay(2.0)` makes tracks take 2 s to load (the
  clock starts that much later); `without_music()` makes none ever sound
  (missing files). Call them before the frames that start the music.
- `Harness::new()` is a first launch (empty storage: the layout picker is on
  top). `Harness::with_layout(layout)` is a later launch with that layout
  saved. `into_storage()` + `Harness::with_storage(..)` restart with the same
  storage.
- `Harness::with_screen(Box::new(MyScreen::new()))` tests a screen on its
  own, with the right-handed layout.
- Key names in scripts depend on the layout (`docs/design/controls.md`):
  right-handed arrows move, `f` confirms, `d` cancels; left-handed `wasd`
  move, `j` confirms, `k` cancels.
- Debug screens are always on in the Harness, so `F2` works in any build.
- Snapshots: read every `.snap.new` before `cargo insta accept`.
