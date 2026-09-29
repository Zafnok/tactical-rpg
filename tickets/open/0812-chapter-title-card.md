---
id: "0812"
title: Chapter title card before a chapter starts
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0801"]
nick_input: decision
completed:
---

# 0812 — Chapter title card

## Context

0801's chapter file has a `title` ("Chapter 1: …"), and 0802's save slots
show it, but no screen ever shows it to the player when a chapter begins.
Fire Emblem games show the chapter number and name on a card before the
chapter's opening scene. Found while listing Chapter 1 gaps (2026-09-29).

It's player-facing UI, so Nick decides whether there is a card and what it
looks like; don't pick one as if it were decided.

## Nick input

**Decision** with the `ask-nick` and `ascii-art` skills. Render mockups at the
game's size in palette D:

- **A. No card:** the chapter starts with its first scene (today's 0801 flow).
- **B. Plain card:** "Chapter 1" and the chapter name, centred on black, for
  a few seconds or until Confirm.
- **C. Card with a border or small glyph ornament** in the style of the
  phase banners (0405).
- **D.** "Describe your own."

## Scope

**In:**
- If Nick picks a card: a `ChapterCardScreen` pushed by `ui::flow` before the
  chapter's `intro_scenes`, showing the chapter file's `title` (with name
  tokens from 0709 if titles use them). It closes after a set time or on
  Confirm; Cancel also closes it.
- Record the choice in `docs/design/look-and-feel.md`.

**Out (do not do):**
- Transitions between screens (0813).
- Music on the card (the intro scene's `@music` decides, 0710).

## Implementation steps

1. Run the decision; record it.
2. If a card: add the screen, push it from `ui::flow`, and add a snapshot.

## Acceptance criteria

- [ ] Nick's choice is recorded.
- [ ] If a card: Harness test New Game → card shows the test chapter's title
      → Confirm → first intro scene; snapshot of the card.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot / integration: Harness flow and card snapshot (if a card).

## Completion notes

*(Filled in by the session that completes the ticket.)*
