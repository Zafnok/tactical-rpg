---
id: "0706"
title: Chapter 1 cast portraits (ASCII art, all expressions)
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: todo
blocked_by: ["0701", "0703", "0011"]
nick_input: sign-off
completed:
---

# 0706 — Chapter 1 cast portraits

## Context

Art for every character who speaks in Chapter 1, using the portrait briefs in
`docs/story/characters/*.md` (0701), the style Nick picked in
`docs/design/look-and-feel.md` (0011), and the `ascii-art` skill.

## Nick input

**Sign-off:** Nick sees a screenshot of each portrait (neutral + one other
expression) and approves or comments; iterate on the ones he dislikes.

## Scope

**In:** one `.portrait` per Chapter 1 speaking character, all required
expressions (+ any extra ones listed in the character sheet), palette entries
they need.

**Out:** generic enemy portraits beyond one shared "soldier" portrait; later chapters.

## Implementation steps

1. List Chapter 1 speakers from `docs/story/chapters/ch01.md`.
2. For each: read the portrait brief; sketch silhouette first; author
   `neutral`; derive other expressions by changing only eyes/brows/mouth cells.
3. Render every portrait in the debug viewer (0703) and look at it before
   moving on (read the screenshot). Fix proportions (cells are 2:1 tall).
4. One generic `soldier` portrait for unnamed enemies.
5. Screenshot sheet for Nick (a debug "contact sheet" screen or one
   screenshot per character), revise per feedback.

## Acceptance criteria

- [ ] Every Chapter 1 speaker has a validated portrait with all required expressions.
- [ ] Characters are distinguishable by silhouette in a greyscale screenshot (check by rendering with colours disabled or by eye with a desaturated screenshot).
- [ ] Nick approved.

## Completion notes

