# ADR-0027: Importing third-party audio; CC BY 3.0 allowed

- **Status:** Accepted
- **Date:** 2026-09-29
- **Related tickets:** 0214, 0213, 0424, 0425, 0807, 0808
- **Amends:** ADR-0013 (the art/audio license list), ADR-0026 (the
  manifest's allowed licenses and `credit` field)

## Context

Ticket 0214 imports the 25 tracks and 17 recorded sounds Nick chose in 0020
(`docs/design/audio.md`). Four things weren't settled:

- **CC BY 3.0.** ADR-0013 allowed only `CC0-1.0` and `CC-BY-4.0` for art
  and audio. Nick, on 0214: "personally idc which CC it is as long as I can
  freely use it (and redistribute? since I will be hosting in this repo) ...
  some tracks I wanted are CC BY 3.0 not 4.0 so those should be ok I
  think?" CC BY 3.0 has the same terms that matter to us as 4.0 (use,
  change, sell and redistribute, with credit). The differences (4.0's
  database rights and its 30-day grace period to fix a missing credit)
  don't change what we must do: credit every work, which `audio.md` rule 3
  already asks for.
- **Where the originals live.** The downloads total about 430 MB (WAV,
  FLAC and 24-bit zips). Committing them would bloat every clone and CI
  checkout for files the game never loads.
- **A sound with variants by different authors.** `step_foot` picks one of
  three footsteps, from three freesound users. ADR-0026's `credit` names one
  work.
- **Loudness.** The sources range from −25 to −10 LUFS; "volume-matched on
  import" (`audio.md`) needs a target.

## Decision

### 1. Licenses for art and audio

Allowed: `CC0-1.0`, `CC-BY-3.0`, `CC-BY-4.0`, or our own work. The
manifest validator (`trpg_content::audio::LICENSES`) accepts exactly these.
Still denied, as in ADR-0013, because each fails "freely use and
redistribute" for a game that is sold:

- **NC** (non-commercial): forbids selling the game.
- **ND** (no derivatives): forbids the trimming, looping and volume changes
  every import makes.
- **SA** (share-alike, copyleft): could require releasing our changes (or
  more) under the same license.

The license text of every license used ships in `assets/audio/licenses/`.
The CC BY 3.0 text is added with the first CC BY 3.0 work (none of the
0020 picks is 3.0: every page still says CC0 or CC BY 4.0).

### 2. Originals stay out of git; the import is a script

`assets-src/audio/import.py` downloads each original into
`assets-src/audio/originals/` (git-ignored), checks its SHA-256, and writes
the game files with ffmpeg: music to `music/`, sounds to
`assets/audio/sfx/`. Anyone can rebuild the files from the script, and a
changed source fails the hash check instead of changing silently. The
commands and settings are in `assets-src/audio/README.md`.

freesound originals and Free Music Archive downloads need a login, which
Claude must not use, so the script takes the public files: freesound's
HQ OGG previews and the MP3 FMA's player streams. Same license, lower
quality than the originals. Nick can replace any of them by downloading the
original himself (0214's "Nick input").

### 3. Loudness

Linear gain only (no compressor or limiter), and never above −1 dBTP true
peak:

- **Music:** −20 LUFS integrated (EBU R128).
- **Sounds:** −18 LUFS *momentary maximum* (the loudest 400 ms). Integrated
  loudness is unreliable for sounds under a second.

A sound whose peaks don't allow the full gain stays quieter (the script
prints each gain). The manifest's per-cue `volume` then tunes the mix in
game (0424/0425/0807); it starts at 100 everywhere.

### 4. Several credits for one cue

`CreditRef` gains `Credits(["a", "b", …])`, beside `Own` and
`Credit("a")`. The validator refuses an empty list or an unknown id.
`CreditRef::ids()` lists the credits of any cue (the credits screen, 0808,
uses it).

### 5. Credits ship with every package

Until the credits screen (0808) exists, every release package includes
`THIRD_PARTY_ASSETS.md` (a row per work with author, link and license) and
`audio-licenses/` (the license texts). This is the attribution CC BY
requires.

## Consequences

- Nick can pick CC BY 3.0 tracks in later rounds.
- Rebuilding the audio needs Python 3 and ffmpeg (MSYS2's
  `mingw-w64-x86_64-ffmpeg`). Neither ships with the game; ffmpeg's GPL
  doesn't touch it.
- Re-downloading can fail if a source page removes a file; the committed
  game files remain, and the credit records where they came from.
- The freesound previews and the FMA MP3 are lossy sources re-encoded to
  Vorbis. At these bitrates that isn't audible in short hits or behind
  gameplay.

## Alternatives considered

- **Commit the originals** (as `assets-src/fonts/` does for the font BDF).
  The BDF is 100 KB; these are 430 MB.
- **Git LFS for originals.** Same quota problem as ADR-0026's music.
- **Loudness-normalise with ffmpeg's `loudnorm` (dynamic).** It changes the
  music's dynamics; a static gain keeps the tracks as composed.
- **Split `step_foot` into three cues with one credit each.** The random
  pick among variants is exactly what a sound's `files` list is for.
