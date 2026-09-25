---
id: "0204"
title: Input actions, data-driven vim-style keymap, key repeat
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0201"]
nick_input: none
completed:
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

- [ ] Default keymap loads and validates; conflicts/typos produce clear errors (tests).
- [ ] Repeat timing exact per the spec (tests with fake `dt`).
- [ ] App logs `CursorRight` repeating when holding `l` in a debug run.
- [ ] `trpg-ui` still has no macroquad dependency.

## Tests required

- Unit: chord parse/display, keymap lookup (with/without shift), conflict detection, switching held direction, release stops repeat, cap on huge `dt`.
- Property: holding a repeatable key for total time `T`, fed in arbitrary `dt`
  slices of ≤ 100 ms (so the cap never triggers), emits exactly
  `1` if `T < delay`, else `2 + floor((T - delay) / interval)` actions
  (the initial press, the first repeat at `delay`, then one per `interval`).
- Property: `Chord::parse(c.to_string()) == Ok(c)`.

## Completion notes

