---
id: "0238"
title: "Voice clips: the manifest, the voice folder, and playing a line's clip"
type: feature
milestone: M1 Engine
model: opus-5.5
effort: high
status: todo
blocked_by: ["0717"]
nick_input: none
completed:
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

- [ ] A Harness test: `play_voice` for a line with a clip queues `PlayVoice`; for a line without one, a stale one, or with voices off, it queues nothing.
- [ ] `app`'s recording backend test: `PlayVoice` then `PlayVoice` stops the first; `StopVoice` stops it; volume follows `voice_volume`.
- [ ] Renaming a name used in a clip's line makes that clip stale (unit test).
- [ ] With no `voice/` folder the game starts and logs nothing about voices (test).
- [ ] The web build fetches and plays a test clip (checked in the browser; say how in the notes).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: manifest validation; `spoken_text`; staleness.
- Integration: Harness requests; `app` backend with the fake.

## Completion notes

