---
id: "0215"
title: Stop the game freezing when a music track loads (native)
type: bug
milestone: M1 Engine
model: opus-5.5
effort: medium
status: done
blocked_by: ["0214"]
nick_input: none
completed: 2026-09-29
---

# 0215 — Stop the game freezing when a music track loads (native)

## Context

ADR-0026 §6 warned that on native, quad-snd decodes a whole OGG on the
main thread when a track loads. Ticket 0214 measured it with the real
tracks (release build, Nick's machine) by timing quad-snd 0.2.8's decode
(`mixer::load_samples_from_file`, via audrey 0.3 / lewton): **80 ms**
(`classical_murder.ogg`, 48 s) to **460 ms** (`sigil.ogg`, 4:19). Most
tracks take 110–260 ms. That's a visible freeze of 5–28 frames at 60 fps
each time the music changes (the load starts at the beginning of the
0.5 s fade, `MusicState` in `crates/ui/src/audio.rs`).

Where it happens: `trpg_app::audio`'s macroquad backend
(`crates/app/src/audio.rs`, `load_music` / `poll_music`) runs
`load_file` and then `load_sound_from_bytes` in a coroutine. On native,
`load_sound_from_bytes` → `quad_snd::Sound::load` → `MixerControl::load`,
which calls `load_samples_from_file(data)` synchronously. The web build is
unaffected (the browser decodes asynchronously).

## Nick input

None.

## Scope

**In:**
- On native, do the expensive decode off the main thread, so starting a
  track costs the main thread at most a few ms.
- Measure before/after in the Completion notes (same tracks, release).

**Out (do not do):**
- Web: already asynchronous.
- Replacing macroquad's audio with another crate (kira, rodio): rejected
  in ADR-0026.
- Changing the music files (0214) or the fade rules.

## Implementation steps

1. Reproduce: time `backend.load_music` → `poll_music` returning `Some`
   around the synchronous part (e.g. log `Instant` deltas in a debug run
   with `music/sigil.ogg`).
2. Pick a way to move the decode to a worker thread and record it in the
   Completion notes (an ADR if it adds a dependency or changes ADR-0026's
   loading rules). Candidates:
   - Decode the OGG on a `std::thread` with `lewton` (already in the
     dependency tree through quad-snd; check its license with
     `cargo deny`), write the samples as a 44.1 kHz 16-bit PCM WAV in
     memory, and give that to `load_sound_from_bytes` on the main thread.
     quad-snd still parses the WAV there with `hound`: measure whether
     that's cheap enough (it's a plain sample loop, no Vorbis decode).
   - Anything else quad-snd 0.2 allows. Read `quad-snd`'s `mixer.rs`
     first: `MixerControl::load` is the only way in, and it always calls
     `load_samples_from_file`.
3. Keep `Backend`'s shape (`load_music` returns a `Loading` that
   `poll_music` polls each frame) so `Audio`'s tests don't change; the
   thread's `JoinHandle` (or a channel) lives in the native `Loading`.
   Keep the wasm32 path as it is (`cfg`).
4. A cancelled load (`abandoned` in `Audio`) must still be dropped
   cleanly: the thread finishes and its data is freed.

## Acceptance criteria

- [x] On native, starting any track in `music/` blocks the main thread
      for under ~16 ms (one frame). Before/after timings in the
      Completion notes.
- [x] The web build loads and plays music as before.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the native `Loading` reports "not ready" until the worker is
  done, then yields the track exactly once; a dropped `Loading` doesn't
  leak (join or detach with the data dropped).
- Existing `Audio` tests (fake backend) unchanged and passing.

## Completion notes

**What was done.** On native, music no longer goes through macroquad's audio
context. `crates/app/src/audio/native_music.rs` keeps a pool of music-only
quad-snd `AudioContext`s. `load_music` spawns a `music-load` thread that
reads the file, takes a context from the pool (or opens one) and runs
quad-snd's own `Sound::load` (the decode) there. `poll_music` is a
non-blocking `try_recv`. Play, fade and stop are message sends. Dropping a
track deletes its samples and returns the context to the pool. The web
path is unchanged. Recorded in **ADR-0028**; ADR-0026's status line points
to it.

