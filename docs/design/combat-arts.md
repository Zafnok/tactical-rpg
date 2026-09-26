# Combat Arts (and what class actives cost)

Decided: 2026-09-25
Source: ticket 0014

## Nick's words

> **Q1. Who gets which arts?** "Mix: rank + weapons (Recommended)" (reaching a
> weapon rank teaches that weapon kind's arts, usable with any weapon of that
> kind; a few special weapons carry their own art).
>
> **Q2. Limits besides durability?** "Durability only" (the offered option:
> no extra restrictions, the art boosts the whole combat).
>
> **Q3. Arts vs class actives?** "I think class actives and weapon arts should
> both draw durability but have no overlap in their function. We'll have to
> see how that can hold but they should do different things flavor wise"

Follow-ups:

> **Actives that don't swing a weapon** (Brace, War Cry, Sanctuary, Shove,
> spell actives): "Weapon dur + spell uses" (weapon actives cost durability,
> spell actives cost spell uses, non-attack actives cost the equipped weapon's
> durability).
>
> **Arts when attacked:** "same as class actives, the effect might hold for
> just your turn or for both your and enemy turn. Nothing to pick on enemy turn
> but not a counter either"
>
> **Where the line between arts and actives is:** "I don't think there's
> necessarily a line just something lore flavored... like take Dark Souls MLGS
> it uses magic damage so a unique weapon like that might have all damage hit
> against Res instead of Prt/Def. But there might also be a Spellsword class
> that can buff itself to make for that turn its weapon do the same effect.
> But these overlaps should make sense and be rare. Basically, both sides
> should be flavorful."
>
> **Starter list:** "Approve as is (Recommended)".
>
> **Ranks of the starter arts:** "E and D (Recommended)".
>
> **Do enemies use arts and actives?** "Bosses only".

After the PR was opened, Nick added:

> **Weapon skill gain:** "I think skill gain should be a formula of base +
> amount of damage. So we can say sure base 2/4 skill gain but if you do a
> lot of dmg you get extra and if you do only a little you get just a tiny
> extra amount."
>
> **Spells:** "per battle actives should only exist on spells otherwise yes
> cost durability" … "spells are like Fire or Blizzard and each of those will
> have some set amount per battle maybe it's 10 per battle for Fire and 8 per
> battle for Blizzard and these are the one exception to using weapon
> durability is that these do not use durability whatsoever they simply tap
> the spell counter."

So:

- **Combat Arts** are weapon techniques. A unit learns a weapon kind's arts by
  reaching **weapon ranks** in that kind (E, D, then C, B, A, S later), and a
  few **special weapons** carry their own art.
- Arts cost the weapon's **durability**, and so do **class actives** now
  (this changes `progression.md`, where actives had uses per battle). Spell
  actives cost **spell uses** instead.
- **Durability is the only limit.** An art boosts **every strike** of the
  combat. It's picked only when attacking on your own turn. Some arts leave an
  effect that lasts until your next phase, so it also helps on the enemy's
  turn, but nothing is ever picked then.
- Arts and actives have **no fixed dividing line**. Each should be flavourful
  for its weapon or class. Overlaps between them should be **rare and make
  sense** (Nick's example: a unique blade whose art hits Res, and a Spellsword
  class whose active does the same for a turn).
- **Only bosses** among enemies use arts and actives.
- **Weapon EXP** is a base (2, or 4 with an art) **plus a bonus from the
  damage dealt** (Nick).
- **Spells are the only per-battle resource.** Casting never touches
  durability; spells and spell actives only spend the spell's uses (Nick).

Everything marked *tunable* is a starting value Claude chose; balance tickets
may change it without asking. Items marked *Claude's starting rule* fill a gap
Nick's answers didn't cover; Nick may veto them. Unmarked rules are Nick's.
All numbers are FE-sized and rescale with ticket 0013.

## Learning arts

- **Rank arts.** Each weapon kind has a list of arts, each with a minimum
  rank. A unit **knows** a rank art once its rank in that kind is at least the
  art's rank. Ranks never drop, so a known art is never lost.
- A unit can **use** a rank art only while its **current class** can wield that
  kind and it attacks with a weapon of that kind. After a reclass to a class
  without that kind, the art is known but unusable (same as the ranks
  themselves, `progression.md`).
- **Weapon arts.** A weapon's data may list its own arts (`arts: [ArtId]`).
  Anyone who can wield that weapon can use its art while attacking with it,
  whatever their rank (*Claude's starting rule*: the rank to wield the weapon
  already gates it).
