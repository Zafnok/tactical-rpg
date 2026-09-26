# Progression: level ups, classes and the class tree

Decided: 2026-09-25
Source: ticket 0005

## Nick's words

> **Q1. How do level ups work?** "yea I think it should be class driven to
> decide the % random. We can have a safety net that changes based on tier of
> class. Like starter class can be blessed 2, intermediate blessed 3, adavnced
> blessed 4 something like that."
>
> **Q2. How do class changes work?** "branching + reclass, but the character
> level shouldn't reset to 1. Open certification seems like a bit of a
> mistake"
>
> **Q3. Skills?** "I think each class can have a number of passive skills but
> maybe only upon unlocking and then mastering learns a new active. So like
> you want a big power jump on unlocking and also a reward on mastery."

Follow-ups:

> **Growths (class only, or personal flavour too?)** "Class only + 1 personal
> stat" (class growths, plus one personal "talent" stat at +20%).
>
> **Levels without a reset:** "Level + class level" (a character level that
> never resets, plus a separate class level per class that decides promotions
> and mastery).
>
> **Mastery, and what you keep:** "class points, keep actives and passives
> (passives may be superseded later... like a white mage that learns healing
> magic lv1 which gives +2 to any healing magic might turn into an oracle that
> learns healing magic lv2 that gives +3 or +4 and thus replaces lv1). But in
> the interim you will feel weaker as an oracle if you do not keep the
> passives which is counterintuitive so I think keeping the pasisves is OK."
>
> **Class tree:** "so eventually we will have many more tiers. But for ch1 /
> playtest we don't need to talk about it right now. Just know I'm thinking
> we'll have 6 or maybe even 10 tiers of classes. Martial classes should not
> be constrained. They also have the same number of tiers as magic.
> Myrmidon is too obviously stolen from FE. Idk about Cavalier or Sky Rider
> either are those also stolen? Let's be careful to be a bit more generic for
> now and can decide on exact flavor later. Brawler is fine for example.
> Berserker is fine for example. Falcon and Wyvern Knight are pushing it into
> stolen territory."
>
> **Safety net per tier:** "the minimum gains per level rise by 1 per tier --
> not necessarily. I just used that as an example. We might have blessed 2 to
> start and wait til tier 3 for blessed 3, and til tier 6 for blessed 4 etc.
> This can be decided a bit later on but just keep it as a design note it
> will go up as tier goes up"
>
> **Approval:** "I think the revised class tree is ok and we can flavor tune
> it later when we're closer to shipping"
>
> **Going back to an unlocked class / reclass:** "promotion is one way. The
> rare full class change costs some rare resource and is also not freely
> swappable. But stat growth from completing many classes can accumulate. And
> if you swap from idk a sword fighting class to a mage and then max the mage
> then wanna go back to sword then the progress on the sword is also saved for
> if you wanna return. But it's not free you have to spend the seal of
> whatever item each time."
>
> **Where the Reclass Seal can go:** "C" (any class whose requirements you
> meet).
>
> **Promotion boost:** "A" (a big boost from the gap between the two classes'
> base stats, FE GBA style).
>
> **Mastery pace:** "C" (it rises with tier: about 6 battles for tier 1,
> 10 for tier 2, 15 for tier 3…).
>
> **Promotion item:** "B" (a separate seal per tier, rarer at higher tiers).
>
> **Generic enemy stats:** "A" (fixed average stats, not random level ups).
>
> **Actives on counters:** "A, but in fortunes weave at least there are
> skills that give bonuses when enemies attack, but you pre-select them. For
> example I think Guarding Strike gives a bonus for both rounds of combat.
> Not a counter or something you select during enemy turn but it lasts for
> your turn and theirs."
>
> **More than +1 per stat per level:** "B but number scale can change this"
> (growths above 100% can give +2).
>
> **Who gains EXP:** "A and any xp earned by green units will be spread
> across the participating battle army at battle end"
>
> **Level pace:** "B and level cap can be quite high TBD. If we go for dragon
> ball numbers maybe even fast 2 levels is not accurate maybe your level
> would be in the millions as well. But we can say level can go faster than
> avg."
>
> **Skills (revised):** "specifics I don't care about for right now but I
> would somewhat like to change this. I'm thinking the active can be
> unlocked at the start and passive at the end. However you cannot bring the
> active with you when you reclass unless you master the class. Exact skills
> we can fine tune after playtesting."
>
> **Class stat numbers:** "B numbers are meaningless without playtesting."
> (Claude's numbers are starting values, judged in the Chapter 1
> playtest.)

So:

- **Level ups:** each stat rolls against its growth %. The **class** sets the
  growth %s, and each character gets **+20%** in one personal *talent* stat.
