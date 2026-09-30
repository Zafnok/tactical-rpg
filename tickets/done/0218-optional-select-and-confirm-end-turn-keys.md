---
id: "0218"
title: "Optional split keys: Select (cursor) vs Confirm, Confirm end turn vs End turn"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0217"]
nick_input: none
completed: 2026-09-30
---

# 0218 — Optional Select and Confirm end turn keys

## Context

Nick (ticket 0030, [`docs/design/controls.md`](../../docs/design/controls.md),
*Rebinding keys* → *Optional split keys*): players may want "separate select
(cursor) and confirm (action) keys, or separate end turn / confirm end turn
keys (right now both can be space)". Both new actions are **optional** and
start with **no key**, so the defaults play exactly as today; a player gives
them keys on the Key bindings screen (0815).

0217 built slots, optional actions and `! not mapped`; this ticket only adds
the two actions and makes the battle screen honour them. Follow the
`keyboard-input` skill.

## Nick input

None. (Nick signs off on the split behaviour when playing 0815.)

## Scope

**In:**
- `Action::Select` and `Action::ConfirmEndTurn`, optional, unbound (`[]`)
  in both layouts.
- Battle screen: Select for on-map cursor picks; ConfirmEndTurn for the
  end-turn prompt; fallbacks when they have no key.
- Help bar and tips name the right key.

**Out (do not do):**
- The Key bindings screen (0815).
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

- [x] With default keys, every existing battle Harness test and snapshot
      passes unchanged.
- [x] With `Select` bound to `g` (via `Ctx::set_layout_bindings`): `g`
      selects a unit and picks its tile; `f` does not select on the map but
      still picks from the action menu and accepts the forecast (Harness
      test).
- [x] With `ConfirmEndTurn` bound to `Enter`: `Space` opens the prompt,
      `Space` again does nothing, `Enter` ends the turn, `f` also ends it,
      `d` backs out (Harness test).
- [x] Help bar and the three tips show `g` when Select is bound and `f`
      when it isn't (tests).
- [x] `cargo xtask check-keys` passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `select_action`, `end_turn_accept_actions`, `{Select}` placeholder.
- Integration: Harness scripts above; existing snapshots unchanged.

## Completion notes

- `Action::Select` and `Action::ConfirmEndTurn` added (optional, `[]` in
  both layouts of `keymap.ron`, pinned by a test).
- `Keymap::select_action` and `Keymap::end_turn_accept_actions` in
  `crates/ui/src/input.rs`, unit-tested both ways.
- **Deviation (mechanism, not behaviour):** instead of rewriting each
  `Action::Confirm` arm in `mode.rs`, the battle screen routes a key before
  the mode sees it: `Mode::route` (`mode.rs`), called at the top of
  `BattleScreen::step_mode`. In map-pick modes it turns
  `select_action()` into Confirm and drops Confirm once Select has a key; in
  the end-turn prompt it turns any of `end_turn_accept_actions()` into
  Confirm and drops End turn once Confirm end turn has a key. `mode::step`
  and its unit tests are unchanged, and menu sounds stay the same.
- Sites that now follow Select (`Mode::picks_on_map`): browsing (select a
  ready unit, show an enemy's range, open the map menu on an empty tile),
  a selected unit (attack an aimed-at enemy, move to its tile), moving after
  an attack (stay / move here), and the skill, item and talk target pickers.
  Kept on Confirm: every menu, the objective and info screens, the attack
  forecast (`Targeting`), the walk skip, combat, tips, banners, EXP/level-up,
  rewind, hold-to-fast-forward, dialogue.
- Help bars: map-pick hints (`select`, `range`, `menu`, `attack`, `move
  here`, `stay`, `use`, `talk` in the target pickers) name
  `select_action()`; the end-turn prompt's help shows both accept keys
  (`Space yes · f yes · d no` by default, `Enter yes · f yes · d no` with
  Confirm end turn on Enter). The prompt box's own `f yes / d no` is
  unchanged (Confirm still accepts).
- Tips: new `{Select}` placeholder (Confirm's key while Select has no
  key), used by `battle_start`, `unit_selected` and `danger_zone`.
- Tests: `crates/ui/tests/split_keys.rs` (Harness), `Mode::route` unit test,
  `{Select}` and shipped-tip tests in `tips.rs`. No existing snapshot or
  test changed.

*Claude's starting rules (reading of "on the map", already flagged in
controls.md):* the skill, item and talk target pickers count as picking on
the map (Select), but the attack target picker counts as the forecast
(Confirm), because it shows the forecast panel. The Select key does nothing
in menus, and Confirm end turn does nothing outside the prompt.
