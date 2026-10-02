---
id: "0230"
title: "Bug: giving Select a key stops the controller picking on the map"
type: bug
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0220"]
nick_input: sign-off
completed:
---

# 0230 — Bug: giving Select a key stops the controller picking on the map

## Context

Two optional actions change what other actions do once they have a **key**
(`docs/design/controls.md`, *Optional split keys*, ticket 0218):

- **Select**: once it has a key, Confirm no longer picks units, tiles or
  targets on the map.
- **Confirm end turn**: once it has a key, End turn pressed again no longer
  accepts the end-turn prompt.

"Has a key" is `Keymap::select_action` and
`Keymap::end_turn_accept_actions` (`crates/ui/src/input.rs`), which look
only at **keyboard** keys, and `Mode::route`
(`crates/ui/src/screens/battle/mode.rs`) applies the result to every
press, whatever it came from. Neither action has a controller button
(`controls.md`, *Default buttons*), and buttons can't be rebound until 0816.

**Repro** (found while working 0220; `crates/ui/tests/controller.rs`,
`an_action_with_no_button_shows_not_mapped_on_a_pad` shows the help text):

1. Options → Key bindings: give Select a key (say `g`).
2. Start a battle and play with a controller.
3. Move the cursor onto one of your units and press the bottom button
   (Confirm).

**Expected:** the unit is selected, as before the key was bound: the
controller's Select still has no button, so its Confirm still picks.
**Actual:** nothing happens, and the help bar says `! not mapped select`.
The controller can't pick anything on the map until the key is removed.
The same goes for Start pressed twice once Confirm end turn has a key.

## Nick input

**Sign-off** on the rule below (it follows what Nick decided, but he
hasn't seen it spelled out for controllers):

> *Claude's starting rule:* the split is per device. Giving Select a
> **key** changes only what the keyboard's Confirm does; the controller's
> Confirm keeps picking on the map until Select has a **button** (0816).
> Same for Confirm end turn and Start. Example: Mia binds Select to `g`.
> On the keyboard `g` picks units and `f` only accepts menus; on her pad
> `A` still does both.

## Scope

**In:**
- `select_action` and `end_turn_accept_actions` answer for the device the
  press came from: keys for the keyboard, buttons for a controller.
- Help text and tips name the matching action for the device in use.

**Out (do not do):**
- Rebinding buttons, or giving Select / Confirm end turn a button (0816).
  If 0816 is done first, this bug still exists for players who bind the
  key but not the button, so this ticket stays as it is.
- Any change to keyboard behaviour.

## Implementation steps

1. `crates/ui/src/input.rs`: give `Keymap::select_action` and
   `Keymap::end_turn_accept_actions` a `Device` argument. For
   `Device::Keyboard` use `primary(...)` as today; for `Device::Pad(_)` use
   `primary_button(...)`.
2. `crates/ui/src/screens/battle/mode.rs`, `Mode::route`: take the device
   and pass it on. The battle screen calls it with `ctx.device`, which
   `Game` has already set from this frame's presses when the screen
   updates (ADR-0036). Check a frame holding a key press *and* a button
   press: both are routed by the device pressed last; say so in a comment
   (it is one frame, and the alternative is tagging every action with its
   device).
3. Callers that name the action in help text (`crates/ui/src/screens/battle/mod.rs`,
   `crates/ui/src/tips.rs`): `HelpKeys` (`crates/ui/src/widgets/help.rs`)
   knows its device; add `HelpKeys::select_action()` /
   `end_turn_accept_actions()` that pass it, and use those (they currently
   reach the keymap's through `Deref`).
4. Write the failing test first (below), then fix.
5. `docs/design/controls.md`, *Optional split keys*: add the rule, marked
   *Claude's starting rule (Nick to veto at sign-off of 0230)*.

## Acceptance criteria

- [ ] A Harness test that fails before the fix: with Select bound to a
      key, a controller's Confirm on a ready unit selects it, and the
      keyboard's Confirm does not.
- [ ] Harness: with Confirm end turn bound to a key, Start pressed twice on
      a controller ends the turn; End turn pressed twice on the keyboard
      does not.
- [ ] On the controller the help bar reads `A select` (not `! not mapped
      select`) in that setup; on the keyboard it reads `g select`.
- [ ] Unit tests of `select_action` / `end_turn_accept_actions` per device,
      including a keymap where Select has a button but no key.
- [ ] `crates/ui/tests/controller.rs`'s
      `an_action_with_no_button_shows_not_mapped_on_a_pad` is rewritten
      around an action that still has no button (it used Select).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the two keymap functions per device.
- Integration: the Harness tests above (`crates/ui/tests/split_keys.rs` or
  `controller.rs`).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
