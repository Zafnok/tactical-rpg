---
id: "0012"
title: "Decide: the game's title"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0701"]
nick_input: decision
completed:
---

# 0012 — Decide: the game's title

## Context

Everything is called `tactical-rpg` until now. A title is needed before store
pages (09xx). Blocked by the story bible (0701) so names can fit the world.
Low priority.

## Nick input

**Decision.**

## Steps

1. Read `docs/story/bible.md` and `docs/design/setting-and-tone.md`.
2. Propose 6–8 titles in 2–3 styles (e.g. FE-style `<Proper noun>: <subtitle>`,
   single evocative word, roguelike-ish). For each: one line on why it fits.
   Do a quick web search on each to flag any existing game with the same or a
   confusingly similar name on Steam/itch.
3. Nick picks one, edits one, or gives his own.

## What to record

`docs/design/title.md` with the title, subtitle (if any), and Nick's words.
Update `README.md` heading. Create a ticket in `09xx` to rename the binary,
window title and store metadata (don't rename in this ticket).

## Acceptance criteria

- [ ] Nick chose a title.
- [ ] `docs/design/title.md` written, README heading updated, rename ticket created.
- [ ] Ticket archived.

## Completion notes

