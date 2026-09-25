---
id: "0108"
title: Deploy the web build to GitHub Pages on every push to main
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: low
status: done
blocked_by: ["0206"]
nick_input: setup
completed: 2026-09-25
---

# 0108 — Deploy the web build to GitHub Pages

## Context

Gives Nick a zero-install "play the latest build" link that updates with
every merged ticket ([ADR-0009](../../docs/adr/0009-distribution.md)).

## Nick input

**Setup:** GitHub → repo **Settings → Pages → Build and deployment → Source:
GitHub Actions**. (Or tell the session "go ahead" to run
`gh api repos/Zafnok/tactical-rpg/pages -X POST -f build_type=workflow`.)

## Scope

**In:** `.github/workflows/pages.yml`, README "Play in browser" link.

**Out:** itch.io web upload (0901).

## Implementation steps

1. Workflow on `push: branches: [main]` and `workflow_dispatch`.
2. Build job: toolchain + cache, run the web packaging command from 0206,
   `actions/upload-pages-artifact` with the web folder.
3. Deploy job: `actions/deploy-pages`, `environment: github-pages`,
   permissions `pages: write`, `id-token: write` on that job only.
4. Pin by SHA. Top-level `permissions: contents: read`.
5. Add the Pages URL (`https://zafnok.github.io/tactical-rpg/`) to `README.md`
   under a "Play" section.

## Acceptance criteria

- [ ] After merge, the Pages URL loads the game in Chrome and Firefox; keys
      reach the game (page doesn't scroll on arrow keys). *(Can't be verified
      from this PR — see Completion notes.)*
- [x] Workflow passes zizmor.

## Completion notes

- Added `.github/workflows/pages.yml`: a `build` job (checkout, Rust cache,
  `cargo xtask web --release`, `actions/upload-pages-artifact` on `dist/web`)
  and a `deploy` job (`actions/deploy-pages`, `environment: github-pages`,
  `pages: write` + `id-token: write` scoped to that job only). Triggers on
  `push` to `main` and `workflow_dispatch`. Top-level `permissions: contents:
  read`; all third-party actions pinned by commit SHA (verified against
  upstream tags with `git ls-remote`), version noted in a trailing comment,
  matching the existing workflows' convention.
- Added a "Play" section to `README.md` linking
  `https://zafnok.github.io/tactical-rpg/`.
- Did not add `actions/configure-pages` or an explicit Pages-enablement step:
  the ticket's Nick input says Pages must be switched to "GitHub Actions" as
  the build source in repo Settings (or Nick can say "go ahead" for the
  session to call the Pages-enablement API) before this workflow's `deploy`
  job can succeed — that's a one-time repo setting, not something the
  workflow itself does.
- **Acceptance criterion 1 (Pages URL loads in Chrome/Firefox after merge)
  cannot be verified from this branch**: the workflow only runs on push to
  `main`, and Pages must first be enabled per the Nick input above. Once this
  PR is merged and Pages is enabled, the next push to `main` will deploy;
  Nick should confirm the live URL loads and keys work, exactly as 0206
  already verified locally for the same `dist/web` build (same `index.html`
  and JS bundle, unchanged here).
- Verified locally instead: `cargo xtask web --release` still produces
  `dist/web/` (same command CI's `wasm` job and this new workflow both run);
  `zizmor --offline --min-severity medium .github/workflows/pages.yml`
  reports no findings (fully online zizmor, as CI runs it with a GitHub
  token, needs network access to GitHub's API for the `artipacked` audit,
  which this sandbox couldn't reach; the pinned-SHA/permissions checks that
  don't need it are clean).
- Local gates run: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, `cargo test --workspace`, `cargo xtask
  ticket-lint`, `typos`, `cargo xtask web --release` — all pass. No Rust
  source changed, so mutation testing has nothing in-diff to cover.

