# ADR-0050: Player controller buttons: one shared set of slots in the saved key bindings (version 2)

- **Status:** Accepted
- **Date:** 2026-10-03
- **Related tickets:** 0816, 0032, 0217, 0219, 0815
- **Amends:** ADR-0031 (the saved config's format and version; its rules
  for keys stand) and ADR-0034 (buttons are no longer only the defaults)

## Context

ADR-0031 gave every action 3 key slots per layout, saved as `PlayerKeys`
version 1. ADR-0034 added controller buttons, always the defaults from
`keymap.ron`'s `pad` table. Nick decided buttons are rebound by the same
rules as keys (`docs/design/controls.md`, *Rebinding buttons*, ticket
0032): 3 slots, a taken button moves, the same required actions, **one
setup shared by both keyboard layouts**, **no fixed buttons**, and both
sticks' directions count as buttons. With no fixed button, a capture binds
a button when it is let go and holding one backs out.

## Decision

- **Model** (`trpg-ui::input::bindings`, pure): `PadBindings`, with the
  same editing API as `LayoutBindings` (`defaults`, `slots`, `find`,
  `bind`, `clear`, `is_unmapped`, `unmapped_required`) plus `pairs()` for
  `Keymap::with_pad`. Both wrap one private generic `SlotTable<T>`, which
  holds the shared rules (a thing is in at most one slot; binding one that
  is in a slot moves it). Keys add their reserved chords on top; **no
  button is reserved**, and Debug has no buttons. Invariant: a button is in
  at most one slot (property-tested).
- **Saved config:** still the `Storage` key `keybindings`, now
  ```ron
  PlayerKeys(
      version: 2,
      layouts: { …as in ADR-0031… },
      pad: Some({
          "Confirm": [Some("South"), None, None],
          …every rebindable action…
      }),
  )
  ```
  Buttons are saved by position name (`Button::name`, as in `keymap.ron`).
  `pad` is `None` (or absent) while the buttons equal the defaults, like a
  layout with no entry, so a later change to the default buttons reaches a
  player who never changed them.
- **Versions:** version 1 files are read as they are (keys only, default
  buttons) and written back as version 2 at the next save. Any other
  version resets everything to the defaults with a warning, as before.
- **Repair on load:** the buttons are repaired by the same function as one
  layout's keys (unknown actions and buttons and extra slots dropped, a
  button in two stored slots stays in the first, a stored button moves off
  a default slot, a required action left without a button resets the
  buttons to their defaults). Warnings about buttons start with
  `controller:`. Broken buttons never reset the keys, nor the reverse.
- **Wiring:** `PlayerKeys::keymap(def, layout)` is that layout's keys with
  the player's buttons; `PlayerKeys::layout_picker_keymap` is the layout
  picker's keys with them (a controller-only player may never pick a
  layout). `Ctx::pad_bindings` / `Ctx::set_pad_bindings` read and replace
  them, save, and rebuild `ctx.keymap` whatever the layout; `Game` hands a
  changed keymap to its input as for keys.
- **Raw button events for capture:** `FrameInput::pressed_buttons` and
  `released_buttons` (binding positions, bound or not, stick directions
  included), filled by `Game` from `RawInputEvent::PadDown` / `PadUp`.
  Only the Key bindings screen reads them.
- **Capture** (`screens/key_bindings.rs`): the first button to go down
  during a capture is watched; released before `HOLD_TO_CANCEL_SECS`
  (1 s, tunable) it goes in a slot that waits for a button, held that long
  it backs out. The frame it went down in doesn't count towards the hold.
  A button already down when the capture started is ignored.

## Consequences

- Switching right/left-handed never changes the buttons; a rebound button
  works in both layouts and before one is chosen.
- A player on the previous build keeps their keys; nothing to migrate by
  hand.
- A build older than this one resets a version 2 file to the defaults
  (ADR-0031's rule for unknown versions).
- The Key bindings screen can be used with only a controller once it is
  open. It names no button itself: names come from `PadKind::button_name`.
- The left stick's directions are named `L-stick` on screen (Nick, ticket
  0816), where ADR-0036 says `stick`.
- `Keymap::with_default_pad` remains for the defaults (`for_layout`,
  `layout_picker`, tests).

## Alternatives considered

- **Buttons per layout** (inside each layout's entry) — Nick chose one
  controller setup for both layouts (0032 Q6a A); it would also make a
  layout switch change the buttons under a controller player.
- **A second `Storage` key for the buttons** — two files to version and
  repair, and two writes when the screen closes, for one screen's data.
- **Bind a button on press, with Start / Back fixed for backing out and
  clearing** — Nick chose no fixed buttons (0032 Q6b B).
- **Keeping version 1 and adding an optional field** — a build that
  predates the buttons would silently drop them on its next save; the
  version bump makes it reset with a warning instead.
- **Copying `LayoutBindings` for buttons** — the move rule, slot checks and
  repair would exist twice and drift; the generic table keeps one copy.
