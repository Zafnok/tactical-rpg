---
id: "0216"
title: Audit and remove hard-coded keys; guard test so none come back
type: infra
milestone: M1 Engine
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-09-30
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

- [x] Completion notes list every hard-coded key found and its fix
      (or "none found" for a searched area).
- [x] `Keymap::layout_picker` builds from `keymap.ron`; the picker still
      works with `Up`/`w`/`Down`/`s` and `f`/`j`/`Enter`/`Space` (existing
      Harness tests pass unchanged).
- [x] `cargo xtask check-keys` passes on the repo and fails on a fixture
      containing `Key::F` in screen code and `"press f"` in help text (tests).
- [x] CI and the `run-gates` skill run `cargo xtask check-keys`.
- [x] No default key changed (existing keymap tests and snapshots pass
      unchanged).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `keymap.ron` loader accepts the `layout_picker` section and rejects
  an unknown action or bad chord in it; `check-keys` scanner on fixtures
  (hit, allowed file, `#[cfg(test)]` skip, `// check-keys:` marker).
- Integration: existing layout-picker Harness tests unchanged.

## Completion notes

Audited all non-test Rust in `crates/ui`, `crates/app`, `crates/content`
and all text in `assets/` (every `.ron` except `keymap.ron`, every `.dlg`,
`assets/dialogue/README.md`). The code was already close: help bars, tips
and messages all use `widgets::help` lookups or `{Action}` placeholders.
What was found and fixed:

| file:line (before) | What it was | What it is now |
| --- | --- | --- |
| `crates/ui/src/input.rs:43-61` | `Keymap::layout_picker(repeat)` built the pre-layout keys (`Up`/`w`, `Down`/`s`, `f`/`j`/`Enter`/`Space`) from `Key::` constants | `Keymap::layout_picker(&KeymapDef)` builds from the new `layout_picker` section of `assets/data/keymap.ron` (same 8 keys), validated by the loader (unknown action, bad or doubled chord = error; unlisted actions unbound) |
| `crates/ui/src/widgets/help.rs:30-50` | `cursor_keys_name` compared against `Key::Up`/`Left`/`Down`/`Right` and wrote `"arrows"` | Logic moved to `Keymap::cursor_keys_name` in `input.rs` (the pipeline file); `help::cursor_keys_name` delegates. Same output |
| `crates/ui/src/screens/layout_picker.rs:40-63` | `TOP_ROW`/`HOME_ROW` `Key` arrays (keyboard picture) | Kept, marked `// check-keys: keyboard picture`. Highlighting already came from the keymap (`key_role` → `km.action(..)`) |
| `crates/ui/src/screens/layout_picker.rs:186-199` | Arrow caps and space bar named `Key::Up`… `Key::Space` inline in `draw_panel` | Moved to marked picture constants `UP_CAP`, `LOWER_ARROW_CAPS`, `SPACE_BAR`; `key_role` marked too |
| `crates/ui/src/screens/layout_picker.rs:105` | Doc comment quoted `w/Up s/Down choose · f/j/Enter/Space pick` | Names the actions (Cursor up/down, Confirm) |
| `crates/ui/src/screens/dialogue.rs:48,142` | Doc comments: "End Turn key (Space)", "Space in every layout" | "the End turn key"; code already used only `Action::Confirm`/`Action::EndTurn` |
| `crates/ui/src/screens/battle/mod.rs:1088` | Doc comment example `w danger zone: OFF · Shift+Space auto-end: ON` | Names the Danger zone / Auto-end keys; code already used `key_name` |
| `crates/ui/src/screens/title.rs:98,168` | Doc comment examples `arrows move · f select · d back`, `press d to go back` | Name the actions; code already used lookups |
| `crates/ui/src/harness.rs` | `Chord::parse` for scripted key presses | Test-only (`cfg(any(test, feature = "harness"))`): allowed, listed in the scanner as test support |
| `crates/app/src` (outside `keys.rs`) | — | None found |
| `crates/content/src` (outside `keymap.rs`) | — | None found |
| `assets/data/*.ron`, `assets/audio/audio.ron`, `assets/fonts/atlas.ron`, `assets/dialogue/*` | — | None found (`tips.ron` already uses placeholders) |

Guard: `cargo xtask check-keys` (`crates/xtask/src/check_keys.rs`), in CI's
`tickets` job and the `run-gates` skill; `real_repo_has_no_hard_coded_keys`
and `check_keys_passes_on_the_real_repo` run it on the repo in
`cargo test`. It is a small line lexer (comments, strings, raw strings,
char literals vs lifetimes) that skips `#[cfg(test)]` items by bracket
counting and test files by name. The `// check-keys: keyboard picture`
marker is honoured only in `layout_picker.rs` and is an error anywhere else.

Deviations:
- The help-hint rule only flags a *lowercase* single letter followed by a
  word (`"f select"`), because keys print lowercase and capitals are names
  (`"Battle Theme B for RPG"` in `audio.ron` was a false positive).
  `"press F"` and `"[F]"` are still caught in any case.
- `.dlg` dialogue scripts get a narrower rule than code and RON strings,
  because they're prose: a line fails only when it tells the player to
  press a key (`press`/`hit`/`tap`/`hold` + a letter, a key name or F1–F12),
  names a `Shift+` chord or `WASD`, or has a bracketed letter (`[F]`).
  "Escape while you can!", "a volley of arrows" or "Plan B" pass.
- The 0705 merge left a stale `tickets/open/0905-…` next to
  `tickets/done/0905-…`, which failed ticket-lint on `main` and in this
  branch's tests. Removed the stale open copy in its own commit.

No gameplay rules were decided. No default key changed.

