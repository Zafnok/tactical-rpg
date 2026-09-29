---
id: "0407"
title: Item (battle pack) and Equip menus in battle
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0306", "0403"]
nick_input: none
completed: 2026-09-28
---

# 0407 — Item and Equip menus

## Context

0306 adds `UnitAction::UseItem` (shared battle pack, target self or an
adjacent ally, ends the action) and `Command::Equip` (free) per
`docs/design/weapons-and-items.md`. This ticket puts them in the action menu
from 0403. The UI never changes state itself: it sends `Command`s and reacts
to `Event`s ([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

None.

## Scope

**In:** `Item` and `Equip` entries in the action menu, the pack list, target
selection for items, a heal number popup, help-bar text.

**Out:** Preparations (0408), shops/villages/chests (0409), Combat Arts
(0014's follow-up tickets). There is no `Trade` (not in the design).

## Implementation steps

1. Action menu: `Item` enabled when the pack has an item usable on the unit or
   an adjacent ally; `Equip` enabled when the unit has ≥ 2 usable weapons.
2. **Item:** a Menu listing the pack grouped by id (`Potion ×3`, one-line
   effect), header `Pack 4/6`. Choosing one → target mode (self + adjacent
   allies, cursor keys/`NextUnit`/`PrevUnit` cycle, preview line `HP 12 → 22`) → Confirm sends
   `Act { UseItem }`. Cancel steps back one level.
3. **Equip:** a Menu of the 3 weapon slots with Mt/Hit/Crit/Wt/Rng and
   durability `12/20` (`(broken)` in the warning colour at 0), a marker on the
   equipped one, unusable weapons dimmed. Confirm sends `Equip` and returns to
   the action menu (the unit has not acted).
4. On `Healed`: a `+10` popup over the target for 0.6 s (timing in the
   screen's const struct, *tunable*).

## Acceptance criteria

- [x] Harness: select → move → Item → Potion → adjacent ally → ally HP rises by the design amount, unit is done, pack count drops.
- [x] Harness: Equip swaps the equipped weapon and the unit can still act.
- [x] Snapshots below.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Harness: the two flows above; cancelling at each level leaves state unchanged.
- Snapshot: pack menu, item target mode, equip menu with a broken weapon.

## Completion notes
- **Action menu** is now `Attack`, `Seize` (when legal), `Item`, `Equip`,
  `Wait`. It opens on the first enabled of Attack/Seize, else `Wait`, so a
  quick `f f f` still waits.
- **Item** (`items.rs`, `Mode::ItemMenu`/`ItemTarget`): the pack grouped by id
  (`Potion ×3  Restore 10 HP`, box title `Pack 4/6`), then target mode
  (self + adjacent allies, cursor keys/`NextUnit`/`PrevUnit` cycle, starts on
  the user) with the `Potion on Rex: HP 12 → 22` preview on the message row.
  Confirm sends `Act { UseItem }`; Cancel steps back one level each.
- **Equip** (`Mode::EquipMenu`): filled weapon slots with Mt/Hit/Crit/Wt/Rng
  and `12/20` durability, `*` marks the equipped one, unusable weapons are
  dimmed and can't be chosen, `(broken)` in the `hp_low` colour (the palette has
  no separate warning colour). Confirm sends `Command::Equip` and reopens the
  action menu (`Effect::ApplyStay`); the unit hasn't acted.
- **Heal popup:** `+N` over the healed unit for `Timings::heal_popup` = 0.6 s
  (*tunable*), from any `Event::Healed` with amount > 0.
- `MenuItem` gained an optional coloured suffix (used for `(broken)`).

Deviations / UI choices (no game rules decided; core still allows any
legal `UseItem`):

- *Claude's starting rule (UI only):* `Item` is enabled, and a target listed,
  only for units that are **hurt**; the player is never offered a use that
  heals nothing. Core would still accept it.
- The `Equip` entry is disabled with fewer than two usable weapons; empty
  weapon slots aren't listed.
- Tests use the screen directly (`item_tests.rs`) rather than the title-screen
  harness, because the Quick Battle has nobody hurt.
