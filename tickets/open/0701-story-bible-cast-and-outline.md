---
id: "0701"
title: Story bible, main cast sheets, act outline, Chapter 1 beat sheet
type: content
milestone: M6 Story & dialogue
model: fable-5.1
effort: high
status: todo
blocked_by: ["0004", "0007", "0008"]
nick_input: sign-off
completed:
---

# 0701 — Story bible, cast, outline

## Context

Steps 2–4 of the story pipeline in
[ADR-0011](../../docs/adr/0011-story-authoring-pipeline.md). Turns Nick's beats
(`docs/story/beats.md`) into a world, a cast with personal arcs, and an
outline. Follow the `story-writing` skill strictly.

## Nick input

**Sign-off at two gates** (keep each to a one-page summary):
1. World + cast summary.
2. Act outline.

## Scope

**In:** `docs/story/bible.md`, `docs/story/characters/*.md`,
`docs/story/outline.md`, `docs/story/chapters/ch01.md`, `docs/story/ledger.md`.

**Out:** dialogue scripts (0707), portraits (0706), game data files.

## Implementation steps

1. Read: `beats.md`, `docs/design/setting-and-tone.md`, `magic.md`,
   `world-structure.md`, `supports.md` (if exists), `chapter-1.md` (if exists),
   `progression.md` (class names to use for characters).
2. **bible.md:** world premise (1 paragraph), geography (5–8 named places, one
   line each), history (the event the story grows from), factions (3–5, what each
   wants), themes (2–3), tone rules (what jokes are OK, how dark it gets), rules
   of magic/tech consistent with `magic.md`, glossary.
3. **Cast:** 8–12 characters for Act 1: the lord/protagonist, 4–6 playable
   companions (covering the Chapter 1 roster classes), 1 antagonist with an
   understandable motive, 1–2 recurring secondary villains/rivals, 1–2 NPCs.
   Each `characters/<id>.md` follows the skill: role, class, want, need, flaw,
   secret/pressure, arc (start → end), relationships (who they clash/bond
   with and why), voice notes + 3 sample lines, **portrait brief** (silhouette,
   hair, clothing, colours — used by 0706), expression list (at least the five
   standard ones).
4. **Gate 1:** send Nick a one-page summary (premise, factions in a line each,
   cast in a line each). Revise until he approves. Record his feedback verbatim
   at the bottom of `bible.md`.
5. **outline.md:** Act 1 in detail (6–10 chapters: goal, conflict, turn, map
   idea, which personal arcs advance, recruits); Acts 2–3 as a paragraph each
   with the ending. Every chapter must advance the main plot **and** a personal arc.
6. **Gate 2:** one-page outline summary to Nick; revise until approved.
7. **chapters/ch01.md:** scene-by-scene beat sheet matching `chapter-1.md`
   (pre-battle scenes, in-battle moments incl. boss/talk-recruit if any,
   post-battle), each scene with purpose and what changes.
8. **ledger.md:** initial state of each character (what they know, relationships).
9. Critique pass per the skill before committing.

## Acceptance criteria

- [ ] Nick approved gate 1 and gate 2 (quotes recorded).
- [ ] All files exist with every required section; no contradiction with `beats.md`.
- [ ] Chapter 1 beat sheet fits `chapter-1.md` (roster size, objective).
- [ ] `docs/story/README.md` status table updated.

## Completion notes

