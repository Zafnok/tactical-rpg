---
id: "0220"
title: "Help bars and tips name controller buttons when the controller was used last"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0032", "0217", "0219"]
nick_input: sign-off
completed: 2026-10-01
---

# 0220 — Help bars and tips show controller buttons

## Context

After 0219 a controller drives the game, but help bars and tips still name
keyboard keys (`f confirm · d back`), which is wrong for someone holding a
pad. 0032 (`docs/design/controls.md`, *Controller*) decided how buttons are
named on screen (letters, shapes, per controller type or by position) and
when the text switches between keyboard and controller. Steam Deck Verified
also requires controller prompts, not keyboard ones (0903).

Help text already goes through `crates/ui/src/widgets/help.rs`
(`key_name`, `all_key_names`, `cursor_keys_name`) and tip placeholders
(`crates/ui/src/tips.rs`, `fill_placeholders`), reworked by 0217
(`! not mapped`). This ticket teaches those helpers about buttons. Follow
the `keyboard-input` and `ascii-art` skills.

## Nick input

**Answer first:** 0032 (button naming style, switching rule).

**Sign-off:** on Pages, play a Quick Battle with the controller and check
the help bar and a tip name buttons; press a key and check they switch back
(0032: help text follows whatever was pressed last). Mockups of the new
PlayStation glyphs first.

## Scope

**In:**
- "Last used device" (keyboard or controller) tracked in `InputState`:
  help text follows whatever was pressed last (0032 Q5 A), no setting.
- Help bars and tip placeholders name the action's button(s) when the
  controller is the current device; `! not mapped` when it has none.
- Names follow the pad in use (0032 Q4 B), using 0219's `PadKind`: Xbox
  letters (also Steam Deck and generic pads); PlayStation `✕ ○ □ △`,
  `L1 R1 L2 R2`, `Options`, `Create`; Switch `A B X Y`, `L R ZL ZR`, `+`,
  `−`. Tables in `controls.md` *Controller → Button names on screen*.
  The cursor names both D-pad and stick: `D-pad/stick move` with the
  defaults, or whatever the cursor is bound to after rebinding (Nick:
  "just render both somehow").
- New font glyphs for the PlayStation shapes: `□` and `△` are missing
  from Terminus (`✕` / `○` may need bolder versions to match). Mockup in
  `docs/screenshots/0032-button-names.png`.
- `InputState::device()` must be usable by 0226, which opens "Pick your
  layout" when the device switches from pad to keyboard.

**Out (do not do):**
- Rebinding buttons (0816). Steam Input glyph images (0903).
- The title prompt and when "Pick your layout" opens (0226).
- Any change to which button does what.

## Implementation steps

*Note from 0219 (done):* `PadKind` is in `crates/ui/src/input/pad.rs`.
`RawInputEvent::PadDown(Button)` doesn't carry the pad's kind yet:
`Pads::update` (which knows which pad pressed) returns `(Button, bool)`.
Extend it to return the pressing pad's kind too and pass it on through
`PadDown` to `InputState`. The button in the event is already the
*binding position* (on a Nintendo pad `South` means the physical right
button), so names must be looked up per kind from the binding position.

1. **Device** (`crates/ui/src/input.rs`): `pub enum Device { Keyboard,
   Pad(PadKind) }` (`PadKind` and its vendor mapping come from 0219).
   `InputState::device()` returns the device of the last *bound* press
   (ignore stick noise below the press threshold).
2. **Names** (`crates/ui/src/widgets/help.rs`): make `key_name`,
   `all_key_names` and `cursor_keys_name` take the device (or an
   `&InputState`) and name buttons in the 0032 style when it's a pad
   (`cursor_keys_name` → the D-pad / stick name). Button names live in one
   table next to the `Button` type, never in screens.
3. **Tips** (`crates/ui/src/tips.rs`): `fill_placeholders` uses the same
   helpers, so `{Confirm}` becomes the button name.
4. **Font** (only if needed): add the glyphs with `cargo xtask font-atlas`
   from `assets-src/`, check the font's license covers edits
   (`THIRD_PARTY_ASSETS.md`), mockups to Nick first (`ascii-art` skill).
5. Update every caller the compiler flags; screens still never name a
   key or button themselves.
6. `keyboard-input` skill: add the button rules (names through the same
   helpers, device-aware text).

## Acceptance criteria

