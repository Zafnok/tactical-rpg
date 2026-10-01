---
id: "0227"
title: "Music clock: tell screens how far into its track the music is"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: ["0212", "0214", "0215"]
nick_input: none
completed: 2026-10-01
---

# 0227 — Music clock: tell screens how far into its track the music is

## Context

Nick wants a title cinematic that "should match the length of the title
song, and loop when the song does" and shows the logo during the song's
quiet pause (ticket 0036). For that a screen must know where the music is
in its track, every frame.

Today it can't. Screens only *ask* for music (`ctx.audio.play_music(cue)`,
ADR-0026). `MusicState` (`crates/ui/src/audio.rs`) emits `Load` and `Start`
commands, and `app` (`crates/app/src/audio.rs`) starts the track when its
file has loaded: a worker-thread decode on native (ADR-0028), a fetch and
decode on the web. That takes from a fraction of a second to several
seconds, so counting frame time from the `Start` command would run ahead of
what is heard. quad-snd has no "position" call and no track length.

This ticket has no design question in it and can be done any time.

## Nick input

None.

## Scope

**In:**
- Each music cue's length in the audio manifest, checked against its file.
- `app` records when a track really starts and reports, each frame, which
  track is sounding and for how long.
- `ui` turns that into a `MusicClock` screens can read from `Ctx`: the cue,
  the position in seconds (wrapped at the loop) and the track length.
- The `Harness` simulates it, so screen tests are deterministic.
- An ADR amending ADR-0026 (`write-adr` skill).

**Out (do not do):**
- The cinematic (0817–0820) or any change to the title screen.
- Seeking, crossfades, or starting a track part-way through.
- Volume settings (0805). A track at volume 0 still plays and still has a
  clock.

## Implementation steps

1. **Length in the manifest.** `assets/audio/audio.ron`: every `music`
   entry gets `length_ms: <u32>`. `trpg_content::audio::MusicCue` gains
   `pub length_ms: u32`; the validator refuses 0. Fill in all tracks: the
   length is the last Ogg page's granule position ÷ 44 100 (the title track,
   `new_sunrise_v1.ogg`, is 5,898,057 samples = 133,743 ms). Add a pure
   helper `ogg_length_ms(bytes: &[u8]) -> Option<u32>` in
   `crates/content/src/audio.rs` and a test beside
   `every_music_file_is_in_the_music_folder` that every cue's `length_ms`
   is within 20 ms of its file. Its failure message prints the right value.
   Document the field in `assets/audio/README.md`, and in
   `assets-src/audio/README.md` that a new or re-cut track needs it.
