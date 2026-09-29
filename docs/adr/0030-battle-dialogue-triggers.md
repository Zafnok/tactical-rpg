# ADR-0030: Dialogue triggers are battle data; their scenes are events at their moment

- **Status:** Accepted
- **Date:** 2026-09-29
- **Related tickets:** 0705, 0801, 0803, 0502

## Context

Ticket 0705 adds a map's story moments: a scene at the start of a turn, when
a unit enters an area, when a boss is engaged or drops to half HP, when a
unit falls (a death quote, or a Casual retreat line), and the `Talk` action.
Some moments recruit a character, who joins the army only after a won
battle (Nick, `docs/design/battle-scenes-and-recruitment.md`).
Forces:

- **Determinism and saves.** Which `once` triggers have fired must survive a
  suspend save (0802), a rewind (0307) and a replay, like the rest of the
  battle (ADR-0004, ADR-0020).
- **Timing.** A death quote must play *before* the unit fades (ADR-0025
  planned it as a playback beat); a boss's line before the combat; a
  turn-start scene after its phase banner. The UI only sees events.
- **Authoring.** The battle file (0801) lists triggers in RON, naming
  characters by id (`"ana"`), and must need no code change per chapter.
- **Evaluation order.** Several triggers can fire in one command (a Line
  Pierce is two combats; a boss's specific line against the lord replaces
  its default).

## Decision

1. **Triggers are core data in the battle.** `trpg_core::Trigger { when:
   TriggerWhen, scene, once }` (serde), with `TriggerWhen` = `TurnStart`,
   `UnitEntersArea`, `CombatStart` (with `against`), `HalfHp`, `UnitFell`
   (with a `GameMode` filter and `recruit`) and `Talk` (either character may
   start it; with `recruit`). `BattleSetup` carries `triggers` and the
   campaign's `mode`; `BattleState` saves both, the fired `once` triggers
   and the recruited units. Triggers name characters (`CharacterId`), never
   `UnitId`s, which battle files don't know.
2. **Recruits leave the battle; the campaign adds them.** A recruit (by
   talk: removed from the map at once; by defeat: as it falls) goes into
   `BattleState::recruited` with `Event::UnitRecruited`. Nobody changes
   faction in a battle. 0801's `Campaign::apply_result` adds the recruits
   to the roster after a victory.
3. **Scenes are events placed at their moment.** After every command (and
   at the battle's start) `BattleState` walks the command's events and
   inserts `Event::SceneTriggered { scene }` where it belongs: after a
   `PhaseStarted`, `UnitMoved` or (half HP) `CombatResolved`, before a
   `CombatResolved` or `UnitFell`.
   Several at once go in trigger-list order. A talk emits its own scene,
   then `UnitRecruited`. Core never looks at the scene's content.
4. **The battle screen plays scenes as `DialogueScreen` overlays.** With a
   combat, the playback holds them as zero-length `Beat::Scene`s before the
   bout or fall they precede; its clock stops there until the screen takes
   the scene and pushes the overlay (a skip stops at each scene too). Without
   a combat, scenes join the banners in one queue (`Queued`), in event
   order, and play when they reach its front.
5. **`CharacterId` serialises as a bare string** (`#[serde(transparent)]`),
   so battle files write `unit: "harl"`. No save format existed yet.
6. **Content checks triggers** with `trpg_content::check_triggers`: scenes
   exist, named characters are units of the battle (arrivals included),
   areas lie on the map, recruits aren't already player units. The battle-file loader (0801) calls it; the debug
   Quick Battle does today.

## Consequences

- Rewind and replay need nothing new: re-applying commands rebuilds the
  fired set, and a rewound command's scene plays again if repeated.
- Any new command that moves units or fights gets its scenes for free, as
  long as it emits the usual events. A move that emits no `UnitMoved` (a
  shove, an arrival) doesn't enter areas.
- The enemy-phase playback (0502) plays the AI's commands through the same
  `apply` path, so boss lines and death quotes in the enemy phase work once
  it lands. Until then an AI phase ends when its banner closes, so a
  turn-start scene of an AI phase plays after that phase has already ended
  in `core`.
- `BattleSetup` literals gained two fields (`triggers`, `mode`).
- A recruited unit is out of the battle like a fallen one: commands naming
  it fail with `CommandError::UnitLeft`.

## Alternatives considered

- **Return triggered scene ids beside the events** (a separate list per
  command): the UI would have to re-derive where each scene goes (which
  fall? which combat?). Placing them in the event stream keeps one ordered
  source of truth.
- **Evaluate triggers in the UI**: breaks determinism of saves and replays
  and duplicates rules outside `core` (ADR-0004).
- **Recruits switch to the player's side at once** (as in Fire Emblem):
  Nick ruled it out; recruits join after the battle.
- **Triggers keyed by `UnitId`**: battle files would need to know generated
  ids; characters are what authors name.
- **A raw RON mirror type in `content`, converted to core's `Trigger`**: two
  types to keep in step for no gain once `CharacterId` reads from a string.
