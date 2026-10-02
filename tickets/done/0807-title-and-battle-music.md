---
id: "0807"
title: "Per-battle music from the battle file (cue or skirmish pool)"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: done
blocked_by: ["0212", "0214", "0801", "0814"]
nick_input: sign-off
completed: 2026-10-01
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
- Every existing battle file gets a `music` value (the test battle uses
  `Pool("skirmish")`, as Quick Battle does today). Chapter 1's battle file
  doesn't exist yet: 0803 writes it after this ticket and chooses its cue
  there (0803 is blocked by this ticket since 2026-10-01).
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

- [x] Harness: a battle emits its cue once at the start, and not again across
      a full player phase → enemy phase → player phase cycle, combat
      playback, or a rewind.
- [x] A `Pool` battle plays a track from that pool (0814's property test
      already covers reachability).
- [x] The validator rejects a battle file with a missing or unknown `music`.
- [ ] Nick signed off.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: schema/validator.
- Snapshot / integration: Harness battle music tests.

## Completion notes

**Done.** Every battle file now names its music, and the game flow plays
it.

- **Battle file:** a required `music: Cue("…") | Pool("…")`
  (`trpg_core::BattleMusic`, plain data on `BattleDef`; core plays and
  picks nothing). The loader refuses a missing `music`, a `Cue` that isn't
  a music cue and a `Pool` that isn't a pool of the audio manifest (the
  audio manifest is now loaded before the battles; with a broken manifest
  the battles are skipped like with any other broken dependency).
  `test.ron` and `quick.ron` use `Pool("skirmish")`.
  `assets/battles/README.md` documents the field.
- **Flow:** `FlowScreen::restart` (every battle start: the first, `Retry
  Battle`, `Restart Battle`) asks for the battle's music once. A pool pick
  goes through the new `Ctx::pick_music(pool)`: 0814's `pick_from_pool`
  with `ctx.music_seed` mixed with a count of the picks made this run.
  The count lives in `Ctx`, not in the flow, because each New Game / Quick
  Battle is a new flow and would otherwise start at the same pick. Core's
  RNG is untouched (ADR-0019), so rewinds and replays never change the
  track. The battle screen asks for no music, so the track holds through
  phases, combat, menus and rewinds.
- **Title:** 0814's Quick Battle pick is gone from `TitleScreen`. The title
  now asks for `title` again whenever it comes back from the flow (New Game
  or Quick Battle); if `title` is still playing (backing out of the mode
  screen) that request does nothing.
- Game Over and "To be continued" emit `stop_music` (0809 replaces Game
  Over's).
- No new ADR: ADR-0026 already says picking from a pool is this ticket's
  job and how requests reach `app`.

**How it plays until 0710 lands:** New Game's mode screen, lead screen and
intro scenes keep the title music; the battle's track starts with the
battle and carries on through the victory scenes; "To be continued" is
silent; back on the title, the title music.

**Claude's starting rules** (Nick: agree or veto)

1. The Preparations screen (0408, not built yet) will already play the
   battle's track. Nothing to hear yet: today the track starts when the
   battle appears.
2. The Game Over and "To be continued" screens are silent: the battle's
   music fades out when they appear.
3. `Retry Battle` and `Restart Battle` in a skirmish roll the track again
   (it may land on the same one). In a story battle with one chosen theme,
   `Restart Battle` lets the theme carry on without starting it over;
   `Retry Battle` after Game Over starts it from the top (it was stopped).

**Tests:** content `battle::tests::music_errors` (missing, unknown cue,
unknown pool, a pool as a cue, a cue as a pool, a sound as music) and the
all-assets test; Harness `tests/battle.rs`
`quick_battle_keeps_one_skirmish_track` and
`a_battle_keeps_the_cue_its_file_names` (asked once; a fight with combat
playback, a rewind, the enemy phase and turn 2 change nothing);
`tests/flow.rs` (music from the battle's start to "To be continued" and the
title; Game Over silent and Retry picks again; Restart Battle); title unit
tests.

**For Nick to try** (Pages build, after merge): Quick Battle a few times:
each plays one skirmish track for the whole battle, through the enemy
phase, fights and rewinds, and they aren't always the same track. New
Game: the title music until the battle starts, then a skirmish track
through the victory scene, silence on "To be continued", the title music
back on the title. Lose on purpose (wait out the 3 turns): Game Over is
silent, `Retry Battle` brings music back.

Follow-ups: none new. 0803 picks Chapter 1's cue; 0710 gives scenes their
own music; 0809 gives Game Over its music; 0408 pushes Preparations from
the flow (rule 1).
