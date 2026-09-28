# ADR-0025: Event playback runs inside the battle screen, as a mode

- **Status:** Accepted
- **Date:** 2026-09-27
- **Related tickets:** 0404, 0502, 0410, 0407, 0705

## Context

Ticket 0404 animates a combat from its `Event`s: strikes, HP bars draining,
a unit fading out where it fell. Its text asked for the playback as "a Screen
pushed with the events". But a pushed screen can't show what the playback
needs. ADR-0017 lets only the top screen update, and draws the screens below
from their own state. Three forces:

- **The map is ahead of the playback.** By the time the events are played,
  `BattleState` already holds the result: the defender's HP is final and a
  fallen unit has left the map. The map's HP bars, the side panel and the
  fading unit must show the *played* state, and the screen that draws the
  map is the battle screen, which doesn't update while an overlay is on
  top.
- **The frame it ends in.** A pushed screen's `Pop` drops the rest of that
  frame's actions (ADR-0017), and the ticket asks that no input is lost
  after a skip.
- **Other tickets play events on the same map.** Enemy-phase playback
  (0502) runs moves and combats one after another; spells (0410), items
  (0407) and death quotes (0705) add their own beats.

## Decision

1. **Playback is a mode of `BattleScreen`** (`Mode::Combat`), holding a pure
   `Playback` (`screens/battle/playback.rs`). `BattleScreen::apply` runs the
   command. If the events hold a combat, it builds the playback from the
   events, the units as they were before, and the battle's fallen units.
   Otherwise it goes on as before (`Mode::after_command`).
2. **A playback is a timeline of beats**, laid out when it starts (`Beat`:
   intro, flash, result, gap, fall, outro), and a clock advanced by `dt`
   (ADR-0004 rule 3). What it shows at time `t` is a pure function of the
   events and `t`: each fighter's HP (`hp`), the fade of a fallen unit
   (`fade`), the message line (`message`). Timings are one const struct
   (`TIMINGS`), tunable.
3. **The battle screen draws the played state.** While a playback runs, units
   are drawn from `shown_units`: the battle's units with the playback's HP,
   plus the fallen ones until they have faded. The side panel and the box
   read the same values. The battle itself is never rewound or copied back.
4. **Input goes through `mode::step` like any mode**: Confirm presses the
   playback (hold for ×4, tap to skip); other actions are ignored. The
   playback ends in `Mode::tick`, after the frame's actions, so the next
   frame's keys reach the browse mode.
5. **New kinds of playback add beats**, not screens. 0502 plays the AI's
   commands through the same `apply` → `Mode::Combat` path. 0705 inserts a
   quote beat before each `Beat::Fall` (the fallen units are
   `Playback::falls`). 0410 and 0407 add beats for heals and terrain
   changes.

## Consequences

- The map, the side panel and the playback box always agree, and tests read
  them all from one screen (`Harness::with_screen`).
- `BattleScreen` owns more modes. The beat logic stays out of it, in
  `playback.rs`, which has no drawing state and is unit-tested on its own.
- Applying a command clones the unit list first (a few dozen units) to know
  the combatants' starting HP and names.
- Screens that do take over the whole display (the 0413 full-body combat
  scene, level-up in 0602) can still be pushed. They get the events and the
  before-state the same way.

## Alternatives considered

- **A pushed overlay screen** (as the ticket first said). It would need the
  map screen to show pre-combat HP while not updating, and a way to hand
  the result back after `Pop`. It loses the actions of the frame it pops
  in.
- **Apply the command only when the playback ends.** The events only exist
  after applying, and a preview can't know the rolls without applying
  anyway.
- **Keep a copy of the pre-command `BattleState` and draw it while the
  playback runs.** That's simple for HP, but a fading unit and the HP drain
  still need the playback's values, and there would be two sources of truth
  for the map.
