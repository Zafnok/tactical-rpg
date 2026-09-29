---
id: "0804"
title: Nick playtests Chapter 1; feedback becomes tickets
type: playtest
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0803", "0802", "0602", "0307", "0406", "0107", "0108", "0414", "0417", "0424", "0425", "0710", "0807", "0215", "0809", "0810"]
nick_input: sign-off
completed:
---

# 0804 — Chapter 1 playtest

## Context

The first milestone Nick plays. He said he'll judge the game by playing and
will report comments, concerns and bugs. This ticket packages a build, gets
his feedback, and turns it into tickets. **No code changes in this ticket.**

**Blockers added 2026-09-29**, so Nick's first impression includes what's
already designed: Combat Arts in the attack flow (0414), boss arts (0503, via
0803), his uppercase-label request (0417), battle and menu sounds (0424,
0425), scene and battle music (0710, 0807), no freeze when music loads
(0215), victory/defeat/game-over music (0809) and the battle results screen
(0810), without which step 4's question about the rewind EXP bonus can't be
answered because the bonus is never shown.

## Nick input

**Sign-off:** play Chapter 1 (≈15–25 min) and report anything: bugs, feel,
difficulty, story, looks, controls.

## Implementation steps

1. Make sure `main` is green and deployed to Pages (0108). Cut `v0.1.0` via
   the release workflow (0107): bump version, merge, tag.
2. Send Nick a short message with:
   - Links: browser build (Pages) and Windows zip (GitHub Release).
   - A key cheat-sheet (from `keymap.ron`, not from memory).
   - What to pay attention to (5 bullets max): cursor feel, readability of the
     map, whether combat numbers make sense, difficulty, whether the story hooks him.
   - "Just tell me in plain words; screenshots welcome."
3. For each piece of feedback, apply the `write-ticket` skill's
   "Turning Nick's playtest feedback into tickets" section. Ask at most one
   clarifying question per unclear item.
4. Ask Nick whether the unused-rewind-charge EXP bonus (7% of a level per
   charge, `death-and-difficulty.md`) felt right, and record any change.
5. Put a summary in Completion notes: feedback verbatim + ticket numbers created.

## Acceptance criteria

- [ ] Nick played and gave feedback.
- [ ] Every feedback item maps to a ticket (or an explicit "won't do" with reason Nick agreed to).
- [ ] `v0.1.0` release exists.

## Completion notes

