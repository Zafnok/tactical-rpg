---
id: "0214"
title: "Import the chosen third-party music and sounds, with credits"
type: content
milestone: M1 Engine
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0212"]
nick_input: setup
completed:
---

# 0214 — Import the chosen third-party music and sounds

## Context

Nick picked every track and recorded sound in ticket 0020. They're listed with
their cue ids, source links and licenses in the "Music cues" and "Sound
effects" tables of [`docs/design/audio.md`](../../docs/design/audio.md). This
ticket downloads them, prepares them for the game and registers them with
full attribution.

Nick's rules (`audio.md` rules 3 and 9):
- **Credit every work, even CC0.**
- "organize them with tags, attribution, and prefer looping / no vocal
  versions."

License policy: ADR-0013. Only `CC0-1.0` and `CC-BY-4.0` may ship.
`THIRD_PARTY_ASSETS.md` needs a row per work, and the license texts must be
committed next to the files.

## Nick input

**Setup:** approve the downloads when asked. Before downloading, the session
lists every file with its source and size, as the safety rules require.
Freesound originals need a logged-in account, and Claude must not log in. So
this ticket uses freesound's public high-quality previews (see step 2). If
Nick would rather have the originals: log in at freesound.org, open each
sound's page (links in `audio.md`), click **Download**, and put the files in
`assets-src/audio/freesound/`.

## Scope

**In:**
- Every music cue and every recorded sound cue in `audio.md`. That's about 20
  music files and 17 sound files, counting the three footstep variants and
  the skirmish pool's five tracks.
- For each source:
  1. Open its page and **re-check the license** (pages change). If it's no
     longer CC0 or CC-BY 4.0, don't import it: note it and tell Nick.
  2. Pick the right version: the loop file where there is one (Squirrel
     Village and Father's Scabbard have head/loop/tail files); the
     no-vocal version where there is one (check Epic Endgame Cinematic); the
     version `audio.md` names (New Sunrise V1 and V2, Ending Scene's
     orchestral version, Battle Themes 1, 2, 3 and 5 only).
  3. Keep the original download in `assets-src/audio/` (not shipped). Write
     the game file into the music folder chosen by ADR-0026, or into
     `assets/audio/sfx/`.
- **Prepare the files:**
  - Convert to OGG Vorbis: music 44.1 kHz stereo, around q5; sounds mono.
  - Loudness-match everything: music to one target, sounds to another.
  - Trim leading silence from sounds.
  - For `step_mounted`, cut a short gallop segment or single hoof beats from
    the 39 s recording. Match the tile-step timing of battle playback (check
    `crates/ui/src/screens/battle/playback.rs`).
  - Nick asked for the magic crits untrimmed.
  - Record the exact conversion commands in `assets-src/audio/README.md`.
    Any free tool is fine (e.g. ffmpeg); it isn't shipped.
- **Attribution:**
  - One `credits` entry per work in `assets/audio/audio.ron`: title, author,
    source URL, license, a note naming the file or version used, and tags.
    Use tags such as `music`/`sfx`, the cue, mood (`calm`, `tense`, `sad`,
    `epic`…), place (`village`, `city`, `dungeon`…) and instruments where the
    page says them.
  - Wire each sound and music cue to its credit.
- License texts: `assets/audio/licenses/CC0-1.0.txt` and `CC-BY-4.0.txt`
  (the official legal code).
- A `THIRD_PARTY_ASSETS.md` row per work.
- A size report in the Completion notes: total music size, total sound size,
  and the WASM download size before and after.

**Out (do not do):**
- Playing any cue in the game (0424, 0425, 0710, 0807).
- Choosing replacements. If a source is gone or its license changed, stop
  and ask Nick.
- Anything `audio.md` lists as an open sub-question (banter track, plain-spell
  crit, fliers).

## Implementation steps

1. Build a table from `audio.md`: cue → page URL → file to take.
2. List the downloads (file, source, size) and ask Nick to approve.
   - Direct links: OpenGameArt file links; the Free Music Archive track
     download.
   - Freesound: the `previews` HQ OGG URL shown on each sound page. It's
     public, and the license is the same as the sound's.
3. Download, re-check licenses, then convert, trim and normalise.
4. Write the manifest entries and license files, and update
   `THIRD_PARTY_ASSETS.md`.
5. Run the manifest validator (0212). Every cue in `audio.md`'s two tables
   must resolve.
6. Add a test that every cue id listed in `audio.md` exists in the manifest.
   Parse the tables' first column, or keep a list in the test with a comment
   pointing at `audio.md`.

## Acceptance criteria

- [ ] Every cue in `audio.md`'s music and sound tables has a file and a
      credit, except the ones marked open.
- [ ] Every credit has title, author, source URL, license, tags and a note.
- [ ] `THIRD_PARTY_ASSETS.md` has a row for every work, with license files
      committed.
- [ ] Loop versions and no-vocal versions used wherever the source offers
      them (list which in the Completion notes).
- [ ] Size report in the Completion notes.
- [ ] Manifest validation and the cue-coverage test pass.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: cue coverage against `audio.md`; manifest validation (from 0212)
  over the real manifest.

## Completion notes

*(Filled in by the session that completes the ticket.)*
