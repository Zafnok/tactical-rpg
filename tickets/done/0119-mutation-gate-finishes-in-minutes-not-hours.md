---
id: "0119"
title: Make the PR mutation gate finish in minutes, not hours
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-03
---

# 0119 — Make the PR mutation gate finish in minutes, not hours

## Context

`mutants (diff)` in `.github/workflows/mutants.yml` (ticket 0105, ADR-0007,
ADR-0008) is a required check, and on any PR that touches a lot of
`crates/ui` it takes hours. It is the only slow job: the rest of CI is
done in about 10 minutes.

Measured from the job logs on 2026-10-02/03:

| PR (ticket) | Mutants | Time | Result |
| --- | --- | --- | --- |
| 0432 map scene and skin | 420 | 4 h 45 min | passed |
| 0408 preparations screen | 296 | 2 h 42 min | passed (after 5 cancelled runs, about 7 h of runner time thrown away) |
| 0711 PNG portraits | 147 | 43 min | passed |
| 0433 sprite map skin (PR #174) | 511 | about 4 h 20 min expected | 2 earlier runs cancelled at 2 h 9 min and 13 min |

Why:

1. **One mutant at a time, on one runner.** The job runs
   `cargo mutants --in-diff git.diff --in-place -vV`: no `--jobs`, no
   shards. Time grows in a straight line with the size of the diff.
2. **Each `crates/ui` mutant costs about 40 s**: 12–16 s to rebuild and
   24–27 s of tests. A *caught* mutant still pays nearly the whole `ui`
   test time, because `cargo test` runs every test in a program before
   reporting. `crates/core` is about 27 s per mutant (11 + 16),
   `crates/content` about 8 s.
3. **Every push starts again from nothing.** The workflow has
   `cancel-in-progress: true`, so merging `main` into a branch (0408 did
   it five times, 0433 three) throws the partial run away. Ticket 0117
   removed the *need* to merge `main` before merging the PR, but any
   review fix or conflict still restarts it.
4. **There is a ceiling.** GitHub kills a job at 6 h. At 40 s a mutant
   that is about 540 `ui` mutants: a PR a little larger than 0433 could
   never go green.

The weekly full run is already split into 4 shards; the PR run never was.

## Nick input

`None.`

## Scope

**In:**
- The `pr` job of `.github/workflows/mutants.yml`: run the diff's mutants
  in parallel shards, and make each mutant cheaper.
- A job time limit and a single required check name, so branch protection
  (ticket 0106) keeps working.
- ADR-0008's table row and the `run-gates` skill text, if what they say
  about the job changes.

**Out (do not do):**
- Weakening the gate: no new `exclude_globs`, no `#[mutants::skip]`, no
  lower `timeout_multiplier`, no "allowed missed" count, no making the
  check optional (CLAUDE.md hard rule 5).
- Running `cargo mutants` on Nick's machine to measure anything. Measure
  on CI only.
- The weekly full run, except where it shares a step with the PR job.
- Making the `ui` test suite itself faster (write a follow-up ticket if
  the numbers say it is worth it).

## Implementation steps

1. **Shard the PR job.** Turn `pr` into a matrix of 8 shards
   (`shard: [0, 1, 2, 3, 4, 5, 6, 7]`, `fail-fast: false`) running
   `cargo mutants --in-diff git.diff --in-place --shard ${{ matrix.shard }}/8 -vV`.
   `--shard` and `--in-diff` combine: the diff picks the mutants, the shard
   takes every 8th. Name the job `mutants (diff, shard N)` and the
   artifact `mutants-pr-<shard>-<run id>`. The repository is public, so
   the extra runners cost nothing.
2. **Skip the per-shard setup cost for small diffs.** Add a first job that
   computes the diff and runs `cargo mutants --in-diff git.diff --list`
   to count the mutants, and outputs the shard list: 1 shard up to about
   40 mutants, 4 up to about 160, otherwise 8. Feed it to the matrix with
   `fromJSON`. A diff with no Rust changes still ends green without
   building anything (as today's `diffcheck` step does).
3. **Keep one required check.** Add a final job `mutants-result`
   (`if: always()`, `needs` the shards) that fails unless every shard
   succeeded or was skipped, the same pattern as `ci-result` in
   `.github/workflows/ci.yml`. Tell Nick in the PR description which
   check name to require in branch protection if it differs from today's
   (that is a setup step for him: exact clicks, as ticket 0106 did).
4. **Stop a caught mutant paying for the whole test suite.** Try
   `--test-tool=nextest` (install `cargo-nextest` with
   `taiki-e/install-action`, same pinned step as `cargo-mutants`).
   nextest stops at the first failing test, so a caught mutant ends as
   soon as one test fails. Check that every test still passes under
   nextest first (it runs each test in its own process; `insta`
   snapshots and doc tests are the usual trouble: nextest does not run
   doc tests, so if any crate relies on them for the gate, keep
   `cargo test` for that crate or leave this step out and say why).
   Keep it only if the unmutated baseline passes and the measured time
   per mutant drops.
5. **Use the cores.** Hosted Linux runners for public repositories have 4
   cores. Try `--jobs 2` (it needs copies of the tree, so drop
   `--in-place` for that run) and keep whichever of `--in-place` or
   `--jobs 2` is faster per shard. Do not go above 2: cargo-mutants'
   own documentation warns that more gives timeouts.
6. **Set `timeout-minutes: 90` on the shard job**, so a hung run fails in
   an hour and a half, not six.
7. **Measure.** Open the PR with a throwaway commit that touches enough of
   `crates/ui/src/map_view/path.rs` to produce 80+ mutants (for example,
   re-indent nothing but add and remove a blank line inside each function
   body, or cherry-pick PR #174's diff onto a scratch branch) and write the
   before/after numbers in the completion notes: mutants, wall-clock time,
   seconds per mutant. Remove the throwaway commit before the PR is
   ready.
8. Update the mutation row in `docs/adr/0008-ci-quality-gates.md` and the
   "Mutation testing is not a local gate" paragraph in
   `.claude/skills/run-gates/SKILL.md` if job names or the command changed.
   If nextest becomes the test tool of the gate, record it with the
   `write-adr` skill (it changes how ADR-0007's layer 7 runs).

## Acceptance criteria

- [x] A PR whose diff gives 400+ `crates/ui` mutants gets its mutation
      result in under 45 minutes of wall-clock time (numbers in the
      completion notes).
- [x] A PR whose diff gives fewer than 40 mutants is no slower than today.
- [x] A docs-only PR and a PR with no `.rs` changes still pass without
      building.
- [x] A deliberately missed mutant (a throwaway commit that adds an
      untested `pub fn` returning a `bool` to `crates/core`) turns
      `mutants-result` red; the commit is removed afterwards.
- [x] The set of mutants tested is the same as before: the sum of
      "Found N mutants" over the shards equals `--list` on the whole
      diff.
- [x] `.cargo/mutants.toml` is unchanged except for options that do not
      remove mutants.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none (workflow change).
- Property: none.
- Snapshot / integration: the measured CI runs listed in the acceptance
  criteria, linked in the completion notes.

## Completion notes

**Done.** The PR run of `.github/workflows/mutants.yml` is now three jobs
(ADR-0047):

1. `mutants (diff, plan)` counts the diff's mutants with `--list` (no build)
   and picks 1 shard (up to 40 mutants), 4 (up to 160) or 8.
2. `mutants (diff, shard N)`: `cargo mutants --in-diff git.diff --jobs 2
   --shard N/n --sharding round-robin --test-tool=nextest`, 90-minute limit.
3. `mutants (diff)`: the aggregate, and the required check.

**Measured on CI, 2026-10-03** (throwaway draft PRs #178, #179, #180, all
closed and their branches deleted; nothing of them is in this PR):

| Run | Mutants | Wall-clock | Per mutant |
| --- | --- | --- | --- |
| Before (ticket table): 0432 | 420 | 4 h 45 min | about 40 s (`ui`: 12–16 s build + 24–27 s tests) |
| Before: 0433 (PR #174's diff) | 511 | about 4 h 20 min expected | about 30 s |
| After, PR #174's diff, nextest + `--in-place` ([run](https://github.com/Zafnok/visions-of-shuyi/actions/runs/37103328087)) | 511 | shards of 13–23 min | 17 s (`ui`: 13.6 s build + 7.7 s tests) |
| After, same diff, nextest + `--jobs 2`, the variant kept ([run, attempt 2](https://github.com/Zafnok/visions-of-shuyi/actions/runs/37103330073)) | 511 | **19 min** (07:46:52 to 08:05:53 UTC; shards of 12–18 min) | about 30 s each, two at a time (`ui`: 22 s build + 14 s tests) |
| After, 5 mutants, one of them an untested `pub fn -> bool` in `crates/core` ([run](https://github.com/Zafnok/visions-of-shuyi/actions/runs/37103332251)) | 5 | plan 15 s + one shard of 4 min | **red**, 5 missed |

- Same mutants as before: the plan job's `--list` says 511; the shards found
  64 × 7 + 63 = 511. Both variants reported the same 8 missed mutants.
- nextest was kept: tests per `ui` mutant fell from 24–27 s to 7.7 s, and the
  unmutated run passes under it on every shard.
- `--jobs 2` was kept over `--in-place`: on runners of the same speed, a shard
  of 64 took 17–20 min against 21–23 min.
- This PR itself has no `.rs` change: the plan job finds 0 mutants, no shard
  runs, and `mutants (diff)` is green without building.
- A diff under 40 mutants: one shard as before, plus about 20 s for the plan
  job; each mutant is cheaper, so it is not slower than before.

**Deviations from the plan**

- The aggregate job has the id `mutants-result` but the *name*
  `mutants (diff)`, the name the single job had. The required check in the
  "protect main" ruleset is therefore unchanged, and Nick has nothing to set
  up.
- `--sharding round-robin` is given explicitly: cargo-mutants' default is now
  `slice` (consecutive ranges), which is not what step 1 describes.
- The measurement used scratch draft PRs, not a throwaway commit on this
  branch, so this branch never carried one.
- The first measured runs took 67–74 min of wall-clock time, because three
  scratch PRs and PR #174's own run were competing for GitHub's 20 jobs at
  once: jobs waited up to 20 min for a runner. The 19 min above is the same
  run started again with the queue empty. Several large PRs open at once will
  still queue.

**For the next sessions**

- nextest does not run doc tests. `crates/ui` has 3 (`audio.rs` twice,
  `harness.rs`); they no longer help catch mutants. A mutant only a doc test
  would catch is reported as missed: stricter, not weaker.
- PR #174 (ticket 0433) has 8 missed mutants in its diff
  (`map_view/path.rs:28`, `map_view/sprite.rs:324` twice,
  `xtask/src/test_tileset.rs:135, 164, 164, 192, 192`). That is that PR's
  work; once it merges `main` it will get the answer in about 20 minutes.

**Follow-up tickets:** none. The `ui` test suite did not need its own ticket:
under nextest a caught mutant pays 7.7 s of tests, not the whole suite.

**Gameplay rules decided:** none (infra ticket).
