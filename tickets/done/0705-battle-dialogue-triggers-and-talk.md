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
- [x] Talk recruits and the recruited unit can act next player phase (or per design) — test. (Per design: it leaves the battle and joins after a win.)
- [x] Harness: engage boss → quote overlay → combat continues; unit falls → death quote → fade.
- [x] Replay determinism tests still pass with triggers.

## Tests required

- Unit/property: trigger evaluation; `once` flags persist across save/load.
- Harness + snapshots as above.

## Completion notes

- **Nick reviewed the starting rules on the PR** and decided recruitment,
  talking and boss lines; recorded in
  `docs/design/battle-scenes-and-recruitment.md` (new). In short: nobody
  changes sides during a battle, recruits join the army after a won battle;
  a talk that recruits makes the recruit leave the battlefield at once;
  either character may start a talk; recruiting will mostly be by defeating
  a character or (later) by quests; a boss has once-per-battle lines for
  its first fight, the first time it drops to half HP, and its defeat; a
  boss's special line for a character replaces its general line; an area
  scene plays only where a unit stops.
- **Core** (`trpg_core::battle`, `battle/triggers.rs`): `Trigger { when,
  scene, once }` with `TriggerWhen::{TurnStart, UnitEntersArea,
  CombatStart, HalfHp, UnitFell, Talk}`, `Who`, `TileRect` and `GameMode`
  (Classic/Casual, for 0801's campaign). `BattleSetup` gained `triggers`
  and `mode`; `BattleState` saves them, the fired-once set and the
  recruits (`BattleState::recruited`). After every command (and at the
  start) the battle inserts `Event::SceneTriggered` at the moment it
  belongs (after a phase start, a move or a combat for half HP; before a
  combat or a fall). `UnitAction::Talk { target }`,
  `BattleState::talk_targets`, `Event::UnitRecruited`,
  `CommandError::{CannotTalk, UnitLeft}`. ADR-0030 records the design.
- **Deviations from the steps:** the ticket's step 3 had the target switch
  to the player's side; Nick ruled that out (recruits join after the
  battle), so it leaves the map instead. Added per Nick: `HalfHp`, and
  `recruit` on `UnitFell` ("joins you if defeated"). `CombatStart` takes
  `against: Option<character>` (Chapter 1's beat sheet wants Harl's lines
  per opponent); the ticket's "once per pair" flag was dropped, since Nick
  wants each boss moment once per battle. `CharacterId` now reads from a
  bare string in RON (`unit: "harl"`); no saves existed yet.
- **Content:** `trpg_content::check_triggers` (scenes exist, characters are
  units of the battle, areas on the map, no self-talk, recruits aren't
  player units). The battle-file loader (0801) must call it; the Quick
  Battle calls it now.
- **Screen:** combats hold scenes as zero-length playback beats (the clock
  stops there; a Cancel-skip stops at each scene too); other scenes queue
  with the phase/outcome banners in event order. Both play as the 0704
  `DialogueScreen` overlay. `Talk` joins the action menu when someone next
  to the unit can be talked to; the player then picks the target with the
  cursor (`f talk`). The rewind list reads "X talked to Y".
- **Quick Battle fixture:** a placeholder enemy "Test Rogue" (`test_rogue`
  in `characters.ron`) arrives on the fort at (12, 3) in turn 2's enemy
  phase (a reinforcement, so turn 1 is unchanged). Triggers with
  placeholder scenes in `assets/dialogue/test_triggers.dlg`: turn 3's
  start, a player unit entering the walled fort room, the rogue's first
  fight, half HP and last words, the lord's and the knight's fall lines
  (the knight's Classic vs Casual), and the lord and the rogue talking (the
  rogue leaves, to join after the battle).
- **Tests:** core unit tests per trigger kind, Talk, recruits and
  save/load; the random-play property tests now get random triggers and
  Talk commands (scenes always at their moment, `once` fires once, recruits
  leave the map); replay and save/load determinism with triggers; content
  validator; screen tests for the queue and Talk; harness runs for the
  engage and half-HP lines, the death quote before the fade, a skipped
  combat and the Quick Battle's turn 3; two snapshots (`trigger_tests__*`).
- **Other tickets:** 0801 now adds `BattleState::recruited()` to the roster
  after a victory (step 3, new acceptance criterion) and runs
  `check_triggers`. New ticket **1009** (design recruitment by quests
  outside battle).
- **For 0502:** until the AI plays its phase, an AI phase ends when its
  banner closes, so a turn-start scene of an AI phase plays after that
  phase has ended in `core`. Enemy-phase boss lines and death quotes will
  work once 0502 routes the AI's commands through `apply`.
- **Nick, when playing:** in the debug Quick Battle end two turns: the
  rogue shows up on the fort, turn 3 opens with a line; walk the lord next
  to it and pick `Talk` (it leaves the map), or attack it for its fight,
  half-HP and last lines. All the lines are placeholders (0707 writes the
  real ones).

Gameplay rules Claude decided where the design was silent (veto any):

1. **Talking** uses up the talker's turn and gives no EXP.
2. **Boss lines when both fighters have one:** if the lead attacks Harl and
   both have a line for that fight, the lead's plays first, then Harl's.
3. **Recruiting the enemy you must defeat:** on a "defeat Harl" map, talking
   Harl into joining also wins the map.
4. **Half-HP line timing:** it plays right after the fight that brings the
   boss to half HP or less, not in the middle of the fight, and not at all
   if that fight defeats it (the defeat line plays instead).
