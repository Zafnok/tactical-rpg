---
id: "0315"
title: Remove non-player units' own consumables
type: feature
milestone: M2 Core rules
model: sonnet-5
effort: medium
status: done
blocked_by: ["0501"]
nick_input: none
completed: 2026-09-28
---

# 0315 — Remove non-player units' own consumables

## Context

Ticket 0306 let units of other factions carry their own consumables
(`Unit::consumables`) and use them with `UnitAction::UseItem`, because
`docs/design/weapons-and-items.md` said "an enemy may carry its own consumable
in its map data". While 0501 (enemy AI) was being reviewed, Nick decided the
opposite: "I think I would stick to no potions and only dedicated healers for
the enemies in a battle." `weapons-and-items.md` → *Battle pack* now says
**enemies carry no consumables**; enemy healing comes only from healer units.
Player units already use only the shared battle pack, so the only consumables
left in a battle are the pack's. The AI (`crates/core/src/ai.rs`) already never
uses consumables.

So the per-unit consumables are dead weight: a field nothing should set and a
rule path nothing should take. This ticket removes them.

## Nick input

None.

## Scope

**In:**
- Remove `Unit::consumables` and every use of it.
- `UnitAction::UseItem` only for player units, from the battle pack; any other
  faction is refused with the existing `CommandError::PlayerOnly(unit)`.
- Update the docs that describe the old rule.

**Out (do not do):**
- Don't change the battle pack, the `Item` action for player units, shops or
  chests.
- Don't touch the AI's behaviour (it already ignores consumables).
- No save migration code: `Unit` has no `deny_unknown_fields`, so an old save
  with a `consumables` field still loads (check this with the test below).

## Implementation steps

1. `crates/core/src/unit.rs`: delete the `consumables` field and its doc, and
   the `consumables: Vec::new()` line in `Unit::fresh` and in the test
   literals in that file.
2. `crates/core/src/battle.rs`:
   - `plan_item`: if `unit.faction != Faction::Player`, return
     `Err(CommandError::PlayerOnly(unit.id))` first; then read only
     `self.pack.items`.
   - `use_item`: always remove from `self.pack.items`.
   - Module docs, *Items* bullet: "a player unit uses a consumable from the
     shared [`BattlePack`]"; drop the "other factions use their own" part.
     `UnitAction::UseItem::pack_index` doc: "Index in the battle pack."
     `CommandError::PlayerOnly` doc: "Only player units can shop, open chests
     or use items."
3. `crates/core/src/item.rs` module docs (line ~40): drop "Other factions
   carry their own consumables on the unit."
4. Fix every struct literal and test that sets `consumables` (`grep -rn
   "consumables" crates --include=*.rs`, ignoring `BattlePack`/pack and the
   item-table `consumables` in `content`): `crates/core/src/battle/tests.rs`
   (`unit()` fixture, the proptest unit strategy around line 3027 and 3072,
   the check around line 2625), `crates/core/src/movement/tests.rs`,
   `crates/core/tests/replay.rs`, `crates/core/src/ai/tests.rs` (the `unit()`
   fixture, and delete the test `wounded_units_never_drink_potions`, which
   only exists to show the AI ignores them).
5. Replace the test `enemies_use_their_own_consumables` in
   `crates/core/src/battle/tests.rs` with
   `only_player_units_use_items`: an enemy's `UseItem` (in the Enemy phase)
   is refused with `CommandError::PlayerOnly`, and the state is unchanged.

## Acceptance criteria

- [x] `grep -rn "\.consumables\|consumables:" crates/core` finds only the item
      table / battle pack, never a unit field.
- [x] Test `only_player_units_use_items` passes.
- [x] Test `old_saves_with_unit_consumables_still_load` (in
      `crates/core/src/battle/tests.rs`, next to the other serde tests): a
      `BattleState` saved to RON with a `consumables: ["potion"]` entry added
      to a unit deserialises, and equals the state without it.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the two tests above.
- Property: the existing battle proptests still pass without per-unit
  consumables.
- Snapshot / integration: none (no screen shows per-unit consumables).

## Completion notes

Removed `Unit::consumables` and all uses; `plan_item` refuses non-player units
with `PlayerOnly` and reads only the battle pack; docs updated. Added
`only_player_units_use_items` and `old_saves_with_unit_consumables_still_load`;
deleted `enemies_use_their_own_consumables` and `wounded_units_never_drink_potions`.

Deviations: `crates/core/tests/replay.rs` scripted an enemy `UseItem`; it is now
an enemy `Wait`. The proptest action generator offers no `UseItem` to non-player
units. No gameplay rules were decided. No follow-up tickets. Nothing changes
when playing.
