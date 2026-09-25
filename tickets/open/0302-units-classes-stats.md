---
id: "0302"
title: Units, classes and stats model with data files
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0001", "0003", "0005", "0301"]
nick_input: answer-first
completed:
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
   numbers. `assets/data/characters.ron`: 3 placeholder player characters
   (`test_lord`, `test_knight`, `test_archer` — whatever classes exist) and 2
   generic enemy templates, clearly marked `// PLACEHOLDER until 0701`.
5. `content` loaders + validation: unknown class/movement/weapon ids;
   promotion targets must exist, be exactly one tier higher and not
   `enemy_only`; `weapon_slots ≤ 3`; start rank ≤ max rank; `min_gains` and
   `cp_per_class_level` are non-decreasing by tier and have an entry for every
   tier used;
   at most 2 personal spells; one affinity per element per class; base ≤ caps; growths
   0..=255; level in 1..=max; every class reachable in the tree.
6. `Unit::from_character(def, class_table, level, faction, pos) -> Unit`, and
   `Unit::generic(class, level, faction, pos)` using the deterministic
   generic-unit formula in `progression.md`.

## Acceptance criteria

- [ ] Stat list, class list and numbers exactly match the design docs (a test compares the class table to a small hand-written expectation for at least 2 classes, one of them a shared promotion such as Iron Rider).
- [ ] The whole class tree in `progression.md` (tiers 1–3 and the enemy-only classes) is in `classes.ron`.
- [ ] All validation errors covered by tests.
- [ ] Data files load in the all-assets test.

## Tests required

- Unit: validators, `Faction::is_hostile_to` truth table, `from_character`, `generic` (matches a hand-worked example).
- Property: any `Unit` created via `from_character` has every stat ≤ its class cap and `hp == stats.hp`.

## Completion notes

