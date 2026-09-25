---
id: "0306"
title: Items, loadouts, battle pack, equipping, weapon ranks and durability
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0003", "0004", "0305"]
nick_input: answer-first
completed:
---

# 0306 — Items, loadouts, battle pack, ranks, durability

## Context

Weapons, gear, consumables and healing as Nick decided in
`docs/design/weapons-and-items.md` (0003) and `docs/design/magic.md` (0004).
Key rules from 0003: **no weapon triangle** (per-type traits instead);
each unit has a **loadout** (3 weapons, 1 armour, 1 accessory); consumables
live in a **shared battle pack** with a per-battle cap; durability only drops
from Combat Arts (0014, not implemented here); **no trading in battle**.
Extends the command/event system from 0305.

## Nick input

**Answer first:** 0003, 0004.

## Scope

**In:** `assets/data/items.ron`, `core::item`, `Loadout`, `BattlePack`,
party `Stock` type, equip rules, weapon ranks + weapon EXP, durability state and
the broken flag, `UnitAction::{UseItem, Equip}`, real weapon ranges feeding 0303's attack tiles, and building
0304's `CombatantInput` from a unit (gear-adjusted stats, equipped weapon,
armour weight, rank).

**Out:** spells, spell uses, healing magic and equipping a spell (0309;
`magic.md`: spells are innate, not items, so there are no tomes or staves in
`items.ron`), Combat Arts (0014 → its own ticket), gold/shops/villages/chests (0308),
Preparations screen (0408), item menus UI (0407), trading (not in the design).

## Implementation steps

1. `items.ron`: every weapon, armour, accessory and consumable listed in
   `weapons-and-items.md` with exact numbers;
   weapon kinds with their traits as data (`trait: SwordFollowUp | Effective(tag, mult) | AxeMinDamage(5) | GauntletAvoid(15) | None`),
   `rank_speed` table, rank EXP thresholds, broken penalties.
2. `core::item`: `ItemId`, `ItemDef` (enum `Weapon(WeaponDef)`, `Armour { weight_class, bonus: Stats, weight }`,
   `Accessory { bonus: Stats }`, `Consumable { effect }` …),
   `WeaponInstance { def: ItemId, durability_left }` with `is_broken()`.
3. `Loadout { weapons: [Option<WeaponInstance>; 3], equipped: Option<usize>, armour: Option<ItemId>, accessory: Option<ItemId> }`
   on `Unit`. Only the first `class.weapon_slots` entries (0302; 0 for
   tier-3+ magic classes per `magic.md`) may hold weapons; validate this. `Unit::effective_stats()` = permanent stats + armour + accessory,
   clamped to the hard ceilings. `Unit::attack_ranges()` = ranges of usable
   weapons in the loadout — replace 0303's parameters with this in callers.
4. Weapon usability: class weapon kinds (0005) and `rank ≥ weapon.rank`.
   `WeaponRanks` per unit: EXP per kind, `rank(kind)`; weapon EXP after combat
   per the design (+2 if any strike hit, +1 if it struck but all missed) with
   a `WeaponExpGained` / `WeaponRankUp` event.
5. `BattlePack { items: Vec<ItemId>, cap }` on `BattleState` (shared by the
   player side). Enemies' own consumables live on the enemy unit.
6. `UnitAction::UseItem { pack_index, target }`: target = self or an adjacent
   ally; consumes the item; **ends the action**; events `ItemUsed`,
   `Healed { target, amount }` (never above max HP).
   `Command::Equip { unit, slot }` (free; does not end the action), event
   `Equipped`. Attacking with a weapon equips it.
7. `ItemBroke` event + durability decrement helper `spend_durability(amount)`
   (used by the Combat Arts ticket; tested here directly).
8. `characters.ron` placeholder units get loadouts; test chapter/battle setup
   gets a pack. Update tests.

## Acceptance criteria

- [ ] Item numbers match the design docs (test compares a few entries).
- [ ] Each new action/command: valid case, each invalid case (state unchanged), event emitted.
- [ ] Combat input uses the equipped weapon, gear-adjusted stats, armour weight and rank; unarmed units can't attack or counter.
- [ ] Worked examples W1–W4 of `weapons-and-items.md` reproduce when built from real units + `items.ron` (not just hand-built `CombatantInput`).
- [ ] Healing never exceeds max HP; a broken weapon still attacks with the penalties.

## Tests required

- Unit: per action; equip rules; rank thresholds and weapon EXP; durability to 0 → `ItemBroke`; pack use.
- Property: loadout never holds > `weapon_slots` (≤ 3) weapons; pack size only shrinks during battle except via item gains; HP stays within 0..=max after any heal/consumable; effective stats ≤ hard ceilings.
- Extend 0305's random-command property test to include `UseItem` and `Equip`.

## Completion notes

