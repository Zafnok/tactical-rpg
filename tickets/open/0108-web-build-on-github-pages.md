---
id: "0108"
title: Deploy the web build to GitHub Pages on every push to main
type: infra
milestone: M0 Foundation
model: sonnet-5
effort: low
status: todo
blocked_by: ["0206"]
nick_input: setup
completed:
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
      reach the game (page doesn't scroll on arrow keys).
- [ ] Workflow passes zizmor.

## Completion notes

