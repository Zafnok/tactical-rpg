# ADR-0008: CI and quality gates (free tier only)

- **Status:** Superseded by ADR-0014
- **Date:** 2026-09-25

## Context

CI is the only reviewer. Nick requires free-tier tooling only (e.g. paid GitHub
Advanced Security is out). The repository `Zafnok/tactical-rpg` is **public**,
which matters: several tools that are paid for private repos are free for
public ones.

> Note on GitHub Advanced Security: GHAS is a paid product **for private
> repositories**. On public repositories, CodeQL code scanning, secret scanning
> and push protection are free. We use them only because this repo is public.
> If the repo is ever made private, remove CodeQL and rely on the rest.

## Decision

GitHub Actions (unlimited free minutes on public repos) runs the following.
"Required" means the check is part of branch protection on `main` (ticket 0106).

| Gate | Tool | Trigger | Required | Ticket |
| ---- | ---- | ------- | -------- | ------ |
| Formatting | `cargo fmt --check` | PR, push | ✅ | 0102 |
| Lints | `cargo clippy --all-targets -- -D warnings` (+ selected `pedantic` lints in workspace `Cargo.toml`) | PR, push | ✅ | 0102 |
| Tests (3 OS) | `cargo test --workspace` on `windows-latest`, `ubuntu-latest`, `macos-latest` | PR, push | ✅ | 0102 |
| WASM build | `cargo build --target wasm32-unknown-unknown -p trpg-app` | PR, push | ✅ | 0102 |
| Docs build | `cargo doc --no-deps` with `-D warnings` | PR, push | ✅ | 0102 |
| Licences (policy: ADR-0013), advisories, banned/duplicate crates, sources | `cargo-deny` | PR, push, weekly | ✅ | 0103 |
| Unused dependencies | `cargo-machete` | PR | ✅ | 0103 |
| Spelling | `typos` | PR | ✅ | 0103 |
| Workflow security | `zizmor` (GitHub Actions linter) | PR touching `.github/` | ✅ | 0103 |
| Code scanning | CodeQL (Rust + Actions) | PR, push, weekly | ✅ | 0103 |
| Supply-chain posture | OpenSSF Scorecard | push to main, weekly | ❌ (report only) | 0103 |
| Dependency updates | Dependabot (cargo + github-actions), grouped weekly | schedule | n/a | 0103 |
| Secret scanning + push protection | GitHub built-in (repo setting) | always | n/a | 0103 |
| Coverage + static analysis + quality gate | `cargo-llvm-cov` → SonarCloud (Clippy report imported) | PR, push | ✅ once stable | 0104 |
| Mutation testing | `cargo-mutants --in-diff` (PR), full run weekly | PR, weekly | ✅ | 0105 |
| Ticket hygiene | small script: ticket frontmatter valid; ticket referenced by branch/PR is moved to `tickets/done/` | PR | ✅ | 0106 |

Conventions for all workflows:

- Third-party actions are **pinned by commit SHA** with the version in a comment
  (zizmor and Scorecard both check this). Dependabot keeps them updated.
- Default `permissions: contents: read`; grant more per job only when needed.
- Use `Swatinem/rust-cache` for build caching.
- `concurrency` groups cancel superseded runs on the same PR.

## Consequences

- A PR that is merged has passed formatting, lints, tests on three OSes, a WASM
  build, security scans, coverage and mutation checks. That is the review.
- SonarCloud needs a one-time setup by Nick (ticket 0104 lists the clicks).
- CI time per PR will be several minutes; acceptable.

## Alternatives considered

- **Self-hosted SonarQube Community Build** — free, but needs a server that is
  always on. SonarCloud is free for public repos and needs no hosting.
- **Codecov** — fine, but SonarCloud already shows coverage alongside its
  analysis; one dashboard is enough.
- **Snyk / Socket** — free tiers exist but overlap with `cargo-deny` + Dependabot.
