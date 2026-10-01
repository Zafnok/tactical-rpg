# ADR-0034: Controller input: gilrs on native, a Gamepad API plugin on web

- **Status:** Accepted
- **Date:** 2026-09-30
- **Related tickets:** 0219, 0032, 0220, 0226, 0816, 0903
- **Amends:** ADR-0015 (controller buttons are a second binding table next
  to the key layouts) and ADR-0004 rule 5 (how `app` is allowed its one
  `unsafe`). Nothing in either is superseded.

## Context

Nick wants to play with a controller on every build: the web build (Pages,
itch.io) and the downloads (Windows, Linux, macOS), not only under Steam
(`docs/design/controls.md`, *Controller*, ticket 0032). Forces:

- Screens only see `Action`s (ADR-0015), so a controller is one more source
  of the same actions.
- macroquad 0.4 has no gamepad API.
- The web build uses macroquad's own JS loader, not `wasm-bindgen`, so
  crates that reach the browser through `wasm-bindgen` (gilrs's web
  backend) can't be used there.
- The rules must be testable without hardware (ADR-0007): sticks as
  directions, the Switch-style Confirm / Cancel swap, several pads as one.
- The workspace forbids `unsafe` (`unsafe_code = "forbid"`), and in the
  2024 edition declaring imported functions needs an `unsafe extern` block.

## Decision

**Buttons are data, like keys.**

- `trpg-content::keymap::Button` names 24 buttons by **position** on an
  Xbox-shaped pad (`South`, `East`, …, `DpadUp`, …), including each stick's
  four directions (`LeftStickUp`, `RightStickLeft`, …), which count as
  buttons. The first 16 are in the browser Gamepad API's "standard" mapping
  order.
- `assets/data/keymap.ron` gains a top-level `pad` table (action → buttons,
  one table for both layouts and the layout picker) validated like a
  layout: every action listed, at most `SLOTS` buttons, a button on one
  action only; and `Debug` must be `[]`. Defaults come from `controls.md`;
  a test pins them. It also gains
  `stick: (press_percent: 50, release_percent: 35)`: integers, so
  `KeymapDef` stays `Eq`.
- `trpg-ui::input::Keymap` carries the button table
  (`with_pad`, `with_default_pad`, `pad_action`, `buttons_for`). Every
  keymap built from the definition has the default buttons;
  `LayoutBindings::keymap` stays keys-only because buttons don't belong to
  a layout. Rebinding buttons is ticket 0816.

**One input model.** `InputState` holds `Input::Key(Key)` or
`Input::Pad(Button)`; `pad_down` / `pad_up` follow the same rules as
`key_down` / `key_up` (a press emits once, the newest held cursor input
repeats with the shared `repeat` timings). `RawKeyEvent` is renamed
`RawInputEvent` and gains `PadDown(Button)` / `PadUp(Button)`. Any press,
key or button, ends the title's "press any key" wait.

**All pad rules are pure code in `trpg-ui::input::pad`.** Each frame `app`
gives `Pads::update` a list of `(PadId, PadKind, PadState)`, one per
connected pad, and gets back `(Button, pressed)` changes:

- `PadState` is the raw state: the real buttons held (by physical
  position) and both sticks as `(x, y)`, down-positive.
- **Sticks:** a stick holds at most one direction button, the direction it
  is pushed furthest (left / right on a perfect diagonal). It starts at
  `press_percent`, ends below `release_percent`, and gives way to another
  direction only when that one is pushed further by the gap between the two
  thresholds, so neither the dead-zone edge nor a diagonal flickers.
- **Pad kind:** `PadKind::from_vendor` (USB vendor id: Microsoft `045e`,
  Sony `054c`, Nintendo `057e`, anything else `Generic`). On a `Nintendo`
  pad `South` and `East` swap before anything else sees them, so bindings
  stay in Xbox positions and the button labelled `A` confirms. 0220 reuses
  `PadKind` for button names.
- **Several pads** are merged: a button is down while any pad holds it; a
  pad missing from the list was removed and releases what it held.

