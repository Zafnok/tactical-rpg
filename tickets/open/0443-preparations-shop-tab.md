---
id: "0443"
title: "Preparations: a Shop tab with the basic shop"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0409", "0044"]
nick_input: sign-off
completed:
---

# 0443 — Preparations: a Shop tab with the basic shop

## Context

Nick (ticket 0408, 2026-10-01; `docs/design/weapons-and-items.md`, *Battle
pack*): the Preparations screen "should also have a basic shop option --
some shops might sell unique items, this shop won't. It will only sell up
to the tier of items you have unlocked as a basic." From the first battle
with Preparations the player packs only consumables they own, so this shop
is where they get them before a story battle.

Builds on:

- the Preparations screen (0408): `crates/ui/src/screens/preparations.rs`
  (`Tab`, `Focus`, `PreparationsScreen`), which edits a `BattleSetup`
  (its `stock`, `gold`, `pack`) through `crates/core/src/prep.rs`;
- the shop rules in core (0308, `crates/core/src/shop.rs`: `Shop`,
  `ShopSession`, `ShopKind`, `sell_price`) and the shop screen (0409,
  `ShopScreen`), which this tab reuses rather than copying;
- the decision in 0044: which items are basics, the tiers and what unlocks
  them.

## Nick input

**Sign-off:** Nick buys Potions on the Quick Battle's Preparations, packs
them, and comments.

## Scope

**In:**
- A `Shop` tab on the Preparations screen, between `Pack` and `Fight!`.
- The basic shop's inventory by tier, as data, following 0044.
- Gold shown on the screen; buying puts the item in the stock and takes the
  gold; selling as in `weapons-and-items.md` (half price).
- The debug Quick Battle gets some gold to try it with (a `solo_gold` field
  next to `solo_stock` in the battle file).

**Out (do not do):**
- Unique items, town shops, the world map (1007).
- Repairs, unless 0044 says the basic shop repairs.
- Changing the on-map shop rules.

## Implementation steps

1. Read 0044's record in `weapons-and-items.md`. Put the basic shop's
   inventory per tier in data (`assets/data/`), loaded and validated by
   `trpg-content` with every error reported (ADR-0005), and the tier the
   campaign has unlocked wherever 0044 says it comes from.
2. `core`: a function that builds the basic `Shop` for a tier, and
   `BattleSetup` methods in `prep.rs` to buy into / sell from the stock
   with the setup's gold, reusing `shop.rs`'s price rules. Unit tests.
3. `ui`: add `Tab::Shop`; draw it with 0409's shop lists (price, the
   unaffordable ones dimmed, `Gold 1 250` in the heading). Confirm buys
   one; a bought consumable is then on the `Pack` tab's stock list.
4. `assets/battles/quick.ron`: `solo_gold`, read by
   `trpg_content::battle_campaign`; document it in
   `assets/battles/README.md`.
5. Follow the `keyboard-input` skill for the help line.

## Acceptance criteria

- [ ] Harness: on the Quick Battle's Preparations, buy a Potion (gold drops by its price, the stock gains one), pack it, Fight! → it is in the battle's pack and the campaign's gold after a win reflects the purchase.
- [ ] An item the player can't afford is dimmed and can't be bought.
- [ ] The shop offers only the basics of the unlocked tier (test with two tiers).
- [ ] Snapshot of the Shop tab.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: buying and selling through the setup (gold, stock), tier inventory.
- Snapshot / integration: the Shop tab; the Harness flow above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
