---
id: "0238"
title: "Voice clips: the manifest, the voice folder, and playing a line's clip"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: done
blocked_by: ["0717"]
nick_input: none
completed: 2026-10-03
---

# 0238 — Voice clips: the manifest, the voice folder, and playing a line's clip

## Context

Nick wants AI-generated voices now and a real cast later
(`docs/design/voices-languages-and-script.md`).
[ADR-0046](../../docs/adr/0046-voice-clips-by-line-id.md): a clip is a
file keyed by a dialogue line id (0717), in a `voice/` folder beside the
game, listed in a manifest, asked for by screens and played by `app`,
like music (ADR-0026). This ticket is the plumbing, tested with a few
clips we make ourselves; it doesn't depend on Nick's choices in 0043.

## Nick input

None. No screen plays a voice yet, so nothing changes for a player.

## Scope

**In:** the manifest types and validation; `AudioRequest::PlayVoice` /
`StopVoice`; loading and playing in `app` (native and web); a voice
volume and an on/off flag in `Ctx`; shipping the folder; test clips.

**Out (do not do):** the dialogue screen asking for voices (0720); the
Options rows and saving the settings (0826); generating clips (0721);
real clips in this repository (they go in the private assets repository,
ADR-0046 rule 5); ducking the music under a voice (a tuning ticket if
Nick asks).

## Implementation steps

1. `crates/content/src/voice.rs`: `VoiceManifest { clips: [VoiceClip] }`,
   `VoiceClip { line: LineId, variant: None | M | F, spoken: String,
   voice: String, made_by: Generated { tool, model, date } |
   Recorded { actor } }`, and `Cast { voices: character id → voice id }`.
   Parsing and validation are pure functions over text (`content` does no
   file I/O): a clip's line id must exist; no duplicate (line, variant);
   `M` and `F` come as a pair. A function
   `spoken_text(line, names, lead gender) -> String` gives the words a
   line says with name tokens filled; a clip is **stale** when its
   `spoken` differs, and stale clips are left out of the playable set.
   Lines containing `{lead}` have no spoken text (`None`).
2. Where the manifest is read: it is not embedded. `app` reads
   `voice/<lang>/voice.ron` at start-up (beside the exe, then the working
   directory; over HTTP on the web) and hands the text to `Ctx`
   (`Ctx::set_voice_manifest`), which validates it and keeps the playable
   set. A missing file means no voices and no warning; an invalid one is
   a logged warning and no voices.
3. `crates/ui/src/audio.rs`: `AudioRequest::PlayVoice { line: LineId,
   variant }` and `StopVoice`; `AudioQueue::play_voice(&LineId, gender)`
   (does nothing when voices are off or the line has no playable clip)
   and `stop_voice()`. `Ctx` gets `voices_on: bool` (default true) and
   `voice_volume` (0–10, default 8, *tunable*), until 0826 moves them
   into `Settings`. The Harness records the requests like sounds.
