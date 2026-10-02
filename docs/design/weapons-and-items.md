# Weapons, gear and items

Decided: 2026-09-25
Source: ticket 0003

## Nick's words

> **Q1. How do weapons work?** "I'm enjoying how Fortune's Weave handles it.
> It just released. Could you look into it a bit? Basically, no built-in
> triangle forcing anything, but each weapon has a bonus. Like bows effective
> against flying unit, armored enemies typically show up with more Def and
> less Res so encouraged to use magic users on those enemies, spear good
> against land cavalry, axe has min hit, etc. We don't have to steal it ver
> batim but most of these are satisfying to forming a good army formation.
> Durability is also done nicely there where there's both a repair option at
> blacksmith and the durability only goes down for skills, and each skill
> takes a different amount of durability balanced by how powerful its
> perceived effect is."
>
> **Q2. Inventory & consumables:** "so I haven't played FFT or Triangle
> Strategy so I don't know exactly how the shared bag works but how it
> inspired me from what you said is maybe the unit can setup their loadout
> (ie 1 armor piece, 1 accessory, and however many weapons lets say 3) and
> they dont need to worry about consumables, on a screen before the match you
> can select idk 10 or 20 consumables to bring into the match and then those
> are the ones your whole army can use/share for the battle. Then when doing
> oevrworld activities you accumulate more but again every battle has a cap
> on the consumables you can take from your real inventory and those are
> shared between all your units."
>
> **Q3. Money & shops:** "FE on-map shops"

Follow-ups:

