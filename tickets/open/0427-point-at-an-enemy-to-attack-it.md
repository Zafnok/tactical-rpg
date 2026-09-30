---
id: "0427"
title: Point at an enemy with a unit selected to attack it
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0427 — Point at an enemy with a unit selected to attack it

## Context

Nick, after playing Quick battle (2026-09-29):

> "moving cursor from unit selected to an enemy in attack range should move it
> to an appropriate tile and open the menu to select an attack / art that is
> allowed w/in range. quick battle currently makes you move to a tile and then
> click attack rather than clicking an enemy directly"

Today (0403/0404, [`mode.rs`](../../crates/ui/src/screens/battle/mode.rs)) the
player must steer the arrow to a tile, Confirm, pick `Attack` in the action
menu, maybe pick a weapon, then pick the target. In `Mode::Selected`, Confirm
does nothing unless the cursor is on the path's end (`step`, the
`Mode::Selected` arm), and moving the cursor onto an enemy just leaves the
arrow where it was (enemy tiles aren't passable, so `path::steer` returns the
path unchanged).

Nick's answers to the two open questions (asked 2026-09-29, Fire Emblem-style
options):

1. **Which tile:** *"Keep my path, else nearest."* If the arrow the player drew
   already ends on a tile the unit can stop on and attack the enemy from, the
   unit attacks from there. Otherwise the arrow jumps to the closest such tile.
   Example: you steer the archer behind a forest, then point at the brigand —
   she shoots from the forest.
2. **What opens:** *"Forecast on that enemy."* After Confirm the unit walks
   there, then straight to the weapon list (only weapons that reach that enemy
   from that tile; skipped if just one), then the attack forecast locked on
   that enemy, where combat actives can be cycled as today (arts too, once
   0414 lands). Cancel goes back to the normal action menu for that tile.

Builds on 0403 (move loop), 0404 (weapon list, `Targeting`), 0412 (actives on
the forecast). Rules stay in `core`: every "can attack from here" check is
`BattleState::preview_attack` (ADR-0004), like `attack::targets`.

## Nick input

**Sign-off:** in Quick battle, select a unit, move the cursor onto an enemy
it can reach, check the arrow jumps to a sensible tile, press Confirm and
check the forecast opens on that enemy. Try it with the arrow first steered
to a tile of your choice (it should keep it) and with an archer.

## Scope

**In:**
- With a unit selected, the cursor on an enemy the unit can attack after
  moving: the path arrow shows the tile it will attack from (rule below);
  the cursor stays on the enemy.
- Confirm there: walk (skippable as today), then the weapon list filtered to
  weapons that reach that enemy (skipped if one), then `Targeting` with that
  enemy under the cursor.
- Cancel from that weapon list / forecast → the normal action menu at that
  tile (focused on `Attack`); Cancel again → `Mode::Selected` with the path,
  as today.
- Help bar in `Mode::Selected` offers Confirm "attack" when the cursor is on
  such an enemy.
- Record the decision in `docs/design/turn-structure.md` under "A unit's
  action" (Nick's words + the rule), dated 2026-09-29, source ticket 0427.

**Out (do not do):**
- Combat arts (0414 adds them to `Targeting`; this flow gets them for free).
- Actives that add range (0426): the "can attack" check is the plain weapon
  attack, same as `attack::targets`.
- Staves / spells / skills / items on allies: only attacking hostile units.
- Any change to `core`.
- Choosing tiles by terrain, safety, or counter-attack risk.

## The tile rule

Let `can_hit(tile)` = some weapon slot `s` in `0..WEAPON_SLOTS` has
`state.preview_attack(unit, tile, &attack(enemy, s)).is_ok()`.

1. If the current path's end is stoppable (`reach.is_stoppable`) and
   `can_hit(end)`: keep the path unchanged.
2. Otherwise, among `reach.stoppable()` tiles with `can_hit`, pick the one
   with the lowest `reach.cost(tile)`; ties → shortest `reach.path_to(tile)`;
   still tied → lowest `(y, x)` (deterministic; *Claude's starting tie-break*,
   mention it to Nick in the PR). The path becomes `reach.path_to(tile)`.
3. No such tile: the path is left as `steer` leaves it today (unchanged), and
   Confirm on the enemy does nothing.

"Keep my path" uses the path *as it was before the cursor landed on the
enemy*, so moving the cursor from the chosen tile straight onto the enemy
keeps it. Moving the cursor off the enemy resumes normal steering from
whatever path is shown.

## Implementation steps

