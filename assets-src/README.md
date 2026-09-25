# Asset sources

Inputs to the asset tools. Nothing here is embedded in the game; the tools
write their output into `assets/`.

| Path | What | Tool |
| ---- | ---- | ---- |
| `fonts/ter-u16n.bdf` | Terminus Font 4.49.1, 8×16 medium (OFL-1.1, see `assets/fonts/Terminus-LICENSE.txt`) | `cargo xtask font-atlas` → `assets/fonts/atlas.{png,ron}` |
