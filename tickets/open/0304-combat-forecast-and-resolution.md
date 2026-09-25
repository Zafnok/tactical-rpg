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
examples), `docs/design/weapons-and-items.md` (weapon stats, triangle),
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
2. `core::combat::WeaponStats { kind: WeaponKindId, might, hit, crit, weight, min_range, max_range, damage_type: Physical | Magical }`
   (fields per the weapons design; drop what isn't used).
3. `CombatantInput { unit stats, weapon: Option<WeaponStats>, terrain: &TerrainRules, … }`
   and `forecast(attacker: &CombatantInput, defender: &CombatantInput, distance: u32) -> Forecast`:
   `Forecast { attacker: SideForecast, defender: Option<SideForecast> }`,
   `SideForecast { damage, hit, crit, strikes: u8 }`. Defender side is `None`
   when it has no weapon or `distance` is outside its range. Include the weapon
   triangle / magic rules from the design docs. All integer maths, rounding as
   specified in the doc.
4. `resolve(forecast, hp_attacker, hp_defender, rng) -> CombatOutcome`:
   strike order per design (FE default: attacker, defender, then the side that
   doubles strikes again); each `Strike { by: Side, hit: bool, crit: bool, damage: u16, target_hp_after: u16 }`;
   stop as soon as someone reaches 0 HP. Hit roll procedure exactly per the
   design's RNG model (e.g. 2RN: hit if `(r1 + r2) / 2 < hit`, specify integer
   handling). Crit only rolls on a hit.
5. Put the three worked examples from the design doc into a table-driven test.

## Acceptance criteria

- [ ] Worked examples from `stats-and-combat.md` reproduce exactly.
- [ ] Changing any formula constant makes at least one test fail (mutation gate will verify).
- [ ] RNG is serializable and deterministic across platforms (test: fixed seed → fixed first 10 outputs, hard-coded).

## Tests required

- Unit: worked examples; triangle advantage/disadvantage; no-counter cases (range, no weapon); doubling threshold boundary (exactly at threshold, one below); lethal first strike ends combat.
- Property: `hit`, `crit` in 0..=100; `damage ≥ 0`; HP never underflows; strike count ≤ 4 (or the design's max); resolving with a `ScriptedRng` of all-0 rolls hits whenever `hit > 0`.
- Statistical (seeded, deterministic): `roll_percent` distribution roughly uniform (chi-square-ish bound over 100k rolls); if 2RN, displayed 80 hits ≈ 92% (±1%).

## Completion notes

