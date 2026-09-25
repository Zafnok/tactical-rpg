---
id: "0305"
title: "Battle state: commands → events, turn/phase system, objectives, replay"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: todo
blocked_by: ["0002", "0006", "0303", "0304"]
nick_input: answer-first
completed:
---

# 0305 — Battle state, commands, events, turns, objectives

## Context

The heart of the rules engine: all changes to a battle go through `Command`s
and produce `Event`s ([ADR-0004](../../docs/adr/0004-crate-architecture.md)).
The UI animates events; the AI issues commands; saves and replays serialise
state/commands. Turn order per `docs/design/turn-structure.md` (0002); falling
units and game over per `docs/design/death-and-difficulty.md` (0006).

## Nick input

**Answer first:** 0002, 0006.

## Scope

**In:** `core::battle` — `BattleSetup`, `BattleState`, `Command`, `UnitAction`,
`Event`, `CommandError`, phase/turn progression, objective + defeat checks,
serde on state, replay test.

**Out:** items/heal/trade actions (0306), EXP (0601), rewind (0307), AI (0501).

## Implementation steps

1. `BattleSetup { map, terrain: Arc<TerrainTable>, classes: Arc<ClassTable>, units: Vec<Unit>, objective: Objective, seed: u64 }`.
   `BattleState::new(setup) -> (BattleState, Vec<Event>)` (emits the first `PhaseStarted`).
2. State: `turn: u32`, `phase: Faction` (or the design's equivalent), units,
   `rng: SimRng`, `outcome: Option<Outcome>`. Derive `Serialize/Deserialize`
   (store table references by id/Arc and rebuild on load — document how).
3. Commands (FE-style: a move and its action commit together, so the UI can let
   players cancel a move freely before choosing an action):
   ```rust
   enum Command {
       Act { unit: UnitId, dest: Pos, action: UnitAction },
       EndPhase,
   }
   enum UnitAction { Wait, Attack { target: UnitId } }   // 0306 extends
   ```
4. `BattleState::apply(&mut self, cmd) -> Result<Vec<Event>, CommandError>`.
   On `Err`, the state is **unchanged**. Validation: battle not over; unit
   exists, alive, belongs to the acting phase, hasn't acted; `dest` is
   stoppable (0303); target hostile, alive, within the unit's weapon range from
   `dest`.
5. Events (serde, `PartialEq`): `PhaseStarted { turn, phase }`,
   `UnitMoved { unit, path }`, `CombatResolved { attacker, defender, forecast, outcome }`,
   `UnitFell { unit }` (per 0006: removed/retreated/injured), `UnitActed { unit }`,
   `BattleEnded { outcome: Outcome }`.
6. Turn flow per `turn-structure.md`: `EndPhase` → next faction's phase (skip
   factions with no units) → after the last, `turn += 1`; units' `acted` reset
   at the start of their phase. If the design says phases auto-end when all
   units acted, emit that automatically after the last `Act`.
7. `Objective`: `Rout`, `DefeatUnit(UnitId)`, `Seize { pos, by_lord: bool }`
   (`Seize` is a `UnitAction` only available on that tile — add it),
   `Survive { turns }`. Loss: any `is_lord` player unit falls (if the design
   says so), or all player units fall. Checked after every command.
8. Replay test: a fixed setup + seed + list of commands applied twice → identical
   event vectors; and serialise state mid-battle → deserialise → apply the rest
   → identical events to an uninterrupted run.

## Acceptance criteria

- [ ] Every `CommandError` variant has a test proving the state is unchanged.
- [ ] Phase/turn sequence matches `turn-structure.md` (test walks 3 full turns).
- [ ] Each objective and loss condition has a win test and a not-yet test.
- [ ] Replay + save/load determinism tests pass.

## Tests required

- Unit: validation errors, phase order, objectives, falling rules.
- Property: random *valid* command sequences (generate by picking from legal
  commands) never panic, never leave two units on one tile, never produce HP > max,
  and `outcome` once set never changes.
- Replay/determinism (above).

## Completion notes

