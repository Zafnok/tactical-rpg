---
id: "0815"
title: "Key bindings screen: press a key to bind, taken keys move, ! not mapped"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: done
blocked_by: ["0217", "0218"]
nick_input: sign-off
completed: 2026-09-30
---

# 0815 — Key bindings screen

## Context

Nick's rules: [`docs/design/controls.md`](../../docs/design/controls.md),
*Rebinding keys* (ticket 0030). In short: each action has 3 slots; pick a
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

- [x] Nick approved the look from mockups (note which option in Completion
      notes).
- [x] Binding `e` to Confirm's slot 2 in right-handed moves it from Info;
      Info shows `! not mapped`; after leaving, `e` confirms and nothing
      opens Info (Harness test).
- [x] With Confirm's only key moved away, Cancel does not leave and the
      message names Confirm; binding a new Confirm key then allows leaving
      (Harness test).
- [x] `Esc` during capture changes nothing; `Delete` during capture is
      ignored; `Delete` on a slot empties it (Harness tests).
- [x] Navigation still works after the player moves every cursor key to
      other actions (Harness test: the opened-with keymap drives the screen).
- [x] Edits persist across restart and per layout (MemoryStorage Harness
      test: edit right-handed, restart, still there; left-handed unchanged).
- [x] Restore defaults resets only the current layout (test).
- [x] Snapshots: list, capture prompt, `! not mapped` row, blocked-leave
      message.
- [x] `cargo xtask check-keys` passes (no key named in the screen code).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: capture state machine (abort, clear ignored, reserved, bind, move).
- Snapshot: the four states above.
- Integration: Harness scripts above.

## Completion notes

**Look (Nick, 2026-09-30).** Three looks were rendered with the real font
(A plain list as in the sketch, B a panel with "Must have a key" /
"Optional" groups, C the list plus a keyboard picture of taken keys). Nick
picked **B: grouped panel** and **required first, then optional** for the
row order. Recorded in `docs/design/controls.md` (*Rebinding keys*, "The
screen's look"), with screenshots of the built screen in
`docs/screenshots/0815-key-bindings*.png`.

**Done.**
- `FrameInput::pressed_chords` (every chord pressed this frame), filled by
  `Game`; `input::{is_capture_abort, is_clear_slot, capture_abort_key_name,
  clear_slot_key_name, CAPTURE_PROMPT}`.
- `KeyBindingsScreen` (`crates/ui/src/screens/key_bindings.rs`): capture,
  moved keys, Delete, blocked leave, Restore defaults, help bar. It edits a
  copy and saves with `Ctx::set_layout_bindings` on leaving, so the keys it
  was opened with steer it throughout.
- Debug menu → "Key bindings" (last entry). 0805's ticket now says to open
  the screen from Options and drop that entry.

**Restore defaults asks first** (Nick, after the first push: "have a
confirmation screen for restore defaults"): Confirm on the row opens a
yes/no question; Cancel there answers no and stays on the screen. Step 6
of the plan restored at once.

**Deviations from the plan.**
- Rows are in Nick's order (`ROWS` in the screen module), not
  `Action::ALL` order.
- `FrameInput::new` keeps its signature; the chords are added with
  `with_pressed_chords`, so the many existing screen tests are untouched.
- The text `Press a key…` lives in `input.rs` (`CAPTURE_PROMPT`), next to
  the capture helpers: `check-keys` reads "Press a" as naming the key `a`,
  and `input.rs` is the file allowed to hold such text. The gate itself is
  unchanged.
- The scope listed sounds as out (0425), but 0425 has since landed and
  `crates/ui/README.md` asks every screen to play the menu sounds, so the
  screen plays the existing move / select / cancel / denied cues. No new
  sounds.
- The screen's name is in `debug::SCREENS`: the Debug key does nothing on
  it (otherwise pressing it at `Press a key…` would open the debug menu
  instead of showing "That key can't be used").

**Claude's starting rules** (Nick to veto at sign-off; also in
`controls.md`):
- The screen is steered with the keys you had when you opened it; your
  changes count from the moment you leave. (Already listed in 0030.)
- Up/down wrap round through the rows and Restore defaults; left/right
  stop at the first and third slot.
- The debug key (only in builds with the debug menu) can't be bound: it
  shows `That key can't be used` and the slot keeps waiting.
- When a key moves, the action that lost it lights up white for 1.5 s.
- If the key you press for a slot is one of your old cursor keys and you
  keep it held, the highlight doesn't run off: cursor moves wait until
  you let go.
- With two required actions unmapped, the message names the one listed
  higher on the screen.
- The restore question's wording (`Restore the default keys for
  Right-handed?`) and look (the end-turn question's box).
- Player-facing action names: Cursor up/down/left/right, Confirm, Cancel,
  End turn, Select, Confirm end turn, Previous ready unit, Next ready unit,
  Unit info, Danger zone, Auto-end on/off, Rewind, Map menu (from the
  table in `controls.md`).

**For Nick's sign-off:** F2 → Key bindings (bottom of the debug menu).

**Follow-ups:** none new. 0805 (Options opens the screen) and 0816
(controller buttons on the same screen) were already ticketed.
