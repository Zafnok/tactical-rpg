# Magic and healing

Decided: 2026-09-25
Source: ticket 0004

## Nick's words

> **Q1. Should there be magic, and what kind?** "I think we can combine
> aspects of all of these. Personally I don't like magic as taking a weapon
> slot. Probably what we can do is, for first 2 tiers of magic classes, they
> can still bring 3 weapons (provided they have enough skill in those weapons)
> to battle but afterwards in advanced class tiers they have to rely on their
> innate magic spells. Then it can be number of uses per battle based (not MP
> but not aesthetically a weapon either). And I think the terrain changing
> sounds interesting so I'm in favor. For now since it's as low poly as it can
> get we can do something basic like the fire and ice terrain effects you
> mentioned. Lightning arcs sounds like it won't be clear enough. As for
> strengths weaknesses I think there can be certain enemies that want to be
> hit by certain spells but not overly common. As an example Fortunes Weave
> has the effective system. What we can have are things like a frost
> elemental or fire elemental that need to be hit by fire or water or
> something for big damage. Otherwise they'll just take normal damage or maybe
> fire on fire is ineffective or even heals it. But these elemental enemies
> won't be overly common. Fortunes Weave also does a good job showing at the
> beginnning of the battle the recommended strategy like stun this monster or
> use this trebuchet etc. When facing a unique enemy like an elemental we can
> show that as well."
>
> **Q2. Healing:** "Potions are available in the pack. But there should be
> classes specializing in white magic as well. And healing should give xp. And
> white magic will also be innate to whoever learned it. Not occupying a
> weapon slot since they'll eventually lose them."
>
> **Q3. Flavour (what is magic in this world?):** "Defer to story bible"

Follow-ups:

> **Spell uses:** "A: Uses per spell (Recommended)" (each spell has its own
> uses counter, refilled every battle).
>
> **Terrain effects:** "The forest burning can take 1 cycle of turns (you
> burn, enemy turn, on your turn it turns walkable). No damage since it won't
> be walkable while burning. I guess one exception is if we have a push spell
> it can do extra damage if they get pushed and collide with the burning
> forest but then pop back to whatever closest free tile. Frozen tiles are
> immediate and permanent for the battle"
>
> **Learning spells:** "C: Class + personal (Recommended)" (the class gives
> the basic list, kept after class change; each named character adds 1–2
> personal signature spells).

