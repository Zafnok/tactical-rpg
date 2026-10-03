---
id: "0825"
title: "Cinematic shots where units march and fight"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: todo
blocked_by: ["0817"]
nick_input: none
completed:
---

# 0825 — Cinematic shots where units march and fight

## Context

Nick picked the "Trailer cuts" storyboard for the title's intro cinematic
(ticket 0036, `docs/design/title-screen.md`, *Intro cinematic*). In the
loud part of the song its shots show units **walking across the
battlefield and fighting**: a march toward the enemy (3×), two close-ups
where units trade blows and an enemy falls (4×), and the whole field with
the rest of the party advancing (2×).

0817's `MapPan` shot only shows a battle's units standing at their
starting places. This ticket lets a map shot carry a small script of moves
and strikes.

The fights are **authored pictures, not played battles**: the cinematic
does not run the combat rules (`core` is not involved), so nothing here
can drift when stats are rebalanced, and every frame is still drawn from
the time `t` alone (0817's rule, its ADR).

Graphics are a skin (ADR-0038): units and the map are painted by the map
skin from a map scene. This ticket changes what is **in the scene** at
time `t` (where each unit is, its HP bar, whether it is flashing or
gone); it draws nothing itself.

## Nick input

None. The storyboard is decided (0036). Nick sees the result at 0820's
sign-off.

## Scope

**In:**
- `MapPan` gets an optional `action` list in the cinematic file format:
  timed `Walk` and `Strike` entries (below), and an `action_from` time so
  several shots can continue one fight across cuts.
- The player builds the map scene for time `t` from the battle's starting
  places plus the actions that have happened by then.
- Validator rules, `assets/cinematics/README.md`, a short action in
  `assets/cinematics/test.ron`.
- An addendum to 0817's ADR (or a new ADR that extends it): actions are
  authored, drawn from `t` alone, and never use the combat rules.

**Out (do not do):**
- Running `core` combat, the forecast, the combat scene (0413) or battle
  sounds. The cinematic makes no sounds of its own (`title-screen.md`).
- EXP, level-ups, items, spells, terrain effects.
- The real title cinematic's action (0820 writes it on the Chapter 1 map).
- Camera changes inside a shot beyond 0817's pan: one size per shot
  (0036).

## Implementation steps

1. **Format** (extend 0817's; keep its naming style):
   ```ron
   (at: 68.0, shot: MapPan(battle: "ch01", from: (6.0, 12.0), to: (10.0, 8.0), zoom: 3,
       action_from: 0.0,
       action: [
           (at: 1.0, do: Walk(unit: "sergeant", path: [(6, 12), (11, 12), (11, 8)], secs: 6.0)),
           (at: 18.5, do: Strike(unit: "lead", target: "bandit_3", hp_left: 40)),
           (at: 22.5, do: Strike(unit: "lead", target: "bandit_3", hp_left: 0)),
       ])),
   ```
   - `action_from` is the action clock's value at the shot's start; the
     clock then runs in real seconds. A later shot with the same `action`
     list and a later `action_from` shows the same fight further on, from
     another camera. To avoid repeating the list, allow a top-level
     `actions: { "<name>": [ … ] }` table and `action: "<name>"` in shots.
   - `Walk`: the unit moves along `path` (tiles; each step to a
     neighbouring tile or a straight run) at an even speed over `secs`,
     and stays at the end.
   - `Strike`: at `at` the unit lunges a few pixels toward the target and
     back (about 0.4 s); halfway through, the target flashes and its HP
     bar drops to `hp_left` percent. At `hp_left: 0` the target fades out
     over about 0.5 s and is gone afterwards.
   - `unit` / `target` are the battle file's unit ids (player slots by
     character id, enemies by their `id`).
2. **Validator:** units and targets exist in the battle; paths start where
   the unit is at that moment (its starting place or the end of its last
   walk), stay on the map and on tiles its movement type can enter; no
   action on a unit after a strike left it at 0; `hp_left` is 0–100 and
   never rises; entries sorted by `at`. All errors at once with the file
   name.
3. **Scene at time `t`:** a pure function
   `cinematic::action_state(battle, actions, clock) -> Vec<UnitView>`
   (position in map pixels, HP fraction, flash, opacity). The player puts
   those into the map scene it hands to the map skin (0432's `MapScene`;
   if the scene's unit entries can't yet carry a pixel offset, a flash or
   an opacity, add those fields there and make both skins paint them).
4. `test.ron`: one walk and one strike that fells a unit on the test
   battle; update 0817's snapshots.

## Acceptance criteria

- [ ] Unit: `action_state` before, during and after a walk (including a
      corner of the path), at each stage of a strike, and after a unit
      falls.
- [ ] Unit: two shots with the same action and different `action_from`
      show the same unit positions at the same action-clock value.
- [ ] Each validator error has a test.
- [ ] Snapshot: the test cinematic mid-walk and mid-strike; the same `t`
      drawn twice gives the same buffer.
- [ ] The tests of what happened read the map scene, not cells (ADR-0038);
      both map skins paint a moving, flashing and fading unit.
- [ ] The ADR text is written and listed in `docs/adr/README.md`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `action_state`; the validator.
- Property: for any clock value, every unit's position lies on its path or
  at an end of it, and HP never rises as the clock grows.
- Snapshot / integration: the test cinematic through the debug "Play test
  cinematic" tool.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
