---
id: "0503"
title: "Boss and green AI: choosing Combat Arts and active skills"
type: feature
milestone: M4 Enemy AI
model: opus-5.5
effort: medium
status: done
blocked_by: ["0501", "0311", "0312"]
nick_input: none
completed: 2026-09-30
---

# 0503 — Boss AI uses arts and actives

## Context

Nick decided (ticket 0014, `docs/design/combat-arts.md` → *Enemies: bosses
only*) that among enemies **only bosses** use Combat Arts and active skills.
They follow the player's rules and pay with their own weapon's durability.
In 0312's review Nick added that **combat green units** use them too
("so long as they're combat green units... not like villagers you have to
protect or wild beasts"); non-combat green units have
`role: Noncombatant` in chapter data. 0501's AI never uses them; this ticket lets it do so for
the units `core` allows (`BattleState` refuses the others with
`CommandError::ArtsNotAllowed`): bosses (`role: Boss`, 0801) and green
units that aren't `role: Noncombatant`. The AI stays pure `core` code issuing
ordinary `Command`s ([ADR-0004](../../docs/adr/0004-crate-architecture.md)).

## Nick input

None. How dangerous bosses feel is judged in the playtest (0804).

## Scope

**In:**
- In 0501's attack scoring, for boss units and combat green units, also
  score each usable art and
  combat active (0312/0311 `usable_arts`, usable actives) with the same
  `forecast`-based score.
- Non-combat actives for bosses (e.g. Brace): use one instead of waiting when
  the boss can't attack this turn and it is in a hostile unit's threat area.
- Deterministic tie-breaks.

**Out (do not do):**
- Ordinary enemies and non-combat green units using arts or actives
  (never).
- UI (0414 shows the names during playback).

## Implementation steps

1. In `core::ai`, when the acting unit may use arts (a boss, or a combat
   green unit), extend the candidate list
   `(dest, target, weapon)` with `(dest, target, weapon, Option<ArtOrActive>)`.
   Score with the same formula, plus a durability penalty
   `w_dur * cost` (new weight in `ai.ron`, *tunable*) so bosses don't burn
   durability on attacks that barely change the outcome.
2. Tie-break: plain attack before arts, then by art/skill id.
3. Non-combat active rule (step "In" above), as the `Stationary`/`Guard`
   behaviours' fallback.
4. `debug_assert!` that every emitted command is valid, as in 0501.

## Acceptance criteria

- [x] A boss with Guard Break in reach of a unit it can only kill without a counter picks Guard Break (test).
- [x] A boss never picks an art it can't pay for; an ordinary enemy never picks one (tests).
- [x] A combat green unit picks arts like a boss; a `Noncombatant` green unit never does (tests).
- [x] Same state → same command (determinism test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cases above.
- Property: extend 0501's random-state AI test with a boss; every emitted command is valid.

## Completion notes

**What was built** (all in `crates/core/src/ai.rs`; the rules are in its
module docs):

- **Arts and combat actives in the attack choice.** For a unit the battle
  lets use them (`Unit::may_use_arts`: bosses, combat green units, and
  player units when the AI plays them), every attack 0501 scores is also
  scored with each art it can pay for with that weapon and each of its
  combat actives. The numbers come from the battle itself: a new
  crate-internal `BattleState::attack_forecast` validates the attack exactly
  as an `Act` is validated and returns its forecast, so range changes (Long
  Shot, Close Shot), Guard Break's missing counter and every refusal (wrong
  weapon, can't pay, not allowed) are the battle's own (ADR-0022).
- **Cost penalty:** two new weights in `assets/data/ai.ron`,
  `durability: 5` (per point of durability) and `spell_use: 15` (the extra
  spell use of a spell active such as Overcast).
- **Tie-break:** after 0501's `(target, tile, weapon, slot)`: the plain
  attack, then the lowest art or active id.
- **Non-combat actives** as the fallback of `Guard` and `Stationary` units.
- `Unit::may_use_arts` is the one place that says who may use arts; the
  battle's `ArtsNotAllowed` check now calls it.
- Tests: 12 new scenario tests in `crates/core/src/ai/tests.rs` (the
  acceptance cases, the cost thresholds, ties, range, Line Pierce, Brace),
  and 0501's property test now draws bosses and non-combat units with
  random arts, actives and worn weapons.

**Measured:** 0501's timing case (20 enemies, 30×30) plans in 5.5 ms in
release (budget 50 ms). It has no boss; a boss only adds a few forecasts per
attack it considers.

**Deviations:**

- Step 1's candidate tuple isn't a type of its own: each weapon or spell
  carries the list of techniques to try with it.
- A spell active costs a spell use, not durability, so it has its own weight
  (`spell_use`) beside the ticket's `w_dur`.
- Line Pierce's extra strike is added to the score (the ticket only says
  "the same forecast-based score"); without it the art could never be worth
  its cost.
- The non-combat rule applies to every unit that may use arts (combat green
  units too), not only bosses, as the rest of the ticket does.

**Claude's starting rules** (gameplay the design docs don't settle; all
*tunable*, judged in playtest 0804; also in `combat-arts.md` → *Enemies*):

1. **An art or active must pay for itself.** Using one counts 5 points per
   durability against the attack (1 damage dealt is worth 10, 1 damage
   taken 5, a kill 300). Example: a boss at full HP facing a swordsman uses
   Guard Break (4 durability = 20 points) to skip a counter of 5 damage
   (25 points); against an archer next to it, who can't counter, it attacks
   normally. A spell active's extra spell use counts 15 points.
2. **Only what the forecast shows counts.** Pinning or slowing the target,
   a stance for the enemy's turn, a move after the attack and drain score
   nothing, so a boss never picks an art for those alone (a boss archer
   doesn't use Pinning Shot). Follow-up ticket 0511.
3. **Line Pierce** counts the hit on the unit behind the target as if the
   boss surely survives the first fight.
4. **Holding bosses brace.** A boss (or combat green unit) set to guard or
   never move, with nothing to attack, uses its non-attack active (Brace,
   War Cry, Sanctuary, Shove) instead of waiting, whenever a hostile unit
   could attack it this turn. Example: the party walks into range of a
   Guard-class boss on its fort; on its turn the boss Braces.
5. **It keeps its weapon's last durability:** it doesn't use a non-attack
   active that would break its equipped weapon. It may still break a weapon
   with an art or combat active when that attack is worth it.

**For Nick when playing:** rule 4 means a boss with Brace spends 3 of its
weapon's 20 durability every turn you stand in range without it being able
to reach you, so hovering in range wears its weapon down to 1–2. Say in the
playtest (0804) if that feels like an exploit; a durability reserve would
fix it.

**Follow-ups:** 0511 (weigh debuffs, stances, moves after attacking and
drain; only if the playtest asks for it).
