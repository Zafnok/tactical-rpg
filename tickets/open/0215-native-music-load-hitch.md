---
id: "0215"
title: Stop the game freezing when a music track loads (native)
type: bug
milestone: M1 Engine
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0214"]
nick_input: none
completed:
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

- [ ] On native, starting any track in `music/` blocks the main thread
      for under ~16 ms (one frame). Before/after timings in the
      Completion notes.
- [ ] The web build loads and plays music as before.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the native `Loading` reports "not ready" until the worker is
  done, then yields the track exactly once; a dropped `Loading` doesn't
  leak (join or detach with the data dropped).
- Existing `Audio` tests (fake backend) unchanged and passing.

## Completion notes

*(Filled in by the session that completes the ticket.)*
