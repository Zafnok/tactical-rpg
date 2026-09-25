# ADR-0009: Distribution — Windows first, web, itch.io, Steam

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

End goal: publish on itch.io and Steam as an executable. Windows first (most
players), cross-platform preferred, browser play nice-to-have.

## Decision

| Target | Build | Channel | When |
| ------ | ----- | ------- | ---- |
| Windows x86_64 | `x86_64-pc-windows-msvc` on GitHub `windows-latest`; single `.exe`, assets embedded, no console window, icon + version metadata | GitHub Releases, itch.io (`windows` channel), later Steam | Milestone M1 onwards |
| Web | `wasm32-unknown-unknown` + macroquad JS loader + `index.html` | GitHub Pages (every push to `main`), itch.io HTML5 (`web` channel) | Milestone M1 onwards |
| Linux x86_64 | `x86_64-unknown-linux-gnu` on `ubuntu-latest` | GitHub Releases, itch.io | Best effort, built by CI from day one |
| macOS (universal) | `aarch64` + `x86_64` on `macos-latest` | GitHub Releases, itch.io | Best effort; unsigned (players must right-click → Open) |
| Steam | Same Windows build + `steamworks` crate behind a Cargo feature `steam` | Steamworks depots via `steamcmd` | After Chapter 1 is fun; requires Nick's Steamworks account and the $100 app fee |

Rules:

- **Releases are cut by pushing a tag** `vMAJOR.MINOR.PATCH`. The release
  workflow (ticket 0107) builds everything and creates a GitHub Release; the
  itch workflow (ticket 0901) pushes with `butler`.
- Version numbers come from the workspace `Cargo.toml`.
- **Saves and settings** go in the OS config directory (`directories` crate) on
  native, and browser `localStorage` on web, behind one storage trait (ticket 0207).
- **Code signing:** none at first (Windows SmartScreen will warn on first run,
  normal for indie itch games). The SignPath Foundation offers free signing to
  open-source projects; a later ticket may apply.
- The `steam` feature must never be required for a normal build; the game runs
  identically without Steam.

## Consequences

- Nick gets a playable build link (Pages) from the very first rendering ticket,
  with no install.
- Store accounts, API keys and fees are Nick's actions; the tickets that need
  them say so in their "Nick input" section.

## Alternatives considered

- **Installer (MSI/NSIS)** — unnecessary for a single self-contained exe; itch
  and Steam both handle zip/depot installs.
- **Web-only** — loses Steam, and Nick wants an executable.
