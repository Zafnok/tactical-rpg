---
id: "0224"
title: Web build shows "Press any key" on the title before the music
type: feature
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0034"]
nick_input: sign-off
completed:
---

# 0224 — Web build shows "Press any key" on the title before the music

## Context

Browsers block sound until the first key press or click, so the web build's
title music can't start with the title (found by ticket 0223). Nick decided
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

- [ ] Snapshot: title in the waiting state (title, subtitle, `Press any
      key`, no menu, no help line).
- [ ] Scripted test: waiting title requests no music; after a frame with the
      key flag set it requests `title` music and the menu shows with
      `New Game` still focused (the key didn't move or choose anything).
- [ ] Scripted test: a Confirm action without the flag does nothing while
      waiting. (The app always sets the flag with a key's actions; this
      pins that the screen reacts to the flag, not the action.)
- [ ] Native builds build the title without the prompt (existing snapshots
      unchanged).
- [ ] Checked in a browser: local `cargo xtask web` shows the prompt; any
      key shows the menu and the title music plays.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot + scripted integration for the title screen (ADR-0007).

## Completion notes

*(Filled in by the session that completes the ticket.)*