1. `attack.rs`: add `pub fn can_hit(state, unit, from: Pos, target: UnitId) -> bool`
   and `pub fn attack_tile(state, sel: &Selection, target: UnitId) -> Option<Pos>`
   implementing the rule above (return `sel.dest()` for case 1).
2. `mode.rs`, `Selection`: add `pub target: Option<UnitId>` — the enemy under
   the cursor the path is aimed at (`None` otherwise). Initialise to `None`
   in `Selection::new`.
3. `Mode::cursor_moved`: when `Mode::Selected`, if a unit hostile to the
   selected unit stands on `to` and `attack_tile` is `Some(tile)`, set
   `sel.path` (to `reach.path_to(tile)` unless case 1) and `sel.target =
   Some(id)`; else `sel.target = None` and `sel.steer(to, state)` as today.
   (Look up the unit at `to` via `state.units()`; hostility = the same test
   `preview_attack` uses — simply rely on `attack_tile` being `None` for
   non-attackable units.)
4. `step`, `Mode::Selected` arm: new first case — `Action::Confirm` when
   `sel.target` is `Some(t)` and the unit at `cursor` is `t`: if the path has
   length 1 go straight to `open_attack(sel, t, state)`, else
   `Mode::Moving { sel, .. }` (the target rides in `sel`).
5. `Mode::tick` and the `Mode::Moving` Confirm arm: when the walk ends, call
   `open_attack` if `sel.target.is_some()`, else `open_menu` as today.
6. `fn open_attack(sel, target, state) -> (Mode, Effect)`: take
   `weapon_choices(state, &sel)`, keep those whose `targets` contain
   `target`, and set each kept choice's `targets` so `target` is first
   (rotate, keeping `(y, x)` order after it — cycling still reaches the
   others). Zero kept → `open_menu`. One → `target_with(state, sel, &c, None)`.
   Several → `Mode::WeaponMenu` built with `weapon_menu` over the kept
   choices. `target_with` already puts the cursor on the first target.
   Note `tick` returns a `Mode`, not an `Effect`: either change `tick` to
   also return an `Effect` (and update the screen caller in `mod.rs`), or have
   the screen snap the cursor to `Targeting::target()`'s tile whenever it
   enters `Mode::Targeting`. Pick one and keep it consistent.
7. Cancel routing: `step_weapon_menu` and `step_targeting` already go back to
   the action menu (`back_to_menu`) / weapon list. Check that a `Targeting`
   opened from `open_attack` with a filtered weapon list cancels to that
   list, and from there to the action menu. Clear `sel.target` whenever
   returning to `Mode::Selected` from the action menu, so a later Confirm on
   the path's end moves normally.
8. `mod.rs` help bar (`Mode::Selected` arm, ~line 729): when
   `sel.target.is_some()` and the cursor is on it, show
   `help_line(&[moves, confirm("attack"), cancel("cancel")])`. Key names come
   from the keymap (`keyboard-input` skill) as the existing entries do.
9. Update the module docs at the top of `mode.rs` with the shortcut.
10. `docs/design/turn-structure.md`: add the decision (see Scope).

## Acceptance criteria

- [ ] Unit test (`mode.rs` tests, Quick battle state `quick()`): with a unit
      selected and the path at its origin, moving the cursor onto an enemy
      reachable only after moving sets the path to the cheapest tile that can
      hit it and `sel.target` to that enemy.
- [ ] Unit test: steering the path to a tile that can hit the enemy, then
      moving the cursor onto the enemy, keeps that path.
- [ ] Unit test: an archer (bow, range 2) pointing at an enemy picks a tile
      at distance 2, never an adjacent one.
- [ ] Unit test: Confirm on that enemy → `Mode::Moving`; after the walk,
      `Mode::Targeting` whose `target()` is that enemy (or `Mode::WeaponMenu`
      listing only weapons that reach it, when there are several).
- [ ] Unit test: Cancel from that forecast reaches the action menu at the
      attack tile; Cancel again reaches `Mode::Selected` with `target == None`.
- [ ] Unit test: an enemy no tile can hit — the path is unchanged and Confirm
      does nothing.
- [ ] Harness test: select, cursor onto enemy, Confirm, Confirm (skip walk
      if needed), Confirm on the forecast → the attack command is applied
      from the chosen tile.
- [ ] Help bar reads "attack" for Confirm with the cursor on such an enemy.
- [ ] `docs/design/turn-structure.md` records the decision.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the tile rule and mode transitions above (`mode.rs` tests).
- Snapshot / integration: harness test of the full flow; review any changed
  help-bar snapshots.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