**Native: gilrs** (`gilrs = "0.11"`, MIT / Apache-2.0, non-wasm targets
only, in `crates/app/src/pads/native.rs`). It covers Windows (Windows
Gaming Input), Linux (evdev through libudev) and macOS (IOKit), and bundles
SDL's controller database (zlib licence), so PlayStation, Switch and
generic pads report the same buttons by position. `app` drains gilrs's
events each frame and reads each connected pad's cached state. If gilrs
can't start, the game logs it and runs without controllers.

**Web: our own miniquad plugin**, `web/gamepad.js` (our code, no
third-party licence), over `navigator.getGamepads()`. It lists only
connected pads with `mapping === "standard"` and exports
`trpg_pad_poll / _index / _vendor / _buttons / _axis`; the vendor id is
parsed from `Gamepad.id`. `crates/app/src/pads/web.rs` declares those
imports and builds the same `PadState`s (`PadState::from_standard`).
xtask tests keep the two function lists and the plugin version equal, and
`cargo xtask web` ships the file.

**`unsafe`.** `trpg-app` no longer inherits the workspace lint table; it
carries a copy with `unsafe_code = "deny"` instead of `forbid` (Cargo can't
override one inherited lint), and `pads/web.rs` has the only
`#[allow(unsafe_code)]`: the `unsafe extern "C"` block of `safe fn`
imports, and the `#[unsafe(no_mangle)]` version export the JS loader asks
for. An xtask test keeps the copy equal to the workspace table otherwise,
and checks every other crate still inherits `forbid`.

**check-keys** treats `Button::` like `Key::`: only the key pipeline, plus
`crates/app/src/pads.rs` and `crates/app/src/pads/`, may name it.

## Consequences

- No screen changed: every screen works with a pad, including the
  first-launch layout picker. Help bars still name keys until 0220.
- All pad behaviour is unit- and property-tested without hardware; `app`
  only translates. The gilrs and browser reads themselves are covered by
  playing (Nick's sign-off), like `keys.rs`.
- Linux builds need `libudev-dev` at build time (added to every CI
  `apt-get` line) and `libudev` at run time, which desktop distributions
  ship.
- `Cargo.lock` now lists `wasm-bindgen`, `js-sys` and `web-sys` (gilrs's
  unused web backend). They are never compiled into our wasm build, and
  `cargo deny` passes.
- Under Steam, Steam Input presents every pad as a standard Xbox pad, so
  this path keeps working there (0903 may add Steam's own glyphs).
- Known limits: pads a browser doesn't give a "standard" mapping are
  ignored on web; XInput pads in Chrome and every pad in Safari report no
  vendor id, so they count as `Generic` (fine for Xbox pads; a Switch pad
  there would not swap); third-party Switch-style pads have their own
  vendor ids and don't swap either. On Linux, evdev reports a pad even when
  the game window isn't focused.
- 0816 adds player button bindings next to `PlayerKeys`; 0226 changes when
  the layout picker shows for controller players.

## Alternatives considered

- **The `gamepads` crate** (gilrs plus a macroquad JS plugin in one) — pins
  an old gilrs (0.10), has a single maintainer, and exposes no vendor id,
  which the Switch-style swap and 0220's button names need.
- **gilrs on web too** — needs `wasm-bindgen`, which macroquad's loader
  doesn't use.
- **Steam Input only** — nothing for Pages, itch.io or non-Steam
  downloads, which is where Nick plays first.
- **A separate FFI crate for the imports**, keeping `forbid` on `app` —
  one more crate with its own copied lint table for five declarations;
  ADR-0004 already allows `unsafe` in `app`.
- **Floats for the stick thresholds in `keymap.ron`** — `KeymapDef` (and
  `Content`) derive `Eq`; whole percents are just as tunable.
- **Independent stick axes** (each direction its own threshold) — a
  slightly off-axis push would step sideways as well; the design says the
  stick never moves diagonally.
- **The swap inside `InputState`** — pads of different kinds couldn't be
  merged into one press before it.
