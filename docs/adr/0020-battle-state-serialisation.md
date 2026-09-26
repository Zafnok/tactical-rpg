# ADR-0020: Battle state saves its own data, not the content tables

- **Status:** Accepted
- **Date:** 2026-09-26
- **Related tickets:** 0305, 0307, 0802

## Context

`trpg_core::BattleState` (0305) must be serialisable: the suspend save (0802)
writes it, turn rewind (0307) replays from a stored state, and replay tests
compare saved and continued battles. ADR-0019 already allows serde derives in
`core`.

A battle reads two kinds of data:

- **Its own state:** the map (terrain magic changes tiles, 0310), the units on
  the map and the fallen ones, reinforcements still to come, the objective,
  turn, phase, the RNG position and the outcome.
- **Shared content:** the terrain table and the class table, loaded from
  `assets/data` by `trpg-content`. They are the same for every battle and
  every save, and a class table holds ~45 classes.

## Decision

1. `BattleState` derives `Serialize`/`Deserialize`. Every field of its own
   state is saved, including the whole `BattleMap` and the `SimRng`.
2. The terrain and class tables are held as `Arc<TerrainTable>` and
   `Arc<ClassTable>` (given in `BattleSetup`) inside a private field marked
   `#[serde(skip)]`. They are **not** saved.
3. A deserialised state has empty tables. The loader must call
   `BattleState::restore_tables(terrain, classes)` with the game's tables
   before applying commands. Until then every `Command::Act` fails with
   `CommandError::UnknownClass` (never a panic), so a forgotten call shows up
   at once in tests instead of corrupting a battle.
4. `Grid<T>` checks its cell count when deserialised (`try_from` a raw form),
   so a malformed save can't break the grid's invariant.
5. The combat constants are `CombatRules::default()` for now; if a balance
   ticket makes them data, they join the tables (not the save) the same way.

## Consequences

- Saves stay small and contain no content, so a content fix (e.g. a class's
  Mov) applies to battles suspended before the fix. The flip side: a content
  change can alter how a suspended battle continues. That is acceptable for a
  one-shot suspend save; replays that must stay exact (tests, rewind) use the
  same tables in the same process.
- Every loader (0802 suspend, 0307 rewind if it serialises) must remember
  `restore_tables`; the loud `UnknownClass` failure and a test in 0305 guard
  it.
- `Unit`, `Pos`, `Stats`, ids and the map types gained serde derives.

## Alternatives considered

- **Serialise the tables inside every save** — self-contained saves, but every
  save (and every rewind snapshot) carries the whole class table, and content
  fixes never reach suspended battles.
- **Separate `SavedBattle` mirror type** — the loader couldn't forget the
  tables, but it duplicates every state field and its tests; ADR-0019 already
  rejected mirror types for the same reason.
- **`DeserializeSeed` passing the tables in** — correct by construction but
  awkward for every format and caller; not worth it for one call site.
