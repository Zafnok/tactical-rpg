---
id: "0003"
title: "Decide: weapons, items and the weapon triangle"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 0003 — Decide: weapons, items and the weapon triangle

## Context

Needed by combat (0304), items/inventory (0306), the forecast UI (0404) and the
Chapter 1 roster (0803). Run with the `ask-nick` skill. Best asked after or
together with 0001.

## Nick input

**Decision.**

## Questions to ask

### Q1. How do weapons work?

**A. Fire Emblem GBA** — Swords > Axes > Lances > Swords triangle (+ bows,
tomes). Weapons have might, hit, crit, weight, range and **durability** (break
after N uses). Units have weapon ranks (E→S) that gate stronger weapons.
Feel: classic; managing durability and buying replacements is part of the game.

**B. FE without durability (Engage / Three Houses-lite)** — same triangle and
ranks, but weapons never break. Feel: FE tactics, far less inventory chores.

**C. Final Fantasy Tactics equipment** — slots for weapon, shield, helm, armour,
accessory. No triangle, no durability; your job decides what you can equip.
Feel: build customisation; less rock-paper-scissors on the map.

**D. Unit-type matchups (Advance Wars / Wargroove)** — no item weapons at all; a
damage chart says how well each class hits each other class.
Feel: clean and readable; no inventory.

**Recommendation:** B — keeps FE's triangle decisions on the map without menu
chores that are clunkier in an ASCII UI.

### Q2. Inventory & consumables

**A. FE: 5 item slots per unit**, healing items (Vulnerary), trade between adjacent units, convoy.
**B. Shared party bag** (FFT/Triangle Strategy): items used from a shared pool.
**C. Minimal**: each unit has a weapon and one consumable slot.
**Recommendation:** A with a small convoy later.

### Q3. Money & shops

**A. FE**: gold from battles/chests/villages; armouries and vendors on maps or between chapters.
**B. Between-chapter shop only.**
**C. No money** — items come from story rewards and loot.
**Recommendation:** B (simplest to build; shops on maps later if wanted). Not needed for Chapter 1 either way.

## What to record

`docs/design/weapons-and-items.md`: Nick's words; weapon types and triangle
effects (exact bonus, e.g. +1 damage +15 hit, *tunable*); weapon stats fields;
ranks (if any) and how they grow; durability (if any); the starter weapon list
with numbers (propose FE-like values: Iron Sword Mt 5 Hit 90 Crit 0 Rng 1…);
inventory size; consumables list for Chapter 1 (at least a healing item); trade
rules; money/shops (or "deferred").

Update `docs/design/README.md`; adjust 0304, 0306, 0404 if needed.

## Acceptance criteria

- [ ] Nick answered Q1–Q3.
- [ ] `docs/design/weapons-and-items.md` written with exact numbers.
- [ ] Downstream tickets adjusted; design README updated.
- [ ] Ticket archived.

## Completion notes

