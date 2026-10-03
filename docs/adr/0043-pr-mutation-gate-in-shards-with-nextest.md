# ADR-0043: The PR mutation gate runs in shards, with nextest as its test runner

- **Status:** Accepted
- **Date:** 2026-10-03
- **Related tickets:** 0119, 0105
- **Related ADRs:** ADR-0007 (layer 7), ADR-0008, ADR-0014

## Context

The PR mutation gate (ADR-0007 layer 7, ADR-0008) was one job running
`cargo mutants --in-diff git.diff --in-place`: one mutant at a time on one
runner. A `crates/ui` mutant cost about 40 s (12–16 s to rebuild, 24–27 s of
tests), because `cargo test` runs every test of a test program before it
reports, even when the first one fails. A PR with 420 mutants took 4 h 45 min,
and GitHub kills a job at 6 h, so a PR with about 540 `ui` mutants could never
go green. Everything else in CI finishes in about 10 minutes.

Measured on CI on 2026-10-03 with the diff of PR #174 (511 mutants, 473 of
them built and tested), 8 shards:

| Variant | Build per mutant | Tests per mutant | One shard of 64 mutants |
| --- | --- | --- | --- |
| Before: `cargo test`, `--in-place`, no shards | 12–16 s | 24–27 s (`ui`) | (whole diff: over 4 h) |
| nextest, `--in-place` | 10 s | 7 s | 21–23 min |
| nextest, `--jobs 2` | 17 s | 13 s, two at a time | 17–20 min |

Both nextest variants found the same 8 missed mutants.

## Decision

- The PR run of `.github/workflows/mutants.yml` is three jobs:
  1. `mutants (diff, plan)` counts the mutants of the diff with
     `cargo mutants --in-diff git.diff --list` (nothing is built) and picks
     the number of shards: **1 up to 40 mutants, 4 up to 160, otherwise 8**.
     With no mutants, no shard runs.
  2. `mutants (diff, shard N)`, a matrix, each running
     `cargo mutants --in-diff git.diff --jobs 2 --shard N/n --sharding round-robin --test-tool=nextest`,
     with `timeout-minutes: 90`.
  3. `mutants (diff)`, the aggregate of ADR-0014 (`if: always()`, fails on any
     `failure` or `cancelled`). **It is the required check**, under the name
     the single job had, so the ruleset did not change.
- **`cargo-nextest` is the test runner of the PR mutation gate.** It stops at
  the first failing test, so a caught mutant ends early. nextest does not run
  doc tests: a mutant that only a doc test would catch is reported as missed,
  and the fix is a unit test. That makes the gate stricter, never weaker.
- `--jobs 2`, never more (cargo-mutants warns that more gives timeouts). It
  cannot be combined with `--in-place`; each job builds in its own copy.
- `.cargo/mutants.toml` is unchanged: the same mutants are tested, under the
  same `timeout_multiplier`.
- `cargo test` stays the test runner everywhere else: the `test` jobs of
  `ci.yml`, the local gates, and the weekly full mutation run.

## Consequences

- A 511-mutant PR gets its result in about 25 minutes of runner time per
  shard instead of over 4 hours; the ceiling moves from about 540 `ui`
  mutants to several thousand.
- A PR uses up to 9 runners for the gate. The repository is public, so they
  are free, but GitHub runs at most 20 jobs at once for the account: with
  several large PRs open, shards wait in the queue.
- Every shard pays about 4 minutes for its own build and unmutated test run,
  which is why small diffs get one shard.
- The 3 doc tests in `crates/ui` no longer count towards catching mutants.
- The tests must pass under nextest (each test in its own process). They do
  today; a test that leans on state shared with another test in the same
  program would fail the shard's unmutated run.
- A new job added to the PR run must be added to the `needs` of
  `mutants (diff)` (ADR-0014).

## Alternatives considered

- **Shards only, keeping `cargo test`** — 8 shards of 64 `ui` mutants at 40 s
  is 43 minutes each, and still grows twice as fast with the diff.
- **nextest with `--in-place`** — about 20% slower per shard than `--jobs 2`
  on like-for-like runners.
- **More than 8 shards** — each adds 4 minutes of setup and takes a runner
  from the other PRs; revisit if diffs of 1000+ mutants become usual.
- **`--sharding slice`** (the default) — puts neighbouring mutants on the same
  shard, so a diff that is mostly `ui` plus a little `content` gives shards of
  very different length. Round-robin spreads the slow crate evenly.
- **Excluding files or raising the allowed misses** — weakens the gate
  (CLAUDE.md hard rule 5).
