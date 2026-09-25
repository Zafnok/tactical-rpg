---
id: "0602"
title: EXP bar animation and the level-up screen
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0601", "0404"]
nick_input: sign-off
completed:
---

# 0602 — EXP bar and level-up screen

## Context

The dopamine moment Nick asked for ("unit progression (level ups…)"). Shows
`ExpGained` and `LeveledUp` events after combat playback.

## Nick input

**Sign-off:** does the level-up feel exciting?

## Scope

**In:** EXP bar overlay, level-up overlay, sequencing after combat playback.

**Out:** sound effects (future audio ticket), promotion UI (0603).

## Implementation steps

1. **EXP bar:** after combat playback, if a player unit gained EXP: small box
   `Ana  EXP ████████░░ 80` filling from old to new over ~0.6 s (wrapping at
   100 if levelled). Hold Confirm to speed up.
2. **Level-up overlay:** portrait area (placeholder box until 0703 exists; use
   the portrait if it does), `LEVEL UP!` banner, `Lv 4 → 5`, then each stat on
   its own row revealed one per ~0.12 s: `Str  6 → 7  +1` with `+1` in
   `text_highlight`; stats that didn't grow shown dim without `+`. Hold Confirm
   = reveal all; Confirm when done closes.
3. **Class progress** (`progression.md`): after the EXP bar, a one-line
   `Swordsman  CL 3 → 4` notice when a `ClassLeveledUp` happened, and a
   `CLASS MASTERED!  Learned: Keen Edge` banner on `ClassMastered`; any
   `SkillLearned` / `SpellLearned` gets a `Learned: <name>` line.
4. Sequencing: combat playback → EXP bar → (level-up) → (class progress) →
   back to battle. Same for heals and tile casts.

## Acceptance criteria

- [ ] Numbers shown equal the `LeveledUp` event.
- [ ] Harness: force a level-up via a scripted setup (unit at 99 EXP) → overlay appears → closes → state correct.
- [ ] Snapshots of EXP bar, completed level-up screen and the mastery banner.

## Tests required

- Harness + snapshots as above; unit tests for reveal timeline.

## Completion notes