4. `crates/app/src/audio`: load a clip when asked (the same loader as
   music, ADR-0028's worker thread on native), play it once at
   `voice_volume`, free it when it ends or is stopped. One voice at a
   time: `PlayVoice` stops the one playing. A clip that isn't loaded
   within 300 ms (*tunable*) of the request is dropped rather than played
   late. Failures are warnings and silence, never a crash.
5. Preloading: `AudioRequest::PreloadVoices { lines }`, which a screen
   may send when a scene starts; `app` loads up to the next 3 clips
   ahead (*tunable*) and frees those passed. Measure memory with 20
   clips and write it in the completion notes.
6. Shipping: `cargo xtask web` copies `voice/` into `dist/web/voice/` if
   it exists; `release.yml`'s packages copy it too. `cargo xtask
   private-assets` checks `game/voice/` out to `voice/` at the repo root
   (git-ignored; add it to `.gitignore`) when the private repository has
   one. Gates never read it.
7. Test clips: three short WAV-to-OGG clips made with our own sound tool
   (`crates/xtask/src/sfx.rs`, tones, not speech) under
   `crates/app/tests/voice/` with a manifest for lines of
   `assets/dialogue/test.dlg`.
8. Document the folder and manifest in a new `voice/README.md`-style
   file kept at `docs/voice.md` (the folder itself is git-ignored).
   Set ADR-0046's status to `Accepted`; record any change.

## Acceptance criteria

- [x] A Harness test: `play_voice` for a line with a clip queues `PlayVoice`; for a line without one, a stale one, or with voices off, it queues nothing.
- [x] `app`'s recording backend test: `PlayVoice` then `PlayVoice` stops the first; `StopVoice` stops it; volume follows `voice_volume`.
- [x] Renaming a name used in a clip's line makes that clip stale (unit test).
- [x] With no `voice/` folder the game starts and logs nothing about voices (test).
- [x] The web build fetches and plays a test clip (checked in the browser; say how in the notes).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: manifest validation; `spoken_text`; staleness.
- Integration: Harness requests; `app` backend with the fake.

## Completion notes

Done as planned, with the deviations below. No screen plays a voice yet, so
nothing changes for a player; no gameplay rule was decided here.

**What exists now**

- `trpg_content::voice`: the manifest and cast types, `from_source` /
  `cast_from_source` (pure, over text), `spoken_text`, staleness and the
  `Playable` set, `clip_file`. `LineId::new` and `LineId::scene_id`.
- `ui`: `AudioRequest::PlayVoice { line, variant }`, `StopVoice`,
  `PreloadVoices { lines }`; `Ctx::set_voice_manifest`, `play_voice`,
  `stop_voice`, `preload_voices`, `has_voice`, `voices_on` (true),
  `voice_volume` (0–10, default 8, *tunable*); `Harness::voices()`.
- `app`: `audio/voice.rs`, the player: loads on the music loader, one voice
  at a time, 3 ahead (*tunable*), 0.3 s lateness limit (*tunable*), frees a
  clip when it ends (length read from the OGG), is stopped or is passed.
- Shipping: `cargo xtask web`, `release.yml`, `cargo xtask private-assets`
  and the CI private-assets action; `/voice/` is git-ignored.
- Test clips: `crates/app/tests/voice/en/` (manifest, cast, three tone
  clips for `test.dlg`), made by the new `cargo xtask voice-test-clips`.
- `docs/voice.md`; ADR-0046 is `Accepted`, with an "As built" section.

**Deviations from the steps**

1. Step 3: screens call `Ctx::play_voice(&line)`, not
   `AudioQueue::play_voice(&LineId, gender)`. The queue is a field of `Ctx`
   and can't see the playable set, the on/off flag or the lead's gender;
   `Ctx` can. `AudioQueue::play_voice(line, variant)` is the raw request.
2. Step 6: the private repository keeps voices in its top-level `voice/`,
   not `game/voice/`. Everything under `game/` is embedded in the binary
   (`include_dir!`), which ADR-0046 rule 4 forbids for voices. `cargo xtask
   private-assets` (and the CI action) check `voice/` out and copy it to
   `voice/` at this repository's root. Ticket 0721's one mention of
   `game/voice/` is corrected.
3. The voice volume travels in `FrameOutput::voice_volume` rather than in
   each request, so a change reaches the clip that is playing (0826's
   slider will want that).
4. A debug-menu tool, **Voice test**, says the test scene's voiced lines
   one per press. It is the only way to hear a clip until 0720, and the
   browser check below needs it. It does nothing without a `voice/` folder.
5. Step 7: the tones come from `sfx.rs`, but the OGG encoding is `ffmpeg`
   (as for the music): there is no permissively licensed pure-Rust Vorbis
   encoder worth a dependency for three files. `cargo xtask
   voice-test-clips` runs both; a test checks the committed clips against
   what it would make.
6. Validation also refuses a line with both a plain clip and an M/F pair,
   and an empty `spoken` or `voice`. A plain clip on a gendered line, and
   any clip on a `{lead}` line, are stale (silent), not errors.
7. `cast.ron` is parsed and validated but the game doesn't read it (0721's
   tool does).

**Memory with 20 clips (step 5)**

Measured on Nick's machine with the ignored test `voice_memory_with_20_clips`
(`crates/app/src/audio/native_music.rs`), Windows working set before and
after loading 20 clips:

- 20 clips of 4.0 s (a typical line): 14.2 MB → 45.6 MB, **31 MB**, about
  1.6 MB a clip (quad-snd decodes to 44.1 kHz stereo floats, 0.35 MB per
  second, plus its context).
- 20 of the 0.42 s test clips: 12.8 MB → 20.1 MB, 7 MB.

The player never holds 20: at most 4 (the one playing or asked for, and 3
ahead), so about 6 MB for 4 s lines.

**The browser check (acceptance 5)**

`cp -r crates/app/tests/voice voice`, `cargo xtask web`, served `dist/web`
with `python -m http.server` and opened it in the built-in browser pane.
Network log: `voice/en/voice.ron` 200 at start-up, no console warning. At
the title: debug key, Up twice to "Voice test", Confirm. The three `.ogg`
files were fetched (the preload), and a hook on
`AudioBufferSourceNode.start` showed a 0.415 s buffer started 47 ms after
the menu sound: the first clip (418 ms). Two more presses started the
second (0.415 s) and third (0.414 s) clips.

**Worth knowing**

- On native each loaded clip has its own quad-snd context (ADR-0028), as
  music tracks do; contexts are pooled, so voices add at most 4.
- On the web a build without voices asks for `voice/en/voice.ron` once and
  gets a 404. The game logs nothing, but the browser's own network log
  shows it. A failed fetch and a missing file look the same there.
- A voice asked for without preloading is dropped if its file takes more
  than 0.3 s to arrive (likely on a slow connection): 0720 should call
  `ctx.preload_voices` when a scene starts.

**Follow-up tickets:** none new (0720, 0721, 0826 already cover the rest).
