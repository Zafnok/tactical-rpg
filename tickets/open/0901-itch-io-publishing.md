---
id: "0901"
title: Publish releases to itch.io with butler
type: infra
milestone: M8 Release
model: sonnet-5
effort: low
status: todo
blocked_by: ["0107"]
nick_input: setup
completed:
---

# 0901 — itch.io publishing

## Context

[ADR-0009](../../docs/adr/0009-distribution.md): itch.io gets Windows/Linux/
macOS downloads and a browser-playable build, pushed automatically on release.

## Nick input

**Setup (≈10 minutes):**
1. Create an itch.io account (if needed) → **Dashboard → Create new project**.
2. Title: working title; **Kind of project: HTML** (lets the web build play in
   the page; downloads can still be attached). Visibility: **Draft** or
   **Restricted** until ready.
3. **Account settings → API keys → Generate new API key.**
4. GitHub repo → **Settings → Secrets and variables → Actions** → new secret
   `BUTLER_API_KEY` with that key.
5. Tell the session the project URL (e.g. `https://<user>.itch.io/<game>`).
6. After the first push: on the project's edit page, tick **"This file will be
   played in the browser"** for the `web` upload, set the embed size to
   1600×1024 (or "click to launch in fullscreen"), and save.

## Implementation steps

1. Add a job to `release.yml` (after packaging): install butler (official
   download from `broth.itch.zone`, verify it runs), then
   `butler push <file-or-dir> <user>/<game>:<channel> --userversion <version>`
   for channels `windows`, `linux`, `mac`, `web` (the web folder, not the zip).
2. `BUTLER_API_KEY` passed via env; skip the job when the secret is absent.
3. Document the itch project settings above in `docs/RELEASING.md` (create it;
   move the release steps from README there and link).

## Acceptance criteria

- [ ] A release (or `workflow_dispatch` run) pushes all four channels; itch shows them.
- [ ] Web build plays in the itch page (Chrome + Firefox).
- [ ] `docs/RELEASING.md` complete.

## Completion notes

