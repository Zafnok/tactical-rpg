---
id: "0106"
title: "Repo guardrails: PR template, ticket lint (xtask), branch protection"
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0102", "0103", "0104", "0105"]
nick_input: setup
completed:
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
4. CI job `tickets` in `ci.yml`: `cargo xtask ticket-lint --pr-branch "${{ github.head_ref }}"`
   on PRs; without `--pr-branch` on push.
5. **Ruleset for `main`** (via `gh api repos/Zafnok/tactical-rpg/rulesets -X POST --input ruleset.json`,
   after Nick's OK): require a pull request (0 approvals — Nick doesn't review),
   require status checks: `fmt`, `clippy`, `test (windows-latest)`,
   `test (ubuntu-latest)`, `test (macos-latest)`, `wasm`, `docs`, `tickets`,
   `deny`, `machete`, `typos`, `zizmor`, CodeQL, mutants PR job, SonarCloud
   quality gate; block force pushes; block deletion. Use the exact check names
   shown on a recent PR. Don't commit `ruleset.json`; paste it in Completion notes.
6. Repo settings (same OK from Nick): squash merge only, auto-delete head
   branches: `gh repo edit --enable-squash-merge --disable-merge-commit --disable-rebase-merge --delete-branch-on-merge`.

## Acceptance criteria

- [ ] `cargo xtask ticket-lint` passes on the current repo and has unit tests
      for each rule (including failing cases).
- [ ] CI `tickets` job passes on this PR (this ticket itself moved to `done/`).
- [ ] Ruleset active on `main`; a direct push to `main` is rejected.
- [ ] PR template appears on new PRs.

## Completion notes

