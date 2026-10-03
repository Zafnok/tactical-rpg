# ADR-0042: PRs merge when their checks are green; they need not be up to date with `main`

- **Status:** Accepted
- **Date:** 2026-10-02
- **Related tickets:** 0117, 0115
- **Related ADRs:** ADR-0041 (superseded), ADR-0014

## Context

ADR-0041 turned on "require branches to be up to date before merging" so that
two PRs that are each green but break together cannot both merge. In use, the
same day: CI on a code PR is slow, every merge marks every other open PR
"out-of-date", and each one then needs "Update branch" plus a full CI run. PRs
could only merge one after another, each waiting for the one before.

The owner weighed it: a PR that is green and has no merge conflict is nearly
always fine on `main`; the rare time it is not (as with #141 and #156), the fix
has been a line or two. The wait on every merge costs more than that.

## Decision

- The "protect main" ruleset (id 24207509) keeps its required status checks
  and their pin to the GitHub Actions app, exactly as listed in ADR-0041, but
  with `strict_required_status_checks_policy: false`.
- A PR may merge when its required checks are green on its own last commit and
  GitHub reports no merge conflict. It does not need to contain the current
  `main`.
- Sessions do not merge `main` into a PR branch just because `main` moved. They
  do it when GitHub reports a conflict, or when the PR needs something that
  landed on `main`.
- If `main` goes red after a merge, fixing it comes before other work: a
  ticket and a small PR (as 0823 did).
- The deletion and force-push rules and the owner bypass stay as they are.

## Consequences

- Open PRs merge in any order, as soon as each is green; no "Update branch"
  round after every merge.
- Two PRs that are each green can break `main` together again. CI on `main`
  shows it after the merge, and every open PR that then merges `main` in is red
  until the fix lands.
- The rest of ADR-0041 still holds: which checks are required, the
  `integration_id` pin for `zizmor`, and that a new or renamed required job
  must be changed in the ruleset too.

## Alternatives considered

- **Keep "up to date before merging" (ADR-0041)** — safe, but makes merging
  sequential with a full CI run between each merge.
- **Merge queue** — tests each PR on top of `main` without anyone clicking
  "Update branch", but still runs CI once more per PR before it lands, and
  needs `merge_group` triggers in every required workflow plus changes to the
  docs-only detection of ADR-0014. Revisit if `main` goes red often.
- **Make CI faster** — worth doing on its own, but does not remove the
  one-after-another merging.
