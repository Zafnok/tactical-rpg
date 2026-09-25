---
id: "0010"
title: "Decide: supports and character relationships"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0010 — Decide: supports and relationships

## Context

Nick wants "story (both overall and personal)". Personal stories in FE mostly
live in **support conversations**. Not needed for Chapter 1 — low priority —
but the answer shapes the character sheets (0701). Nick said in 0007 that the
supporting cast should have "some arcs or role to play in the story etc as well
as support conversations", so supports of some kind are expected. Run with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

### Q1. How are personal stories told?

**A. FE supports (GBA / Path of Radiance)** — pairs of units build support
points by fighting next to each other; at thresholds a C / B / A conversation
unlocks (two portraits talking). Supported pairs get small combat bonuses when
adjacent. Paired endings. Feel: you *earn* the character stories through play.

**B. Hub activities (FE Three Houses)** — between battles, a base where you
talk to units, share meals, give gifts. Feel: dating-sim-adjacent, lots of content.

**C. Camp events (Triangle Strategy, Unicorn Overlord)** — scripted optional
conversations appear between chapters; no grinding for them.

**D. Main-script only** — personal arcs happen inside story scenes.

**Recommendation:** A — fits our two-portrait dialogue system perfectly and
adds tactical meaning to positioning.

### Q2 (if A). Support limits

**A. Unlimited supports per unit.** **B. Max 5 A-ranks per unit (FE GBA).**
Recommendation: B.

**No romance option:** 0007 ruled out romance, S ranks, marriage and shipping
([`setting-and-tone.md`](../../docs/design/setting-and-tone.md)). Don't offer
it. Supports are friendship, rivalry, mentorship and family (pre-existing
relationships only).

## What to record

`docs/design/supports.md`: Nick's words; the chosen system; if A: point gain
rules (e.g. +1 per turn ending adjacent, +3 per combat adjacent, *tunable*),
thresholds, bonuses, limits; which conversations exist per pair is left to the
story pipeline. Then create a `10xx` implementation ticket (don't implement).

## Acceptance criteria

- [ ] Nick answered Q1 (and Q2 if relevant).
- [ ] `docs/design/supports.md` written; follow-up `10xx` ticket created.
- [ ] Design README updated; ticket archived.

## Completion notes

