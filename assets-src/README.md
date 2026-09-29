# Asset sources

Inputs to the asset tools. Nothing here is embedded in the game; the tools
write their output into `assets/`.

| Path | What | Tool |
| ---- | ---- | ---- |
| `fonts/ter-u16n.bdf` | Terminus Font 4.49.1, 8×16 medium (OFL-1.1, see `assets/fonts/Terminus-LICENSE.txt`) | `cargo xtask font-atlas` → `assets/fonts/atlas.{png,ron}` |
| `audio/sound-audition.html` | The listening page Nick chose the game's sounds on (ticket 0020, `docs/design/audio.md`). Its Web Audio code is the recipe for the sounds we make ourselves: menu move B, select L, cancel B, dodge `miss:quick` (W2), heal HE5 | `cargo xtask sfx` → `assets/audio/sfx/*.wav` (`--check` only verifies the committed files; a test runs it) |
