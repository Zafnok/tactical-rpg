---
id: "0903"
title: "Steam readiness: steamworks feature flag, depot build script, Deck check"
type: research
milestone: M8 Release
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0804", "0902"]
nick_input: setup
completed:
---

# 0903 — Steam readiness

## Context

Steam is the end goal after Chapter 1 proves fun
([ADR-0009](../../docs/adr/0009-distribution.md)). Steam needs a Steamworks
partner account and a $100 app fee — Nick's decision and money.

## Nick input

**Setup (when Nick decides to go for Steam):** register at
<https://partner.steamgames.com>, pay the app fee, create the app, and share the
**App ID** and a build account for CI (a dedicated Steam account with 2FA via
the `steamcmd` guard flow — the session will explain exact steps then).
Until then this ticket can do everything with Valve's test App ID 480.

## Implementation steps

1. Add optional Cargo feature `steam` to `trpg-app` using the `steamworks`
   crate: init on startup (fail gracefully → run without Steam), run callbacks
   each frame, show overlay-compatibility check. App ID 480 for development.
   The default build must not require Steam or its SDK.
2. Research and write `docs/steam.md`: Steamworks setup, depot layout, redistributing
   `steam_api64.dll`, `steamcmd` + `app_build.vdf` upload flow, Steam Cloud for
   saves (maps to our storage keys), achievements idea list (don't implement).
3. **Steam Deck:** the game is keyboard-driven. Document what's needed for Deck
   Verified (full controller support, readable text at 1280×800) and create a
   `10xx` ticket for controller support mapping gamepad buttons to `Action`s.
4. Write a new ADR only if the integration approach departs from ADR-0009.

## Acceptance criteria

- [ ] `cargo build -p trpg-app --features steam` works in CI on Windows; default build unchanged.
- [ ] With Steam running (App ID 480), the overlay opens in-game (manual check, screenshot).
- [ ] `docs/steam.md` complete; controller-support ticket created.

## Completion notes

