---
id: "0826"
title: "Options: voices on or off, voice volume, and what a new game asks"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0043", "0238", "0805"]
nick_input: answer-first
completed:
---

# 0826 — Options: voices on or off, voice volume, and what a new game asks

## Context

Nick: AI voices "with options menu to turn it off, maybe even ask upon
starting a new game" (`docs/design/voices-languages-and-script.md`).
0043 settles whether voices start on or off, whether a new game asks, and
the wording. 0238 put `voices_on` and `voice_volume` in `Ctx`; 0805 builds
Options and `Settings`.

## Nick input

**Answer first:** ticket 0043 (questions 5 and 6). Build exactly the
wording and behaviour in the design doc.

## Scope

**In:** the two settings saved; the Options rows; the new-game question
if 0043 chose one.

**Out (do not do):** the credits line and store text (0907); a voice
language setting (a new ticket if 0043 chose Japanese voices).

## Implementation steps

1. `Settings` (0805) gains `voices: bool` and `voice_volume: u8` (0–10,
   default 8, *tunable*), defaults per 0043; bump the settings version
   and migrate. Move 0238's `Ctx` fields to read from `Settings`.
2. Options: a "Voices" row (On / Off) and a "Voice volume" row beside
   music and sound volume, with 0043's label. Turning voices off stops a
   clip that is playing.
3. If 0043 chose a question at New Game: ask it once in the new-game
   flow (where 0043 placed it), with the default focus on the
   non-destructive answer 0043 names; the answer sets `Settings::voices`
   and can be changed in Options afterwards. If no `voice/` folder was
   found, don't ask and hide nothing: the rows stay, they just have
   nothing to play.
4. All text through `ctx.text` if 0233 has landed; otherwise literals,
   and add a line to 0234's list.

## Acceptance criteria

- [ ] Voices off: the dialogue screen queues no voice request; on: it does (Harness test, with 0720; if 0720 isn't done, test `play_voice` directly).
- [ ] Voice volume changes what `app` plays; 0 is silent; both settings survive a restart (tests).
- [ ] Old saved settings load with 0043's defaults (test).
- [ ] The new-game question behaves as the design doc says (Harness test), or is absent if not chosen.
- [ ] Snapshot of the Options rows.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: settings migration.
- Snapshot / integration: Options rows; the new-game question.

## Completion notes

