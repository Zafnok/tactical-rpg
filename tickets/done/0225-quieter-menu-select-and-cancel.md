---
id: "0225"
title: Make the menu select and cancel sounds quieter
type: tuning
milestone: M2 Engine
model: sonnet-5
effort: low
status: done
blocked_by: ["0222"]
nick_input: sign-off
completed: 2026-09-30
---

# 0225 — Make the menu select and cancel sounds quieter

## Context

After 0222 lowered every sound effect to volume 40, Nick played again:
"select/confirm and cancel still sound much too high rest sound better".
The cues are `menu_select` and `menu_cancel` in `assets/audio/audio.ron`
(in-house sounds from 0213). Decisions: `docs/design/audio.md`.

## Nick input

**Sign-off:** after merge, play on Pages and say if confirm/cancel now sit
right next to the menu move tick and the battle sounds.

## Scope

**In:**
- `menu_select` and `menu_cancel`: volume 40 → 15 (about 8.5 dB quieter).
- Record it in `docs/design/audio.md` "Starting values".

**Out (do not do):**
- Other cues, music, re-rendering the sounds (`cargo xtask sfx`), pitch.

## Implementation steps

1. In `assets/audio/audio.ron`, set `menu_select` and `menu_cancel` to
   `volume: 15`.
2. Add a bullet to `docs/design/audio.md` "Starting values".

## Acceptance criteria

- [x] `menu_select` and `menu_cancel` are at 15; every other cue unchanged.
- [x] Manifest still loads and validates (`cargo test --workspace`).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none new (data-only tuning; the manifest loader tests cover it).

## Completion notes

Set both cues to 15, as planned. No code changes.

*Claude's starting rule:* 15 % is my pick for "much too high". I read "too
high" as too loud, not as pitch that's too high. If you meant pitch, say so
and that becomes a separate ticket to re-make the sounds.
