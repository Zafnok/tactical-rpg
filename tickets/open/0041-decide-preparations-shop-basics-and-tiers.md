---
id: "0041"
title: "Decide: the Preparations shop's basic items and how its tiers unlock"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0804"]
nick_input: decision
completed:
---

# 0041 — Decide: the Preparations shop's basic items and tiers

## Context

While building the Preparations screen (ticket 0408, 2026-10-01) Nick said
the screen should also have a shop, and how it differs from other shops
(`docs/design/weapons-and-items.md`, *Battle pack*):

> "The preparation screen, when it comes, which could be ch2, should also
> have a basic shop option -- some shops might sell unique items, this shop
> won't. It will only sell up to the tier of items you have unlocked as a
> basic. So for the beginning it might be Potion (heal 20) for the mid it
> might be a consumable to heal 40, etc."

and that from then on the player brings only their own consumables ("from
i.e. shops or battles or collecting or opening chests"). `world-structure.md`
already says Preparations has a shop from Chapter 2 and that towns'
inventories grow with the story; shop rules (gold, buying, selling) are in
`weapons-and-items.md`, *Money and shops*, and built in core by 0308.

What is not decided: which items are "basics", what the tiers are, and what
unlocks a tier. Ticket 0442 (the shop tab) waits for this.

Chapter 1 has no Preparations and no shop (`chapter-1.md`), so this waits
for the Chapter 1 playtest (0804), like the other post-playtest decisions.

## Nick input

**Decision**, with the `ask-nick` skill. At most three questions per
message:

1. **What unlocks a tier?** Story progress (a chapter number, as FE's
   between-chapter armouries grow), the world map's towns (the best town
   shop visited), the army's level, or something bought or found. Say what
   each means for a player who falls behind.
2. **What does the basic shop sell?** Consumables only (his examples are
   healing items), or also basic weapons, armour and accessories (Iron,
   then Steel)? If weapons are sold here, does it repair too, or is that
   only the Blacksmith?
3. **The healing items per tier.** His examples are "Potion (heal 20)" and
   "a consumable to heal 40"; today's Potion heals 10 and the Elixir heals
   everything (`weapons-and-items.md`, *Chapter 1 consumables*, tunable,
   and all numbers rescale with 0013). Propose a tier table (name, heal,
   price) and ask whether lower tiers stay on sale.

Also ask (it came up in the same answer, as a "maybe"): should **green
units carry their own consumables**, not drawn from the player's pack?

## Scope

**In:**
- The decision, recorded in `docs/design/weapons-and-items.md` (his words,
  the rules, numbers marked Nick's or *tunable*) and the
  `docs/design/README.md` table.
- Updating ticket 0442's steps to match.
- `write-ticket` for anything else he chooses (new consumables in
  `items.ron`, green units' items).

**Out (do not do):**
- Any code or data.

## Implementation steps

1. Read `weapons-and-items.md` (*Battle pack*, *Money and shops*),
   `world-structure.md` (towns) and `chapter-1.md`.
2. Ask with `ask-nick`; check each option against `assets/data/items.ron`.
3. Record the answer; update 0442.

## Acceptance criteria

- [ ] `weapons-and-items.md` says which items the Preparations shop sells at each tier and what unlocks a tier, with Nick's words.
- [ ] `docs/design/README.md` row updated.
- [ ] Ticket 0442 matches the decision.

## Tests required

- None (documentation).

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
