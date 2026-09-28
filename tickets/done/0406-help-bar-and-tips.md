---
id: "0406"
title: One-time contextual tips (teaching approach from Chapter 1 scope)
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0009", "0405", "0207"]
nick_input: answer-first
completed: 2026-09-28
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

- [x] Each trigger fires once and only once (Harness tests using `MemoryStorage`).
- [x] Key placeholders render actual bound keys.
- [x] Tips never block input permanently (dismiss works in all states).

## Tests required

- Harness per trigger (at least 4 of them), placeholder substitution unit test, snapshot of a tip.

## Completion notes

One-time tips are in. `assets/data/tips.ron` holds eight tips (one per trigger:
`FirstBattleStart`, `FirstUnitSelected`, `FirstEnemyInRange`, `FirstForecast`,
`FirstLowHp`, `FirstLevelUp`, `FirstEnemyPhase`, `DangerZoneAvailable`),
validated on load (`trpg_content::tip`: unique ids and triggers, at most 3
lines of 60 characters, known `{Action}` / `{Cursor}` placeholders). The
battle screen (`screens/battle/tips.rs`, `crates/ui/src/tips.rs`) fires the
triggers, queues unseen tips and shows one as a double-line box near the top
of the map. Confirm or Cancel dismisses it; other keys wait until then.
Placeholders are filled from the active keymap, so they follow the layout.
Seen ids are saved as `tips_seen` in `Storage` (recorded when queued, so a
tip shows at most once). `ui::tips::reset_tips` is the hook for 0805's "reset
tips".

Deviations and notes:

- The help bar half of the ticket's old title was already done (0405/0420).
- `Ctx::tips_enabled` is **off by default** so the many existing battle
  tests aren't interrupted by a popup; `app` turns it on and tests use
  `Harness::with_tips`. 0805 should make it an option.
- No level-up UI exists yet; `FirstLevelUp` fires on any player
  `Event::LeveledUp`, so it will show once 0602 lands.
- 0502 (enemy phase playback) should re-check `FirstEnemyPhase`: it shows
  over the phase banner, before the enemy moves, so it never runs during
  playback.

*Claude's starting rules* (gameplay-affecting, Nick can veto):

- "Low HP" is a player unit at or below 25% of max HP.
- `FirstEnemyInRange`: a selected unit could reach an enemy (move or
  attack tiles).
- `DangerZoneAvailable`: the cursor rests on an enemy while browsing and
  the danger zone isn't already shown.
- Tips wait for a combat, a banner or the rewind screen to finish, and
  never show in the enemy phase apart from the enemy-phase tip itself.
