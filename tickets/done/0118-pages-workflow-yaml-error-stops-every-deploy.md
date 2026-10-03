---
id: "0118"
title: Fix the Pages workflow YAML error that stops every deploy
type: bug
milestone: M0 Foundation
model: sonnet-5
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-03
---

# 0118 — Fix the Pages workflow YAML error that stops every deploy

## Context

The web build on GitHub Pages (ticket 0108) is what Nick plays for his
sign-offs. It has not deployed since ticket 0114 (PR #169, merged
2026-10-02 23:47 UTC). The last good deploy is commit `b7d5e4f` (ticket
0110).

0114 changed one line of `.github/workflows/pages.yml`:

```yaml
        run: cargo test -p trpg-content --locked --features private-assets --test it private_assets::
```

The line is an unquoted YAML scalar that ends in `::`. A `:` at the end of
a line starts a mapping, so the file does not parse:

```
mapping values are not allowed here
  in ".github/workflows/pages.yml", line 39, column 101
```

GitHub then records a failed `.github/workflows/pages.yml` run with no jobs
on **every push to every branch** ("This run likely failed because of a
workflow file issue"), and nothing is deployed. Nothing caught it: the
`actions` CodeQL analysis and zizmor both passed on PR #169, and the Pages
workflow only runs after merge.

Found while looking at slow CI on PR #174 (see also ticket 0119).

## Nick input

`None.` After the PR merges, the Pages site updates by itself; Nick's
pending sign-offs for everything merged since 0110 can then be played.

## Scope

**In:**
- Make `.github/workflows/pages.yml` valid YAML again.
- A check on every PR that all workflow and action files parse, so a
  broken workflow file cannot merge again.

**Out (do not do):**
- Any other change to what the Pages workflow builds or deploys.
- PR preview deploys.
- The slow mutation job (ticket 0119).

## Implementation steps

1. In `.github/workflows/pages.yml`, step "Check the content loads with the
   bought art", write the command as a block scalar so the trailing `::`
   is plain text:
   ```yaml
        run: >-
          cargo test -p trpg-content --locked --features private-assets
          --test it private_assets::
   ```
2. Search the other workflow and action files for the same mistake
   (`grep -rnE ':\s*$' .github | grep 'run:'`) and fix any found.
3. Add a test to `crates/xtask` (next to the existing repo-layout tests,
   e.g. the one 0114 added for `tests/` folders) named
   `every_workflow_file_parses`: it reads every `*.yml` under
   `.github/workflows/` and every `action.yml` under `.github/actions/`
   and parses each as YAML, failing with the file name and the parser's
   message. Use a YAML crate under a license ADR-0013 allows, as a
   dev-dependency of `xtask` only; if none is acceptable, add an
   `actionlint` job to `.github/workflows/security.yml` instead (pinned by
   commit like the other actions) and say so in the completion notes.
4. Check the test fails on the broken file first (run it before step 1's
   fix, or temporarily revert the fix), then passes.
5. After merge, confirm the `Pages` run on `main` is green and that it
   deployed; write the run's URL in the completion notes of the PR
   description.

## Acceptance criteria

- [x] `python -c "import yaml; yaml.safe_load(open('.github/workflows/pages.yml'))"`
      exits 0 (or the equivalent with the new test).
- [x] `every_workflow_file_parses` (or the `actionlint` job) fails on the
      line as it is on `main` today and passes with the fix.
- [ ] Pushing the branch no longer creates a failed
      `.github/workflows/pages.yml` run.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `every_workflow_file_parses` in `crates/xtask` (or the CI lint job
  named in step 3).
- Property: none.
- Snapshot / integration: none.

## Completion notes

- `.github/workflows/pages.yml`: the command of the step "Check the content
  loads with the bought art" is now a `>-` block scalar, so the trailing
  `::` is plain text. The command itself is unchanged.
- The grep of step 2 found no other `run:` line ending in `:`.
- New test `every_workflow_file_parses` in `crates/xtask/src/main.rs`: it
  parses every `*.yml` under `.github/workflows/` and every `action.yml`
  under `.github/actions/`, and fails with the file name and the parser's
  message. It uses `serde_norway`, which `xtask` already depends on, so no
  new crate was added. It runs in CI's test job on every PR that touches a
  workflow file (workflow files count as code in `changes.yml`).
- Checked before the fix: the test failed with `pages.yml is not valid
  YAML: mapping values are not allowed in this context at line 39 column
  101`. It passes with the fix.
- The test checks that the files are valid YAML, not that GitHub accepts
  their content (a wrong key name would still pass). That was the ticket's
  scope.
- Still to do after the merge (step 5): confirm the `Pages` run on `main`
  is green and deployed, and add its URL to the PR description. The third
  acceptance criterion is checked on the PR once the branch is pushed.
- Follow-up tickets: none.
- Gameplay rules decided: none.

**For Nick:** nothing to decide. Once this merges, the Pages site updates by
itself and everything merged since ticket 0110 can be played there.
