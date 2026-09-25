# ADR-0005: Data-driven content formats

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Classes, units, items, terrain, maps, dialogue and portraits will be authored
mostly by AI sessions and tweaked after Nick's playtests. Changing a number or a
line of dialogue should never require touching Rust code, and bad data should
fail CI, not crash the game.

## Decision

All content lives under `assets/` and is loaded by the `content` crate.

| Content | Format | Location (planned) | Why |
| ------- | ------ | ------------------ | --- |
| Terrain, classes, items/weapons, unit templates, palette, keymap defaults | **RON** (`.ron`) via `serde` | `assets/data/` | Rust-native, supports comments, enums and structs cleanly |
| Battle maps | **Plain-text ASCII grid + RON header** (`.map`) | `assets/maps/` | Authors (human or LLM) can *see* the map; diff-friendly |
| Dialogue / cutscenes | **Line-based script** (`.dlg`), format defined in ticket 0702 | `assets/dialogue/` | Prose-first so LLM writers produce it naturally; trivial to parse |
| Portraits | **Text art file** (`.portrait`): glyph layer + colour layer per expression | `assets/portraits/` | ASCII art is authored as text; colour kept separate for readability |
| Fonts | Bitmap PNG atlas or TTF with licence file beside it | `assets/fonts/` | Licence travels with the font |

Rules:

1. Every content type has a **validator** in `content` that reports all errors
   with file name + line/column + a human message (not just the first error).
2. Cross-references are by **string id** (`class: "myrmidon"`), resolved and
   checked at load time. Unknown ids are errors.
3. A test in `content` loads the entire embedded asset bundle and runs every
   validator. CI therefore fails on any broken asset.
4. Numbers that are balance knobs (growth rates, weapon might, AI weights) live
   in data, not code.
5. Schema changes update every existing asset in the same PR.

## Consequences

- Balance changes after playtests are data edits — cheap tickets for cheaper models.
- LLM story sessions write `.dlg` files directly; no conversion step.
- We maintain a few small hand-written parsers (maps, dialogue, portraits); each
  gets unit tests, property tests and ideally a fuzz target later.

## Alternatives considered

- **JSON** — no comments, noisy for hand editing.
- **TOML** — awkward for nested/enum-heavy data like class definitions.
- **Tiled editor (`.tmx`) maps** — great tool, but binary-ish XML is hard for LLMs
  to author and review; our maps are small grids.
- **Ink / Yarn Spinner for dialogue** — powerful, but we need a small subset
  (speakers, portraits, expressions, triggers) and a Rust runtime for either adds
  weight. Revisit if branching dialogue becomes a big feature.
