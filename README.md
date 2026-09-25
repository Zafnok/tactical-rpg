# tactical-rpg *(working title)*

An ASCII-art tactical RPG in the spirit of Fire Emblem, drawn with coloured
glyphs the way Dwarf Fortress and Rogue do it, and played entirely from the
keyboard: a virtual cursor, vim-style movement keys, one hand steering and one
hand acting.

Planned features include unit progression with level ups and class changes, a
main story plus personal character stories, and dialogue scenes with two ASCII
character portraits on screen.

**Status:** workspace skeleton only, no game logic yet. Work happens one
ticket at a time; see the [roadmap](docs/ROADMAP.md).

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=Zafnok_tactical-rpg&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=Zafnok_tactical-rpg)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=Zafnok_tactical-rpg&metric=coverage)](https://sonarcloud.io/summary/new_code?id=Zafnok_tactical-rpg)

## Play

The latest `main` build runs in your browser, no install:
**https://zafnok.github.io/tactical-rpg/**

## Targets

- Windows executable first (itch.io, later Steam), then Linux and macOS.
- Browser build (WASM).

## How this repo works

| Where | What |
| ----- | ---- |
| [`tickets/`](tickets/README.md) | The backlog. Each ticket is a Markdown file; finished ones move to `tickets/done/` |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Milestones and the path to a playable Chapter 1 |
| [`docs/adr/`](docs/adr/README.md) | Technical decisions (Rust, macroquad glyph renderer, testing, CI…) |
| [`docs/design/`](docs/design/README.md) | Game-design decisions, made by Nick |
| [`docs/story/`](docs/story/README.md) | Story beats, bible and characters |
| [`CLAUDE.md`](CLAUDE.md) + [`.claude/skills/`](.claude/skills) | Instructions for the AI sessions that build the game |

Code is written by Claude, one ticket per pull request. CI (tests on three
OSes, mutation testing, coverage, SonarCloud, security scanners) is the code
review. Nick owns the game design and plays the builds.

## License

**Source-available, not open source.** Copyright (c) 2026 Nick Wentz, all
rights reserved. You may read and learn from the code, use it in
non-commercial teaching, make free non-commercial mods, and stream or post
videos of the game (monetised is fine). You may not redistribute or sell it or
games made from it. See [`LICENSE`](LICENSE) for the exact terms, and
[`THIRD_PARTY_ASSETS.md`](THIRD_PARTY_ASSETS.md) for third-party material.
