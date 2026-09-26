---
id: "0313"
title: Villages and the Visit action (core rules)
type: feature
milestone: M2 Core rules
model: sonnet-5
effort: medium
status: blocked
blocked_by: ["0308"]
nick_input: none
completed:
---

# 0313 — Villages and the Visit action

## Context

0308 added gold, on-map shops and chests but left **villages** out: Nick
deferred every building tile except `fort` (`docs/design/terrain.md`,
2026-09-26, Q7: "No village, gate or throne tiles yet"). The village rule
itself is already written (`docs/design/weapons-and-items.md`, "Money and
shops": a player unit on the village gate uses `Visit` for a one-time gift of
gold, an item or a scene; then the village closes).

**Blocked** until `docs/design/terrain.md` defines a village tile (a later
terrain decision by Nick). Don't invent the tile: if it isn't in `terrain.md`,
stop and say so. The UI half is ticket 0409 (it builds the village UI only
once this ticket lands).

## Nick input

None (the tile itself comes from a terrain design decision, not this ticket).

## Scope

**In:**
- `TileFeature::Village(Gift)` in `crates/core/src/map.rs`, with
  `Gift = Gold(n) | Item(id) | Scene(id)` (reuse `shop::Loot` for gold/item).
- `UnitAction::Visit` in `crates/core/src/battle.rs`, `Event::VillageVisited`.
- The `village: (gift: …)` feature in the `.map` header
  (`crates/content/src/map.rs`, `assets/maps/README.md`), validated like
  chests (ids known, tile passable, `Gold(0)` refused); scene ids validated
  once the dialogue format exists (0702) — until then any non-empty id.

**Out (do not do):**
- Enemies destroying villages, village UI (0409), playing the scene (0705).

## Implementation steps

1. Map: add `Village(Gift)` to `TileFeature`, a `BattleMap::village(pos)`
   lookup, and the header syntax `Village(Gold(n) | Item("id") | Scene("id"))`
   with the same checks as chests in `check_features`.
2. Battle: `UnitAction::Visit` for a **player** unit on an unvisited village
   (reuse `CommandError::PlayerOnly`; add `NoVillage(Pos)` and
   `AlreadyVisited(Pos)`). Track visited villages in `BattleState` (saved, like
   `opened`). Gold → `GoldChanged`; items go where chest items go (consumable →
   pack, equipment → stock); a scene only appears in the event.
3. Event `VillageVisited { unit, pos, gift }`, before `GoldChanged` and
   `UnitActed`. Document the rule in the `battle.rs` module docs.
4. Extend `legal_commands` in `crates/core/src/battle/tests.rs` with `Visit`
   and add a village to `prop_map()`.

## Acceptance criteria

- [ ] `Visit` valid case per gift kind, and each error case with the state
      unchanged (`refused_act`).
- [ ] A visited village can't be visited twice; visited state survives a RON
      save/load.
- [ ] Map files with villages parse, print back and are validated.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: per gift kind and per error; map parsing/printing/validation.
- Property: the random-command test in `battle/tests.rs` includes `Visit`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
