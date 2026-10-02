---
id: "0408"
title: Preparations screen (loadouts and battle pack)
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0306", "0403"]
nick_input: sign-off
completed: 2026-10-01
---

# 0408 — Preparations screen

## Context

Nick (0003): "on a screen before the match you can select … consumables to
bring into the match and then those are the ones your whole army can
use/share", with a per-battle cap, and each unit sets up a loadout
(3 weapons, 1 armour, 1 accessory). Rules in
`docs/design/weapons-and-items.md` (Loadout, Battle pack); types from 0306.

## Nick input

**Sign-off:** Nick sets up loadouts and a pack before a Quick Battle and
comments on how easy it is.

## Scope

**In:** `PreparationsScreen` (in `ui`, returns a `BattleSetup` with loadouts
and pack), `Loadouts` and `Pack` tabs, `Fight!` to start.

**Out:** shops (0409), choosing/swapping start positions, chapter flow wiring
(0801 pushes this screen when the chapter has `preparations: true`).
**Exception (2026-09-29):** 0801 no longer waits for this ticket. If 0801 is
already done, this ticket also pushes `PreparationsScreen` from `ui::flow`
for battles with `preparations: true`, and removes 0801's validator check
that rejects `preparations: true` until this screen exists.

## Implementation steps

1. Screen with tabs (Left/Right cursor actions switch): **Loadouts**, **Pack**, and a `Fight!`
   entry.
2. **Loadouts:** list of deployed units; choosing one shows its 3 weapon
   slots, armour and accessory beside the stock, filtered to what that unit
   can use (unusable items dimmed with the reason, e.g. `needs rank D`).
   Confirm moves an item between a slot and the stock. Show the unit's attack
   speed change live (`AS 12 → 10`) using `core`'s formula.
3. **Pack:** stock consumables on the left (never seals: `ItemDef::Seal`
   items aren't consumables and can't be brought into a battle, ticket
   0603), pack on the right, header
   `Pack 3/6`; adding past the cap shows a message instead. The chapter's
   default pack is preselected.
4. `Fight!` returns the setup. `Cancel` at the top level does nothing unless
   the caller allows backing out (then it asks `Leave preparations?`).
5. Debug Quick Battle opens this screen first.

## Acceptance criteria

- [x] Harness: move a weapon from the stock into a slot, add 2 Potions to the pack, Fight! → the battle starts with that loadout and pack.
- [x] The pack can never exceed the cap; unusable items can't be equipped.
- [x] Snapshots below.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Harness: flow above; cap enforcement.
- Snapshot: Loadouts tab (with a dimmed unusable item), Pack tab at the cap.

## Completion notes

**Done.** A battle file with `preparations: true` now opens the Preparations
screen before the battle; the debug Quick Battle does, with a stock of
spare gear, six Potions and two Elixirs to try it with.

