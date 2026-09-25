# Chapter 1 scope

Decided: 2026-09-25
Source: ticket 0009

## Nick's words

> **Q1. Objective.** "To me a chapter represents a story beat. A chapter isn't
> tied to a single battle but does often finish with a climactic battle. But
> some chapters you might have many random overworld triggered skirmishes to
> train for a climactic battle while others might just be a single battle. For
> simplicity, sure, we can stick to a single battle for the opening/tutorial
> chapter. Let's pick Rout."
>
> **Q2. Size.** "Bigger: 5–6 units" (the offered option: lord + 4–5 units
> covering more classes, 12–14 enemies + boss, about 24×16, 20–30 min.)
>
> **Q2b. Roster.** "Full spread but Lord has a unique starting class/tree."
> (Full spread = lord + Rider + Archer + Cleric + Guard + Mage.)
>
> **Q3. Teaching.** "Contextual hints" (help bar + one-time tips).
>
> **Q4. Extras.** "We can introduce more in chapters 2 and 3. Keep it simple
> for tutorial none of these for now"
>
> **Q5. Preparations and pack.** "No prep, fixed pack" (the offered option:
> default loadouts, pack of 3 Potions with cap 3, 0 starting gold, 1000 gold
> for clearing the map; Preparations first appears in Chapter 2.)

## What a chapter is (Nick, for later chapters)

A **chapter is a story beat**, not a single battle. A chapter often ends with
a climactic battle, and some chapters may contain several battles first (e.g.
overworld-triggered skirmishes to train before the climax). How that works
belongs to world structure (0008) and game flow (0801 / later tickets).
**Chapter 1 is exactly one battle.**

## Rules for Chapter 1

### Objective and loss

- **Objective: Rout**: defeat every enemy unit on the map. The boss is one of
  them and counts like any other enemy.
- **No turn limit.**
- **Loss:** the lord falls → game over (`death-and-difficulty.md`, both modes).
  No other loss conditions (no ally or neutral units to protect).
- **Map difficulty tier: Easy** → 2 rewind charges (*Claude's choice*,
  *tunable*; it's the opening tutorial map).

### Player roster (6 units, fixed, all deployed)

| Slot | Class | Notes |
| ---- | ----- | ----- |
| Lord | **Unique lord class** (tier 1 of the lord's own line) | Nick: the lord has a unique starting class/tree. Designed in ticket **0016**. |
| 2 | Rider | mounted |
| 3 | Archer | ranged |
| 4 | Cleric | healer (Heal) |
| 5 | Guard | armoured tank |
| 6 | Mage | Fire |

- All tier 1. Character names, personalities and who's who come from the
  story (0701); this file fixes only the class slots.
- Starting levels, stats and loadouts: 0803, per `progression.md` and
  `weapons-and-items.md`.

### Enemies (*starting values, tunable*)

- **12–14 enemies + 1 boss** (the boss has a name and portrait, 0803).
- Mix: ordinary tier-1 human enemies, mostly Brigand, Raider and Archer. 0803
  may add 1–2 Riders, Guards or a Flier so the per-type weapon traits show
  (spears vs mounted, bows vs flying).
- **No elemental enemies** in Chapter 1 (Nick: no extras).
- No reinforcements (Nick: no extras).

### Map (*starting values, tunable*)

- About **24×16 tiles**.
- **Length:** about 20–30 minutes for a first-time player.
- Layout idea (Claude's rough sketch for 0803; one character per tile, not
  final): the party starts at the bottom left. Two lanes lead to the boss's
  fort at the top right: a road over a bridge, and a slower route through a
  forest. It follows the FE principles listed in 0803 step 1.

  ```
  ........♣♣♣......^^^^^..
  ...♣♣♣♣♣♣♣.......^^^F^..
  ...♣♣♣♣♣.......E..^.B.E.
  .....♣♣.....E.......E...
  ..............E.........
  ≈≈≈≈≈≈≈≈≈≈=≈≈≈≈≈≈≈≈≈≈≈≈≈
  ≈≈≈≈≈≈≈≈≈≈=≈≈≈≈≈≈≈≈≈≈≈≈≈
  ....E.....:.....E.......
  ..♣♣♣.....:.......♣♣♣...
  .♣♣E♣♣....:......♣♣E♣♣..
  ..♣♣♣.....:.......♣♣♣...
  ..........:....E........
  ....P.P...:.............
  ...P.L.P..:....^^^......
  ....P.....:...^^^^^.....
  ..........:.............
  ```
  `L` lord, `P` player units, `E` enemies, `B` boss on `F` fort, `=` bridge,
  `:` road, `♣` forest, `^` mountain, `≈` river. This sketch isn't a spec.
  0803 designs the real map (counts in the sketch are illustrative).

### Teaching

- **Contextual hints**: the help bar plus one-time tips when something new
  happens (ticket 0406). **No forced tutorial steps.**

### Extras

- **None** in Chapter 1: no talk-to-recruit, village, chest, on-map shop,
  reinforcement wave, scripted terrain-magic moment, elemental enemy or
  battle note. Nick: "We can introduce more in chapters 2 and 3."
- The Mage's Fire still follows the normal `magic.md` rules (it can burn a
  forest tile if the map has one); Chapter 1 just doesn't script or hint at it.

### Preparations, pack and gold

- **No Preparations screen** before Chapter 1 (FE-style opening). It first
  appears in Chapter 2.
- Units start with their **default loadouts** from the chapter data (0803).
- **Pack cap 3**, **default pack: 3 Potions** (*tunable*).
- **Starting gold: 0.** Starting stock: empty. (*tunable*)
- **Clear reward: 1000 gold** (*tunable*; buys 3 Potions at 300).

## Open sub-questions (deferred)

- **The lord's unique class line** (weapons, movement, tiers, how it
  relates to reclassing, any signature skill): ticket **0016**.
- **Multi-battle chapters** (skirmishes before a climax): 0008 / later.
- **Which extras Chapter 2 and 3 introduce**: decided with those chapters.
