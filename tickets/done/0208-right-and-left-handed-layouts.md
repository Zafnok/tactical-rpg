---
id: "0208"
title: Right- and left-handed key layouts, first-launch layout picker
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0204", "0205", "0207"]
nick_input: sign-off
completed: 2026-09-25
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

- [x] Both layouts load and validate; a broken layout gives clear errors (tests).
- [x] Each layout's bindings match `docs/design/controls.md` exactly (test per layout).
- [x] First launch shows the picker; the choice persists across restart (`MemoryStorage` round-trip).
- [x] After picking left-handed, `W A S D` move and `J` confirms (Harness test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: loader errors for a missing layout, an unknown layout, and a conflict inside one layout; `Key::Semicolon` round-trips.
- Snapshot: picker screen.
- Harness: first launch → pick → saved; second launch skips the picker.

## Completion notes

- `assets/data/keymap.ron` now has `layouts: { "RightHanded": {..},
  "LeftHanded": {..} }` plus a shared `repeat`. `KeymapDef` holds
  `layouts: BTreeMap<Layout, Bindings>`; the loader validates each layout like
  a single keymap (every error prefixed `layout "X": `, line numbers point
  inside that layout's block) and reports missing and unknown layouts.
- `Layout` lives in `trpg-content::keymap` (the loader needs it) and is
  re-exported from `trpg_ui::input`. `Key::Semicolon` (`";"`) added; `app`
  maps `KeyCode::Semicolon` to it.
- `Keymap::from_def` is replaced by `Keymap::for_layout(&KeymapDef, Layout)`
  and `Keymap::new(bindings, repeat)`.
- **Before a layout is chosen** the active keymap is
  `Keymap::layout_picker`, defined in code rather than data: `Up`/`w`,
  `Down`/`s` move and `f`/`j`/`Enter`/`Space` confirm, exactly as step 5 asks.
  It isn't one of the player's layouts, so it isn't in `keymap.ron`.
- `Ctx` tracks the layout (`layout()`, `use_layout`, `choose_layout`, which
  saves under the `Storage` key `layout`, and `saved_layout`). The saved value
  is the layout name, e.g. `LeftHanded`; an unreadable or unknown value
  means the picker shows again. If saving fails, the layout is still used for
  the session.
- `Game::start` loads the saved layout, or pushes `LayoutPickerScreen` over
  the title on first launch. When a screen changes layout, `Game` gives
  `InputState` the new keymap (`InputState::set_keymap` forgets held keys, so
  the picking keypress can't leak into the new layout).
- The picker cannot be cancelled. Each option is a panel with a QWERTY
  diagram (movement keys in `player` blue, action keys in `text_highlight`,
  unused keys dim) and a legend; both come from that layout's bindings.
  `controls.md` had no mockups, so the design is new; the snapshots are in
  `crates/ui/tests/snapshots/layout_picker__*.snap`.
- Harness: `Harness::new()` is now a first launch (picker on top);
  `Harness::with_layout(layout)` starts at the title with a saved layout;
  `into_storage()` / `with_storage()` model a restart. The existing title
  tests use `with_layout(RightHanded)`.
- No follow-up tickets. Changing layout from Options stays with 0805 (the
  picker screen can be reused there).

**For Nick (sign-off):** on first launch you'll see "Pick your layout". Choose
with Up/Down or W/S, pick with F, J, Enter or Space. Then check each key in
the table in `docs/design/controls.md`. Only Confirm, Cancel and cursor
movement do anything visible so far (title menu); the other keys have no
screens yet. To see the picker again, delete `layout.ron` in the game's
data folder (`%APPDATA%\tactical-rpg` on Windows; on web, clear the site's storage).
