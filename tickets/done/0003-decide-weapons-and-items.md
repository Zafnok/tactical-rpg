---
id: "0003"
title: "Decide: weapons, items and the weapon triangle"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-25
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

**Follow-up (from 0001):** units can strike up to 4 times based on attack
speed (`docs/design/stats-and-combat.md`), and Nick expects "investment in
the weapon skill" and "best in slot gear" to help reach 3x/4x. Ask how
weapons/gear feed `as_bonus`: e.g. weapon rank grants +Spd for attack speed
(Three Houses-style skill levels), gear with +Spd, and/or weapon weight
lowering attack speed (FE). Record the exact rule.

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
ranks (if any) and how they grow; how weapons/gear/ranks change attack speed
(`as_bonus` in `stats-and-combat.md`); durability (if any); the starter weapon list
with numbers (propose FE-like values: Iron Sword Mt 5 Hit 90 Crit 0 Rng 1…);
inventory size; consumables list for Chapter 1 (at least a healing item); trade
rules; money/shops (or "deferred").

Update `docs/design/README.md`; adjust 0304, 0306, 0404 if needed.

## Acceptance criteria

- [x] Nick answered Q1–Q3.
- [x] `docs/design/weapons-and-items.md` written with exact numbers.
- [x] Downstream tickets adjusted; design README updated.
- [x] Ticket archived.

## Completion notes

Nick answered all three questions plus three follow-ups. Recorded in
`docs/design/weapons-and-items.md`.

- **Weapons:** Nick went with neither A nor B. He asked for *Fire Emblem:
  Fortune's Weave*'s model, which Claude researched: **no weapon triangle**,
  a trait per weapon type (sword ×1.2 on follow-ups, spear ×2 vs Mounted,
  bow ×3 vs Flying at range 2, axe minimum 5 damage, gauntlet +15 avoid),
  durability spent **only by Combat Arts**, broken weapons still usable with
  penalties, repair at a blacksmith. Weapon ranks E–S grow with weapon EXP.
- **Attack speed:** Nick asked for weight, Str, weapon skill and Spd, with
  gauntlets reaching multiple strikes more easily than axes. Claude's formula:
  `as_bonus = rank_speed − max(0, weapon wt + armour wt − Str/5)`, with gear
  adding to Spd. `stats-and-combat.md`'s examples are unchanged (weight 0, rank E).
- **Inventory:** Nick's own design. Each unit has a loadout of 3 weapons,
  1 armour and 1 accessory. Consumables go in one shared battle pack, capped
  **per battle** (Nick: "scale them to the battle").
- **Money:** FE on-map shops (Armoury / Vendor / Blacksmith), plus the same
  shops between chapters. Villages and chests.
- Claude's starting rules that Nick may veto: using an item ends the unit's
  action (self or an adjacent ally); no trading during battle; items found
  mid-battle go into the pack past the cap; weapon EXP +2/+1 per combat;
  an art needs `durability_left ≥ cost`; chests need no key yet; default
  pack cap 6.

Downstream tickets adjusted: 0004, 0005, 0009, 0302, 0304, 0305, 0306
(rewritten: loadouts, pack, ranks, durability, no trade), 0403, 0404
(forecast: effectiveness marker and sword follow-up damage, not triangle
arrows), 0405, 0501, 0801 (chapter pack/gold fields, now blocked by 0408),
0803. `stats-and-combat.md` now points to the new `as_bonus` rule.

Follow-up tickets created: **0014** Decide Combat Arts (Nick: its own ticket,
in Chapter 1), **0308** gold/shops/villages/chests (core), **0407** Item and
Equip menus, **0408** Preparations screen, **0409** shop screen and
Visit/Open.
