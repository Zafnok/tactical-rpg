# ADR-0002: Rust as the implementation language

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Requirements from Nick:

- Ship an executable on itch.io and Steam, Windows first, cross-platform preferred.
- Browser build (WASM) is desirable, not mandatory.
- Heavy automated testing (unit, mutation, integration) and CI gates, free tier only.
- SonarQube/SonarCloud is suggested.
- Code is written almost entirely by AI models and not reviewed by a human,
  so the toolchain itself must catch as many mistakes as possible.

## Decision

Use **Rust (stable)**, as a Cargo workspace. The toolchain version is pinned in
`rust-toolchain.toml` (ticket 0101).

## Consequences

- One codebase compiles to a single native `.exe` with no runtime to install,
  and to `wasm32-unknown-unknown` for the browser.
- The compiler and `clippy` reject whole classes of bugs (nulls, data races,
  unhandled enum cases) before tests even run — valuable when no human reviews.
- First-class free tooling: `cargo test`, `proptest`, `insta`, `cargo-mutants`,
  `cargo-llvm-cov`, `cargo-deny`. SonarCloud analyses Rust and imports Clippy
  and LCOV reports. CodeQL supports Rust.
- Steam integration exists via the `steamworks` crate.
- Compile times are slower than scripting languages; mitigated by CI caching and
  keeping heavy engines out (see ADR-0003).
- **Local environment note:** Nick's Windows machine uses the
  `x86_64-pc-windows-gnu` host toolchain (no MSVC Build Tools installed). CI and
  release builds use MSVC on GitHub's Windows runners. Nothing may depend on
  MSVC being present locally.

## Alternatives considered

- **C# + Godot** — solid editor, but Godot 4 cannot export C# projects to web,
  and an editor-centric workflow suits AI agents poorly (scene files, GUI state).
- **TypeScript (web-first) + Electron/Tauri for desktop** — easy web, but a
  ~100 MB Electron exe for an ASCII game is silly, and type safety is weaker.
- **C++** — no benefit over Rust here, and memory-safety bugs are exactly what
  unreviewed AI code would produce.
- **Python (pygame, tcod)** — packaging to a clean Windows exe and to the web is
  painful; weak static guarantees.
