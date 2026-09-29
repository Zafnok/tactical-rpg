---
id: "0216"
title: Audit and remove hard-coded keys; guard test so none come back
type: infra
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0216 — Audit and remove hard-coded keys

## Context

Nick (2026-09-29, ticket 0030): "never hardcode keyboard input, draw from
the config which can be set by user". Players will rebind every key
(`docs/design/controls.md`, *Rebinding keys*; tickets 0217, 0815), so any
key that game code reads or names directly would break or lie after a
rebind.

[ADR-0015](../../docs/adr/0015-input-actions-and-keymap-layouts.md) already
says screens only see `Action`s and help text reads key names from the
active keymap (`crates/ui/src/widgets/help.rs`: `key_name`,
`all_key_names`, `cursor_keys_name`; tips use `{Action}` placeholders,
`crates/ui/src/tips.rs`). This ticket checks that it's true everywhere,
fixes what isn't, and adds a test so it stays true. It is the prerequisite
for 0217: the list it produces is every place 0217's player config must
reach. Follow the `keyboard-input` skill.

Known suspects from a quick grep when the ticket was written (not a full
audit):

- `crates/ui/src/input.rs`, `Keymap::layout_picker`: the keys that work
  before a layout is chosen (`Up`/`w`, `Down`/`s`, `f`/`j`/`Enter`/`Space`)
  are Rust constants, not data.
- `crates/ui/src/screens/layout_picker.rs`: `TOP_ROW`/`HOME_ROW` `Key`
  arrays. These draw a *picture* of a QWERTY keyboard, which is fine; check
  that the highlighted keys come from the keymap, not constants.
- `crates/ui/src/screens/dialogue.rs:48,142`: doc comments name "Space";
  check the code only uses `Action`s.
- Test code pressing `Key::F`, `Key::D`, … or `h.keys("f")`: allowed (tests
  pin today's default layout), but see step 4.

## Nick input

None.

## Scope

**In:**
- Search all non-test code in `crates/ui`, `crates/app`, `crates/content`
  and all text in `assets/` for keys used or named directly.
- Move each one to the keymap (data) or to a `key_name`-style lookup.
- Record the findings in the Completion notes as a table
  (file:line → what it was → what it is now).
- A guard test that fails if a hard-coded key is added later.

**Out (do not do):**
- The player-editable key config, 3 slots, required/optional actions, fixed
  `Esc`/`Delete` (0217).
- New actions (Select, Confirm end turn: 0218).
- The Key bindings screen (0815).
- Changing any default key. Every key must behave exactly as before.

## Implementation steps

1. **Audit.** Grep non-test code (outside `#[cfg(test)]` modules and
   `*_tests.rs`/`tests.rs` files) for:
   - `Key::` and `Chord::` / `Chord::plain` / `Chord::parse` outside
     `crates/content/src/keymap.rs`, `crates/ui/src/input.rs` and
     `crates/app/src/keys.rs`;
   - `KeyCode`, `is_key_down`, `is_key_pressed`, `get_keys_pressed`,
     `get_char_pressed`, `get_last_key_pressed` outside `crates/app/src/keys.rs`;
   - string literals that name a key to the player: single letters in help
     or UI text (`"f "`, `"[F]"`), `"Space"`, `"Esc"`, `"Enter"`, `"Shift"`,
     `"arrows"`, `"WASD"`, in `crates/ui/src/**` and `assets/data/*.ron`
     (except `keymap.ron`), `assets/dialogue/**`, `assets/**/*.dlg`.
   Record every hit.
2. **Pre-layout keys to data.** Move `Keymap::layout_picker`'s bindings into
   `assets/data/keymap.ron` as a new top-level `layout_picker: { <action>:
   [chords] }` section, validated by `KeymapDef`'s loader like a layout
   (unknown actions and malformed chords are errors; unlisted actions are
   unbound there). `Keymap::layout_picker(def)` builds from it. Keep the
   same keys. Update the file's header comment.
3. **Fix every other hit**: help/UI text uses `widgets::help` lookups or
   tips placeholders; input goes through `Action`s. Fix doc comments that
   name a specific key to name the action instead ("the End Turn key", not
   "Space").
4. **Guard test** in `crates/xtask` (new `cargo xtask check-keys`
   subcommand, plus a `#[test]` that runs it on the repo like the existing
   `ticket-lint` test in `crates/xtask/src/tickets.rs`):
   - scans `crates/ui/src`, `crates/app/src`, `crates/content/src` `.rs`
     files, skipping `#[cfg(test)]` modules and test files;
   - fails on the patterns in step 1 outside an explicit allow-list
     (`crates/content/src/keymap.rs`, `crates/ui/src/input.rs`,
     `crates/app/src/keys.rs`, and `crates/ui/src/screens/layout_picker.rs`
     for its keyboard-picture rows only, marked with a
     `// check-keys: keyboard picture` comment the scanner honours);
   - error messages name the file:line and say "use an Action and
     `widgets::help::key_name`; see the keyboard-input skill".
   Add `cargo xtask check-keys` to the `run-gates` skill's order and to CI
   next to `ticket-lint` (`.github/workflows/`).
5. Update the `keyboard-input` skill (`.claude/skills/keyboard-input/SKILL.md`)
   if the allow-list or command name differs from what it says.

## Acceptance criteria

- [ ] Completion notes list every hard-coded key found and its fix
      (or "none found" for a searched area).
- [ ] `Keymap::layout_picker` builds from `keymap.ron`; the picker still
      works with `Up`/`w`/`Down`/`s` and `f`/`j`/`Enter`/`Space` (existing
      Harness tests pass unchanged).
- [ ] `cargo xtask check-keys` passes on the repo and fails on a fixture
      containing `Key::F` in screen code and `"press f"` in help text (tests).
- [ ] CI and the `run-gates` skill run `cargo xtask check-keys`.
- [ ] No default key changed (existing keymap tests and snapshots pass
      unchanged).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `keymap.ron` loader accepts the `layout_picker` section and rejects
  an unknown action or bad chord in it; `check-keys` scanner on fixtures
  (hit, allowed file, `#[cfg(test)]` skip, `// check-keys:` marker).
- Integration: existing layout-picker Harness tests unchanged.

## Completion notes
