# Stats and combat maths

Decided: 2026-09-25
Source: ticket 0001

## Nick's words

> **Q1. Which stats should units have?** "Streamlined FE (Recommended)"
>
> **Q2. How random should hitting be?** "FE 'true hit' (Recommended)"
>
> **Q3. Should fast units attack twice?** "we can have even beyond doubling,
> but make it truly rare for a unit to attack 3 or 4 times from speed alone...
> as an example, a unit in a specialized speedy class (i.e. a punching class)
> with huge investment in the weapon skill, the speed stat, best in slot gear,
> and 10 lvs up on an enemy might be able to hit 3x in a combat. The same unit
> at even lv might just hit 2x. The same unit at max lv might be able to hit
> 4x. Lv doesn't play a role in the number of attacks I am just using it here
> as an example of stat growth."

Follow-up, on the strike thresholds and number sizes:

> "I don't think we ever decided on scaling algorithms yet... so I think we can
> push this question to later. Like maybe I wanna be crazy like dragon ball and
> we're talking trillions of points endgame who knows. Unlikely given we're
> limited by the terminal size but just saying. Let's just say it's possible
> and you can figure out the scaling algorithm once I have a playtest and see
> if big numbers or low numbers or even medium numbers look nicer to me"

So: streamlined Fire Emblem stats (no Luck, no Constitution/Build), FE "true
hit" (2RN) hit rolls, and FE doubling extended to up to **4 strikes**, where
3 and 4 need very large Speed gaps. Level itself never enters combat maths.

**The number scale is not decided.** How big stats get (single digits, FE-ish
tens, or huge "Dragon Ball" numbers) and therefore the exact strike thresholds
are placeholders until Nick has played (ticket 0013, after the Chapter 1
playtest 0804). The ranges, caps and thresholds below are FE-sized starting
values so the game can be built and played; the *rules* (which stats, how they
combine, 2RN, up to 4 strikes, 3x/4x rare) are Nick's and stay.

Everything below marked *tunable* is a starting value Claude chose; balance
tickets may change it without asking Nick. Unmarked rules are Nick's choices.

## Stat list

All stats are non-negative integers. Code must use one stat value type
alias everywhere (not a hard-wired `u8`) so the scale can grow later without a
rewrite. Every unit also has a current HP (`0..=HP`). Per-class caps and growths come from
`progression.md` (ticket 0005); the hard ceilings here are absolute limits
that no class cap, gear or bonus may exceed.

| Stat | Meaning | Range |
| ---- | ------- | ----- |
| `HP`  | Max hit points. The unit is defeated at 0 current HP. | `1..=80` hard ceiling, cap per class *(tunable)* |
| `Str` | Physical attack power (added to physical weapon might). | `0..=50` hard ceiling, cap per class *(tunable)* |
| `Mag` | Magical attack power (added to magical weapon/tome might). | `0..=50` hard ceiling, cap per class *(tunable)* |
| `Dex` | Precision: raises hit and crit, and lowers the crits taken. | `0..=50` hard ceiling, cap per class *(tunable)* |
| `Spd` | Quickness: raises avoid and decides extra strikes. | `0..=50` hard ceiling, cap per class *(tunable)* |
| `Def` | Reduces physical damage taken. | `0..=50` hard ceiling, cap per class *(tunable)* |
| `Res` | Reduces magical damage taken. | `0..=50` hard ceiling, cap per class *(tunable)* |
| `Mov` | Movement points per turn (see movement/pathfinding). Set by class; does not grow on level up. | `0..=15`, set per class, typically 4–8 *(tunable)* |

