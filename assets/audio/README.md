# Audio manifest

`audio.ron` lists every sound and music cue the game can play, the music
pools, and a credit for every third-party work. Screens ask for a cue by id
(`ctx.audio.play_sound("menu_move")`); the game resolves the id here.
Why it works this way: [ADR-0026](../../docs/adr/0026-audio-cues-and-music-files.md).
Which cue is which track: [`docs/design/audio.md`](../../docs/design/audio.md).

## Where the files go

| What | Folder | Shipped how |
| ---- | ------ | ----------- |
| Sound effects | `assets/audio/` (e.g. `assets/audio/sfx/menu_move.wav`) | Embedded in the game binary |
| Music | `music/` at the repo root | A folder next to the exe; served next to the WASM on the web |

Formats: **OGG Vorbis** for anything recorded (all music, third-party
sounds). **WAV** (PCM or float) is allowed for our own short sounds. Nothing
else. Every file is **44.1 kHz, mono or stereo**: the native player plays
at 44.1 kHz and resamples anything else badly, and a file it can't read
crashes it, so the checks below read every file's header.

## Format

```ron
(
    music_fade_ms: 500,           // old track fades out this long, then the new one starts
    sounds: [
        // files: relative to assets/audio/. Several files = variants, one
        // picked at random each time. volume: percent, 0-100.
        (id: "menu_move", files: ["sfx/menu_move.wav"], volume: 100, credit: Own),
        (id: "step_foot", files: ["sfx/step_1.ogg", "sfx/step_2.ogg"], volume: 80,
         credit: Credit("footsteps")),
    ],
    music: [
        // file: relative to music/. Loops unless `looped: false`.
        (id: "title", file: "new_sunrise_v1.ogg", volume: 100, credit: Credit("new_sunrise")),
    ],
    pools: [
        // Music cues one is picked from (e.g. skirmish battles).
        (id: "skirmish", music: ["battle_1", "battle_2"]),
    ],
    credits: [
        // One per third-party work (credit even CC0: audio.md rule 3).
        (id: "new_sunrise", title: "New Sunrise", author: "nene",
         source: "https://opengameart.org/content/new-sunrise",
         license: "CC0-1.0", tags: ["music", "title", "calm"], note: "V1 file"),
    ],
)
```

- `credit` is `Own` (we made it), `Credit("<credit id>")`, or
  `Credits(["<id>", "<id>"])` when a sound's variants come from several
  works (e.g. `step_foot`).
- `license` is `"CC0-1.0"`, `"CC-BY-3.0"`, `"CC-BY-4.0"` or `"Own"`
  (ADR-0013, ADR-0027). A credit that isn't `Own` needs a `title`, an
  `author` and a `source` link. The license texts are in `licenses/`.
- Every cue id (sounds, music and pools together) is unique.

## Third-party files

Made by `assets-src/audio/import.py` (converted, trimmed and
loudness-matched; see [`assets-src/audio/README.md`](../../assets-src/audio/README.md)).
Don't edit them by hand: change the script and run it again. Add every new
work to `THIRD_PARTY_ASSETS.md` too.

## Checks

`cargo test -p trpg-content` refuses: a duplicate id, an unknown or missing
file, a wrong file format (by extension and by header; wrong sample rate or
channel count), a volume over 100, a disallowed license, a credit
without title/author/link, an unknown credit (or an empty `Credits` list),
and a pool that is empty or names anything but a music cue. The game
checks the embedded sounds at start-up; the music folder is checked by the
tests only. Another test checks that every cue in `docs/design/audio.md`'s
tables is in the manifest.
