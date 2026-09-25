---
id: "0705"
title: "Battle dialogue triggers: turn events, boss encounters, death quotes, Talk/recruit"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: todo
blocked_by: ["0704", "0405"]
nick_input: none
completed:
---

# 0705 — Battle dialogue triggers and Talk

## Context

In-battle story moments are how FE makes maps feel alive: boss taunts when
engaged, a villager's plea at turn 3, death quotes, and "Talk" conversations
that recruit enemies. This ticket adds the trigger model and the `Talk` action.

## Nick input

None.

## Scope

**In:** trigger definitions in battle/chapter data, trigger evaluation after
events, playing scenes as overlays, `UnitAction::Talk` with optional
recruitment, death quotes, boss-battle quotes.

**Out:** writing the actual lines (0707).

## Implementation steps

1. `core::battle::Trigger` (data, serde): `TurnStart { turn, phase }`,
   `UnitEntersArea { unit_or_faction, rect }`, `CombatStart { unit }` (e.g.
   boss engaged; fires once per pair or once total — flag), `UnitFell { unit }`
   (death quote, plays *before* the unit is removed visually), `Talk { a, b }`.
   Each trigger references a scene id and has `once: bool`.
2. Evaluation in `core`: after each command, return triggered scene ids as an
   `Event::SceneTriggered { scene }` (ordered, deterministic). Fired-once state
   lives in `BattleState` (so it saves/replays).
3. `UnitAction::Talk { target }`: available in the action menu when a `Talk`
   trigger exists for (unit, adjacent target) and hasn't fired. Effects:
   scene plays; optional `recruit: bool` switches the target to the player
   faction (event `UnitRecruited`); the talker's action ends.
4. `BattleScreen` plays triggered scenes as overlay `DialogueScreen`s in order,
   before continuing animations (death quote before the fall fade).
5. Validation in content: referenced scene ids exist; characters in triggers
   exist in the battle setup.
6. Test chapter data (debug Quick Battle) gets one of each trigger with test scenes.

## Acceptance criteria

- [ ] Each trigger type fires at the right moment exactly as often as configured (core tests).
- [ ] Talk recruits and the recruited unit can act next player phase (or per design) — test.
- [ ] Harness: engage boss → quote overlay → combat continues; unit falls → death quote → fade.
- [ ] Replay determinism tests still pass with triggers.

## Tests required

- Unit/property: trigger evaluation; `once` flags persist across save/load.
- Harness + snapshots as above.

## Completion notes

