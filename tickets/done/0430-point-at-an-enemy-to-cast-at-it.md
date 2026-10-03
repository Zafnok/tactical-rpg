---
id: "0430"
title: Point at an enemy with a caster selected to cast at it
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-10-03
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

Rewritten after Nick's answers (2026-10-03, see Completion notes): no list;
the forecast opens with what is equipped and left / right swap.

- [x] Test: the Quick Battle's mage selected, cursor on the Frost Elemental:
      the path ends two tiles from it; Confirm walks there and the forecast
      of Fire on it is open
      (`harness_pointing_at_the_elemental_walks_there_and_opens_fires_forecast`).
- [x] Test: left / right swap Fire and Frost in that forecast; Cancel goes to
      the action menu (on `Magic`), then the selection, and the battle is
      unchanged (same test).
- [x] Test: a unit with a sword and spells that reach the enemy opens with
      what it has equipped and swaps through all of them
      (`the_pointed_forecast_opens_with_what_is_equipped_and_swaps_through_sword_and_spells`).
- [x] The weapon-only tests of 0428 pass, changed only where the weapon list
      was expected (the forecast opens instead).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `can_hit` and `attack_tile` with a spell-only unit, a spell at 0
  uses, an enemy out of spell range.
- Snapshot / integration: a harness script with the default keys for the
  first criterion.

## Completion notes

**Nick's answers** (asked before building, 2026-10-03; recorded in
`docs/design/turn-structure.md` → *Pointing at an enemy*):

- Several spells reach the enemy: "straight to forecast, for weapons too,
  with keys to quickly go thru potential weapons/spells without backing out
  of forecast screen."
- A sword and a spell both reach: "Whatever is equipped, no list".
- Which keys: "like three houses, but up/down should not pick an art. the
  arts should not be in the same menu. we should have an arts menu. that was
  something i didnt like about the current quick battle setup anyway".

**Done.** Pointing at an enemy now counts attack spells (`attack::can_hit`,
`aimed_options`), so a caster's path aims at it. Confirm walks there and opens
the forecast at once (`Targeting::aimed`, `mode::open_attack`) with the
equipped weapon or spell. Left / right swap through the weapons and attack
spells that reach that enemy (`Targeting::swap`); previous / next unit change
the target. The help bar says `cast` when a spell would open. No `core`
changes.

**Deviations from the ticket as written** (all from Nick's answers):

- No list after pointing, for weapons either: this changes 0428's weapon
  list. Three tests of 0428 were updated for it.
- A spell's forecast opened by pointing is `Mode::Targeting` with the spell,
  not `Mode::CastTarget`: Cancel goes to the action menu on `Magic`, not to
  the spell list (swapping replaces the list).

**Not done here:** the separate Combat Arts menu Nick asked for is ticket
**0442** (it asks him where the menu goes first). Until then up / down still
move the arts list in the forecast.

**Rules I decided** (*Claude's starting rules*, Nick can veto):

- If what is equipped can't reach the enemy from that tile, the forecast
  opens with the first weapon or spell that does (weapons before spells).
  Example: a mage with a sword equipped points at an enemy two tiles away:
  Fire's forecast opens.
- The swap order is weapons in slot order, then attack spells, wrapping
  round: `Iron Sword → Steel Sword → Fire → Frost → Iron Sword`.
- With only one weapon or spell that reaches, left / right change the target
  as before.
- After changing target, a swap keeps that target if the new weapon reaches
  it; if not, the cursor returns to the enemy pointed at.
- A swap goes back to a plain attack (a chosen art or active is dropped).
- Cancel from the forecast goes to the action menu on `Attack`, or on `Magic`
  if it showed a spell.
- The forecast opened from the action menu (`Attack` / `Magic`) has no
  swapping yet; 0442 asks.

**For Nick's sign-off:** in Quick Battle the Test Mage has Fire equipped, so
Fire's forecast opens on the Frost Elemental; press right for Frost (which it
absorbs).

Follow-up tickets: 0442. A note was added to 0429.