- **Safety net:** a level up has a minimum number of stat gains, and that
  minimum goes **up with class tier** (Nick's "blessed N").
- **Two levels:** a **character level** that never resets, and a **class
  level** for each class, raised by **class points** earned while fighting in
  that class.
- **Promotion and reclass:** you promote by branching, FE-style. A rarer
  reclass also exists. Open certification (Three Houses) is out.
- **Skills** (Nick revised Q3 in the final round):
  - **Unlocking** a class gives its **active** skill.
  - **Mastering** a class (its top class level) teaches its **passive**
    skills.
  - Passives are **kept** after a class change. A higher rank of the same
    passive replaces the lower one.
  - An active **stays behind** when the unit leaves the class, unless the
    unit mastered that class.
- **Level pace:** faster than FE, about **2 character levels per battle**.
  The level cap is **high and not decided yet**; it may be huge if the
  number scale goes "Dragon Ball" (ticket 0013).
- **Tiers:** every line, martial and magic alike, has the same number of
  tiers. Chapter 1 needs tiers 1–3. Nick plans 6–10 tiers eventually, so every
  rule here must work for any number of tiers.
- **Names:** class names are generic placeholders. Flavour names are decided
  closer to shipping, and must not copy Fire Emblem's.

Numbers marked *tunable* are starting values Claude chose, which balance
tickets may change without asking. Nick: numbers "are meaningless without
playtesting", so he judges them in the Chapter 1 playtest (0804). Rules marked
*Claude's starting rule* fill a gap in Nick's answers, and Nick may veto them.
Unmarked rules are Nick's.
All stat numbers are FE-sized, like `stats-and-combat.md`, and get rescaled
with the rest in ticket 0013.

## Character level and unit EXP

- Every unit has a **character level**, starting at 1, which **never resets**
  (Nick). It is the level shown as `Lv` and the one used in EXP maths.
- **Level cap:** high and **not decided yet** (Nick). It may be very large
  if the number scale goes "Dragon Ball" (0013). It is a data value; the
  Chapter 1 build uses **99** as a placeholder (*tunable*). At the cap, EXP
  is shown as `--` and no more unit EXP is gained. Class points are still
  earned.
- **Pace** (Nick): levels come **faster than FE**. The awards below are
  sized for about **2 levels per battle** for an active unit (*tunable*).
- **100 EXP = 1 level.** EXP carries over past 100. A single award is at most
  100, so it gives at most one level.
- **Only Player-faction units** gain EXP and class points (Nick, FE rule).
  Enemies and allies keep their data levels.
- **Allied (green) units' EXP is shared out** (Nick). An Ally-faction unit
  earns EXP by the same formulas, but it goes into a battle **EXP pool**
  instead of to that unit. At the end of the battle (on a win), the pool is
  split evenly among the player units that took part: deployed and alive
  at the end, and not at the level cap. Each one gets
  `pool / number of those units`, rounded down, and the remainder is lost.
  This can give more than one level up at once. Each level up is rolled
  separately.
- **Level-up stats** use the unit's **current class** (its growths, caps and
  tier) at the moment of the level up.

### EXP formulas (*tunable*: FE GBA's shape, doubled for Nick's faster pace)

`d = target.level - unit.level` (character levels, signed).

| Event | EXP |
| ----- | --- |
| Took part in a combat (attacked or countered), dealt no damage and killed nobody | `2` |
| Dealt damage (≥ 1 HP) and the target survived | `clamp(2 * ((31 + d) / 3), 2, 100)` |
| Killed the target | `clamp(2 * ((31 + d) / 3 + max(0, 20 + 3 * d)) + (target is boss ? 40 : 0), 2, 100)` |
| Healed an ally with a spell | `24` |
| Tile cast (fire/ice on terrain, `magic.md`) | `24` (same as a heal, as `magic.md` asked) |
| Used an active skill that isn't a combat (e.g. a buff) | `20` |

- `/` rounds toward zero, as in `stats-and-combat.md`.
- One combat gives one award, no matter how many strikes it has. A kill
  replaces the damage award; the two are not added.
- A heal or a tile cast is one award per cast.
- Counters give EXP in the same way, so a player unit that counters on the
  enemy's phase earns EXP.
- A combat that uses an active skill gives the normal combat award. The
  skill-use line doesn't also apply.

Examples:

| Case | d | EXP |
| ---- | - | --- |
| Hit, equal levels | 0 | 2 × 10 = 20 |
| Hit, enemy 5 levels higher | +5 | 2 × 12 = 24 |
| Kill, equal levels | 0 | 2 × (10 + 20) = 60 |
| Kill, enemy 5 levels higher | +5 | 2 × (12 + 35) = 94 |
| Kill, enemy 5 levels lower | −5 | 2 × (8 + 5) = 26 |
| Kill, enemy 10 levels lower | −10 | 2 × (7 + 0) = 14 |
| Kill a boss, equal levels | 0 | 60 + 40 = 100 |
| Missed every strike | any | 2 |

## Growths and level ups

### Growth rates

```
growth(stat) = class.growth[stat] + (stat == character.talent ? 20 : 0)
```

- Growth rates exist for HP, Str, Mag, Dex, Spd, Def, Res. **Mov never grows**;
  it comes from the class.
- **Talent** (Nick): each named character has exactly one talent stat (not
  Mov), which gets +20% growth in every class. Generic units have none. The
  story cast (0701) picks each character's talent.
- **Growths above 100% can give more than +1** (Nick). A stat gets
  `growth / 100` points for sure, plus 1 more with a `growth % 100` percent
  chance. For example, 120% gives +1 for sure and +2 with a 20% chance.
  Nick noted that the number-scale decision (ticket 0013) may change this.

### Minimum gains per level up (Nick's "blessed N")

