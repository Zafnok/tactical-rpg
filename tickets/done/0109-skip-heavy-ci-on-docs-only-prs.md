---
id: "0109"
title: Skip heavy CI jobs on docs-only pull requests
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: medium
status: done
blocked_by: ["0102", "0103", "0104", "0105"]
nick_input: none
completed: 2026-09-25
---

# 0109 — Skip heavy CI jobs on docs-only pull requests

## Context

Every PR runs the whole pipeline (ADR-0008): `ci.yml` (fmt, clippy, tests on
three OSes, docs, wasm, coverage), `codeql.yml` (`analyze (rust)` takes about
2.5 min), `mutants.yml` and `security.yml`. That includes PRs that only change
Markdown, for example Zafnok/tactical-rpg#10 (ticket 0005 changed only
`docs/**` and `tickets/**`). Most `00xx` and `07xx` tickets will be docs-only,
so this is wasted time on almost half the backlog. CodeQL on a PR always
analyses the whole codebase; main's baseline is only used to show new alerts,
never to skip.

Constraint: ticket 0106 makes these checks **required**. A workflow skipped by
a `paths` / `paths-ignore` trigger never reports a status, so a required check
stays pending forever and blocks the merge. Workflow-level path filters are
therefore not allowed.

A second trap, seen on Zafnok/tactical-rpg#11: a **matrix** job skipped at job
level reports a single check with the unexpanded name
(`mutants (full, shard ${{ matrix.shard }})`), never the per-leg names. So a
skipped `test` or `analyze` matrix would never report `test (windows-latest)`
or `analyze (rust)`, and requiring those names would block docs-only PRs too.

## Nick input

`None.`

## Scope

**In:**
- A reusable workflow `.github/workflows/changes.yml` that outputs whether a PR
  touches anything besides docs.
- `needs: changes` + `if:` on the heavy jobs in `ci.yml`, `codeql.yml`,
  `mutants.yml` and `security.yml`.
- An always-running aggregate job in `ci.yml` (`ci-result`) and `codeql.yml`
  (`codeql-result`), which becomes the required check for those workflows.
- A new ADR recording the pattern, superseding ADR-0008.
- Update ticket 0106's list of required check names.

**Out (do not do):**
- Workflow-level `paths` / `paths-ignore`.
- Changing what any gate checks or its thresholds.
- Creating the branch-protection ruleset (that is 0106).
- `scorecard.yml` (runs only on push to main and weekly; nothing to skip).

## Implementation steps

1. `.github/workflows/changes.yml` (`on: workflow_call`, output `code`): on any
   event other than `pull_request`, `code=true`. On a PR, check out with
   `fetch-depth: 2` (the PR merge commit and its first parent, the base tip) and
   run `git diff --name-only --no-renames HEAD^1 HEAD`. `code=true` if any path
   is not docs-only. Docs-only = `*.md` anywhere, `docs/**`, `tickets/**`,
   `.claude/**` (skills and local agent settings; no CI job reads them).
2. In each heavy workflow add `changes: uses: ./.github/workflows/changes.yml`
   and give heavy jobs `needs: changes` and
   `if: needs.changes.outputs.code == 'true'`.
   - `ci.yml`: all jobs.
   - `codeql.yml`: `analyze`.
   - `mutants.yml`: `mutants (diff)` only (the weekly jobs already run only on
     `schedule`/`workflow_dispatch`).
   - `security.yml`: `deny`, `machete`, `zizmor`. Keep `typos` always running.
3. Add `ci-result` / `codeql-result`: `if: always()`, `needs:` every job in the
   workflow (including `changes`), fail if any result is `failure` or
   `cancelled`; `skipped` passes.
4. Write the ADR (`write-adr`), superseding ADR-0008; update the ADR index.
5. Update 0106 step 5 with the new required check names.

## Acceptance criteria

- [x] A docs-only PR shows the heavy jobs as skipped; `ci-result`,
      `codeql-result`, `mutants (diff)`, `deny`, `machete`, `zizmor` report
      skipped/success and `typos` runs. The PR is mergeable. Run URL in notes.
- [x] A PR touching `crates/**` runs every job. Run URL in notes.
- [x] A push to `main` runs CodeQL (`analyze (rust)`, `analyze (actions)`) in
      full; the weekly schedule path is unchanged (`code=true` for any
      non-PR event). Run URL in notes.
