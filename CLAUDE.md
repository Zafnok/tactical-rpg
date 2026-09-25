# CLAUDE.md

ASCII-art tactical RPG (Fire Emblem-like, drawn with coloured glyphs like Dwarf
Fortress / Rogue), keyboard-driven with a virtual cursor and vim-style keys.
Rust. Windows exe first, plus web (WASM); itch.io then Steam.

## Who decides what

- **Nick (owner)** decides *game design*: stats, combat, magic, progression,
  story beats, difficulty, look & feel. He does **not** review code and does
  **not** want technical questions. Ask him design questions only via the
  `ask-nick` skill (options drawn from real games + "describe your own").
- **Claude** decides everything technical and records significant choices as
  ADRs in `docs/adr/`.

## Hard rules

1. **Work one ticket at a time** from `tickets/open/`, using the `work-ticket`
   skill. One ticket = one branch = one PR. The PR moves the ticket to
   `tickets/done/`.
2. **No unticketed work.** Don't write game code "to get a head start". Found
   something else to do? Create a ticket (`write-ticket` skill).
3. **Never invent game-design answers.** They come from `docs/design/` (decided
   by `00xx` tickets) and `docs/story/beats.md`. If one is missing, the ticket
   is blocked; say so.
4. **Respect crate boundaries** (ADR-0004): `core` is pure and deterministic
   (no I/O, no clock, no macroquad); state changes only via `Command` → `Event`s;
   only `app` touches macroquad, files, clock, keyboard.
5. **Tests are the review** (ADR-0007). Run the `run-gates` skill before
   pushing. Never weaken a gate to pass it.
6. **Licensing** (ADR-0013): the game is proprietary (see `LICENSE`) and will
   be sold. Only ship dependencies and assets under the permissive licenses
   allowed by ADR-0013: no GPL/LGPL/MPL/copyleft, no non-commercial, nothing that
   costs money. Every non-crate asset goes in `THIRD_PARTY_ASSETS.md`. Never
   change `LICENSE`; that is Nick's call.

## Map of the repo

| Path | What |
| ---- | ---- |
| `tickets/` | Backlog. `README.md` explains numbering & lifecycle. `open/` → `done/` |
| `docs/ROADMAP.md` | Milestones and the Chapter 1 critical path |
| `docs/adr/` | Technical decisions (read the index) |
| `LICENSE`, `THIRD_PARTY_ASSETS.md` | Proprietary source-available license; registry of shipped third-party assets |
| `docs/design/` | Nick's game-design decisions (filled by `00xx` tickets) |
| `docs/story/` | Story beats, bible, characters, outline, ledger (ADR-0011) |
| `.claude/skills/` | `work-ticket`, `write-ticket`, `write-adr`, `ask-nick`, `story-writing`, `ascii-art`, `run-gates` |
| `crates/` | `core` (`trpg-core`), `content` (`trpg-content`), `ui` (`trpg-ui`), `app` (`trpg-app`, binary `tactical-rpg`), `xtask` (repo tooling) |
| `assets/` | Everything embedded in the game: `data/`, `fonts/` (later: maps, dialogue, portraits) |
| `assets-src/` | Inputs to asset tools (e.g. the font BDF for `cargo xtask font-atlas`); not embedded |

## Environment

- Nick's machine: Windows 11, Rust with the **GNU** host toolchain; **no MSVC**.
  Never require MSVC locally. CI builds with MSVC on GitHub runners.
- Local builds need a full MSYS2 mingw-w64 toolchain (`winget install -e --id
  MSYS2.MSYS2` then `pacman -S mingw-w64-x86_64-gcc`): rustup's self-contained
  GNU linker ships without some import libraries (e.g. `imm32`, needed by
  macroquad). `.cargo/config.toml` points the `x86_64-pc-windows-gnu` linker
  at the MSYS2 install.
- Use `cargo install --locked <tool>` (cargo-binstall fails to build here).
- Git remote: `https://github.com/Zafnok/tactical-rpg` (public).
