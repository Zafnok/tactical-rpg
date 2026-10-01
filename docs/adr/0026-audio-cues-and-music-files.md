# ADR-0026: Audio cues as data, played by `app`; music files beside the game

- **Status:** Accepted; allowed licenses and the `credit` field amended by ADR-0027; native music loading (§3, §6) amended by ADR-0028; track lengths and the music clock added by ADR-0036
- **Date:** 2026-09-28
- **Related tickets:** 0212 (this plumbing), 0213, 0214, 0424, 0425, 0710, 0805, 0807, 0808

## Context

Nick chose the game's music and sounds in ticket 0020
([`docs/design/audio.md`](../design/audio.md)): about 20 recorded orchestral
tracks (several minutes each), about 17 recorded sounds, a few sounds we make
ourselves, a 0.5 s fade between tracks, random footstep variants, music
pools, and a credit for every work.

Forces:

- ADR-0004: only `app` touches macroquad, files and the clock. Screens in
  `ui` must stay testable in the headless `Harness` (ADR-0017), so they can
  only *ask* for audio.
- macroquad 0.4's `audio` feature (quad-snd 0.2) decodes WAV and OGG Vorbis:
  WASAPI on Windows, ALSA on Linux, CoreAudio on macOS, Web Audio in the
  browser. It offers play, stop, per-sound volume and looping, and nothing
  else: no streaming, seeking or crossfades. A sound is decoded whole into
  memory.
- Everything under `assets/` is embedded with `include_dir!`. Embedding the
  music would add tens of MB to the exe and, worse, to the WASM download
  that must finish before the web build shows anything.
- Web browsers keep audio locked until the player presses a key or clicks.

## Decision

### 1. Cues are data: `assets/audio/audio.ron`

One manifest, loaded and validated by `trpg_content::audio` into
`Content::audio` (format: [`assets/audio/README.md`](../../assets/audio/README.md)):

- `music_fade_ms`: the fade length (`audio.md`: 500, *tunable*).
- `sounds`: id, one or more variant `files` (relative to `assets/audio/`),
  `volume` (percent), `credit`.
- `music`: id, `file` (relative to `music/`), `volume`, `looped` (default
  true), `credit`.
- `pools`: id and the music cues one is picked from.
- `credits`: title, author, source URL, license, tags, note. The credits
  screen (0808) reads these.

`credit` is `Own` or `Credit("<id>")`. Volumes are whole percents so
`Content` stays `Eq`. Validation refuses: an empty or duplicate id (sounds,
music and pools share one namespace; credits have their own), a missing
sound file, a wrong format (see 4), a volume over 100, a license other than
`CC0-1.0`, `CC-BY-4.0` or `Own` (ADR-0013), a non-`Own` credit without
title, author or web link, an unknown credit, and a pool that is empty or
names anything but a music cue. Music files can't be checked at run time
(content does no file I/O), so a unit test checks them against the repo's
`music/` folder.

Picking a track from a pool is 0807's job (the battle's music is chosen
once per battle and must survive rewinds); nothing here picks from pools.

### 2. Screens request; `ui` decides the music; `app` plays

- `trpg_ui::audio::AudioRequest`: `PlaySound { cue, volume }`,
  `PlayMusic { cue }`, `StopMusic`. Screens push them with
  `ctx.audio.play_sound("menu_move")`, `play_sound_at(cue, v)`,
  `play_music(cue)`, `stop_music()`.
- `Game` drains the queue every frame. In debug builds, a cue the manifest
  lacks (or of the wrong kind) panics, so typos fail tests.
- `MusicState` (pure, in `ui`) holds the current track and the fade:
  asking for the playing (or incoming) cue does nothing; a switch loads
  the new track at once, fades the old one out over `music_fade_ms`, then
  stops it and starts the new one at full volume; `StopMusic` fades to
  silence; a newer request during a fade replaces the waiting track and
  keeps the fade's progress; asking again for the track that is fading out
  cancels the fade. It emits `MusicCommand`s: `Load`, `Start`, `Gain`
  (fade multiplier) and `Stop` (which also frees the track).
- `FrameOutput` gains `audio: &[AudioRequest]` (everything asked this
  frame) and `music: &[MusicCommand]`. The `Harness` records both
  (`audio_requests()`, `last_frame_audio()`, `music_commands()`).
