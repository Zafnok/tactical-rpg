---
id: "0302"
title: Units, classes and stats model with data files
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: done
blocked_by: ["0001", "0003", "0005", "0301"]
nick_input: answer-first
completed: 2026-09-26
---

# 0302 — Units, classes and stats

## Context

Implements the stat list and class structure Nick chose. Source of truth:
`docs/design/stats-and-combat.md` (0001), `docs/design/progression.md` (0005),
`docs/design/weapons-and-items.md` (0003), `docs/design/magic.md` (0004). Do **not** invent stats or classes not in those docs.

## Nick input

**Answer first:** tickets 0001, 0003, 0005.

## Scope

**In:** `core::stats`, `core::class`, `core::unit`, `assets/data/classes.ron`,
`assets/data/characters.ron` (placeholder characters only), loaders and
validators.

**Out:** inventory/items (0306), level-up logic (0601), promotion logic (0603),
real story characters (07xx creates them).

## Implementation steps

1. `core::stats`: `StatKind` enum with exactly the stats in the design doc;
   `Stats` struct with one integer field per stat, all of one type alias
   `pub type StatValue = i32;` used everywhere stats, HP and damage are
   stored (the number scale is undecided until ticket 0013 and may grow
   to huge values; changing the alias must be the only edit needed), and
   `get(kind)`/`set(kind)`; `Growths` (percent per stat) if the
   design uses growths.
2. `core::class::ClassDef`: `id`, `name`, `tier`, `movement_type: MovementTypeId`,
   `move_points`, `base: Stats`, `caps: Stats`, growth modifiers (if design
   has them), usable weapon kinds with starting and maximum weapon rank (kinds from
   `weapons-and-items.md`), allowed armour weights (Light/Medium/Heavy, if
   0005 limits them), `tags: UnitTags` (`Mounted`, `Flying`, `Armored`; used
   by weapon effectiveness), `promotes_to: Vec<ClassId>`,
   `active: Option<SkillId>` (usable from unlock; permanent on mastery) and
   `passives: Vec<SkillId>` (learned on mastery) — ids only, effects are 0311; `enemy_only: bool`;
   and from `magic.md`: `weapon_slots: u8` (3, or 0 for tier-3+ magic
   classes), `spells: Vec<(u8 /* class level */, SpellId)>` (ids only; spell
   definitions, uses and id validation are 0309), `affinities: Vec<(Element, Affinity)>`.
   `tier` is a plain number (Nick expects 6–10 tiers eventually; nothing may
   assume 3 is the top). Per-tier `min_gains` (safety net) and
   `cp_per_class_level` tables live in the same data file. Character defs get `talent: StatKind` (not Mov),
   base stats, starting weapon ranks, and
   `personal_spells: Vec<(u8 /* character level */, SpellId)>` (0–2 entries).
3. `core::unit`:
   - `Faction { Player, Enemy, Ally, Neutral }` with `is_hostile_to(other)`
     (Player+Ally friendly; Enemy hostile to both; Neutral per design default:
     hostile to nobody).
   - `UnitId(u32)`, `Unit { id, character: Option<CharacterId>, name, class, level, exp, class_records: BTreeMap<ClassId, ClassRecord { class_level, class_points }>, stats, hp, faction, pos: Pos, acted: bool, is_lord: bool }`.
     `level` is the **character level** (never resets); class levels live in
     `class_records` (`progression.md`).
     Stats stored are *current permanent* stats (base + growth gains). They
     are ≤ the class caps, except after a reclass, when stats above the new
     class's caps are kept (`progression.md`).
4. `assets/data/classes.ron`: the class tree from `progression.md` with its
   numbers. The flying line starts at tier 3 (Flier, promoted from Lancer;
   ticket 0017); its tier-4/5 classes have no numbers yet, so leave them out
   and give Flier an empty `promotes_to`. The tree includes the **lord's
   line** (Exile → Blade Heir / Commander → Sovereign / Grand Marshal;
   ticket 0016), marked `lord_only`. `assets/data/characters.ron`: 3 placeholder player characters
   (`test_lord`, `test_knight`, `test_archer` — whatever classes exist) and 2
   generic enemy templates, clearly marked `// PLACEHOLDER until 0701`.
