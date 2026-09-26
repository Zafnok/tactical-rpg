---
id: "1005"
title: "Parked: support pair abilities (dual attack / dual guard)"
type: research
milestone: Post–Chapter 1
model: opus-5.5
effort: medium
status: blocked
blocked_by: ["1002"]
nick_input: decision
completed:
---

# 1005 — Parked: pair abilities

## Context

In 0010 ([`docs/design/supports.md`](../../docs/design/supports.md)) Nick
chose a small flat Hit/Avoid bonus for supports "for now", adding that "pair
abilities might be added far in the future" (e.g. FE Awakening's dual strike
and dual guard). This ticket keeps the idea on record. **Do not start it**
until Nick asks for it.

## Nick input

**Decision:** when Nick wants it, run a `00xx` decision ticket (`ask-nick`
skill) with options such as Awakening's dual strike/guard, Engage's chain
attacks and Unicorn Overlord-style assists, and how they interact with
unlimited A-ranks and the existing bonus. Then replace this ticket with
implementation tickets.

## Scope

**In:** writing that `00xx` decision ticket when Nick revives the idea.

**Out (do not do):** any pair-ability rules or UI before Nick's decision.

## Implementation steps

1. Wait for Nick to ask for pair abilities.
2. Write the `00xx` decision ticket (`write-ticket` skill) and close this one
   as superseded.

## Acceptance criteria

- [ ] Nick asked for it, and a `00xx` decision ticket exists.

## Tests required

- None.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
