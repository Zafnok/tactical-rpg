---
id: "0229"
title: "PlayStation 4 pads show Share, not Create"
type: feature
milestone: M1 Engine
model: sonnet-5
effort: low
status: done
blocked_by: ["0220"]
nick_input: none
completed: 2026-10-02
---

# 0229 — PlayStation 4 pads show `Share`, not `Create`

## Context

`docs/design/controls.md` (*Controller → Button names on screen*, decided
in 0032) names the PlayStation left centre button `Create`, with "PS4:
`Share`". Ticket 0220 shows `Create` on every Sony pad, because a pad's
kind comes only from its USB **vendor** id (`PadKind::from_vendor`,
`crates/ui/src/input/pad.rs`) and a PS4 pad (DualShock 4) and a PS5 pad
(DualSense) share Sony's. Telling them apart needs the **product** id.
Found while working 0220; see ADR-0036.

The button does the same thing on both pads (Auto-end on/off by default);
only the word differs.

## Nick input

`None.` (The names were decided in 0032.) Not needed for the Chapter 1
playtest.

## Scope

**In:**
- Read each pad's USB product id next to its vendor id, on native and web.
- A DualShock 4 shows `Share` for the left centre button; every other
  button name is the PlayStation one, unchanged.

**Out (do not do):**
- Any other pad-specific names (Xbox `View` / `Menu`, Switch `Capture`…).
- Changing which button does what, or the Confirm / Cancel swap.
- Steam Input glyphs (0903).

## Implementation steps

1. `crates/ui/src/input/pad.rs`: add `PadKind::PlayStation4`, documented as
   "a Sony DualShock 4". Replace `PadKind::from_vendor(vendor)` with
   `PadKind::from_ids(vendor, product)`: Sony (`0x054c`) with product
   `0x05c4` or `0x09cc` (the two DualShock 4 models) or `0x0ba0` (its
   wireless adaptor) → `PlayStation4`; any other Sony product →
   `PlayStation`; other vendors as today, whatever the product.
2. Same file, `PadKind::button_name`: `PlayStation4` uses the PlayStation
   column except `Button::Select` → `"Share"`. Keep the one table (`names`);
   don't copy the column. `PadKind::position` treats it like `PlayStation`
   (no swap).
3. `crates/app/src/pads/native.rs`: pass `pad.product_id().unwrap_or(0)`.
4. Web: `web/gamepad.js` parses the vendor from `Gamepad.id`; add
   `trpg_pad_product(pad)` the same way (Chrome: `Vendor: 054c Product:
   09cc`; Firefox: `054c-09cc-…`), 0 when the browser doesn't say. Declare
   it in `crates/app/src/pads/web.rs` and bump `PLUGIN_VERSION` there and
   `version` in `web/gamepad.js` together (an xtask test keeps them equal).
5. `docs/design/controls.md`, *Notes from building it (ticket 0220)*:
   replace the sentence saying PS4 pads show `Create`.

## Acceptance criteria

- [x] Unit test: `from_ids` gives `PlayStation4` for `0x054c` + `0x05c4`,
      `0x054c` + `0x09cc` and `0x054c` + `0x0ba0`, `PlayStation` for `0x054c` + `0x0ce6`
      (DualSense) and `0x054c` + `0`, and ignores the product for other
      vendors.
- [x] Unit test: on `PlayStation4` every button is named as on
      `PlayStation` except `Select`, which is `Share`; names are still
      distinct and all in the font (extend
      `button_names_are_distinct_on_each_pad` and
      `every_button_name_is_in_the_font` in
      `crates/ui/src/input/pad/tests.rs` by adding the kind to `KINDS`).
- [x] Harness test (`crates/ui/tests/controller.rs`): with
      `h.use_pad(PadKind::PlayStation4)` the battle's toggle row shows
      `Share auto-end: OFF`.
- [x] The web plugin's two version numbers match (existing xtask test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: id → kind, names.
- Integration: the Harness test above.
- The web plugin can't be unit-tested; try it in a browser with a simulated
  `Gamepad` whose `id` holds a DualShock 4's ids (as 0219 did, by replacing
  `navigator.getGamepads`) and say in the Completion notes whether that
  was done.

## Completion notes

- `PadKind::PlayStation4` added; `PadKind::from_vendor` is now
  `PadKind::from_ids(vendor, product)`. Sony products `0x05c4`, `0x09cc`
  and `0x0ba0` are PS4 pads; any other Sony product (including 0, "not
  reported") stays `PlayStation`. `button_name` keeps the one `names`
  table and only overrides `Select` → `Share`; `position` is unchanged
  (no swap).
- Native reads `product_id()` from gilrs; web gets the new
  `trpg_pad_product` from `web/gamepad.js` (plugin version 1 → 2 on both
  sides).
- Tests: `usb_ids_give_the_pad_kind` (replaces the vendor-only test),
  `a_dualshock_4_says_share_and_otherwise_names_as_a_playstation_pad`,
  `PlayStation4` added to `KINDS` and the other kind lists (no swap,
  Confirm/Cancel, proptest), and the harness test
  `a_playstation_4_pad_names_auto_end_share`.
- **Web check done** in the browser pane with a simulated pad
  (`navigator.getGamepads` replaced): the Chrome-style id `Vendor: 054c
  Product: 09cc` showed `Share auto-end: OFF` in a Quick Battle; switching
  the id to `0ce6` (DualSense) showed `Create`; the Firefox-style id
  `054c-05c4-…` showed `Share` again.
- Not tried with a real PS4 pad (none available).
- No deviations, no follow-up tickets, no gameplay rules decided.
