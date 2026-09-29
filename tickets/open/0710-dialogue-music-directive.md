---
id: "0710"
title: "Dialogue scripts choose the music: @music"
type: feature
milestone: M6 Story & dialogue
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0212", "0704"]
nick_input: none
completed:
---

# 0710 — Dialogue scripts choose the music

## Context

Nick decided (ticket 0020, [`docs/design/audio.md`](../../docs/design/audio.md)
rule 7) that conversations use **mood tracks, like Fire Emblem**: a story scene
can switch to a mood cue such as `talk_calm`, `talk_antagonist`,
`scene_sad`, `scene_tragic` or `dungeon_tense`. Which scene uses which mood is
story content, so the script must be able to say it. Scripts are `.dlg` files
(format in `assets/dialogue/README.md`, parser in
`crates/content/src/dialogue/`, ADR-0005 and ADR-0011). The dialogue screen is
ticket 0704.

## Nick input

None.

## Scope

**In:**
- A new directive: `@music <cue>` switches to that music cue, and
  `@music stop` fades the music out. It may appear anywhere inside a scene,
  and takes effect when playback reaches that line.
- Parser, validator and printer support. The validator checks that the cue
  is a music cue in `assets/audio/audio.ron` (0212). Update
  `assets/dialogue/README.md`, and the `story-writing` skill's notes on the
  format if it documents directives.
- The dialogue screen emits `play_music` / `stop_music` (0212) when it
  reaches the line.
- Music keeps playing after a scene ends, until something else asks for a
  different cue: the next scene, a battle (0807) or the title. A scene
  without `@music` doesn't change the music.
- Skipping a scene (0704's skip) still applies its **last** `@music`, so the
  music after a skipped scene is the same as after a watched one.

**Out (do not do):**
- Picking moods for actual scenes. That's story work (0707 script,
  `story-writing` skill) using the cue list in `audio.md`.
- Battle or title music (0807, 0814).

## Implementation steps

1. Add the `Music(MusicLine)` line kind to the parsed scene model and parse
   `@music <cue>` and `@music stop`. It's an error to give no argument or more
   than one.
2. Round-trip it in `print_scene`.
3. Validate the cue against the audio manifest where the content is
   validated as a whole (dialogue checks already cross-reference characters
   and portraits).
4. In the dialogue screen, emit the request when the line is reached; on
   skip, emit the last `@music` of the scene.
5. Document it with an example in `assets/dialogue/README.md`.

## Acceptance criteria

- [ ] Parser unit tests: valid `@music talk_calm`, valid `@music stop`, and
      errors for none/two arguments; `print_scene` round-trips it.
- [ ] Validation fails for an unknown cue or a sound (not music) cue.
- [ ] Harness test: a scene with `@music talk_calm` mid-scene emits the
      request when that line is reached, not before.
- [ ] Harness test: skipping a scene with two `@music` lines emits only the
      last one.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parser, printer, validator.
- Snapshot / integration: Harness dialogue tests above.

## Completion notes

*(Filled in by the session that completes the ticket.)*
