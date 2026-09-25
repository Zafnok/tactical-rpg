# ADR-0014: CI quality gates, skipping heavy jobs on docs-only PRs

- **Status:** Accepted
- **Date:** 2026-09-25
- **Related tickets:** 0109, 0106
- **Supersedes:** ADR-0008

## Context

ADR-0008 set up the CI gates and ran all of them on every pull request.
Many tickets (all `00xx` design decisions, most `07xx` story work) change only
Markdown, yet each such PR ran tests on three OSes, a WASM build, coverage,
mutation testing and CodeQL (`analyze (rust)` alone takes about 2.5 min). CodeQL
on a PR always analyses the whole codebase; main's baseline is only used to show
*new* alerts, never to skip work.

Two GitHub behaviours constrain the fix:

1. A workflow skipped by an `on: pull_request: paths` / `paths-ignore` filter
   **reports no status at all**. If that check is required (ticket 0106), the PR
   waits forever and cannot be merged. A *job* skipped by `if:` does report
   (as "skipped"), and a skipped check satisfies branch protection.
2. A **matrix** job skipped by `if:` reports one check with the unexpanded name,
   e.g. `test (${{ matrix.os }})`, never the per-leg names such as
   `test (windows-latest)` (seen on the `mutants (full, …)` jobs on
   Zafnok/tactical-rpg#11). A required per-leg check would therefore also wait
   forever.

Everything else in ADR-0008 still holds: free-tier only; CodeQL and secret
scanning are free only because the repo is public. If the repo is ever made
private, remove CodeQL.

## Decision

### Docs-only detection

- `.github/workflows/changes.yml` is a reusable workflow (`on: workflow_call`)
  with one job, `detect`, and one output, `code`.
- On `pull_request`, it checks out the PR merge commit with `fetch-depth: 2` and
  lists `git diff --name-only --no-renames HEAD^1 HEAD` (the first parent of the
  merge commit is the base branch tip). `--no-renames` makes a move such as
  `crates/x.rs → docs/x.md` list both paths, so it counts as code.
- **Docs-only paths:** `*.md` anywhere, `docs/**`, `tickets/**`, `.claude/**`.
  `.claude/**` counts as docs: it holds agent skills (Markdown) and local agent
  settings, and no CI job reads it. Anything else (including `.github/**`,
  `Cargo.*`, `deny.toml`, `_typos.toml`) is code.
- On **any other event** (`push` to `main`, `schedule`, `workflow_dispatch`)
  `code` is always `true`, so main's CodeQL baseline, the weekly scans and the
  weekly mutation run stay complete.

### Gating

Each heavy workflow calls it as a job named `changes` and heavy jobs get:

```yaml
needs: changes
if: needs.changes.outputs.code == 'true'
```

Never use workflow-level `paths` / `paths-ignore` on a workflow with a required
check.

A workflow with a matrix job also gets an aggregate job (`ci-result`,
`codeql-result`): `if: always()`, `needs:` every other job in the workflow
(including `changes`), fails if any result is `failure` or `cancelled`, passes
on `success`/`skipped`. **The aggregate is the required check**, not the matrix
legs. Any new job added to such a workflow must be added to its aggregate's
`needs`.

### Gates

"Required" means the check is part of branch protection on `main` (ticket 0106).
"Docs-only PR" says what happens on a PR that only touches docs-only paths.

| Gate | Tool | Trigger | Docs-only PR | Required check | Ticket |
| ---- | ---- | ------- | ------------ | -------------- | ------ |
| Formatting | `cargo fmt --check` | PR, push | skipped | via `ci-result` | 0102 |
| Lints | `cargo clippy --all-targets -- -D warnings` (+ selected `pedantic` lints) | PR, push | skipped | via `ci-result` | 0102 |
| Tests (3 OS) | `cargo test --workspace` on `windows-latest`, `ubuntu-latest`, `macos-latest` | PR, push | skipped | via `ci-result` | 0102 |
| WASM build | `cargo build --target wasm32-unknown-unknown -p trpg-app` | PR, push | skipped | via `ci-result` | 0102 |
| Docs build | `cargo doc --no-deps` with `-D warnings` | PR, push | skipped | via `ci-result` | 0102 |
| Coverage + static analysis | `cargo-llvm-cov` → SonarCloud (Clippy report imported) | PR, push | skipped | via `ci-result` | 0104 |
| Licences (ADR-0013), advisories, bans, sources | `cargo-deny` | PR, push, weekly | skipped | `deny` | 0103 |
| Unused dependencies | `cargo-machete` | PR, push, weekly | skipped | `machete` | 0103 |
| Spelling | `typos` | PR, push, weekly | **runs** (useful for Markdown) | `typos` | 0103 |
| Workflow security | `zizmor` (medium+) | PR, push, weekly | skipped (`.github/**` is code) | `zizmor` | 0103 |
| Code scanning | CodeQL (Rust + Actions) | PR, push, weekly | skipped | `codeql-result` | 0103 |
| Mutation testing | `cargo-mutants --in-diff` (PR), full run weekly | PR, weekly | skipped | `mutants (diff)` | 0105 |
| Supply-chain posture | OpenSSF Scorecard | push to main, weekly | n/a | ❌ (report only) | 0103 |
| Dependency updates | Dependabot (cargo + github-actions), grouped weekly | schedule | n/a | n/a | 0103 |
| Secret scanning + push protection | GitHub built-in (repo setting) | always | n/a | n/a | 0103 |
| Ticket hygiene | `cargo xtask ticket-lint` | PR, push | should run (tickets are docs) | `tickets` | 0106 |

Do not require checks that are not produced by a job in these workflows (for
example the code-scanning `CodeQL` / `zizmor` result checks, or a SonarCloud app
check): they are not posted when their job is skipped.

Conventions for all workflows (unchanged from ADR-0008):

- Third-party actions are **pinned by commit SHA** with the version in a comment
  (zizmor and Scorecard both check this). Dependabot keeps them updated.
- Default `permissions: contents: read`; grant more per job only when needed.
- Checkouts use `persist-credentials: false`; `${{ }}` expressions go through
  `env:`, never directly into `run:`. `zizmor` reports nothing at medium+.
- Use `Swatinem/rust-cache` for build caching.
- `concurrency` groups cancel superseded runs on the same PR.

## Consequences

- Docs-only PRs finish in well under a minute (`changes`, `typos`, the
  aggregates) instead of several minutes, and are still mergeable with every
  required check green or skipped.
- Every merged code PR has still passed formatting, lints, tests on three OSes,
  a WASM build, security scans, coverage and mutation checks. That is the review.
- A docs-only PR is not scanned by CodeQL; main is, on every push and weekly.
- The docs-only path list lives in one file (`changes.yml`). If a CI job ever
  starts reading Markdown or `.claude/**` (e.g. docs tests, a Markdown linter),
  that job must not be gated, or the path list must change.
- Adding a job to `ci.yml` or `codeql.yml` means adding it to the aggregate's
  `needs`, otherwise it is not enforced.

## Alternatives considered

- **Workflow-level `paths-ignore`** — simplest, but skipped workflows report no
  status and required checks block the merge forever.
- **Skip steps instead of jobs** (`if:` on every step) — per-leg check names keep
  expanding, but each leg still boots a runner (Windows and macOS are slow to
  start), and a green check that did nothing is misleading.
- **`dorny/paths-filter` or `tj-actions/changed-files`** — well known, but a
  third-party action (and `tj-actions/changed-files` was compromised in 2025) for
  what is a five-line `git diff`.
- **Requiring each matrix leg by name** — breaks on docs-only PRs (behaviour 2
  above).
