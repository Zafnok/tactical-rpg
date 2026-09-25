---
id: "0409"
title: Shop screen and Visit/Open actions in battle
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0308", "0403"]
nick_input: sign-off
completed:
---

# 0409 — Shop screen, Visit, Open

## Context

UI for 0308's rules (`docs/design/weapons-and-items.md`, "Money and shops"):
FE on-map Armoury / Vendor / Blacksmith, villages and chests. The same shop
screen is reused between chapters through 0308's `ShopSession`.

## Nick input

**Sign-off:** Nick buys, sells and repairs on a test map and comments.

## Scope

**In:** `ShopScreen` (Buy / Sell / Repair tabs by shop kind), `Shop`, `Visit`
and `Open` entries in the action menu, gold in the side panel and shop
header, gift/loot message lines.

**Out:** where between-chapter shopping sits in the game flow (0801 / 0008).

## Implementation steps

1. Action menu: `Shop` on a shop tile, `Visit` on an unvisited village gate,
   `Open` on an unopened chest (player units only).
2. `ShopScreen`: header `ARMOURY    Gold 1 250`; Buy list with price and a
   stat line; Sell list (half price); a Blacksmith shows Repair with the cost
   and `12/20 → 20/20`. Unaffordable entries dimmed. Transactions collect into
   one `UnitAction::Shop`; leaving with none sends nothing (unit not done).
3. `Visit` / `Open`: send the action; show `Got 300 gold.` / `Got Potion.`
   (plus the scene hook if a village gift is a scene).
4. The side panel shows party gold.

## Acceptance criteria

- [ ] Harness: buy a Potion on a Vendor tile → gold drops by its price, pack grows, unit done; leaving without buying → unit not done.
- [ ] Harness: repair a damaged weapon at a Blacksmith; visit a village; open a chest.
- [ ] Snapshots below.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Harness: flows above.
- Snapshot: Armoury buy tab, Blacksmith repair tab.

## Completion notes

