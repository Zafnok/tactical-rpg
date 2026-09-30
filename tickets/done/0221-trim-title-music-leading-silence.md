---
id: "0221"
title: Trim the silence before the title music's fade-in
type: bug
milestone: M1 Engine
model: sonnet-5
effort: low
status: done
blocked_by: ["0214"]
nick_input: sign-off
completed: 2026-09-29
---

# 0221 — Trim the silence before the title music's fade-in

## Context

Nick after playing: the title music has about 3 s of silence before a faint
fade-in, so the game seems silent at first and makes a bad first impression.
He is fine with the fade-in itself.

The silence is in the original recording ("New Sunrise" V1 by nene, imported
by ticket 0214 with `assets-src/audio/import.py`, ADR-0027): it stays below
−60 dB for its first ~3 s, then fades in over the next few seconds. The game
adds no delay of its own.

## Nick input

**Sign-off:** open the game (Pages build after merge) and listen to the title
screen: the music should start fading in straight away.

## Scope

**In:**
- Cut the first 3.0 s of `music/new_sunrise_v1.ogg` in `import.py`, keeping
  the fade-in, and regenerate only that file.
- Let `import.py` redo just the named outputs, so the other files aren't
  re-encoded for nothing.

**Out (do not do):**
- Any other track, the V2 file (`city_first_visit`), or the game's music code.

## Implementation steps

1. `import.py`: a `MUSIC_CUTS` table of extra ffmpeg filters per music output;
   `new_sunrise_v1.ogg` gets `atrim=start=3.0,asetpts=PTS-STARTPTS,afade=t=in:d=0.05`
   (the short fade avoids a click at the cut). Applied before loudness matching.
2. `import.py`: output names on the command line limit the download and the
   conversion to those outputs.
3. Run `python assets-src/audio/import.py new_sunrise_v1.ogg`.
4. Document both in `assets-src/audio/README.md`.

## Acceptance criteria

- [x] `music/new_sunrise_v1.ogg` starts at the fade-in (first quarter-second
  about −61 dB RMS, was about −106 dB; −50 dB reached at ~2.2 s, was ~5.2 s).
- [x] Its loudness match is unchanged (gain −1.0 dB, integrated −19.0 LUFS
  measured on the source).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None new: `cargo test -p trpg-content` already checks the music file's
  format, sample rate and channels.

## Completion notes

Cut 3.0 s, where the original first reaches about −60 dB, matching what Nick
heard as silence. The fade-in that follows is untouched. If the start still
feels too quiet, raise the `atrim` start in `MUSIC_CUTS` (e.g. 4.0 s starts
at about −57 dB) and rerun the command in step 3.
