---
id: "0602"
title: EXP bar animation and the level-up screen
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: done
blocked_by: ["0601", "0404"]
nick_input: sign-off
completed: 2026-09-29
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
   `text_highlight`; stats that didn't grow shown dim without `+`. No `MAX`
   marker here: it is a battle screen (Nick, ticket 0019). Hold Confirm
   = reveal all; Confirm when done closes.
3. **Class progress** (`progression.md`): after the EXP bar, a one-line
   `Swordsman  CL 3 → 4` notice when a `ClassLeveledUp` happened, and a
   `CLASS MASTERED!  Learned: Keen Edge` banner on `ClassMastered`; any
   `SkillLearned` / `SpellLearned` gets a `Learned: <name>` line.
4. Sequencing: combat playback → EXP bar → (level-up) → (class progress) →
   back to battle. Same for heals and tile casts.

## Acceptance criteria

- [x] Numbers shown equal the `LeveledUp` event.
- [x] Harness: force a level-up via a scripted setup (unit at 99 EXP) → overlay appears → closes → state correct.
- [x] Snapshots of EXP bar, completed level-up screen and the mastery banner.

## Tests required

- Harness + snapshots as above; unit tests for reveal timeline.

## Completion notes

- New `crates/ui/src/screens/battle/progress.rs`: `Progress` builds pages
  from a command's events (per player unit, in the order they first gained
  something: EXP bar → one level-up page per `LeveledUp` → class progress)
  and times them; `draw` renders them. The battle screen keeps the pages of
  the last command and shows them once the combat playback ends (at once for
  a heal or tile cast, which have no playback). Banners (including the
  victory banner), tips, auto-end and rewind wait until they close.
- The level-up page draws the character's portrait (`happy`, else
  `neutral`, else its first expression); a unit with no portrait gets the
  same `portrait` placeholder box as the info screen.
- Tests: unit tests of the page order and reveal timeline in `progress.rs`;
  harness tests in `progress_tests.rs` (lord at 99 EXP levels up, the page
  shows the `LeveledUp` event's numbers, Confirm closes it and the battle
  equals `core`'s; a mastery; a heal going straight to the EXP bar);
  snapshots `exp_bar`, `level_up`, `class_mastered`. Existing attack, rewind
  and battle tests now close the EXP bar after a combat.

**Deviations:** the EXP bar is 20 cells (5 EXP each) instead of the ticket's
10-cell example, so the 0.6 s fill looks smooth. Several awards to one unit
in one command (e.g. Line Pierce's combats) show as one bar with the sum.

**Choices Claude made where the docs were silent** (presentation, not game
rules; Nick may veto any):
- The EXP bar closes by itself 0.5 s after it fills; a class-level notice
  with nothing else closes by itself after 1.2 s. The level-up page and a
  mastery/`Learned:` box wait for Confirm.
- Cancel does nothing on these pages, so a level up can't be skipped by
  accident; Confirm finishes the animation, then closes.
- A stat that didn't grow shows as `Def   5` (dim), with no arrow.
- A personal spell learned at a character level shows as `Learned: <spell>`
  in the class box (under the class's name) after the level-up page.
- At mastery the box lists the passives learned; the active that becomes
  permanent isn't listed (there's no event for it).
- EXP shared from green units at the battle's end shows each unit's bar
  before the victory banner.

**Follow-ups:** 0022 (Nick picks the EXP bar and level-up sounds) and 0605
(play them); `audio.md` lists "level up" as never asked.

