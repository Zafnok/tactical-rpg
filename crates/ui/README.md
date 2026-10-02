# trpg-ui

Everything between raw key events and the glyphs on screen, with no macroquad,
clock or files (ADR-0004). `app` calls `Game::frame` once per frame and blits
the buffer it returns; tests drive the same `Game` headlessly with the
`Harness`.

| Module | What |
| ------ | ---- |
| `glyph_buffer`, `color`, `snapshot`, `console` | The 100×32 `GlyphBuffer` virtual console (cells, plus rectangles and sprites placed in pixels; see *What a frame holds*), palette colours, the text snapshot format |
| `map_view` | The battle map (ADR-0038): `MapScene` (what is on the visible map, plain data), `MapSkin` (how it looks) and the `GlyphSkin`. See *Map view* below |
| `input` | `Action`s, `Layout`, `Keymap`, `InputState` (key repeat) |
| `screen` | `Screen` trait, `Transition`, `FrameInput`, `Ctx` (shared resources, active layout), `ScreenStack` |
| `game` | `Game`: owns the stack, input state, `Ctx`, buffer and music state; `frame(events, dt)` |
| `audio` | `AudioRequest`, the `AudioQueue` screens push to (`ctx.audio`), `MusicState` (which track plays, fades) and its `MusicCommand`s (ADR-0026), `MusicClock` (how far into its track the music is, ADR-0037) |
| `widgets` | `Menu` (vertical list in a box), `help` (help text that names keys, or controller buttons when a pad was pressed last) |
| `flow` | `FlowScreen`: the game flow (ADR-0035). One screen on the stack that owns the `Campaign` and hosts the flow's screens itself: mode, lead, a chapter's scenes, its battle, the results of a won battle, Game Over, "To be continued" |
| `screens` | Game screens: `TitleScreen`, `ModeSelectScreen`, `LeadSelectScreen` (with the name grid), `GameOverScreen`, `ToBeContinuedScreen`, `ResultsScreen` (a won battle's gold, rewind bonus and EXP bars, then its level-up pages, 0810), `LayoutPickerScreen`, `KeyBindingsScreen` (rebinding, 0815), `CreditsScreen` (0808), `DialogueScreen` (full-screen or over the map), `ClassChangeScreen` (`screens/class_change`: promotion and reclass between battles, 0603), `BattleScreen` (`screens/battle`: its `mode` state machine, `attack` targeting, `forecast` panel and combat `playback`, which runs as a mode of the battle screen, ADR-0025) |
| `portrait` | `draw_portrait`: a 32×32-pixel portrait as 32×16 half-block cells, dimmed and/or mirrored (ADR-0018) |
| `debug` | Debug menu (F2 in debug builds): glyph sampler, portrait viewer, test scene (full-screen or overlay), Key bindings (until Options, 0805, opens it), sprite test, class change on a test unit (promote, reclass) |
| `dialogue` | `DialoguePlayer`: plays a dialogue `Scene` one text box at a time and gives the `View` (portraits, speaker, text, caption) to draw |
| `harness` | Headless test driver (tests, or the `harness` feature) |

## What a frame holds

A `GlyphBuffer` is the whole frame. `app` draws it and knows nothing else
about the game (ADR-0003, ADR-0038). It holds three kinds of thing:

| Thing | What | Added with |
| ----- | ---- | ---------- |
| Cells | A glyph with a foreground and a background colour, one per 8×16-pixel cell. All text, boxes and menus | `print`, `fill_rect`, `draw_box`, `set`, … |
| Rectangles (`Overlay`) | A solid colour in a rectangle of console pixels, for what whole cells can't draw (HP bars, the path line, the cursor's corners; ADR-0018) | `add_overlay` |
| Sprites (`Sprite`) | Part of an image file, scaled into a rectangle of console pixels. Every picture (ADR-0038) | `add_sprite` |

Rectangles and sprites are the frame's **items** (`Item`, `buf.items()`;
`overlays()` and `sprites()` give one kind). Both follow the same rules:

- **Layer.** `Layer::Under` items are drawn after the cells' backgrounds
  and before their glyphs; `Layer::Over` items after the glyphs. Within a
  layer, items are drawn in the order they were added, rectangles and
  sprites alike.
- **Clipping.** An item is clipped to the buffer; one wholly outside is
  dropped.
- **Items belong to the cells under them.** `fill_rect` and `blit` replace
  cells, so they remove the parts of items over those cells (a box drawn
  over half a picture hides that half). `blit` brings the source buffer's
  items along, moved with its cells. So draw a panel first, then the items
  on it.
- `dim` and `blend_bg` change cells only.

A sprite:

```rust
let card = ctx.content.images.id("images/test_card.png")?;   // an ImageId
let mut sprite = Sprite::new(
    card,
    Rect::new(0, 0, 16, 16),     // src: the part of the image, in image pixels
    Rect::new(16, 48, 80, 80),   // dest: where, in console pixels (here 5×)
    Layer::Over,
);
sprite.flip_x = true;            // mirrored left to right
sprite.opacity = 128;            // 255 = solid
buf.add_sprite(sprite);
```