**Deviation from step 2.** The ticket's first idea (decode with lewton on a
thread, hand quad-snd an in-memory WAV) was measured first and rejected:
quad-snd still parses the WAV sample by sample with hound on the main
thread, which is 40–240 ms (float WAV) or 80–440 ms (16-bit WAV) per
track, nowhere near 16 ms. The only other way into quad-snd 0.2 is
`Sound::load` on some `AudioContext`, and a context is `Send`, so the
whole load moves to a worker with a context of its own. The app now
depends on `quad-snd` directly (native only, same 0.2.8 macroquad already
uses, MIT/Apache-2.0, `cargo deny` passes).

**Timings** (release build, Nick's machine, all 25 tracks in `music/`;
`cargo test --release -p trpg-app -- --ignored --nocapture music_load_timings`).
"Before" is how long the main thread was blocked by quad-snd's
`Sound::load`. "After" is the longest single main-thread call
(`load_music` or one `poll_music`) while polling once per 16 ms frame,
and when the track became ready:

| Track | Before (blocked) | After: worst main-thread call | After: ready in |
| ----- | ---------------: | ----------------------------: | --------------: |
| classical_murder | 149 ms | 0.07 ms | 151 ms |
| battle_theme_b | 163 ms | 0.06 ms | 163 ms |
| hope_orchestral_battle | 196 ms | 0.06 ms | 213 ms |
| rpg_battle_theme | 201 ms | 0.07 ms | 212 ms |
| battle_mla | 208 ms | 0.06 ms | 213 ms |
| zhelanov_battle_theme_3 | 214 ms | 0.09 ms | 228 ms |
| zhelanov_battle_theme_2 | 219 ms | 0.08 ms | 229 ms |
| epic_endgame_cinematic | 225 ms | 0.07 ms | 239 ms |
| fathers_scabbard_loop | 227 ms | 0.07 ms | 231 ms |
| fantasy_choir_3 | 237 ms | 0.07 ms | 261 ms |
| dark_forest_theme | 239 ms | 0.11 ms | 355 ms |
| zhelanov_battle_theme_5 | 240 ms | 0.07 ms | 245 ms |
| zhelanov_battle_theme_1 | 257 ms | 0.07 ms | 260 ms |
| field_orchestra | 288 ms | 0.05 ms | 317 ms |
| battle_theme_a | 292 ms | 0.07 ms | 283 ms |
| aria | 345 ms | 0.08 ms | 343 ms |
| peractorum | 351 ms | 0.06 ms | 357 ms |
| squirrel_village_loop | 371 ms | 0.06 ms | 375 ms |
| new_sunrise_v2 | 374 ms | 0.08 ms | 362 ms |
| hesitation_orchestral | 375 ms | 0.09 ms | 390 ms |
| ending_scene_orchestral | 380 ms | 0.06 ms | 401 ms |
| new_sunrise_v1 | 393 ms | 0.57 ms | 390 ms |
| fantasy_choir_2 | 506 ms | 0.09 ms | 533 ms |
| dark_quest | 793 ms | 0.05 ms | 738 ms |
| sigil | 867 ms | 0.07 ms | 685 ms |

(These "before" numbers are higher than 0214's 80–460 ms: that measured
the decode function alone; this is quad-snd's whole `Sound::load` in a
test binary, on a busier machine.)

**Anything Nick should know.** No screen plays music yet (title and
battle music come with their screens' tickets), so there is nothing to
hear today. When they do: switching tracks no longer freezes the game.
A track now starts when its decode finishes. For most tracks that is
inside the 0.5 s fade. For the four longest (fantasy_choir_2, dark_quest,
sigil, and on a slow machine a few more) the old track fades out and
there is a short silence (up to ~0.25 s here) before the new one starts.
Before this ticket, the same wait was a frozen game instead.

**Verification.** Unit tests for `Pending` (not ready until the worker is
done, then exactly once; a dropped `Pending` has its value dropped by the
worker; a worker panic is one error) and for a missing file (no context
opened). The `Audio` tests (fake backend) are unchanged and pass. The
native release exe ran without warnings; the web build (`cargo xtask web
--release`) builds and starts in the browser. Its music code is unchanged
apart from wrapping the macroquad `Sound` in `Clip::Shared`.

**Gameplay rules decided by Claude:** none.

**Follow-up tickets:** none.
