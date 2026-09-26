---
id: "1003"
title: Supports and Camp lists between chapters
type: feature
milestone: Post–Chapter 1
model: sonnet-5
effort: medium
status: todo
blocked_by: ["1002", "0408", "0704", "0801"]
nick_input: sign-off
completed:
---

# 1003 — Supports and Camp lists between chapters

## Context

[`docs/design/supports.md`](../../docs/design/supports.md) (ticket 0010):
once unlocked, support conversations are read **at camp** (the
between-chapters screen, before Preparations) from a **Supports** list, next
to a list of optional scripted **camp events** (extras, Triangle Strategy
style). The rules and state come from 1002, the dialogue player from
0702/0704, and the between-chapters flow from 0801/0408.

## Nick input

**Sign-off:** Nick plays to the end of a chapter, opens Supports and Camp,
reads a conversation of each kind, and says whether it's easy to see what's
new.

## Scope

**In:**
- A **Supports** screen: every pair with an unlocked or seen conversation,
  grouped by unit, showing the rank (C/B/A) and "NEW" on unlocked, unviewed
  ones. Picking one plays it with the dialogue player and sends 1002's view
  command. Seen conversations can be re-read.
- A **Camp** screen: camp events available now (conditions: after chapter N,
  listed units alive and recruited), "NEW" markers, played with the dialogue
  player. Seen events are remembered in the campaign state and saved.
- A camp event data format (id, conditions, script id) with content
  validation.
- A **Camp** screen in the between-chapters flow (before Preparations) that
  holds both lists.
- The unit info screen lists the unit's support partners and ranks.
- One placeholder support conversation and one placeholder camp event in the
  data, so the screens can be tested.

**Out (do not do):**
- Writing real support or camp conversations (story pipeline tickets).
- Rules changes (1002), hub activities (1004), pair abilities (1005).

## Implementation steps

1. Read `supports.md`, 1002's code, and the 0408 and 0704 screens.
2. Add the camp event format and loader, and campaign state for seen events.
3. Build the two list screens with the existing menu widgets and help bar.
4. Wire them into the between-chapters flow and the unit info screen.

## Acceptance criteria

- [ ] The Supports list shows unlocked and seen conversations with rank and NEW.
- [ ] Viewing a support conversation raises the rank (1002 rules).
- [ ] The Camp list shows only events whose conditions are met; seen state is saved.
- [ ] Unit info lists supports and ranks.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: camp event conditions; seen-state tracking.
- Snapshot: Supports list, Camp list, unit info support section.
- Integration: scripted keys open Supports, play a NEW conversation, then
  check that the rank went up and NEW is gone.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
