---
id: "0430"
title: Point at an enemy with a caster selected to cast at it
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 0430 — Point at an enemy with a caster selected to cast at it

## Context

Found while working 0410 (spell menu). Ticket 0428 made pointing at an enemy
with a unit selected walk to a tile it can attack from and open the forecast
(`docs/design/controls.md`; `attack_tile`, `can_hit` and `open_attack` in
[`attack.rs`](../../crates/ui/src/screens/battle/attack.rs) and
[`mode.rs`](../../crates/ui/src/screens/battle/mode.rs)). Those only look at
the three **weapon** slots. A unit that fights with spells (the Quick
Battle's Test Mage, every tier-3 magic class) shows its red attack range
when selected, but pointing at an enemy in it does nothing: the player must
steer to a tile, Confirm, choose `Magic`, the spell, then the target.

Nick's rule from 0428 ("Keep my path, else nearest") already says which tile
to use. The rules of magic are in `docs/design/magic.md`; the cast target
mode is 0410's [`magic.rs`](../../crates/ui/src/screens/battle/magic.rs).

## Nick input

**Sign-off:** in Quick Battle, select the Test Mage, point at the Frost
Elemental and confirm. Say whether the spell it picks and the tile it walks
to feel right.

## Scope

**In:**
- Pointing at an enemy only a spell can reach aims the path, as for weapons.
- Confirm walks there and opens the cast forecast on that enemy.
- With several attack spells (or weapons and spells) that reach it: a list
  to choose from, as the weapon list today.

**Out (do not do):**
- Pointing at a forest or water tile to cast on it (tiles stay in `Magic`).
- Pointing at an ally to heal it.
- AI changes.

## Implementation steps

1. `attack.rs`: make `can_hit` also true when an attack spell with a use
   left reaches `target` from `from` (`art_choices` already takes an
   `Equipped::Spell`). `attack_tile` then works for casters unchanged.
2. `mode.rs`, `open_attack`: collect the weapons **and** attack spells that
   reach `sel.target`. One of them: go straight to its forecast. For a
   spell that is `Mode::CastTarget` on that enemy: build the
   `CastTargeting` for that spell (`spell_choices`, `spell_menu`) and step
   its cursor to the enemy, so Cancel goes back to the spell list as from
   `Magic`. Several: the unit's equipped attack first. If Nick's sign-off
   on 0410 changed how targets are picked, follow that.
3. Help bar in `Mode::Selected` over an aimed enemy: `cast` instead of
   `attack` when only a spell reaches it (`BattleScreen::help` in
   [`mod.rs`](../../crates/ui/src/screens/battle/mod.rs)); key names through
   the keymap (`keyboard-input` skill).
4. If a weapon and a spell both reach the enemy and the choice needs a rule
   the design lacks (which comes first, one list or two), ask Nick with the
   `ask-nick` skill before building it.

## Acceptance criteria

- [ ] Test: the Quick Battle's mage selected, cursor on the Frost Elemental:
      the path ends two tiles from it; Confirm walks there and the forecast
      of Fire on it is open.
- [ ] Test: Cancel from that forecast goes to the spell list, then the
      action menu, then the selection, and the battle is unchanged.
- [ ] Test: a unit with a sword and a spell that both reach the enemy gets a
      list with both.
- [ ] The weapon-only tests of 0428 pass unchanged.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `can_hit` and `attack_tile` with a spell-only unit, a spell at 0
  uses, an enemy out of spell range.
- Snapshot / integration: a harness script with the default keys for the
  first criterion.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