> **How should attack speed work?** (weight, weapon skill, gear) "weight, str,
> skill, and spd... exact algorithm can be worked out by you but make it make
> sense. obviously gauntlets should be easier to hit 2, 3, 4 hits than
> something heavier like axe"
>
> **When do Combat Arts arrive?** "Own ticket, in Ch1 (Recommended)"
>
> **Battle consumable cap?** "I think we can scale them to the battle. Maybe
> some battles are many units but a small bag. Maybe some battles are many
> units with a large bag. Maybe some battles are a handful of units with a
> small bag. Etc"
>
> **Do enemies carry potions?** (2026-09-28, ticket 0501) "I think I would
> stick to no potions and only dedicated healers for the enemies in a
> battle."
>
> **Preparations: where do the items you can bring come from?** (2026-10-01,
> ticket 0408) "the pack shouldn't have potion-specific slots but item
> slots... and there shouldn't really be suggestions for items for
> battles... so from the point you are able to buy or obtain consumables
> (which might be ch2, might be after going to open world setting) then the
> player has to prepare their own consumables from i.e. shops or battles or
> collecting or opening chests... and of course they would only bring their
> own. Maybe green units could spawn with their own consumables not drawn
> from your pack though. The preparation screen, when it comes, which could
> be ch2, should also have a basic shop option -- some shops might sell
> unique items, this shop won't. It will only sell up to the tier of items
> you have unlocked as a basic. So for the beginning it might be Potion
> (heal 20) for the mid it might be a consumable to heal 40, etc."
>
> **Units left out of a battle keep their gear out of reach?** (2026-10-01,
> PR #140) "make sure we have trade option for this case otherwise i
> agree... but if we have 2 archers and only taking one but the other one
> has the stronger bow, we should be able to trade their equips around in
> the prep screen but not once inside battle"

Reference (researched for Nick, 2026-09-25): *Fire Emblem: Fortune's Weave*
has no weapon triangle. Swords deal 1.2x damage on follow-up strikes, spears
are 2x effective against mounted units, bows 3x against flyers (range 2),
axes always deal at least 5 damage but hit less often, gauntlets give high
avoid, magic targets Res. Normal attacks cost no durability; only Combat Arts
do. At 0 durability a weapon is broken (weaker, less accurate, no arts) until
a blacksmith repairs it for gold and materials. Weapons have weight that
slows attack speed.

So: **no weapon triangle**. Each weapon type has its own trait (Fortune's
Weave-style) so army composition matters. **Durability only drops when using
Combat Arts** (designed in ticket 0014), each art costing its own amount; a
broken weapon still works, badly, until repaired at a blacksmith. Each unit
has a **loadout** of 3 weapons, 1 armour and 1 accessory; **consumables live
in one shared battle pack** whose size is set **per battle**. **FE-style
gold**, with shops on maps as well as between chapters.

Everything marked *tunable* is a starting value Claude chose; balance tickets
may change it without asking. Items marked *Claude's starting rule* fill a gap
Nick's answers didn't cover; Nick may veto them. Unmarked rules are Nick's.
All numbers are FE-sized, like `stats-and-combat.md`, and will be rescaled
with the rest in ticket 0013.

## Weapon types and traits

There is **no weapon triangle**: no type gets bonuses or penalties just
from facing another weapon type.

| Type | Trait (Nick's choice, from Fortune's Weave) | Exact rule (*tunable* numbers) |
| ---- | ------------------------------------------- | ------------------------------ |
| Sword | Stronger follow-ups | Strikes 2..N (a unit's extra strikes) deal `damage * 6 / 5` (×1.2, rounded down) before crit. The first strike is normal. |
| Spear | Good against land cavalry | Effective against **Mounted**: weapon might ×2. |
| Axe | Minimum damage, lower accuracy | A hit always deals at least **5** damage: `damage = max(damage, 5)` before crit. Axes have lower base `hit` in their stats. |
| Bow | Good against flyers, ranged | Effective against **Flying**: weapon might ×3. Standard bows have range **2 only** (cannot attack or counter an adjacent unit). |
| Gauntlet | Evasive, light | While a gauntlet is **equipped**, the wielder's avoid is **+15**. Gauntlets are the lightest weapons (easiest to strike 2/3/4 times). |
| Magic | Hits Res | Not a weapon: magic is **innate spells** with uses per battle, outside the loadout. See `magic.md` (0004). |

**Armoured units** have no special weapon rule: armoured *classes* have high
Def and low Res (Nick: "encouraged to use magic users on those enemies").
That is class data for ticket 0005.

**Unit tags.** Classes carry tags (ticket 0005/0302 data): `Mounted` (land
cavalry), `Flying`, `Armored`. Flying units are **not** `Mounted` (spears are
for land cavalry). `Armored` is used by effective weapons added later
(e.g. an FE-style Armorslayer) and by class design.

**Effectiveness.** A weapon lists `effective: [(tag, multiplier)]`; its type
trait adds to that list (spear: `(Mounted, 2)`, bow: `(Flying, 3)`). If the
defender has several matching tags, use the **largest** multiplier (they do
not stack). The forecast marks effective attacks.

### Formulas (additions to `stats-and-combat.md`)

```
eff_mult    = max multiplier among weapon.effective whose tag the target has, else 1
might_eff   = broken ? weapon.might / 2 : weapon.might
power       = (Physical ? A.Str : A.Mag) + might_eff * eff_mult
damage      = max(0, power - mitigation)                 (as before)
if weapon.kind == Axe and the strike hits:  damage = max(damage, 5)
follow-up damage (strikes 2..N):  Sword ? damage * 6 / 5 : damage
crit damage = (that strike's damage) * 3

hit         = clamp(weapon.hit - (broken ? 20 : 0) + A.Dex * 2 - avoid_B, 0, 100)
avoid_B     = B.Spd * 2 + terrain_B.avoid + (B has a Gauntlet equipped ? 15 : 0)
```

Order for a strike's damage: base `damage` → axe minimum → sword follow-up
×1.2 → crit ×3. The forecast shows the first-strike damage and, for swords
with more than one strike, the follow-up damage too.

`A.Str`, `A.Spd`, `B.Def`, etc. are **gear-adjusted** stats: permanent stats
plus the equipped armour and accessory bonuses. Gear may never push a stat
above the hard ceiling in `stats-and-combat.md` (classes have no caps,
ticket 0019).

## Attack speed (`as_bonus`)

Nick: weight, Str, weapon skill and Spd all matter; gauntlets should reach
2/3/4 strikes more easily than axes.

```
burden   = max(0, weapon.weight + armour.weight - A.Str / 5)
as_bonus = rank_speed[A's rank in weapon.kind] - burden
AS_A     = A.Spd + as_bonus          (A.Spd is gear-adjusted)
```

| Rank | E | D | C | B | A | S |
| ---- | - | - | - | - | - | - |
| `rank_speed` (*tunable*) | 0 | 0 | +1 | +2 | +3 | +4 |

- **Weight** slows you; **Str** carries it (every 5 Str cancels 1 weight);
  **weapon skill** gives speed back as you master the weapon; **gear**
  (e.g. a Speed Ring) raises Spd. Level never adds directly.
- A unit with no weapon equipped uses weight 0 and rank bonus 0.
- `stats-and-combat.md`'s worked examples use weapons with weight 0, rank E,
  no armour and **no type trait**, so their `as_bonus` is 0 and their numbers
  are unchanged.

What this does (illustration, all at Str 10, rank B, enemy AS 10):

| Unit | Spd | Weapon (wt) | Armour (wt) | burden | AS | diff | strikes |
| ---- | --- | ----------- | ----------- | ------ | -- | ---- | ------- |
| Brawler with Speed Ring | 20 + 2 | Iron Gauntlets (1) | Leather Vest (0) | 0 | 24 | 14 | **3** |
| Brawler | 20 | Iron Gauntlets (1) | Leather Vest (0) | 0 | 22 | 12 | 2 |
| Fighter | 20 | Iron Axe (6) | Chain Mail (2) | 6 | 16 | 6 | 2 |
| Fighter | 20 | Steel Axe (8) | Chain Mail (2) | 8 | 14 | 4 | 2 |

## Weapon stats

Every weapon has:

| Field | Meaning |
| ----- | ------- |
| `kind` | Sword, Spear, Axe, Bow, Gauntlet (spells are not weapons; see `magic.md`) |
| `rank` | Minimum weapon rank to wield it (E…S) |
| `might`, `hit`, `crit` | As used by the combat formulas |
| `weight` | Feeds `burden` |
| `min_range`, `max_range` | Tiles (Manhattan distance) |
| `damage_type` | Physical or Magical |
| `durability` | Maximum durability; each owned copy tracks `durability_left` |
| `effective` | Extra `(tag, multiplier)` pairs beyond the type trait (usually empty) |
| `price` | Gold (buy); sells for half |

A unit can wield a weapon if its class can use that `kind` (0005) and its
rank in that kind is ≥ the weapon's `rank`.

### Starter weapons (*tunable*)

| Weapon | Kind | Rank | Mt | Hit | Crit | Wt | Rng | Dur | Price |
| ------ | ---- | ---- | -- | --- | ---- | -- | --- | --- | ----- |
| Iron Sword | Sword | E | 5 | 90 | 0 | 2 | 1 | 20 | 460 |
| Steel Sword | Sword | D | 8 | 75 | 0 | 4 | 1 | 25 | 600 |
| Iron Spear | Spear | E | 7 | 80 | 0 | 4 | 1 | 20 | 360 |
| Steel Spear | Spear | D | 10 | 70 | 0 | 6 | 1 | 25 | 480 |
| Iron Axe | Axe | E | 8 | 75 | 0 | 6 | 1 | 20 | 270 |
| Steel Axe | Axe | D | 11 | 65 | 0 | 8 | 1 | 25 | 360 |
| Iron Bow | Bow | E | 6 | 85 | 0 | 3 | 2 | 20 | 540 |
| Steel Bow | Bow | D | 9 | 70 | 0 | 5 | 2 | 25 | 720 |
| Iron Gauntlets | Gauntlet | E | 3 | 95 | 5 | 1 | 1 | 20 | 400 |
| Steel Gauntlets | Gauntlet | D | 5 | 85 | 5 | 2 | 1 | 25 | 560 |

All are Physical. Magic is innate spells, not weapons (`magic.md`). Which of these appear in Chapter 1 is
up to the Chapter 1 roster/enemies (0803).

## Weapon ranks (weapon skill)

- Each unit has a rank per weapon kind its class can use: `E, D, C, B, A, S`.
  Starting ranks and the highest rank a class can reach come from 0005.
- **Weapon EXP** (Nick, ticket 0014: "a formula of base + amount of
  damage"; numbers *tunable*): after each combat, a unit gains weapon EXP in
  the kind it fought with:

  ```
  base  = 2 if at least one of its strikes hit
          1 if it struck but missed every time
          0 if it didn't strike (no weapon EXP at all)
  base  = base * 2 if it used a Combat Art this combat   (4 / 2)
  bonus = dealt / 5
  weapon_exp = base + bonus
  ```

  `dealt` is the HP its strikes actually removed this combat, summed over
  every target (a Line Pierce strike counts). Overkill doesn't count: a
  9-damage strike on a unit with 4 HP left adds 4. So a big hit gives a big
  extra, and a scratch gives little or none (under 5 damage adds nothing).
  Examples: two hits of 9 and 10 → `2 + 19/5 = 5`; one hit of 3 → `2 + 0 = 2`;
  the same 9 + 10 with an art → `4 + 3 = 7`.
- Rank thresholds (total weapon EXP, FE GBA values): D 30, C 70, B 120,
  A 180, S 250.
- Ranks gate weapons (table above) and add `rank_speed` to attack speed.

## Durability

- Each weapon copy has `durability_left` (starts at the weapon's
  `durability`).
- **Normal attacks and counters never cost durability.**
- **Combat Arts** cost durability: each art has its own cost, higher for
  stronger effects (Nick). **Class actives** cost durability too (Nick,
  0014). The arts, their costs and the active costs are in
  [`combat-arts.md`](combat-arts.md). An art or active can be used while the weapon
  isn't broken, even with less left than its cost: it spends what is left
  and the weapon breaks after the action (Nick, 0414 review).
- At `durability_left == 0` the weapon is **broken**: it still attacks and
  counters, with **might halved** (rounded down) and **−20 hit** (*tunable*),
  and cannot use Combat Arts. Its type trait still applies.
- **Repair** at a Blacksmith (on a map or between chapters; see Shops) restores
  `durability_left` to full. Cost (*tunable*): `ceil(price / 2 × missing / durability)`
  gold. Materials for repairing rare weapons are deferred until materials
  exist (later; see `world-structure.md`); Chapter 1 repairs cost gold only.
- Armour, accessories and consumables have no durability.

## Loadout (per unit)

Nick's shape:

| Slot | Count | Notes |
| ---- | ----- | ----- |
| Weapons | **3** (0 for tier-3+ magic classes, `magic.md`) | One is **equipped** at a time |
| Armour | 1 | Optional |
| Accessory | 1 | Optional |

- The **equipped** weapon is the one used to counter (FE). Choosing a weapon
  when attacking equips it. Changing the equipped weapon from the unit's
  menu is free (does not end the action) (FE rule).
- Units don't carry consumables; see Battle pack.
- Loadouts are set before battle (Preparations). **No trading during
  battle** (Nick, PR #140: trade "in the prep screen but not once inside
  battle"; loadouts are fixed once the battle starts, and consumables are
  already shared).
- Everything not in a loadout is in the party's **stock** (shared,
  unlimited *tunable*).

**On the Preparations screen** (ticket 0408; all *Claude's starting rules*
unless marked):

- The `Loadouts` tab lists **the whole army** (Nick, PR #140): the units
  going into this battle, then the ones left out of it (dimmed, marked
  `Not in this battle`), so gear can be **traded between any of them**.
  With two archers and only one in the battle, the other's stronger bow
  can go to the one who fights.
- Pick a unit, then one of its slots, then an item from the stock: the item
  goes in the slot and what was there goes back to the stock. A weapon
  keeps its durability either way. A trade is two such steps: put the
  benched archer's bow back in the stock, then give it to the other
  (*Claude's starting rule*: trades go through the stock, there is no
  unit-to-unit swap).
- A unit can only be given **gear it can use**: a weapon of a kind its
  class uses, at a rank it has, and armour of a weight its class wears.
  Anything else is shown dimmed with the reason (`needs rank D`,
  `can't use axes`, `can't wear heavy armour`) and can't be taken. (Ticket
  0408's rule.)
- The unit goes on wielding the weapon it had in hand. If that weapon was
  put back, it wields its first usable weapon.
- The attack speed line shows the speed with the weapon of the highlighted
  slot in hand, and the change a highlighted stock item would make
  (`AS 6 → 4 with Steel Sword`), by the formula above. Skill bonuses that
  only apply in a fight aren't counted.

### Armour (*tunable*)

Classes may be limited to some armour weights (Light / Medium / Heavy) by 0005.

| Armour | Type | Bonus | Wt | Price |
| ------ | ---- | ----- | -- | ----- |
| Leather Vest | Light | Def +1 | 0 | 300 |
| Chain Mail | Medium | Def +3 | 2 | 800 |
| Iron Plate | Heavy | Def +5 | 5 | 1500 |
| Warded Robe | Light | Res +2 | 0 | 700 |

### Accessories (*tunable*)

Any unit can wear any accessory.

| Accessory | Bonus | Price |
| --------- | ----- | ----- |
| Speed Ring | Spd +2 | 2000 |
| Power Ring | Str +2 | 2000 |
| Focus Charm | Dex +2 | 1500 |

## Battle pack (shared consumables)

- Before a battle, on the **Preparations** screen, the player picks
  consumables from the stock to bring, up to the battle's **pack cap**.
  The pack's slots take any consumable. **Nothing is suggested or
  pre-filled** (Nick, 0408): the pack starts empty and the player brings
  only what they own, from shops, battles, chests and so on.
- The cap is **set per battle** in the chapter data (Nick: "scale them to the
  battle"). Chapter 1's cap comes from `chapter-1.md` (0009); default if a
  chapter doesn't say: **6** (*tunable*).
- A battle without a Preparations screen (e.g. an opening chapter) uses the
  chapter's default loadouts and default pack, given for free. A battle
  with Preparations has no default pack.
- **Restarting** such a battle (`Restart Battle`, or `Retry Battle` after a
  defeat) goes **back to Preparations** (Nick, 0408), with the loadouts and
  pack as the player left them, so they can change them before trying
  again (`death-and-difficulty.md`).
- Preparations can't be left in the story (*Claude's starting rule*: there
  is nowhere to go back to until the world map); `Fight!` is the way on.
- **The Preparations shop** (Nick, 0408): the screen also gets a **basic
  shop**. It sells no unique items, only the basics up to the tier the
  player has unlocked (early on a Potion; later a stronger healing item,
  and so on). Which items are basics and what unlocks a tier: ticket 0038;
  built in ticket 0436.
- **Using an item** (*Claude's starting rule*, FE Vulnerary): `Item` in the
  action menu → pick a consumable from the pack → target the unit itself or an
  adjacent ally → the item is consumed and **the unit's action ends**.
- Items gained during the battle (chests, villages, bought on-map) go
  straight into the pack, even past the cap (the cap only limits what you
  bring in) (*Claude's starting rule*).
- After the battle, unused pack items return to the stock.
- **Seals are not consumables** (Nick, ticket 0603): promotion and reclass
  seals stay in the stock and are only used between battles
  (`progression.md`). They can't be put in the pack.
- **Enemies carry no consumables** (Nick, 0501): they never use potions.
  Enemy healing comes only from dedicated healer units (heal spells,
  `magic.md`).

### Chapter 1 consumables (*tunable*)

| Item | Effect | Price |
| ---- | ------ | ----- |
| Potion | Restore 10 HP (never above max) | 300 |
| Elixir | Restore all HP | 3000 |

## Money and shops (FE)

- **Gold** is party-wide. Sources: map-clear reward (per chapter data),
  chests, villages, selling items (half price).
- **On-map shops** (FE): shop tiles on a map. A unit standing on one uses the
  `Shop` action. Kinds:
  - **Armoury**: weapons, armour, accessories.
  - **Vendor**: consumables.
  - **Blacksmith**: repairs (and later, forging).
  Buying, selling or repairing **ends that unit's action**; leaving without
  doing anything doesn't (FE rule). Bought weapons/gear go to the buyer's
  free loadout slot, else the stock.
- **Between chapters**, the same shop screen is available. From the world
  map on, shops are in **town nodes** (0008, `world-structure.md`).
- **Villages** (*Claude's starting rule*, FE): a player unit on the village
  gate uses `Visit`: a one-time gift (gold, an item or a scene), then the
  village closes. Enemies destroying villages: not in Chapter 1.
  **On hold:** there is no village tile yet; Nick deferred building tiles
  other than `fort` (`terrain.md`, 2026-09-26).
- **Chests** (*Claude's starting rule*): a player unit on a chest uses `Open`;
  no key needed for now. Contents: gold or an item.
- Chapter 1 has no shop, village or chest (0009, `chapter-1.md`).

## Worked examples

Rules as above; all other formulas from `stats-and-combat.md`. Ticket 0304
turns these into tests alongside that file's examples.

### Example W1 — sword follow-up vs. axe

| | HP | Str | Dex | Spd | Def | Weapon | Rank | Armour | Terrain |
|-|----|-----|-----|-----|-----|--------|------|--------|---------|
| Attacker (swordfighter) | 22 | 8 | 7 | 9 | 5 | Iron Sword | E | — | plain |
| Defender (brigand) | 20 | 9 | 3 | 5 | 4 | Iron Axe | E | — | plain |

Distance 1.

- Attacker: burden `max(0, 2 + 0 − 8/5) = 1`, AS `9 − 1 = 8`.
  Damage `8 + 5 − 4 = 9`; follow-up `9 × 6 / 5 = 10`; hit `90 + 14 − 10 = 94`;
  crit `0 + 3 − 0 = 3`.
- Defender: burden `max(0, 6 − 9/5) = 5`, AS `5 − 5 = 0`.
  Damage `9 + 8 − 5 = 12`; hit `75 + 6 − 18 = 63`; crit `0 + 1 − 1 = 0`.
- diff `8 − 0 = 8` → attacker **2 strikes**, defender 1.

**Forecast:** Attacker `dmg 9 (then 10) ×2, hit 94, crit 3` / Defender
`dmg 12, hit 63, crit 0`. If the attacker's second strike crits: 30.

### Example W2 — spear vs. cavalry; axe minimum damage

| | HP | Str | Dex | Spd | Def | Weapon | Rank | Armour | Tags | Terrain |
|-|----|-----|-----|-----|-----|--------|------|--------|------|---------|
| Attacker (soldier) | 20 | 7 | 5 | 6 | 5 | Iron Spear | E | Leather Vest (Def +1) | — | plain |
| Defender (cavalier) | 24 | 8 | 6 | 7 | 7 | Iron Axe | C | — | Mounted | plain |

Distance 1.

- Attacker: effective ×2 → power `7 + 7 × 2 = 21`, damage `21 − 7 = 14`;
  hit `80 + 10 − 14 = 76`; crit `0 + 2 − 1 = 1`. Burden
  `max(0, 4 + 0 − 7/5) = 3`, AS `6 − 3 = 3`.
- Defender: damage `8 + 8 − (5 + 1) = 10`; hit `75 + 12 − 12 = 75`;
  crit `0 + 3 − 1 = 2`. Burden `max(0, 6 − 8/5) = 5`, rank C `+1`,
  AS `7 + 1 − 5 = 3`.
- diff 0 → 1 strike each.

**Forecast:** Attacker `dmg 14 (effective), hit 76, crit 1` / Defender
`dmg 10, hit 75, crit 2`.

Variant W2b: the soldier has Def 20 instead. The cavalier's damage is
`max(0, 16 − 21) = 0`, raised to the axe minimum **5** (crit 15).

### Example W3 — bow vs. flyer at range, no counter

| | HP | Str | Dex | Spd | Def | Weapon | Rank | Tags | Terrain |
|-|----|-----|-----|-----|-----|--------|------|------|---------|
| Attacker (archer) | 18 | 6 | 8 | 7 | 4 | Iron Bow | E | — | plain |
| Defender (flier) | 18 | 7 | 7 | 12 | 4 | Iron Spear | E | Flying | forest |

Distance 2.

- Attacker: effective ×3 → power `6 + 6 × 3 = 24`, damage `24 − 4 = 20`
  (flyers get no terrain Def); hit `85 + 16 − 24 = 77` (no terrain avoid);
  crit `0 + 4 − 1 = 3`. AS `7 − max(0, 3 − 1) = 5`; flier AS
  `12 − max(0, 4 − 1) = 9` → attacker 1 strike.
- Defender: Iron Spear range 1 → **no counter**.

**Forecast:** Attacker `dmg 20 (effective), hit 77, crit 3` / Defender `—`.

### Example W4 — gauntlet avoid, broken weapon

| | HP | Str | Dex | Spd | Def | Weapon | Rank | Terrain |
|-|----|-----|-----|-----|-----|--------|------|---------|
| Attacker (fighter) | 26 | 10 | 5 | 6 | 5 | Iron Axe, **broken** | E | plain |
| Defender (brawler) | 22 | 7 | 8 | 10 | 3 | Iron Gauntlets | E | plain |

Distance 1.

- Attacker: might `8 / 2 = 4`, damage `10 + 4 − 3 = 11`; avoid of the
  brawler `10 × 2 + 0 + 15 = 35`; hit `75 − 20 + 10 − 35 = 30`;
  crit `0 + 2 − 2 = 0`. AS `6 − max(0, 6 − 2) = 2`.
- Defender: damage `7 + 3 − 5 = 5`; hit `95 + 16 − 12 = 99`;
  crit `5 + 4 − 1 = 8`. AS `10 − max(0, 1 − 1) = 10`; diff 8 → **2 strikes**.

**Forecast:** Attacker `dmg 11, hit 30, crit 0 (broken)` / Defender
`dmg 5 ×2, hit 99, crit 8`.

## Open sub-questions (deferred)

- **Combat Arts:** decided in [`combat-arts.md`](combat-arts.md) (0014).
- **Magic** weapons/tomes, their weight and traits: 0004.
- **Class weapon kinds, starting/max ranks, armour limits, tags:** 0005.
- **Pack cap, shops/villages/chests in Chapter 1:** decided in 0009
  (`chapter-1.md`: cap 3, no shops/villages/chests).
- **Materials and forging** at the blacksmith: later; towns, skirmishes or
  side quests on the world map (`world-structure.md`) are the likely sources.
- **Enemies destroying villages, chest keys, thief classes:** later chapters.
- **Number scale:** all numbers here rescale with ticket 0013.
- **The Preparations shop's basics and tiers** (Nick, 0408): ticket 0038.
  Nick's examples there ("Potion (heal 20)", then "a consumable to heal
  40") aren't the Potion's current 10 HP; the numbers are settled with
  0038 and 0013.
- **Green units with their own consumables** (Nick, 0408: "maybe"), not
  drawn from the player's pack: decide when the first green units that
  would carry one are designed.