- [x] Nick approved the look (note the option in Completion notes).
- [x] Harness test: after a pad press the battle help bar shows the
      Confirm button's name; after a key press it shows the key again
      (or per 0032's rule).
- [x] A tip with `{Confirm}` shows the button name on a pad (test).
- [x] An action with no button shows `! not mapped` on a pad (test).
- [x] Unit test: `device()` switches Keyboard → Pad → Keyboard with the
      presses.
- [x] Snapshots of a help bar and a tip for keyboard and each pad style used.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Device` switching, names per style, vendor mapping.
- Snapshot: help bar and tip in each style.
- Integration: Harness pad → help text switch.

## Completion notes

Done; ADR-0036 records the approach. Help bars, prompts and tips name the
controller's buttons after a button press and keys again after a key
press, on every screen.

**The look Nick approved:** option **A, "Thin, small"** of the four
rendered in `docs/screenshots/0220-ps-glyphs.png` (2026-10-01): the
PlayStation shapes are one cell each, as tall as a lower-case letter, with
thin lines.

**What was tested, and what wasn't.** Unit, snapshot and Harness tests
cover the device switching, every button's name on every kind of pad, the
help bar and a tip on keyboard / Xbox / PlayStation / Switch-style pads,
`! not mapped`, and that the longest battle help bar fits the row on every
pad. The web build was run in a real browser with a **simulated**
PlayStation pad: the lead screen, the battle's first tip and both bottom
rows showed `✕ ○ □ △`, `R1`, `Create`, `D-pad/stick`, and a key press
switched them back to keys. **No physical controller was available**, so
real pads are first tried by Nick's sign-off.

**Sign-off for Nick (after merge, on Pages):** press a controller button
and start a Quick Battle. The tip and the two bottom rows should name
your pad's buttons (`A select`, or `✕ select` on a PlayStation pad). Press
any keyboard key that does something (an arrow, say): they should name
keys again. Press a button: buttons again.

**Deviations from the steps**

- **Step 1:** the pressing pad's kind isn't returned by `Pads::update`.
  `Pads::kind_holding` answers it for a pressed button, and
  `RawInputEvent::from_pads` (replacing `RawInputEvent::pad`) builds the
  events, so `update`'s many tests stay as they are.
- **Step 2:** the helpers take neither a `Device` nor an `&InputState` but
  a `HelpKeys` (keymap + device) from `ctx.help_keys()`; `Game` copies
  `InputState::device()` into `Ctx::device` every frame, because screens
  draw from `&Ctx` and never see the `InputState`. 0226 reads `ctx.device`
  (note added there: it only follows presses that do something).
- **Step 2:** the button-name table is `PadKind::button_name` in
  `crates/ui/src/input/pad.rs`, next to `PadKind` (it needs the kind),
  not next to `Button` in `trpg-content`.
- **Step 4:** the glyphs are in a **second** BDF,
  `assets-src/fonts/pad-shapes.bdf`, which `cargo xtask font-atlas` now
  merges into the atlas; `ter-u16n.bdf` is untouched. Licence checked: the
  OFL allows a modified version that isn't named after the font
  (`assets/fonts/README.md`, `THIRD_PARTY_ASSETS.md`). The circle is a new
  glyph at U+25EF; Terminus's small `○` (U+25CB) is unchanged.
- **Not in the steps, added:** `Harness::use_pad(kind)` and
  `Harness::snapshot_as(device)`; the 0219 tests that compare pad play with
  keyboard play now compare the pad's screen as the keyboard would show it.
- **Scope:** `KeyBindingsScreen::help` takes `&Ctx` now. Its `Escape` /
  `Delete` hints still show on a pad; that screen's controller side is
  0816 (note added there).

**Follow-up tickets:** 0229 (PS4 pads should show `Share`; they show
`Create` like PS5 pads for now), 0230 (bug found here: giving Select a
*key* stops the controller's Confirm picking on the map). Notes added to
0226 and 0816.

*Claude's starting rules* (the design docs didn't say; Nick can veto;
also in `controls.md`, *Notes from building it (ticket 0220)*):

- **Only a press that does something switches the text.** A key or button
  with no job doesn't. Example: playing on a pad, you bump `q`, which does
  nothing: the help bar keeps naming buttons. You press an arrow key: it
  names keys.
- **Two pads of different kinds:** the text names the buttons of the pad
  pressed last.
- **A single D-pad direction is shown as an arrow.** Example: choosing an
  attack's target and Combat Art reads `←/→ target · ↑/↓ art`.
- **PS4 pads show `Create`**, not `Share`, until 0229.
- **The debug hint** (`F2 debug`) still names its key on a controller.
- **"Pick your layout"** always lists keys, since it is about the
  keyboard; its bottom line names buttons on a pad.
- **Names nobody sees until buttons can be rebound (0816):** a stick's
  single direction is `stick ↑` / `R-stick ↑`; pressing a stick in is
  `LS` / `RS` (PlayStation `L3` / `R3`); a cursor moved to other buttons
  is named by them (`R-stick move`, `Y/X/A/B move`).
