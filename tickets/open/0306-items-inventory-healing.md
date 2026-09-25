---
id: "0306"
title: Items, inventory, equipping, healing and trade
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0003", "0004", "0305"]
nick_input: answer-first
completed:
---

# 0306 — Items, inventory, healing, trade

## Context

Weapons, consumables and healing as Nick decided in
`docs/design/weapons-and-items.md` (0003) and `docs/design/magic.md` (0004).
Extends the command/event system from 0305.

## Nick input

**Answer first:** 0003, 0004.

## Scope

**In:** `assets/data/items.ron`, `core::item`, unit inventories, equip rules,
`UnitAction::{UseItem, Heal, Trade, Equip}` (only those the design includes),
real weapon ranges feeding 0303's attack tiles and 0304's combat input.

**Out:** shops/money (later ticket if designed), convoy, UI (0403/0404).

## Implementation steps

1. `items.ron`: all Chapter 1 items from the design docs (weapons, a healing
   consumable, healing staff/spell if any) with exact numbers.
2. `core::item`: `ItemId`, `ItemDef` (enum `Weapon(WeaponStats…)`, `Consumable { effect, uses }`,
   `Staff { heal, range, uses }` …), `ItemInstance { def: ItemId, uses_left: Option<u8> }`.
3. `Unit.inventory: Vec<ItemInstance>` (max size from design). Equipped weapon
   per design (FE default: first usable weapon in inventory). `Unit::attack_ranges()`
   = ranges of usable weapons — replace 0303's parameters with this in callers.
4. Weapon usability: class weapon types (+ ranks if designed).
5. New `UnitAction`s with validation + events: `ItemUsed`, `Healed { target, amount }`,
   `Traded { a, b }`, `ItemBroke` (only if durability). Durability decrements
   per the design.
6. `characters.ron` placeholder units get starting inventories; update tests.

## Acceptance criteria

- [ ] Item numbers match the design docs.
- [ ] Each new action: valid case, each invalid case (state unchanged), event emitted.
- [ ] Combat uses the equipped weapon; unarmed units can't attack or counter.
- [ ] Healing never exceeds max HP.

## Tests required

- Unit: per action; equip rules; durability (if any).
- Property: inventory size never exceeds max; HP stays within 0..=max after any heal/consumable.
- Extend 0305's random-command property test to include new actions.

## Completion notes

