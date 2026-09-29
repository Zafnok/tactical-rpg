---
id: "0815"
title: "Play the title music and a skirmish track in Quick Battle"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: done
blocked_by: ["0212", "0214", "0215"]
nick_input: sign-off
completed: 2026-09-29
---

# 0815 — Title music and Quick Battle music

*Renumbered from 0814 after it merged (PR #90) at the same time as the
0030 decision that filed 0814 (Key bindings screen); the id was taken
twice.*

## Context

Split out of 0807 (2026-09-29). Nick: "the quick battle and current demo is
really missing music sorely". The audio engine (0212), the imported tracks
(0214) and the worker-thread decoding (0215) are all done, but no screen asks
for music yet, so the game is silent. 0807 needs battle files from 0801,
which is several tickets away. This ticket covers the parts that don't need
battle files.

Decided in [`docs/design/audio.md`](../../docs/design/audio.md) (0020):

- The title screen plays `title` (New Sunrise V1).
- One track per battle, across both phases.
- Skirmishes pick one track **at random** from the `skirmish` pool
  (`assets/audio/audio.ron`, `pools`).

The debug Quick Battle (`screens/battle/mod.rs`, `quick_battle`) is a test
skirmish, so it uses the `skirmish` pool. That isn't a new design choice.

Audio is data (ADR-0026): screens call `ctx.audio.play_music(cue)` /
`stop_music()`. `Game::collect_audio` feeds `MusicState`, which ignores a
request for the track already playing and fades between tracks.

## Nick input

**Sign-off:** start the game (native exe and the web build). Check that the
title music plays, that Quick Battle switches to a battle track that keeps
going through player phase, enemy phase, combat and menus, and that going
back to the title brings the title music back. Start several Quick Battles
and check that the track varies.

## Scope

**In:**
- `TitleScreen` plays `title` when it's first shown and again whenever it's
  back on top (e.g. after a Quick Battle ends and pops back to it).
- Starting a Quick Battle plays one track picked at random from the
  `skirmish` pool. The track keeps playing for the whole battle: both phases,
  combat playback, menus, rewind. The battle screen never asks for music again
  after the start.
- A reusable pool pick, `pick_from_pool(manifest, pool_id, seed) ->
  Option<&str>`, for 0807 and 1008 to use later.
- The random pick doesn't use core's simulation RNG (ADR-0019), so it never
  changes a battle's outcome or replay. `app` puts a seed in `Ctx` at startup
  (taken from the clock). `Ctx`'s default and the Harness use a fixed seed.

**Out (do not do):**
- The `music` field in battle files, choosing Chapter 1's track, and stopping
  music on Game Over / "To be continued" (0807, after 0801).
- Victory/defeat stings and Game Over music (0809).
- Sound effects (0424, 0425).
- Volume settings (0805).
- Any music for `New Game`'s placeholder screen. It just keeps whatever is
  playing, i.e. the title music.

## Implementation steps

1. **Seed in `Ctx`** (`crates/ui/src/screen.rs`): add `pub music_seed: u64`
   (default a fixed constant, documented). In `crates/app/src/main.rs`, set
   it from the clock at startup (`app` owns the clock, ADR-0004). Native:
   `SystemTime`. Web: macroquad's `get_time()`/`miniquad::date::now()`,
   whichever works on both targets.
2. **Pool pick** (`crates/ui/src/audio.rs`): `pub fn pick_from_pool<'a>(
   manifest: &'a AudioManifest, pool: &str, seed: u64) -> Option<&'a str>`.
   Mix the seed (e.g. splitmix64) and index into the pool. `None` for an
   unknown or empty pool. Keep it pure and deterministic.
3. **Title music** (`crates/ui/src/screens/title.rs`): the `Screen` trait has
   no "shown" hook, so add a `music_on: bool` field to `TitleScreen`. At the
   top of `update`, if it's false, call `ctx.audio.play_music("title")` and set
   it to true. When the title pushes another screen that sets its own music
   (Quick Battle), set it back to false. The next `update` after the battle
   pops then asks for `title` again. `MusicState` ignores repeats, so don't
   add any other checks.
4. **Quick Battle music** (same file, the `QUICK_BATTLE` arm): keep a
   `battles_started: u64` counter on `TitleScreen`. Pick with
   `pick_from_pool(&ctx.content.audio, "skirmish", ctx.music_seed ^
   battles_started)` (or any mix of the two), call `ctx.audio.play_music(cue)`,
   increment the counter, and push the battle. Doing it here, not in
   `BattleScreen`, keeps the battle screen music-free until 0807 gives battle
   files a `music` field. 0807 then moves the call to where battles start.
5. Put the pool id `"skirmish"` in one named constant with a doc comment.
6. Update 0807's scope if anything here changes what it has to do (0807
   already assumes the title part is done).

## Acceptance criteria

- [x] Harness: the first frame on the title emits `PlayMusic { cue: "title" }`,
      and a later title frame emits no music request.
- [x] Harness: choosing Quick Battle emits one `PlayMusic` with a cue from the
      `skirmish` pool. A full player phase → enemy phase → player phase
      cycle with a combat emits no further music request.
- [x] Harness: when the Quick Battle ends and pops back, the title emits
      `PlayMusic { cue: "title" }` again.
- [x] Unit: `pick_from_pool` returns `None` for an unknown pool, the same cue
      for the same seed, and (property test over many seeds) every track in
      `skirmish` at least once.
- [x] Two Quick Battles in a row with the same `music_seed` can pick different
      tracks, because the counter changes the seed.
- [ ] Nick signed off.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `pick_from_pool`.
- Property: every pool track is reachable.
- Snapshot / integration: Harness title and Quick Battle music tests.

## Completion notes

- `Ctx::music_seed` (fixed `DEFAULT_MUSIC_SEED` = 0 in tests; `app` sets it
  from `miniquad::date::now()`, the same clock source the audio variant
  picker already uses). `ui::audio::pick_from_pool` mixes the seed with
  splitmix64. Core's simulation RNG is untouched, so rewinds and replays
  never change the track.
- `TitleScreen` asks for `title` on its first update and again on the first
  update after a Quick Battle (the `music_on` flag). It picks the Quick
  Battle track itself, mixing a per-title counter into the seed so each
  Quick Battle rolls afresh.
- Tests: pool-pick unit and property tests (`audio.rs`); title unit tests
  (music on show, after a Quick Battle, every pool track reachable across
  Quick Battles); Harness tests `the_title_asks_for_its_music_once` and
  `quick_battle_keeps_one_skirmish_track` (combat, rewind screen, enemy
  phase, turn 2: no further music); `game::tests::the_title_music_returns_after_a_battle`
  (win a battle, pop back, `title` requested again).
- On the web, the browser holds audio until the first keypress (ADR-0026
  §5), so the title music starts on the first key, not on page load.
- ADR-0026 says pool picks are 0807's job. This ticket was split out of
  0807, and 0807 now reuses `pick_from_pool`, so the ADR still holds and no
  new ADR is needed.
- No gameplay rules decided. Follow-ups: none. 0807 was trimmed (per-battle
  music from battle files only) and now depends on this ticket.