So: magic is **innate spells**, never items in the 3-weapon loadout. Each
spell has **uses per battle** that refill at the start of every battle.
Magic classes carry weapons in their first two tiers and **only spells from
tier 3 on**. **White magic** (healing) belongs to healer classes, works the
same way and gives EXP. **Fire and ice change terrain**: fire burns a forest
for one round and leaves it walkable; ice freezes water at once and for good.
There is **no magic triangle**. A few uncommon **elemental enemies** are weak
to one element and absorb their own, and battles that have them say so at the
start (Fortune's Weave-style battle notes). What magic *is* in the world is
decided by the story bible (0701).

Numbers marked *tunable* are starting values Claude chose; balance tickets
may change them without asking. Rules marked *Claude's starting rule* fill a
gap in Nick's answers, and Nick may veto them. Unmarked rules are Nick's. All
numbers are FE-sized, like `stats-and-combat.md`, and get rescaled with the
rest in ticket 0013.

## Spells, not tomes

- A **spell** is an innate ability a unit knows. It is **not an item**: it
  doesn't take a loadout slot, can't be bought, sold, traded or put in the
  stock, and has no durability.
- Each unit has a **spell list**, made of:
  - **class spells:** each class has a list of `(level, spell)` pairs. The
    unit learns a spell when it reaches that level in that class, or when it
    enters the class at or above that level. **Learned spells are kept after
    a class change.** The spell lists themselves are set by ticket 0005.
  - **personal spells:** each named character may have **1–2 signature
    spells** of their own (character data, chosen when the cast is written
    in 0701, with numbers set by 0005 or a balance ticket). They can be
    learned at a set level or known from the start.
- Every learned spell can be used in battle. There is no "equip N spells"
  limit (*Claude's starting rule*; revisit if lists get long).
- Any unit whose class or character grants spells can cast. Magic classes
  are just the classes whose lists are mostly spells.

## Magic classes and weapon slots

- Every class has a `weapon_slots` value, **3** by default.
- **Magic classes** (black magic and white magic lines) have 3 weapon slots in
  **tiers 1 and 2**, and can wield weapons their class allows as usual (rank
  requirement from `weapons-and-items.md`).
- From **tier 3** on, magic classes have **0 weapon slots**: they fight only
  with spells. They keep their armour and accessory slots.
- When a unit is promoted into a 0-slot class, its weapons go back to the
  party stock (*Claude's starting rule*).
- This means the class tree in 0005 needs **at least 3 tiers for the magic
  lines**. How many tiers the other lines have is up to 0005.

## Uses per battle

- Each spell has `uses` (its maximum). A unit tracks `uses_left` for every
  spell it knows. **All spells refill to full at the start of every battle.**
  Uses left over never carry between battles.
- Casting spends **1 use per combat or cast**, no matter how many strikes the
  combat has (*Claude's starting rule*: a mage who doubles doesn't burn two
  uses). That covers attacking, countering, healing, and casting on a tile.
- A spell with `uses_left == 0` can't be cast, and can't be used to counter.
- Uses are fixed per spell (*tunable*). Growing uses with Mag or level is not
  planned (it would be a balance ticket).

## Attack spells in combat

An attack spell fights like a weapon with `damage_type = Magical`, and uses
every formula in `stats-and-combat.md` and `weapons-and-items.md`:

```
power      = A.Mag + spell.might * eff_mult
mitigation = B.Res + terrain_B.defense
damage     = max(0, power - mitigation)          (then the affinity step below)
hit        = clamp(spell.hit + A.Dex * 2 - avoid_B, 0, 100)
crit       = clamp(spell.crit + A.Dex / 2 - B.Dex / 4, 0, 100)
```

- **Range** comes from the spell (`min_range`, `max_range`). The starter
  attack spells are range 1–2, so a caster can counter at either distance.
- **Attack speed** (*Claude's starting rule*): spells weigh 0 and have no
  rank, so `as_bonus = −max(0, armour.weight − A.Str / 5)`. Heavy armour
  still slows a caster down.
- **Strikes:** extra strikes from Speed work the same as with weapons (up to
  4). They cost no extra uses.
- **No broken state, no weapon-type trait, no Combat Arts** for spells
  (arts are ticket 0014; spell arts aren't planned).
- **Counters (equipped attack):** a unit's *equipped* attack is either one of
  its weapons or one of its attack spells. It counters with that, following
  the normal counter rules (range, uses left). Choosing a spell to attack
  equips it, and changing the equipped weapon or spell from the unit menu is
  free, as with weapons. A unit with 0 weapon slots and no usable attack spell
  can't counter.
- **Weapon EXP:** casting gives no weapon EXP, because spells have no rank
  (*Claude's starting rule*). Unit EXP for casting follows 0005.

## Elements and affinities

- Every spell has an `element`: `Fire`, `Ice` or `None` (Chapter 1 set).
  Lightning was left out because its effects wouldn't read clearly in ASCII
  (Nick). More elements (e.g. light/holy) can be added as data later.
- There is **no magic triangle**: elements never get bonuses against each
  other or against weapon types.
- A class (normally an elemental monster) may list **affinities**
  `[(element, Weak | Resist | Absorb)]`. Units without an affinity for an
  element take normal damage from it (Nick: "otherwise they'll just take
  normal damage"). Physical attacks ignore affinities.

| Affinity | Rule (*tunable* numbers) | Forecast shows |
| -------- | ------------------------ | -------------- |
| `Weak` | Counts as effectiveness: spell might ×**3**. If a unit-tag effectiveness also applies, use the larger multiplier (they never stack), as in `weapons-and-items.md`. | effectiveness marker `!` |
| `Resist` | `damage = damage / 2` (rounded down), after the base damage and before crit. | `(resist)` |
| `Absorb` | A hit **heals** the target by `damage` (never above max HP) instead of hurting it. Crit is forced to 0 for that strike. | `heals N` |

The order of a magic strike's damage is base → Resist halving → crit ×3.
(Weak is already in the might, and the axe minimum and sword follow-up are
weapon traits that spells don't have.) Hit and crit rolls follow the normal
2RN procedure, including for Absorb strikes.

### Elemental enemies (*tunable*, uncommon by design)

| Class | Affinities | Notes |
| ----- | ---------- | ----- |
| Fire Elemental | Fire `Absorb`, Ice `Weak` | Casts Fire |
| Frost Elemental | Ice `Absorb`, Fire `Weak` | Casts Frost |

Nick: these "won't be overly common". Whether Chapter 1 has one is decided in
0009. Their other stats come from 0005.

## Battle notes (strategy hints)

Nick asked for this, modelled on Fortune's Weave. A battle may define
**battle notes**: short lines that recommend a strategy, e.g.
`Frost Elemental: weak to Fire, absorbs Ice.` or `Burn the forest to cut off
the ambush.`

- Notes are shown **at the start of the battle**, before the first Player
  Phase banner (after Preparations, if there is one), and can be read again
  from the map menu's `Objective` page.
- A battle with a unique enemy such as an elemental should have a note about
  it. Each note may name the units it is about, so the screen can highlight
  them.
- The unit info screen lists a unit's affinities, so the player can always
  check.
- Notes are chapter data, written with the chapter's content.

## Terrain magic (fire and ice)

A Fire or Ice attack spell can be cast **on an empty tile** in range instead
of at a unit. Casting on a tile costs 1 use, **ends the action**, deals no
damage, and changes the terrain as below. Casting **at a unit** never changes
terrain (*Claude's starting rule*: this keeps Nick's "never walkable while
burning", since a unit can never be standing on a burning tile). The target
tile must be empty and must be one of the listed terrains, or the cast isn't
allowed.

| Spell element | Target terrain | Becomes | When / how long |
| ------------- | -------------- | ------- | --------------- |
| Fire | `forest` | `burning` | At once. Lasts **one round**: it turns into `burnt` at the **start of the caster's side's next phase**, before anyone acts. |
| — | `burning` | `burnt` | Permanent for the rest of the battle. |
| Ice | `water`, `sea` | `ice` | **At once, permanent** for the battle (Nick). |

Nick's example: "you burn, enemy turn, on your turn it turns walkable". A
player's fire burns through the Enemy and Other phases and is `burnt` when
the next Player Phase starts. An enemy's fire burns through the Other and
Player phases.

New terrain rules (*tunable* numbers; the glyphs and colours are look & feel
and belong to the terrain display data):

| Terrain | Movement | Def | Avoid | Heal % | Notes |
| ------- | -------- | --- | ----- | ------ | ----- |
| `burning` | Impassable for every movement type, **flying included** (*Claude's starting rule*) | — | — | — | Can't be targeted by more Fire |
| `burnt` | As `plain` | 0 | 0 | 0 | The forest's cover is gone for good |
| `ice` | As `plain` for all ground movement types; flyers as usual | 0 | 0 | 0 | Lets ground units cross sea; removes the water avoid bonus |

- A burning tile deals **no damage**, because nobody can be on it (Nick).
- Other interactions (fire on `thicket` or `ice`, ice on `burning`, fire
  spreading) **don't exist yet**. Adding one is a design change.
- **Push spells (future rule, Nick):** no Chapter 1 spell pushes. If one is
  added later: a unit pushed into a `burning` tile takes **5** collision
  damage (*tunable*, not reduced by Def or Res), then is placed on the
  nearest free tile it can stand on. "Nearest" means the smallest Manhattan
  distance from the burning tile, with ties broken by `Dir` order.
- **EXP for a tile cast** (*Claude's starting rule*): the same as a heal. The
  amount is set by 0005.

Mock-up (glyphs are illustrative only; 0011 and the terrain data fix the
real look):

```
 before      Fire on ♣ at (2,1)   next own phase   Ice on ≈ at (4,2)
 ♣ ♣ ♣ . .    ♣ ♣ ♣ . .            ♣ ♣ ♣ . .         ♣ ♣ ♣ . .
 ♣ ♣ ♣ . .    ♣ ♣ ^ . .   burning  ♣ ♣ , . .  burnt  ♣ ♣ , . .
 ≈ ≈ ≈ ≈ ≈    ≈ ≈ ≈ ≈ ≈            ≈ ≈ ≈ ≈ ≈         ≈ ≈ ≈ ≈ ░  ice
```

## White magic (healing)

- Healing spells are innate spells like any other: `element: None`, no
  combat, they can't counter.
- **Healer classes** specialise in white magic (their class lists are mostly
  healing spells). They follow the same weapon-slot rule as other magic
  classes: 3 weapon slots in tiers 1–2, and 0 from tier 3 on (Nick: "they'll
  eventually lose them").
- **Potions stay** in the shared battle pack (`weapons-and-items.md`).
- Heal rule:

```
amount = min(spell.heal_power + A.Mag, target.max_hp - target.hp)
```

- Target: an ally (Player or Ally faction, for a player caster; same side
  for an enemy caster) within the spell's range whose HP is below max. The
  caster can't target itself (*Claude's starting rule*, as with FE staves).
- A heal always works: no hit roll and no counter. It spends 1 use and
  **ends the action**. Event: `Healed { target, amount }`, where `amount` is
  the HP actually restored.
- **Healing gives the caster unit EXP** (Nick). The formula is set by 0005
  and implemented in 0601 (`exp_for_heal`).

## Spell stats

Every spell has:

| Field | Meaning |
| ----- | ------- |
| `id`, `name` | |
| `kind` | `Attack` or `Heal` |
| `element` | `Fire`, `Ice`, `None` |
| `might`, `hit`, `crit` | Attack spells only |
| `heal_power` | Heal spells only |
| `min_range`, `max_range` | Tiles (Manhattan distance) |
| `uses` | Uses per battle |
| `effective` | Extra `(tag, multiplier)` pairs, as for weapons (usually empty) |
| `terrain_effect` | Whether a tile cast is allowed. Derived from the element in Chapter 1 (Fire → burn forest, Ice → freeze water), but stored as data |

### Starter spells (*tunable*)

| Spell | Kind | Element | Mt / Heal | Hit | Crit | Rng | Uses | Tile cast |
| ----- | ---- | ------- | --------- | --- | ---- | --- | ---- | --------- |
| Fire | Attack | Fire | 5 | 90 | 0 | 1–2 | 10 | forest → burning |
| Frost | Attack | Ice | 4 | 95 | 0 | 1–2 | 10 | water/sea → ice |
| Force | Attack | None | 6 | 80 | 5 | 1–2 | 8 | — |
| Heal | Heal | None | 10 | — | — | 1 | 8 | — |
| Mend | Heal | None | 20 | — | — | 1 | 4 | — |

Which classes learn which spell, and at what level, is ticket 0005.
Whether each one appears in Chapter 1 depends on the roster (0009 and 0803).
Personal signature spells are added when the cast is written (0701).

## Worked examples

All other formulas come from `stats-and-combat.md`. Nobody wears armour, and
the terrain is plain. Ticket 0304 turns these into tests alongside the other
examples.

### Example M1: Fire into a weak Frost Elemental, which counters with Frost

| | HP | Str | Mag | Dex | Spd | Def | Res | Attack | Affinities |
|-|----|-----|-----|-----|-----|-----|-----|--------|------------|
| Attacker (mage) | 18 | 1 | 9 | 6 | 7 | 2 | 6 | Fire (10/10) | — |
| Defender (Frost Elemental) | 30 | 0 | 8 | 4 | 5 | 8 | 4 | Frost (10/10) | Fire Weak, Ice Absorb |

Distance 2.

- Attacker: Weak → power `9 + 5 × 3 = 24`, damage `24 − 4 = 20`;
  hit `90 + 12 − 10 = 92`; crit `0 + 3 − 1 = 2`. AS `7`, elemental AS `5`,
  diff 2 → 1 strike each.
- Defender: Frost range 1–2 → counters; the mage has no Ice affinity: power
  `8 + 4 = 12`, damage `12 − 6 = 6`; hit `95 + 8 − 14 = 89`;
  crit `0 + 2 − 1 = 1`.

**Forecast:** Attacker `dmg 20 !, hit 92, crit 2` / Defender `dmg 6, hit 89,
crit 1`. Afterwards: mage Fire `9/10`, elemental Frost `9/10`.

### Example M2: Frost into the same elemental (absorb), and Resist

Same units; the mage casts Frost (Mt 4, Hit 95) instead.

- Attacker: power `9 + 4 = 13`, damage `13 − 4 = 9` → **Absorb**: a hit heals
  the elemental by 9 (from 20/30 to 29/30; from 25/30 only 5, to 30/30).
  Crit is forced to 0. Hit `95 + 12 − 10 = 97`.

**Forecast:** Attacker `heals 9, hit 97, crit 0` / Defender as in M1.

Resist variant: if the elemental had Ice `Resist` instead, the damage would be
`9 / 2 = 4`, and a crit `12`.

### Example M3: Heal

A healer with Mag 6 casts Heal (power 10, range 1) on an adjacent ally at
10/25 HP: `amount = min(10 + 6, 25 − 10) = 15`, and the ally ends at 25/25.
On an ally at 20/25, `amount = 5`. An ally at 25/25 can't be targeted.
Heal goes from 8/8 to 7/8 uses.

### Example M4: no uses left

A tier-3 mage (0 weapon slots) with Fire `0/10` and Force `3/8` equipped with
Fire is attacked at distance 2. Fire can't counter, and equipping doesn't
change on its own, so the forecast shows `--` for the mage. After the player
equips Force on their turn, the mage counters with Force (and spends a use).

## Open sub-questions (deferred)

- **What magic is in the world:** deferred to the story bible (0701; Nick,
  Q3). The bible's "rules of magic" must fit the mechanics here.
- **Class spell lists, magic class tree with ≥3 tiers, EXP for heals and tile
  casts:** 0005.
- **Personal signature spells:** named by 0701, with numbers set by 0005 or a
  balance ticket.
- **Elemental enemies in Chapter 1, and a terrain-magic moment on the Chapter
  1 map:** 0009 and 0803.
- **Push spells, more elements, more terrain interactions, spell Combat
  Arts:** not planned. Each would need a design change.
