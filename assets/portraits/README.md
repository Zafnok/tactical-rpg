# Character portraits

A portrait is a **sidecar file** here, `<id>.ron`, that maps expression
names to **PNG images** (ADR-0043). It is loaded by
`trpg_content::portrait` and validated by the all-assets test (ADR-0005).
The file stem is the character id and must equal the sidecar's `character`
(`ana.ron` → `"ana"`). The lead is the exception: the character `lead` has
two portraits, `lead_m` and `lead_f`, and dialogue shows the one for the
gender the player picked.

`test_lord` and `test_knight` are **placeholders** for tests and the viewer,
not real characters. `lead_m` and `lead_f` are placeholder recolours of
`test_lord`. The real portraits are bought art (ADR-0032): they are not in
this repository but in `assets-private/game/portraits/` (ADR-0040), where a
file replaces the one at the same path here. Ticket 0706 picks them.

## Format

```ron
// RON `//` comments are allowed.
(
    character: "ana",
    expressions: {
        "neutral": "ana/neutral.png",
        "happy": "ana/smile.png",
        "angry": "ana/stern.png",
        "sad": "ana/sad.png",
        "surprised": "ana/surprise.png",
        "sly": "ana/sly.png",
    },
)
```

- `character`: the character id (the file stem).
- `expressions`: expression name → image file, relative to this directory.
  Keep a character's images in a folder named after it. Two expressions may
  use the same image.

Required expressions: `neutral`, `happy`, `angry`, `sad`, `surprised`. More
are allowed (dialogue can name them).

Images are PNG with transparency, at their native pixel size, at most
256×256. The colours are the image's own, not `palette.ron`'s.

## How it's drawn

One sprite item (ADR-0038) in the 32×16-cell portrait frame, which is
256×256 console pixels: the whole image at the **largest whole scale that
fits**, centred. A 64×64 cut bust is drawn at 4× and fills the frame; a
48×48 face at 5× with an 8-pixel margin; the 32×32 placeholders at 8×.
Transparent pixels show the frame's background. Dimming (the listener in a
conversation) is the sprite's opacity; mirroring is its left-right flip.

To look at a portrait, run a debug build, press **F2** and pick
**Portraits**: left/right switch expression, up/down switch character. Or
render a frame without a window: `cargo xtask frame-png --help`.

## Importing bought busts

```bash
cargo xtask private-assets --library
cargo xtask portrait-import assets-private/library/tiny-tales/characters/heroes/<Name> <id>
```

This cuts each 80×80 bust to its middle 64 columns and bottom 64 rows
(`--shift-x <pixels>`, −8 to 8, moves the cut sideways), writes the 64×64
files to `assets-private/game/portraits/<id>/`, and writes a sidecar there
if there is none. Then push the private repository and pin it (ADR-0040).
`cargo xtask portrait-import --help` says what else it takes.

## Rules checked by the loader

Each is reported with its file (and line, for an expression), and all are
reported at once:

- a RON syntax error, or a field the format doesn't have;
- a `character` that doesn't match the file name;
- a missing required expression;
- an expression whose file isn't a PNG image under `assets/portraits/`;
- an image wider or taller than 256 pixels.
