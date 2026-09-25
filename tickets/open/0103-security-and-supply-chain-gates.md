---
id: "0103"
title: "CI: security and supply-chain gates (cargo-deny, CodeQL, Dependabot, Scorecard, zizmor, typos, machete)"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0102"]
nick_input: setup
completed:
---

# 0103 — CI: security and supply-chain gates

## Context

The free security and hygiene gates from
[ADR-0008](../../docs/adr/0008-ci-quality-gates.md). CodeQL and secret
scanning are free here **only because the repo is public**.

## Nick input

**Setup (2 minutes):** in GitHub → `Zafnok/tactical-rpg` → **Settings → Code
security**, turn on **Secret Protection** (secret scanning) and **Push
protection**, and make sure **Dependabot alerts** is on. (Alternatively Nick
can tell the session "go ahead" and it runs the equivalent `gh api` command,
printing it first.)

## Scope

**In:** `deny.toml`, `_typos.toml`, `.github/dependabot.yml`,
`.github/workflows/security.yml` (deny, machete, typos, zizmor),
`.github/workflows/codeql.yml`, `.github/workflows/scorecard.yml`.

**Out:** Sonar (0104), mutants (0105), branch protection (0106).

## Implementation steps

1. **cargo-deny:** `cargo install --locked cargo-deny`, `cargo deny init`, then edit `deny.toml`:
   - `[licenses] allow = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib", "Unicode-3.0", "BSL-1.0", "CC0-1.0"]`
     — add others only if a real dependency needs them and they allow closed
     commercial distribution (no GPL/AGPL/LGPL). Note the reason in a comment.
   - `[advisories]`: default (deny vulnerabilities, warn unmaintained).
   - `[bans] multiple-versions = "warn"`, `wildcards = "deny"`.
   - `[sources] unknown-registry = "deny"`, `unknown-git = "deny"`.
   - `cargo deny check` must pass locally.
2. **cargo-machete:** `cargo install --locked cargo-machete`; `cargo machete` must pass.
3. **typos:** `_typos.toml` with `[default.extend-words]` for legitimate
   words it flags (e.g. `hjkl`). Run `typos` locally (`cargo install --locked typos-cli`).
4. **`.github/workflows/security.yml`** (`pull_request`, `push` to main, weekly
   `schedule`), jobs: `deny` (use `EmbarkStudios/cargo-deny-action`), `machete`
   (install via `taiki-e/install-action`), `typos` (`crate-ci/typos` action),
   `zizmor` (run `zizmor .github/workflows` via `uvx zizmor` or its official
   action; fix everything it reports at medium+ severity).
5. **CodeQL** `.github/workflows/codeql.yml`: languages `rust` and `actions`,
   `build-mode: none` for rust. Triggers: PR, push to main, weekly. Job
   permissions: `security-events: write`, `contents: read`.
6. **Scorecard** `.github/workflows/scorecard.yml`: official
   `ossf/scorecard-action` template, push to main + weekly, publish results,
   upload SARIF. Report only.
7. **Dependabot** `.github/dependabot.yml`: ecosystems `cargo` (`/`) and
   `github-actions` (`/`), weekly, each with a `groups:` entry grouping all
   minor/patch updates into one PR.
8. Pin all actions by SHA (see 0102 for how). Top-level `permissions: contents: read`.
9. Update `docs/adr/0008` **only** if a tool had to be swapped (write a
   superseding ADR in that case, per `write-adr`).

## Acceptance criteria

- [ ] `cargo deny check`, `cargo machete`, `typos` pass locally and in CI.
- [ ] zizmor reports no medium/high findings on all workflows.
- [ ] CodeQL runs on the PR and completes (Security tab shows the analysis).
- [ ] Scorecard workflow file present and valid (runs on main after merge).
- [ ] Dependabot config valid (GitHub shows it under Insights → Dependency graph → Dependabot).
- [ ] Nick's setup step done (or `gh api` run with his go-ahead).

## Tests required

Workflows are the tests. Link run URLs in Completion notes.

## Completion notes

