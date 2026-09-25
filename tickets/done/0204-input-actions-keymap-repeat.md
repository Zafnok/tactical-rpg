---
id: "0204"
title: Input actions, data-driven vim-style keymap, key repeat
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0201"]
nick_input: none
completed: 2026-09-25
---

# 0204 — Input actions, keymap, key repeat

## Context

[ADR-0006](../../docs/adr/0006-input-actions-and-virtual-cursor.md) defines
the `Action` layer, the default vim-style bindings and key-repeat timings.
Screens only ever see `Action`s. The cursor feel Nick will judge depends on
the repeat logic being exact.

## Nick input

None (feel is judged in playtest 0804).

## Scope

**In:** `trpg-ui::input` (keys, chords, actions, keymap, repeater),
`assets/data/keymap.ron` + its loader/validator in `trpg-content`, `app`
translation of macroquad keys.

**Out:** rebinding UI (0805), controller support, mouse.

## Implementation steps

1. `trpg-ui::input::Key`: hardware-agnostic enum — `A..=Z`, `Digit0..=9`,
   `Up/Down/Left/Right`, `Enter, Escape, Space, Tab, Backspace`, `F1..=F12`.
   `Chord { key: Key, shift: bool }`. `Chord::parse("Shift+h")`, `Display`
   round-trips.
2. `Action` enum exactly per ADR-0006: `CursorLeft/Down/Up/Right`,
   `CursorJumpLeft/Down/Up/Right`, `Confirm`, `Cancel`, `Info`, `DangerZone`,
   `NextUnit`, `PrevUnit`, `EndTurn`, `Menu`, `Debug` (F12), plus
   `ToggleAutoEnd` (default `Shift+e`; required by
   `docs/design/turn-structure.md`, added after ADR-0006). Mark which are
   *repeatable* (the eight cursor actions) via a method.
3. `assets/data/keymap.ron`: `bindings: { "CursorLeft": ["h", "Left"], "CursorJumpLeft": ["Shift+h", "Shift+Left"], … }`,
   `repeat: (delay_ms: 170, interval_ms: 55)`. `Esc`-as-menu rule from the ADR
   is screen logic: bind `Escape` to `Cancel`; screens with nothing to cancel
   treat `Cancel` as opening the menu.
4. `trpg-content`: `KeymapDef` loader with validation: unknown action name,
   unparsable chord, same chord bound to two actions → errors (all reported).
5. `trpg-ui::input::Keymap::from_def(&KeymapDef)` → lookup `Chord → Action`.
   A lowercase letter with `shift: true` is distinct from without.
6. `trpg-ui::input::InputState`:
   - `key_down(chord)`, `key_up(key)`, `update(dt: f32) -> Vec<Action>`.
   - A press emits its action once immediately. While held, repeatable actions
     re-emit first after `delay`, then every `interval`. Releasing stops it.
     Pressing a new direction while holding another switches repeat to the new
     one (most recent wins).
   - `is_held(Action) -> bool` (for "hold to fast-forward").
   - Large `dt` (lag spike) emits at most 1 + N repeats where N is what
     elapsed time implies, capped at 5 per update to avoid cursor teleporting.
7. `app`: each frame, `get_keys_pressed()` / `get_keys_released()` → map
   `macroquad::KeyCode` → `Key` (unknown → ignored); shift from
   `is_key_down(LeftShift|RightShift)`; call `InputState::update(get_frame_time())`.
   The sampler can ignore actions for now; log actions in debug builds to verify.

## Acceptance criteria

- [x] Default keymap loads and validates; conflicts/typos produce clear errors (tests).
- [x] Repeat timing exact per the spec (tests with fake `dt`).
- [x] App logs `CursorRight` repeating when holding `l` in a debug run.
- [x] `trpg-ui` still has no macroquad dependency.

## Tests required

- Unit: chord parse/display, keymap lookup (with/without shift), conflict detection, switching held direction, release stops repeat, cap on huge `dt`.
- Property: holding a repeatable key for total time `T`, fed in arbitrary `dt`
  slices of ≤ 100 ms (so the cap never triggers), emits exactly
  `1` if `T < delay`, else `2 + floor((T - delay) / interval)` actions
  (the initial press, the first repeat at `delay`, then one per `interval`).
- Property: `Chord::parse(c.to_string()) == Ok(c)`.

## Completion notes

- `trpg-content::keymap`: `Key` (letters, digits, arrows, Enter/Escape/Space/
  Tab/Backspace, F1–F12), `Chord { key, shift }` with `parse`/`Display`/
  `FromStr`, `Action` (ADR-0006 set + `ToggleAutoEnd`, `is_repeatable()` for
  the eight cursor actions), `RepeatDef`, and the validating `KeymapDef`
  loader. `Content` now carries `keymap`.
- `assets/data/keymap.ron`: the ADR-0006 defaults, `ToggleAutoEnd` on
  `Shift+e`, `repeat: (delay_ms: 170, interval_ms: 55)`.
- `trpg-ui::input`: `Keymap::from_def`, `InputState` (`key_down`, `key_up`,
  `update(dt)`, `is_held`), `MAX_REPEATS_PER_UPDATE = 5`. `trpg-ui` still
  has no macroquad dependency.
- `app`: `keys.rs` maps macroquad `KeyCode` → `Key` (keypad Enter counts as
  Enter; everything else unknown is ignored), feeds releases then presses,
  and debug builds log each action. If content fails to load, the window
  shows the errors instead of the game.
- Verified in a debug run (Xvfb + xdotool): holding `l` for 0.5 s logged 8
  `CursorRight` (press, first repeat at 170 ms, then every 55 ms);
  `Shift+l` logged `CursorJumpRight`.
- Deviations / decisions:
  - `Key`, `Chord` and `Action` are defined in `trpg-content` and re-exported
    from `trpg-ui::input`: the loader must parse chords and action names to
    validate the file, and `ui` depends on `content`, not the reverse.
    Screens still only see `trpg_ui::input::Action` (ADR-0006 intent kept).
  - Extra validation: every action must appear in the file (`[]` = unbound),
    so a forgotten action is caught; `interval_ms: 0` is rejected; a chord
    listed twice for the same action is an error too.
  - `Menu` has no default key: per the ADR it's opened by `Cancel` with
    nothing to cancel or `Confirm` on an empty tile (screen logic).
  - Releasing the repeating key hands repeat back to the most recent
    repeatable key still held, after a fresh delay (no immediate emit).
    The ticket didn't specify this; it avoids a "dead" held key.
  - Repeats beyond the per-update cap are dropped, not carried over, so the
    cursor never catches up after a lag spike.
  - Time is accumulated in whole microseconds (each `dt` rounded) so
    millisecond timings stay exact despite `f32` error.
- No follow-up tickets. Nothing new to play; repeat feel is judged in 0804.
