---
id: "0406"
title: One-time contextual tips (teaching approach from Chapter 1 scope)
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0009", "0405", "0207"]
nick_input: answer-first
completed:
---

# 0406 — Contextual tips

## Context

Implements the teaching approach Nick picked in `docs/design/chapter-1.md`
(0009): **contextual hints** (help bar + one-time tips), no forced tutorial
steps. Chapter 1 has no Preparations screen, so no Preparations tip is needed
yet.

## Nick input

**Answer first:** 0009.

## Scope

**In:** `assets/data/tips.ron`, tip triggers, tip popup, "seen" persistence.

**Out:** forced tutorial steps (unless Nick chose that).

## Implementation steps

1. `tips.ron`: list of `(id, trigger, title, text)`; triggers enum:
   `FirstBattleStart`, `FirstUnitSelected`, `FirstEnemyInRange`, `FirstForecast`,
   `FirstLowHp`, `FirstLevelUp`, `FirstEnemyPhase`, `DangerZoneAvailable`.
   Write short, friendly text (≤ 3 lines × 60 chars) mentioning the actual keys
   (read them from the keymap so rebinding stays correct: use placeholders like
   `{Confirm}` replaced at display time).
2. Trigger detection in `BattleScreen` at the relevant points; each tip shows
   once per profile.
3. Popup overlay: small double-line box near the top, dismissed with Confirm or
   Cancel; never shown during enemy phase playback.
4. Persist seen ids via `Storage` (`tips_seen` key). Options menu (0805) will
   offer "reset tips" — leave a function for it.

## Acceptance criteria

- [ ] Each trigger fires once and only once (Harness tests using `MemoryStorage`).
- [ ] Key placeholders render actual bound keys.
- [ ] Tips never block input permanently (dismiss works in all states).

## Tests required

- Harness per trigger (at least 4 of them), placeholder substitution unit test, snapshot of a tip.

## Completion notes

