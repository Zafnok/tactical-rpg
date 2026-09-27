# ADR-0023: A `debug-tools` cargo feature for the Pages build

- **Status:** Accepted
- **Date:** 2026-09-27
- **Related tickets:** 0402

## Context

Debug tools (the title screen's **Quick Battle** and the F12 glyph sampler)
are offered when `Ctx::debug_tools` is on, which was
`cfg!(debug_assertions)`. The GitHub Pages site is a **release** build
(`cargo xtask web --release`), so Nick couldn't reach Quick Battle from the
Pages link, and most battle-UI sign-offs (0402, 0404, 0408, 0410, 0414, 0502)
ask him to try things there. The shipped game (itch.io, Steam, the Windows
exe from `release.yml`) must not show debug tools.

## Decision

- `trpg-ui` has a cargo feature **`debug-tools`**; `trpg-app` has one of the
  same name that turns it on. `trpg_ui::screen::DEBUG_TOOLS` is
  `cfg!(any(debug_assertions, feature = "debug-tools"))` and is what
  `Ctx::new` sets `debug_tools` from.
- `cargo xtask web` takes `--debug-tools`, which builds with
  `--features debug-tools`.
- **Only `pages.yml`** passes it: `cargo xtask web --release --debug-tools`.
  `release.yml`, CI's wasm check and every shipped build (itch.io 0901,
  Steam 0903) build without it.
- So there are two release flavours: **shipped** (no debug tools) and
  **Pages preview** (release optimisations plus debug tools).

## Consequences

- Nick can open Quick Battle from the Pages link for sign-offs.
- The Pages build is not exactly what ships; anything that differs by
  `debug_tools` must be checked with it off too (tests already cover both,
  via `Ctx::debug_tools`).
- Publishing tickets must not copy the Pages command. `release.yml` stays
  without the flag.

## Alternatives considered

- **Build Pages in debug mode** — larger, slower wasm, and it would also turn
  on every other `debug_assertions` check (e.g. the storage smoke check),
  so it wouldn't show the real game's speed.
- **A runtime switch (URL parameter or key combo)** — the code would ship in
  every build and players could find it.
- **An environment variable read at build time** — works, but a cargo
  feature is the standard, visible way and `cargo tree -e features` shows it.
