---
id: "0816"
title: "Rebind controller buttons on the Key bindings screen"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: done
blocked_by: ["0032", "0219", "0220", "0815"]
nick_input: sign-off
completed: 2026-10-03
---

# 0816 — Rebind controller buttons

## Context

Players rebind every key (`docs/design/controls.md`, *Rebinding keys*;
0217 config, 0815 screen). 0219 added controller buttons with defaults
from `keymap.ron`, and 0220 names them in help bars. This ticket lets the
player rebind buttons too, following the rules 0032 set for buttons
(`controls.md`, *Controller*): slots per action, which actions must keep a
button, one setup or one per layout. Follow the `keyboard-input` skill.

## Nick input

**Answer first:** 0032 (rebinding rules for buttons).

**Sign-off:** open Options → Key bindings with a controller, move a button
to another action, check the `! not mapped` flag and blocked leave, and
play a turn with the new button.

## Scope

**In:**
- Button slots in the saved `keybindings` config (0217): format version
  bump, old configs load with default buttons.
- A pure editing API for buttons, same shape as 0217's `LayoutBindings`
  (`bind`, `clear`, `defaults`, `unmapped_required`).
- The Key bindings screen (0815) shows and edits buttons: a Keyboard /
  Controller switch or column, as mocked up for Nick.
- The screen is usable with only a controller (0032 Q6b), with **no
  fixed buttons**: while `Press a button…` shows, a button goes in the
  slot when released; **holding any button ~1 s** (*tunable*) backs out
  with no change. When the pad was used last, Confirm on a slot offers
  **`Clear`** (with the keyboard, Confirm still goes straight to `Press a
  key…` and `Delete` empties). A hint says so, e.g. `Press a button… ·
  hold any button to cancel`.
- Rules as for keys (0032 Q6a): 3 slots, a taken button moves,
  `! not mapped`, leaving blocked while cursor ×4, Confirm, Cancel or End
  turn has no button. **One button setup shared by both keyboard
  layouts.**

**Out (do not do):**
- Changing default buttons (that's `keymap.ron` + 0032).
- Stick sensitivity or dead-zone settings (ticket it if Nick asks).

## Implementation steps

*Notes from 0220 (done):* every button already has an on-screen name
(`PadKind::button_name`, `crates/ui/src/input/pad.rs`), stick directions
and stick presses included; nobody has seen those last ones yet (`stick ↑`,
`R-stick ↑`, `LS` / `RS`, PlayStation `L3` / `R3`), so show them in step
1's mockup and let Nick change them (`controls.md`, *Notes from building it
(ticket 0220)*). The slot cells should use that function, with `ctx.device`
giving the pad's kind. The Key bindings screen's help line already names
buttons on a pad (`KeyBindingsScreen::help(ctx)`), but its capture and
clear hints still name `Escape` and `Delete`: replace them when the pad was
used last. Bug 0230 (a Select *key* stops the pad's Confirm picking on the
map) is separate; if it isn't fixed yet, don't work around it here.

1. Mockup the controller view of the Key bindings screen (`ascii-art`
   skill), 2–3 options, before building.
2. Config: extend 0217's `PlayerKeys` with one shared set of button
   slots (not per layout); `version: 2`; repair-on-load rules as for keys
   (drop unknown / duplicate buttons, reset if a required action is left
   unmapped). Both sticks' directions can be bound, like buttons (Nick:
   "right stick should be available to use if wanted").
3. `PadBindings` model (pure), mirroring `LayoutBindings`; the "button
   moves" rule, no reserved buttons. Property test the no-duplicate
   invariant.
4. Screen: raw button presses in `FrameInput` (like 0815's
   `pressed_chords`) for capture, with bind-on-release and
   hold-to-back-out; the `Clear` choice on a slot when the pad was used
   last. No button is named in the screen.
5. Update the `keyboard-input` skill and 0217's ADR (or a new one) with the
   button config format.

## Acceptance criteria

- [ ] Nick approved the look and played with a rebound button (sign-off).
      *Look approved from the mockups (2026-10-03); playing it is the
      sign-off after merge.*
- [x] Harness: move Confirm's button to Info; Confirm shows `! not mapped`
      and leaving is blocked.
- [x] Harness: holding a button during `Press a button…` backs out with
      nothing changed; a tap binds; `Clear` empties a slot.
