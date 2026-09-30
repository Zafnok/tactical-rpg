---
id: "0032"
title: "Decide: controller buttons, button names on screen, rebinding buttons"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-30
---

# 0032 — Decide: controller buttons

## Context

Nick (2026-09-29) wants to play with a controller soon, starting with Quick
Battle, on the web build, the itch.io downloads (Windows, Linux) and later
Steam / Steam Deck. The game is keyboard-first
([`docs/design/controls.md`](../../docs/design/controls.md)): screens only
see `Action`s (ADR-0015), so a controller is one more way to produce the
same actions. Which button does what is Nick's call, the same way he picked
every key (ticket 0015). This ticket asks him; 0219 (controller input),
0220 (button names in help bars) and 0816 (rebinding buttons) build it.

Asked with the `ask-nick` skill. Base options on real tactics games played
on a pad (Fire Emblem Three Houses / Engage, Advance Wars 1+2 Re-Boot Camp,
Triangle Strategy, Unicorn Overlord, Into the Breach, Wargroove, XCOM 2 on
console). Check each game's real layout before quoting it (see memory note:
honest game comparisons). Show a picture of a pad with the buttons labelled
for each option (`ascii-art` skill).

## Nick input

**Decision.**

## Questions to ask

Every action in `controls.md` needs a button or an explicit "no button":
cursor (4 directions), Confirm, Cancel, Previous / Next ready unit, Unit
info, Danger zone, End turn, Auto-end on/off, Rewind, Map menu, and the
optional Select / Confirm end turn (0218). Debug stays keyboard-only.

1. **Confirm and Cancel.** Bottom button confirms and right button cancels
   (Xbox / PlayStation / most PC games), or the Nintendo way (right
   confirms, bottom cancels)? Or follow the controller type?
2. **The other actions.** Which face / shoulder / trigger / Start / Select
   button for each (e.g. shoulders cycle units, like FE)? End turn is a
   double-tap of Space on keyboard: does the End turn button keep the
   double-tap?
3. **Moving the cursor.** D-pad, left stick, or both? Does the stick move
   one tile at a time with the same repeat as held keys (300 ms, then every
   55 ms), or faster when pushed further? Anything on the right stick (e.g.
   pan the camera, or nothing)?
4. **Button names in help bars and tips.** Today they show the player's
   keys (`f confirm · d back`). With a controller: Xbox letters (`A` `B`
   `X` `Y`), PlayStation shapes (✕ ○ □ △), follow whichever controller is
   plugged in, or position words (`Bottom`, `Right`)? Mock each in the
   game's glyph style (`docs/design/look-and-feel.md`): shapes may need new
   font glyphs.
5. **Switching between keyboard and controller.** Help bars show whatever
   was pressed last (most PC games), or a setting in Options? And the
   first-launch "Pick your layout" screen (right/left-handed keys): still
   shown to someone who presses a controller button first, or skipped
   until they touch the keyboard?
6. **Rebinding buttons.** Same rules as keys (`controls.md`, *Rebinding
   keys*: slots per action, a taken button moves, `! not mapped`, blocked
   leave)? How many buttons per action? Which actions must keep a button?
   One controller setup for both keyboard layouts (right/left-handed), or
   one per layout? On the Key bindings screen with only a controller, which
   fixed buttons back out of "Press a button…" and empty a slot (the
   keyboard uses `Esc` and `Delete`)?
7. **Rumble.** Any vibration (a crit, a unit dying), or none?

## Acceptance criteria

- [x] Answers recorded in `docs/design/controls.md` (new *Controller*
      section) with Nick's words verbatim, including a defaults table like
      the keyboard one.
- [x] `docs/design/README.md` row updated.
- [x] 0219, 0220 and 0816 updated to match the answers (e.g. drop rumble
      from 0219's out-list if Nick wants it, adjust 0816's rules).

## Completion notes

Asked Nick in three rounds with the `ask-nick` skill, with rendered
mockups: three pad layouts (`docs/screenshots/0032-controller-defaults.png`
is the chosen one) and a help bar + tip in each naming style
(`docs/screenshots/0032-button-names.png`). Answers: 1C (Confirm/Cancel
follow the controller), 2A (shoulders cycle units, Start ends the turn when
pressed twice), 3A (D-pad and left stick, one tile, key repeat; right stick
unused), 4B (names follow the pad), 5A (last device pressed), 6a A (same
rules as keys, one pad setup for both layouts), 6b B (hold any button to
back out, `Clear` on the slot, with a hint), 7: no rumble for now.

Recorded in `docs/design/controls.md` (*Controller*), README row updated,
and 0219, 0220 and 0816 rewritten to match (0219 now also detects the pad
kind, because the Confirm/Cancel swap needs it; 0220 skips the layout
picker for controller players).

Game comparisons only quote layouts a source confirmed (Three Houses,
Wargroove, Triangle Strategy, XCOM 2 console). Rumble and stick speed in
those games couldn't be confirmed, so no claims were made.

*Recorded without an explicit answer:* the "Pick your layout" screen is
skipped for someone whose first press is a pad button, and shown once when
they first touch the keyboard (my recommendation; Nick answered 5A without
the sub-question).

*Claude's starting rules* (in `controls.md`, Nick to veto at sign-off):
- Switch-style pads swap only Confirm's and Cancel's buttons; on a Switch
  pad Unit info stays on the top button, the danger zone on the left one.
- Rebinding by tapping: a button goes in the slot when you let go of it;
  held for a second it backs out instead.
- The `Clear` choice appears only when the pad was used last; the keyboard
  keeps Confirm → `Press a key…` and `Delete`.
- Left-stick directions can be rebound like buttons; the right stick can't.
- The help bar names the D-pad for moving (`D-pad move`).
- Several pads at once all drive the game, as one player.
