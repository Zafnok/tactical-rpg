---
id: "0005"
title: "Decide: level ups, class changes and the class tree"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: ["0001"]
nick_input: decision
completed: 2026-09-25
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

- [x] Nick answered Q1–Q3 and approved a class tree (Q4).
- [x] `docs/design/progression.md` written with exact formulas and the class table.
- [x] Design README updated; downstream tickets adjusted.
- [x] Ticket archived.

## Completion notes

Decided with Nick on 2026-09-25 and recorded in
[`docs/design/progression.md`](../../docs/design/progression.md).

- **Q1:** growth comes from the class, plus one personal **talent** stat at
  +20%. There is a per-tier minimum number of gains per level up ("blessed
  N") that goes up with the tier. The exact steps are left open (starting
  values 2/2/3 for tiers 1–3).
- **Q2:** branching promotion plus reclass. The character level **never
  resets**. There is also a per-class **class level** raised by class
  points; mastering a class opens its promotions. Open certification was
  rejected.
- **Q3 (revised by Nick in the final round):** unlocking a class gives its
  active, and mastering it teaches its passives. Passives are kept for good;
  an active stays behind when you reclass out of a class you haven't
  mastered. A higher rank of a skill family
  replaces the lower one (Nick's White Magic 1 → 2 example).
- **Q4:** Nick approved the revised tree: 9 starter lines, 16 tier-2 classes
  and 16 tier-3 classes. Martial lines have as many tiers as magic lines.
  Names are generic placeholders, with no Fire Emblem-specific names
  (Nick). Nick expects 6–10 tiers eventually, so every rule takes the tier
  as a number.
- **Deviation:**
  - The first tree preview inside the question pop-up didn't show for
    Nick, so the tree was shown as plain text instead.
  - The first tree gave only the magic lines a tier 3. Nick asked for tier 3
    on every line and for generic names.
- **Follow-up rounds:** at first Claude filled several game levers in by
  itself. Nick asked to decide them, so they were put to him in three more
  rounds. His decisions:
  - Promotion is one-way. Every reclass costs a Reclass Seal. Class progress
    is saved, and stats carry over.
  - The seal can reach any class whose requirements the unit meets.
  - Promotion gives a big boost from the base-stat gap (FE GBA style).
  - Mastery pace rises with tier. There's a separate seal per tier.
    Generic enemies have fixed average stats.
  - Actives are used only when attacking, plus stance riders (Fortune's
    Weave's Guarding Strike).
  - Growths above 100% can give +2 (may change with the number scale).
  - Only player units gain EXP. Green units' EXP is pooled and split at the
    end of the battle.
  - Faster levels (about 2 per battle), with the level cap high and still
    to be decided.
  - Skills flipped: active on unlock, passives on mastery.
  - The skill list and stat numbers are placeholders, to be judged in the
    playtest.
- **Claude filled in** (Nick may veto): how the ally-EXP split works (even
  split, remainder lost; deployed, alive and uncapped units only), the
  99-level placeholder cap, and the CP-per-tier values 10/17/25.
- **Downstream tickets adjusted:** 0302 (class record, talent, generic
  units, validation), 0309 (spells by class level), 0601 (class points,
  class levels, safety net), 0602 (class-progress banners), 0603 (no reset,
  mastery, seals, reclass; now also blocked by 0306), 0803 (talent, generic
  units, blocked by 0412). ROADMAP updated.
- **Follow-up tickets created:** 0311 (class skills in core), 0412 (skill UI),
  1001 (tier-3 class skills).
