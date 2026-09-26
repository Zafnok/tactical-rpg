# ADR-0019: In-crate PCG32 simulation RNG; serde derives in `core`

- **Status:** Accepted
- **Date:** 2026-09-26
- **Related tickets:** 0304, 0305, 0307, 0802

## Context

ADR-0004 makes `core` deterministic, with all randomness from a seeded RNG
owned by the battle state, and leaves the exact generator to ticket 0304.
Replays, turn rewind (0307) and save/suspend (0802) need the RNG's position to
be saved and restored exactly, and the same seed must give the same rolls on
Windows, WASM and CI Linux, for every future dependency update.

`core` had no dependencies. Battle state (0305) must derive
`Serialize`/`Deserialize` so saves and rewinds can serialise it; its fields
are `core` types.

## Decision

1. **Generator:** `trpg_core::rng::SimRng` is a PCG32 (XSH RR, 64-bit state,
   64-bit odd increment) written in `crates/core/src/rng.rs` (~40 lines), not a
   crate. `SimRng::new(seed)` uses PCG's reference seeding on one fixed stream.
   A test pins its output to the PCG reference vector (seed 42, stream 54) and
   another pins the first 10 `roll_percent` values of a fixed seed.
2. **Percent rolls** (`roll_percent`, `0..=99`) use rejection sampling
   against the largest multiple of 100 below `2^32`, capped at 16 redraws. A
   draw is rejected with chance `96 / 2^32`, so the cap never matters for
   fairness; it guarantees termination (and lets mutation testing kill loop
   mutants instead of timing out).
3. **Injection:** rules take `&mut impl RandomSource` (one method,
   `roll_percent`). Tests use `ScriptedRng`, which replays a list of rolls and
   panics when it runs out.
4. **serde in `core`:** `core` depends on `serde` (derive) and derives
   `Serialize`/`Deserialize` on types that go into saved state or events
   (`SimRng`, combat forecast/outcome types, and the small enums they
   contain). This is not I/O: `core` still never reads or writes anything;
   `content`/`app` choose the format (RON) and do the writing. `ron` is a
   dev-dependency of `core` only, for round-trip tests.

## Consequences

- The random sequence depends only on `rng.rs`; no dependency bump can change
  replays or saved battles. Changing `rng.rs` (or the seeding stream) breaks
  old replays and must be treated like a save-format change.
- Later tickets add `Serialize`/`Deserialize` derives to `core` types as they
  join the battle state; no new ADR needed for that.
- We maintain ~40 lines of RNG code ourselves, pinned by tests.

## Alternatives considered

- **`rand_pcg` / `rand_xoshiro` crates** — fine algorithms, but the output
  (and serde representation) then depends on external crate versions, and
  `rand_core` API churn has changed seeding helpers before. The algorithm is
  tiny, so owning it is cheaper than tracking it.
- **xoshiro128++** — equally good; PCG32 has a single well-known reference
  vector that makes the cross-platform test trivial.
- **Serialising battle state through mirror types in `content`** — doubles
  every state type and its tests for no gain; serde derives are pure.
