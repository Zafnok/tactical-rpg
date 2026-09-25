---
id: "0004"
title: "Decide: magic (whether, what kind, healing)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
---

# 0004 — Decide: magic

## Context

Nick explicitly flagged "should there be magic, what type" as a question for
him. It affects combat (0304), classes (0302/0005), the story bible (0701) and
healing (0306). Run with the `ask-nick` skill.

From 0003 (`docs/design/weapons-and-items.md`): Nick rejected a weapon
triangle ("no built-in triangle forcing anything"), preferring per-weapon-type
traits (Fortune's Weave); magic's role there is hitting Res, which armoured
enemies lack. So a magic triangle is unlikely to fit; offer per-element traits
instead. Decide also whether tomes are weapons in the 3-weapon loadout, their
weight (it feeds attack speed via `burden`), and whether spells spend
durability like Combat Arts or have their own uses.

## Nick input

**Decision.**

## Questions to ask

### Q1. Should there be magic, and what kind?

**A. Fire Emblem tomes** — magic is a weapon type (tomes) used by mage classes,
hitting **Res** instead of Def, usually at range 1–2. Optional magic triangle
(Anima > Light > Dark > Anima in FE7). Feel: mages are glass cannons that melt
armoured knights; simple to read.

**B. Final Fantasy Tactics spells** — MP pool, spells with **charge time**
(cast now, resolves later) and **area of effect** that can hit allies too;
elements (fire/ice/lightning/holy). Feel: big dramatic plays, positioning around
AoE, planning ahead of the charge.

**C. Elemental terrain magic — Tactics Ogre / Divinity-style** — spells change
the map: fire burns forest tiles, ice freezes water into walkable tiles,
lightning arcs through water. Feel: environmental puzzles; shows off ASCII
nicely (tiles visibly change glyph and colour).

**D. Low / no magic** — gritty war story (Battle Brothers, early Tactics Ogre
flavour): archers, siege engines, alchemy potions, maybe rare "miracles" tied to
the story. Feel: grounded, every soldier is mortal.

**Recommendation:** A as the base for Chapter 1 (cheap to build, FE feel), with
C's terrain interactions as a later signature feature.

### Q2. Healing

**A. Healer class with staves (FE)** — heals don't cost HP, give healer EXP.
**B. Potions/items only.**
**C. White magic in a spell list (FFT).**
**Recommendation:** A.

### Q3. Flavour: what *is* magic in this world?

Open question for the story (e.g. "a dying god's blood", "forbidden science",
"pacts with spirits"). Nick can answer now or leave it to the story bible
(ticket 0701) — record either way.

## What to record

`docs/design/magic.md`: Nick's words; whether magic exists; how it deals damage
(stat used vs stat defended, formula linked to `stats-and-combat.md`); range;
MP or tome uses (if any); elements / triangle (exact bonuses, *tunable*); healing
mechanics with numbers (e.g. Heal staff: restores Mag + 10, range 1); starter
spell list for Chapter 1; the flavour answer (or "deferred to 0701").

Update `docs/design/README.md`; adjust 0304, 0306, 0701 if needed.

## Acceptance criteria

- [x] Nick answered Q1–Q2 (Q3 answered or explicitly deferred).
- [x] `docs/design/magic.md` written with exact mechanics.
- [x] Design README updated; downstream tickets adjusted.
- [x] Ticket archived.

## Completion notes

Recorded in `docs/design/magic.md`.

- **Magic (Q1):** Nick combined the options. Magic is **innate spells, not
  weapons**: spells never take a loadout slot. Each spell has its own **uses
  per battle** (not MP), refilled every battle. **Magic classes carry 3
  weapons in tiers 1–2, and from tier 3 on fight only with spells**
  (`weapon_slots` 0). **Terrain magic, basic version:** Fire on an empty forest
  makes it burning (impassable) for one round, then burnt (walkable) from the
  caster side's next phase. Ice on water or sea makes walkable ice at once, for
  the whole battle. Lightning was dropped as unclear in ASCII. **No magic
  triangle.** Uncommon **elemental enemies** are Weak (×3 might, like
  effectiveness) to one element and **Absorb** their own. **Battle notes** at
  the start of a battle (Fortune's Weave-style) point out unique enemies.
- **Healing (Q2):** potions stay in the pack. **White-magic healer classes**
  cast innate heal spells (Heal: `10 + Mag`, range 1, 8 uses). They follow the
  same weapon-slot rule, and **healing gives EXP**.
- **Flavour (Q3):** deferred to the story bible (0701).
- **Learning:** spells come from the class list by level (kept after a class
  change), plus 1–2 personal signature spells per named character.
- **Claude's starting rules Nick may veto:**
  - one use per combat, not per strike;
  - counters use the equipped weapon or spell, and there is no counter at 0
    uses;
  - spells weigh 0 and have no rank (so no weapon EXP);
  - no limit on how many learned spells are available;
  - weapons go to the stock when a unit is promoted into a 0-slot class;
  - tile casts only on empty tiles, and casting at a unit never changes
    terrain;
  - burning tiles block flyers too;
  - Resist halves damage, and Absorb forces crit 0;
  - healers can't heal themselves;
  - a tile cast gives the same EXP as a heal.
- Push-spell collision rule recorded for later, as Nick described (5 damage,
  then placed on the nearest free tile). No push spell exists yet.

Downstream tickets adjusted: 0005 (magic lines with ≥3 tiers, spell lists,
EXP for heals and tile casts), 0009 (terrain-magic and elemental extras),
0302 (`weapon_slots`, class/personal spell lists, affinities), 0304
(affinities, examples M1–M2), 0306 (no tomes/staves, weapon-slot limit,
healing moved to 0309), 0501 (spells in AI scoring, blocked by 0309), 0701
(magic flavour deferred here, and signature spells in character sheets), 0803
(blocked by 0410/0411). `weapons-and-items.md` and `stats-and-combat.md` now
point to `magic.md`. ROADMAP updated.

Follow-up tickets created:
- **0309** Spells: data, lists, uses, Cast, healing.
- **0310** Terrain magic (burn and freeze).
- **0410** Spell menu, casting UI and terrain display.
- **0411** Battle notes.
