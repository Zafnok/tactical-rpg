---
id: "0403"
title: "Select → move (range overlays, path arrow, walk animation) → action menu → Wait"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: done
blocked_by: ["0402", "0305"]
nick_input: none
completed: 2026-09-27
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
inspecting enemy ranges. After an attack that ends with
`Event::MoveAfterOffered` (Vault, Swoop; 0311), let the player pick one of
`BattleState::move_after_tiles()` or stay, and send `Command::MoveAfter`
(nothing else is accepted until then).

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

- [x] Full loop works in Quick Battle: select, steer path, move, Wait, unit dims and its label turns lowercase.
- [x] Cancel from action menu restores the unit exactly (state unchanged — test).
- [x] Only valid destinations are accepted.
- [x] Harness integration tests and snapshots below pass.

## Tests required

- Unit/property: path-arrow logic — result is always a valid path (0303 `path_cost` Ok) ending at the cursor when the cursor is reachable.
- Harness: select → `l l` → `f` → `Wait` → unit at x+2, acted; select → move → cancel → cancel → state unchanged; enemy range toggle.
- Snapshot: selected unit with overlays and path arrow; action menu open.

## Completion notes
- **Interaction state machine** (`crates/ui/src/screens/battle/mode.rs`):
  `Mode::{Idle, Selected, Moving, ActionMenu, MoveAfter}` with pure
  transitions `step(mode, action, cursor, &BattleState) -> (Mode, Effect)`
  and `Mode::tick` for the walk. The screen owns the cursor and applies the
  `Effect` (`Apply(Command)`, `Cursor`, `Leave`). The unit's position during
  `Moving`/`ActionMenu` is a drawing override (`Mode::drawn_pos`); the
  `BattleState` changes only when `Command::Act` is applied.
- **Path arrow** (`path.rs`): `steer` (cut back / append / cheapest path /
  unchanged if unreachable) with a property test that the path always passes
  `path_cost` and ends on the cursor whenever the cursor is reachable.
  Drawing: a 3-px `Under` line from the unit tile's edge, 6 stacked-rect `Over`
  arrowhead on the destination, clipped to the map view.
- **Cursor:** `►Lo◄` on the selected unit, no frame on the arrowhead's tile,
  brackets elsewhere; hidden while walking and in the menu.
- **Action menu** beside the unit (right, or left near the view edge; first
  item level with the unit): `Attack` (disabled until 0404), `Seize` when
  legal, `Wait`.
- **Enemy threat area:** Confirm on any non-player unit toggles its red area;
  Confirm again, on another enemy (switches), selecting a unit or Cancel
  hides it. Help shows `f range` / `d hide range`.
- **Move after attack:** when `BattleState::pending_move()` is set, the screen
  enters `MoveAfter`: the tiles are tinted blue, Confirm on one sends
  `Command::MoveAfter { to: Some(tile) }`, Confirm on the unit sends `to: None`
  (stay), everything else is ignored. Tested with a real Vault attack.
- **Side panel** border turns double-line while a unit is selected
  (`look-and-feel.md`) and shows the unit where it is drawn.
- **Core:** `BattleState::can_seize(unit, dest)` so the menu doesn't duplicate
  the seize rule. **Content:** `path` joined the required palette colours (it
  was already in `palette.ron`).

Deviations:

- Blue range covers every tile the unit can move *through* (including an
  ally's tile), not only stoppable tiles, so an ally's tile never looks like
  attack range. Only stoppable tiles are accepted as destinations.
- "Hold Confirm = instant": holding Confirm skips the walk once held for
  0.2 s (`HOLD_SKIP_S`), so the tap that starts the walk doesn't skip it; a
  second press skips at once.
- Ranges are hidden while walking and in the action menu.
- The harness test uses `Right Right` (right-handed layout) where the ticket
  said `l l` (written before the layouts ticket).
- Ticket 0415 (Quick Battle starts fresh) landed first; this branch builds on it.

No gameplay rules decided: every choice above is UI presentation.
