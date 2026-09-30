---
id: "0816"
title: "Rebind controller buttons on the Key bindings screen"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0032", "0219", "0220", "0815"]
nick_input: sign-off
completed:
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
- The screen is usable with only a controller (capture a button press;
  a fixed back-out and clear button chosen in 0032, never bindable).

**Out (do not do):**
- Changing default buttons (that's `keymap.ron` + 0032).
- Stick sensitivity or dead-zone settings (ticket it if Nick asks).

## Implementation steps

1. Mockup the controller view of the Key bindings screen (`ascii-art`
   skill), 2–3 options, before building.
2. Config: extend 0217's `PlayerKeys` with button slots (per layout or
   shared, per 0032); `version: 2`; repair-on-load rules as for keys
   (drop unknown / reserved / duplicate buttons, reset if a required action
   is left unmapped).
3. `PadBindings` model (pure), mirroring `LayoutBindings`; the "button
   moves" rule and reserved buttons from 0032. Property test the
   no-duplicate invariant.
4. Screen: raw button presses in `FrameInput` (like 0815's
   `pressed_chords`) for capture; controller back-out / clear via helpers in
   `input.rs` (never named in the screen).
5. Update the `keyboard-input` skill and 0217's ADR (or a new one) with the
   button config format.

## Acceptance criteria

- [ ] Nick approved the look and played with a rebound button (sign-off).
- [ ] Harness: move Confirm's button to Info; Confirm shows `! not mapped`
      and leaving is blocked if Confirm is required for buttons (per 0032).
- [ ] The screen can be used start to finish with pad events only (Harness).
- [ ] Old `version: 1` config loads with default buttons (test); edits
      persist across restart (MemoryStorage).
- [ ] Property test: no button in two slots, no reserved button in any slot.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit / property: `PadBindings`, config migration and repair.
- Snapshot: controller view of the screen, capture prompt.
- Integration: Harness pad-only rebind.

## Completion notes

*(Filled in by the session that completes the ticket.)*
