# ADR-0041: PRs must be up to date with `main` and green before they merge

- **Status:** Superseded by ADR-0042
- **Date:** 2026-10-02
- **Related tickets:** 0115, 0106
- **Related ADRs:** ADR-0014

## Context

`main` went red on 2026-10-02: PR #141 was green when its CI last ran, PR #156
merged after that and changed what #141's tests relied on, and #141 merged
without running again. The "protect main" ruleset (id 24207509) only blocked
deletion and force pushes; the required status checks planned in 0106 were
never added.

GitHub offers two ways to make a PR pass against the latest `main`:

1. **Require branches to be up to date before merging**
   (`strict_required_status_checks_policy: true`). A PR whose base is behind
   `main` cannot merge until it is updated (the "Update branch" button, or a
   push) and its required checks pass again.
2. **A merge queue.** GitHub tests each queued PR on top of `main` plus the PRs
   ahead of it, then merges. Every required workflow needs an
   `on: merge_group` trigger, and the docs-only detection of ADR-0014 would
   have to handle the `merge_group` event.

One person merges, a few PRs a day.

## Decision

- Use **"up to date before merging"** (option 1). No merge queue.
- The ruleset requires these checks, all pinned to the GitHub Actions app
  (`integration_id: 15368`): `ci-result`, `codeql-result`, `mutants (diff)`,
  `deny`, `machete`, `typos`, `zizmor`, `tickets`. These are the aggregates and
  single jobs of ADR-0014, never matrix legs.
- The pin matters for `zizmor`: the code-scanning app (GitHub Advanced
  Security, id 57789) also posts a check named `zizmor` when the job uploads
  its SARIF, and does not post it when the job is skipped.
- The ruleset keeps its deletion and force-push rules and its bypass for the
  repository owner (for emergencies only; ticking "bypass" is never the normal
  way to merge).
- Sessions: when `main` moves while a PR is open, update the branch (merge
  `main` into it) and let CI run again before saying it can merge
  (`work-ticket` step 6).

## Consequences

- A PR is only mergeable when its required checks passed on a branch that
  contains the current `main`, so two PRs that are each green but break
  together can no longer both merge silently.
- After every merge the other open PRs show "This branch is out-of-date" and
  need an update plus a fresh CI run (several minutes for code PRs, under a
  minute for docs-only ones). With a few PRs a day that is acceptable.
- Dependabot PRs update the same way ("Update branch" or `@dependabot rebase`).
- A new required job must be added to the ruleset (or to an aggregate's
  `needs`, ADR-0014); a renamed job must be renamed in the ruleset, otherwise
  every PR waits forever for the old name.

## Alternatives considered

- **Merge queue** — tests the combination automatically and scales to many
  merges a day, but needs `merge_group` triggers in every required workflow
  and changes to the docs-only detection, for a repository where one person
  merges a few PRs a day. Revisit if updating branches becomes a chore.
- **Required checks without "up to date"** — exactly what let #141 break
  `main`.
