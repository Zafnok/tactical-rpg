---
id: "0314"
title: Count attack spells in a unit's attack ranges (threat area, danger zone)
type: feature
milestone: M2 Core rules
model: sonnet-5
effort: medium
status: done
blocked_by: ["0309"]
nick_input: none
completed: 2026-09-27
---

# 0314 — Attack spells in attack ranges

## Context

`Unit::attack_ranges` (`crates/core/src/item.rs`, ticket 0306) returns the
ranges of the loadout weapons a unit can wield. Callers pass it to
`threat_area` / `danger_zone` (`crates/core/src/movement.rs`, ticket 0303),
which the danger-zone overlay (0405) and the enemy AI (0501) will use.

Ticket 0309 added innate spells (`crates/core/src/spell.rs`,
`docs/design/magic.md`). A mage with attack spells, and especially a tier-3+
mage with 0 weapon slots, can attack at range 1–2. However,
`attack_ranges` ignores spells, so such a mage would show no threat at all.
0309 left this out because it was outside that ticket's scope.

## Nick input

None. The rule comes straight from `magic.md`: a spell with uses left can
attack, and one at 0 uses can't.

## Scope

**In:**
- `Unit::attack_ranges` also lists the range of every learned attack spell
  the unit can cast now (`Unit::castable_attack`: learned, an attack spell
  in the table, at least 1 use left), with no duplicates.

**Out (do not do):**
- Heal-spell ranges (heals aren't attacks) and tile casts (0310).
- UI overlays (0403/0405) and AI (0501).

## Implementation steps

1. Change the signature to
   `pub fn attack_ranges(&self, classes: &ClassTable, items: &ItemTable, spells: &SpellTable) -> Vec<AttackRange>`.
   Keep the weapon ranges first, in slot order, then append each castable
   attack spell's `(min_range, max_range)` in spell-id order (the order of
   `Unit::learned`), skipping any range already in the list.
2. Update the doc comments in `item.rs` and `movement.rs` (`threat_area`,
   `danger_zone`) that describe where the ranges come from.
3. Update the existing tests in `crates/core/src/item/tests.rs`
   (`attack_ranges_of_usable_weapons`) for the new argument.

## Acceptance criteria

- [ ] A 0-slot mage knowing Fire (1–2) with uses left has attack ranges `[(1, 2)]`; at 0 uses, `[]` (test).
- [ ] Heal spells never add a range; a spell range equal to a weapon's isn't repeated (test).
- [ ] `threat_area` of that mage covers the tiles 1–2 away from its reachable tiles (test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above, in `crates/core/src/item/tests.rs` and
  `crates/core/src/movement/tests.rs`.

## Completion notes

`Unit::attack_ranges` now takes `spells: &SpellTable` and, after the weapon
ranges, appends the `(min_range, max_range)` of every spell in `self.learned`
that `Unit::castable_attack` says the unit can cast now (learned, an attack
spell, ≥1 use left), in `learned`'s id order, skipping ranges already in the
list — exactly as scoped. Heal spells and spells with 0 uses contribute
nothing, matching the acceptance criteria.

Doc comments in `item.rs` (`attack_ranges`) and `movement.rs` (`threat_area`,
`danger_zone`) now describe ranges as coming from weapons *and* castable
spells rather than weapons alone. The only caller of `attack_ranges` outside
its own tests was `item/tests.rs`; no UI or AI code calls it yet (0405/0501
are still open), so no other call sites needed updating.

Tests added:
- `item::tests::attack_ranges_of_castable_attack_spells`
  (`crates/core/src/item/tests.rs`): a 0-slot mage with Fire (1–2) and uses
  left gets `[(1, 2)]`; at 0 uses, `[]`; a learned heal spell never adds a
  range; a spell range equal to an already-listed weapon range isn't
  repeated.
- `movement::tests::threat_area_of_a_mage_covers_spell_range`
  (`crates/core/src/movement/tests.rs`): `threat_area` with a range `(1, 2)`
  threat covers the same diamond as the existing weapon-range tests, since
  `threat_area` takes ranges as plain data and doesn't care whether they came
  from a weapon or a spell.

No design questions or deviations from the ticket's plan. No follow-up
tickets created.

Gates run locally: `cargo fmt --all`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace` (all green), `cargo doc --workspace
--no-deps` (with `RUSTDOCFLAGS=-D warnings`), and `cargo build -p trpg-app
--target wasm32-unknown-unknown`. `cargo deny`/`cargo machete`/`typos` aren't
installed in this environment and this ticket added no dependencies, so
they were skipped (per the ticket board, these gates land after 0103).
Mutation testing runs in CI per `run-gates`.
