---
id: "1004"
title: "Parked: hub activities between chapters (Three Houses style)"
type: research
milestone: Post–Chapter 1
model: opus-5.5
effort: medium
status: blocked
blocked_by: ["1003"]
nick_input: decision
completed:
---

# 1004 — Parked: hub activities

## Context

In 0010 ([`docs/design/supports.md`](../../docs/design/supports.md)) Nick said
he likes Three Houses-style hub activities (walk a base, meals, gifts) but
that they risk bloat and "dating sim vibes": "Hub activities we can add much
later when we've refined the core loop. It should be a ticket in the far
future or maybe never implemented due to scope explosion." This ticket keeps
the idea on record. **Do not start it** until Nick asks for it.

## Nick input

**Decision:** when Nick wants it, run a `00xx` decision ticket (`ask-nick`
skill) on what a hub would contain, how it ties into supports without romance,
and how much content it may add. Then replace this ticket with implementation
tickets.

## Scope

**In:** writing that `00xx` decision ticket when Nick revives the idea.

**Out (do not do):** any hub code or content before Nick's decision.

## Implementation steps

1. Wait for Nick to ask for hub activities.
2. Write the `00xx` decision ticket (`write-ticket` skill) and close this one
   as superseded.

## Acceptance criteria

- [ ] Nick asked for it, and a `00xx` decision ticket exists.

## Tests required

- None.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
