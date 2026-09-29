---
id: "0022"
title: "Decide: key rebinding rules (slots, required keys, conflicts, per-layout keys)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-29
---

# 0022 — Decide: key rebinding rules

## Context

Nick asked (2026-09-29) for key remapping in Options: no key may be
hard-coded, every key comes from a player-editable config, and a key-binding
screen takes any key press and resolves conflicts by moving the key and
flagging the action that lost it (`! not mapped`). Some actions are required
(cursor, confirm, cancel, end turn), others optional, including two new
optional split keys (Select vs Confirm, End turn vs Confirm end turn).

0805 already had a rebinding step, written by Claude before Nick was asked
("ask to swap" on conflicts); this ticket replaces it with Nick's rules.
Asked with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

1. What happens when leaving the screen while a required action has no key?
2. How many keys per action?
3. Custom keys when switching right/left-handed layout?
4. How to back out of "Press a key…" (is `Esc` bindable)?
5. How to empty a slot?

## Acceptance criteria

- [x] Answers recorded in `docs/design/controls.md` (*Rebinding keys*) with
      Nick's words verbatim.
- [x] `docs/design/README.md` row updated.
- [x] Downstream tickets written/updated.

## Completion notes

- Recorded in `docs/design/controls.md`, *Rebinding keys*: 3 slots per
  action (one needed); required = cursor ×4, Confirm, Cancel, End turn;
  a taken key moves and the loser shows `! not mapped`; leaving is blocked
  while a required action has no key; `Esc` backs out and is never bindable;
  `Delete` empties a slot and is never bindable; each layout keeps its own
  custom keys; new optional Select and Confirm-end-turn keys, unbound by
  default.
- Claude's starting rules (listed in the design doc for Nick's veto):
  `Esc` doesn't count as Cancel's required key; `Delete` may empty a
  required action's last key; blocked-leave message wording; a Restore
  defaults row; which presses count as Select "on the map".
- New tickets: 0216 (audit hard-coded keys, prerequisite), 0217 (player
  key-bindings config), 0218 (optional Select / Confirm end turn keys),
  0809 (Key bindings screen). 0805 now opens 0809's screen instead of
  building its own rebinding UI.
- New skill: `.claude/skills/keyboard-input` (never hard-code keys).