All ranges above are *placeholders* for the FE-sized first build (see
ticket 0013). Guideline for class caps (*tunable*, for 0005): specialist stats
cap around 35–40 (e.g. a speed class's Spd), ordinary stats 20–30, HP 50–60.
The strike thresholds below are sized for these numbers.

No Luck, no Constitution/Build/weight stat. Whether weapons have weight (and
what it would subtract from) is ticket 0003's decision; see *Attack speed*.

## Combat formulas

All maths is integer. `/` is integer division rounding toward zero (all
operands are non-negative where it's used). `clamp(x, lo, hi)` bounds `x`.
Intermediate values are signed (`i32`) so subtractions can't underflow.

Notation: `A` = the side computing its numbers, `B` = the other side.
`weapon.*` = A's equipped weapon (fields fixed by 0003); `terrain_B` = the
terrain B stands on.

### Attack and damage

```
power      = (weapon.damage_type == Physical ? A.Str : A.Mag) + weapon.might
mitigation = (weapon.damage_type == Physical ? B.Def : B.Res) + terrain_B.defense
damage     = max(0, power - mitigation)
crit_damage = damage * 3
```

Weapon-type traits, effectiveness, broken weapons and gauntlet avoid are in
`weapons-and-items.md` (0003); spells, elemental affinities and healing in
`magic.md` (0004).

### Hit

```
avoid_B = B.Spd * 2 + terrain_B.avoid
hit     = clamp(weapon.hit + A.Dex * 2 - avoid_B, 0, 100)
```

### Crit

```
crit = clamp(weapon.crit + A.Dex / 2 - B.Dex / 4, 0, 100)      (tunable)
```

`B.Dex / 4` is the "crit avoid" that Luck gave in classic FE.

### Attack speed and number of strikes

```
AS_A = A.Spd + A.as_bonus
diff = AS_A - AS_B

strikes_A = 1  if diff < 4
            2  if 4  <= diff < 14
            3  if 14 <= diff < 24
            4  if diff >= 24
```

- Thresholds `4 / 14 / 24` are *placeholders*, stored as data (a list
  `[4, 14, 24]`), so the maximum number of strikes is `1 + len(list)` = 4.
  Nick fixed only that 3x and 4x are possible but rare; the actual thresholds
  (and whether they are fixed gaps, percentages of the enemy's Speed, or
  something else) are set with the number scale in ticket 0013.
- At most one side gets more than 1 strike (diff is positive for only one).
- Sizing (what Nick asked for): a speed specialist a bit ahead of an equal
  enemy (+4..+13) strikes 2x; ten levels of heavy Spd growth plus best gear
  (+14..+23) gives 3x; a capped Spd-40 unit with gear against an ordinary
  Spd-16..20 enemy (+24) gives 4x. 3x and 4x should be rare.
- `as_bonus` is defined in `weapons-and-items.md` (0003): weapon rank bonus
  minus the burden of weapon + armour weight not carried by Str; gear adds to
  Spd itself. Class skills (0005) may add more. Level never adds to it
  directly. The worked examples below use weight 0, rank E and no gear, so
  their `as_bonus` is 0.

### Counterattacks

The defender counters only if it has an equipped weapon whose range includes
the combat distance. Otherwise the defender's side of the forecast is empty.
Both sides use the same formulas; the defender may also get extra strikes.

### Strike order

1. Attacker's first strike.
2. Defender's first strike (if it can counter).
3. The faster side's extra strikes (strikes 2..N), one after another.

Combat stops the moment either unit reaches 0 HP. A strike that deals 0
damage still happens (and can still "hit").

### RNG procedure ("true hit", 2RN)

`roll()` = a uniform, unbiased integer in `0..=99` from the battle's seeded
RNG. For each strike, in strike order:

1. `r1 = roll()`, `r2 = roll()` — always both, even when `hit` is 0 or 100.
2. **Hit** iff `r1 + r2 < 2 * hit`. (Equivalent to "average of two rolls
   `< hit`" with no rounding.) `hit = 0` never hits; `hit = 100` always hits.
3. Only if it hit: `r3 = roll()`; **crit** iff `r3 < crit`.
4. Damage dealt = `crit ? crit_damage : damage`, applied as
   `hp = hp.saturating_sub(dealt)`.
5. A miss consumes exactly 2 rolls; a hit consumes exactly 3.

Resulting true chances (for the forecast help text, not for code): displayed
80 → 92.2%, 90 → 98.1%, 50 → 50.5%, 20 → 8.2%, 10 → 2.1%. (Hit iff
`r1 + r2 < 160` for 80: 780 of 10 000 pairs fail.) Crit is a single honest roll.

The forecast always shows the *displayed* numbers (`hit`, `crit`), never the
true percentage.

## Terrain combat effects

Terrain Def/Avoid values, the flier rule and healing tiles are decided in
[`terrain.md`](terrain.md) (Nick, 2026-09-26). In the formulas above,
`terrain_B.defense` and `terrain_B.avoid` are that table's values for the tile
B stands on (0 for a flying B).

## Worked examples

These use placeholder weapons (the numbers are inputs, not a weapon list;
0003 owns the real weapons): weight 0, rank E, no armour, no weapon-type
trait. `weapons-and-items.md` has examples with real weapons. Ticket 0304 turns
them into table-driven tests.

### Example 1 — plain melee, attacker doubles (exactly at the threshold)

| | HP | Str | Mag | Dex | Spd | Def | Res | Weapon | Terrain |
|-|----|-----|-----|-----|-----|-----|-----|--------|---------|
| Attacker (swordfighter) | 22 | 8 | 0 | 7 | 9 | 5 | 2 | physical Mt 5 Hit 90 Crit 0, range 1 | plain |
| Defender (brigand) | 20 | 9 | 0 | 3 | 5 | 4 | 1 | physical Mt 8 Hit 75 Crit 0, range 1 | plain |

Distance 1.

- Attacker: damage `8+5 − 4 = 9`; hit `90 + 14 − 10 = 94`; crit `0 + 3 − 0 = 3`;
  diff `9 − 5 = 4` → **2 strikes**.
- Defender: damage `9+8 − 5 = 12`; hit `75 + 6 − 18 = 63`; crit `0 + 1 − 1 = 0`;
  **1 strike**.

**Forecast:** Attacker `dmg 9 ×2, hit 94, crit 3` / Defender `dmg 12, hit 63, crit 0`.

Resolution with scripted rolls `[10, 20, 50, 70, 80, 0, 0, 99]`:

1. Attacker: `10+20 = 30 < 188` hit; crit roll `50 ≥ 3` no → defender 20 → 11.
2. Defender: `70+80 = 150 < 126`? no → miss (no crit roll).
3. Attacker: `0+0 < 188` hit; crit roll `99` no → defender 11 → 2.

End: attacker 22 HP, defender 2 HP; 8 rolls consumed.

Boundary: with defender Spd 6 (diff 3) the attacker gets 1 strike (avoid
becomes 12, hit 92).

### Example 2 — magic at range into a forest, no counter

| | HP | Str | Mag | Dex | Spd | Def | Res | Weapon | Terrain |
|-|----|-----|-----|-----|-----|-----|-----|--------|---------|
| Attacker (mage) | 18 | 1 | 9 | 6 | 7 | 2 | 6 | magical Mt 5 Hit 85 Crit 0, range 1–2 | plain |
| Defender (knight) | 25 | 10 | 0 | 4 | 3 | 11 | 2 | physical Mt 7 Hit 80 Crit 0, range 1 | forest |

Distance 2.

- Attacker: damage `9+5 − (2+1) = 11`; hit `85 + 12 − (6+20) = 71`;
  crit `0 + 3 − 1 = 2`; diff `7 − 3 = 4` → **2 strikes**.
- Defender: range 1 only → **no counter**.

**Forecast:** Attacker `dmg 11 ×2, hit 71, crit 2` / Defender `—`.

Variant 2b, same units at distance 1: the knight counters with damage
`10+7 − 2 = 15` (physical, so the mage's Def counts), hit `80 + 8 − 14 = 74`,
crit `0 + 2 − 1 = 1`, 1 strike.

### Example 3 — the rare triple (speed specialist vs. fort)

| | HP | Str | Mag | Dex | Spd | Def | Res | Weapon | Terrain |
|-|----|-----|-----|-----|-----|-----|-----|--------|---------|
| Attacker (brawler) | 34 | 20 | 0 | 22 | 30 | 10 | 8 | physical Mt 3 Hit 100 Crit 10, range 1 | plain |
| Defender (warrior) | 40 | 18 | 0 | 10 | 16 | 12 | 5 | physical Mt 8 Hit 75 Crit 0, range 1 | fort |

Distance 1.

- Attacker: damage `20+3 − (12+2) = 9` (crit 27); hit `100 + 44 − (32+20) = 92`;
  crit `10 + 11 − 2 = 19`; diff `30 − 16 = 14` → **3 strikes**.
- Defender: damage `18+8 − 10 = 16`; hit `75 + 20 − 60 = 35`;
  crit `0 + 5 − 5 = 0`; **1 strike**.

**Forecast:** Attacker `dmg 9 ×3, hit 92, crit 19` / Defender `dmg 16, hit 35, crit 0`.
Strike order: attacker, defender, attacker, attacker.

Boundaries: defender Spd 17 (diff 13) → attacker 2 strikes; attacker Spd 40
vs. defender Spd 16 (diff 24) → 4 strikes.

## Open sub-questions (deferred)

- **Number scale and strike thresholds:** ticket 0013, decided after Nick's
  Chapter 1 playtest. Small, FE-sized or huge numbers; thresholds follow.

- **Attack-speed modifiers:** decided in `weapons-and-items.md` (0003);
  class skills that add more: 0005.
- **Weapon traits / effectiveness:** decided in `weapons-and-items.md` (0003):
  no weapon triangle.
- **Magic specifics** (spells, uses, elements, healing): decided in `magic.md` (0004).
- **Skills** that add strikes (e.g. brave weapons) or ignore thresholds: not
  planned yet; would need a new decision ticket.
