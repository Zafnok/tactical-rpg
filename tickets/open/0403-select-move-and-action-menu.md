---
id: "0403"
title: "Select → move (range overlays, path arrow, walk animation) → action menu → Wait"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0402", "0305"]
nick_input: none
completed:
---

# 0403 — Select, move, action menu

## Context

The Fire Emblem move loop: select a unit, see blue movement and red attack
ranges, steer a path with the cursor, move, pick an action. The move only
commits (as a `Command::Act`) when an action is chosen, so cancelling is free
([0305](0305-battle-state-commands-turns.md)).

## Nick input

None.

## Scope

**In:** battle-screen interaction state machine, overlays, path arrow, walking
animation, action menu with `Wait` (and `Seize` when legal), cancel flows,
inspecting enemy ranges.

**Out:** `Attack` targeting/forecast/playback (0404; show `Attack` in the menu
but disabled until 0404), `Item`/`Equip` menu entries (0407; there is no in-battle trade),
map menu (0405).

## Implementation steps

1. Interaction state enum inside `BattleScreen`:
   `Idle | Selected { unit, reach, path } | Moving { unit, path, t } | ActionMenu { unit, dest, menu } | …`.
   Keep transitions in a pure function `fn step(state, action, &BattleState) -> (NewState, Option<Command>)`
   where possible, so they're unit-testable without drawing.
2. **Idle + Confirm on own ready unit** → compute `reach` (0303) → `Selected`.
   Overlays: stoppable tiles blend `move_range` bg; attack tiles blend
   `attack_range` bg.
3. **Path arrow:** starts as `[unit pos]`. When the cursor moves to an adjacent
   tile that extends the path within budget (use `path_cost`), append; if it
   revisits a tile on the path, truncate to it; otherwise replace with
   `reach.path_to(cursor)` if reachable. Pure logic + property test.
   **Drawing** (`look-and-feel.md`, ADR-0018): while the cursor is on the
   selected unit, the cursor shows `►` `◄` instead of brackets. Once it moves
   away: a 3-px `Under` overlay line in `path` colour through tile centres,
   starting at the **edge** of the unit's tile (never over its label), ending
   in a **single arrowhead** (`Over` overlay triangle) on the destination tile;
   no cursor frame is drawn there.
4. **Selected + Confirm on stoppable tile** → `Moving` (unit steps along path at
   ~12 tiles/s using `dt`; hold Confirm = instant). Confirm on non-stoppable → ignored.
   **Selected + Cancel** → `Idle`, cursor back on the unit.
5. **ActionMenu** (Menu widget from 0205, placed beside the unit): `Attack`
   (disabled until 0404), `Seize` (if legal), `Wait`. `Wait` → apply
   `Command::Act { dest, Wait }` → unit marked acted → `Idle`.
   **Cancel** → unit drawn back at origin, `Selected` again with the same path.
6. **Idle + Confirm on enemy/other-faction unit** → toggle that unit's threat
   area overlay (red); Confirm again or Cancel clears it.
7. The *drawn* position of a unit during `Moving`/`ActionMenu` is a UI override;
   `BattleState` is untouched until the command is applied.

## Acceptance criteria

- [ ] Full loop works in Quick Battle: select, steer path, move, Wait, unit dims and its label turns lowercase.
- [ ] Cancel from action menu restores the unit exactly (state unchanged — test).
- [ ] Only valid destinations are accepted.
- [ ] Harness integration tests and snapshots below pass.

## Tests required

- Unit/property: path-arrow logic — result is always a valid path (0303 `path_cost` Ok) ending at the cursor when the cursor is reachable.
- Harness: select → `l l` → `f` → `Wait` → unit at x+2, acted; select → move → cancel → cancel → state unchanged; enemy range toggle.
- Snapshot: selected unit with overlays and path arrow; action menu open.

## Completion notes

