---
id: "0009"
title: "Decide: Chapter 1 scope (objective, roster, map, tutorial)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: ["0007"]
nick_input: decision
completed: 2026-09-25
---

# 0009 — Decide: Chapter 1 scope

## Context

Nick hopes Chapter 1 is playable within a few days of starting tickets. This
decision pins down exactly what "Chapter 1" contains so the content ticket
(0803) and the story script (0707) have a fixed target. Blocked by 0007 so the
proposal can fit the setting. Run with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

### Q1. Objective

**A. Rout** — defeat all enemies (FE's most common; easy to learn).
**B. Defeat the boss / seize** — kill the boss or stand the lord on the throne.
Feel: rush vs. clear dilemma.
**C. Defend / survive N turns** — hold a position against waves.
**D. Escape** — get the lord to an exit tile.
**Recommendation:** B (defeat boss) — teaches focus, short map, room for a boss
conversation.

### Q2. Size

Propose, and let Nick adjust:
- **Player units:** 4 (e.g. lord, a mounted unit, an archer, a healer — final
  classes from 0005).
- **Enemies:** 8–10 + 1 boss.
- **Map:** about 20×14 tiles (fits nearly on one screen at our tile size).
- **Length:** 10–20 minutes for a first-time player.

### Q3. Teaching

**A. Guided tutorial map (FE Blazing Blade's Lyn prologue)** — forced steps
("move here, now attack").
**B. Contextual hints** — a help bar and one-time tips when something new
appears (first enemy in range, first level up). Feel: no hand-holding.
**C. None.**
**Recommendation:** B.

### Q4. Extras in Chapter 1

Pick any: a talk-to-recruit enemy (FE staple), a village to visit for an item,
a mid-battle reinforcement wave, a treasure chest, an on-map shop (armoury,
vendor or blacksmith; Nick chose FE on-map shops in 0003), a terrain-magic
moment (a forest to burn or water to freeze, 0004 `magic.md`), an elemental
enemy with a battle note (0004: uncommon), none.
**Recommendation:** one talk-recruit — it shows off two-portrait dialogue mid-battle.

### Q5. Preparations and the battle pack

From 0003 (`docs/design/weapons-and-items.md`): each battle has its own cap
on shared consumables ("scale them to the battle"), picked on a Preparations
screen. Ask: does Chapter 1 open with a Preparations screen (loadouts +
pack) or start straight in with a fixed loadout and pack (FE's first chapters
skip Preparations)? What pack cap and contents (e.g. 4 Potions)? Starting
gold, and gold for clearing the map?

## What to record

`docs/design/chapter-1.md`: Nick's words; objective and loss conditions; roster
(slots with class, not names yet unless the story has them); enemy count and
mix; map size and rough layout idea (an ASCII sketch at tile resolution is
welcome); teaching approach; extras; Preparations yes/no, pack cap and default
pack, starting and reward gold. Adjust tickets 0707, 0801, 0803, 0407–0409 if needed.

## Acceptance criteria

- [x] Nick answered Q1–Q5.
- [x] `docs/design/chapter-1.md` written.
- [x] Design README updated; downstream tickets adjusted; ticket archived.

## Completion notes

Nick answered Q1–Q5 on 2026-09-25; recorded in `docs/design/chapter-1.md`.

- **Q1:** Rout, no turn limit. Nick also said a chapter is a story beat, which
  can hold several battles (e.g. overworld skirmishes before a climax).
  Chapter 1 is a single battle. The multi-battle idea is noted for 0008 / 0801.
- **Q2:** the bigger option: 6 units (lord + Rider, Archer, Cleric, Guard,
  Mage), 12–14 enemies + boss, about 24×16, 20–30 min. Nick added that **the lord
  has a unique starting class/tree**, which is new design work, so I created
  **0016** (decide the lord's class line) and made 0803 wait for it.
- **Q3:** contextual hints (0406 context updated).
- **Q4:** no extras in Chapter 1 (no talk-recruit, village, chest, shop,
  reinforcements, terrain-magic moment, elemental or battle note). 0707 and 0803
  updated to drop the talk-recruit scene and the extras.
- **Q5:** no Preparations; fixed pack of 3 Potions (cap 3); 0 starting gold;
  1000 gold clear reward (tunable).
- Claude's choices (tunable): map difficulty tier Easy (2 rewind charges); the
  enemy mix (mostly Brigand/Raider/Archer); a rough ASCII layout sketch for 0803.
- Pointers in `magic.md` and `weapons-and-items.md` updated. Also updated the
  design README (with a 0016 row) and ROADMAP (Nick's queue).
- No changes were needed in 0408/0409: Preparations and shops are still built
  for Chapter 2 onward and for Quick Battle.
