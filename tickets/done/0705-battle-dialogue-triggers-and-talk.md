---
id: "0705"
title: "Battle dialogue triggers: turn events, boss encounters, death quotes, Talk/recruit"
type: feature
milestone: M6 Story & dialogue
model: opus-5.5
effort: high
status: done
blocked_by: ["0704", "0405"]
nick_input: none
completed: 2026-09-29
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
   (death quote, plays *before* the unit is removed visually; a player unit
   may have separate Classic death / Casual retreat scenes per
   `docs/design/death-and-difficulty.md`, picked by the campaign's mode), `Talk { a, b }`.
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

- [x] Each trigger type fires at the right moment exactly as often as configured (core tests).
- [x] Talk recruits and the recruited unit can act next player phase (or per design) — test.
- [x] Harness: engage boss → quote overlay → combat continues; unit falls → death quote → fade.
- [x] Replay determinism tests still pass with triggers.

## Tests required

- Unit/property: trigger evaluation; `once` flags persist across save/load.
- Harness + snapshots as above.

## Completion notes

- **Core** (`trpg_core::battle`, `battle/triggers.rs`): `Trigger { when,
  scene, once }` with `TriggerWhen::{TurnStart, UnitEntersArea,
  CombatStart, UnitFell, Talk}`, `Who`, `TileRect` and `GameMode`
  (Classic/Casual, for 0801's campaign). `BattleSetup` gained `triggers`
  and `mode`; `BattleState` saves them and the fired-once set. After every
  command (and at the start) the battle inserts `Event::SceneTriggered` at
  the moment it belongs (after a phase start or a move, before a combat or
  a fall). `UnitAction::Talk { target }`, `BattleState::talk_targets`,
  `Event::UnitRecruited`, `CommandError::CannotTalk`. ADR-0030 records the
  design.
- **Small deviations:** `CombatStart` also takes `against: Option<character>`
  (the Chapter 1 beat sheet wants Harl's lines per opponent; the ticket left
  it to us) and the once-per-pair flag is `per_opponent`. `CharacterId` now
  reads from a bare string in RON (`unit: "harl"`); no saves existed yet.
  The talk's scene is emitted by the action itself (not by the after-command
  pass), then `UnitRecruited`.
- **Content:** `trpg_content::check_triggers` (scenes exist, characters are
  units of the battle, areas on the map, no self-talk). The battle-file
  loader (0801) must call it; the Quick Battle calls it now.
- **Screen:** combats hold scenes as zero-length playback beats (the clock
  stops there; a Cancel-skip stops at each scene too); other scenes queue
  with the phase/outcome banners in event order. Both play as the 0704
  `DialogueScreen` overlay. `Talk` joins the action menu when someone next
  to the unit can be talked to; the player then picks the target with the
  cursor (`f talk`). The rewind list reads "X talked to Y".
- **Quick Battle fixture:** a placeholder enemy "Test Rogue" (`test_rogue`
  in `characters.ron`) arrives on the fort at (12, 3) in turn 2's enemy
  phase (a reinforcement, so turn 1 is unchanged). One of each trigger with
  placeholder scenes in `assets/dialogue/test_triggers.dlg`: turn 3's
  start, a player unit entering the walled fort room, the rogue's engage
  line and last words, the lord's and the knight's fall lines (the knight's
  Classic vs Casual), and the lord talking the rogue into joining.
- **Tests:** core unit tests per trigger kind, Talk and save/load; the
  random-play property tests now get random triggers and Talk commands
  (scenes always at their moment, `once` fires once, recruits join done);
  replay and save/load determinism with triggers; content validator; screen
  tests for the queue and Talk; harness runs for the engage line, the death
  quote before the fade, a skipped combat and the Quick Battle's turn 3;
  two snapshots (`trigger_tests__*`).
- **For 0502:** until the AI plays its phase, an AI phase ends when its
  banner closes, so a turn-start scene of an AI phase plays after that
  phase has ended in `core`. Enemy-phase boss lines and death quotes will
  work once 0502 routes the AI's commands through `apply`.
- **Nick, when playing:** in the debug Quick Battle end two turns: the
  rogue shows up on the fort, turn 3 opens with a line; walk the lord next
  to it and pick `Talk`, or attack it for its engage line and last words.
  All the lines are placeholders (0707 writes the real ones).

Gameplay rules decided where the design docs were silent (*Claude's
starting rules*, veto any):

1. A recruited unit is done for the rest of that phase and acts from the
   next player phase (the ticket's default).
2. Talking needs the talker right next to the other unit, goes one way
   (the trigger's first character talks to the second), ends the talker's
   action and gives no EXP.
3. Recruiting the last enemy on a Rout map wins the battle.
4. A boss's engage line plays whether it attacks or is attacked, and
   before an extra Line Pierce strike too; when both fighters have a line,
   the attacker's plays first.
5. A line written for one opponent (Harl vs the lead) replaces the default
   line for that pair, even after it has played once.
6. "Entering an area" means ending a move there; walking through, being
   shoved in or arriving as a reinforcement doesn't count.
7. A turn-start scene plays after that phase's banner.
8. Skipping a combat with Cancel still stops for its lines (death quotes
   aren't skipped with the animation); the scene itself can still be
   skipped with its own Cancel.

Follow-up tickets: none.
