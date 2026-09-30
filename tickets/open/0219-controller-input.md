---
id: "0219"
title: "Controller input on Windows, Linux, macOS and web: buttons and sticks become Actions"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0032"]
nick_input: sign-off
completed:
---

# 0219 — Controller input

## Context

Nick (2026-09-29) wants to try Quick Battle with a controller soon, on the
web build (Pages and itch.io web) and the downloads (Windows first, and
Linux for players outside Steam). Steam will later offer Steam Input
(0903), but that only helps Steam players; this ticket gives every build
controller support of its own.

Screens only see `Action`s (ADR-0015), so a controller is one more source
of the same actions: no screen changes. Button defaults come from 0032
(`docs/design/controls.md`, *Controller*). Follow the `keyboard-input`
skill: buttons are treated exactly like keys (never named in game code or
text, defaults only in `keymap.ron`).

Engine facts that shape the approach:
- macroquad 0.4 has no gamepad API.
- **Native:** `gilrs` (MIT/Apache-2.0) covers Windows (XInput / Windows
  Gaming Input), Linux (evdev via libudev) and macOS (IOKit), and ships
  SDL's controller database, so PlayStation, Switch Pro and generic pads
  report the same standard buttons as an Xbox pad. It builds on the GNU
  toolchain (the `windows` crate carries its own import libraries).
- **Web:** gilrs's web backend needs `wasm-bindgen`, which macroquad's JS
  loader doesn't use. Instead, add our own small miniquad JS plugin over the
  browser Gamepad API, the same way `web/quad-storage.js` adds
  `localStorage`. Browsers only report a pad after a button is pressed
  while the page has focus.
- The `gamepads` crate bundles both, but pins an old gilrs (0.10) and has
  one maintainer; prefer gilrs directly plus our own JS. Record the choice
  in the ADR.

## Nick input

**Answer first:** 0032 (default buttons, stick behaviour).

**Sign-off:** on Pages, plug in a controller, press a button, and play a
Quick Battle with it. Help bars still show keyboard keys in this ticket
(0220 changes that). Optionally the same with the Windows download.

## Scope

**In:**
- Native pads through gilrs; web pads through a new `web/gamepad.js` plugin.
- Hot-plug: pads connected or removed while playing; several pads at once
  all drive the game.
- Default button bindings from 0032 in `assets/data/keymap.ron`.
- Held buttons repeat like held keys (same `repeat` timings).
- Left stick to 4-way cursor with a dead zone, one tile per step with the
  held-key repeat (0032: D-pad and left stick both move; how far the stick
  is pushed doesn't change speed; right stick and stick presses have no
  default job, but the right stick's directions are read like the left's
  so 0816 can bind them).
- **Confirm / Cancel follow the controller (0032 Q1 C):** bottom confirms
  and right cancels, except on Switch-style pads where those two swap (only
  those two). Needs the pad kind, detected here (step 5b); 0220 reuses it
  for button names.
- Pads keep working on the first-launch layout picker (Confirm / cursor).
  When the picker shows for controller players is 0226's job.
- Linux CI and release builds install `libudev-dev`.
- ADR for the controller input approach.

**Out (do not do):**
- Button names in help bars and tips (0220).
- Rebinding buttons, a controller page in Options (0816).
- Steam Input API / action sets, Steam Deck glyphs (0903).
- Rumble (0032: none for now, maybe a later ticket), mouse, touch.

## Implementation steps

