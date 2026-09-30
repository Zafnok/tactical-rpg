---
id: "0220"
title: "Help bars and tips name controller buttons when the controller was used last"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0032", "0217", "0219"]
nick_input: sign-off
completed:
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
(or whatever 0032 decided). Mockups first if new glyphs are needed.

## Scope

**In:**
- "Last used device" (keyboard or controller) tracked in `InputState`,
  switched as 0032 decided.
- Help bars and tip placeholders name the action's button(s) when the
  controller is the current device; `! not mapped` when it has none.
- The naming style from 0032, including controller-type detection
  (Xbox / PlayStation / Nintendo / other) if 0032 asked for it.
- New font glyphs if the style needs them (e.g. PlayStation shapes).
- The layout-picker rule for controller players from 0032.

**Out (do not do):**
- Rebinding buttons (0816). Steam Input glyph images (0903).
- Any change to which button does what.

## Implementation steps

1. **Device** (`crates/ui/src/input.rs`): `pub enum Device { Keyboard,
   Pad(PadKind) }`, `PadKind { Xbox, PlayStation, Nintendo, Generic }`.
   `InputState::device()` returns the device of the last *bound* press
   (ignore stick noise below the press threshold). `app` passes the pad's
   kind with its events: gilrs gives the pad's name / vendor id; on web
   `Gamepad.id` contains the vendor and product ids. Map known vendor ids
   (Microsoft 045e, Sony 054c, Nintendo 057e) and fall back to `Generic`.
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

- [ ] Nick approved the look (note the option in Completion notes).
- [ ] Harness test: after a pad press the battle help bar shows the
      Confirm button's name; after a key press it shows the key again
      (or per 0032's rule).
- [ ] A tip with `{Confirm}` shows the button name on a pad (test).
- [ ] An action with no button shows `! not mapped` on a pad (test).
- [ ] Vendor-id → `PadKind` mapping unit-tested, unknown → `Generic`.
- [ ] Snapshots of a help bar and a tip for keyboard and each pad style used.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Device` switching, names per style, vendor mapping.
- Snapshot: help bar and tip in each style.
- Integration: Harness pad → help text switch.

## Completion notes

*(Filled in by the session that completes the ticket.)*
