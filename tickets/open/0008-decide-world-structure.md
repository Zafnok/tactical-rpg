---
id: "0008"
title: "Decide: world structure (linear, world map, branching, open world)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0008 — Decide: world structure

## Context

Nick said "maybe there can be some open-worldedness as well? idk it's an
option." Chapter 1 is built as a linear chapter regardless (0801); this
decision shapes the long-term roadmap (10xx tickets) and the story outline
(0701). Run with the `ask-nick` skill.

In 0007 Nick also said the game starts in European-style kingdoms "with
potential for additional continents that might feel different (i.e. ... travel
to Asia and encounter wuxia inspired encounters...)"
([`setting-and-tone.md`](../../docs/design/setting-and-tone.md)). Ask how
travel between continents fits the structure he picks (e.g. a later act moves
to a new continent, or the world map expands).

In 0009 Nick said a **chapter is a story beat, not a single battle**: it often
ends with a climactic battle, and some chapters may hold several battles first
(e.g. overworld-triggered skirmishes to train before the climax)
([`chapter-1.md`](../../docs/design/chapter-1.md)). Ask how those skirmishes
fit the structure he picks (optional or required, repeatable, where they
trigger), and create follow-up tickets for multi-battle chapters.

Also ask Nick about **saving and camp in multi-battle chapters** (raised in
0010). Today `death-and-difficulty.md` only offers a save after a chapter
victory, plus a one-time suspend in battle. A chapter full of skirmishes
could then go a long time without a save. Ask when saving is allowed (e.g.
after every battle, or anywhere on the overworld like FE Sacred Stones).
Also ask where **camp** appears (`supports.md`: support conversations and
camp events are read at camp, which sits between battles). Record the save
answer in `death-and-difficulty.md` and the camp answer in `supports.md`.
Update 0802 (save/load) and 1003 (camp screens) to match.

## Nick input

**Decision.** Nick can also answer "decide later" — record that.

## Questions to ask

### Q1. How does the player move through the game?

**A. Linear chapters (FE Blazing Blade, FE Path of Radiance)** — story → battle
→ story → next battle. Feel: tight, cinematic pacing; best for authored
character arcs. Least to build.

**B. World map with optional battles (FE Sacred Stones, FE Echoes)** — an
overworld of nodes; story battles unlock paths; optional skirmishes and towers
for training and loot; towns with shops. Feel: "open-world lite" — some
freedom and grinding, story stays authored.

**C. Branching story routes (Tactics Ogre, Triangle Strategy, FE Three Houses)** —
key choices split the campaign into different chapters and endings. Feel: big
replay value; much more writing and map work.

**D. Sandbox campaign (Battle Brothers, Mount & Blade)** — roam a procedurally
generated world, take contracts, story events trigger along the way.
Feel: endless replay, weakest authored story.

**Recommendation:** A for Chapter 1 and early chapters, grow into B. That gives
the "some open-worldedness" feel without sacrificing the story Nick wants.

### Q2 (if B or D). What's on the overworld?

Towns/shops, optional skirmishes, recruitable wanderers, side quests with
character stories, random encounters (FFT-style) — let Nick pick what excites
him.

## What to record

`docs/design/world-structure.md`: Nick's words; the chosen structure; when it
starts (e.g. "world map unlocks after Chapter 3"); overworld contents if any;
implications for the story outline. Then create **one** placeholder ticket in
`10xx` (e.g. `1001-world-map-screen.md`, fully written per `write-ticket`) if
Nick chose B/C/D — do not implement.

## Acceptance criteria

- [ ] Nick answered Q1 (or chose "decide later").
- [ ] `docs/design/world-structure.md` written.
- [ ] Follow-up `10xx` ticket created if applicable.
- [ ] Design README updated; ticket archived.

## Completion notes

