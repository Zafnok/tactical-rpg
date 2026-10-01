# Audio sources

| Path | What |
| ---- | ---- |
| `sound-audition.html` | The listening page Nick chose the game's sounds on (0020). Recipes for the sounds we make ourselves (0213) |
| `import.py` | Downloads the chosen third-party music and sounds and makes the game files (0214, ADR-0027) |
| `originals/` | The downloads. **Git-ignored**: `import.py` fetches them again and checks each one's SHA-256 |

## Rebuilding the third-party audio

Needs Python 3 and ffmpeg. On Windows, with MSYS2:

```bash
C:/msys64/usr/bin/pacman.exe -S mingw-w64-x86_64-ffmpeg
python assets-src/audio/import.py          # download, convert, loudness-match
python assets-src/audio/import.py --check  # print each output's loudness
python assets-src/audio/import.py new_sunrise_v1.ogg  # redo only the named outputs
```

It writes `music/*.ogg` and `assets/audio/sfx/*.ogg`. Which cue uses which
file, and every credit: `assets/audio/audio.ron`. To add a work, add its
download (URL and SHA-256) and its conversion to the lists at the top of
`import.py`, then a cue and a credit in `audio.ron` and a row in
`THIRD_PARTY_ASSETS.md`.

A new music track, or one whose cut changes, also needs its length in
`audio.ron` (`length_ms`, ADR-0036). `cargo test -p trpg-content` fails
until it is right and prints the value to write
(see [`assets/audio/README.md`](../../assets/audio/README.md)).

## What the script does

For each **music** track (`MUSIC` in `import.py`):

1. Takes the original, or the named file from its zip (the loop file where
   the source has one: `SV_loop.mp3`, `Father's Scabbard - Loop.wav`).
2. Refuses anything over 4.5 minutes (ADR-0026's memory budget).
3. An optional cut (`MUSIC_CUTS`; only `new_sunrise_v1`: the original's first
   3 s of near-silence, `atrim=start=3.0,asetpts=PTS-STARTPTS,afade=t=in:d=0.05`).
4. Measures its loudness with `ffmpeg -af [<cut>,]aresample=44100:resampler=soxr,apad=whole_dur=0.5,ebur128=peak=true:framelog=info -f null -`.
5. Encodes: `ffmpeg -i <src> -map_metadata -1 -vn -af [<cut>,]aresample=44100:resampler=soxr,volume=<gain>dB -ac 2 -ar 44100 -c:a libvorbis -q:a 5 music/<name>.ogg`.

For each **sound** (`SFX`):

1. Filters: an optional cut (only `step_mounted`: `atrim=start=0.895:end=1.035,asetpts=PTS-STARTPTS,afade=t=out:st=0.105:d=0.035`),
   a stereo-to-mono mix (`pan=mono|c0=0.5*c0+0.5*c1`), `aresample=44100:resampler=soxr`,
   and a leading-silence trim (`silenceremove=start_periods=1:start_threshold=-50dB`).
   Tails are never cut (Nick wanted the magic crits whole).
2. Measures loudness as above, after those filters.
3. Encodes: `ffmpeg -i <src> -map_metadata -1 -af <filters>,volume=<gain>dB -ac 1 -ar 44100 -c:a libvorbis -q:a 6 assets/audio/sfx/<name>.ogg`.

**Gain** (ADR-0027): linear, to −20 LUFS integrated for music and −18 LUFS
momentary maximum for sounds, but never pushing the true peak above
−1 dBTP. A few quiet sounds stop short of the target because of that
ceiling (see the ticket 0214 completion notes).
