---
id: "0222"
title: Make every sound effect quieter by default
type: tuning
milestone: M2 Engine
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-09-30
---

# 0222 — Make every sound effect quieter by default

## Context

Nick, after playing: "all SFX are way too loud as default make them
significantly quieter". Every sound cue in `assets/audio/audio.ron` plays at
volume 100 (the loudness-matched level from 0214), except `cursor_move` at 60
(`docs/design/audio.md` "Starting values"). Music is fine and stays as is.

A player-facing sound volume setting comes later with 0805; this ticket only
changes the cues' own starting volume, which 0805's setting multiplies.

## Nick input

**Sign-off:** after merge, play on Pages and say if sounds are now quiet
enough next to the music (too quiet / right / still too loud).

## Scope

**In:**
- Lower every entry in `sounds:` of `assets/audio/audio.ron` to 40 % of its
  current volume (100 → 40, `cursor_move` 60 → 24), about −8 dB. Keeps the
  cursor tick at 60 % of `menu_move`.
- Update the manifest header comment and `docs/design/audio.md` "Starting
  values".

**Out (do not do):**
- Music volumes, the import script's loudness matching, the options menu
  (0805), code changes.

## Implementation steps

1. In `assets/audio/audio.ron`, set each sound cue's `volume` as above.
2. Fix the header comment ("every cue starts at volume 100") to say sounds
   start at 40.
3. In `docs/design/audio.md` "Starting values", add a bullet: sound effects
   play at 40 % of their matched loudness (Nick: too loud at 100), the cursor
   tick at 24 %.

## Acceptance criteria

- [x] Every `sounds:` cue in `audio.ron` has volume 40, `cursor_move` 24;
      music unchanged.
- [x] `cargo xtask` content validation passes (manifest still loads).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none new (data-only tuning; the manifest loader test covers it).

## Completion notes

Set all 21 sound cues in `assets/audio/audio.ron` to 40 (`cursor_move` 24);
the 25 music cues stay at 100. Header comment and `docs/design/audio.md`
"Starting values" updated. No code changes; no deviations.

*Claude's starting rule:* 40 % (about 8 dB quieter) is my pick for
"significantly quieter". Nick: if it's now too quiet or still too loud, say
so and the number gets nudged. A player volume slider comes with 0805.
