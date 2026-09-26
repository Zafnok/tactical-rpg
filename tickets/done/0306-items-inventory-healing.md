---
id: "0306"
title: Items, loadouts, battle pack, equipping, weapon ranks and durability
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: done
blocked_by: ["0003", "0004", "0305"]
nick_input: answer-first
completed: 2026-09-26
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
   **From 0305:** `Unit.weapon: Option<WeaponStats>` is a stand-in for the
   equipped weapon, and `BattleState::combatant` (in `core::battle`) builds
   `CombatantInput` from it with permanent stats and no armour. Replace both
   with the loadout.
4. Weapon usability: class weapon kinds (0005) and `rank ≥ weapon.rank`.
   `WeaponRanks` per unit: EXP per kind, `rank(kind)`; weapon EXP after combat
   per the design (`weapons-and-items.md`: base 2 if any strike hit, 1 if it
   struck but all missed, plus `dealt / 5` for the HP its strikes actually
   removed; take a `used_art: bool` that doubles the base for 0312) with
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

- [x] Item numbers match the design docs (test compares a few entries).
- [x] Each new action/command: valid case, each invalid case (state unchanged), event emitted.
- [x] Combat input uses the equipped weapon, gear-adjusted stats, armour weight and rank; unarmed units can't attack or counter.
- [x] Worked examples W1–W4 of `weapons-and-items.md` reproduce when built from real units + `items.ron` (not just hand-built `CombatantInput`).
- [x] Healing never exceeds max HP; a broken weapon still attacks with the penalties.

## Tests required

- Unit: per action; equip rules; rank thresholds and weapon EXP; durability to 0 → `ItemBroke`; pack use.
- Property: loadout never holds > `weapon_slots` (≤ 3) weapons; pack size only shrinks during battle except via item gains; HP stays within 0..=max after any heal/consumable; effective stats ≤ hard ceilings.
- Extend 0305's random-command property test to include `UseItem` and `Equip`.

## Completion notes

**Done.**

- `assets/data/items.ron`: every weapon, armour, accessory and consumable in
  `weapons-and-items.md` with its exact numbers, each kind's trait as data,
  `rank_speed`, rank EXP thresholds, broken penalties, the weapon EXP numbers
  and the default pack cap (6). Loaded and validated by `trpg_content::item`
  (unique ids across lists, ranges, durability, one trait per kind, increasing
  thresholds…); tests compare every entry with the design tables, and the
  kind traits / rules with `CombatRules::default()`.
- `core::item`: `ItemId`, `ItemDef` (`Weapon`/`Armour`/`Accessory`/
  `Consumable`), `ItemTable`, `WeaponRules`, `WeaponInstance`
  (`is_broken`, `spend_durability`), `Loadout`, `LoadoutDef`, `BattlePack`,
  `Stock`, and on `Unit`: `rank`, `can_wield`, `usable_weapon`,
  `effective_stats` (gear up to the hard ceilings), `armour_weight`,
  `attack_ranges`, `combat_input`, `validate_loadout`, `with_loadout`,
  `gain_weapon_exp`, `spend_durability` (→ `Event::ItemBroke`).
- Battle: `BattleSetup` takes `items` and a `pack`; `UnitAction::Attack` now
  names a loadout `slot` (attacking equips it, `Event::Equipped`);
  `UnitAction::UseItem { pack_index, target }` (player units use the pack,
  other factions their own `Unit::consumables`), `Command::Equip` (free).
  New events `Equipped`, `WeaponExpGained`, `WeaponRankUp`, `ItemUsed`,
  `Healed`, `ItemBroke`. Combat uses gear-adjusted stats, the equipped (or
  chosen) weapon, armour weight and rank; only a wieldable equipped weapon
  counters.
- `characters.ron` placeholder units have loadouts (validated at load); the
  debug Quick Battle has a pack of 3 Potions.

**Deviations / technical choices.**

- The `Unit.weapon` stand-in from 0305 is gone; `BattleState::combatant`
  builds `CombatantInput` via `Unit::combat_input`.
- "`WeaponRanks` per unit" is two maps on `Unit`: the existing
  `weapon_ranks` plus a new `weapon_exp`. EXP counts from the current rank's
  threshold (a unit given rank D by its class starts at 30) and stops at the
  class's max rank's threshold.
- The item table is shared content like the terrain and class tables: not
  saved, reattached with `restore_tables(terrain, classes, items)`
  (ADR-0020, whose point 5 anticipated this). The battle's `CombatRules` now
  come from `ItemTable::combat_rules()`.
- `Unit::attack_ranges` is what `threat_area`/`danger_zone` callers pass;
  there are no non-test callers yet (the UI overlays are 0403/0405).
- A unit with no rank recorded in a kind counts as rank E. A unit may carry a
  weapon it can't wield but never equip it.
- Weapon EXP is given to every faction that struck (enemies too); a unit that
  fell gets none.
- Using an item on a unit at full HP is allowed (heals 0); the design doesn't
  forbid it and the item menu (0407) can grey it out.

**Nick:** nothing to play yet: the item and equip menus are 0407. The Quick
Battle units now carry the design's starter weapons and armour.

