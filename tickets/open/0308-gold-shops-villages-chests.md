---
id: "0308"
title: Gold, on-map shops, villages, chests and repair (core rules)
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0306"]
nick_input: none
completed:
---

# 0308 — Gold, shops, villages, chests, repair

## Context

Nick chose FE on-map shops (0003, `docs/design/weapons-and-items.md`,
"Money and shops"): party-wide gold; Armoury / Vendor / Blacksmith tiles on
maps and the same shops between chapters; villages and chests. Builds on the
loadout, stock and battle pack from 0306 and the command/event system
from 0305 ([ADR-0004](../../docs/adr/0004-crate-architecture.md): pure
`core`, changes only via `Command` → `Event`s).

**Villages are on hold:** Nick deferred every building tile except `fort`
(`docs/design/terrain.md`, 2026-09-26). Unless `terrain.md` has a village by
the time this ticket is worked, leave villages (and `Visit`) out and say so in
the Completion notes.

## Nick input

None.

## Scope

**In:** `core::shop` (pure), party `gold` in `BattleState` (seeded from the
campaign), map-format fields for shop/village/chest tiles,
`UnitAction::{Shop, Visit, Open}`, the repair-cost formula, a `ShopSession`
API reused between chapters.

**Out:** the shop screen UI (0409), Preparations (0408), materials/forging
(later, `world-structure.md`), enemies destroying villages, chest keys and thieves.

## Implementation steps

1. Map format (0301's file): optional features per tile —
   `shop: (kind: Armoury | Vendor | Blacksmith, stock: [item ids])`,
   `village: (gift: Gold(n) | Item(id) | Scene(id))`, `chest: (Gold(n) | Item(id))`.
   Validate ids and that features sit on passable tiles.
2. `core::shop`: `buy`, `sell` (price / 2), `repair_cost(weapon)` =
   `ceil(price / 2 × missing / durability)` in integer maths
   (`(price * missing + 2 * durability - 1) / (2 * durability)`), `repair`
   (back to full durability). Errors: not enough gold, item not sold here,
   nothing to repair.
3. `UnitAction::Shop { txns: Vec<ShopTxn> }` for a unit on a shop tile:
   applying ≥ 1 transaction **ends the unit's action**; an empty list is
   rejected (the UI just closes without acting). Bought weapons/gear go to the
   unit's free loadout slot, else the stock; bought consumables go straight
   into the battle pack (even past the cap). Events: `Bought`, `Sold`,
   `Repaired`, `GoldChanged`.
4. `UnitAction::Visit` (player unit on an unvisited village): gift applied,
   village marked visited; event `VillageVisited`. `UnitAction::Open` (player
   unit on an unopened chest): contents go to gold or the pack (consumables) /
   stock (equipment); event `ChestOpened`.
5. `ShopSession` (buy/sell/repair against the campaign's gold and stock,
   outside battle) sharing the same functions, for between chapters.

## Acceptance criteria

- [ ] Each action: valid case, each invalid case (state unchanged), events emitted.
- [ ] Repair cost matches a table of ≥ 4 cases (full, half, 1 missing, 0 missing).
- [ ] Gold never goes negative; a visited village / opened chest can't be used twice.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: per action and per error; repair cost table; sell price.
- Property: gold ≥ 0 and items conserved (bought + held − sold) across random shop sequences.
- Extend 0305's random-command property test with the new actions.

## Completion notes

