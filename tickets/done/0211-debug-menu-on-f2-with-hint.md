---
id: "0211"
title: Open the debug menu with F2 and show a hint for it
type: tuning
milestone: M1 Engine
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-09-28
---

# 0211 — Open the debug menu with F2 and show a hint for it

## Context

The debug menu (ticket 0205, 0703; `crates/ui/src/debug.rs`) opens on the
`Debug` action, bound to `F12` in both layouts of `assets/data/keymap.ron`.
In the web build F12 opens the browser's developer tools, so Nick asked
(2026-09-28) to move it to a key that Chrome and Windows don't use, and to
show a hint on the title and battle screens while debug tools are being built:

> "change debug menu trigger from F12 because that is already bound to web
> inspector... let's use an unused chrome / windows keybind, something unique.
> You can also display it for now as a hint in battle / title screen"

Plain **F2** has no default action in Chrome, Edge, Firefox or the Windows
shell for a focused game window (F1 help, F3 find, F5 reload, F6 address bar,
F7 caret browsing, F9 Edge reader, F10 menu, F11 fullscreen, F12 dev tools).

The debug menu only exists when `Ctx::debug_tools` is on (debug builds and the
`debug-tools` Pages build, ADR-0023), so the hint is shown only then.

## Nick input

**Sign-off:** run a debug build (or the web preview); the title screen and a
Quick Battle show `F2 debug` in the bottom-right corner, and F2 opens the menu.

## Scope

**In:**
- Bind `Debug` to `F2` instead of `F12` in both layouts.
- Draw `<key> debug` right-aligned on the bottom row of the title and battle
  screens when `ctx.debug_tools` is on and the key is bound, skipping it when
  it would overlap the screen's own help text.
- Update docs/tests that name F12 for the debug menu.

**Out (do not do):**
- No hint on other screens; no new keys or modifiers in the keymap format.
- Don't change what the debug menu contains.

## Implementation steps

1. `assets/data/keymap.ron`: `"Debug": ["F2"]` in both layouts.
2. `crates/ui/src/widgets/help.rs`: add `draw_debug_hint(ctx, buf, row, used)`
   that prints `key_name(Debug) + " debug"` right-aligned (one cell margin) on
   `row` in `TextDim`, only if `ctx.debug_tools`, the key is bound, and it
   doesn't reach column `used` (end of the screen's help text + 1 gap).
3. Call it from `TitleScreen::draw` and from the battle screen's help-row
   drawing (`crates/ui/src/screens/battle/mod.rs`, both places that print
   `self.help(ctx)`).
4. Replace F12 with F2 in tests (`crates/ui/src/game.rs`,
   `crates/ui/src/harness.rs`, `crates/ui/tests/title.rs`,
   `crates/content/src/keymap.rs` layout tests) and docs
   (`crates/ui/README.md`, `assets/portraits/README.md`, `crates/ui/src/debug.rs`,
   `crates/ui/src/game.rs` doc comments).
5. Re-accept the snapshots that gain the hint.

## Acceptance criteria

- [x] F2 opens the debug menu (`f2_opens_the_debug_menu` in
      `crates/ui/tests/title.rs`); F12 no longer does.
- [x] Title and battle snapshots show `F2 debug` at the bottom right.
- [x] Unit tests: hint hidden when `debug_tools` is off, when `Debug` is
      unbound, and when the help text would collide.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `draw_debug_hint` cases above.
- Snapshot / integration: title and battle snapshots; F2 harness test.

## Completion notes

- `Debug` is bound to `F2` in both layouts; F12 now does nothing in-game
  (asserted in `f2_opens_the_debug_menu`).
- `draw_debug_hint` in `crates/ui/src/screens/mod.rs` prints `F2 debug`
  (key name from the keymap) right-aligned on the bottom row of the title and
  battle screens when debug tools are on.
- **Deviation:** instead of callers passing the column where their help text
  ends, the hint checks that its cells and the cell before them are blank and
  skips itself otherwise. Simpler for callers and fully covered by tests.
- Battle test helpers that read the help row now read only its left 90
  columns (the help text), since the hint shares the row.
- Updated the F12 mentions in `crates/ui/README.md`, `assets/portraits/README.md`,
  `crates/ui/src/debug.rs`, and open tickets 0704 and 0801. ADR-0023 still
  says "F12 glyph sampler" in its context; accepted ADRs aren't edited.
- No gameplay rules decided.
