---
id: "0809"
title: "Key bindings screen: press a key to bind, taken keys move, ! not mapped"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0217", "0218"]
nick_input: sign-off
completed:
---

# 0809 — Key bindings screen

## Context

Nick's rules: [`docs/design/controls.md`](../../docs/design/controls.md),
*Rebinding keys* (ticket 0022). In short: each action has 3 slots; pick a
slot, press any key, and it goes there; a key already used elsewhere moves
and the action that lost it shows `! not mapped` if it has no key left;
`Esc` backs out and is never bindable; `Delete` empties a slot; leaving is
blocked while a required action has no key; each layout keeps its own keys.

0217 built the model (`LayoutBindings`: `bind`, `clear`,
`unmapped_required`, `defaults`; `Ctx::set_layout_bindings`; `NOT_MAPPED`)
and 0218 added the optional Select and Confirm end turn actions. This ticket
is only the screen. The Options menu (0805) opens it from its "Key
bindings" row; until 0805 lands, reach it from the debug menu (F2 in debug
builds, `crates/ui/src/debug.rs`). Follow the `keyboard-input` and
`ascii-art` skills.

## Nick input

**Sign-off:** open the screen (debug menu → Key bindings until Options
exists), and:
- bind a key another action uses: it moves, and that action shows
  `! not mapped`;
- try to leave with Confirm unmapped: it's blocked with a message;
- `Esc` while "Press a key…" backs out; `Delete` empties a slot;
- give Select and Confirm end turn their own keys and play a turn;
- check the Claude's starting rules listed in `controls.md` and below.

## Scope

**In:**
- `KeyBindingsScreen` for the active layout: every rebindable action in
  `Action::ALL` order with its 3 slots, `Esc` shown as fixed on Cancel.
- Capture mode, conflicts, `Delete`, blocked leave, Restore defaults row.
- Raw key presses in `FrameInput` for capture.
- Debug-menu entry "Key bindings".

**Out (do not do):**
- The Options menu itself and its Layout row (0805).
- Switching layout from this screen (0805's Layout row does that).
- Controller or mouse input. Sounds (0425).

## Implementation steps

1. **Raw presses** (`crates/ui/src/screen.rs`, `game.rs`): add
   `FrameInput::pressed_chords: Vec<Chord>`, the chords pressed this frame
   in order (presses only, no repeats), filled by `Game::frame` from the
   `RawKeyEvent::Down`s. Add to `crates/ui/src/input.rs` (allow-listed by
   `check-keys`): `pub fn is_capture_abort(c: Chord) -> bool` (plain
   `Escape`) and `pub fn is_clear_slot(c: Chord) -> bool` (plain `Delete`),
   so the screen never names a key itself.
2. **Mockup first** (`ascii-art` skill): draw the screen at the console
   size and show Nick 2–3 options for the look before building (his visual
   reviews usually take a few rounds). A starting sketch, not decided:
   ```
    Key bindings: Right-handed
    ------------------------------------------------------------
    Cursor up           [Up    ]  [      ]  [      ]
    Cursor down         [Down  ]  [      ]  [      ]
    Confirm             [f     ]  [      ]  [      ]
    Cancel              [d     ]  [      ]  [      ]   + Esc
    Select              [      ]  [      ]  [      ]   ! not mapped
    End turn            [Space ]  [      ]  [      ]
    Confirm end turn    [      ]  [      ]  [      ]   ! not mapped
    ...
    Restore defaults
    ------------------------------------------------------------
    f bind · Delete clear · d back
   ```
   Action names come from one table in the screen module (player-facing
   labels). Required actions with no key are drawn in the warning colour;
   optional ones in the dim colour.
3. **Navigation** uses the keymap the screen was **opened with** (a copy
   kept in the screen), not the one being edited, so the player can never
   lose the keys needed to move around this screen; edits apply to the game
   when the screen closes. *(Claude's starting rule: note it for Nick's
   sign-off.)* Up/Down move between rows and wrap; Left/Right between the 3
   slots; the Restore defaults row has no slots.
4. **Capture:** Confirm on a slot → the slot shows `Press a key…`. The next
   `pressed_chords` entry:
   - `is_capture_abort` → leave capture, nothing changed;
   - `is_clear_slot` → ignored (stay in capture);
   - reserved (0217's `BindError::Reserved`, e.g. Debug's key in debug
     builds) → a short message "That key can't be used", stay in capture;
   - anything else → `LayoutBindings::bind`; if it returned the slot the key
     came from, briefly highlight that row. Leave capture.
   Actions and repeats that frame are ignored while capturing.
5. **Delete** on a slot (not capturing) → `LayoutBindings::clear`.
6. **Restore defaults** row: Confirm → `LayoutBindings::defaults` for the
   current layout (the other layout untouched).
7. **Leaving** (Cancel, which includes the fixed `Esc`): if
   `unmapped_required()` is empty, save with `Ctx::set_layout_bindings`
   (0217) and pop; otherwise stay and show `Give <action> a key first`
   naming the first one.
8. Help bar via `widgets::help` (`key_name` of the *opened-with* keymap for
   bind/back; `Delete` via a name helper next to `is_clear_slot`).
9. Debug menu: add "Key bindings" to `TOOLS` in `crates/ui/src/debug.rs`,
   pushing the screen; add its name to `SCREENS` if the debug key should do
   nothing on it.

## Acceptance criteria

- [ ] Nick approved the look from mockups (note which option in Completion
      notes).
- [ ] Binding `e` to Confirm's slot 2 in right-handed moves it from Info;
      Info shows `! not mapped`; after leaving, `e` confirms and nothing
      opens Info (Harness test).
- [ ] With Confirm's only key moved away, Cancel does not leave and the
      message names Confirm; binding a new Confirm key then allows leaving
      (Harness test).
- [ ] `Esc` during capture changes nothing; `Delete` during capture is
      ignored; `Delete` on a slot empties it (Harness tests).
- [ ] Navigation still works after the player moves every cursor key to
      other actions (Harness test: the opened-with keymap drives the screen).
- [ ] Edits persist across restart and per layout (MemoryStorage Harness
      test: edit right-handed, restart, still there; left-handed unchanged).
- [ ] Restore defaults resets only the current layout (test).
- [ ] Snapshots: list, capture prompt, `! not mapped` row, blocked-leave
      message.
- [ ] `cargo xtask check-keys` passes (no key named in the screen code).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: capture state machine (abort, clear ignored, reserved, bind, move).
- Snapshot: the four states above.
- Integration: Harness scripts above.

## Completion notes
