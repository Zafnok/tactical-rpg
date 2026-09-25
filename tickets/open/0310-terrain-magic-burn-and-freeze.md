---
id: "0310"
title: "Terrain magic: burn forests and freeze water with tile casts"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0301", "0303", "0305", "0309"]
nick_input: none
completed:
---

# 0310 — Terrain magic (burn and freeze)

## Context

Nick wants spells that change the map (`docs/design/magic.md`, ticket 0004).
Fire cast on an empty forest sets it **burning**: it is impassable for one
round, then **burnt** (walkable) from the start of the caster's side's next
phase. Ice cast on empty water or sea turns it to **ice** at once, for the
rest of the battle. Changes go through `Command`s and `Event`s
([ADR-0004](../../docs/adr/0004-crate-architecture.md)), so rewind (0307) and
replays pick them up for free as long as the terrain lives in `BattleState`.

## Nick input

None.

## Scope

**In:**
- New terrains `burning`, `burnt` and `ice` in `terrain.ron`, with the
  numbers from `magic.md`.
- A mutable terrain grid in `BattleState`.
- `CastTarget::Tile(Pos)`.
- The phase-start transition from burning to burnt.
- Movement and pathfinding reading the current terrain.

**Out (do not do):**
- Push spells (only the collision rule is documented in `magic.md`; nothing
  pushes yet).
- Fire spreading, and any other interaction.
- Drawing the tiles (0410).
- AI tile casts.

## Implementation steps

1. `terrain.ron`: add `burning` (impassable for every movement type, flying
   included), `burnt` (costs as `plain`, Def 0, Avoid 0) and `ice` (plain
   costs for ground types, flying as usual, Def 0, Avoid 0). Glyphs and
   colours: pick readable placeholders and mark them `// TUNABLE (0011)`.
2. `BattleState.tiles: Grid<TerrainId>`, copied from the map at setup.
   `BattleState` owns it, and 0303's functions take this grid, not the
   static map. Add `burning: Vec<(Pos, Phase /* caster's phase */)>`.
3. `TerrainEffect` data in `spells.ron`: `(from: [terrain ids], to: terrain id, lasts: Permanent | UntilCastersNextPhase(then: terrain id))`.
   Fire: `forest → burning`, then `UntilCastersNextPhase(burnt)`. Frost:
   `water|sea → ice`, `Permanent`.
4. `UnitAction::Cast { spell, target: CastTarget::Tile(pos) }`: the spell is
   known, has uses left and has a `terrain_effect`; the target is in range,
   **unoccupied**, and its current terrain is in `from`. Spend 1 use, change
   the tile, and end the action. Events: `SpellCast`,
   `TerrainChanged { pos, from, to }`, `SpellUsesChanged`. Casting at a unit
   (0309) never changes terrain.
5. At `PhaseStarted` for phase P, every burning tile whose caster phase is P
   becomes its `then` terrain, emitting `TerrainChanged`. Do this before any
   unit acts, and before reinforcements are placed.
6. Emit the EXP hook the same way as for heals (the amount comes from 0601).

## Acceptance criteria

- [ ] A player Fire on a forest: impassable during the Enemy and Other phases, `burnt` and walkable at the start of the next Player Phase (test walks the phases). An enemy-cast fire turns at the next Enemy Phase.
- [ ] Frost on sea: a foot unit can cross immediately and on every later turn.
- [ ] Invalid tile casts (occupied tile, wrong terrain, out of range, 0 uses, already burning) leave the state unchanged.
- [ ] Rewind/replay determinism still holds with terrain changes (extend 0305's replay test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each rule and error above; `reachable` honours changed tiles.
- Property: no unit ever stands on a `burning` tile; terrain only changes via
  `TerrainChanged` events.

## Completion notes