- `trpg_app::audio::Audio` executes them through a small `Backend` trait
  (macroquad in the game, a recording fake in tests). A sound's volume is
  `cue volume × request volume`; a track's is `cue volume × fade gain`.
  Sound variants are picked by a `SplitMix64` in `app`, seeded from the
  clock, separate from core's simulation RNG (ADR-0019): what plays never
  affects the game. Failures (a missing track, a file that won't decode)
  are logged as warnings and play as silence, never a crash. When the game
  quits (the web page stays open), all music stops.

### 3. Where the files live

| What | Where in the repo | How it ships |
| ---- | ----------------- | ------------ |
| Sound effects | `assets/audio/` | Embedded (small); decoded once at start-up |
| Music | `music/` at the repo root | Not embedded. A `music/` folder next to the exe; on the web, `music/` next to `index.html` |

- Native: the game looks for `music/` beside the executable, then falls back
  to the working directory (`cargo run` from the repo root).
- Web: tracks are fetched over HTTP from `music/` relative to the page.
- A track is loaded when `MusicState` emits `Load` (at the start of the
  fade, so the fetch and decode overlap the old track fading out) with a
  macroquad coroutine, and freed on `Stop`. A load cancelled before it
  finished is polled until done and then dropped, so its decoded data
  doesn't linger.
- Shipping: `cargo xtask web` copies `music/*.ogg` into `dist/web/music/`
  (creating the folder even when empty); that covers CI's `wasm` job, the
  Pages build and `release.yml`'s web package. `release.yml`'s Windows,
  Linux and macOS packages copy `music/*.ogg` into `<package>/music/`.
- Tracks are committed as plain git files (no Git LFS: every CI checkout
  would spend the LFS bandwidth quota). A track is a few MB of OGG.

### 4. Formats

**OGG Vorbis** for anything recorded: all music, and third-party sounds.
**WAV** (PCM or float) only for our own short sounds (0213). Every file is
**44.1 kHz, mono or stereo**. The validator enforces all of it, reading each
file's header (music must be `.ogg`; sounds `.ogg` or `.wav`; the content
must match the extension). The reason: on native, quad-snd plays at 44.1 kHz
and resamples anything else nearest-neighbour, asserts one or two channels,
and `unwrap`s the decode, so an unreadable file would crash the game; on
the web a file that fails to decode never finishes loading, which would hang
start-up for an embedded sound. The header check catches the likely
mistakes (wrong format, rate or channels) before a file ships; a file that
is corrupt past its header is still possible, and 0214's import converts
every file with a known tool.

### 5. Web: before the first keypress

Read from quad-snd's `audio.js` (bundled in `web/mq_js_bundle.js`) and
checked in the browser for this ticket:

- The `AudioContext` is created at start-up; the browser's autoplay policy
  starts it *suspended*. `audio.js` resumes it on the first `keydown`,
  `mousedown` or touch anywhere on the page.
- Decoding works while suspended: a test sound decoded and a test track was
  fetched and decoded before any key was pressed.
- A `play` while suspended is **queued, not dropped**: `audio.js` calls
  `source.start(0)`, and a suspended context's clock doesn't move, so the
  sound starts from the beginning when the context resumes. Title music
  asked for at start-up therefore begins, at the latest, on the first
  keypress. One-shot sounds asked for before that key would all sound at
  once on it, but screens play sounds only in response to input, so none
  can be waiting.

### 6. Memory budget

A decoded track is 32-bit float PCM: about 21 MB per minute of 44.1 kHz
stereo (a 3-minute track ≈ 64 MB), on the web and on native. At most two
tracks are in memory (the one fading out and the next), so about 130 MB at
peak for 3-minute tracks, plus the decoded sounds (a few MB). 0214 should
keep tracks at or under about 4 minutes (cut long pieces to their loop).

On native, quad-snd decodes a whole OGG synchronously when the load
coroutine runs, which can cost one long frame when a track starts loading
(it lands at the start of a fade, while the old track still plays). 0214
measures it with the real files.

### 7. Size

Turning on the `audio` feature and this code: Windows exe (local GNU
release build) 2,906,624 → 3,292,160 bytes (+377 KiB); WASM (release, before
`wasm-opt`) 2,351,300 → 2,489,590 bytes (+135 KiB). New crates: quad-snd,
audrey, lewton, ogg, hound, dasp, smallvec 0.6, maybe-uninit,
quad-alsa-sys, audir-sles: all MIT/Apache-2.0 or BSD-3-Clause, and
`cargo deny check` passes.

## Consequences

- Screens stay pure: a Harness test asserts on the cues a screen asked for.
- Adding a cue is data only: an entry in `audio.ron`, a file, and a credit.
- The game needs its `music/` folder beside it. itch.io and Steam packaging
  (09xx) must ship it, as the release workflow does.
- 0214 must ship the audio license texts (CC0, CC-BY) in the release
  packages too; `release.yml` copies only `assets/fonts/*LICENSE*` today.
- No crossfades, streaming or seeking while we stay on quad-snd. A master or
  music volume setting (0805) multiplies into `Audio`'s volumes.
- On the web, a track that doesn't exist yields a 404 and a warning in the
  console; the game keeps running silently.

## Alternatives considered

- **Embed music with the rest of `assets/`** — simplest, but tens of MB
  more in the exe and in the WASM the player must download before the title
  screen appears, and every track decoded into memory up front.
- **A dedicated audio crate (kira, rodio)** — streaming and real crossfades,
  but a second audio stack beside macroquad's, weak or wasm-bindgen-based web
  support that clashes with miniquad's JS loader, and more binary size. Not
  worth it for loops and one-shots.
- **The music state machine in `app`** — `app` isn't testable headlessly;
  the fade and "don't restart" rules belong where the Harness can see them.
- **Float volumes in the manifest** — would make `Content` lose `Eq`;
  percents read naturally for tuning anyway.
- **Load a track only when its fade ends** — halves peak memory, but on the
  web the fetch and decode would leave a gap of silence at every switch.
- **Git LFS for music** — the free quota's bandwidth would be eaten by CI
  checkouts; plain files of a few MB each are fine.
