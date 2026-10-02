# ADR-0037: A music clock: `app` reports what is sounding, `ui` wraps it at the track's length

- **Status:** Accepted
- **Date:** 2026-10-01
- **Related tickets:** 0227 (this), 0036, 0817–0820 (the title cinematic that reads it), 0212, 0215

## Context

Nick wants a title cinematic that lasts as long as the title song, loops when
the song loops and shows the logo in the song's quiet pause (ticket 0036). A
screen must therefore know, every frame, how far into its track the music
is.

ADR-0026 gives screens no way to know. They only *ask* for music;
`MusicState` (in `ui`) turns requests into `Load`/`Start`/`Gain`/`Stop`
commands and `app` plays them. Forces:

- **The track doesn't sound when it is started.** `app` plays a track once
  its file has loaded: a worker-thread decode on native (150–740 ms for the
  current tracks, ADR-0028), a fetch and a decode on the web (seconds on a
  slow connection). A screen counting frame time from its own `play_music`
  call would run ahead of what is heard by that much.
- **Frames stop, music doesn't.** A hidden browser tab gets no animation
  frames, but Web Audio keeps playing. Summed frame times fall behind.
- **quad-snd 0.2 has no position and no length.** Its whole API is load,
  play, stop, volume (ADR-0026). It doesn't say when a track played once
  has ended either.
- ADR-0004: only `app` reads a clock; screens and the `Harness` must stay
  deterministic.

## Decision

### 1. The track's length is manifest data

Every `music` entry in `assets/audio/audio.ron` has `length_ms: <u32>`
(`MusicCue::length_ms`). The validator refuses 0.

`trpg_content::audio::ogg_length_ms(bytes)` reads a file's real length: the
granule position of its last Ogg page (for Vorbis, the number of samples in
the stream) over the sample rate in its header, to the nearest millisecond.
The last page is the last `OggS` whose page runs exactly to the end of the
file, since the four letters can also occur inside audio data.

Where the validator can read the music files (the
`every_music_file_is_in_the_music_folder` test; the game itself can't,
ADR-0026 §1), a `length_ms` more than 20 ms from the file's length is an
error whose message gives the value to write. A new or re-cut track fails
that test until its length is filled in.

The length is the loop length on both targets: natively quad-snd loops the
decoded samples, and lewton trims the last packet to that same granule
position (read from its source; the title track decoded with ffmpeg gives
exactly its 5,898,057 samples). On the web the browser loops the decoded
buffer.

### 2. `app` records when a track really starts

`Audio::play` takes `now: f64` (seconds). A `Track` gets
`started_at: Option<f64>`, set where `backend.play` is called for it: in
`MusicCommand::Start` if the track is already loaded, else in the poll that
finds its load finished. `Load` makes a fresh track (no start time); `Stop`
removes it.

`Audio::music_playing(now) -> Option<(&str, f64)>` returns the cue and
`now - started_at` (never negative) of the started track that started last.
In practice only one track has started at a time: `MusicState` stops the
old track in the same frame it starts the new one. So during a fade the
report is the track fading out, and after the switch it is the new track
from the moment that one sounds (silence in between if it is still
loading). A track that failed to load is never reported.

### 3. The clock is the wall clock

`main.rs` reads `miniquad::date::now()` once per frame and uses it for both
the report and `Audio::play`. Checked in the sources of miniquad 0.4.11 and
macroquad 0.4.16:

| | `miniquad::date::now()` | `macroquad::time::get_time()` |
| --- | --- | --- |
| Native | `SystemTime::now()` since the Unix epoch | `date::now() - start_time` |
| Web | JS `Date.now() / 1000` | `date::now() - start_time` |

They are the same clock; `get_time()` only subtracts the launch time. Both
keep counting while a web tab is hidden. We use `date::now()`, which
`main.rs` already reads for its seeds.

### 4. `ui` turns the report into a `MusicClock`

```rust
pub struct MusicClock { pub cue: String, pub position: f32, pub length: f32 }
impl MusicClock {
    pub fn from_elapsed(manifest: &AudioManifest, cue: &str, elapsed: f64) -> Option<Self>
}
```

