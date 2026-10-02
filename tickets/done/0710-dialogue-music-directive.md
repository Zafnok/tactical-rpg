---
id: "0710"
title: "Dialogue scripts choose the music: @music"
type: feature
milestone: M6 Story & dialogue
model: sonnet-5
effort: medium
status: done
blocked_by: ["0212", "0704"]
nick_input: none
completed: 2026-10-02
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

- [x] Parser unit tests: valid `@music talk_calm`, valid `@music stop`, and
      errors for none/two arguments; `print_scene` round-trips it.
- [x] Validation fails for an unknown cue or a sound (not music) cue.
- [x] Harness test: a scene with `@music talk_calm` mid-scene emits the
      request when that line is reached, not before.
- [x] Harness test: skipping a scene with two `@music` lines emits only the
      last one.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parser, printer, validator.
- Snapshot / integration: Harness dialogue tests above.

## Completion notes

Done as planned.

- **Format:** `@music <cue>` and `@music stop` parse into a new
  `Step::Music(MusicLine)` (`MusicLine::Cue(id)` or `MusicLine::Stop`), in a
  scene or in a reply's reaction. No argument, or more than one, is an
  error. `print_scene` writes it back; the round-trip property test covers it.
- **Validation:** `dialogue::load` now takes the audio manifest (loaded
  before the scripts). A cue that is unknown, a sound, or a music pool is an
  error that says which. Skipped if `audio.ron` itself failed to load.
- **Playback:** `DialoguePlayer` keeps the last `@music` line it reached
  (`take_music`); the dialogue screen asks for it (`play_music` /
  `stop_music`) at the end of the frame that reached it. So a skip, which
  passes many lines in one frame, asks only for the last.
- **Docs:** `assets/dialogue/README.md` has the two lines in the table, a
  "Music" section with an example, and `@music` in the full example; tests
  check both examples against the real `audio.ron`. The `story-writing`
  skill doesn't list directives (it points at the README), so it is unchanged.
- **Fixed on the way** (needed for this to be right): advancing a scene that
  had already finished re-ran its trailing lines. Harmless for portraits,
  but it would have asked for the last `@music` again.
- No scene in `ch01.dlg` got music: that is story work (out of scope here).

Deviation: step 1 says "line kind"; the scene model's kinds are `Step`s, so
it is `Step::Music(MusicLine)`.

**Claude's starting rules** (the ticket and `audio.md` were silent; Nick can
veto):

1. *Two `@music` lines with no text between them:* only the second is
   heard. Example: `@music village` then `@music talk_calm` then a line of
   speech starts `talk_calm`; `village` never plays.
2. *A reply can change the music, and replies needn't agree.* Example: the
   blunt reply's reaction says `@music talk_antagonist`, the other replies
   don't; after the choice the music is whatever the picked reply left.
   (Portraits and captions must match across replies; music needn't.)
3. *A scene shown over the battle map that changes the music leaves it
   changed:* the battle's own track does not come back when the scene
   closes, unless a later scene asks for it by name. (This follows "music
   keeps playing until something else asks".) Restarting the battle starts
   its track again.
4. *`@music` can't name a music pool* (like `skirmish`): a scene names one
   track.

Follow-ups: none. Picking the moods for Chapter 1's scenes is already ticket
0803 (it sets the chapter's music).
