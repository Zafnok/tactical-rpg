---
id: "0034"
title: "Decide: how the web build starts its title music (press a key)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: low
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-30
---

# 0034 — Decide: how the web build starts its title music

## Context

Found by ticket 0223 (web console errors). Browsers block sound until the
player clicks or presses a key, so on the Pages build the title is silent
until the first input and the console warns "The AudioContext was not
allowed to start". The usual fix is a "click to start" step; Nick said a
mouse click is odd in a keyboard-only game and asked for a key instead.

## Nick input

**Decision**, asked with the `ask-nick` skill: which key starts, how the
prompt looks, and whether it is web only.

## Scope

**In:** record the answer in `docs/design/title-screen.md`; write the
implementation ticket.

**Out:** any code.

## Acceptance criteria

- [x] `docs/design/title-screen.md` records Nick's words and the rules.
- [x] `docs/design/README.md` lists it.
- [x] Implementation ticket written (0224).

## Completion notes

Nick picked **any key**, shown as a `Press any key` line on the title screen
where the menu goes (like GBA Fire Emblem's "Press Start"), **web build
only, for now**. Claude's starting rules (listed in the design doc): the
dismissing key does nothing else, the line is dim and still, the help line
is hidden until the menu shows, a click doesn't dismiss it, and it shows
once per page load. Built by ticket 0224. Ticket 0811 (title art) now notes
the prompt line.
