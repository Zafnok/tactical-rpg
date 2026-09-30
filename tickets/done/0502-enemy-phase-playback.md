---
id: "0502"
title: "Enemy phase playback: camera pans, moves, attacks, fast-forward"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: medium
status: done
blocked_by: ["0501", "0405"]
nick_input: sign-off
completed: 2026-09-30
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
2. Player input during AI phase: only Confirm and Cancel are handled
   (hold Confirm = ×4 speed, including combat playback; Cancel skips a
   fight's playback, as the player's own fights do since 0418); every other
   action is ignored.
3. Pacing constants in one struct (tunable).
4. When the player phase starts, cursor returns to where it was at end of the last player phase.

## Acceptance criteria

- [x] End turn in Quick Battle → enemies act visibly one by one → player phase Turn 2.
- [x] Holding Confirm speeds everything up; nothing is skipped logically (state identical to un-sped run — test).
- [x] Harness: end turn, `wait()` long enough, state equals applying the AI commands directly.

## Tests required

- Harness integration as above; snapshot mid-enemy-move.

## Completion notes

Done. New `crates/ui/src/screens/battle/ai_phase.rs` (`Pacing`/`PACING`,
`AiAction`) and a new `Mode::AiAction` in the battle screen; tests in
`ai_phase_tests.rs` and the module's own unit tests.

- **Driver:** in an Enemy or Other phase, once nothing else is on screen
  (banner, tip, scene, EXP bar, previous action), the screen asks
  `next_command`; `None` applies `EndPhase`. The command is applied at
  once (so the battle is always the real state) and then *shown*: the map
  draws the units as they were until the walk ends, then the normal combat
  playback (ADR-0025) takes over. The 0405 stub (phase ends when its banner
  closes) is gone.
- **Pacing** (`ai_phase::PACING`, one struct): camera pan 0.25 s (only if
  the unit is out of view; it slides a tile at a time), cursor on the unit
  0.2 s, walk at the player's walk speed (12 tiles/s), ×4 while Confirm is
  held. Combat keeps the 0404 timings (already ×4 on hold).
- **Input lockout:** during the AI phase only Confirm and Cancel reach the
  screen: Confirm closes banners / EXP pages and holding it speeds up;
  Cancel skips a fight's playback. Everything else (cursor, info, danger
  zone, auto-end toggle, rewind, end turn) is ignored.
- **Cursor and camera** go back to where they were when the player ended
  their phase.
- Tests: every enemy acts one at a time and turn 2 follows; held-Confirm
  run gives the identical state and history in < 1/3 of the frames; the
  screen's final state and command history equal applying `next_command`
  directly; a Harness run (end turn, `wait`) shows exactly the screen of
  the directly-played state; lockout; Cancel skip; cursor/camera restore;
  snapshot mid-enemy-move.
- Old tests that relied on the stub now play through the enemy phase
  (`testing::through_ai_phases`, and `wait_for` in `tests/battle.rs`). The
  music test now rewinds its turn-1 fight: the hurt lord would otherwise
  die in the enemy phase.

*Claude's starting rules* (pacing is Nick's sign-off; these are defaults):

1. Enemies walk as fast as the player's units (12 tiles/s).
2. The camera only pans when the acting unit is out of view; while it
   walks, the camera follows it and the side panel shows it.
3. The auto-end toggle key does nothing during the enemy phase (the ticket
   says every other key is ignored); press it in your own phase.
4. Confirm and Cancel don't skip an enemy's walk, only speed it up (hold
   Confirm); Cancel skips fights only.

**For Nick:** Quick Battle → end the turn → watch each enemy walk and
fight; hold `f` to fast-forward, `d` to skip a fight. Is the pacing right?
