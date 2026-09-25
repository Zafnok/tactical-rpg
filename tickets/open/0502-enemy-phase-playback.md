---
id: "0502"
title: "Enemy phase playback: camera pans, moves, attacks, fast-forward"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0501", "0405"]
nick_input: sign-off
completed:
---

# 0502 — Enemy phase playback

## Context

Replaces the 0405 stub: during non-player phases the battle screen asks
`core::ai::next_command` for each action and animates it using the move
animation (0403) and combat playback (0404).

## Nick input

**Sign-off:** Nick plays a few turns in Quick Battle and says whether enemy
phase pacing is right.

## Scope

**In:** AI phase driver in `BattleScreen`, camera pan, per-action pacing,
fast-forward, input lockout, returning control at player phase.

**Out:** "skip enemy phase entirely" option (0805 adds the setting).

## Implementation steps

1. On `PhaseStarted` for a non-player phase (Enemy **and** Other, per
   `docs/design/turn-structure.md`): after the banner, loop:
   `next_command` → if `None`, apply `EndPhase` → else animate: camera pans
   (smooth, ~0.25 s) to the unit, 0.2 s highlight, walk along `UnitMoved.path`,
   then combat playback for `CombatResolved`, then next.
2. Player input during AI phase: only Confirm is handled (hold = ×4 speed,
   including combat playback); every other action is ignored.
3. Pacing constants in one struct (tunable).
4. When the player phase starts, cursor returns to where it was at end of the last player phase.

## Acceptance criteria

- [ ] End turn in Quick Battle → enemies act visibly one by one → player phase Turn 2.
- [ ] Holding Confirm speeds everything up; nothing is skipped logically (state identical to un-sped run — test).
- [ ] Harness: end turn, `wait()` long enough, state equals applying the AI commands directly.

## Tests required

- Harness integration as above; snapshot mid-enemy-move.

## Completion notes

