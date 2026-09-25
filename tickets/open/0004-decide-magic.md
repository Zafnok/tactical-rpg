---
id: "0004"
title: "Decide: magic (whether, what kind, healing)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
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

- [ ] Nick answered Q1–Q2 (Q3 answered or explicitly deferred).
- [ ] `docs/design/magic.md` written with exact mechanics.
- [ ] Design README updated; downstream tickets adjusted.
- [ ] Ticket archived.

## Completion notes

