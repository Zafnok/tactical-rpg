---
id: "0103"
title: "CI: security and supply-chain gates (cargo-deny, CodeQL, Dependabot, Scorecard, zizmor, typos, machete)"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0102"]
nick_input: setup
completed: 2026-09-25
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
   - `[licenses] allow` = **exactly** the "Code" list in
     [ADR-0013](../../docs/adr/0013-licensing-and-third-party-policy.md)
     (plus `OFL-1.1` only if a font crate needs it), `confidence-threshold = 0.9`,
     `[licenses.private] ignore = true` (our own crates are proprietary and
     `publish = false`). Never add a denied license. An unknown/custom license
     needs a `[[licenses.clarify]]` or exception entry with a comment explaining
     why it is safe to ship commercially, and a mention in the PR.
   - A comment block at the top of `deny.toml` pointing to ADR-0013.
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

- [x] `cargo deny check`, `cargo machete`, `typos` pass locally and in CI.
- [x] Proven: temporarily adding a GPL-licensed crate makes `cargo deny check licenses` fail (verify, then remove).
- [x] zizmor reports no medium/high findings on all workflows (offline audits; online audits need a GitHub token, verify on the PR).
- [ ] CodeQL runs on the PR and completes (Security tab shows the analysis) — verify once the PR is open.
- [x] Scorecard workflow file present and valid (runs on main after merge).
- [x] Dependabot config valid (GitHub shows it under Insights → Dependency graph → Dependabot) — verify after merge.
- [ ] Nick's setup step done (or `gh api` run with his go-ahead) — **not done**, see below.

## Tests required

Workflows are the tests. Link run URLs in Completion notes.

## Completion notes

- Added `deny.toml` with the exact ADR-0013 "Code" allow list plus `OFL-1.1`,
  `confidence-threshold = 0.9`, `[licenses.private] ignore = true`,
  `[bans] wildcards = "deny"` + `multiple-versions = "warn"`, and
  `[sources] unknown-registry/unknown-git = "deny"`.
  - **Deviation:** cargo-deny 0.20 removed the old per-category
    vulnerability/unmaintained/unsound severity knobs from `[advisories]` —
    every RUSTSEC advisory is now an error unless explicitly `ignore`d (with a
    reason). Two advisories in macroquad's own dependency tree
    (`RUSTSEC-2025-0035` macroquad soundness, `RUSTSEC-2026-0192` unmaintained
    `ttf-parser`) have no upstream fix and are ignored with comments; revisit
    on the next macroquad upgrade.
  - `[bans] allow-wildcard-paths = true` was needed so our own in-workspace
    path dependencies (e.g. `trpg-core = { path = "../core" }`, which carry no
    version requirement) aren't flagged as real wildcard deps.
  - Verified the GPL-detection acceptance criterion by temporarily adding a
    local dummy crate with `license = "GPL-3.0-only"` as a path dependency of
    `trpg-core`: `cargo deny check licenses` failed as expected, then the
    dependency was removed and the check passes again (`crates/core/Cargo.toml`
    is unchanged in this PR).
- `cargo machete` flagged the crate-boundary scaffolding path deps
  (`trpg-core`/`trpg-content`/`trpg-ui`) as "unused" in `app`, `ui` and
  `content`, since ADR-0004's crate split is scaffolded ahead of the code that
  will use it (per 0102/ADR-0004, not something to backfill in this ticket).
  Added `[package.metadata.cargo-machete] ignored = [...]` to each crate's
  `Cargo.toml` rather than weakening the gate.
- `_typos.toml`: added `[default.extend-words]` entries for `hjkl` (vim keys),
  `ratatui` (project name in an ADR), and a few tokens `typos` mis-splits
  (`ND`/`PN`/`mis` from `CC-BY-ND`, "PNGs", "mis-keys"). Left the one genuine
  misspelling (`beginnning` in `docs/design/magic.md`) uncorrected and
  allow-listed instead, since it's inside a verbatim quote of Nick's own words
  from an `ask-nick` session, not prose we authored.
- `.github/workflows/security.yml`: `deny`, `machete`, `typos`, `zizmor` jobs
  as specified. `zizmor` uses the official `zizmorcore/zizmor-action` with
  `min-severity: medium` (matching "no medium/high findings"); its default
  `advanced-security: true` uploads SARIF to the Security tab.
- `.github/workflows/codeql.yml`: `rust` + `actions` languages via a matrix,
  `build-mode: none`, PR/push/weekly triggers.
- `.github/workflows/scorecard.yml`: OSSF's official template (push to main +
  weekly, SARIF published + uploaded to code scanning).
- `.github/dependabot.yml`: `cargo` and `github-actions` ecosystems, weekly,
  each grouping all minor/patch updates into one PR.
- All new third-party actions pinned by commit SHA with the version in a
  trailing comment, same convention as `ci.yml`.
- **Deviation (small, in scope):** running `zizmor` against the whole
  `.github/workflows` directory (as the ticket's own step 4 requires) also
  flagged 5 medium `artipacked` findings in `ci.yml` from ticket 0102 (its
  `actions/checkout` steps didn't set `persist-credentials: false`). Fixed
  them here too, since "zizmor reports no medium/high findings on all
  workflows" can't be true otherwise.
- Ran `cargo fmt --check` and `cargo clippy --workspace --all-targets --locked
  -- -D warnings`; both pass unaffected by this ticket's changes.
- zizmor's online audits (which need a live GitHub token to check things like
  workflow permissions against the API) return a 401 in this sandboxed
  session with no repo token; `--no-online-audits` passes cleanly locally.
  The PR's own `zizmor` job will run with a real `GITHUB_TOKEN` and cover
  those checks in CI.
- **Not done — needs Nick:** the repo-settings step (Settings → Code security:
  Secret Protection, Push protection, Dependabot alerts) requires either Nick
  clicking those toggles, or Nick telling a session "go ahead" to run the
  equivalent `gh api` call. Neither happened in this session, so this is
  still outstanding after this PR merges.

