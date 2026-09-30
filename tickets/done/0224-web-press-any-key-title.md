---
id: "0224"
title: Web build shows "Press any key" on the title before the music
type: feature
milestone: M1 Engine
model: sonnet-5
effort: medium
status: done
blocked_by: ["0034"]
nick_input: sign-off
completed: 2026-09-30
---

# 0224 — Web build shows "Press any key" on the title before the music

## Context

Browsers block sound until the first key press or click, so the web build's
title music can't start with the title (found by ticket 0225). Nick decided
(ticket 0034, [`docs/design/title-screen.md`](../../docs/design/title-screen.md))
that on the web build the title shows a `Press any key` line where the menu
goes; the first key press shows the menu and starts the music. The bundled
JS loader (`web/mq_js_bundle.js`) already resumes the browser's audio on
the first `keydown`, so the key press both unlocks sound and dismisses the
prompt.

## Nick input

**Sign-off:** after merge, open the Pages build. The title should show
`Press any key` with no music; pressing any key shows the menu and starts
the title music. The Windows build is unchanged.

## Scope

**In:**
- A "waiting for a key" state on `TitleScreen`, turned on only for the web
  build (`target_arch = "wasm32"` decided in `crates/app`, passed in; `ui`
  stays platform-agnostic).
- A way for screens to know "some key was pressed this frame", whatever it
  is bound to (not an `Action`): e.g. a `bool` on `FrameInput` set by the
  app from macroquad's pressed keys.
- Every rule in `docs/design/title-screen.md`, including Claude's starting
  rules.

**Out (do not do):**
- The prompt on Windows.
- Mouse or controller handling.
- Title art (0811).

## Implementation steps

1. `crates/ui/src/screen.rs`: add an "any key pressed this frame" flag to
   `FrameInput` (constructor or builder; keep existing callers compiling,
   default `false`, and have the test harness able to set it).
2. `crates/app`: set the flag when macroquad reports any key pressed this
   frame (e.g. `get_last_key_pressed().is_some()` or
   `!get_keys_pressed().is_empty()`). This names no key, so it doesn't break
   the keyboard-input rules; check `cargo xtask check-keys` still passes.
3. `crates/ui/src/screens/title.rs`: a `TitleScreen` option (e.g.
   `.with_press_any_key()`) that starts in the waiting state. While waiting:
   no music request, draw `Press any key` centred at `MENU_ROW` in
   `UiColor::TextDim`, no menu, no bottom help line, ignore actions. On the
   frame the flag is set: leave the waiting state, request the title music,
   and don't pass that frame's actions to the menu.
4. `crates/ui/src/game.rs`: where the title screen is built, turn the option
   on when the app says it is the web build (a field/argument from `app`).
5. Returning to the title later (Pop from another screen) must not show the
   prompt again: the state only goes from waiting → menu.

## Acceptance criteria

- [x] Snapshot: title in the waiting state (title, subtitle, `Press any
      key`, no menu, no help line).
- [x] Scripted test: waiting title requests no music; after a frame with the
      key flag set it requests `title` music and the menu shows with
      `New Game` still focused (the key didn't move or choose anything).
- [x] Scripted test: a Confirm action without the flag does nothing while
      waiting. (The app always sets the flag with a key's actions; this
      pins that the screen reacts to the flag, not the action.)
- [x] Native builds build the title without the prompt (existing snapshots
      unchanged).
- [ ] Checked in a browser (partly, see notes): local `cargo xtask web` shows the prompt; any
      key shows the menu and the title music plays.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot + scripted integration for the title screen (ADR-0007).

## Completion notes

- **Deviation from steps 1 and 2:** no new `FrameInput` flag and no
  app-side key polling. `Game` already receives every raw key press, so it
  moves a new `Ctx::key_prompt` (`KeyPrompt::Off / Waiting / Pressed`) from
  `Waiting` to `Pressed` on the first one. `app` sets `Waiting` on wasm.
  It's an enum rather than two `bool`s because clippy's
  `struct_excessive_bools` rejects more bools on `Ctx`.
- Because that state lives on `Ctx`, a key pressed on the first-launch
  layout picker also counts: after picking a layout, the title shows its
  menu and music straight away (test `picking_a_layout_first_skips_the_prompt`).
- **Extra fix in `web/index.html`:** browsers don't treat Escape (or
  modifier keys) as the player's gesture, and the bundle's own audio unlock
  removes itself after the first key whether or not it worked. Pressing
  Escape first would have dismissed the prompt and left the game silent
  for the whole session. A small script now wraps `AudioContext` and
  resumes it on every key press, click or touch until it runs; xtask test
  `index_html_keeps_resuming_audio_until_it_runs` guards it. After Escape
  the prompt is gone and the music starts on the next key.
- Tests: snapshot `web_title_waits_for_a_key`; scripted
  `any_key_shows_the_menu_and_starts_the_music` (an unbound key),
  `the_key_that_ends_the_wait_does_nothing_else`,
  `back_on_the_web_title_shows_the_menu_not_the_prompt`,
  `picking_a_layout_first_skips_the_prompt`; unit
  `a_key_press_moves_the_key_prompt_on` (covers the "Confirm without a key"
  criterion: releases alone don't end the wait).
- **Browser check, partly done:** the local web build loads with the
  script in place and no new console errors. The in-app test browser
  allows sound without a key press, so it can't show the
  blocked-then-unlocked path. Nick's first run on Pages is the real check.
- The F2 debug hint still shows in the corner while waiting (debug and
  Pages builds only).
- Claude's starting rules are in `docs/design/title-screen.md`.