- The image table (`ctx.content.images`, `trpg_content::image`) lists every
  PNG under `assets/` by path, with its size. `ui` never reads pixels; `app`
  decodes the files into textures.
- Clipping and cutting never move or rescale the picture: they only shrink
  the sprite's `clip` (the part of `dest` that is drawn). `src` and `dest`
  stay as given, and a cut sprite becomes up to four sprites with the same
  `src` and `dest`.
- Use whole-number scales (`dest` a multiple of `src`): the nearest image
  pixel is drawn, so other scales look uneven.
- In a snapshot each item is one line under `--- overlays ---`; a sprite's
  is `over  sprite 16,48 80x80  images/test_card.png 0,0 16x16`, followed
  by ` flip`, ` opacity=128` and ` clip=x,y wxh` when they apply
  (`snapshot.rs`).
- The "Sprite test" debug tool shows each of these on the test card; look
  at it after touching how `app` draws items.

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

## Map view

The battle map is not drawn by the battle screen (ADR-0038). Each frame:

1. `BattleScreen::scene(ctx)` builds a `MapScene`: the visible tiles (their
   terrain, whether it just changed and flashes, the ranges on them:
   danger, move, attack, heal, and what a spell being aimed would turn
   them into), the units
   on them (where each is drawn, HP, acted, under an effect, how far it has
   faded, whether it is picked out by a battle note), the cursor (if shown) and the selected unit's path. Plain data:
   no colours, glyphs, cells or pixels.
2. `ctx.map_skin.paint(ctx, &scene, MAP_VIEW, buf)` paints it. The only
   skin so far is the `GlyphSkin` (`map_view/glyph`): today's look.
3. Menus and heal numbers go beside a tile by asking the skin where it is
   (`tile_px` / `tile_cells`).

Rules:

- **The tile size is the skin's.** Nothing outside `map_view/glyph` knows a
  tile is two cells or 16 pixels. The camera gets the view's size in tiles
  from `skin.view_tiles(area)`.
- **A new thing on the map** (a village, a spell's flash on a tile) is a new
  field of `MapScene`, set in `BattleScreen::scene` and painted by every
  skin. Never draw it into the buffer from the battle screen.
- **A skin never changes the game**: only the frame and how many tiles are
  on screen. It must paint nothing outside the area it is given.
- **Tests of what happened read the scene** (or the `BattleState`), not
  cells and colours: `screen.scene(&ctx)` in unit tests, `h.map_scene()` /
  `h.map_text()` and the helpers below in Harness tests. Only tests of a
  skin's look read the buffer (see *Writing a Harness test*).

`MapScene::to_text(&content)` is the scene as text, for snapshots and bug
reports:

```text
origin (-10,-11) size 35x30
   0: -*10 sea*3 water*2 plain+m*2 forest+da -*17
units 1
  #4 Br enemy brigand (7,3) hp 20/30 acted effect fade=0.25
cursor (3,5) corners 1.00
path (3,5) (4,5)
```

One line per row of tiles (`-` = off the map, `+` then a letter per range:
`d` danger, `m` move, `a` attack, `h` heal; `!` = flashing after its
terrain changed; `>` then the terrain a spell would turn it into; `*n` =
`n` such tiles in a row), then the units, the cursor and the path.

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
   - Text that names a key gets it from `widgets::help` (`key_name`,
     `cursor_keys_name`, `help_line`) with `ctx.help_keys()`: each layout
     binds actions to different keys, and after a controller press the same
     calls name that pad's buttons instead (ADR-0036).
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
     and always handle `None` (ADR-0037).
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
- `map_scene()`: what the battle on the stack shows on its map, as a
  `MapScene` (`None` with no battle); `map_text()`: the same as text.
  Shortcuts: `cursor_tile()`, `unit_at(pos)`, `tints_at(pos)`, `path()`
  (and on a `MapScene`: `cursor_tile()`, `unit_at(pos)`, `tints_at(pos)`,
  `terrain_at(pos)`). Use these to check what happened on the map (*Map
  view* above).
- **A test that checks what happened reads the scene or the state. Only a
  test of a look reads cells, colours or items, and it lives with the
  skin** (`map_view/glyph*`). "The lord moved to (5, 5)" is
  `h.unit_at(Pos::new(5, 5))`, not the letters `Lo` at cell (30, 16);
  "(8, 7) is in the brigand's range" is `h.tints_at(..)`, not a cell's
  background. Panel, menu and help-bar text is the same in every skin, so
  reading it from the buffer is fine.
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
  storage. Saves live in it too (`trpg_ui::save`: keys `slot_01`…`slot_30`
  and `suspend`; `crates/ui/tests/save.rs`).
- `Harness::with_screen(Box::new(MyScreen::new()))` tests a screen on its
  own, with the right-handed layout.
- Key names in scripts depend on the layout (`docs/design/controls.md`):
  right-handed arrows move, `f` confirms, `d` cancels; left-handed `wasd`
  move, `j` confirms, `k` cancels.
- Debug screens are always on in the Harness, so `F2` works in any build.
- Snapshots: read every `.snap.new` before `cargo insta accept`.
