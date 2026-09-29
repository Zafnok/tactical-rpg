# ADR-0028: Native music is decoded on a worker thread, in its own quad-snd context

- **Status:** Accepted
- **Date:** 2026-09-29
- **Related tickets:** 0215 (this), 0212, 0214

## Context

ADR-0026 loads a music track with a macroquad coroutine
(`load_file` then `load_sound_from_bytes`). On native that ends in quad-snd
0.2.8's `MixerControl::load`, which decodes the whole OGG
(`load_samples_from_file`, audrey → lewton) synchronously on the main
thread before handing the samples to the mixer thread. With the real
tracks (release build, Nick's machine) that is 150–870 ms: the game froze
for 9–52 frames every time the music changed (ticket 0215 has the table).

quad-snd 0.2 has one way in: `Sound::load(&AudioContext, &[u8])`, which
always decodes. Play, volume, stop and delete only send a message to the
context's mixer thread. An `AudioContext` is just that channel's `Sender`
and two id counters, so it is `Send` on every platform (not `Sync`). Each
context opens its own output stream and mixer thread, which never exits.

Pre-decoding the OGG ourselves into an in-memory WAV and loading that on
the main thread was measured and rejected: hound's per-sample WAV read
still costs 40–240 ms (float WAV) or 80–440 ms (16-bit) per track.

## Decision

- **Web: unchanged.** Music goes through macroquad like the sounds; the
  browser decodes asynchronously.
- **Native:** music does not use macroquad's audio context.
  `trpg_app::audio::native_music` keeps a pool of spare quad-snd
  `AudioContext`s (`Contexts`, shared behind a mutex).
  - `load_music` spawns a `music-load` thread (`Pending`): it reads the file,
    takes a context from the pool (opening one if none is spare) and calls
    `Sound::load` there. quad-snd `unwrap`s decode errors, so the call is
    wrapped in `catch_unwind` and a bad file becomes an error (the context
    goes back to the pool).
  - `poll_music` is a non-blocking `try_recv`: nothing until the worker is
    done, then the track exactly once (or an error if the worker died).
  - The loaded `Music` owns its context. Play, fade (`set_volume`) and
    stop are message sends from the main thread. Dropping it deletes the
    sound (its samples are freed on the mixer thread) and returns the
    context to the pool, so contexts are opened once and reused: at most as
    many as tracks alive at the same time (two, three with a cancelled
    load), each with an idle output stream.
  - A load dropped before it finished detaches its thread: the worker's
    send fails and it drops the `Music` itself, which frees the samples and
    returns the context.
- `macroquad`'s sound effects still use macroquad's context. The backend's
  sound type is `Clip::Shared(macroquad Sound)` or `Clip::Music(Music)`.
- The app depends on `quad-snd` directly (native only), pinned to the
  version macroquad's `audio` feature already pulls in (MIT/Apache-2.0),
  so no new code ships.

This replaces ADR-0026 §3's "loaded with a macroquad coroutine" and the
native paragraph of §6 for native builds; the rest of ADR-0026 stands.

## Consequences

- Starting a track costs the main thread well under 1 ms.
- The track is ready when its decode ends, 150–740 ms after `Load` for the
  current tracks. The fade is 500 ms, so the longest tracks start a little
  after the old one has faded out (a short silence) instead of freezing
  the game; `Audio` already starts a track asked to play as soon as it is
  ready.
- Music plays through a second (third) OS audio stream beside the one for
  sounds; Windows, ALSA's default device and CoreAudio mix them. A future
  master/music volume setting (0805) keeps working: it multiplies into the
  volumes `Audio` sends either way.
- Upgrading macroquad/quad-snd must keep the two versions in step, and
  re-check that `AudioContext` is still `Send` (the compiler will).

## Alternatives considered

- **Decode to an in-memory WAV on a thread, load that on the main thread**
  (the ticket's first idea) — measured: parsing the WAV is itself 40–440 ms.
- **One worker thread owning a single music context** — fades of the old
  track would queue behind the new track's decode and stall.
- **A new context per track, never reused** — quad-snd's mixer threads and
  streams never exit, so each track change would leak one.
- **Another audio crate (kira, rodio)** — rejected in ADR-0026.
