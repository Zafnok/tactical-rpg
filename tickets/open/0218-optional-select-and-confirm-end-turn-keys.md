---
id: "0218"
title: "Optional split keys: Select (cursor) vs Confirm, Confirm end turn vs End turn"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0217"]
nick_input: none
completed:
---

# 0218 — Optional Select and Confirm end turn keys

## Context

Nick (ticket 0030, [`docs/design/controls.md`](../../docs/design/controls.md),
*Rebinding keys* → *Optional split keys*): players may want "separate select
(cursor) and confirm (action) keys, or separate end turn / confirm end turn
keys (right now both can be space)". Both new actions are **optional** and
start with **no key**, so the defaults play exactly as today; a player gives
them keys on the Key bindings screen (0814).

0217 built slots, optional actions and `! not mapped`; this ticket only adds
the two actions and makes the battle screen honour them. Follow the
`keyboard-input` skill.

## Nick input

None. (Nick signs off on the split behaviour when playing 0814.)

## Scope

**In:**
- `Action::Select` and `Action::ConfirmEndTurn`, optional, unbound (`[]`)
  in both layouts.
- Battle screen: Select for on-map cursor picks; ConfirmEndTurn for the
  end-turn prompt; fallbacks when they have no key.
- Help bar and tips name the right key.

**Out (do not do):**
- The Key bindings screen (0814).
- Any change to the default keys or to how the game plays with the defaults.

## Implementation steps

1. `crates/content/src/keymap.rs`: add `Action::Select` ("Pick a unit,
   tile or target on the map with the cursor") and `Action::ConfirmEndTurn`
   ("Accept the end-turn prompt"), extend `Action::ALL`, names for RON.
   Both optional (`is_required` false). Add `"Select": []` and
   `"ConfirmEndTurn": []` to both layouts in `assets/data/keymap.ron`, with a
   comment pointing at `controls.md`.
2. `crates/ui/src/input.rs`, `Keymap`: add
   - `select_action(&self) -> Action`: `Select` if it has a key, else
     `Confirm`;
   - `end_turn_accept_actions(&self) -> [Action; 2]`: `[ConfirmEndTurn,
     Confirm]` if `ConfirmEndTurn` has a key, else `[EndTurn, Confirm]`.
   Unit-test both ways.
3. Battle screen (`crates/ui/src/screens/battle/mod.rs`, `mode.rs`,
   `attack.rs`, `map_menu.rs` as needed): every place that reacts to
   `Action::Confirm` **while the player steers the map cursor** (choosing a
   unit, choosing its destination tile, choosing an attack/heal/item target
   on the map, Confirm on an empty tile opening the map menu, Confirm on an
   enemy showing its range) matches `ctx.keymap.select_action()` instead.
   Menus, prompts, the forecast, banners, info screens and hold-to-fast-
   forward keep `Action::Confirm`. List each site you changed in the
   Completion notes.
4. End-turn prompt: accepts on any of `end_turn_accept_actions()`; Cancel
   still backs out. With `ConfirmEndTurn` bound, a second End turn press in
   the prompt does nothing.
5. Help bars: map-cursor hints use `key_name(km, km.select_action())`; the
   end-turn prompt's "yes" hint uses the first of
   `end_turn_accept_actions()`. Tips (`assets/data/tips.ron`): wherever a
   tip means the map pick ("Press {Confirm} on one of your units to select
   it", "Steer to a blue tile and press {Confirm} to move", "{Confirm} on an
   enemy shows just its range"), use a new `{Select}` placeholder that
   resolves via `select_action()` (so it shows Confirm's key while Select is
   unbound). Menu/forecast wording keeps `{Confirm}`.
6. `dialogue.rs` advances on Confirm and End turn today; leave it as is.

## Acceptance criteria

- [ ] With default keys, every existing battle Harness test and snapshot
      passes unchanged.
- [ ] With `Select` bound to `g` (via `Ctx::set_layout_bindings`): `g`
      selects a unit and picks its tile; `f` does not select on the map but
      still picks from the action menu and accepts the forecast (Harness
      test).
- [ ] With `ConfirmEndTurn` bound to `Enter`: `Space` opens the prompt,
      `Space` again does nothing, `Enter` ends the turn, `f` also ends it,
      `d` backs out (Harness test).
- [ ] Help bar and the three tips show `g` when Select is bound and `f`
      when it isn't (tests).
- [ ] `cargo xtask check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `select_action`, `end_turn_accept_actions`, `{Select}` placeholder.
- Integration: Harness scripts above; existing snapshots unchanged.

## Completion notes
