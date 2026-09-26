---
id: "0304"
title: Deterministic RNG, combat forecast and combat resolution
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: done
blocked_by: ["0001", "0003", "0004", "0302"]
nick_input: answer-first
completed: 2026-09-26
---

# 0304 — RNG, combat forecast, combat resolution

## Context

Implements the combat maths Nick chose, exactly. Sources of truth:
`docs/design/stats-and-combat.md` (formulas, RNG model, doubling, three worked
examples), `docs/design/weapons-and-items.md` (weapon stats, weapon-type traits,
effectiveness, attack speed / `as_bonus`, broken weapons, four more worked
examples; there is **no weapon triangle**),
`docs/design/magic.md` (attack spells use the same maths with Mag vs Res;
elemental affinities Weak/Resist/Absorb; four more worked examples M1–M4). Determinism per
[ADR-0004](../../docs/adr/0004-crate-architecture.md).

## Nick input

**Answer first:** 0001, 0003, 0004.

## Scope

**In:** `core::rng`, `core::combat` (forecast + resolution), a minimal
`WeaponStats` value type sufficient for combat maths.

**Out:** inventory/equipping (0306), EXP (0601), applying results to battle
state (0305), UI (0404).

## Implementation steps

1. `core::rng::SimRng`: a small PCG32 (or xoshiro) implemented in-crate or via
   `rand_pcg` — must be `Clone + Serialize + Deserialize + PartialEq` so battle
   state can be saved and replayed. `SimRng::new(seed: u64)`, `next_u32()`,
   `roll_percent() -> u8` (uniform 0..=99, **unbiased** — use rejection
   sampling). A `RandomSource` trait so tests can inject a `ScriptedRng`
   (returns a given sequence).
2. `core::combat::WeaponStats { kind: WeaponKindId, trait_: WeaponTrait, might, hit, crit, weight, min_range, max_range, damage_type: Physical | Magical, effective: Vec<(UnitTag, u8)>, broken: bool }`
   with `WeaponTrait { None, SwordFollowUp, AxeMinDamage(v), GauntletAvoid(v), Effective(tag, mult) }`
   (spear/bow traits are `Effective`). `None` exists for the typeless
   weapons in `stats-and-combat.md`'s examples and for attack spells
   (`magic.md`: a spell enters combat as a `WeaponStats` with weight 0,
   rank bonus 0, never broken, plus an `element: Element { Fire, Ice, None }`
   field; `None` for physical weapons). Constants (×6/5, min 5, +15,
   broken might /2 and −20 hit, `rank_speed` table) come in as data, not
   scattered literals.
3. `CombatantInput { unit stats (gear-adjusted), tags: UnitTags (Mounted/Flying/Armored), affinities: Vec<(Element, Affinity)>, weapon: Option<WeaponStats>, weapon_rank, armour_weight, terrain: &TerrainRules, … }`
   (`Affinity { Weak, Resist, Absorb }`, `magic.md`)
   and `forecast(attacker: &CombatantInput, defender: &CombatantInput, distance: u32) -> Forecast`:
   `Forecast { attacker: SideForecast, defender: Option<SideForecast> }`,
   `SideForecast { damage, followup_damage, hit, crit, strikes: u8, effective: bool, broken: bool, affinity: Option<Affinity> }`
   (for `Absorb`, `damage` is the amount healed per hit and `crit` is 0).
   Defender side is `None` when it has no weapon or `distance` is outside its
   range. Implement attack speed (`burden`, `rank_speed`), effectiveness,
   type traits and broken penalties from `weapons-and-items.md`, and the
   affinity rules from `magic.md` (Weak = effectiveness ×3, largest
   multiplier wins; Resist halves damage before crit; Absorb heals the target
   on a hit, never above max HP; physical attacks ignore affinities). All integer maths, rounding as specified; damage
   order: base → axe minimum → sword follow-up ×6/5 → crit ×3.
4. `resolve(forecast, hp_attacker, hp_defender, rng) -> CombatOutcome`:
   strike order per design (attacker, defender, then the faster side's extra
   strikes 2..N, N ≤ 4); each `Strike { by: Side, hit: bool, crit: bool, damage: u16, target_hp_after: u16 }`;
   stop as soon as someone reaches 0 HP. Hit roll procedure exactly per the
   design's 2RN model (always two rolls, hit iff `r1 + r2 < 2 * hit`). Crit
   rolls only on a hit (third roll). Strike thresholds `[4, 14, 24]` are data,
   not hard-coded constants scattered in code.
5. Put the three worked examples from `stats-and-combat.md`, W1–W4 from
   `weapons-and-items.md` and M1–M2 from `magic.md` into a table-driven
   test. (Spell uses, healing and M3/M4 are ticket 0309.)

