---
id: "0431"
title: Show the burn damage when a fire burns out under a unit
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 0431 — Show the burn damage when a fire burns out under a unit

## Context

Found while working 0410 (spell menu). `docs/design/magic.md`, *Terrain
magic*: nobody can walk onto a burning tile, but a reinforcement arrives on
one anyway, and when the tile burns out the unit on it loses 5 HP (never
below 1). The core emits `Event::BurnDamage { unit, pos, amount }` just
before the tile's `Event::TerrainChanged` (0310,
[`battle.rs`](../../crates/core/src/battle.rs), `burn_out`).

The battle screen ignores the event: the tile flashes (0410's
`TerrainFlash` in [`mod.rs`](../../crates/ui/src/screens/battle/mod.rs)),
but the unit's HP just drops with nothing said. It is rare (it needs a
reinforcement tile set on fire), so it was left out of 0410.

## Nick input

None.

## Scope

**In:**
- A `-5` over the unit when the fire burns it, like the `+N` of a heal.
- A sound, if `docs/design/audio.md` already has one that fits.

**Out (do not do):**
- New sound files.
- Changing the burn rules or amounts.
- Push collision damage.

## Implementation steps

1. `mod.rs`: `HealPopup` shows `+amount` in `hp_high`. Give it a sign (or a
   kind) so `BattleScreen::apply` can also push one for
   `Event::BurnDamage { pos, amount, .. }`, drawn as `-5` in `hp_low`. The
   lifetime stays `TIMINGS.heal_popup`.
2. `event_sounds.rs`: if `audio.md` lists a cue for damage outside combat,
   play it for `BurnDamage` through `event_cues`. If none fits, add no
   sound and say so in the Completion notes.
3. Check the popup doesn't show during the phase banner in a confusing
   order: the burn-out happens at the start of the caster's side's phase,
   with the banner. Starting the popup at once is fine.

## Acceptance criteria

- [ ] Test: a battle with a reinforcement due on a forest, the forest set
      burning the turn before: after the phase starts, the screen has a
      `-5` popup on that tile and the unit has lost 5 HP.
- [ ] Test: the popup is gone after `TIMINGS.heal_popup` seconds.
- [ ] Heal popups look as before (their tests pass unchanged apart from the
      new field).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the popup pushed for `BurnDamage`, its text and colour.
- Snapshot / integration: none needed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
