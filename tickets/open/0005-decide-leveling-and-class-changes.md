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

**A. Class skills (FE)** — each class has 1–2 passive skills (e.g. Vantage).
Note (0002, `docs/design/turn-structure.md`): Nick ruled out built-in Canto;
post-action movement exists only as specific combat skills (e.g. FE's bow
skill that steps 1 tile away after attacking). Use such skills as examples,
not Canto for all mounts. **B. Learnable skill loadouts (FE Awakening, FFT)** —
units learn skills and equip a few. **C. No skills for now.**
**Recommendation:** A, small.

### Q4. Class tree

Based on the answers above and 0003/0004, **propose** a starter class tree.
From 0003 (`docs/design/weapons-and-items.md`): weapon kinds are Sword, Spear,
Axe, Bow, Gauntlet, each with a trait and no triangle, so
the tree should cover them (a gauntlet "punching class" speed specialist was
Nick's own example in 0001); armoured classes should have high Def and low Res
(Nick: "encouraged to use magic users on those enemies"); spears need
`Mounted` targets and bows `Flying` ones to matter
(about 6–8 base classes, each with 1–2 promotions) as an ASCII tree.
From 0004 (`docs/design/magic.md`): magic is **innate spells** (uses per
battle, outside the loadout). Include a **black-magic line** and a
**white-magic (healer) line** with **at least 3 tiers**: tiers 1–2 carry 3
weapons *and* spells; tier 3+ has **0 weapon slots** and fights with spells
only. Propose each class's spell list `(level, spell)` from `magic.md`'s
starter spells (Fire, Frost, Force, Heal, Mend), and the Fire/Frost Elemental
enemy classes (affinities are fixed in `magic.md`). Example:

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
stat bonuses); skills (if any) with exact effects; unit EXP for healing
(Nick: healing gives EXP) and for tile casts (`magic.md`: same as a heal); the approved class tree with,
for each class: move, movement type (foot / mounted / armoured / flying),
usable weapon kinds with starting and maximum weapon rank (E…S), allowed
armour weights (Light/Medium/Heavy), tags (`Mounted`, `Flying`, `Armored`),
`weapon_slots` (3 or 0), spell list `(level, spell)`, affinities (elementals),
base stats, stat caps, growth rates or modifiers.

Update `docs/design/README.md`; adjust 0302, 0309, 0601–0603, 0803 if needed.

## Acceptance criteria

- [ ] Nick answered Q1–Q3 and approved a class tree (Q4).
- [ ] `docs/design/progression.md` written with exact formulas and the class table.
- [ ] Design README updated; downstream tickets adjusted.
- [ ] Ticket archived.

## Completion notes