- [x] `zizmor` reports no medium/high findings on the changed workflows.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Integration: the three CI runs above (docs-only PR, code PR, push to main).

## Completion notes

**Done as planned.** `.github/workflows/changes.yml` (reusable, job `detect`,
output `code`) gates the heavy jobs with `if:`; `ci-result` and
`codeql-result` aggregate the matrix workflows; ADR-0014 supersedes ADR-0008;
ticket 0106 now lists `ci-result`, `codeql-result`, `mutants (diff)`, `deny`,
`machete`, `typos`, `zizmor`, `tickets` as the required checks, says not to
gate `tickets`, and is blocked by 0109.

**`.claude/**` counts as docs-only:** it holds agent skills (Markdown) and
local agent settings, and no CI job reads it. `.github/**` counts as code, so
workflow changes always run zizmor and CodeQL (actions).

**Proof runs** (throwaway PRs based on this branch so they used the new
workflows; closed and branches deleted afterwards):

- Docs-only PR (Zafnok/tactical-rpg#15, one line in `docs/ROADMAP.md`):
  every heavy job **skipped**; `changes / detect`, `typos`, `ci-result`,
  `codeql-result` passed; PR state `MERGEABLE / CLEAN`.
  CI <https://github.com/Zafnok/tactical-rpg/actions/runs/36170535916>,
  CodeQL <https://github.com/Zafnok/tactical-rpg/actions/runs/36170536194>,
  Security <https://github.com/Zafnok/tactical-rpg/actions/runs/36170536125>,
  Mutants <https://github.com/Zafnok/tactical-rpg/actions/runs/36170536199>.
- `crates/**` PR (Zafnok/tactical-rpg#16, comment in `crates/core/src/lib.rs`):
  **everything ran and passed** (fmt, clippy, test ×3, wasm, docs, coverage,
  deny, machete, typos, zizmor, `analyze (rust)`, `analyze (actions)`,
  `mutants (diff)`).
  CI <https://github.com/Zafnok/tactical-rpg/actions/runs/36170539307>,
  CodeQL <https://github.com/Zafnok/tactical-rpg/actions/runs/36170539418>,
  Security <https://github.com/Zafnok/tactical-rpg/actions/runs/36170539322>,
  Mutants <https://github.com/Zafnok/tactical-rpg/actions/runs/36170539377>.
- Push event (branch `ci-proof/push` = this branch with only its name added to
  the `push: branches` lists, since a real push to `main` needs this PR merged
  first): `changes` returned `code=true`, CodeQL ran `analyze (rust)` and
  `analyze (actions)` in full, all CI jobs ran.
  CodeQL <https://github.com/Zafnok/tactical-rpg/actions/runs/36170551716>,
  CI <https://github.com/Zafnok/tactical-rpg/actions/runs/36170551734>,
  Security <https://github.com/Zafnok/tactical-rpg/actions/runs/36170551822>.
  `schedule` takes the same non-PR branch of `changes.yml` (`code=true`).
  The first real push to `main` after merge is the final confirmation.
- Negative check (Zafnok/tactical-rpg#17, deliberate fmt error): `fmt` failed
  and **`ci-result` failed**, so the aggregate cannot pass silently.
  <https://github.com/Zafnok/tactical-rpg/actions/runs/36170973367>.

**Deviations:**
- A skipped matrix job reports one check with the unexpanded name (seen live
  on #15: `test`, `analyze (${{ matrix.language }})`), so the per-leg names
  in 0106 could never be required. That is why the aggregates exist.
- `mutants.yml` already had zizmor medium/high findings (`artipacked` on
  three checkouts, `template-injection` of `github.base_ref`). The ticket
  requires zero medium+ findings on changed workflows, so they were fixed
  (`persist-credentials: false`, `BASE_REF` via `env:`). `uvx zizmor
  --offline --min-severity medium .github/workflows` → no findings.
- The code-scanning `CodeQL` / `zizmor` result checks and any SonarCloud app
  check are not posted on docs-only PRs; 0106 now says not to require them.
- `run-gates` skill now points at ADR-0014 instead of ADR-0008.

**Follow-ups:** none.
