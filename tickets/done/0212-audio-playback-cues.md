---
id: "0212"
title: "Audio playback: sound and music cues from screens, played by app"
type: infra
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-09-28
---

# 0212 — Audio playback: sound and music cues

## Context

Nick chose the game's music and sounds in ticket 0020
([`docs/design/audio.md`](../../docs/design/audio.md)). Nothing plays audio
yet. This ticket builds the plumbing every audio ticket after it uses: a data
manifest of cues, a way for `ui` screens to ask for a sound or a music track,
and `app` actually playing them.

Constraints:

- ADR-0004: only `app` touches macroquad, files and the clock. `ui` screens
  must stay testable in the headless `Harness`, so they *emit requests as
  data*. ADR-0017 (screen stack, `FrameOutput`) is the pattern to extend.
- macroquad 0.4's `audio` feature (quad-snd 0.2, MIT/Apache) decodes WAV and
  OGG Vorbis on Windows (WASAPI) and uses Web Audio in the browser. It has
  play, stop, volume and looping only: no streaming, seek or crossfade. On
  web, the audio context unlocks on the first keypress.
- Assets are embedded with `include_dir!` over the whole `assets/` folder
  (`crates/content/src/bundle.rs`). The chosen music is about 20 orchestral
  tracks (several minutes each). Embedding it all would bloat the Windows exe
  and, worse, the WASM download. A decoded 3-minute stereo track is about
  60 MB of memory on web.

## Nick input

None.

## Scope

**In:**
- An audio manifest `assets/audio/audio.ron` and its loader and validator
  in `trpg-content`: sound cues (one or more variant files, a volume), music
  cues (a file, looped, a volume), music pools (a list of music cues), and
  credits (title, author, source URL, license, tags, and a note such as
  "V2 file"). Every sound and music entry names its credit, or `Own` for
  sounds we make.
- `trpg-ui::audio`: the request type (e.g. `AudioRequest::{PlaySound { cue,
  volume }, PlayMusic { cue }, StopMusic}`), a queue on `Ctx` that screens
  push to, and the queue drained into `FrameOutput` every frame.
- A pure, unit-tested music state machine in `ui`. It tracks the current cue;
  asking for the same cue again doesn't restart it; switching fades the old
  track out over 0.5 s (*tunable*, `audio.md`) and then starts the new one.
  It outputs volume and start/stop commands per frame for `app` to execute.
- `trpg-app::audio`: enable macroquad's `audio` feature and execute the
  requests. It loads sounds and plays them. For a cue with several variants
  it picks one at random (footsteps, `audio.md`), using a small non-simulation
  RNG in `app` (never core's simulation RNG, ADR-0019). It plays music looped
  and applies the fades.
- **Where music files live** (decide it, then record it in the ADR). Sound
  effects are small and may stay embedded. Music must not bloat the exe or
  WASM. Suggested:
  - Keep music out of `include_dir!` in its own top-level folder shipped next
    to the exe (and served next to the WASM on the web build).
  - Load it with macroquad's `load_file` when first played, and unload the
    previous track.
  - Update `cargo xtask web`, `release.yml` and the Pages build to ship that
    folder.
- `Harness` records every `AudioRequest` so screen tests can assert on it.
- ADR-0026 (`write-adr` skill): audio cues as data, where the files live,
  formats (OGG Vorbis for recordings, WAV allowed for our own short sounds),
  the web unlock behaviour, and the memory budget.

**Out (do not do):**
- Any real audio content (0213 makes ours, 0214 imports the third-party
  files) or wiring cues into screens (0424, 0425, 0710, 0807).
- Volume settings UI (0805).
- Credits screen (0808).

## Implementation steps

1. Add `features = ["audio"]` to macroquad in the workspace `Cargo.toml`.
   Run `cargo deny check` and check that quad-snd's decoders (audrey, lewton,
   hound, claxon…) pass the license list. Build the WASM (`cargo xtask web`)
   and a Windows release build, and note the size change.
2. `trpg-content`: `audio.rs` with the manifest schema (serde), `load` and
   `validate`. Checks: unique ids; every referenced file exists (embedded
   ones in the bundle, music in its folder at build/test time); pools only
   name music cues; every credit's license is `CC0-1.0`, `CC-BY-4.0` or
   `Own` (ADR-0013); every credit has title, author and source URL (except
   `Own`). Document the format in `assets/audio/README.md`. Start with an
   empty manifest, which must be valid.
3. `trpg-ui`: `audio.rs` with `AudioRequest`, the `Ctx` queue
   (`ctx.audio.play_sound("menu_move")`, `.play_music("title")`, …, with a
   debug assertion or a validation-time check that the cue exists), the music
   state machine, and the `FrameOutput` field.
4. `Harness`: expose the requests of the last frame and the whole run (for
   example `harness.audio_requests()`).