- [x] Switching right/left-handed leaves the buttons unchanged (test).
- [x] The screen can be used start to finish with pad events only (Harness).
- [x] Old `version: 1` config loads with default buttons (test); edits
      persist across restart (MemoryStorage).
- [x] Property test: no button in two slots.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit / property: `PadBindings`, config migration and repair.
- Snapshot: controller view of the screen, capture prompt.
- Integration: Harness pad-only rebind.

## Completion notes

**Nick's picks (2026-10-03, from rendered mockups).**
- The screen: **B, a switch row** (`Keyboard · <layout>` / `Controller`)
  at the top of the one panel (over A "shows what you pressed last" and C
  "a menu first, two screens").
- Stick names: **name both sticks**: `L-stick ↑` and `R-stick ↑`; pressing
  a stick stays `LS` / `RS` (PlayStation `L3` / `R3`). The help bar now says
  `D-pad/L-stick move`.
- The choice on a slot: **`Change` / `Clear`**, on every slot.

Recorded in `docs/design/controls.md` (*Rebinding buttons*), with
screenshots of the built screen in `docs/screenshots/0816-*.png`.

**Done.**
- `PadBindings` (`crates/ui/src/input/bindings.rs`): the buttons' editing
  model, same shape as `LayoutBindings`; both sit on one private
  `SlotTable<T>`. No button is reserved; both sticks' directions bind.
- Saved config **version 2** with a `pad` entry (ADR-0050). Version 1
  files load with their keys and the default buttons. Repair on load as
  for keys; warnings about buttons start `controller:`.
- `Ctx::pad_bindings` / `set_pad_bindings`; the buttons apply in both
  layouts and before a layout is chosen.
- `FrameInput::pressed_buttons` / `released_buttons` (raw button events,
  from `Game`).
- The Key bindings screen: switch row, controller side, bind on release,
  hold 1 s to back out, `Change` / `Clear` when the pad was used last,
  blocked leave and Restore defaults per side. It names no button itself.
- `keyboard-input` skill, `crates/ui/README.md`, ADR index updated.

**Deviations from the plan.**
- The title is now just `Key bindings`; the layout's name moved into the
  switch row, and every row sits one line lower (part of look B).
- The left stick's name changed everywhere it shows (help bars, tips), not
  only on this screen, since Nick asked for both sticks to be named.
- New screen text went into `assets/lang/en/ui.ron` (`key_bindings.*`),
  because `check-text` (ADR-0045, landed while this was in progress) allows
  no new literals. `Press a button…` stays in `input.rs` next to `Press a
  key…` (`check-keys` reads "Press a" as naming a key), so the `Optional`
  heading moved to a text key too, and `MAX_LITERALS` went from 221 to 220.
  The rest of the screen's text is ticket 0234's.
- After a capture, cursor moves are now held back only if a cursor key or
  button really is still down (before: always until the next frame with
  none down). Needed because a button binds when it is let go.
- The ADR is 0050 (0049 is taken by two open PRs).

**Claude's starting rules** (Nick can veto; also in `controls.md`):
- The screen opens on the side you pressed last (buttons after a button,
  keys after a key).
- The switch row is one up from the first action. There, left shows the
  keys and right the buttons; Confirm flips it too.
- Hold a button for **1 second** to back out of `Press a button…`; let go
  sooner and it goes in the slot.
- Leaving is blocked for either side. The message is `Give Confirm a
  button first`, and the screen flips to the side that lacks one.
- Restore defaults restores only the side shown, after asking `Restore the
  default buttons?`.
- With the keyboard on the controller side: Confirm goes straight to
  `Press a button…`, `Esc` backs out, `Delete` empties the slot. A key
  pressed there does nothing.
- With a controller on the keyboard side: Confirm opens `Change` / `Clear`
  too; `Change` waits for a key and holding any button backs out.
- Until a controller is used, the controller side shows Xbox names.
- Rebound buttons count from the moment you leave the screen, like keys.

**For Nick's sign-off:** F2 → Key bindings (bottom of the debug menu),
then press a controller button (or go up one row and press right).

**Not done here:** bug 0230 (a Select *key* stops the pad's Confirm picking
on the map) is still open and wasn't worked around. A controller can't
open the screen by itself until the Options menu (0805) lists it: the
debug menu's key is keyboard only.

**Follow-ups:** none new.
