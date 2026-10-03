---
id: "0117"
title: PRs merge on green without being up to date with main
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: setup
completed: 2026-10-02
---

# 0117 — PRs merge on green without being up to date with main

## Context

0115 ([ADR-0041](../../docs/adr/0041-prs-up-to-date-before-merging.md)) made
the "protect main" ruleset require every PR to be up to date with `main`
before merging. The same day Nick asked for it to be turned off: CI is slow,
every merge makes the other open PRs out-of-date, so PRs could only merge one
after another with a full CI run between each. A green PR with no merge
conflict is nearly always fine, and the rare break on `main` has been a
one- or two-line fix.

## Nick input

**Setup:** the ruleset is a repository security setting. Nick said "turn it
off" in chat; the session ran the command.

## Scope

**In:**
- The "protect main" ruleset: `strict_required_status_checks_policy: false`.
  The required checks themselves stay.
- An ADR that supersedes ADR-0041.
- The `work-ticket` skill's step 6 line about updating the branch.

**Out (do not do):**
- Removing the required status checks.
- A merge queue.
- Changing what the CI jobs do or how long they take.

## Implementation steps

1. Read the ruleset, set the strict policy to `false`, `PUT` it back.
2. Write the ADR; mark ADR-0041 superseded; update the index.
3. Update the `work-ticket` skill.

## Acceptance criteria

- [x] `gh api repos/Zafnok/visions-of-shuyi/rulesets/24207509 --jq .rules` lists `required_status_checks` with `strict_required_status_checks_policy: false` and the same eight checks.
- [x] ADR written and in the index; ADR-0041 marked superseded.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None in code.

## Completion notes

- Ran on 2026-10-02: read the ruleset with `gh api`, changed only
  `strict_required_status_checks_policy` to `false`, and
  `gh api repos/Zafnok/visions-of-shuyi/rulesets/24207509 -X PUT --input ruleset.json`
  (not committed). The reply lists the same eight required checks, the
  deletion and force-push rules, and the strict policy `false`.
- [ADR-0042](../../docs/adr/0042-prs-merge-on-green-without-updating.md)
  supersedes ADR-0041. The list of required checks and the `zizmor` pin from
  ADR-0041 still apply.
- `work-ticket` step 6: merge `main` into a PR branch only for a conflict or
  when the PR needs something from `main`.
- **For Nick:** a PR merges as soon as its own checks are green and GitHub
  shows no conflict; no more "Update branch" after each merge. If `main` turns
  red after a merge, say so and a session fixes it first.
