---
id: "0007"
title: "Decide: setting, tone, and Nick's story beats"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0007 — Decide: setting, tone, and Nick's story beats

## Context

Nick can give "a few story beats but not the whole character arcs". This ticket
captures his beats and the high-level direction; the story pipeline
(ADR-0011, tickets 07xx) turns them into a bible, characters and scripts.
Run with the `ask-nick` skill. Keep it light: Nick shouldn't have to write prose.

## Nick input

**Decision** + whatever beats he wants to give.

## Questions to ask

### Q1. Setting

**A. Classic medieval fantasy war (Fire Emblem)** — kingdoms, a young lord,
knights and dragons, an ancient evil stirring.
**B. Grim political low fantasy (Tactics Ogre, FFT, Triangle Strategy)** —
class war, betrayal, no clean good side, faith and power corrupted.
**C. Weird / science-fantasy (Caves of Qud, Final Fantasy VI)** — ruins of a lost
high-tech age, mutants, magitek. Plays to the ASCII roguelike heritage.
**D. Something else** — Nick describes it (mythological Japan, Norse, gothic
horror, pirates…).

### Q2. Tone

**A. Earnest and heroic** (FE Blazing Blade). **B. Dark with warmth and humour**
(FE Path of Radiance, Triangle Strategy). **C. Grim and tragic** (Tactics Ogre).
**D. Light-hearted** (Disgaea).
**Recommendation:** B — lets character moments land between hard story beats.

### Q3. Who is the player following?

**A. A single lord** whose death is game over (FE).
**B. A mercenary company** with a captain (FE Path of Radiance's Greil
Mercenaries, Battle Brothers). **C. An ensemble with rotating viewpoints**
(Triangle Strategy, FE Three Houses factions).

### Q4. Your beats (free text)

Prompt Nick with, and accept any subset of:
- How does the story start? (inciting incident)
- A twist or midpoint he'd love.
- How should it end (or feel at the end)?
- Any characters he already imagines (even one line each).
- Anything off-limits or he's tired of seeing?
- Favourite stories/games/films to draw vibes from.

## What to record

1. `docs/story/beats.md` — Nick's answers and beats **verbatim**, clearly
   labelled as canon. Nothing added.
2. `docs/design/setting-and-tone.md` — the Q1–Q3 decisions in a few lines each,
   plus a short "vibe references" list.

Update `docs/design/README.md` and `docs/story/README.md` status.

## Acceptance criteria

- [ ] Nick answered Q1–Q3 and gave any beats he wants to (even zero is OK).
- [ ] `docs/story/beats.md` contains his words verbatim.
- [ ] `docs/design/setting-and-tone.md` written.
- [ ] READMEs updated; ticket archived.

## Completion notes

