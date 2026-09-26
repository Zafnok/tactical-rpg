---
id: "0018"
title: "Decide: Combat Arts for ranks C–S and the first special weapons"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0014"]
nick_input: decision
completed:
---

# 0018 — Decide: higher-rank arts and special weapons

## Context

Ticket 0014 (`docs/design/combat-arts.md`) decided how Combat Arts work and
the Chapter 1 list: two arts per weapon kind, at ranks E and D. Tier-1
classes reach rank C, and tier-2 classes reach A, so later chapters need arts
for ranks **C, B, A and S**. Nick also chose **special weapons** that carry
their own art (his example: a unique blade, like Dark Souls' Moonlight
Greatsword, whose art makes the combat hit Res). None exist yet, since Chapter
1 has no loot. Until this is decided, reaching rank C teaches nothing new.
Run with the `ask-nick` skill, ideally after the Chapter 1 playtest (0804),
so the starter arts have been felt first.

## Nick input

**Decision.**

## Questions to ask

1. **Arts for ranks C–S**, per weapon kind, with exact effects, costs and
   ranges. Draw them from Three Houses, Fortune's Weave and Engage, keep our
   own names, and avoid repeating class actives' functions (Nick: overlaps
   "should make sense and be rare"). Show a forecast mockup.
2. **Special weapons:** the first few (for Chapters 2–3), their stats and
   their own arts, including Nick's Res-hitting blade.
3. **Playtest follow-ups:** after 0804, does any tier 1–2 active need a new
   flavour so it overlaps arts less (Heavy Blow, Lance Rush, Dive are "might
   +N")?

## What to record

Extend `docs/design/combat-arts.md` (Later ranks, special weapons). Create
the implementation ticket(s) with the `write-ticket` skill (data in
`arts.ron`/`items.ron` on top of 0312; any new effect kinds in `core`).
Update `docs/design/README.md`.

## Acceptance criteria

- [ ] Nick answered 1–3.
- [ ] `combat-arts.md` updated with exact numbers.
- [ ] Implementation tickets created; design README updated.
- [ ] Ticket archived.

## Completion notes
