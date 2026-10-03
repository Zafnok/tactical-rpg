# The `voice/` folder

Voice clips for dialogue lines ([ADR-0046](adr/0046-voice-clips-by-line-id.md)).
The folder sits **beside the game** (next to the executable, or next to
`index.html` on the web) and is never embedded. Without it the game runs
the same, silent, and says nothing about it.

In this repository `voice/` is git-ignored: real clips live in the private
assets repository, in its top-level `voice/` folder, and `cargo xtask
private-assets` copies them here. The only clips in this repository are the
test tones in `crates/app/tests/voice/`, laid out the same way.

## Layout

```
voice/
  en/                         one folder per language; only `en` is read today
    voice.ron                 the manifest: every clip
    cast.ron                  which voice says each speaker's lines
    ch01_intro/               one folder per scene id
      ch01_intro_1a2b3c4d.ogg     <line id>.ogg
      ch01_intro_5e6f7a8b.m.ogg   said of a male lead
      ch01_intro_5e6f7a8b.f.ogg   said of a female lead
```

- A clip is named after its **line id** (`cargo xtask lines [scene]` lists
  them). Rewording a line changes its id, so the old clip no longer
  belongs to anything.
- Files are OGG Vorbis, 44.1 kHz, mono.
- A line whose words change with the lead's gender (`{they}`, `{their}`…)
  has two clips, `.m.ogg` and `.f.ogg`. Every other line has one.
- A line with `{lead}` (the name the player types) can't have a clip.

## `voice.ron`

```ron
(
    clips: [
        (
            line: "ch01_intro_1a2b3c4d",
            spoken: "The king rides for Harrow Keep at dawn.",
            voice: "hollis_a",
            made_by: Generated(tool: "…", model: "…", date: "2026-10-03"),
        ),
        (
            line: "ch01_intro_5e6f7a8b",
            variant: M,
            spoken: "He left his sword with the king.",
            voice: "hollis_a",
            made_by: Recorded(actor: "A. Person"),
        ),
        (
            line: "ch01_intro_5e6f7a8b",
            variant: F,
            spoken: "She left her sword with the king.",
            voice: "hollis_a",
            made_by: Recorded(actor: "A. Person"),
        ),
    ],
)
```

| Field | Meaning |
| ----- | ------- |
| `line` | The dialogue line id. |
| `variant` | `M` or `F` for a gendered pair; left out otherwise. |
| `spoken` | The words the clip says: the line's text with every `{n:…}` name and pronoun filled in, as it was when the clip was made. |
| `voice` | The cast voice that says it. |
| `made_by` | `Generated(tool, model, date)` or `Recorded(actor)`. The credits and store pages say which voices are generated while any are. |

The game refuses the whole manifest (a logged warning, no voices) when:

- a `line` is not the id of a dialogue line,
- a line has the same variant twice,
- a line has `M` without `F` or the other way round, or both a plain clip
  and a gendered pair,
- `spoken` or `voice` is empty, or the file isn't valid RON.

### Stale clips

A clip is **stale** when its `spoken` is not what its line says today, for
example after a rename in `assets/data/names.ron`. A stale clip is not an
error: the game leaves it out and that line is silent until the clip is
made again. The same goes for a plain clip on a line that needs a gendered
pair, and for any clip on a line with `{lead}`.

## `cast.ron`

```ron
(
    voices: {
        "hollis": "hollis_a",
        ">": "narrator_a",
    },
)
```

Speaker (a character id, or `>` for narration) to voice id. The game doesn't
read it; the clip-making tool (`cargo xtask voice`, ticket 0721) does.

## How the game plays them

- At start-up `app` reads `voice/en/voice.ron` (from beside the executable,
  else the working directory; fetched on the web) and hands the text to
  `Ctx::set_voice_manifest`, which validates it and keeps the clips that
  aren't stale.
- A screen says a line with `ctx.play_voice(&line_id)`. Nothing happens if
  voices are off (`ctx.voices_on`), or the line has no playable clip. One
  voice plays at a time: a new one stops the old. `ctx.stop_voice()` stops
  it.
- When a scene starts, a screen calls `ctx.preload_voices(&line_ids)` with
  its lines in script order. `app` keeps the next 3 clips loaded and frees
  those passed.
- A clip that isn't loaded within 0.3 s of being asked for is skipped
  rather than played late.
- Clips play at `ctx.voice_volume` (0–10, default 8).
- A missing or unreadable clip file is a logged warning and silence.

No game screen plays a voice yet (the dialogue screen will, ticket 0720).
Until then the debug menu's **Voice test** says the test scene's voiced
lines, one per press.

## Trying it with the test clips

```bash
cp -r crates/app/tests/voice voice
```

```bash
cargo run -p trpg-app
```

Open the debug menu and choose "Voice test". For the web build, run
`cargo xtask web` after the copy: it puts `voice/` in `dist/web/voice/`.

The test clips are tones made by `cargo xtask voice-test-clips` (it needs
`ffmpeg`), one per entry of `crates/app/tests/voice/en/voice.ron`.

## Shipping

- `cargo xtask web` copies `voice/` (its `.ron` and `.ogg` files) into
  `dist/web/voice/` when the folder exists.
- `release.yml` copies `voice/` into each package when the folder exists.
- CI builds get the folder from the private repository
  (`.github/actions/private-assets`). Gates never read it.
