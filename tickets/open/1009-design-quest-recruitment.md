---
id: "1009"
title: "Design recruitment by quests outside battle"
type: design-decision
milestone: Post–Chapter 1
model: opus-5.5
effort: medium
status: todo
blocked_by: ["1007"]
nick_input: decision
completed:
---

# 1009 — Design recruitment by quests outside battle

## Context

Nick (ticket 0705 review, `docs/design/battle-scenes-and-recruitment.md`):
"most recruitment triggers should be via defeating them or completing a
quest which happens outside of battle". Recruiting by defeating an enemy and
by talking in battle is built (0705) and joins the roster after a won battle
(0801). Recruiting through quests needs the world map (1007), where towns
may hold "recruitable wanderers" (`world-structure.md`).

## Nick input

**Decision** (with the `ask-nick` skill): what a recruitment quest is (a
side quest battle, a town request, a condition over several chapters…),
where it is offered, how the player sees its progress, and when the
character joins.

## Scope

**In:**
- Ask Nick; record the answers in `battle-scenes-and-recruitment.md`.
- Write the implementation ticket(s) the answers call for.

**Out (do not do):**
- Implementing quests.

## Implementation steps

1. Draft options from real games (e.g. Fire Emblem Three Houses' recruit
   requirements, Unicorn Overlord's side quests, Triangle Strategy's
   recruit conditions) and ask Nick.
2. Record the decision and file the implementation tickets.

## Acceptance criteria

- [ ] The design doc has Nick's words and a rule set concrete enough to build.
- [ ] Implementation ticket(s) filed.

## Tests required

- None (design decision).

## Completion notes
