---
id: "0208"
title: Right- and left-handed key layouts, first-launch layout picker
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0204", "0205", "0207"]
nick_input: sign-off
completed:
---

# 0208 — Right- and left-handed key layouts, first-launch layout picker

## Context

Nick chose two keyboard layouts and a picker on first launch
([`docs/design/controls.md`](../../docs/design/controls.md), ticket 0015).
0204 shipped a single keymap that is the right-handed layout.
[ADR-0015](../../docs/adr/0015-input-actions-and-keymap-layouts.md): layouts
are data; the chosen layout is a saved setting; Options overrides (0805)
apply on top.

## Nick input

**Sign-off:** on first launch, pick each layout and check every key in the
table in `docs/design/controls.md` does what it says.

## Scope

**In:**
- `keymap.ron` holding both layouts; loader validates each one.
- `Key::Semicolon` (the left-handed Previous-unit key) + `app` mapping.
- `Layout` enum (`RightHanded`, `LeftHanded`) and building `Keymap` for it.
- A first-launch "Pick your layout" screen; the choice saved via `Storage`.

**Out (do not do):**
- Options-menu layout switch and per-key rebinding (0805).
- A fast-cursor key (Nick: revisit after a large-map playtest).
- Controller or mouse support.

## Implementation steps

1. `keymap.ron` format: `layouts: { "RightHanded": { <bindings> },
   "LeftHanded": { <bindings> } }` plus the shared `repeat`. Bindings exactly
   per the table in `docs/design/controls.md` (right-handed = today's file;
   left-handed = `WASD` move, `j` confirm, `k`/`Escape` cancel, `;` previous
   unit, `l` next unit, `i` info, `o` danger zone, `Space` end turn,
   `Shift+Space` auto-end, `F12` debug).
2. `KeymapDef` loader: every layout must be present and validated like a single
   keymap (all errors reported, prefixed with the layout name); unknown layout
   names are errors.
3. `Key::Semicolon` (name `";"`) in `trpg-content::keymap`; `app` maps
   `KeyCode::Semicolon` to it.
4. `trpg-ui::input::Keymap::for_layout(&KeymapDef, Layout)`.
5. **Layout picker screen** (screen stack from 0205): title `Pick your layout`,
   two options, each with a small key diagram (use the `ascii-art` skill;
   mockups in `controls.md`). Choose with Up/Down *or* W/S and confirm with
   F, J, Enter or Space, so it works before any layout is chosen. Save the
   choice (`Storage` key `layout`) and switch `InputState` to the new keymap.
6. On startup: if no saved layout, push the picker first; otherwise load the
   saved layout.
7. Help texts read key names from the active keymap (never hard-coded).

## Acceptance criteria

- [ ] Both layouts load and validate; a broken layout gives clear errors (tests).
- [ ] Each layout's bindings match `docs/design/controls.md` exactly (test per layout).
- [ ] First launch shows the picker; the choice persists across restart (`MemoryStorage` round-trip).
- [ ] After picking left-handed, `W A S D` move and `J` confirms (Harness test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: loader errors for a missing layout, an unknown layout, and a conflict inside one layout; `Key::Semicolon` round-trips.
- Snapshot: picker screen.
- Harness: first launch → pick → saved; second launch skips the picker.

## Completion notes
