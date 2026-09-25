# ADR-0017: Screen stack, frame driver and test Harness

- **Status:** Accepted
- **Date:** 2026-09-25
- **Related tickets:** 0205

## Context

Every later UI ticket (battle map, menus, dialogue, level-up, options) needs
the same plumbing: something that owns the screens, routes input, draws them
in order and can be driven headlessly by tests (ADR-0007 layer 4). ADR-0004
puts screens, the screen stack and the Harness in `trpg-ui` and forbids
screen logic in `app`.

## Decision

- **`Screen` trait** (`trpg_ui::screen`): `name()` (stable `snake_case`, for
  tests), `update(&mut Ctx, &FrameInput) -> Transition`, `draw(&Ctx, &mut
  GlyphBuffer)`, `is_overlay()` (default `false`). Screens see `Action`s,
  never keys.
- **`Transition`**: `None`, `Push`, `Pop`, `Replace`, `Quit`. Screens don't
  touch the stack directly. Popping the last screen quits.
- **`ScreenStack`**: only the top screen updates; drawing starts at the
  top-most opaque screen and goes up through overlays.
- **`Ctx`**: one plain struct of shared resources (`Content`, `Palette`, the
  active `Keymap`), passed `&mut` to `update` and `&` to `draw`. New shared
  state (settings, storage) becomes a field; no service locator, no globals.
- **`FrameInput`**: the frame's actions (presses, then repeats), `dt`, and an
  `is_held(action)` query.
- **`Game`**: owns the stack, `InputState`, `Ctx` and the console
  `GlyphBuffer`. `frame(&[RawKeyEvent], dt) -> FrameOutput { buffer, quit }`
  is the only call `app` makes per frame. After a quit it does nothing.
  `Game` itself handles `Action::Debug` (debug builds) by pushing the glyph
  sampler.
- **Harness** (`trpg_ui::harness`, compiled for unit tests and behind the
  `harness` cargo feature, which the crate's dev-dependency on itself turns
  on for `tests/`): wraps a `Game` and drives it with key scripts
  (`keys("Down f")`, `hold`, `wait`) in 1/60 s frames; reports
  `top_screen()`, `snapshot()`, `quit_requested()`.
- **Help text names keys from the keymap** (`widgets::help`), never literals.

How-to for adding screens and tests: `crates/ui/README.md`.

## Consequences

- `app`'s loop is: collect events → `game.frame` → blit → `next_frame`. All
  behaviour is testable without a window.
- A screen gets at most one transition per frame; actions after the one that
  transitions are dropped. Screens that need finer control can keep state.
- Only the top screen animates. An overlay that must keep the screen below
  animating will need an explicit hook later.
- The Harness ships only in test builds; the release binary doesn't carry it.

## Alternatives considered

- **Screens mutate the stack through `Ctx`** — flexible, but makes screen
  order implicit and hard to test; a returned `Transition` is explicit and
  easy to assert.
- **Every screen updates each frame** — background screens would react to
  input meant for the top one; animating backgrounds can be added when needed.
- **`[[test]] required-features = ["harness"]`** — plain `cargo test` would
  silently skip the integration tests; the self dev-dependency always builds
  them.
