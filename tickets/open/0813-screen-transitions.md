---
id: "0813"
title: Transitions between screens (title, scenes, battle)
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0801"]
nick_input: decision
completed:
---

# 0813 — Screen transitions

## Context

Screens replace each other instantly (screen stack, ADR-0017, ticket 0205): title →
mode select → lead select → scene → battle → scenes → "To be continued". Only
music fades (0212). Found while listing Chapter 1 gaps (2026-09-29).

How screens change is look and feel, so Nick decides; don't add a transition
as if it were decided.

## Nick input

**Decision** with the `ask-nick` skill. Show each option as a short recorded
or animated mockup (an Artifact page that plays it), for example:

- **A. Instant cut** (today).
- **B. Fade to black and back**, about 0.3 s each way (*tunable*).
- **C. Glyph wipe:** cells turn to blank row by row or column by column.
- **D.** "Describe your own."

Sub-question: the same transition everywhere, or only between big steps
(scene ↔ battle) and a cut inside menus.

## Scope

**In:**
- A transition done in `ui` over the glyph buffer (e.g. dimming every
  cell's colours toward black), driven by the frame clock `ui` already gets
  (ADR-0004: no clock in `core`). Input is ignored while it plays.
- Used between the screens Nick names.
- Respect 0805's animation setting if it exists (`anim_speed`): Fast halves
  the time. If 0805 isn't done, add a note there.
- Record the choice in `docs/design/look-and-feel.md`.

**Out (do not do):**
- Transitions inside the battle screen (phase banners, combat playback).
- Chapter title card (0812).

## Implementation steps

1. Run the decision; record it.
2. Add the transition to the screen stack or `ui::flow` so any `Transition`
   can request it.
3. Harness tests use the existing frame stepping to run it to the end.

## Acceptance criteria

- [ ] Nick's choice is recorded.
- [ ] Unit: the transition's colour at 0 %, 50 % and 100 % of its time.
- [ ] Harness: title → New Game still reaches the same screens; input during
      a transition is ignored.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: transition blend.
- Snapshot / integration: a mid-transition snapshot; existing flow tests
  still pass.

## Completion notes

*(Filled in by the session that completes the ticket.)*
