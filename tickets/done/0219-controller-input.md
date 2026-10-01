---
id: "0219"
title: "Controller input on Windows, Linux, macOS and web: buttons and sticks become Actions"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: ["0032"]
nick_input: sign-off
completed: 2026-09-30
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

- [ ] Nick played a Quick Battle on Pages with a controller (sign-off:
      after merge, see Completion notes).
- [x] Harness test: `PadDown(South)` / `PadUp(South)` with the default pad
      bindings confirms; a held D-pad direction repeats with keyboard timings.
- [x] Unit test: on a `Nintendo` pad `East` confirms and `South` cancels,
      other buttons unchanged; vendor id → `PadKind`, unknown → `Generic`.
- [x] Unit tests: `pad_events` press / release, stick dead zone and
      hysteresis (no flicker between `press` and `release`), a pad removed
      mid-press releases its buttons, two pads holding one button = one press.
      (The function became `Pads::update`, see Completion notes.)
- [x] Keymap loader rejects a button bound twice and a button on `Debug`;
      `keymap.ron` pad defaults match `controls.md` (test).
- [x] `cargo xtask web` output includes `gamepad.js` and `index.html` loads it.
- [x] Linux build in CI passes with gilrs (checked on the PR); `cargo deny
      check` passes.
- [x] ADR written (ADR-0034); ADR index updated.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Button` names round-trip; `pad_events`; `InputState` pad
  press / repeat / release mixed with keys.
- Property: any sequence of pad and key downs / ups never leaves an input
  "held" after its up, and repeats only the newest held cursor input.
- Integration: Harness with pad events (no hardware needed).

## Completion notes

Done; ADR-0034 records the approach. A controller now drives every screen
through the same `Action`s as the keys. Help bars still name keys (0220).

**What was tested, and what wasn't.** All the rules are covered by unit,
property and Harness tests (`crates/ui/src/input/pad/tests.rs`,
`crates/ui/tests/controller.rs` plays Quick Battle with buttons and
compares every screen with the same play on keys). The web plugin was run
in a real browser against **simulated** Gamepad objects: layout picker,
title, Quick Battle, a held stick, a Switch-style pad (right button
confirmed, bottom cancelled), unplugging mid-press. **No physical
controller was available**, so gilrs on Windows and real pads in a browser
are first tried by Nick's sign-off.

**Sign-off for Nick (after merge, on Pages):** plug in a controller, press
any button (the browser only notices the pad then), and play a Quick
Battle: D-pad or left stick moves, bottom button confirms, right button
backs out, shoulders jump between units, top button unit info, left button
danger zone, Start twice ends the turn, Back toggles auto-end, left trigger
rewinds. Optionally the same with the Windows download.

**Deviations from the steps**

- **Step 2, stick thresholds** are whole percents,
  `stick: (press_percent: 50, release_percent: 35)`, not floats: `KeymapDef`
  and `Content` derive `Eq`.
- **Step 5:** instead of `pad_events(prev, now, stick)`, a small
  `Pads` tracker (`crates/ui/src/input/pad.rs`): `Pads::update(&[(PadId,
  PadKind, PadState)]) -> Vec<(Button, bool)>`. Hysteresis needs what was
  *held* last frame, not the last raw stick values, and merging several
  pads and the Switch swap had to live in the same pure, tested place.
  `PadState` also has the right stick.
- **Step 5b:** the swap happens in `Pads` (before pads are merged), not at
  the keymap lookup, so a Switch pad's `A` and an Xbox pad's `A` held
  together are one press. `PadDown(Button)` carries the binding position.
- **Step 4:** renamed `RawKeyEvent` → `RawInputEvent` (updated in open
  ticket 0815 and `crates/ui/README.md`).
- **Steps 6–7:** `crates/app/src/pads.rs` plus `pads/native.rs` and
  `pads/web.rs`. The JS functions are `trpg_pad_poll`, `_index`, `_vendor`,
  `_buttons`, `_axis`.
- **`unsafe`:** the workspace forbids it, and importing our own JS
  functions needs an `unsafe extern` block. `trpg-app` now carries a copy
  of the workspace lints with `unsafe_code = "deny"` and one
  `#[allow(unsafe_code)]` in `pads/web.rs` (ADR-0004 rule 5 already allows
  `unsafe` in `app` only). An xtask test keeps the copy in step and checks
  the other crates still forbid it.
- **Not in the steps, added:** a controller button also ends the web
  title's `Press any key` wait (one condition in `Game::step`). Without it
  a controller-only player is stuck on the title on Pages, and the sign-off
  couldn't be done with a pad. The line still reads "key"; the wording, the
  pictures and "every build" stay 0226's.
- `THIRD_PARTY_ASSETS.md` has a row for SDL's controller database (zlib),
  which gilrs compiles into native builds; its licence text is at
  `crates/app/SDL_GameControllerDB-LICENSE.txt`.
- **itch.io** (step 7): its game iframe has `allow="gamepad"` (checked on a
  live HTML5 game page, 2026-09-30), so browsers let the game see pads
  there. Noted in `web/README.md`.
- macOS release job: unchanged; gilrs uses IOKit with no extra packages.
  It only runs on a release tag, so it is first exercised by the next
  release.

**Known limits** (also in ADR-0034): on web only pads the browser gives its
"standard" mapping work; a pad counts as Switch-style only when it reports
Nintendo as its maker (third-party Switch-shaped pads, every pad in Safari
and Xbox-type pads in Chrome report none, and count as ordinary pads); on
Linux a pad is read even when the game window isn't focused.

**Notes left in other tickets:** 0220 (how to get the pad's kind to
`InputState::device()`), 0226 (the prompt already accepts a button). No new
tickets.

*Claude's starting rules* (the design docs didn't say; Nick can veto; also
in `controls.md`):

- **Diagonal stick pushes move one way only**, the way the stick is pushed
  furthest. Example: pushed up and slightly right, the cursor goes up, never
  up-and-right. (A D-pad diagonal presses two buttons, like two arrow keys.)
- **The stick's dead zone:** it counts as pushed from half way out and
  stops counting once it falls back to about a third (*tunable*).
- **A controller button dismisses `Press any key`** on the web title
  already (see above).
- **Only Nintendo-made pads swap Confirm / Cancel.** Other makers'
  Switch-shaped pads confirm with the bottom button.
