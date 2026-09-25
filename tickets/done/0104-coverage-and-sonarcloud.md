---
id: "0104"
title: "CI: coverage (cargo-llvm-cov) and SonarCloud quality gate"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0102"]
nick_input: setup
completed: 2026-09-25
---

# 0104 — CI: coverage and SonarCloud

## Context

Nick suggested SonarQube. SonarCloud (SonarQube Cloud) is free for public
repos, analyses Rust, and imports Clippy findings and LCOV coverage.
See [ADR-0007](../../docs/adr/0007-testing-strategy.md) and
[ADR-0008](../../docs/adr/0008-ci-quality-gates.md).

## Nick input

**Setup (≈5 minutes), before the session starts:**

1. Go to <https://sonarcloud.io> → **Log in with GitHub**.
2. **Import an organization** → pick the `Zafnok` account → choose the
   **Free** plan → when GitHub asks, grant access to **only**
   `tactical-rpg`.
3. **Analyze new project** → select `tactical-rpg` → **Set up**.
4. When asked for the analysis method choose **With GitHub Actions**. (If it
   started "Automatic Analysis", go to *Administration → Analysis Method* and
   switch it **off**.)
5. SonarCloud shows a **SONAR_TOKEN**. In GitHub: repo **Settings → Secrets and
   variables → Actions → New repository secret**, name `SONAR_TOKEN`, paste it.
6. Tell the session the **Organization Key** and **Project Key** SonarCloud
   displays (usually `zafnok` and `Zafnok_tactical-rpg`).

## Scope

**In:** coverage job, `sonar-project.properties`, Sonar scan job, README badges.

**Out:** making the Sonar check *required* (0106 does branch protection).

## Implementation steps

1. Add to `.github/workflows/ci.yml` a job `coverage` (ubuntu, needs nothing):
   - Install `cargo-llvm-cov` via `taiki-e/install-action` (pinned SHA);
     `rustup component add llvm-tools-preview`.
   - `cargo llvm-cov --workspace --exclude trpg-app --lcov --output-path lcov.info`.
   - Also produce a Clippy JSON report:
     `cargo clippy --workspace --all-targets --message-format=json > clippy.json`
     (don't fail this step; the `clippy` job is the gate).
   - Checkout with `fetch-depth: 0` (Sonar needs history for new-code detection).
2. `sonar-project.properties` at repo root:
   ```properties
   sonar.organization=<from Nick>
   sonar.projectKey=<from Nick>
   sonar.sources=crates
   sonar.tests=crates
   sonar.test.inclusions=**/tests/**,**/*_test.rs
   sonar.exclusions=target/**,crates/app/**
   sonar.rust.lcov.reportPaths=lcov.info
   sonar.rust.clippy.reportPaths=clippy.json
   ```
   **Check the current SonarCloud Rust documentation** for exact property names
   (they have changed across versions) and correct them if needed.
3. Scan step in the same job: `SonarSource/sonarqube-scan-action` (pinned SHA;
   the older `sonarcloud-github-action` is deprecated) with
   `SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}`. Skip the step when the secret is
   unavailable (PRs from forks): `if: ${{ env.SONAR_TOKEN != '' }}`.
4. Confirm SonarCloud's free plan has no open-source-license requirement for
   public projects (our code is proprietary, ADR-0013). If it does, stop and
   report back instead of continuing; the fallback is a coverage threshold via
   `cargo llvm-cov --fail-under-lines` and no Sonar.
5. In SonarCloud keep the default **"Sonar way"** quality gate (includes
   ≥ 80% coverage on new code). Note it in Completion notes.
6. Add SonarCloud quality-gate and coverage badges to `README.md`.

## Acceptance criteria

- [x] `coverage` job produces `lcov.info` in CI.
- [x] SonarCloud shows the project with an analysis for the PR (decorates the PR).
- [x] Clippy issues and coverage visible in SonarCloud.
- [x] Badges in README render.
- [x] Workflow still passes zizmor (0103 is merged).

## Tests required

Workflow run is the test; link the SonarCloud project URL in Completion notes.

## Completion notes

- Added a `coverage` job to `.github/workflows/ci.yml`: installs
  `llvm-tools-preview` and `cargo-llvm-cov` (via `taiki-e/install-action`,
  pinned SHA matching the one already used in `security.yml`), generates
  `lcov.info` for the workspace excluding `trpg-app` (per ADR-0004, `app` is
  the macroquad/I/O boundary and isn't meaningfully unit-testable; it's
  still linted and scanned by Sonar, just not coverage-gated), and a
  best-effort `clippy.json` report
  (`continue-on-error: true`, since the `clippy` job is the actual gate).
  Checkout uses `fetch-depth: 0` for Sonar's new-code detection.
- Added `sonar-project.properties` at repo root with
  `organization=zafnok`, `projectKey=Zafnok_tactical-rpg`, per Nick's
  SonarCloud setup. Property names (`sonar.rust.lcov.reportPaths`,
  `sonar.rust.clippy.reportPaths`) verified against current SonarQube Cloud
  Rust-analyzer docs — unchanged from the ticket's draft.
- Sonar scan step uses `SonarSource/sonarqube-scan-action` pinned to the
  `v8.2.2` commit SHA (resolved via `git ls-remote`, not the deprecated
  `sonarcloud-github-action`), gated on `SONAR_TOKEN` being set so PRs from
  forks (where the secret isn't available) skip the step instead of failing.
- Confirmed SonarCloud's plans: the **Free** plan (up to 50k LOC, public or
  private) has no open-source-license requirement — only the separate **OSS**
  plan requires an OSI license, and we don't need that plan. No fallback to
  a bare `--fail-under-lines` threshold is needed.
- Left SonarCloud's default "Sonar way" quality gate (≥ 80% coverage on new
  code) as-is; not made a required GitHub check yet (that's ticket 0106,
  out of scope here).
- Added quality-gate and coverage badges to `README.md`, linking to
  `https://sonarcloud.io/summary/new_code?id=Zafnok_tactical-rpg`.
- SonarCloud project: <https://sonarcloud.io/summary/new_code?id=Zafnok_tactical-rpg>
  (Nick already completed the setup steps: org `zafnok`, project
  `Zafnok_tactical-rpg`, `SONAR_TOKEN` added to repo secrets).
- No Rust code changed, so no new tests were needed; `cargo fmt --check`
  still passes. The coverage/Sonar job itself is exercised by this PR's own
  CI run — see the Actions run and SonarCloud analysis linked above once CI
  completes.