- **`core::prep`** (rules, on `BattleSetup`): `gear_from_stock`,
  `gear_to_stock`, `pack_from_stock`, `pack_to_stock`, `unusable` (why a
  unit can't use an item), `gear_slots`, `attack_speed`. Unit tests for
  every refusal.
- **`ui::screens::preparations::PreparationsScreen`**: tabs `Loadouts`,
  `Pack`, `Fight!`. It edits the `BattleSetup` and hands it back.
- **Flow** (`ui::flow`): `Stage::Preparations` before the battle;
  `Restart Battle` and `Retry Battle` go back to it, as the player left it.
- **Content**: the validator no longer refuses `preparations: true`; it
  refuses a `default_pack` on such a battle instead. New optional battle
  field `solo_stock` (the stock when a battle is played on its own, i.e.
  the Quick Battle). `assets/battles/README.md` updated.

**Nick's answers (2026-10-01)**, recorded in `weapons-and-items.md` and
`death-and-difficulty.md`:

1. With Preparations there are **no suggested items**: the pack starts
   empty and the player brings only consumables they own. (So step 3's
   "the chapter's default pack is preselected" was dropped; a battle
   without Preparations still gets its default pack for free.)
2. `Retry Battle` and `Restart Battle` land **back on Preparations**, and
   Game Over should also offer **Rewind** while charges are left (new
   ticket 0822).
3. Preparations should get a **basic shop** (new tickets 0038 to decide its
   items and tiers, 0436 to build the tab). Green units carrying their own
   consumables is noted as an open "maybe".

**Deviations from the ticket**

- Left and Right switch tabs only while the cursor is on the tab row.
  Confirm (or Down) opens a tab; Cancel steps back out. Inside the Pack tab
  Left and Right switch between the stock and the pack.
- The pack and stock lists group copies (`Potion ×4`), like the battle's
  Item menu.
- The tests that started the Quick Battle from the title now pass through
  Preparations (`Left f`: Left wraps to `Fight!`). The test helper
  `quick_battle()` packs three Potions itself, so the battle tests see the
  pack they always had.

4. (PR #140) **Trading:** "if we have 2 archers and only taking one but
   the other one has the stronger bow, we should be able to trade their
   equips around in the prep screen but not once inside battle". So the
   `Loadouts` tab lists the whole army: units left out of the battle come
   after the others, dimmed and marked `Not in this battle`, and their
   gear moves like anyone's. The Quick Battle has a benched Test Scout
   with a Steel Bow to try it (`solo_bench` in the battle file).
   `core::prep::Preparations` holds the setup and the bench;
   `Campaign::bench` / `set_members` carry the bench in and out.

**Claude's starting rules** (Nick may veto)

1. **Trades go through the stock:** to give the benched scout's Steel Bow
   to the archer, put it back in the stock from her slot, then take it
   from the stock into his. There is no direct unit-to-unit swap.
2. **Swapping:** putting the Steel Spear in a slot that holds the Iron
   Spear sends the Iron Spear back to the stock, wear and all.
3. **What a unit wields:** it goes on holding the weapon it had in hand. If
   you put that weapon back in the stock, it holds its first usable weapon
   instead.
4. **The attack speed line:** on a weapon slot it shows the speed with that
   slot's weapon in hand, and highlighting a stock item shows the change
   before you take it (`AS 7 → 6 with Iron Sword`). On armour and
   accessories it uses the weapon in hand (`AS 6 → 4 with Iron Sword` for
   Chain Mail on the lord). Skill bonuses that only count in a fight aren't
   included.
5. **Leaving:** in the story you can't back out of Preparations (there is
   nowhere to go back to yet); `Fight!` is the way on. The Quick Battle
   asks `Leave preparations?` and returns to the title.
6. **Taking an item off the pack** removes the copy packed last.
7. **Quick Battle's test stock** (placeholder data): Steel Spear, Iron
   Axe, Iron Sword, Iron Plate, Warded Robe, Chain Mail, Speed
   Ring, Power Ring, Focus Charm, 6 Potions, 2 Elixirs.

**After merging `main` (2026-10-02):** seals (0603) are offered nowhere on
the screen (tested). A suspended battle (0802) continues with what
Preparations set up, and a restart after it goes back to Preparations as
they were left: `BattleSetup::prepared_as` takes the loadouts, stock and
pack from the battle's first state, which the suspend save holds.

**Follow-up tickets:** 0038 (decide the Preparations shop's basics and
tiers, after the playtest), 0436 (the Shop tab, blocked by 0409 and 0038),
0822 (Game Over offers Rewind).

**For Nick to try** (Pages build, title → Quick Battle): `Loadouts` → Test
Lord → his empty third weapon slot → the Iron Sword (the spear, bow and axe
are dimmed with why); try the Knight's armour slot (Iron Plate) and a ring.
Trade: pick Test Scout (dimmed, last in the list), put her Steel Bow back
in the stock, then give it to Test Archer.
Then `Pack` → add Potions until it says the pack is full → `Fight!`. In the
battle, the map menu's `Restart Battle` brings you back to Preparations
with everything as you left it.