`position` is in seconds, in `0..length`. A looped track's position is
`elapsed % length`; a track played once has no clock once `elapsed` reaches
its length (that is how "it has ended" is known, since quad-snd doesn't
say). An unknown cue has none.

`Game::set_music_playing(Option<(&str, f64)>)` stores it in
`Ctx::music_clock: Option<MusicClock>` (default `None`). `main.rs` calls it
before `game.frame` each frame, with the report from before that frame's
commands are played, so the clock a screen reads is at most one frame old.
`None` means silence: nothing asked for, the track still loading, its file
missing, or a one-shot that has ended. A screen that keeps time with the
music must do something sensible then (the cinematic tickets decide what).

### 5. The `Harness` plays the part of `app`

A small `Player` in `crates/ui/src/harness.rs` follows each frame's music
commands: `Start` starts counting from 0 and each later frame adds its
`dt`; `Stop` (or a new `Load`) of that track ends it. The Harness calls
`set_music_playing` before every frame, as `app` does.
`Harness::music_load_delay(seconds)` makes a track sound that long after
its `Start`; `Harness::without_music()` makes none ever sound (missing
files). `Harness::music_clock()` reads the result. Screen tests are
deterministic and can cover slow loads and silence.

## Consequences

- A screen reads `ctx.music_clock` in `update` and `draw`; nothing else
  changes for screens that don't care.
- Adding or re-cutting a track now also means setting `length_ms`; the
  content test says what to write.
- Known limits, all fine for timing pictures to music, none fit for
  rhythm-game accuracy:
  - **Drift.** The system clock and the sound card's clock differ by a few
    milliseconds per loop of a two-minute track, and it adds up over many
    loops (the clock is not re-synchronised at the loop point).
  - **Start latency.** `started_at` is when `app` asked the backend to
    play; the sound follows within an audio buffer (some tens of
    milliseconds at most).
  - **Clock steps.** The wall clock isn't monotonic. If the system clock
    is corrected while a track plays, the position jumps by that much
    until the track next starts.
  - **Web, before the first key press.** A `play` made while the browser
    still blocks audio is queued and starts at the first key press
    (ADR-0026 §5), but `started_at` is the time of the `play`, so the
    reported position is early by the wait. The title is not affected: on
    the web it asks for its music only after that press
    (`docs/design/title-screen.md`). Any other screen that could be the
    first to play music on the web would be.
  - **A browser that pauses audio in a hidden tab** leaves the clock ahead
    of the music until the track next starts. Web Audio is specified to
    keep playing in a hidden tab, and that is what the wall clock assumes;
    some mobile browsers suspend it in the background. Not tested on a
    device for this ticket.
- No seeking, crossfades or starting part-way through a track (still
  ADR-0026). Volume settings (0805) don't touch the clock: a track at
  volume 0 still plays and still has one.
- This amends ADR-0026 (a new manifest field; `Audio::play` takes the
  time; `app` reports back to `ui`). The rest of ADR-0026 stands.

## Alternatives considered

- **Count frame time in `ui` from the `Start` command** — no `app` change,
  but ahead of the music by the load time and behind it after a hidden
  tab; the two reasons for this ticket.
- **Measure the length in `app` when the track loads** — quad-snd doesn't
  expose the decoded sample count, and screen tests in the `Harness` have
  no files. As data it is checked once, by a test.
- **Ask the audio backend for the position** — the right source, but
  quad-snd has no such call. Patching or forking it (and its JS) is a
  dependency to maintain for a few milliseconds of accuracy.
- **`AudioContext.currentTime` through a JS plugin on the web** — exact
  there and immune to the queued-play limit, but a second code path and
  another plugin; the wall clock is good enough.
- **A monotonic clock (`std::time::Instant`) on native** — immune to clock
  steps, but it panics on `wasm32-unknown-unknown`, so the two targets
  would use different clocks for a case that is rare and heals itself.
- **Put the clock in `FrameInput`** — `draw` gets only `Ctx`, and a
  cinematic draws from the position.
