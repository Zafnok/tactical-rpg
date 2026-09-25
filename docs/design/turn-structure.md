# Turn structure

Decided: 2026-09-25
Source: ticket 0002

## Nick's words

> **Q1. Who acts when during a battle?** "Phases, like FE (Recommended)"
>
> **Q2. Moving and acting: how does a unit's turn work?** "fire emblem standard
> ONLY however certain combat skills can enable movement such as that one bow
> skill from FE that lets you move 1 tile away... no in-built Canto"
>
> **Q3. Should maps be able to spawn enemy reinforcements on set turns?**
> "Yes, never act on arrival (Recommended)"

Follow-ups:

> **How should your phase end?** "auto-end as option but have it enabled by
> default and it should have a shortcut to toggle auto-end"
>
> **Should there be a third side: allied/neutral NPC units that move on their
> own (green units in FE)?** "Yes, Other phase (Recommended)"
>
> **Turn limits: how should turn counts interact with map objectives?**
> "Per-map, optional (Recommended)"

So: Fire Emblem phases (Player → Enemy → Other), FE move-then-act with no
built-in Canto (post-action movement exists only as specific skills),
reinforcements that never act on the turn they arrive, auto-end on by default
with a toggle shortcut, and optional per-map turn limits.

Everything marked *tunable* / *Claude's starting rule* is a default Claude
chose where Nick's answer didn't say; Nick may veto it and balance tickets may
change it without asking. Unmarked rules are Nick's choices.

## Sides and phases

| Phase | Units that act | Controlled by | Banner |
| ----- | -------------- | ------------- | ------ |
| Player phase | `Player` faction | the player | `PLAYER PHASE` |
| Enemy phase | `Enemy` faction | AI | `ENEMY PHASE` |
| Other phase | `Ally` and `Neutral` factions ("green" units) | AI | `OTHER PHASE` |

- Other-phase units are AI-controlled. `Ally` units fight the enemy alongside
  the player; `Neutral` units (e.g. villagers) fight no one. Hostility between
  factions is the `Faction::is_hostile_to` table from ticket 0302.
- The player can never command Other-phase units.

## Exact sequence

```
battle start: turn = 1
loop:
    Player phase  (turn)
    Enemy phase   (turn)
    Other phase   (turn)
    turn += 1
```

Each phase:

1. **Start of phase:** every unit of the phase's factions becomes *ready*
   (`acted = false`). Then that phase's reinforcements for this turn arrive
   (see below) already *done*. Then turn-limit checks that happen at phase
   start (see Objectives). Then the banner shows.
2. **Actions:** ready units act one at a time, in any order the controller
   chooses (player: any order; AI: its own ordering, ticket 0501).
3. **End of phase:** see "Ending a phase" below.

A phase whose factions have **no living units** (and no reinforcements
arriving this phase) is skipped entirely: no banner, no pause. The turn
counter still advances after the Other phase slot, whether or not it ran.

The battle can end in the middle of any phase: objectives and defeat are
checked after every action and at every phase boundary; the moment an outcome
is decided, nothing further happens.

## A unit's action

FE standard, and **only** FE standard. There is **no built-in Canto** and no
split movement for anyone.

1. **Select** a ready unit.
2. **Move** it to any tile it can stop on (0 tiles = stay put). Moving is a
   preview: the player can cancel the move freely until they choose an action.
3. **Choose one action** from the action menu: `Attack`, `Item`, `Wait`, plus
   any action the map or other design docs add (e.g. `Seize` on a seize tile,
   `Talk`, `Trade`, `Heal`/magic per `magic.md`, `Items` per
   `weapons-and-items.md`).
4. Choosing the action **commits** the move and the action together. They
   can't be undone (except by rewind, per `death-and-difficulty.md`).
5. The unit is now **done** (`acted = true`, drawn dimmed) until the start of
   its side's next phase.

**Skill-granted movement.** Specific combat skills may give a unit movement
*after* its action, like Fire Emblem's bow skill that steps the archer 1 tile
away after attacking. These are not a general rule: each skill states exactly
what movement it grants (e.g. "after attacking, move 1 tile away from the
target"). Which skills exist and who gets them is decided by
`progression.md` (0005) and `weapons-and-items.md` (0003). The turn system
only guarantees that an action may end with a skill-defined extra move, after
which the unit is done.

## Ending a phase

**Player phase:**

- **Manual:** the player can end the phase at any time from the map menu or
  the End Turn key. If any units are still ready, a confirmation asks first
  (`End turn with N units ready?`); units still ready simply don't act.
- **Auto-end:** when **auto-end is ON** and the last ready player unit becomes
  done, the player phase ends immediately (no confirmation).
- **Auto-end is ON by default.** It's a player setting, saved with the other
  options, and has a **shortcut key to toggle it** during battle (default key
  `Shift+e`, *tunable*/rebindable), which shows a brief `Auto-end: ON/OFF`
  message. The current state is also shown in the help bar.
- With auto-end OFF and every unit done, nothing happens until the player ends
  the turn manually (no confirmation needed then, since no units are ready).

**Enemy and Other phases:** always end automatically when the AI has no more
actions to take (every unit acted or chose to wait).

## Reinforcements

Maps may schedule reinforcements: a list of `(turn, faction, units at tiles)`.

- Reinforcements for turn `N` arrive at the **start of their own side's phase
  on turn `N`**, and arrive **already done**: they never act on the turn they
  appear. Their first action is in their side's phase on turn `N + 1`.
- So enemy reinforcements scheduled for turn `N` appear at the start of Enemy
  phase `N`, and the player always gets Player phase `N + 1` to react before
  they move.
- Any side can have reinforcements (enemy waves, allied militia arriving,
  player units joining), same rule for all.
- *Claude's starting rule:* if an arrival tile is occupied, that reinforcement
  waits and tries again at the same point on the next turn (blocking a
  reinforcement point is a legitimate tactic, as in FE). Other reinforcements
  in the same wave still arrive.
- Arriving reinforcements are visible immediately (map, danger zone) and the
  arrival is an event the UI can show (camera pans to them).

## Turn limits and objectives

The current turn number is always visible (map menu → Objective, phase
banner). Turn limits are **optional and per map**; most maps have none.

- **Survive N turns** (an objective by itself): the player **wins** when turn
  `N`'s last phase ends, i.e. at the moment `turn` would become `N + 1`, if
  the player hasn't lost by then.
- **Any other objective "within N turns"** (Rout, Defeat boss, Seize, ...):
  the player **loses** if the objective isn't complete when turn `N`'s last
  phase ends. Completing it at any point up to then wins.
- A map shows its limit in the objective text, e.g. `Seize the gate (Turn 3/8)`.
- Loss conditions from `death-and-difficulty.md` (0006) are checked the same
  way (after every action and at every phase boundary).

## Open sub-questions (deferred)

- Which skills grant post-action movement, and exactly how much: tickets
  0003 / 0005.
- Whether losing an Ally/Neutral unit can lose a map ("protect the villagers"
  objectives): decided per chapter when such a map is designed (0009 and
  later chapter tickets).
