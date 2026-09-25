---
id: "0104"
title: "CI: coverage (cargo-llvm-cov) and SonarCloud quality gate"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0102"]
nick_input: setup
completed:
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

- [ ] `coverage` job produces `lcov.info` in CI.
- [ ] SonarCloud shows the project with an analysis for the PR (decorates the PR).
- [ ] Clippy issues and coverage visible in SonarCloud.
- [ ] Badges in README render.
- [ ] Workflow still passes zizmor (if 0103 is merged).

## Tests required

Workflow run is the test; link the SonarCloud project URL in Completion notes.

## Completion notes