## Acceptance criteria

- [x] Worked examples from `stats-and-combat.md`, `weapons-and-items.md` and `magic.md` (M1–M2) reproduce exactly.
- [x] Changing any formula constant makes at least one test fail (mutation gate will verify).
- [x] RNG is serializable and deterministic across platforms (test: fixed seed → fixed first 10 outputs, hard-coded).

## Tests required

- Unit: worked examples; each weapon trait (sword follow-up only on strikes 2..N, spear ×2 vs Mounted but not Flying, bow ×3 vs Flying, largest multiplier wins when several tags match, axe minimum only on a hit and before crit, gauntlet avoid only when equipped); affinities (Weak ×3 vs a larger tag multiplier, Resist halving before crit, Absorb heals and never exceeds max HP, physical ignores affinities); burden/rank_speed boundaries (Str exactly cancelling weight); broken penalties; no-counter cases (range, no weapon); strike-count boundaries for 2, 3 and 4 strikes (exactly at each threshold, one below); strike order A, D, A, A for a triple; Example 1's scripted-roll resolution trace; lethal first strike ends combat.
- Property: `hit`, `crit` in 0..=100; `damage ≥ 0`; HP never underflows; strike count ≤ 4 (or the design's max); resolving with a `ScriptedRng` of all-0 rolls hits whenever `hit > 0`.
- Statistical (seeded, deterministic): `roll_percent` distribution roughly uniform (chi-square-ish bound over 100k rolls); if 2RN, displayed 80 hits ≈ 92% (±1%).

## Completion notes

**Done.** New modules: `trpg_core::rng` (`SimRng`, `RandomSource`,
`ScriptedRng`) and `trpg_core::combat` (`CombatRules`, `WeaponStats`,
`WeaponTrait`, `DamageType`, `CombatantInput`, `forecast`, `SideForecast`,
`Forecast`, `resolve`, `roll_hit`, `Strike`, `Side`, `CombatHp`,
`CombatOutcome`). New ADR-0019 (in-crate PCG32; serde derives in `core`).

What the next tickets should know (API differences from the plan):

- **Constants are a `CombatRules` value** (thresholds `[4, 14, 24]`,
  `rank_speed`, ×6/5, min 5, +15, broken /2 and −20, crit ×3, Weak ×3,
  Resist /2, the Dex/Spd factors). `CombatRules::default()` holds the design
  values; `forecast` and `resolve` take `&CombatRules` as their first
  argument. It derives serde, so a balance ticket (e.g. 0013) can load it from
  data instead of editing `Default`.
- **`forecast` returns `Option<Forecast>`**: `None` when the attacker has no
  weapon or the target is outside its range (the UI/AI should only offer
  valid targets anyway).
- **`resolve(rules, &forecast, CombatHp, CombatHp, rng)`**: `CombatHp` is
  current + max HP, because an Absorb hit heals up to max HP.
- **`WeaponStats.kind` is `Option<WeaponKind>`** (the existing enum; there is
  no `WeaponKindId`). `None` = spell or typeless weapon: rank speed 0. Build a
  weapon's `trait_` with `CombatRules::type_trait(kind)`.
- **Values use `StatValue`** (not `u16`) for damage and HP, per
  `stats-and-combat.md`'s "one stat value type everywhere". `hit`/`crit` are
  `u8` in `0..=100`, `strikes` is `u8`.
- **`Strike` has an extra `healed: bool`** (Absorb). `damage` is the strike's
  damage (crit included) if it hit, else 0; for Absorb it is the heal before
  the max-HP cap. `target_hp_after` is the result.
- **Fliers** get no terrain Def/Avoid inside the combat maths
  (`CombatantInput.tags.flying`), so callers pass the real tile.
- `SimRng::roll_percent` caps rejection redraws at 16 (never reached in
  practice) so mutation testing can't hang; see ADR-0019.
- Added serde derives to `Element`, `Affinity`, `UnitTag`, `WeaponKind`
  (needed by the combat output types).

Tests: every worked example (stats 1–3 with their boundaries, W1–W4 with
W2b, M1–M2 with the Resist variant) in one table; Example 1's scripted trace
(8 rolls); W1's crit (30); M2 healing 20→29 and 25→30; one test per trait and
affinity rule; burden/rank boundaries and the attack-speed illustration table;
strike-count thresholds; strike orders A-D-A-A and A-D-D; lethal first strike
and lethal counter; exact 2RN counts over all 10 000 roll pairs (80 → 9220 etc.)
and a seeded 100k-roll check (≈92%); chi-square uniformity; PCG reference
vector; RON round-trip; proptest invariants. `cargo mutants` on the diff:
145 caught, 0 missed.

No follow-up tickets. Nothing for Nick to play yet (no UI; 0404 shows it).