5. `trpg-app`: `audio.rs` executes the requests. On web, check what quad-snd
   does with a `play` before the first keypress: does it queue or drop?
   Document it in the ADR. The title music must start at the latest on the
   first keypress.
6. Ship the music folder with every build target (native release zip,
   `cargo xtask web`, Pages).
7. ADR-0026, plus a row in `docs/adr/README.md`.

## Acceptance criteria

- [x] `cargo deny check` passes with the `audio` feature on.
- [x] Manifest validation unit tests: a duplicate id, a missing file, an
      unknown license and a pool naming a sound each fail with a clear
      message; the empty manifest passes.
- [x] Music state machine unit tests: same cue doesn't restart; switch fades
      then starts; `StopMusic` fades out; a new request during a fade wins.
- [x] A Harness test shows a screen's request arriving in `FrameOutput` and
      in `audio_requests()`.
- [x] A test-only WAV (generated in the test, not shipped) plays through the
      `app` path without panicking (at least a smoke run of `app::audio`
      logic that doesn't need a sound device, if the device part can't run
      in CI).
- [x] The web build and the release package include the music folder (even
      if empty) and still start.
- [x] ADR-0026 written and indexed.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: manifest validation; music state machine; variant picking gives each
  variant a chance (property test over seeds).
- Snapshot / integration: Harness request recording.

## Completion notes

**Done.** The audio plumbing is in; nothing plays yet because the manifest
is empty until 0213/0214 add files and 0424/0425/0710/0807 wire cues into
screens. Decisions are in ADR-0026.

- `assets/audio/audio.ron` (empty, valid) + `trpg_content::audio`: schema,
  loader and validator, documented in `assets/audio/README.md`. Loaded into
  `Content::audio`. Volumes are whole percents (keeps `Content` `Eq`); the
  fade length is data too (`music_fade_ms: 500`).
- `trpg_ui::audio`: `AudioRequest`, `AudioQueue` on `Ctx` (`ctx.audio`),
  the pure `MusicState` and its `MusicCommand`s (`Load`, `Start`, `Gain`,
  `Stop`). `FrameOutput` gained `audio` and `music`. An unknown cue panics
  in debug builds. `Harness`: `audio_requests()`, `last_frame_audio()`,
  `music_commands()`, `clear_audio()`.
- `trpg_app::audio`: `Audio` executes requests through a `Backend` trait
  (macroquad in the game, a recording fake in tests). Sounds are decoded at
  start-up; music loads with a macroquad coroutine when the fade starts and
  is freed when it stops. Variant picking uses a `SplitMix64` in `app`,
  seeded from the clock (not core's RNG).
- Music lives in a top-level `music/` folder, not embedded. `cargo xtask
  web` copies `music/*.ogg` to `dist/web/music/` (so CI, Pages and the web
  release package get it); `release.yml` copies it into the Windows, Linux
  and macOS packages.
- **Deviation (extra safety):** native quad-snd `unwrap`s the decode (a bad
  file crashes the game), asserts mono/stereo, and resamples anything not at
  44.1 kHz nearest-neighbour; on the web a file that fails to decode never
  finishes loading. So the validator also reads each file's header: the
  content must match the extension (OGG Vorbis / PCM or float WAV), with 1–2
  channels at 44.1 kHz. I added the 44.1 kHz rule, the license texts in the
  release packages and a hitch measurement to ticket 0214's scope.
- **Checked by hand (not in CI):** a temporary build played a generated WAV
  and loaded a "track" through the real macroquad backend on Windows
  (WASAPI) and in the browser; a missing track came back as an error on
  both (no hang, no crash). In the browser, decoding and loading worked
  before any keypress. quad-snd's `audio.js` shows a `play` before the first
  keypress is queued (it starts when the context unlocks), so title music
  starts on the first keypress at the latest. That temporary code isn't
  committed.
- **Size:** Windows exe (local GNU release) 2,906,624 → 3,292,160 bytes
  (+377 KiB); WASM (release, before `wasm-opt`) 2,351,300 → 2,489,590 bytes
  (+135 KiB).
- **Acceptance criteria:** all met, except that the release packages'
  `music/` step is proven only by a `release.yml` dry run (see the PR); the
  web build was run locally.
- No new follow-up tickets. The two console errors on the web page
  (`register_plugin`, quad_storage version) are the known ones from
  0206/0207.

**Claude's starting rules** (the design docs don't cover these; Nick can
veto any of them):

1. When the music is switching and the game asks for the old track again
   during the 0.5 s fade, the old track snaps back to full volume
   and keeps playing (no restart, no fade back in).
2. If a third track is asked for during a fade, the fade carries on from
   where it was (it doesn't restart), then the newest track starts.
3. A new track starts at full volume straight away (no fade-in), as
   `audio.md` describes.
4. Music loops unless a cue says otherwise (for future stings).
5. On the web, if the next track hasn't finished downloading when the fade
   ends, there is silence until it has.
6. When the player quits from the title screen on the web page, the music
   stops.
