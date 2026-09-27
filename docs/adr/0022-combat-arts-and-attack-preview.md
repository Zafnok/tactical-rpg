# ADR-0022: Combat Arts as weapon-input changes, and the attack preview

- **Status:** Accepted
- **Date:** 2026-09-27
- **Related tickets:** 0312, 0414, 0503, 0018

## Context

Ticket 0312 adds Combat Arts (`docs/design/combat-arts.md`): weapon
techniques learned by weapon rank, paid with durability, boosting every
strike of one attack. Their effects are close to class skills' (ADR-0021):
hit bonuses and changed effectiveness. But some touch the weapon itself
(the sword follow-up ratio, the axe minimum, the bow's minimum range), some
act outside the combat maths (no counter, a debuff after the first hit, a
stance, Line Pierce's extra strike at another unit), and debuffs last until
the end of the *target's* next phase, not the user's. The UI (0414) must
show the art's numbers, cost and non-number effects before the player
commits, and the boss AI (0503) must score arts. Neither may compute
numbers itself (ADR-0004).

## Decision

1. **Arts are data** in `assets/data/arts.ron`, loaded by `trpg-content`
   into `core::art::ArtTable` and given to the battle like the other
   tables (`BattleSetup::arts`, `restore_tables`). An `ArtDef` has a kind,
   a rank (`None` = a weapon art, listed by a weapon's `arts`), a cost and
   one flat `ArtEffect` struct of optional fields. There are no variants or
   scripts: a new kind of art effect is a new field plus its rule and tests.
2. **An art changes only the attacker's inputs.** `ArtEffect::apply` edits
   the attacker's `WeaponStats` (effectiveness, axe minimum, minimum range)
   and `CombatMods` (hit, `sword_followup`); `combat.rs` knows nothing
   about arts. What isn't a combat input is the battle's: "no counter"
   drops the forecast's defender side, and Line Pierce's strike is a second
   forecast worked out when the attack is validated (one strike, no
   counter) and resolved as its own `CombatResolved`.
3. **Timed effects are keyed by `EffectSource`** (`Skill(SkillId)` or
   `Art(ArtId)`), in `TimedEffect` and in `EffectApplied`/`EffectExpired`.
   A debuff is a timed effect with negative stat bonuses. A negative bonus
   never takes a stat below 0 (`Bonuses::apply`). A timed effect's Mov also
   counts for movement (`Unit::move_points`). "Until the end of phase X" is
   stored as "until the start of X's next phase", which is the same moment
   because every phase slot is started in turn, even a skipped one.
4. **`BattleState::preview_attack(unit, dest, action)`** returns an
   `AttackPreview` (forecast, chosen art or active, durability before and
   after, the art's `ArtNote`s, the pierce's numbers). It runs the same
   validation as `apply`, with the same errors and no state change. The UI
   and the AI use it instead of building combat inputs themselves.
5. **`Unit::boss`** (saved, default `false`) marks the only non-player
   units allowed to use arts and actives (`CommandError::NotABoss`).

## Consequences

- Retuning an art, or adding one of an existing shape (0018's higher
  ranks, special weapons), is a data edit validated at load.
- Forecast and resolution can't disagree: both come from the one validated
  plan. The property test checks strike counts against every forecast.
- Every `UnitAction::Attack` literal names `art` (`art: None`), and every
  `EffectApplied`/`EffectExpired` match uses `source`.
- Line Pierce's numbers are fixed at validation, with the attacker's HP
  going in; a skill condition on the attacker's HP would see the HP from
  before the main combat.
- 0601's unit EXP/CP can award per `CombatResolved`, which gives the pierce
  its own award as the design asks.

## Alternatives considered

- **Art checks inside `combat.rs`** (an `art: Option<&ArtDef>` input to
  `forecast`, as 0312's text first suggested). The combat maths would learn
  about arts, breaking ADR-0021's "combat sees only numbers", and every
  `forecast` call would change.
- **`no_counter` as a `CombatMods` flag.** It worked, but it is not a
  combat number, and a fourth flag trips clippy's `struct_excessive_bools`.
- **A separate `Expiry` kind (start vs end of a phase)** on timed effects.
  It is equivalent to storing the next phase's start, and it would add a
  second expiry path to test.
- **A UI-side forecast** from `combat::forecast`. The UI would have to
  rebuild skill, aura, art and terrain inputs, duplicating the battle's
  rules.
