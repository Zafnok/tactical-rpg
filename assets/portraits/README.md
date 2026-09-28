# Character portraits (`.portrait`)

Each `*.portrait` file here is one character's portrait, loaded by
`trpg_content::portrait` and validated by the all-assets test (ADR-0005). The
file stem is the character id and must equal the header's `character`
(`ana.portrait` → `"ana"`). The style is in ADR-0018, `docs/design/look-and-feel.md`
and the `ascii-art` skill.

`test_lord` and `test_knight` are **placeholders** for tests and the viewer,
not real characters. Real portraits come with ticket 0706.

## Format

```
// PLACEHOLDER art …
(
    character: "ana",
    size: (32, 32),
    colors: { 'K': "portrait_outline", 'h': "hair_brown", 's': "skin_light", 'q': "skin_light_mid", 'e': "eye_green" },
)
=== neutral
................KKKKKKKK........
... 32 rows of exactly 32 keys ...
=== happy
...
```

1. **Header**: a RON struct with
   - `character`: the character id (the file stem);
   - `size`: `(32, 32)`, in pixels. It's drawn as 32×16 cells;
   - `colors`: pixel key (one character) → colour name from
     `assets/data/palette.ron`. Add portrait colours (skin tones, hair,
     metals…) to the palette as needed.
   RON `//` comments are allowed.
2. **Expressions**: each starts with a line `=== <name>`, followed by the
   pixel rows, top first: one key per pixel, `.` for transparent. Blank lines
   at the end of a block are ignored. The first `===` line ends the header.

Required expressions: `neutral`, `happy`, `angry`, `sad`, `surprised`. More are
allowed (dialogue can name them).

## How it's drawn

Each cell is `▀` with fg = the top pixel and bg = the bottom pixel, so pixels
are square (8×8 screen px). Where the top pixel is transparent the cell is `▄`
over the background; where both are, a blank cell. Dimming (the listener in
a conversation) lerps toward the background. Mirroring reverses each row.

To look at a portrait, run a debug build, press **F12** and pick
**Portraits**: left/right switch expression, up/down switch character.

## Rules checked by the loader

Each is reported with `file:line:column` where it has one, and all are
reported at once:

- no `===` line at all, or a RON syntax error in the header;
- `size` other than `(32, 32)`;
- an expression with the wrong number of rows, or a row of the wrong width;
- a pixel key that isn't in `colors`;
- a `colors` entry naming a colour the palette doesn't define, or using
  `.` (reserved for transparent);
- an expression with no name, or the same expression twice;
- a missing required expression;
- a `character` that doesn't match the file name.
