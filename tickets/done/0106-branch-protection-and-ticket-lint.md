---
id: "0106"
title: "Repo guardrails: PR template, ticket lint (xtask), branch protection"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0102", "0103", "0104", "0105", "0109"]
nick_input: setup
completed: 2026-09-25
---

# 0106 — Repo guardrails: PR template, ticket lint, branch protection

## Context

Makes the ticket workflow ([ADR-0010](../../docs/adr/0010-ticket-workflow-and-model-routing.md))
enforced by CI, and makes all gates *required* before merging to `main`.

## Nick input

**Setup:** approve the branch-protection change. The session prints the exact
`gh api` command (a repository ruleset) and runs it after Nick says OK. Nick
can instead click it in **Settings → Rules → Rulesets**, following the list below.

## Scope

**In:** `.github/pull_request_template.md`, a `ticket-lint` command in the
existing `crates/xtask` (created by 0101), a CI job running it, a repository ruleset for `main`,
repo merge settings.

**Out:** changing any gate's behaviour.

## Implementation steps

1. **PR template** `.github/pull_request_template.md`:
   ```markdown
   ## Ticket
   [NNNN] — link to tickets/done/NNNN-….md

   ## Summary
   -

   ## Checklist
   - [ ] Only this ticket's scope (follow-ups filed as new tickets: …)
   - [ ] Acceptance criteria all met (or explained below)
   - [ ] `run-gates` passed locally
   - [ ] Ticket moved to `tickets/done/` with Completion notes

   ## Nick input
   None / Sign-off: …
   ```
2. The `xtask` crate exists from 0101. Put the lint logic in
   `crates/xtask/src/tickets.rs`. Exclude xtask from Sonar coverage only if
   noisy; keep it in mutation testing (the lint rules deserve real tests).
3. `cargo xtask ticket-lint [--pr-branch <name>]` checks and prints *all* errors:
   - Every `tickets/{open,done}/*.md` (except `README.md`) is named
     `^\d{4}-[a-z0-9-]+\.md$`.
   - Frontmatter (YAML between `---` lines) parses; required keys present:
     `id, title, type, milestone, model, effort, status, blocked_by, nick_input`.
   - `id` equals the filename prefix; ids are unique across both folders.
   - `type`, `model`, `effort`, `status`, `nick_input` take values listed in
     `tickets/README.md`.
   - Files in `done/` have `status: done` and a `completed:` date; files in
     `open/` do not have `status: done`.
   - Every `blocked_by` id exists.
   - With `--pr-branch t0304-foo`: ticket `0304` must be in `done/`.
   Use `serde_yaml`-compatible parsing (e.g. `serde_yml` or `serde_norway`;
   check which is maintained) — keep deps minimal.
   Unit-test the checker functions with in-memory inputs.
4. CI job `tickets` in `ci.yml`: `cargo xtask ticket-lint --pr-branch "$HEAD_REF"`
   (with `HEAD_REF: ${{ github.head_ref }}` in `env:`, never inline; zizmor)
   on PRs; without `--pr-branch` on push. Do **not** gate it on
   `needs.changes` (ticket PRs are docs-only), and add it to `ci-result`'s
   `needs` (ADR-0014).
5. **Ruleset for `main`** (via `gh api repos/Zafnok/tactical-rpg/rulesets -X POST --input ruleset.json`,
   after Nick's OK): require a pull request (0 approvals — Nick doesn't review),
   require status checks (names per [ADR-0014](../../docs/adr/0014-ci-gates-skip-docs-only-prs.md)):
   `ci-result`, `codeql-result`, `mutants (diff)`, `deny`, `machete`,
   `typos`, `zizmor`, `tickets`. Do **not** require the matrix legs
   (`test (…)`, `analyze (…)`): on docs-only PRs they are skipped and never
   report under those names; `ci-result` / `codeql-result` cover them. Do not
   require the code-scanning `CodeQL` / `zizmor` result checks or a SonarCloud
   app check either (not posted when their job is skipped); the Sonar quality
   gate is enforced through the `coverage` job inside `ci-result` only if that
   job fails on a red gate. Block force pushes; block deletion. Check the names
   against a recent PR. Don't commit `ruleset.json`; paste it in Completion notes.
6. Repo settings (same OK from Nick): squash merge only, auto-delete head
   branches: `gh repo edit --enable-squash-merge --disable-merge-commit --disable-rebase-merge --delete-branch-on-merge`.

