---
id: "0604"
title: Remove per-class stat caps (hard ceilings only)
type: feature
milestone: M5 Progression
model: sonnet-5
effort: medium
status: done
blocked_by: ["0019"]
nick_input: none
completed: 2026-09-28
---

# 0604 — Remove per-class stat caps (hard ceilings only)

## Context

Ticket 0019 (Nick, 2026-09-28): classes **don't cap stats**. A stat's maximum
comes only from the **hard ceilings** (`classes.ron` `hard_ceilings`,
`docs/design/stats-and-combat.md`) and the level cap. Classes shape growth only
through their growth rates, and staying in one class for a long time is never
penalised. See `docs/design/progression.md` → *Stat caps* and
*Level-up procedure* (step 2 now says "below its hard ceiling").

Today every class in `assets/data/classes.ron` has a `caps` tuple, and code
reads it in these places (found with `grep -rn caps crates`):

- `crates/content/src/class.rs`: the raw `caps` field, the
  `base ≤ caps ≤ hard ceilings` validation (`fn stats`), test fixtures.
- `crates/content/src/character.rs:368`: named characters' base stats must be
  ≤ the starting class's caps.
- `crates/core/src/class.rs:111`: `Class::caps` (`caps.mov == move_points`).
- `crates/core/src/progression.rs:206`: level-up gains clamped to
  `class.caps`, and a stat at its cap isn't eligible.
- `crates/core/src/unit.rs:253, 293`: `from_character` and the generic-unit
  formula clamp to `class.caps`.
- Doc comments: `core/src/progression.rs:14`, `core/src/unit.rs:110, 139`,
  `core/src/battle.rs:115`, `core/src/item.rs:26`,
  `ui/src/screens/battle/info.rs:4`, and the header of
  `assets/data/characters.ron`.

## Nick input

None. (The `MAX` marker for stats at a ceiling belongs to out-of-battle
screens and is part of 0603's choice screen, not this ticket.)

## Scope

**In:**
- Remove `caps` from the class data format, `classes.ron`, the loader,
  validation and `core::class::Class`.
- Everywhere a stat was clamped to a class cap, clamp to
  `Classes::hard_ceilings` instead.
- Update tests, fixtures, proptests and doc comments.

**Out (do not do):**
- Changing the hard-ceiling numbers, growth rates, base stats or the level
  cap (ticket 0013 decides the numbers).
- Any `MAX` marker or UI change (0603).
- Changing the number of RNG calls per level up (always 7 plus the safety
  net's; the replay format depends on it).

## Implementation steps

1. `crates/content/src/class.rs`: delete the raw `caps` field; `fn stats`
   checks `0 ≤ base ≤ hard ceilings`. Drop the cap-specific error messages
   and fixture lines; keep a test that a base over the hard ceiling is
   rejected. If `caps.mov` was used to validate or carry Mov, use `mov` and
   `mov_ceiling` directly.
2. `assets/data/classes.ron`: delete every `caps:` line and update the file's
   header comment.
3. `crates/core/src/class.rs`: remove `Class::caps`. Fix every compile error.
4. `crates/core/src/progression.rs`: a stat is eligible when
   `unit.stats.get(kind) < classes.hard_ceilings.get(kind)` and
   `growth > 0`; gains are clamped to the hard ceiling. Pass `&Classes` (or
   the ceilings) in if the function only had the `Class`.
5. `crates/core/src/unit.rs`: `from_character` clamps base stats to the hard
   ceilings; the generic-unit formula is
   `min(hard ceiling, base + growth × (L − 1) / 100)`.
6. `crates/content/src/character.rs`: named characters' base stats must be
   ≤ the hard ceilings (not class caps). Update the `characters.ron` header.
7. Update the doc comments listed above so none mention class caps.
8. `grep -rn -i "caps\b\|class cap" crates assets` returns nothing about stat
   caps (level cap, pack cap and crit "caps at 100" are fine).

## Acceptance criteria

- [x] `classes.ron` has no `caps`; `Class` has no `caps` field.
- [x] A unit whose stat was at its old class cap still gains in that stat on
      level up (unit test with fixed rolls, e.g. a Swordsman at Spd 25 gains
      Spd on a roll under 60).
- [x] A stat at its hard ceiling never gains, and is not a safety-net
      candidate (unit test).
- [x] Generic units at a high level are clamped to the hard ceilings, not a
      class cap (unit test, e.g. a level-99 Brawler's Spd).
- [x] Property test: after any sequence of level ups, every stat ≤ its hard
      ceiling and never lower than before.
- [x] Content validation rejects a class or character base stat above the
      hard ceiling.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the level-up and generic-unit cases above; content validation.
- Property: stats stay ≤ hard ceilings and never drop across level ups.
- Snapshot / integration: existing replays and snapshots still pass; if a
  replay changes because a stat no longer stops at a class cap, re-record it
  and say why in the PR.

## Completion notes

Done as planned. `caps` is gone from the class format, `classes.ron`, the
loader and `core::class::ClassDef`. `progression::level_up` now takes the
hard ceilings (`&Stats`) as an argument; `Unit::from_character` and
`Unit::generic` clamp to `ClassTable::hard_ceilings`; content validation
rejects a class or character base stat over the ceiling. Still 7 RNG calls
per level up. No replay or snapshot changed.

Deviations: core tests that used tight class caps now pass tight hard
ceilings instead (`roll_under` / `tight()` in `progression/tests.rs`). The
"stat above cap after a reclass" case became "stat above its ceiling".

No gameplay rules were decided here (*Claude's starting rule*: none).
