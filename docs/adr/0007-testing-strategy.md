# ADR-0007: Testing strategy

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

No human reviews the code. Tests and CI are the review. Nick explicitly wants
unit, mutation and integration testing "as many variants as needed".

## Decision

Seven layers, each with a clear job:

| # | Layer | Tool | Where | What it proves |
| - | ----- | ---- | ----- | -------------- |
| 1 | **Unit tests** | `cargo test` | `#[cfg(test)] mod tests` beside the code | Individual functions behave as specified |
| 2 | **Property tests** | [`proptest`](https://docs.rs/proptest) | `core`, `content` parsers | Invariants hold for *all* inputs (e.g. pathfinding never exceeds move points; hit chance always in 0..=100; stats never exceed caps; parse∘print round-trips) |
| 3 | **Snapshot tests** | [`insta`](https://insta.rs) | `ui` | A screen's `GlyphBuffer` renders exactly as approved. Snapshots are text (glyphs) plus a compact colour layer, so diffs are readable in PRs |
| 4 | **Scripted integration tests** | `cargo test` + `ui::Harness` | `crates/ui/tests/` | Feeding a sequence of `Action`s (e.g. `"l l l f j f f"`) through real screens produces the expected state and screen. This is how "play the game" is tested headlessly |
| 5 | **Replay / determinism tests** | `cargo test` | `crates/core/tests/` | Same seed + same commands ⇒ byte-identical event log. Also: a recorded winning playthrough of each chapter still wins (catches balance/logic regressions that make a map unwinnable) |
| 6 | **Content validation** | `cargo test` | `crates/content/tests/` | Every embedded asset parses and all cross-references resolve |
| 7 | **Mutation testing** | [`cargo-mutants`](https://mutants.rs) | CI | The tests actually detect bugs: if flipping `<` to `<=` in combat code doesn't fail a test, the tests are too weak |

Supporting measures:

- **Coverage** via `cargo-llvm-cov`, uploaded to SonarCloud (ADR-0008). Targets:
  `core` ≥ 85% lines, `content` ≥ 80%, `ui` ≥ 70%. `app` is excluded.
- **Mutation gate:** on pull requests, `cargo mutants --in-diff` over changed
  code in `core`, `content`, `ui` — **no missed mutants** allowed in new code
  (use `#[mutants::skip]` with a justification comment only for genuinely
  untestable code such as `Debug` impls). A weekly scheduled full run publishes
  a report as a CI artifact.
- **Fuzzing** (`cargo-fuzz`) of the map/dialogue/portrait parsers is a later
  nice-to-have; it needs nightly Rust, so it stays out of required gates.
- The `app` crate is not unit tested. It is covered by: CI builds on all
  platforms + WASM, and a manual smoke checklist in the release ticket.

### Rules for every ticket

1. New logic ships with unit tests; rules code in `core` also needs at least one
   property test per invariant stated in its ticket.
2. New screens ship with at least one snapshot and one scripted integration test.
3. Bugs are fixed test-first: write the failing test that reproduces the bug,
   then fix it.
4. Tests must not depend on wall-clock time, real files outside `assets/`, or
   unseeded randomness.

## Consequences

- The `ui::Harness` (ticket 0205) is critical infrastructure; most later
  tickets depend on it.
- Snapshot updates must be reviewed by the model in the diff (`cargo insta review`
  is interactive, so use `cargo insta accept` only after reading the `.snap.new`
  files).
- Mutation testing is slow; restricting PR runs to the diff keeps CI reasonable.

## Alternatives considered

- **Coverage-only gate** — coverage shows code was *run*, not that results were
  *checked*. Mutation testing closes that gap.
- **End-to-end GUI tests (screenshot comparison of the real window)** — flaky
  across GPUs/OSes. The `GlyphBuffer` snapshot gives the same confidence without
  pixels.
