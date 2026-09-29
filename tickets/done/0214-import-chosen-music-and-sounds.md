---
id: "0214"
title: "Import the chosen third-party music and sounds, with credits"
type: content
milestone: M1 Engine
model: sonnet-5
effort: medium
status: done
blocked_by: ["0212"]
nick_input: setup
completed: 2026-09-29
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
  - Convert to OGG Vorbis: music 44.1 kHz stereo, around q5; sounds mono,
    also **44.1 kHz** (ADR-0026: the validator refuses any other rate,
    because the native player resamples badly).
  - Keep each track at or under about 4 minutes (ADR-0026 memory budget:
    a decoded track is about 21 MB per minute).
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
  (the official legal code). Ship them in every release package too
  (`release.yml` copies only `assets/fonts/*LICENSE*` today; ADR-0026).
- A `THIRD_PARTY_ASSETS.md` row per work.
- A size report in the Completion notes: total music size, total sound size,
  and the WASM download size before and after.
- Measure the frame hitch when a track starts loading on native (quad-snd
  decodes the whole OGG in one frame, ADR-0026). If it's bad, write a ticket.

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

- [x] Every cue in `audio.md`'s music and sound tables has a file and a
      credit, except the ones marked open.
- [x] Every credit has title, author, source URL, license, tags and a note.
- [x] `THIRD_PARTY_ASSETS.md` has a row for every work, with license files
      committed.
- [x] Loop versions and no-vocal versions used wherever the source offers
      them (list which in the Completion notes).
- [x] Size report in the Completion notes.
- [x] Manifest validation and the cue-coverage test pass.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: cue coverage against `audio.md`; manifest validation (from 0212)
  over the real manifest.

## Completion notes

**Done.** All 25 music tracks and 16 recorded sound files (17 recorded
sound cues: `hit_axe` shares `hit_spear`'s file) are in `music/` and
`assets/audio/sfx/`, each with a credit (title, author, link, license,
tags, note) in `assets/audio/audio.ron` and a row in
`THIRD_PARTY_ASSETS.md`. Every license was re-checked on its page on
2026-09-29: all still CC0 or CC BY 4.0 as `audio.md` says (Battle
Theme A's page lists several licenses; we take its CC0). The in-house
cues (`menu_*`, `cursor_move`, `miss`, `heal`) are 0213's.

**Licenses (Nick, on this ticket):** "idc which CC it is as long as I can
freely use it (and redistribute)", and CC BY 3.0 is fine. ADR-0027 adds
`CC-BY-3.0` to the allowed art/audio licenses (validator too). NC, ND and
SA stay out (NC forbids selling; ND forbids our trims and loops; SA is
copyleft). None of the chosen works turned out to be CC BY 3.0.

**Versions used:**
- Loop files: Squirrel Village `SV_loop.mp3` (not the head/tail files),
  Father's Scabbard `Loop.wav` (not the intro). Hope and RPG - Battle
  Theme are made to loop (the uncompressed WAV for the latter).
- New Sunrise V1 (`New Sunrise.wav`) → `title`, V2 → `city_first_visit`;
  Ending Scene's orchestral version; Battle Themes 1, 2, 3, 5 (not 4).
- **Epic Endgame Cinematic has no instrumental version**; the only file
  has the female vocals Nick heard on the audition page.
- Hesitation: FMA's download button needs a login, so we use the MP3 its
  player streams. freesound sounds: the public HQ previews (as planned).

**Deviations:**
- The originals (~430 MB) are **not committed**: `assets-src/audio/originals/`
  is git-ignored, and `assets-src/audio/import.py` re-downloads them
  (SHA-256 checked) and rebuilds every file (ADR-0027).
- `step_foot` has three authors, so the manifest gained
  `credit: Credits([...])` (ADR-0027), with tests.
- Dark Quest (4:20) and Sigil (4:19) are a little over "about 4 minutes";
  kept whole (cutting would break their loops). Each decodes to ~92 MB.
- Release packages now include `THIRD_PARTY_ASSETS.md` and
  `audio-licenses/` (CC0 and CC BY 4.0 legal code), the attribution
  CC BY needs until the credits screen (0808).

**Loudness** (ADR-0027): music −20 LUFS integrated, sounds −18 LUFS
momentary max, linear gain, true peak ≤ −1 dBTP. Every track hit its
target. Sounds kept below target by that peak ceiling: `step_mounted`
−25.8, `hit_bow` −21.9, `step_foot`'s dirt variant −21.7 and gravel
variant −20.1, `hit_spear`/`hit_axe` −20.4, `step_armored` −20.0,
`hit_sword` −19.8. Per-cue `volume` tunes the mix in game (0424/0425).

**Sizes:**
- Music: 25 files, 52,887,835 bytes (50.4 MiB), not in the WASM.
- Sounds: 16 files, 216,240 bytes (211 KiB), embedded.
- WASM (release, before `wasm-opt`): 2,541,402 → 2,806,684 bytes
  (+259 KiB: the sounds, the two license texts, the manifest).

**Frame hitch** (native, release, quad-snd 0.2.8's decode timed on each
track on Nick's machine): 80 ms (48 s track) to 460 ms (4:19); most
110–260 ms. That's a visible freeze whenever the music changes, so I
wrote **0215** (decode off the main thread).

**Claude's starting rules** (Nick can veto; all *tunable*):
- `step_mounted` is one hoof-beat pair (0.14 s) played per tile, like the
  other footsteps, rather than the gallop looping while the unit moves.
  Units walk 12 tiles/s.
- Mla's "Battle" loops as a whole file: its ~8 s intro plays again on
  each loop (the author's loop point needs seeking, which the player
  can't do).
- Sounds are mixed a little louder than the music (targets above).

**Follow-up tickets:** 0215.
