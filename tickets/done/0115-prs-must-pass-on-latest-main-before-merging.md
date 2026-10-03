---
id: "0115"
title: PRs must pass CI on the latest main before they merge
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: setup
completed: 2026-10-02
---

# 0115 — PRs must pass CI on the latest main before they merge

## Context

`main` went red on 2026-10-02: PR #141 ([0802]) was green when its CI last
ran, PR #156 ([0435]) merged after that and changed what 0802's tests
relied on, and #141 merged without running again. Every open PR was red
until 0823 fixed the tests.

Nothing stops this today. The ruleset on `main` ("protect main", id
24207509) only blocks deletion and force pushes
(`gh api repos/Zafnok/visions-of-shuyi/rulesets/24207509 --jq .rules`):
the status checks 0106 planned (step 5 and the JSON in its Completion
notes) were never added, and nothing asks a PR to be up to date with
`main`.

## Nick input

**Setup:** the ruleset is a repository security setting, so Nick turns it
on. The session prepares the exact command (or the clicks under
Settings → Rules → Rulesets → "protect main") and Nick runs it, or says OK
for the session to run it. Nick merges PRs himself, so tell him what
changes for him: a PR that is behind `main` shows an "Update branch"
button, and merges once its checks are green again.

## Scope

**In:**
- The "protect main" ruleset: require status checks, with "require
  branches to be up to date before merging" (`strict_required_status_checks_policy: true`).
- Which checks: the names in 0106 step 5, checked against a recent PR's
  real check names ([ADR-0014](../../docs/adr/0014-ci-gates-skip-docs-only-prs.md):
  the aggregates such as `ci-result`, never the matrix legs).
- A short ADR (`write-adr` skill) for the choice between "up to date before
  merging" and a merge queue (which needs `merge_group` triggers in every
  required workflow). Start with "up to date": one person merges, a few PRs
  a day.
- Dependabot PRs: check they can still merge (they rebase themselves on
  `@dependabot rebase` or the "Update branch" button).

**Out (do not do):**
- Requiring reviews (Nick doesn't review code).
- Changing what the CI jobs do.
- Committing `ruleset.json` (paste it in the Completion notes, as 0106 did).

## Implementation steps

1. Read 0106's Completion notes and ADR-0014; list the check names a recent
   code PR and a recent docs-only PR reported
   (`gh pr checks <n> --json name,state`).
2. Write the ADR.
3. Prepare the ruleset update (`gh api
   repos/Zafnok/visions-of-shuyi/rulesets/24207509 -X PUT --input
   ruleset.json`) keeping the two rules it has, adding
   `required_status_checks` with the strict policy.
4. Ask Nick for the OK (plain words: what he will see when merging), then
   run it or hand him the command.
5. Check on a real PR that is behind `main`: GitHub blocks the merge until
   the branch is updated and green. Check a docs-only PR and a Dependabot PR
   can still merge.
6. Update the `work-ticket` skill's step 6 if the merge steps change for a
   session (e.g. "update the branch when `main` moves").

## Acceptance criteria

- [x] `gh api repos/Zafnok/visions-of-shuyi/rulesets/24207509 --jq .rules` lists `required_status_checks` with `strict_required_status_checks_policy: true`.
- [x] A PR behind `main` can't be merged until updated (seen on a real PR; say which in the Completion notes).
- [ ] A docs-only PR and a Dependabot PR still merge.
- [x] ADR written and in the index.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- None in code: the checks in the acceptance criteria, done by hand on real PRs.

## Completion notes

- Real check names read from #151 (code PR), #164 and #166 (docs-only) with
  `gh pr checks`; they match 0106 step 5 / ADR-0014. One catch: on code PRs
  **two** checks are named `zizmor` (the Security job, app `github-actions`
  15368, and the code-scanning result, app `github-advanced-security` 57789,
  which isn't posted when the job is skipped). Every required check is
  therefore pinned to `integration_id: 15368`.
- [ADR-0041](../../docs/adr/0041-prs-up-to-date-before-merging.md): "up to
  date before merging", no merge queue. Added to the ADR index.
- Nick said OK in chat; the session ran
  `gh api repos/Zafnok/visions-of-shuyi/rulesets/24207509 -X PUT --input ruleset.json`
  on 2026-10-02 with (not committed):
  ```json
  {
    "name": "protect main",
    "target": "branch",
    "enforcement": "active",
    "conditions": { "ref_name": { "include": ["~DEFAULT_BRANCH"], "exclude": [] } },
    "bypass_actors": [ { "actor_id": 10923040, "actor_type": "User", "bypass_mode": "always" } ],
    "rules": [
      { "type": "deletion" },
      { "type": "non_fast_forward" },
      {
        "type": "required_status_checks",
        "parameters": {
          "strict_required_status_checks_policy": true,
          "do_not_enforce_on_create": false,
          "required_status_checks": [
            { "context": "ci-result", "integration_id": 15368 },
            { "context": "codeql-result", "integration_id": 15368 },
            { "context": "mutants (diff)", "integration_id": 15368 },
            { "context": "deny", "integration_id": 15368 },
            { "context": "machete", "integration_id": 15368 },
            { "context": "typos", "integration_id": 15368 },
            { "context": "zizmor", "integration_id": 15368 },
            { "context": "tickets", "integration_id": 15368 }
          ]
        }
      }
    ]
  }
  ```
  The owner bypass already on the ruleset is kept (emergency use only).
- PR behind `main`: right after the change, Dependabot PR #158 (14 commits
  behind) reported `mergeStateStatus: BEHIND` and #140 (1 behind, CI
  running) `BLOCKED`. Before the change both would have been mergeable.
- Docs-only PR: this ticket's own PR (docs, skill and ticket only); its
  skipped jobs report as skipped and satisfy the rule. Dependabot: #161
  (github-actions bump) passed every required check including `tickets`,
  so Dependabot PRs pass the rule once updated ("Update branch" or
  `@dependabot rebase`). #158 itself is red on its own (the png 0.18 bump
  breaks the build); that is not caused by this rule.
- `work-ticket` step 6 now says: when `main` moves, merge it into the
  branch, push and wait for CI again.
- Not done (out of scope): requiring a pull request for every push to
  `main` (0106 planned it; this ticket only asked for the checks).
- **For Nick:** after you merge a PR, the other open PRs say "This branch
  is out-of-date". Click "Update branch", wait for green, then merge. Leave
  the "bypass rules" box unticked.
