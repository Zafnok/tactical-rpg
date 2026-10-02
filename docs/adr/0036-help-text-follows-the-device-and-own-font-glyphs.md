# ADR-0036: Help text follows the device pressed last; our own glyphs join the font from a second BDF

- **Status:** Accepted
- **Date:** 2026-10-01
- **Related tickets:** 0220, 0032, 0219, 0226, 0816, 0903
- **Amends:** ADR-0034 (button names, and `PadDown` now carries the pad's
  kind) and ADR-0016 (the atlas is built from more than one BDF). Nothing
  in either is superseded.

## Context

After ADR-0034 a controller drives the game, but help bars and tips still
named keys. Nick decided (0032, `docs/design/controls.md`, *Controller*)
that help text names **whatever was pressed last**, with no setting, and
names buttons **the way the pad in use labels them**: Xbox letters,
PlayStation shapes, Nintendo's letters.

Facts that shape the approach:

- Screens draw from `&Ctx` and name keys through `widgets::help`
  (`key_name`, `all_key_names`, `cursor_keys_name`) and tip placeholders.
  About 70 call sites pass `&ctx.keymap`.
- `Game` compares `ctx.keymap` with the input's keymap to notice rebinding,
  so the keymap itself must not carry anything that changes with every
  press.
- `Pads` merges every pad into one stream of button presses (ADR-0034);
  the pad's kind was dropped there.
- The font (Terminus, OFL-1.1) has no `□ △ ✕`, and its `○` is 4 px wide.
  The atlas is generated from the BDF by `cargo xtask font-atlas`, and a
  test keeps the committed atlas equal to the tool's output.

## Decision

**The device.** `input::Device` is `Keyboard` or `Pad(PadKind)`.
`InputState::device()` is where the last press *that did something* came
from: a bound key or button going down. Unbound presses, presses of
something already held, releases and repeats never change it, and
`set_keymap` keeps it. It starts as `Keyboard`.

**Getting the kind there.** `RawInputEvent::PadDown(Button, PadKind)`.
`app` builds a frame's pad events with `RawInputEvent::from_pads(&mut Pads,
&connected)`, which asks `Pads::kind_holding` for the pad that pressed
each newly pressed button (the first connected one, if two press it in the
same frame).

**Reaching the screens.** `Game` copies `input.device()` into
`Ctx::device` each frame, before the screens update. Screens never look at
it directly: they call `ctx.help_keys()`, which gives a `HelpKeys`
(`&Keymap` + `Device`, `Copy`, derefs to the keymap), and pass that to the
help helpers and `fill_placeholders` in place of the keymap. There is
deliberately no conversion from `&Keymap`, so a caller that forgets the
device doesn't compile. `HelpKeys::keyboard(&keymap)` is for the few
places that are about the keyboard whatever is in the player's hands: the
layout picker's key legend and the debug-menu hint.

**Names.** `PadKind::button_name(Button)` in `ui/src/input/pad.rs` is the
only table of button names (the `check-keys` rules for `Button::` already
confine it there). It is indexed by *binding position*, so on a Nintendo
pad `South` (its right face button after the Confirm / Cancel swap) is
`A`. `Generic` pads use the Xbox column. The cursor is named by
`Keymap::cursor_buttons_name`: every whole set of four directions the
cursor is on (`D-pad`, `stick`, `R-stick`), joined with `/`; if it is on
no whole set, each direction's first button.

**Glyphs.** `cargo xtask font-atlas <font.bdf>... <out-dir>` takes several
BDF files. The first is the font; the others **add** glyphs and must have
the same cell size. A glyph the font already has is an error, so an extra
file can't quietly change Terminus. Our glyphs live in
`assets-src/fonts/pad-shapes.bdf`: `✕` U+2715, `◯` U+25EF (not U+25CB,
which stays Terminus's small circle), `□` U+25A1, `△` U+25B3, each 7×7 px
on the baseline (the look Nick picked from four rendered options). The
committed-atlas test builds from both files.

## Consequences

- Every piece of text that names a key now also names buttons, with no
  per-screen code: screens changed one line each.
- A new screen must use `ctx.help_keys()`; passing `&ctx.keymap` to a help
  helper no longer compiles.
- The Harness has `use_pad(kind)` (which pad `pad()` presses) and
  `snapshot_as(device)` (the screen as another device would show it), so
  tests can still compare pad play with keyboard play.
- 0226 can read `ctx.device` to see the player switch from pad to
  keyboard. Note it only follows *bound* presses; before a layout is
  chosen few keys are bound, so 0226 must look at the raw key events for
  "any key".
- 0816 (rebinding buttons) gets names for every button, stick directions
  and stick presses included; those are Claude's starting names until Nick
  sees them there.
- The atlas is a derivative of Terminus plus our own glyphs. OFL-1.1
  allows that as long as the result stays under the OFL and isn't called
  "Terminus Font"; it never was (`assets/fonts/README.md`).
- PlayStation 4 pads show `Create` like PS5 ones: telling them apart needs
  the USB product id, which `PadKind::from_vendor` doesn't get (ticket
  0229).

## Alternatives considered

- **Put the device inside `Keymap`** — every press would make the keymap
  "change", and `Game` resets held keys when it does.
- **Pass `&Keymap` and a `Device` to each helper** — two arguments at 70
  call sites, and helpers called with only a keymap would have needed a
  default device, which is exactly the bug to avoid.
- **Give the helpers `&Ctx`** — several callers name keys from a keymap
  that isn't `ctx.keymap` (the Key bindings screen's "opened with" keys,
  the layout picker's two layouts).
- **`Pads::update` returning the kind with every change** — releases have
  no single pad; `kind_holding` asks only for presses and keeps the 50-odd
  existing tests of `update` as they are.
- **Edit `ter-u16n.bdf`** — mixes our work into a third-party file and
  makes updating Terminus a merge. A second file keeps "theirs" and "ours"
  apart.
- **Replace Terminus's `○`** — would change a glyph other text may use;
  a separate code point costs nothing.
- **Private-use code points for the shapes** — the real code points read
  correctly in snapshots, tests and docs.
