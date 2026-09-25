# ADR-0004: Crate layering and deterministic core

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Most of the game's value (and bugs) will live in its rules: movement, combat,
turn order, AI, progression. These must be heavily tested, including by
mutation testing, which needs fast pure code. Rendering and platform code are
hard to test and should be as thin as possible.

## Decision

A Cargo workspace with four crates under `crates/`. Dependencies flow one way:

```
app  ──►  ui  ──►  content  ──►  core
 │         └───────────────────►  core
 └──────────────────────────────► core
```

| Crate | Package name | Kind | Owns | Must NOT |
| ----- | ------------ | ---- | ---- | -------- |
| `crates/core` | `trpg-core` | lib | Grid, terrain rules, units, stats, pathfinding, combat, turn/phase system, commands & events, AI, progression, RNG | Do I/O, read files, use wall-clock time, depend on macroquad, `ui` or `content` |
| `crates/content` | `trpg-content` | lib | Data schemas (serde), parsers for map/dialogue/portrait files, validation, the embedded asset bundle | Contain game rules; depend on macroquad or `ui` |
| `crates/ui` | `trpg-ui` | lib | `GlyphBuffer`, colours, screens & screen stack, input `Action`s, key-repeat, menus, animations (driven by a `dt` argument), headless test `Harness` | Depend on macroquad; read the clock or keyboard directly |
| `crates/app` | `trpg-app` (binary `tactical-rpg`) | bin | Window, font atlas, blitting, raw keyboard → `ui` input, storage backend, audio, main loop | Contain game logic or screen logic |

Workspace-wide rules:

1. **Determinism.** `core` is fully deterministic. All randomness comes from a
   seeded RNG owned by the battle state (a small PCG-style generator; exact crate
   chosen in ticket 0304). Same seed + same commands ⇒ identical results. This
   enables replays, reproducible bug reports and exact tests.
2. **Commands in, events out.** Game state changes only through
   `core` `Command`s (e.g. `Move`, `Attack`, `Wait`, `EndTurn`). Applying a
   command validates it, mutates state, and returns a list of `Event`s
   (`UnitMoved`, `AttackResolved { hit, damage, crit }`, `UnitDied`, …). The UI
   animates events; it never computes outcomes itself. The AI issues the same
   commands as the player.
3. **Time is an argument.** Anything animated takes `dt` (seconds) as input.
   No crate other than `app` reads a clock.
4. **Assets are embedded.** `content` embeds `assets/` into the binary at compile
   time (e.g. `include_dir`). The exe is a single file and the WASM build needs
   no network fetches.
5. **No `unsafe`** outside `app` (and ideally not there either). Enforced with
   `#![forbid(unsafe_code)]` in `core`, `content`, `ui`.
6. **Errors:** libraries return `Result` with typed errors (`thiserror`);
   `app` may use `anyhow`. No `unwrap()`/`expect()` in library code outside
   tests, except where an invariant is documented in a comment.

A future `xtask` crate (tooling: wasm packaging, asset checks) is allowed when a
ticket needs it.

## Consequences

- `core`, `content` and `ui` can be tested at full speed with no window, which
  makes mutation testing and coverage practical.
- The GUI shell is small enough that manual smoke testing plus a CI build is
  acceptable for it.
- Contributors must resist the temptation to "just compute it in the UI".
  Reviewers (CI + the `work-ticket` skill checklist) enforce the table above.

## Alternatives considered

- **Single crate** — simpler at first but nothing stops rules leaking into
  rendering code, and mutation testing would crawl over UI code.
- **ECS (bevy_ecs, hecs)** — suited to real-time games with thousands of
  entities; a turn-based game with ~30 units is clearer as plain structs.
