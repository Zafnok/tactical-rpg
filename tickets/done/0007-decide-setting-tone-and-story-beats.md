---
id: "0007"
title: "Decide: setting, tone, and Nick's story beats"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
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

- [x] Nick answered Q1–Q3 and gave any beats he wants to (even zero is OK).
- [x] `docs/story/beats.md` contains his words verbatim.
- [x] `docs/design/setting-and-tone.md` written.
- [x] READMEs updated; ticket archived.

## Completion notes

- Nick answered Q1 **A** (FE-style medieval war, with other continents that
  feel different later, e.g. wuxia-inspired Asia), Q2 **B** (dark with warmth
  and humour) and Q3 **A** (single lord for now; ensemble maybe later;
  supporting cast still gets arcs and supports).
- Q4 beats: the ending must be a resolved, hard-fought victory (not a
  tragedy), and scripted losses are OK. **No romance or shipping.** Vibe list
  recorded. Nick deferred the inciting incident and twist until the lead is
  defined, and asked for leading questions about characters.
- Extra follow-up round on the lead (deviation from the ticket, needed because
  Nick wanted to "anchor on the main character first"): disgraced/exiled
  noble, aged 18–25, **Persona-style lead with player-picked reply tones that
  don't branch the plot, and player-chosen gender**.
- Recorded in `docs/story/beats.md` (verbatim, canon) and
  `docs/design/setting-and-tone.md` (rules, vibe references, open
  sub-questions).
- Follow-up ticket created: **0708** Dialogue: lead reply choices and lead
  name/pronoun tokens.
- Downstream tickets updated: 0008 (continents), 0010 (no romance/S rank),
  0701 (lead sheet rules, gate-1 questions on disgrace/cast/renaming, gate-2
  twist options), 0702 (out-of-scope pointer to 0708), 0706 (two lead
  portraits), 0707 (lead rules, blocked by 0708), 0801 (lead gender select,
  `Campaign.lead`, blocked by 0708). Roadmap graph updated.
