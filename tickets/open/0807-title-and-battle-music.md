---
id: "0807"
title: "Per-battle music from the battle file (cue or skirmish pool)"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0212", "0214", "0801", "0814"]
nick_input: sign-off
completed:
---

# 0807 — Per-battle music from the battle file

## Context

The title music and a stand-in for Quick Battle (a random `skirmish` track
picked on the title screen) were split out into **0814** (2026-09-29) so
the demo has music before 0801 lands. This ticket does the rest: each
battle file names its music.

From [`docs/design/audio.md`](../../docs/design/audio.md) (ticket 0020):

- The title screen plays `title` (New Sunrise V1).
- **One track per battle, across both phases.** There is no enemy-phase
  music.
- Story battles each name a theme (`battle_bright`, `battle_bittersweet`,
  `battle_easy`, `battle_new_area`, `battle_epic_a/b`,
  `battle_church_2/3`).
- Skirmishes pick one track **at random** from the `skirmish` pool.

Battle files are defined in 0801 (`assets/battles/*.ron`).

## Nick input

**Sign-off:** start the game and a battle and check that the music plays and
keeps playing across phases and combat.

## Scope

**In:**
- The battle file gets a required field
  `music: Cue("battle_bright") | Pool("skirmish")`. The validator checks that
  it names a music cue or a pool in the audio manifest. Update
  `assets/battles/README.md`.
- When a battle starts, its music plays. The track stays through player and
  enemy phases, combat playback and menus. A `Pool` picks one track at
  random when the battle starts.
  - Where the random pick comes from is a technical choice: prefer not to
    touch core's simulation RNG (ADR-0019), so rewinding never changes the
    music.
  - Rewinding (0307) doesn't restart the music.
  - Restarting the battle may pick again.
- Every existing battle file gets a `music` value. For `ch01`, **ask Nick**
  (ask-nick, one short question with the `battle_*` cues from `audio.md`)
  unless 0803 has already chosen one.
- Scenes around the battle set their own music with `@music` (0710). If 0710
  isn't done yet, the battle's track keeps playing into the victory scenes.
- *Claude's starting rules* (list them in the PR for Nick):
  - The Preparations screen (0408) already plays the battle's track.
  - The Game Over and "To be continued" screens stop the music.

**Out (do not do):**
- Place music on the world map and in towns (1007), and skirmish battles
  themselves (1008, which uses this `Pool` field).
- Victory and defeat stings and Game Over music (decided in 0022–0024,
  played by 0809, which replaces this ticket's Game Over `stop_music`).

## Implementation steps

1. Add `music` to the battle file schema, loader and validator, and to every
   battle file.
2. The flow that starts a battle emits `play_music` once when the battle
   starts. For a `Pool`, use 0814's `pick_from_pool` with `ctx.music_seed`
   mixed with a per-attempt counter. Remove 0814's Quick Battle pick from
   `TitleScreen` (0801 replaces the Quick Battle wiring anyway); the title
   music part of 0814 stays.
3. Game Over and "To be continued" emit `stop_music`.

## Acceptance criteria

- [ ] Harness: a battle emits its cue once at the start, and not again across
      a full player phase → enemy phase → player phase cycle, combat
      playback, or a rewind.
- [ ] A `Pool` battle plays a track from that pool (0814's property test
      already covers reachability).
- [ ] The validator rejects a battle file with a missing or unknown `music`.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: schema/validator.
- Snapshot / integration: Harness battle music tests.

## Completion notes

*(Filled in by the session that completes the ticket.)*
