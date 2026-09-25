---
id: "0408"
title: Preparations screen (loadouts and battle pack)
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0306", "0403"]
nick_input: sign-off
completed:
---

# 0408 — Preparations screen

## Context

Nick (0003): "on a screen before the match you can select … consumables to
bring into the match and then those are the ones your whole army can
use/share", with a per-battle cap, and each unit sets up a loadout
(3 weapons, 1 armour, 1 accessory). Rules in
`docs/design/weapons-and-items.md` (Loadout, Battle pack); types from 0306.

## Nick input

**Sign-off:** Nick sets up loadouts and a pack before a Quick Battle and
comments on how easy it is.

## Scope

**In:** `PreparationsScreen` (in `ui`, returns a `BattleSetup` with loadouts
and pack), `Loadouts` and `Pack` tabs, `Fight!` to start.

**Out:** shops (0409), choosing/swapping start positions, chapter flow wiring
(0801 pushes this screen when the chapter has `preparations: true`).

## Implementation steps

1. Screen with tabs (Left/Right cursor actions switch): **Loadouts**, **Pack**, and a `Fight!`
   entry.
2. **Loadouts:** list of deployed units; choosing one shows its 3 weapon
   slots, armour and accessory beside the stock, filtered to what that unit
   can use (unusable items dimmed with the reason, e.g. `needs rank D`).
   Confirm moves an item between a slot and the stock. Show the unit's attack
   speed change live (`AS 12 → 10`) using `core`'s formula.
3. **Pack:** stock consumables on the left, pack on the right, header
   `Pack 3/6`; adding past the cap shows a message instead. The chapter's
   default pack is preselected.
4. `Fight!` returns the setup. `Cancel` at the top level does nothing unless
   the caller allows backing out (then it asks `Leave preparations?`).
5. Debug Quick Battle opens this screen first.

## Acceptance criteria

- [ ] Harness: move a weapon from the stock into a slot, add 2 Potions to the pack, Fight! → the battle starts with that loadout and pack.
- [ ] The pack can never exceed the cap; unusable items can't be equipped.
- [ ] Snapshots below.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness: flow above; cap enforcement.
- Snapshot: Loadouts tab (with a dimmed unusable item), Pack tab at the cap.

## Completion notes

