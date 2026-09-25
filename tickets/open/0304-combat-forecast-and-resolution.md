---
id: "0304"
title: Deterministic RNG, combat forecast and combat resolution
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: todo
blocked_by: ["0001", "0003", "0004", "0302"]
nick_input: answer-first
completed:
---

# 0304 — RNG, combat forecast, combat resolution

## Context

Implements the combat maths Nick chose, exactly. Sources of truth:
`docs/design/stats-and-combat.md` (formulas, RNG model, doubling, three worked
examples), `docs/design/weapons-and-items.md` (weapon stats, weapon-type traits,
effectiveness, attack speed / `as_bonus`, broken weapons, four more worked
examples; there is **no weapon triangle**),
`docs/design/magic.md` (magic damage). Determinism per
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
   weapons in `stats-and-combat.md`'s examples. Constants (×6/5, min 5, +15,
   broken might /2 and −20 hit, `rank_speed` table) come in as data, not
   scattered literals.
3. `CombatantInput { unit stats (gear-adjusted), tags: UnitTags (Mounted/Flying/Armored), weapon: Option<WeaponStats>, weapon_rank, armour_weight, terrain: &TerrainRules, … }`
   and `forecast(attacker: &CombatantInput, defender: &CombatantInput, distance: u32) -> Forecast`:
   `Forecast { attacker: SideForecast, defender: Option<SideForecast> }`,
   `SideForecast { damage, followup_damage, hit, crit, strikes: u8, effective: bool, broken: bool }`.
   Defender side is `None` when it has no weapon or `distance` is outside its
   range. Implement attack speed (`burden`, `rank_speed`), effectiveness,
   type traits and broken penalties from `weapons-and-items.md`, and magic
   rules from `magic.md`. All integer maths, rounding as specified; damage
   order: base → axe minimum → sword follow-up ×6/5 → crit ×3.
4. `resolve(forecast, hp_attacker, hp_defender, rng) -> CombatOutcome`:
   strike order per design (attacker, defender, then the faster side's extra
   strikes 2..N, N ≤ 4); each `Strike { by: Side, hit: bool, crit: bool, damage: u16, target_hp_after: u16 }`;
   stop as soon as someone reaches 0 HP. Hit roll procedure exactly per the
   design's 2RN model (always two rolls, hit iff `r1 + r2 < 2 * hit`). Crit
   rolls only on a hit (third roll). Strike thresholds `[4, 14, 24]` are data,
   not hard-coded constants scattered in code.
5. Put the three worked examples from `stats-and-combat.md` and W1–W4 from
   `weapons-and-items.md` into a table-driven test.

## Acceptance criteria

- [ ] Worked examples from `stats-and-combat.md` and `weapons-and-items.md` reproduce exactly.
- [ ] Changing any formula constant makes at least one test fail (mutation gate will verify).
- [ ] RNG is serializable and deterministic across platforms (test: fixed seed → fixed first 10 outputs, hard-coded).

## Tests required

- Unit: worked examples; each weapon trait (sword follow-up only on strikes 2..N, spear ×2 vs Mounted but not Flying, bow ×3 vs Flying, largest multiplier wins when several tags match, axe minimum only on a hit and before crit, gauntlet avoid only when equipped); burden/rank_speed boundaries (Str exactly cancelling weight); broken penalties; no-counter cases (range, no weapon); strike-count boundaries for 2, 3 and 4 strikes (exactly at each threshold, one below); strike order A, D, A, A for a triple; Example 1's scripted-roll resolution trace; lethal first strike ends combat.
- Property: `hit`, `crit` in 0..=100; `damage ≥ 0`; HP never underflows; strike count ≤ 4 (or the design's max); resolving with a `ScriptedRng` of all-0 rolls hits whenever `hit > 0`.
- Statistical (seeded, deterministic): `roll_percent` distribution roughly uniform (chi-square-ish bound over 100k rolls); if 2RN, displayed 80 hits ≈ 92% (±1%).

## Completion notes