- Chapter 1 has **no special weapons** (no loot in Chapter 1, `chapter-1.md`).
  The first ones are designed with later chapters.

## Using an art (exact rules)

1. **When:** only when the unit **attacks on its own phase**, as an option of
   the `Attack` action (like combat actives, `progression.md`). Never picked
   on counters or during another side's phase (Nick).
2. **One per attack** (*Claude's starting rule*): an attack uses at most one
   art **or** one combat active, not both.
3. **Can use** if: the art is usable (above), the attacking weapon is **not
   broken**, and `durability_left ≥ cost`.
4. **Cost** is paid **once per combat**, when the attack is committed, from
   the attacking weapon. It is paid even if every strike misses
   (*Claude's starting rule*, Three Houses).
5. If paying brings the weapon to 0, the combat still uses the **unbroken**
   weapon. The weapon breaks (`ItemBroke`) after the combat
   (*Claude's starting rule*).
6. **Durability only** (Nick): the art's effect applies to **all** of the
   attacker's strikes this combat, including extra strikes from attack speed.
   The defender still counters normally (unless the art says otherwise), and
   its counters are normal attacks.
7. **Lasting effects** (Nick: "might hold for just your turn or for both your
   and enemy turn"): an art may apply a **timed effect**. Two durations exist:
   - **Stance** (on the user): from this combat until the **start of the
     user's next phase**. This is the same as a stance rider in
     `progression.md`, so it also applies when the user is attacked on the
     enemy's turn.
   - **Debuff** (on the target): until the **end of the target's next phase**.
     An enemy hit on the player's phase is slowed through its own coming
     phase. The same effect doesn't stack with itself; applying it again
     refreshes it.
8. **Weapon EXP** (Nick: "base 2/4 … if you do a lot of dmg you get
   extra"): a combat using an art **doubles the base** of the weapon-EXP
   formula in `weapons-and-items.md` (**4** if at least one strike hit, **2**
   if every strike missed). The damage bonus `dealt / 5` is added on top and
   isn't doubled (*tunable*).
9. Unit EXP and class points are the normal combat award (`progression.md`).

## Chapter 1 arts (*tunable* numbers; list approved by Nick)

Range is the weapon's own unless the art says otherwise. "This combat" means
all of the attacker's strikes.

| Art | Kind | Rank | Cost | Range | Effect | Weapon EXP base |
| --- | ---- | ---- | ---- | ----- | ------ | --------------- |
| **Flowing Cut** | Sword | E | 2 | weapon | This combat, the sword follow-up bonus is **×3/2** instead of ×6/5 (strikes 2..N: `damage * 3 / 2`). | ×2 |
| **Guard Break** | Sword | D | 4 | weapon | This combat: hit **+10**, and the defender **can't counter**. | ×2 |
| **Unhorse** | Spear | E | 2 | weapon | This combat: hit **+10**, and the spear's Mounted effectiveness is **×3** instead of ×2. | ×2 |
| **Line Pierce** | Spear | D | 4 | weapon | After the combat, if the attacker is still standing, it makes **one strike** at the unit **directly behind** the target (see below). | ×2 |
| **Crushing Swing** | Axe | E | 2 | weapon | This combat: hit **+20**, and the axe's minimum damage is **8** instead of 5. | ×2 |
| **Armor Cleave** | Axe | D | 4 | weapon | This combat, the axe is effective against **Armored ×2**. | ×2 |
| **Close Shot** | Bow | E | 2 | **1–2** | This attack, the bow's minimum range is 1 (it can shoot an adjacent enemy). Hit **−10**. | ×2 |
| **Pinning Shot** | Bow | D | 3 | weapon | On the first strike that hits: the target gets **Mov −3** (not below 0), a debuff until the end of its next phase. | ×2 |
| **Pressure Point** | Gauntlet | E | 2 | weapon | On the first strike that hits: the target gets **Spd −3** (not below 0), a debuff until the end of its next phase. | ×2 |
| **Sidestep** | Gauntlet | D | 3 | weapon | Stance: **avoid +20**, from this combat until the start of the user's next phase. | ×2 |

Details:

- **Flowing Cut** changes only the follow-up multiplier. The order of
  `weapons-and-items.md` stays: base damage → axe minimum → sword follow-up
  (now ×3/2) → crit ×3. With one strike it does nothing extra, and the
  forecast shows that.
- **Guard Break:** the defender's side of the forecast shows `—  no counter`.
- **Unhorse:** effectiveness still uses the largest matching multiplier
  (`weapons-and-items.md`), so against a unit that is Mounted it's ×3.
  Against anything else only the hit bonus applies.
- **Armor Cleave** adds `(Armored, 2)` to the weapon's effective list for
  this combat. It uses the largest-multiplier rule too.
- **Line Pierce, exact:**
  - "Directly behind" is the tile `target + (target − attacker)`. It only
    exists when attacker and target are in a straight line (always true at
    distance 1, which is every spear today). At a diagonal distance-2 attack,
    there is no pierce.
  - The pierce happens **even if the main target died**, as long as the
    attacker is still alive and the tile holds a unit **hostile** to the
    attacker.
  - The pierce is one strike using the normal hit/damage/crit formulas
    against that unit (its own Def, avoid and terrain; the attacker's weapon
    and effectiveness). It gets **no counter** and no extra strikes. It uses
    the 2RN hit roll and the crit roll in strike order, after the main
    combat's strikes.
  - For unit EXP and class points the pierce counts as a **second combat**
    against that unit (its own award, `progression.md`). Weapon EXP is one
    award for the whole combat: the pierce's damage adds to `dealt`.
- **Close Shot** can be used at distance 1 or 2. At distance 2 it just costs
  durability and hit, and the forecast shows that honestly.
- **Pinning Shot / Pressure Point** apply once per combat, on the first
  strike that hits. Mov and Spd changes feed movement ranges, the danger zone
  and attack speed at once (a slowed enemy may now be doubled by the next
  attacker).
- **Sidestep's** avoid is added to `avoid_B` whenever the user is the one
  being attacked, and to its avoid during the rest of this combat.

### Later ranks

Arts for ranks **C, B, A and S**, and the first **special weapons** with
their own arts, come in a later decision (ticket 0018). Until then, reaching
rank C teaches nothing new. Nick's example for a special weapon: a unique
blade (like Dark Souls' Moonlight Greatsword) whose art makes that combat
**Magical** (hitting Res instead of Def).

## Class actives now cost durability

Nick: "class actives and weapon arts should both draw durability". This
replaces "uses per battle" for actives in `progression.md`. Nick chose
"Weapon dur + spell uses":

| Kind of active | Examples | Pays with |
| -------------- | -------- | --------- |
| **Combat active** (with a weapon attack) | Keen Edge, Flurry, Heavy Blow, Long Shot | The attacking weapon's durability |
| **Non-attack active** (an action of its own) | Brace, Fortify, War Cry, Sanctuary, Shove | The **equipped** weapon's durability |
| **Spell active** (with a spell) | Overcast, Siphon | **1 extra use** of the spell being cast |

- The rules above for arts apply to actives too: they can't be paid with a
  broken weapon, need `durability_left ≥ cost`, cost is paid once when
  committed, and a weapon that hits 0 breaks after the action.
- A non-attack active needs an **equipped**, unbroken weapon with enough
  durability, of any kind. A unit with nothing equipped can't use it.
- A spell active needs the spell to have `uses_left ≥ 2` (the cast plus the
  extra use). It never costs durability.
- Classes with **0 weapon slots** (tier-3 magic classes) can only have spell
  actives. That is a constraint for ticket 1001, which designs tier-3 skills.
- **Spells are the only thing with uses per battle** (Nick). Spells (Fire,
  Frost, Heal…) never use durability at all; they only spend their own uses,
  which refill every battle (`magic.md`). Spell actives follow that: they
  spend spell uses, never durability. Every other active costs durability.
- Actives don't double the weapon-EXP base (*Claude's starting rule*; only
  arts do). The damage bonus applies as in any combat.

### Costs (*tunable*: 2/battle actives cost 3, 1/battle actives cost 5)

| Active | Class | Was | Now |
| ------ | ----- | --- | --- |
| Keen Edge | Swordsman | 2/battle | 3 dur |
| Flurry | Brawler | 1/battle | 5 dur |
| Heavy Blow | Raider | 2/battle | 3 dur |
| Long Shot | Archer | 2/battle | 3 dur |
| Brace | Guard | 2/battle | 3 dur (equipped) |
| Lance Rush | Rider | 2/battle | 3 dur |
| Swoop | Flier, Sky Lancer | 2/battle | 3 dur |
| Overcast | Mage, Sorcerer | 1/battle | +1 spell use |
| Sanctuary | Cleric | 1/battle | 5 dur (equipped) |
| Blade Flurry | Duelist | 1/battle | 5 dur |
| Deadly Blow | Shadowblade | 1/battle | 5 dur |
| Hundred Fists | Striker | 1/battle | 5 dur |
| Shove | Grappler | 2/battle | 3 dur (equipped) |
| Rampage | Berserker | 1/battle | 5 dur |
| War Cry | Vanguard | 1/battle | 5 dur (equipped) |
| Long Shot 2 | Marksman | 2/battle | 3 dur |
| Volley | Outrider | 1/battle | 5 dur |
| Fortify | Bulwark | 1/battle | 5 dur (equipped) |
| Trample | Iron Rider | 2/battle | 3 dur |
| Piercing Lance | Lancer | 2/battle | 3 dur |
| Dive | Sky Warden | 1/battle | 5 dur |
| Siphon | Mystic | 1/battle | +1 spell use |
| Sanctuary 2 | Priest | 1/battle | 5 dur (equipped) |

The Chapter 1 budget: iron weapons have 20 durability and Chapter 1 has no
blacksmith, so each weapon's 20 is shared by its arts and actives for the
whole battle.

### Overlap check (Nick: overlaps rare and sensible)

The starter arts were picked so they don't repeat the tier 1–2 actives'
functions. The closest pairs, all judged in the playtest:

- **Crushing Swing** (axe, hit +20) and **Keen Edge** (Swordsman, hit +30,
  crit +10): both raise accuracy, but Crushing Swing fixes the axe's
  weakness and Keen Edge is a swordsman's precision.
- **Sidestep** (gauntlet stance, avoid) and **Brace / Fortify** (Guard
  stances, Def/Res): both are stances, but they defend in different ways.
- **Close Shot** (bow min range 1) and **Long Shot** (bow max range +1):
  opposite ends of the bow's range.

## Enemies: bosses only

- Only units marked **boss** in chapter data use arts and actives. They follow
  the same rules and pay with their own weapon's durability (Nick: "Bosses
  only").
- Ordinary enemies and green (Other-phase) units never use arts or actives.
  Their passives still apply (at class level 1, generic units have none).
- When a boss attacks with an art or active, the enemy-phase forecast and
  playback show its name.
- How the boss AI chooses: ticket 0503.

## Forecast display

- In the attack flow, after choosing a target, the player can pick an art or
  combat active from a list. Each row shows name, cost and source
  (`E`/`D` rank, `weapon` art, `active`). Unaffordable or broken-weapon
  entries are shown dimmed with the reason.
- The forecast then shows the numbers **with** the art applied, plus a line
  with the art's name and the durability change, e.g. `Guard Break (20 → 16)`.
- Effects that aren't numbers are spelled out on the forecast: `no counter`,
  `pierces`, `pins: Mov −3`, `slows: Spd −3`, `stance: avoid +20`.
- Mockup (approved with the list):

  ```
   ┌─ Swordsman ── Iron Sword 20/20 ──────┐
   │ ▸ Attack                              │
   │   Flowing Cut     −2 dur   E          │
   │   Guard Break     −4 dur   D          │
   │   Keen Edge       −3 dur   active     │
   ├───────────────────────────────────────┤
   │  Guard Break          (20 → 16)       │
   │  HP   22              HP   20         │
   │  Dmg  9 (then 10) ×2  Dmg  —  no counter
   │  Hit  100             Hit  —          │
   │  Crit 3               Crit —          │
   └───────────────────────────────────────┘
  ```

## Worked example (for 0312's tests)

Example W1 from `weapons-and-items.md` (Swordsman, Iron Sword 20/20, rank D,
vs Brigand), with each sword art:

- **Normal:** `dmg 9 (then 10) ×2, hit 94, crit 3` / Brigand `dmg 12, hit 63`.
- **Flowing Cut** (20 → 18): `dmg 9 (then 13) ×2, hit 94, crit 3`
  (`9 × 3 / 2 = 13`) / Brigand unchanged.
- **Guard Break** (20 → 16): `dmg 9 (then 10) ×2, hit 100, crit 3`
  (`94 + 10`, clamped to 100) / Brigand `—  no counter`.
- Weapon EXP if both strikes hit (base 4 with an art, plus `dealt / 5`):
  normal attack `2 + 19/5 = 5`; Flowing Cut `4 + 20/5 = 8` (9 + 13 = 22,
  but the Brigand only had 20 HP, so `dealt` is 20 and it falls); Guard Break
  `4 + 19/5 = 7`.

## Open sub-questions (deferred)

- **Arts for ranks C–S, and special weapons with their own arts:** ticket
  0018.
- **Tier-3 actives** must be spell actives for 0-weapon classes: ticket 1001.
- **Whether any tier 1–2 active should be reflavoured** to overlap arts less:
  judged in the Chapter 1 playtest (0804).
- **Number scale:** all numbers rescale with ticket 0013.