## Acceptance criteria

- [x] `cargo xtask ticket-lint` passes on the current repo and has unit tests
      for each rule (including failing cases).
- [x] CI `tickets` job passes on this PR (this ticket itself moved to `done/`).
- [ ] Ruleset active on `main`; a direct push to `main` is rejected. **Blocked
      on Nick running the `gh api ... rulesets` command below — this session
      has no ruleset-API access.**
- [x] PR template appears on new PRs (`.github/pull_request_template.md`).

## Completion notes

- Added `.github/pull_request_template.md` (verbatim from this ticket).
- Added `cargo xtask ticket-lint [--pr-branch <name>]` in
  `crates/xtask/src/tickets.rs` (dispatched from `crates/xtask/src/main.rs`).
  It parses each `tickets/{open,done}/*.md` file's frontmatter with
  `serde_norway` (a maintained fork of `serde_yaml`, which is archived;
  `serde_yml`, the other common fork, has had maintainer-trust concerns —
  `serde_norway` is MIT/Apache-2.0 and allowed by ADR-0013) and checks every
  rule from the ticket: filename shape, frontmatter parses and has all
  required keys, `id` matches the filename and is unique across both
  folders, enum fields (`type`, `model`, `effort`, `status`, `nick_input`)
  take a value from `tickets/README.md`, `done/` files have `status: done`
  and a `completed:` date while `open/` files don't have `status: done`,
  every `blocked_by` id exists, and (with `--pr-branch t<NNNN>-…`) ticket
  `NNNN` is in `done/`. 21 unit tests cover each rule (including failing
  cases) with in-memory frontmatter, plus one end-to-end test that runs the
  full checker against this repo's real `tickets/` tree. `xtask` was already
  covered by the `coverage` job's `cargo llvm-cov --workspace --exclude
  trpg-app` (only `trpg-app` is excluded) and wasn't noisy, so it stays in
  both Sonar coverage and `cargo mutants` scope, as the ticket allows.
- Added the `tickets` job to `ci.yml`: runs unconditionally (not gated on
  `needs.changes`, per ADR-0014 — ticket hygiene is exactly what a docs-only
  PR needs checked) and added to `ci-result`'s `needs`. On `pull_request` it
  passes `--pr-branch "$HEAD_REF"` (`HEAD_REF`/`EVENT_NAME` come from
  `env:`, never interpolated into `run:`, per zizmor); on `push` it runs
  without `--pr-branch`.
- **Not done by this session — needs Nick's OK, see "Nick input" above:**
  the repository ruleset and repo merge/branch settings. This session has
  no `gh` CLI or GitHub ruleset-API access, so it could only prepare, not
  run, these:

  Ruleset (`gh api repos/Zafnok/tactical-rpg/rulesets -X POST --input ruleset.json`,
  do not commit `ruleset.json`):
  ```json
  {
    "name": "main-protection",
    "target": "branch",
    "enforcement": "active",
    "conditions": { "ref_name": { "include": ["refs/heads/main"], "exclude": [] } },
    "rules": [
      {
        "type": "pull_request",
        "parameters": {
          "required_approving_review_count": 0,
          "dismiss_stale_reviews_on_push": false,
          "require_code_owner_review": false,
          "require_last_push_approval": false,
          "required_review_thread_resolution": false
        }
      },
      {
        "type": "required_status_checks",
        "parameters": {
          "required_status_checks": [
            { "context": "ci-result" },
            { "context": "codeql-result" },
            { "context": "mutants (diff)" },
            { "context": "deny" },
            { "context": "machete" },
            { "context": "typos" },
            { "context": "zizmor" },
            { "context": "tickets" }
          ],
          "strict_required_status_checks_policy": false
        }
      },
      { "type": "deletion" },
      { "type": "non_fast_forward" }
    ]
  }
  ```

  Repo settings:
  ```
  gh repo edit Zafnok/tactical-rpg --enable-squash-merge --disable-merge-commit --disable-rebase-merge --delete-branch-on-merge
  ```

  Until Nick runs these, `main` is not actually protected — the code side
  (PR template, `tickets` CI job, all checks named above) is in place and
  green, but the last two acceptance criteria ("ruleset active", "a direct
  push to main is rejected") are outstanding pending his OK and access.

