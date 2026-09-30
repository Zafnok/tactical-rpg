---
id: "0012"
title: "Decide: the game's title"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: ["0701"]
nick_input: decision
completed: 2026-09-29
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

- [x] Nick chose a title.
- [x] `docs/design/title.md` written, README heading updated, rename ticket created.
- [x] Ticket archived.

## Completion notes

Nick chose **Visions of Shuyi** (VoS), with no subtitle. "Shuyi" is an homage to
his wife. Recorded in `docs/design/title.md` with his words.

- He first dropped the subtitle styles ("it is the first game"), then picked
  from six "V… of Shuyi" titles.
- Story role (Nick fixed): Shuyi is the Jade Reach's guardian of the lifeblood
  its people worship. Nobody has seen it in ages, so many call the lifeblood
  itself "Shuyi". Added to the bible glossary and the name registry (id
  `shuyi`, marked fixed because it's in the title).
- Left open: how big Shuyi's role is, what the guardian is, and how it relates
  to Breath and the vows. The Act 2 story tickets must ask Nick.
- Web search (2026-09-29): no Steam or itch.io game uses "Shuyi". The nearest
  are *Shuyan Saga* and *Shu*.
- Follow-up: **0905**, rename the binary, window, title screen, web page and
  release packages, without moving save data.
- No Claude's starting rules: this ticket made no gameplay decisions.