2. **`app` reports what is sounding.** In `crates/app/src/audio.rs`:
   `Audio::play` takes `now: f64` (seconds). `Track` records
   `started_at: Option<f64>` at the real `backend.play` call: in
   `MusicCommand::Start` when the track is `Ready`, or in `poll` when a
   track asked to play finishes loading. `Audio::music_playing(now) ->
   Option<(&str, f64)>` returns the cue and seconds since start of the
   playing, loaded track that started last (during a fade two tracks exist;
   the incoming one hasn't started yet, so this is the one fading out).
   Use the clock `main.rs` already reads (`miniquad::date::now()`), unless
   macroquad's `get_time()` also keeps counting while a web tab is hidden;
   check both targets and record the choice in the ADR.
3. **`ui` builds the clock.** `crates/ui/src/audio.rs`:
   ```rust
   pub struct MusicClock { pub cue: String, pub position: f32, pub length: f32 }
   impl MusicClock {
       /// `None` if the cue is unknown, or isn't looped and has ended.
       pub fn from_elapsed(manifest: &AudioManifest, cue: &str, elapsed: f64) -> Option<Self>
   }
   ```
   A looped track's `position` is `elapsed % length`. `Game` gets
   `pub fn set_music_playing(&mut self, playing: Option<(&str, f64)>)`,
   which stores the clock in a new `Ctx::music_clock: Option<MusicClock>`
   (default `None`). `crates/app/src/main.rs` calls it before `game.frame`
   each frame. A one-frame lag is fine.
4. **Harness** (`crates/ui/src/harness.rs`): play the part of `app`. When a
   frame's music commands contain `Start { cue }`, start counting from 0
   and add each later frame's `dt`; `Stop` of that cue clears it. Feed it
   through `set_music_playing`. Add `Harness::music_load_delay(seconds)`
   (the clock starts that long after `Start`) and
   `Harness::without_music()` (never starts: a missing file), so screens
   can test both.
5. **ADR**: why `app` reports the position (load time, tab hidden), the
   clock source, that the length is manifest data, and the known limits:
   the system clock and the sound card drift apart by a few milliseconds
   per loop, which is fine for picture timing; and a `play` made on the web
   before the first key press is queued by the browser (ADR-0026 §5), so
   its reported start is early. The title only asks for music after that
   press (`title-screen.md`), so it isn't affected; say so.

## Acceptance criteria

- [x] `audio.ron` has `length_ms` for every music cue; the content test
      fails with the right value if one is wrong.
- [x] `app` unit tests (fake backend, fake time): no report before the
      track has loaded; a report from the moment it starts; none after
      `Stop`; after a switch the report follows the new track once it
      starts.
- [x] `ui` unit tests: `from_elapsed` wraps a looped track at its length,
      returns `None` past the end of a track that isn't looped and for an
      unknown cue.
- [x] Harness test: on the title, `ctx.music_clock` is `None` before the
      music starts, then counts up with `wait`, and wraps at the title
      track's length. With `music_load_delay` it starts late; with
      `without_music` it stays `None`.
- [x] The ADR is written and listed in `docs/adr/README.md`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `ogg_length_ms`; manifest validation; `MusicClock::from_elapsed`;
  `app` audio with the fake backend.
- Property: `from_elapsed` position is always in `0..length` for a looped
  track.
- Snapshot / integration: the Harness tests above.

## Completion notes

**What was done**

- `assets/audio/audio.ron`: every music cue has `length_ms` (25 tracks,
  from each file's last Ogg page; the title is 133,743 ms, as the ticket
  said). `MusicCue::length_ms`; the validator refuses 0.
- `trpg_content::audio::ogg_length_ms(bytes)`: last page's granule position
  over the header's sample rate, to the nearest millisecond.
- `app`: `Audio::play(.., now)` records `started_at` at the real
  `backend.play` call; `Audio::music_playing(now)` reports the cue and the
  seconds since. `main.rs` passes `miniquad::date::now()` and calls
  `game.set_music_playing(..)` before `game.frame`.
- `ui`: `MusicClock { cue, position, length }` with `from_elapsed`,
  `Ctx::music_clock`, `Game::set_music_playing`.
- `Harness`: a small simulated player follows the music commands;
  `music_load_delay(seconds)`, `without_music()`, and `music_clock()` to
  read the result.
- ADR-0036 (amends ADR-0026); READMEs for the manifest, the audio sources,
  `music/` and `crates/ui`.

**Deviations**

- The length check is in the manifest validator, not in a separate test:
  wherever the music files can be read (the existing
  `every_music_file_is_in_the_music_folder` test), a `length_ms` more than
  20 ms off its file is an error such as `music "title":
  music/new_sunrise_v1.ogg: it plays for 133743 ms, but length_ms is
  120000; write length_ms: 133743`. This way the message itself is unit
  tested. Checked by hand too: setting the title to 120000 fails that test
  with exactly this line.
- `ogg_length_ms` divides by the sample rate in the file's header rather
  than a fixed 44,100 (the same number for every file we ship).
- Clock: `miniquad::date::now()`. macroquad's `get_time()` is the same
  clock minus the launch time on both targets (read from their sources),
  so there was nothing to choose between; the ADR has the table.
- The Harness also ends the count on a `Load` of the sounding track (as
  `app` replaces a track loaded again), and has a `music_clock()` reader
  the ticket didn't list.

**Not verified**

- Nothing reads the clock on screen yet, so it was not watched against real
  sound, natively or in a browser. The `app` logic is unit tested with the
  fake backend and fake time; the cinematic tickets (0817, 0819) are where it
  meets real audio.
- The ADR's note that a hidden tab keeps playing is from how Web Audio is
  specified, not from a test on a device.

**Follow-up tickets:** none.

**Gameplay rules decided by Claude:** none. Nothing a player sees or hears
changes in this ticket.

**For Nick:** nothing to try yet. This is the plumbing the title cinematic
needs to stay in step with the song.
