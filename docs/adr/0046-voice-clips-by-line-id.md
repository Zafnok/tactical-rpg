# ADR-0046: Voice clips are files keyed by dialogue line id, generated now, replaceable by recordings

- **Status:** Proposed (becomes Accepted with ticket 0238, which may adjust details)
- **Date:** 2026-10-03
- **Related tickets:** 0043, 0238, 0717, 0720, 0721, 0722, 0826, 0907, 0903
- **Amends:** ADR-0026 (a third kind of audio request) and ADR-0032 rule 2
  (AI-generated *voice* is allowed; the rule for pictures is unchanged).

## Context

Nick wants voiced story lines without paying a cast yet: AI-generated
voices, a switch to turn them off, and a real cast replacing them later
(`docs/design/voices-languages-and-script.md`). What is voiced, by which
voices, with which tool, and whether paying for a tool is fine are his to
decide in ticket 0043.

Facts:

- ADR-0026: screens ask for audio, `app` plays it through quad-snd, which
  decodes a whole file into memory and can play, stop and set volume.
  Music sits in `music/` beside the game, not embedded.
- ADR-0032: audio is free; AI-assisted art needs human work on it; files
  that may not be redistributed live in the private assets repository
  (ADR-0040).
- Chapter 1 has about 370 script lines. A voiced chapter is roughly 15 to
  20 minutes of speech; a whole game is hours.
- Lines contain tokens: `{n:king}` (names Nick may change), `{lead}` (the
  player's own name) and `{they}` (his/her by the lead's gender).
- Steam requires disclosing pre-generated AI content on the store page.

## Decision

1. **A clip belongs to a dialogue line id** (ADR-0045 §3). Rewording a line
   gives it a new id, so it has no clip until one is made. No clip is ever
   played for words it doesn't say.
2. **Files:** `voice/<lang>/<scene id>/<line id>.ogg`, OGG Vorbis,
   44.1 kHz, mono (ADR-0026 §4). Where the lead's gender changes the
   spoken words, `<line id>.m.ogg` and `<line id>.f.ogg`.
3. **Manifest:** `voice/<lang>/voice.ron` lists every clip: line id,
   `spoken` (the text with name tokens filled in, as it was said), the
   cast voice, and `made_by: Generated(tool, model, date)` or
   `Recorded(actor)`. A clip whose `spoken` no longer matches the line
   with today's names is stale and isn't played. A `cast.ron` beside it
   maps each character to a voice.
4. **Not embedded.** `voice/` ships beside the game like `music/`; the web
   build fetches a scene's clips when the scene starts. With no `voice/`
   folder, or no clip for a line, the line is silent and nothing warns.
5. **Kept in the private assets repository**, not this one: hours of audio
   don't belong in a public git history, and a hired cast's recordings
   will come with a contract that forbids handing them out. Gates never
   need them (ADR-0040).
6. **Playing:** `AudioRequest::PlayVoice { line }` and `StopVoice`, queued
   by screens like sounds (ADR-0026 §2). One voice plays at a time; a new
   one stops the old. `Settings` gets voices on/off and a voice volume
   (0826). What the dialogue screen does with them is 0720.
7. **Making them:** `cargo xtask voice` (0721) reads the scripts, the cast
   and the manifest, and generates the clips that are missing or stale.
   It runs on a developer machine, never in the game or in CI.
8. **Which tools are allowed.** The tool's terms, read on the day, must
   give commercial rights to the output with no royalties. No voice
   cloned from a real person without their written consent; no voice
   imitating a known actor or character. The terms are quoted in
   `THIRD_PARTY_ASSETS.md`. A free tool is preferred (ADR-0032: audio
   stays free); a paid one needs Nick's yes in 0043, as he buys art.
9. **Disclosure.** The credits screen, the itch page and Steam's content
   survey say the voices are AI-generated while any are (0907, 0903).
10. **Replacing with a cast.** Recorded files go in under the same ids
    with `made_by: Recorded`; nothing in the game changes. The voice tool
    can print a recording script per character (line id, scene, the line
    before, the words).

## Consequences

- The game works the same with no voice files; voices are a layer.
- A rename in `names.ron`, or a rewritten scene, silences the lines it
  touches until the tool is run again. That's visible in the tool's
  report, not in a gate.
- Lines with `{lead}` can't be voiced as written. 0043 decides what
  happens to them.
- The Pages and release builds need the private repository for voices, as
  they do for art.
- Generated speech is not reproducible bit for bit; the manifest, not a
  rebuild, is the record of what shipped.

## Alternatives considered

- **Text-to-speech at run time** — no usable engine in WASM, voices would
  differ per machine, and it can't be replaced by recordings.
- **Clips as cues in `audio.ron`** — thousands of entries in an embedded
  file, validated on every start-up.
- **`@voice <file>` lines in the script** — clutter in a file meant for a
  human writer, and nothing would notice a line reworded under its clip.
- **Clips in this repository** — size, and recorded performances later
  can't be public.
- **Embedding clips** — the web build would download every voice before
  the title screen.