1. **Button type** (`crates/content/src/keymap.rs`): `pub enum Button`
   with position names (`South`, `East`, `West`, `North`, `LeftShoulder`,
   `RightShoulder`, `LeftTrigger`, `RightTrigger`, `Select`, `Start`,
   `LeftStickPress`, `RightStickPress`, `DpadUp/Down/Left/Right`,
   `LeftStickUp/Down/Left/Right` and `RightStickUp/Down/Left/Right` for
   each stick's 4 directions), with
   `Display`/`parse` names for RON. Round-trip test every variant.
2. **`keymap.ron`**: a top-level `pad: { "Confirm": ["South"], … }` table
   (one table for both layouts, 0032 Q6a), validated like layouts: every
   action listed (`[]` for none), a button bound to at most one action,
   `Debug` must be `[]`, at most 3 buttons per action. Values exactly as
   in `controls.md` *Controller → Default buttons*; extend the
   keymap-matches-design test. Stick tuning next to `repeat`:
   `stick: (press: 0.5, release: 0.35)` (*tunable*), release below press so
   a stick resting on the edge doesn't flicker.
3. **InputState** (`crates/ui/src/input.rs`): an input is now
   `enum Input { Key(Key), Pad(Button) }`. `held` and `Repeat` store an
   `Input` instead of a `Key`; add `pad_down(Button)` / `pad_up(Button)`
   next to `key_down` / `key_up`, looking up `Keymap`'s pad bindings. Same
   rules: a press emits once, the newest held cursor input repeats. Several
   pads pressing the same button count as one (track per pad in `app`,
   send one down / one up).
4. **Events** (`crates/ui/src/game.rs`): `RawKeyEvent` gains
   `PadDown(Button)` and `PadUp(Button)` (rename to `RawInputEvent` if
   clearer); `Game::step` routes them to `InputState`.
5. **Pure pad-state diff** (`crates/ui/src/input.rs` or a submodule, no
   I/O): `PadState { buttons: u32 bitmask, left_stick: (f32, f32) }` and
   `fn pad_events(prev: &PadState, now: &PadState, stick: StickDef) ->
   Vec<(Button, bool)>`, including the stick-to-direction hysteresis. Both
   platforms fill a `PadState` per pad each frame, so all the logic is
   shared and tested here.
5b. **Pad kind and the Nintendo swap** (pure, `crates/ui/src/input.rs`):
   `PadKind { Xbox, PlayStation, Nintendo, Generic }` from a vendor id
   (Microsoft 045e, Sony 054c, Nintendo 057e; anything else `Generic`).
   For `Nintendo` pads, swap `South` ↔ `East` before the keymap lookup, so
   bindings stay in Xbox positions and a Switch pad's right button
   confirms. `app` passes each pad's kind with its events (gilrs vendor
   id; on web, the vendor id inside `Gamepad.id`).
6. **Native polling** (`crates/app/src/pads.rs`, `cfg(not(wasm32))`):
   `Gilrs::new()` once (on failure, log and run without pads); each frame
   drain events (for connect / disconnect), read each connected pad's
   buttons and left stick into a `PadState`, diff against last frame. A
   removed pad releases all its buttons.
7. **Web polling**: `web/gamepad.js` registers a miniquad plugin (see
   `web/quad-storage.js`) exporting e.g. `trpg_pad_count()`,
   `trpg_pad_buttons(i) -> u32` and `trpg_pad_axis(i, a) -> f32` over
   `navigator.getGamepads()`, standard-mapping pads only (`mapping ===
   "standard"`). `crates/app/src/pads.rs` `cfg(wasm32)` declares the
   `extern "C"` imports and builds the same `PadState`s. Add the script tag
   to `web/index.html` and the file to `SHELL_FILES` in
   `crates/xtask/src/web.rs` (with its test). It's our own code: no
   license file, no `THIRD_PARTY_ASSETS.md` entry. Check the itch.io web
   embed (an iframe) still sees pads; if the browser blocks it, note it.
8. **Main loop** (`crates/app/src/main.rs`): `keys::poll()` plus
   `pads.poll()`, both passed to `game.frame`.
9. **Dependencies**: `gilrs` (latest, `default-features` as needed) in
   `trpg-app` for non-wasm targets only. `cargo deny check` must pass
   (ADR-0013). Add `libudev-dev` to every `apt-get install` line that
   builds for Linux (`.github/workflows/ci.yml`, `mutants.yml`,
   `release.yml`); check the macOS release job still builds.
10. **check-keys** (if 0216 has landed): allow `Button::` in the same files
    that may name `Key::`, plus `crates/app/src/pads.rs`.
11. **ADR** (`write-adr` skill): "Controller input: gilrs native, Gamepad API
    plugin on web", amending ADR-0015 (buttons are a second binding table).
    Note that under Steam, Steam Input presents pads as a standard Xbox pad,
    so this keeps working there.

## Acceptance criteria

- [ ] Nick played a Quick Battle on Pages with a controller (sign-off).
- [ ] Harness test: `PadDown(South)` / `PadUp(South)` with the default pad
      bindings confirms; a held D-pad direction repeats with keyboard timings.
- [ ] Unit test: on a `Nintendo` pad `East` confirms and `South` cancels,
      other buttons unchanged; vendor id → `PadKind`, unknown → `Generic`.
- [ ] Unit tests: `pad_events` press / release, stick dead zone and
      hysteresis (no flicker between `press` and `release`), a pad removed
      mid-press releases its buttons, two pads holding one button = one press.
- [ ] Keymap loader rejects a button bound twice and a button on `Debug`;
      `keymap.ron` pad defaults match `controls.md` (test).
- [ ] `cargo xtask web` output includes `gamepad.js` and `index.html` loads it.
- [ ] Linux build in CI passes with gilrs; `cargo deny check` passes.
- [ ] ADR written; ADR index updated.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Button` names round-trip; `pad_events`; `InputState` pad
  press / repeat / release mixed with keys.
- Property: any sequence of pad and key downs / ups never leaves an input
  "held" after its up, and repeats only the newest held cursor input.
- Integration: Harness with pad events (no hardware needed).

## Completion notes

*(Filled in by the session that completes the ticket.)*