| Class tier | Minimum gains (*tunable*) |
| ---------- | ------------------------- |
| 1 | 2 |
| 2 | 2 |
| 3 | 3 |
| 4 and up | Decided when those tiers are designed (Nick's example: 4 by tier 6) |

**Design note (Nick):** the minimum **goes up as the tier goes up**. A higher
tier never has a lower minimum than the tier below it. The exact steps are
tuned later. Store the table as data, one entry per tier.

### Level-up procedure (exact, for 0601)

`roll()` is the battle's seeded RNG giving a uniform integer in `0..=99`, as
in `stats-and-combat.md`. `roll_below(n)` gives a uniform integer in
`0..n` (n ≥ 1).

1. Stat order is always `HP, Str, Mag, Dex, Spd, Def, Res`.
2. A stat is **eligible** if its value is below its current class cap and its
   `growth > 0`.
3. **Rolls:** for each stat in order, call `r = roll()`. This happens for
   **every** stat, eligible or not, so RNG use is always 7 calls. If the
   stat is eligible it gains `growth / 100 + (r < growth % 100 ? 1 : 0)`
   points, clamped so it doesn't pass its cap. A stat that gains ≥ 1 point
   counts as **one gain** for the safety net.
4. **Safety net:** `need = min(min_gains[class.tier], number of eligible
   stats)`. While `gains < need`:
   - `candidates` = the eligible stats that haven't gained, in stat order.
   - `total = sum of their growths`, then `r = roll_below(total)`.
   - Walk `candidates` adding up growths; the first stat whose running sum
     is `> r` gains +1.
   (Only stats with 0 points so far are candidates, and those have growth
   below 100%.)
   The net therefore favours the stats the class is good at, but stays random.
5. Apply the gains. If HP gained, current HP rises by the same amount.
6. Learn anything new (see *Skills*, *Spells*). Class skills and spells come
   from the class level, not the character level, so a character level up
   teaches nothing by itself.

Example: a tier-1 Swordsman (growths HP 70, Str 40, Mag 10, Dex 55, Spd 60,
Def 25, Res 20) rolls `[85, 72, 50, 91, 77, 30, 40]`. That is 0 gains. The net
needs 2. The first pick is over all 7 stats (total 280). The second is over
the 6 left, and its total leaves out the growth of the stat that just gained.
Result: `Lv 4→5: HP+1 Spd+1` (for example). With the rolls
`[10, 20, 50, 30, 77, 30, 40]`, HP, Str and Dex gain (3 ≥ 2), so the net does
nothing.

### Stat caps

- Each class has a cap per stat (table below). A stat at its cap can't gain.
  Caps are always ≤ the hard ceilings in `stats-and-combat.md`.
- **After a class change into a class with lower caps** (a reclass), stats
  above the new caps are **kept**. They just can't grow while in that class
  (Nick: stat growth from many classes accumulates).
- Gear may go above caps, but never above the hard ceilings
  (`weapons-and-items.md`).

## Class levels, class points and mastery

- Each unit has a **class record** for every class it has unlocked:
  `(class_level, class_points)`.
- **Unlocking** a class means entering it for the first time, by promotion,
  reclass or as the unit's starting class. The record starts at class
  level 1 with 0 CP.
- **Class points (CP)** go to the unit's **current class only**
  (*tunable*):

| Event | CP |
| ----- | -- |
| Took part in a combat (attacked or countered) | 2 |
| … and killed the target | +2 |
| Heal, tile cast, or non-combat active skill | 2 |

- **Class level:** each tier has a `cp_per_class_level` value, stored as
  data with one entry per tier. **It rises with the tier** (Nick: early
  classes are quick, late ones are long-term goals). A higher tier never
  needs fewer CP than the tier below it.

  | Tier | CP per class level (*tunable*) | CP to master | About how many battles |
  | ---- | ------------------------------ | ------------ | ---------------------- |
  | 1 | 10 | 90 | 6 |
  | 2 | 17 | 153 | 10 |
  | 3 | 25 | 225 | 15 |
  | 4 and up | Set when those tiers are designed | | |

  The class level cap is **10** for every tier (*tunable*). Class level `n`
  needs `(n − 1) × cp_per_class_level[tier]` total CP in that class.
- **Mastered** = class level 10. After that, CP for that class stop counting.
- What class levels do:
  - **Class level 1 (unlock):** gain the class's **active skill** (Nick).
  - **Class spells** at their listed class level (see *Spells*).
  - **Class level 10 (mastery):** learn the class's **passive skills** for
    good, **keep its active for good** (Nick), and open up the class's
    promotions.
- Pace: the battle counts above assume about 15 CP per battle for an active
  unit. Chapter 1 is shorter than that, so no unit masters a class in it.

## Promotion (branching)

- A unit can **promote** from its current class to one of the classes in its
  `promotes_to` list (one tier higher) when:
  1. its current class is **mastered** (class level 10), and
  2. it uses the **seal for the target tier** (Nick: a separate seal per
     tier). The placeholder names are *Tier 2 Seal*, *Tier 3 Seal* and so
     on. Higher-tier seals are rarer; that rarity is how pacing is
     controlled. A seal is a consumable in the battle pack, or used from the
     between-battle menu. Prices and drops belong to the shop and chapter
     tickets.
- **The character level doesn't reset** (Nick). EXP stays the same.
- **Promotion bonus** (Nick: FE GBA-style big boost): for each stat,
  `bonus = max(0, new_class.base[stat] − old_class.base[stat])`. The bonus is
  added, then clamped to the hard ceiling (not to the class cap). Current HP
  rises by the HP bonus. Mov becomes the new class's Mov.
- The new class is unlocked: its record starts at class level 1, so its
  active is gained at once, plus any class-level-1 spells.
- **Weapon ranks:** for each weapon kind the new class can use,
  `rank = max(current rank, class start rank)`. Ranks in kinds the new class
  can't use are kept (for later reclasses) but unusable.
- **Weapon slots:** if the new class has fewer slots (e.g. 0 for tier-3 magic
  classes), the extra weapons go to the party stock (`magic.md`).
- Event: `Promoted { unit, from, to, gains }`.

## Reclass

Nick chose branching plus reclass (Q2), and set its rules in the follow-up
round.

- **Promotion is one-way** (Nick). A unit never drops back a tier by
  promoting.
- A **full class change (reclass)** always costs one **Reclass Seal**
  (*placeholder name*; a rare resource, never sold in Chapter 1). It is
  **never free**, including a return to a class the unit already unlocked
  (Nick: "you have to spend the seal of whatever item each time").
- **Where a seal can take a unit** (Nick chose "any class whose requirements
  you meet"):
  - any **tier-1** class (not enemy-only);
  - any class whose **prerequisite** the unit has **mastered** (a class
    that lists it in `promotes_to`);
  - any class the unit has **already unlocked**. Its saved progress comes
    back (Nick: "the progress on the sword is also saved for if you wanna
    return").
- **Class progress is saved** (Nick): every class record keeps its class
  level and CP after the unit leaves the class. Entering a class for the
  first time unlocks it at class level 1, with its active and
  class-level-1 spells.
- **Stats accumulate** (Nick: "stat growth from completing many classes can
  accumulate"):
  - A reclass never lowers stats and gives no promotion bonus.
  - Stats above the new class's caps are kept, but can't grow while in that
    class.
  - From then on, level ups use the new class's growths, caps and tier.
- Skills and spells already learned are kept (Nick; `magic.md`).
- Weapon ranks and slots follow the promotion rules above.
- **Open certification** (Three Houses style, entering any class by passing an
  exam) is **not** in the game (Nick).

## Skills

Nick (revised in the final round): the **active** comes when a class is
**unlocked**, and the **passives** when it is **mastered**. An active stays
behind when you leave an unmastered class.

- **Actives, by class:**
  - A unit can use the active of its **current class** as soon as the class
    is unlocked (class level 1).
  - When it **masters** a class, that class's active becomes **permanent**:
    usable in any class from then on.
  - When a unit leaves a class it has **not** mastered (by reclass), that
    active **stays behind**. It can't be used until the unit returns to the
    class (with a Reclass Seal; its saved progress comes back) or masters it.
  - Promotion always happens from a mastered class, so a promoted unit keeps
    its old class's active and gains the new class's active too.
- **Passives, by class:** learned when the class is **mastered**, and
  **kept** for good after any class change (Nick: keeping them is "OK").
  A passive is always on, with no equip limit.
- **Superseding (Nick):**
  - Skills belong to a **family** and have a **rank**, e.g. `White Magic 1`
    and `White Magic 2`.
  - Learning a higher rank of a family replaces the lower rank. Only the
    highest rank a unit knows is active.
  - Learning a lower rank than one already known does nothing.
- An **active skill** is used on purpose:
  - It costs **weapon durability** (Nick, ticket 0014; this replaces the
    uses per battle first decided here). Combat actives pay with the
    attacking weapon, non-attack actives with the equipped weapon, and spell
    actives cost 1 extra use of the spell instead. Exact costs and rules:
    [`combat-arts.md`](combat-arts.md).
  - **Combat actives** are chosen from the attack menu as an option for that
    attack. **Non-combat actives** are an action of their own and end the
    action.
  - Combat actives are used only **when attacking** on your own turn, never
    chosen on counters (Nick, like Three Houses arts).
  - **Stance riders** (Nick, like Fortune's Weave's "Guarding Strike"): a
    combat active may also give a bonus that **lasts from this attack until
    the start of the unit's next phase**. That covers the enemy's turn, so it
    also helps in the unit's defending combats (counters) then. It's picked
    when attacking, never during the enemy's turn. It uses the timed-effect
    rules below.
  - Actives are separate from Combat Arts (weapon techniques, also paid
    with durability, `combat-arts.md`). An attack uses one art or one combat
    active, not both. Nick: both should be flavourful and overlaps rare.
- **Timed effects**, "until the start of the unit's next phase", start when
  the skill is used. They end at the start of the owner's side's next phase,
  before anyone acts. The same effect doesn't stack with itself; using it
  again refreshes it.
- **The skill list below is placeholder content** (Nick: "exact skills we
  can fine tune after playtesting"). It exists so Chapter 1 has something to
  play with. Skills, numbers and flavour names get tuned after the playtest.

### Skill list: tiers 1–2

"Sw/Sp/Ax/Bw/Gt equipped" means the equipped weapon is of that kind. Bonuses
add to the numbers in the combat formulas (`stats-and-combat.md`,
`weapons-and-items.md`) and show in the forecast.

| Class | Active (unlock) | Passive (mastery) |
| ----- | --------------- | ----------------- |
| Swordsman | **Keen Edge** (3 dur, combat): this combat hit +30, crit +10 | **Sword Focus 1**: Sw equipped → crit +10 |
| Brawler | **Flurry** (5 dur, combat): this combat the attacker gets +1 strike (max 4) | **Light Feet 1**: Gt equipped → attack speed +2 |
| Raider | **Heavy Blow** (3 dur, combat): might +5; the attacker gets 1 strike only | **Axe Focus 1**: Ax equipped → hit +10 |
| Archer | **Long Shot** (3 dur, combat): Bw max range +1 for this attack | **Skirmish**: after attacking with a Bw, may move 1 tile (the post-action move from `turn-structure.md`) |
| Guard | **Brace** (3 dur, action): Def and Res +5 until its next phase | **Steadfast 1**: Def +2 while not in its own phase |
| Rider | **Lance Rush** (3 dur, combat): might +5 | **Charge 1**: damage +2 when it moved ≥ 4 tiles this turn before attacking |
| Flier | **Swoop** (3 dur, combat): after this attack, may move 1 tile | **Sky Dodge 1**: avoid +10 against bows |
| Mage | **Overcast** (+1 spell use, combat, spell only): spell might +5 | **Black Magic 1**: attack spells might +1 |
| Cleric | **Sanctuary** (5 dur, action): heals every adjacent ally by `Mag + 5`; ends the action | **White Magic 1**: heal spells +2 HP (Nick's example) |
| Duelist | **Blade Flurry** (5 dur, combat, Sw): +1 strike (max 4) | **Sword Focus 2**: Sw equipped → crit +20 |
| Shadowblade | **Deadly Blow** (5 dur, combat): crit ×2 for this combat (clamped to 100) | **Evasion 1**: avoid +10 |
| Striker | **Hundred Fists** (5 dur, combat, Gt): +1 strike (max 4) and hit +10 | **Light Feet 2**: Gt equipped → attack speed +4 |
| Grappler | **Shove** (3 dur, action): push an adjacent enemy 1 tile straight away (no damage). The push rules come from `magic.md` (5 damage into `burning`); it fails if the tile is blocked | **Iron Grip**: Gt equipped → Def +3 |
| Berserker | **Rampage** (5 dur, combat): might +8, and its avoid −20 for this combat | **Fury**: crit +15 while HP ≤ 50% |
| Vanguard | **War Cry** (5 dur, action): adjacent allies Str +2 until the start of this unit's next phase | **Axe Focus 2**: Ax equipped → hit +20 |
| Marksman | **Long Shot 2** (3 dur, combat): Bw max range +2 | **Bow Focus**: Bw equipped → hit +10, crit +5 |
| Outrider | **Volley** (5 dur, combat, Bw): +1 strike (max 4) | **Skirmish** (learned again, no effect if already known) + **Charge 1** |
| Bulwark | **Fortify** (5 dur, action): Def and Res +8 until its next phase | **Steadfast 2**: Def +4 while not in its own phase |
| Iron Rider | **Trample** (3 dur, combat): might +4, and the target's terrain Def/avoid is ignored | **Charge 1** + **Steadfast 1** |
| Lancer | **Piercing Lance** (3 dur, combat, Sp): ignore 5 of the target's Def | **Charge 2**: damage +4 after moving ≥ 4 tiles |
| Sky Lancer | **Swoop** | **Sky Dodge 2**: avoid +20 against bows |
| Sky Warden | **Dive** (5 dur, combat): might +6 | **Sky Guard**: Def +3 |
| Sorcerer | **Overcast** | **Black Magic 2**: attack spells might +3 |
| Mystic | **Siphon** (+1 spell use, combat, spell): the caster heals by half the damage dealt (rounded down) | **Black Magic 1** + **White Magic 1** |
| Priest | **Sanctuary 2** (5 dur, action): heals every ally within 2 tiles by `Mag + 5` | **White Magic 2**: heal spells +4 HP (supersedes 1, Nick's example) |

- **Skills learned twice:** a unit that learns an active it already knows
  (e.g. Swoop from both flier lines, Overcast from Mage and Sorcerer) gets
  nothing more. Ranks of the same active (Sanctuary 2, Long Shot 2) supersede
  like passives.
- **"+1 strike" skills:** these add to the strikes worked out from attack
  speed, never above the 4-strike maximum. They're meant to make 3x/4x
  happen more often, but rarely (`stats-and-combat.md`).
- **Tier-3 skills** and the elemental enemies' skills aren't designed yet.
  Chapter 1 can't reach tier 3. That work is follow-up ticket 1001.

## Spells in class lists

`magic.md` says a unit learns a class spell "when it reaches that level in
that class". That level is the **class level** from this doc. Entering a
class at class level 1 teaches its class-level-1 spells at once. Learned
spells are kept after a class change (`magic.md`). Personal signature spells
use the **character level**.

## Class tree (approved; names are placeholders)

```
TIER 1                  TIER 2                         TIER 3
────────────────────────────────────────────────────────────────────────────
Swordsman (Sw) ──────┬─► Duelist        Sw        ──► Blade Dancer   Sw
                     └─► Shadowblade    Sw Gt     ──► Nightblade     Sw Gt Bw

Brawler (Gt) ────────┬─► Striker        Gt        ──► Tempest Fist   Gt
                     └─► Grappler       Gt Ax     ──► Colossus       Gt Ax

Raider (Ax) ─────────┬─► Berserker      Ax        ──► Ravager        Ax Sw
                     └─► Vanguard       Ax Bw     ──► Warchief       Ax Bw Sp

Archer (Bw) ─────────┬─► Marksman       Bw        ──► Deadeye        Bw
                     └─► Outrider [M]   Bw Sw     ──► Windrunner [M] Bw Sw

Guard [A] (Sp) ──────┬─► Bulwark [A]    Sp Ax     ──► Bastion [A]    Sp Ax
                     └─► Iron Rider [A][M] Sp Ax Sw ► Juggernaut [A][M] Sp Ax Sw
                              ▲
Rider [M] (Sw Sp) ───┼────────┘  (shared promotion)
                     └─► Lancer [M]     Sp Sw     ──► High Lancer [M] Sp Sw Ax

Flier [F] (Sp) ──────┬─► Sky Lancer [F] Sp Sw     ──► Storm Lancer [F] Sp Sw
                     └─► Sky Warden [F] Sp Ax     ──► Sky Tyrant [F]  Sp Ax

Mage (Sw · Fire) ────┬─► Sorcerer (Sw) ───────────► Archmage     spells only
                     └─► Mystic (Sw/Gt) ──────────► Arcanist     spells only
                              ▲  (black + white magic)
Cleric (Gt · Heal) ──┼────────┘  (shared promotion)
                     └─► Priest (Gt) ─────────────► Oracle       spells only

Enemy-only: Brigand (Ax) · Fire Elemental · Frost Elemental

[M] Mounted (spears hit ×2)   [F] Flying (bows hit ×3)   [A] Armored (high Def, low Res)
Sw sword · Sp spear · Ax axe · Bw bow · Gt gauntlet
```

- **Martial classes aren't constrained** (Nick): they have the same number of
  tiers as magic, and keep **3 weapon slots at every tier**. Only the magic
  lines' tier-3+ classes have 0 weapon slots (`magic.md`).
- **Shared promotions:** Iron Rider (from Guard or Rider) and Mystic (from
  Mage or Cleric). The promotion bonus is worked out from whichever class the
  unit came from.
- **More tiers later:** Nick expects 6–10 tiers. Tiers above 3 are not
  designed yet. Every rule here takes the tier as a number, and nothing
  assumes 3 is the top.
- **Flavour names** are placeholders, to be tuned closer to shipping (Nick).

## Class table

Columns: tier; Mov; movement type (`foot`, `mounted`, `armored`, `flying`);
tags; weapon kinds with *start rank / max rank*; allowed armour weights
(L/M/H); `weapon_slots`; class spells as `(class level, spell)`.
Everything is *tunable*.

A unit's rank in a kind is raised to the class's start rank when it enters
the class (never lowered), and can't grow past the class's max rank while in
that class.

### Tier 1

| Class | Mov | Move type | Tags | Weapons | Armour | Slots | Spells | Promotes to |
| ----- | --- | --------- | ---- | ------- | ------ | ----- | ------ | ----------- |
| Swordsman | 5 | foot | — | Sw D/C | L | 3 | — | Duelist, Shadowblade |
| Brawler | 5 | foot | — | Gt D/C | L | 3 | — | Striker, Grappler |
| Raider | 5 | foot | — | Ax D/C | L M | 3 | — | Berserker, Vanguard |
| Archer | 5 | foot | — | Bw D/C | L M | 3 | — | Marksman, Outrider |
| Guard | 4 | armored | Armored | Sp D/C | M H | 3 | — | Bulwark, Iron Rider |
| Rider | 7 | mounted | Mounted | Sw E/C, Sp D/C | L M | 3 | — | Iron Rider, Lancer |
| Flier | 7 | flying | Flying | Sp D/C | L | 3 | — | Sky Lancer, Sky Warden |
| Mage | 5 | foot | — | Sw E/D | L | 3 | (1, Fire), (5, Frost) | Sorcerer, Mystic |
| Cleric | 5 | foot | — | Gt E/D | L | 3 | (1, Heal) | Mystic, Priest |
| Brigand *(enemy)* | 5 | foot | — | Ax D/C | L | 3 | — | — |
| Fire Elemental *(enemy)* | 4 | foot | — | — | — | 0 | (1, Fire) | — |
| Frost Elemental *(enemy)* | 4 | foot | — | — | — | 0 | (1, Frost) | — |

Elemental affinities are fixed in `magic.md`: Fire Elemental = Fire `Absorb`,
Ice `Weak`; Frost Elemental = Ice `Absorb`, Fire `Weak`. Nobody else has
affinities yet. Enemy-only classes can't be promoted into or reclassed into.

### Tier 2

| Class | Mov | Move type | Tags | Weapons | Armour | Slots | Spells | Promotes to |
| ----- | --- | --------- | ---- | ------- | ------ | ----- | ------ | ----------- |
| Duelist | 6 | foot | — | Sw C/A | L | 3 | — | Blade Dancer |
| Shadowblade | 6 | foot | — | Sw C/A, Gt D/B | L | 3 | — | Nightblade |
| Striker | 6 | foot | — | Gt C/A | L | 3 | — | Tempest Fist |
| Grappler | 5 | foot | — | Gt C/A, Ax D/B | L M | 3 | — | Colossus |
| Berserker | 5 | foot | — | Ax C/A | L M | 3 | — | Ravager |
| Vanguard | 5 | foot | — | Ax C/A, Bw D/B | M H | 3 | — | Warchief |
| Marksman | 6 | foot | — | Bw C/A | L M | 3 | — | Deadeye |
| Outrider | 7 | mounted | Mounted | Bw C/A, Sw D/B | L M | 3 | — | Windrunner |
| Bulwark | 4 | armored | Armored | Sp C/A, Ax D/B | M H | 3 | — | Bastion |
| Iron Rider | 6 | mounted | Armored, Mounted | Sp C/A, Ax D/B, Sw D/B | M H | 3 | — | Juggernaut |
| Lancer | 8 | mounted | Mounted | Sp C/A, Sw C/B | L M | 3 | — | High Lancer |
| Sky Lancer | 8 | flying | Flying | Sp C/A, Sw D/B | L | 3 | — | Storm Lancer |
| Sky Warden | 7 | flying | Flying | Sp C/A, Ax D/B | L M | 3 | — | Sky Tyrant |
| Sorcerer | 5 | foot | — | Sw D/C | L | 3 | (1, Force) | Archmage |
| Mystic | 5 | foot | — | Sw D/C, Gt D/C | L | 3 | (1, Fire), (1, Heal), (5, Mend) | Arcanist |
| Priest | 5 | foot | — | Gt D/C | L | 3 | (1, Mend) | Oracle |

### Tier 3

| Class | Mov | Move type | Tags | Weapons | Armour | Slots | Spells |
| ----- | --- | --------- | ---- | ------- | ------ | ----- | ------ |
| Blade Dancer | 6 | foot | — | Sw A/S | L | 3 | — |
| Nightblade | 6 | foot | — | Sw A/S, Gt B/A, Bw D/B | L | 3 | — |
| Tempest Fist | 6 | foot | — | Gt A/S | L | 3 | — |
| Colossus | 5 | foot | — | Gt A/S, Ax B/A | L M H | 3 | — |
| Ravager | 5 | foot | — | Ax A/S, Sw C/B | L M | 3 | — |
| Warchief | 5 | foot | — | Ax A/S, Bw B/A, Sp C/B | M H | 3 | — |
| Deadeye | 6 | foot | — | Bw A/S | L M | 3 | — |
| Windrunner | 8 | mounted | Mounted | Bw A/S, Sw B/A | L M | 3 | — |
| Bastion | 4 | armored | Armored | Sp A/S, Ax B/A | M H | 3 | — |
| Juggernaut | 6 | mounted | Armored, Mounted | Sp A/S, Ax B/A, Sw B/A | M H | 3 | — |
| High Lancer | 8 | mounted | Mounted | Sp A/S, Sw B/A, Ax C/B | L M | 3 | — |
| Storm Lancer | 8 | flying | Flying | Sp A/S, Sw B/A | L | 3 | — |
| Sky Tyrant | 7 | flying | Flying | Sp A/S, Ax B/A | L M | 3 | — |
| Archmage | 5 | foot | — | — | L | **0** | (1, Fire), (1, Frost), (1, Force) |
| Arcanist | 5 | foot | — | — | L | **0** | (1, Fire), (1, Frost), (1, Heal), (1, Mend) |
| Oracle | 5 | foot | — | — | L | **0** | (1, Heal), (1, Mend) |

Tier-3 spell lists only use the five starter spells for now. New spells for
higher tiers are added when tiers above 3 are designed.

### Base stats, caps and growths

`base` is used for **generic units** (see below) and for the **promotion
bonus**. It isn't a named character's stats: those come from the character's
data (0701 / 0803). Order: `HP Str Mag Dex Spd Def Res`.

| Class | Base | Caps | Growths % |
| ----- | ---- | ---- | --------- |
| Swordsman | 18 5 0 7 8 3 1 | 40 20 10 24 25 18 15 | 70 40 10 55 60 25 20 |
| Brawler | 18 5 0 6 9 3 1 | 40 20 10 22 26 16 14 | 70 40 5 50 65 20 20 |
| Raider | 22 7 0 3 4 4 0 | 45 24 8 18 18 20 12 | 85 55 5 35 30 30 10 |
| Archer | 17 5 0 7 5 3 1 | 40 20 10 24 20 18 15 | 65 45 10 60 45 25 20 |
| Guard | 20 7 0 4 2 9 0 | 45 22 8 18 14 26 10 | 80 45 5 35 20 55 10 |
| Rider | 20 6 0 5 5 5 1 | 42 21 10 20 20 20 14 | 75 45 5 45 45 35 15 |
| Flier | 16 4 1 6 7 3 5 | 38 18 14 22 24 16 22 | 60 35 20 50 55 20 45 |
| Mage | 16 1 6 5 5 1 5 | 36 12 24 20 20 12 22 | 55 15 60 45 45 15 45 |
| Cleric | 16 1 5 4 5 1 7 | 36 12 22 18 20 12 24 | 55 15 50 40 45 15 55 |
| Brigand | 20 6 0 2 4 3 0 | 45 24 8 16 18 18 10 | 80 50 0 30 30 25 5 |
| Fire Elemental | 30 0 8 4 5 8 4 | 60 10 30 24 20 30 30 | 80 0 50 35 30 40 40 |
| Frost Elemental | 30 0 8 4 5 8 4 | 60 10 30 24 20 30 30 | 80 0 50 35 30 40 40 |
| Duelist | 24 8 0 11 13 5 3 | 50 26 12 32 34 22 20 | 70 45 10 60 65 25 25 |
| Shadowblade | 22 7 0 12 12 4 3 | 48 24 12 34 32 20 20 | 65 40 10 65 60 20 25 |
| Striker | 24 8 0 9 14 5 3 | 50 26 12 30 36 20 18 | 70 45 5 55 70 20 20 |
| Grappler | 28 10 0 7 10 8 2 | 55 30 10 26 28 28 16 | 85 55 5 40 45 40 15 |
| Berserker | 30 12 0 6 8 6 1 | 60 32 8 24 26 24 14 | 90 65 5 40 40 30 10 |
| Vanguard | 28 10 0 7 7 8 2 | 55 29 10 26 24 28 16 | 85 55 5 45 35 40 15 |
| Marksman | 22 8 0 12 9 5 3 | 48 26 12 34 28 22 20 | 65 50 10 65 50 25 25 |
| Outrider | 23 7 0 10 10 6 3 | 48 25 12 30 30 22 20 | 70 45 10 55 55 30 20 |
| Bulwark | 28 11 0 6 4 14 2 | 58 30 10 24 20 36 14 | 85 50 5 40 25 60 10 |
| Iron Rider | 27 10 0 6 6 11 2 | 56 30 10 24 22 32 14 | 85 50 5 40 35 50 10 |
| Lancer | 25 9 0 8 9 8 3 | 52 28 12 28 28 26 18 | 80 50 5 50 50 35 20 |
| Sky Lancer | 21 6 2 9 11 5 8 | 46 24 18 30 32 20 30 | 65 40 20 55 60 20 50 |
| Sky Warden | 26 10 0 7 8 9 3 | 54 30 12 26 26 30 18 | 80 55 5 45 45 45 15 |
| Sorcerer | 20 2 10 7 7 2 8 | 42 14 32 26 26 14 28 | 55 15 70 50 50 15 50 |
| Mystic | 21 2 9 7 7 2 9 | 44 14 30 26 26 14 30 | 60 15 60 45 50 15 55 |
| Priest | 20 2 8 6 7 2 11 | 42 14 28 24 26 14 32 | 60 15 55 45 45 15 65 |
| Blade Dancer | 30 11 0 15 17 7 5 | 58 32 14 40 40 26 24 | 70 45 10 60 65 25 25 |
| Nightblade | 28 10 0 16 16 6 5 | 56 30 14 40 38 24 24 | 65 40 10 65 60 20 25 |
| Tempest Fist | 30 11 0 12 18 7 5 | 58 32 14 36 40 24 22 | 70 45 5 55 70 20 20 |
| Colossus | 36 14 0 9 12 11 3 | 66 38 12 30 32 34 18 | 85 55 5 40 45 40 15 |
| Ravager | 38 16 0 8 11 8 2 | 70 40 10 30 32 28 16 | 90 65 5 40 40 30 10 |
| Warchief | 36 13 0 9 9 11 3 | 66 36 12 32 28 34 18 | 85 55 5 45 35 40 15 |
| Deadeye | 28 11 0 16 12 7 5 | 56 32 14 40 34 26 24 | 65 50 10 65 50 25 25 |
| Windrunner | 29 10 0 13 14 8 5 | 56 30 14 36 36 26 24 | 70 45 10 55 55 30 20 |
| Bastion | 36 14 0 8 5 19 3 | 70 36 12 30 24 45 18 | 85 50 5 40 25 60 10 |
| Juggernaut | 34 13 0 8 8 15 3 | 66 36 12 30 28 40 18 | 85 50 5 40 35 50 10 |
| High Lancer | 32 12 0 10 12 10 4 | 60 34 14 34 34 32 22 | 80 50 5 50 50 35 20 |
| Storm Lancer | 27 9 3 12 15 7 11 | 54 30 22 36 40 24 38 | 65 40 20 55 60 20 50 |
| Sky Tyrant | 33 13 0 9 11 12 4 | 64 36 14 32 32 36 22 | 80 55 5 45 45 45 15 |
| Archmage | 25 3 14 9 9 3 11 | 50 16 40 32 32 16 34 | 55 15 70 50 50 15 50 |
| Arcanist | 26 3 13 9 9 3 12 | 52 16 38 32 32 16 36 | 60 15 60 45 50 15 55 |
| Oracle | 25 3 12 8 9 3 14 | 50 16 36 30 32 16 40 | 60 15 55 45 45 15 65 |

Design intent behind the numbers:
- **Armoured classes** (Guard, Bulwark, Iron Rider, Bastion, Juggernaut) have
  the highest Def and the lowest Res. Nick: "encouraged to use magic users on
  those enemies".
- **Brawler → Striker → Tempest Fist** has the highest Spd caps (up to 40,
  the specialist guideline in `stats-and-combat.md`). With the lightest
  weapons and Light Feet, this is the line most likely to strike 3x/4x
  (Nick's "punching class").
- **Fliers** have high Res, low Def and high Spd. Bows (×3) are their
  weakness. **Mounted** classes have high Mov, and spears (×2) are their
  weakness.

### Generic units (enemies and generic allies)

A generic unit of class `C` at character level `L` has
`stat = min(cap, base + (growth × (L − 1)) / 100)` for each stat, with no
talent and no randomness (Nick: fixed average stats, so a map plays the
same way every time and you can plan exactly). Its weapon ranks are the class's start ranks
unless the chapter data says otherwise. Its class level is 1, with the
class's active (it hasn't mastered anything), but **only bosses use actives
and Combat Arts** (Nick, `combat-arts.md`). Chapter data may give a
generic enemy extra skills or spells.

### Named characters

Each named character's data (0701 / 0803) has: starting class, character
level, base stats (≤ that class's caps), **talent** stat, starting weapon
ranks, class records (usually just the starting class at class level 1), and
0–2 personal spells (`magic.md`).

## Open sub-questions (deferred)

- **Tiers 4 and up** (Nick expects 6–10 tiers): the classes, the minimum-gain
  steps, the level cap and new spells. Not needed for Chapter 1.
- **Tier-3 skills** and elemental enemy skills: ticket 1001.
- **Flavour names** for every class, skill and seal: closer to shipping
  (Nick). They must not copy Fire Emblem names.
- **Number scale:** all stats, caps and EXP numbers rescale with ticket 0013.
- **How Combat Arts relate to actives:** decided in `combat-arts.md` (0014).
- **Where the tier seals and Reclass Seals come from** (shops, chests, story):
  chapter and shop data (0009 and later).
