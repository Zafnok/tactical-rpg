---
id: "0005"
title: "Decide: level ups, class changes and the class tree"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0001"]
nick_input: decision
completed:
---

# 0005 — Decide: level ups, class changes and the class tree

## Context

Nick specifically wants unit progression: level ups and class changes "like
Fire Emblem". Needed by the unit model (0302), progression (0601–0603) and the
Chapter 1 roster (0803). Blocked by 0001 because growth applies to the chosen
stat list. Run with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

### Q1. How do level ups work?

Show an example level-up for each, e.g. `Lv 4→5: HP+1 Str+1 Skl+0 Spd+1 …`.

**A. FE random growths** — each stat has a growth chance (e.g. HP 80%, Str 45%);
every level each stat rolls. Feel: exciting "blessed" level ups, occasional
painful "empty" ones; units feel unique run to run.

**B. FE fixed growths (Fates fixed mode)** — growth % accumulates and the stat
rises each time it passes 100%. Feel: predictable, same every playthrough.

**C. Job-driven growth (FFT)** — the class you're in when you level decides your
gains, so what you level in shapes the unit. Feel: planning and min-maxing.

**D. Random with a safety net** — like A, but a level with fewer than 2 gains is
rerolled (or guarantees +1 in the highest-growth stat). Feel: A's excitement
without the rage-inducing empty level.

**Recommendation:** D.

### Q2. How do class changes work?

**A. Branching promotion (FE Sacred Stones)** — at level 10+ use a promotion
item (e.g. Master Seal) and pick one of two advanced classes
(Cavalier → Paladin *or* Great Knight). Stat boosts on promotion, level resets
to 1. Feel: meaningful fork per unit; simple.

**B. Open certification (Three Houses)** — any unit can train into any class by
raising weapon/skill ranks and passing an exam. Feel: freedom, less identity.

**C. Job system (FFT)** — earn job points; unlock jobs from prerequisites
(Squire → Knight → …); switch jobs between battles and keep learned abilities.
Feel: deepest customisation, lots of menus.

**D. Reclass items (FE Awakening/Fates)** — promotion as A, plus a rarer item to
switch to a different class line entirely. Feel: A with late-game flexibility.

**Recommendation:** A for the core, D's reclass as a later extra.

### Q3. Skills?

**A. Class skills (FE)** — each class has 1–2 passive skills (e.g. Canto for
mounts, Vantage). **B. Learnable skill loadouts (FE Awakening, FFT)** —
units learn skills and equip a few. **C. No skills for now.**
**Recommendation:** A, small.

### Q4. Class tree

Based on the answers above and 0003/0004, **propose** a starter class tree
(about 6–8 base classes, each with 1–2 promotions) as an ASCII tree, e.g.:

```
Lord ─────────► Great Lord
Myrmidon ─┬──► Swordmaster
          └──► Assassin
Cavalier ─┬──► Paladin
          └──► Great Knight
…
```

Ask Nick to approve, rename or cut.

## What to record

`docs/design/progression.md`: Nick's words; EXP formula (propose FE's: base
EXP for hit/kill scaled by level difference, 100 EXP = 1 level, *tunable*); level
cap per tier; growth mechanics exactly (roll procedure incl. safety net);
per-class growth modifiers (if any); promotion rules (level requirement, item,
stat bonuses); skills (if any) with exact effects; the approved class tree with,
for each class: move, movement type (foot / mounted / armoured / flying),
usable weapons, base stats, stat caps, growth rates or modifiers.

Update `docs/design/README.md`; adjust 0302, 0601–0603, 0803 if needed.

## Acceptance criteria

- [ ] Nick answered Q1–Q3 and approved a class tree (Q4).
- [ ] `docs/design/progression.md` written with exact formulas and the class table.
- [ ] Design README updated; downstream tickets adjusted.
- [ ] Ticket archived.

## Completion notes

