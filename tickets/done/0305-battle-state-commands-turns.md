---
id: "0305"
title: "Battle state: commands → events, turn/phase system, objectives, replay"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: done
blocked_by: ["0002", "0006", "0303", "0304"]
nick_input: answer-first
completed: 2026-09-26
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
serde on state, replay test, and swapping the battle screen's stand-in
state for `BattleState` (step 11).

**Out:** item/equip/heal actions (0306), EXP (0601), rewind (0307), AI (0501).

## Implementation steps

1. `BattleSetup { map, terrain: Arc<TerrainTable>, classes: Arc<ClassTable>, units: Vec<Unit>, objective: Objective, rewind_charges: u8, seed: u64 }`
   (`rewind_charges` comes from the chapter's difficulty tier, 0801; used by 0307).
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
   `UnitFell { unit }` (the unit leaves the map; Classic vs Casual only matters to the campaign, 0801), `UnitActed { unit }`,
   `BattleEnded { outcome: Outcome }`.
6. Turn flow per `turn-structure.md`: phases are `Player → Enemy → Other`
   (`Other` = `Ally` + `Neutral` factions), then `turn += 1`. Use a `Phase`
   enum (not `Faction`) for `phase`. `EndPhase` → next phase; skip phases
   with no living units and no arrivals; units' `acted` reset at the start of
   their phase. **Auto-end is not in `core`:** it's a player setting handled
   by the battle screen (0405), which issues `EndPhase` itself. AI phases end
   when the AI issues `EndPhase` (0501/0502).
7. **Reinforcements:** `BattleSetup.reinforcements: Vec<Reinforcement { turn, unit: Unit }>`
   (the unit's faction decides its phase). At the start of that faction's
   phase on `turn`, place it with `acted = true` (never acts on arrival);
   if its tile is occupied, retry on the next turn. Event
   `UnitsArrived { units }`.
8. `Objective`: `Rout`, `DefeatUnit(UnitId)`, `Seize { pos, by_lord: bool }`
   (`Seize` is a `UnitAction` only available on that tile — add it),
   `Survive { turns }` (win when turn `N`'s last phase ends), plus an optional
   `turn_limit: Option<u32>` on the non-Survive objectives (lose if not done
   when turn `N`'s last phase ends). Loss (both modes, `death-and-difficulty.md`): any `is_lord` player
   unit falls, or all player units on the map fall. Checked after every command
   and at every phase boundary.
9. Design for skill-granted post-action movement (turn-structure.md): no
   Canto, but leave `UnitAction`/`Event` open to an action ending with a
   skill-defined extra move (implemented by the skill tickets, not here).
10. Replay test: a fixed setup + seed + list of commands applied twice → identical
   event vectors; and serialise state mid-battle → deserialise → apply the rest
   → identical events to an uninterrupted run.
11. **Battle screen hookup (left over from 0401).** 0401 had no `BattleState`,
   so `ui::screens::battle` holds a stand-in `BattleScene { map, units }`.
   Replace it:
   - `BattleScreen::new` takes a `BattleState`, and drawing reads its map and
     units. The screen only draws, so this change stays inside that module.
   - `quick_battle(content)` builds a `BattleSetup` (e.g. `Objective::Rout`,
     a fixed seed) and keeps its current units: one wounded, one acted.
   - Delete `BattleScene`.
   - Existing snapshots must not change: the drawing is the same.

## Acceptance criteria

- [x] Every `CommandError` variant has a test proving the state is unchanged.
- [x] Phase/turn sequence matches `turn-structure.md` (test walks 3 full turns, with and without Other units; empty phases skipped).
- [x] Reinforcements arrive at their phase start already acted, act next turn, and wait while their tile is occupied (tests).
- [x] Survive and `turn_limit` resolve exactly when turn N's last phase ends (tests at N-1 and N).
- [x] Each objective and loss condition has a win test and a not-yet test.
- [x] Replay + save/load determinism tests pass.

## Tests required

- Unit: validation errors, phase order, objectives, falling rules.
- Property: random *valid* command sequences (generate by picking from legal
  commands) never panic, never leave two units on one tile, never produce HP > max,
  and `outcome` once set never changes.
- Replay/determinism (above).

## Completion notes

**Done.** New module `trpg_core::battle` (`crates/core/src/battle.rs`, rules in
its module doc; tests in `battle/tests.rs`): `BattleSetup`, `BattleState`
(`new`, `apply`, read-only accessors, `restore_tables`), `Phase`, `Turn`,
`Command`, `UnitAction` (`Wait`, `Attack`, `Seize`), `Event`, `CommandError`,
`Objective`, `Outcome`, `Reinforcement`. Replay/save-load tests in
`crates/core/tests/replay.rs`. New ADR-0020 (saves hold the battle's own
data; terrain/class tables are `Arc`s skipped by serde and reattached with
`restore_tables`).

What the next tickets should know (differences from the plan):

- **Attacks need a weapon, and there are no items yet (0306).** `Unit` got a
  stand-in `weapon: Option<WeaponStats>` (`None` from `from_character` /
  `generic`, so the Quick Battle units are unarmed). `BattleState::combatant`
  builds `CombatantInput` from it (permanent stats, class tags/affinities,
  rank from `weapon_ranks`, no armour). 0306's text now says to replace both.
  Combat constants are `CombatRules::default()`.
- **`apply` takes `&Command`.** The `Objective` variants carry their own
  `turn_limit` (so `Survive` can't have one); `Objective::turn_limit()` reads
  it. `Seize { pos, by_lord, turn_limit }` needs a **player** unit (a lord if
  `by_lord`).
- **Extra `CommandError`s** beyond the ticket's list: `UnknownClass`,
  `OffMap`, `UnknownTerrain` (bad data, or tables not restored after
  loading), `NoWeapon`, `CannotSeize`. Unknown and fallen units are
  `UnknownUnit` / `UnitFallen` for both the actor and the target.
- **Extra events:** `Seized { unit, pos }`. `UnitMoved` is only sent when the
  unit actually moves. An `Act` always ends with `UnitActed` unless the unit
  fell (then no `UnitActed`), optionally followed by `BattleEnded`. At a
  phase start, `UnitsArrived` comes **before** `PhaseStarted`, the order
  `turn-structure.md` gives (arrivals, then the banner).
- **Falling:** fallen units move from `units()` to `fallen()` (HP 0), so the
  campaign (0801) can apply Classic/Casual.
- **Post-action skill moves (step 9):** documented in the module doc: a
  skill's move becomes a new event just before `UnitActed`; no `UnitAction`
  change needed.
- **Validation of setups** (unique ids, one unit per tile, on the map) is left
  to content validation of maps (0803); `BattleState::new` trusts its setup.
  Quick Battle's archer is now marked "acted" by issuing a real `Wait`
  command.

Rules I had to pin down where the docs are silent (Nick may veto):

- **Rout** counts enemies **on the map**: killing the last one wins even if
  enemy reinforcements are still scheduled (as in Fire Emblem). Maps that
  must not end early shouldn't use Rout with late reinforcements.
- A battle with no player units at the start is lost at once; a Rout with no
  enemies at the start is won at once.

Tests: every `CommandError` variant (state compared before/after), phase order
over 3 turns with and without Other units, skipped empty phases, ready/done
flags per phase, move/attack/kill/counter-kill events, terrain from each
unit's tile, class tags and weapon rank in combat, every objective and loss
condition (win + not yet), Survive and turn limits at N−1 and N,
reinforcements (arrive done, act next turn, wait while blocked, wave order,
arrivals un-skip a phase, turn-1 player arrivals), RON round-trips of state,
commands and events. Property test (512 cases): random legal command
sequences are always accepted, never put two units on one tile, keep HP in
`1..=max` on the map and 0 when fallen, and never change a decided outcome.
Replay test: same seed + commands ⇒ identical events and state; saving,
loading and continuing at **every** command boundary ⇒ identical events.
`cargo mutants` on the diff: 74 caught, 0 missed (27 unviable). The
property test's saved regression cases (`proptest-regressions/battle/`)
are cases it used to catch mutants; they now re-run every time.

No follow-up tickets. Nothing new for Nick to see: the Quick Battle looks
exactly the same (snapshots unchanged); moving and attacking arrive with
0403/0404.
