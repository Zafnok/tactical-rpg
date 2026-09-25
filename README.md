# tactical-rpg (working title)

An ASCII-art tactical RPG in the spirit of Fire Emblem, drawn with coloured
glyphs the way Dwarf Fortress and Rogue did it, controlled entirely from the
keyboard with a virtual cursor and vim-style movement keys.

Written in Rust. Ships as a native executable (Windows first, then Linux and
macOS) and as a browser build.

> The game does not have a name yet. Everything is called `tactical-rpg`
> until that decision is made.

## Status

Scaffold only. The three crates compile and are empty; every feature is a
ticket. See the [roadmap](docs/ROADMAP.md) and the
[issues](https://github.com/Zafnok/tactical-rpg/issues).

## Play

- Browser: deployed from `main` to GitHub Pages once the first feature lands.
- Desktop: download from [Releases](https://github.com/Zafnok/tactical-rpg/releases)
  once the first tag exists.

## Build

```bash
cargo run -p tactical-rpg        # desktop
cargo wasm                       # browser build, see docs/DEV_SETUP.md
cargo test --workspace
```

## Layout

| Path | What |
| ---- | ---- |
| `crates/core` | Game rules: pure, deterministic, no I/O |
| `crates/ui` | Virtual console, input actions, screens |
| `crates/app` | Window, font, raw input (macroquad); desktop and WASM |
| `assets/` | Game content as data (maps, units, dialogue) |
| `web/` | Browser shell page and vendored loader |
| `docs/DESIGN.md` | Game design decisions (Nick's) and open questions |
| `docs/adr/` | Technical decisions |
| `docs/story/` | Story bible and beats |
| `.claude/skills/` | Workflows for AI sessions working the backlog |

## How work happens

GitHub Issues are the backlog. One issue becomes one branch and one PR; CI
(format, clippy, tests on three OSes, mutation testing, licence and advisory
checks, CodeQL, SonarCloud) is the review. Game-design questions are labelled
`needs-nick` and answered by Nick. See [CONTRIBUTING.md](CONTRIBUTING.md).
