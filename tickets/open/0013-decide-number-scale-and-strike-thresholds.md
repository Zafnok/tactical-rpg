---
id: "0013"
title: "Decide: number scale and strike thresholds"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0804"]
nick_input: decision
completed:
---

# 0013 — Decide: number scale and strike thresholds

## Context

Ticket 0001 fixed the stat list and combat rules
(`docs/design/stats-and-combat.md`), including "up to 4 strikes, 3x/4x rare".
Nick deferred **how big the numbers are** until after he has played: "maybe I
wanna be crazy like dragon ball and we're talking trillions of points endgame
who knows... figure out the scaling algorithm once I have a playtest and see if
big numbers or low numbers or even medium numbers look nicer to me."

Chapter 1 ships with FE-sized placeholder numbers (stats ~0–50, HP ≤ 80,
strike thresholds `[4, 14, 24]`). This ticket asks Nick, after the Chapter 1
playtest (0804), which scale he wants, and derives the strike-threshold rule
from it.

## Nick input

**Decision.** Run with the `ask-nick` skill after Nick has played the 0804
build. Show real numbers from the build next to mock-ups at other scales.

## Questions to ask

### Q1. How big should numbers get?

Show the same unit's info panel and one attack forecast at each scale (ASCII
mock-ups at the real panel width):

**A. Small — like Into the Breach / Advance Wars:** HP 3–10, damage 1–4.
**B. FE-sized (current) — like Fire Emblem:** stats 5–40, HP 20–60.
**C. Medium — like Final Fantasy Tactics / Disgaea early game:** HP in the
hundreds, damage 30–300.
**D. Huge — like Disgaea late game / Dragon Ball power levels:** grows to
millions or more; displayed with suffixes (`1.2M`, `3.4B`, `5T`).

Recommendation: decide from the playtest; mention panel width limits for C/D.

### Q2 (follows from Q1). When do 3 and 4 strikes happen?

Offer: fixed Speed gaps (current: +4 / +14 / +24), gaps as a percentage of the
enemy's Speed (e.g. 2x at 125%, 3x at 175%, 4x at 250%; scale-free), or a
ratio table. Show how often each would occur with the Chapter 1 roster.

## What to record

Update `docs/design/stats-and-combat.md`: Nick's words; the scale; stat
ranges/ceilings; the exact strike rule and thresholds; how large numbers are
displayed if C/D. Update `docs/design/README.md`. Write follow-up tickets for
rebalancing data (`classes.ron`, weapons, terrain) and, if needed, widening
`StatValue` (0302 made it a single alias) and number display in the UI.

## Acceptance criteria

- [ ] Nick answered Q1 and Q2 after playing.
- [ ] `stats-and-combat.md` updated with scale, ranges and strike rule.
- [ ] Follow-up rebalance / display tickets created.
- [ ] Ticket archived.

## Completion notes