5. `content` loaders + validation: unknown class/movement/weapon ids;
   promotion targets must exist, be exactly one tier higher and not
   `enemy_only`; a `lord_only` class promotes only into `lord_only` classes,
   and only an `is_lord` character may start in one; `weapon_slots ≤ 3`; start rank ≤ max rank; `min_gains` and
   `cp_per_class_level` are non-decreasing by tier and have an entry for every
   tier used;
   at most 2 personal spells; one affinity per element per class; base ≤ caps; growths
   0..=255; level in 1..=max; every class reachable in the tree.
6. `Unit::from_character(def, class_table, level, faction, pos) -> Unit`, and
   `Unit::generic(class, level, faction, pos)` using the deterministic
   generic-unit formula in `progression.md`.

## Acceptance criteria

- [x] Stat list, class list and numbers exactly match the design docs (a test compares the class table to a small hand-written expectation for at least 2 classes, one of them a shared promotion such as Iron Rider).
- [x] The whole class tree in `progression.md` (tiers 1–3, including the tier-3 Flier, the lord's line, and the enemy-only classes) is in `classes.ron`.
- [x] All validation errors covered by tests.
- [x] Data files load in the all-assets test.

## Tests required

- Unit: validators, `Faction::is_hostile_to` truth table, `from_character`, `generic` (matches a hand-worked example).
- Property: any `Unit` created via `from_character` has every stat ≤ its class cap and `hp == stats.hp`.

## Completion notes

- **Core** (`trpg-core`): `stats` (`StatValue = i32` alias used for every
  stat/HP value, `StatKind` with the 8 stats of `stats-and-combat.md`,
  `Stats` with `get`/`set`, `Growths`), `class` (`ClassDef`, `ClassTable`
  with the per-tier `min_gains` / `cp_per_class_level` tables, class-level
  cap, level cap and hard ceilings), `unit` (`Faction::is_hostile_to`,
  `Unit`, `CharacterDef`, `ClassRecord`, `Unit::from_character`,
  `Unit::generic`), plus two small id/enum modules: `weapon` (`WeaponKind`,
  `WeaponRank`) and `magic` (`SpellId`, `Element`, `Affinity`). Weapons and
  spells themselves stay with 0306 / 0309.
- **Data**: `assets/data/classes.ron` holds all 45 classes of tiers 1–3
  (incl. the tier-3 Flier, the lord's line marked `lord_only`, and the three
  enemy-only classes). A script cross-checked every class's stats, Mov,
  movement type, weapon ranks, armour, tags, slots, spells and promotions
  against `progression.md`. `assets/data/characters.ron` has the 3
  placeholder characters and 2 generic enemy templates, marked
  `PLACEHOLDER until 0701`.
- **Validation** (`trpg-content` `class` / `character`): every rule in step 5
  and a few more from the design docs: caps ≤ hard ceilings, no negative
  stats, Mov in `0..=15`, tier ≥ 1, duplicate ids, a non-lord class can't
  promote into a lord-only class, generic templates can't use lord-only
  classes, talent isn't Mov, the lord has no personal spells, and starting
  weapon ranks ≤ the class's max rank. Each has a test.

Deviations:

- `Unit::from_character` / `Unit::generic` take a `UnitId` and return
  `Result<Unit, UnitError>` (unknown class, or a non-lord in a lord-only
  class). `from_character` has no `level` argument: the character's data
  level is used, since spawning at another level would need level-up logic
  (0601).
- `Unit` also has `weapon_ranks` (the character's starting ranks, raised to
  the class's start ranks; generics get the class's start ranks), which the
  ticket's field list left out but the design needs somewhere.
- `Stats` includes Mov; it's always set from the class's `move_points`.
- Character levels (incl. personal-spell levels) use `Level = u32` instead
  of `u8`, because the level cap may become huge (0013).
- Stat rows in the RON files are tuples `(HP, Str, Mag, Dex, Spd, Def, Res)`.
- Tier-3 classes other than Flier have no skills yet (ticket 1001).

No follow-up tickets.

