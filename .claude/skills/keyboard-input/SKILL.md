---
name: keyboard-input
description: Rules for anything that reads keyboard input or shows a key to the player — never hard-code a key; go through Actions and the player's keymap. Use when adding or changing a screen's controls, a help bar, a tip or dialogue text that names a key, a new input Action, a default key, or the key-binding config/screen.
---

# Keyboard input: never hard-code a key

Nick (ticket 0030): "never hardcode keyboard input, draw from the config
which can be set by user". Players rebind every key
([`docs/design/controls.md`](../../../docs/design/controls.md), *Rebinding
keys*), so a key written into game code or text is a bug: it stops working
or lies after a rebind, and the two layouts already use different keys.

Background: [ADR-0015](../../../docs/adr/0015-input-actions-and-keymap-layouts.md)
(actions, layouts), and [ADR-0031](../../../docs/adr/0031-player-key-bindings.md)
(player bindings, slots, fixed keys, saved config).

## The pipeline (only these places know about keys)

```
app/src/keys.rs        macroquad KeyCode → Key / Chord         (only macroquad key code)
assets/data/keymap.ron default chords per action, per layout   (Nick's keys, controls.md)
player config          Storage key `keybindings`               (per-layout, 3 slots; input/bindings.rs)
ui/src/input.rs        Keymap + InputState: Chord → Action      (fixed Esc = Cancel)
screens                see only `FrameInput.actions` / `is_held(Action)`
widgets/help.rs, tips  Action → key name for text
```

Files allowed to name `Key::…`, `Chord::…` or key names in strings:
`crates/app/src/keys.rs`, `crates/content/src/keymap.rs`,
`crates/ui/src/input.rs` (and its submodules), test code (`#[cfg(test)]`
items, `tests.rs`, `*_tests.rs`, `tests/`, `crates/ui/src/harness.rs`), and
the keyboard *picture* in `crates/ui/src/screens/layout_picker.rs`: each
picture item there sits under a `// check-keys: keyboard picture` comment
(the marker is an error in any other file). Only `crates/app/src/keys.rs`
may use `KeyCode` or macroquad's key reads. Anywhere else is a bug.

`cargo xtask check-keys` enforces this (CI `tickets` job and `run-gates`).
It scans `crates/{ui,app,content}/src` for key types, macroquad key
reads, and string literals naming a key (`"f select"`, `"press f"`,
`"[F]"`, `Space`, `Esc`, `Escape`, `Enter`, `Shift`, `arrows`, `WASD`).
Text in `assets/` (`.ron` strings except `keymap.ron`: tips, item and skill
descriptions; `.dlg` dialogue) is prose, so it only fails on an
instruction to press a key (`press f`, `hold Shift`, `hit Space`), a
`Shift+` chord, `WASD` or `[F]`. Plain words like "Escape!" or "fires
arrows" are fine there. Comments are
not scanned, but doc comments should still name the action, not the key.
Even the layout picker's own keys are data: the `layout_picker` section of
`assets/data/keymap.ron`.

## Rules

1. **React to `Action`s, not keys.** Match on `Action::Confirm`, never on
   `Key::F`. Need a new kind of input? Add an `Action` (below); don't read
   raw keys. The one exception is the Key bindings screen's capture mode
   (0815), which reads `FrameInput::pressed_chords` and asks `input.rs`
   helpers (`is_capture_abort`, `is_clear_slot`) instead of naming keys.
2. **Name keys in text through the keymap.** Help bars:
   `widgets::help::{key_name, all_key_names, cursor_keys_name, help_line}`.
   Tips and other data text: `{ActionName}` / `{Cursor}` placeholders
   (`assets/data/tips.ron`, `crates/ui/src/tips.rs`). Never write `"f"`,
   `"[F]"`, `"Space"`, `"Esc"`, `"arrows"` or `"WASD"` into player text.
   Doc comments name the action ("the End turn key"), not a key.
3. **Expect any key, or none.** Game code can't assume which key an action
   has, that two actions have different keys in both layouts, or (for
   optional actions) that it has a key at all. Text for an
   action with no key shows `NOT_MAPPED` (`! not mapped`), which the help
   helpers do for you.
4. **Default keys are Nick's.** They live only in `assets/data/keymap.ron`
   and must match the table in `docs/design/controls.md`; a test pins it.
   Changing a default is a design change: ask with the `ask-nick` skill,
   then edit both files. Never pick a default key yourself.
5. **Reserved keys**: `Esc` is always Cancel and backs out of
   "Press a key…"; `Delete` empties a slot. Neither may appear in
   `keymap.ron` or in a player's slots (`Key::is_reserved`,
   `LayoutBindings::is_reserved`). `Keymap::chords_for(Cancel)` leaves
   `Esc` out; `Keymap::fixed_chords_for` names it. The Debug key is not
   rebindable (and reserved in builds with debug tools).
6. **Held and repeated keys** come from `InputState` (repeat timings in
   `keymap.ron`); don't time key presses in a screen.

## Adding an action

1. `crates/content/src/keymap.rs`: add the `Action` variant with a doc
   comment saying what it does, add it to `Action::ALL`, give it a RON name.
2. Is it **required** (must always have a key) or **optional**? That's
   Nick's call: the list is in `controls.md` *Rebinding keys*. If the new
   action isn't covered, ask with `ask-nick`. Set `Action::is_required` to
   match.
3. `assets/data/keymap.ron`: add it to **every** layout (`[]` if it has no
   default key), at most 3 chords (`SLOTS`), keys from `controls.md`.
4. The Key bindings screen (after 0815): add its player-facing label to the
   screen's label table.
5. Screens: react to the action; help bars/tips name it via rule 2.
6. Tests: the keymap-matches-design test, a screen test driven by the
   `Action`, and a Harness test using the default key.

## Tests

- Unit tests of screens: feed `FrameInput`s of `Action`s.
- Harness scripts (`h.keys("Down f")`) may use key names: they pin the
  default right-handed layout on purpose. When a test is about the player's
  own keys, rebind in the test (`h.ctx_mut().set_layout_bindings(…)`
  with an edited `ctx.layout_bindings(layout)`) and press the new key.

## Checklist before pushing input work

- [ ] No `Key::`, `Chord::` or `KeyCode` outside the allowed files.
- [ ] No key names in player text or data; placeholders/helpers used.
- [ ] Works in both layouts and with a rebound key (test at least one).
- [ ] Unbound optional actions don't panic and show `! not mapped`.
- [ ] `cargo xtask check-keys` passes.
