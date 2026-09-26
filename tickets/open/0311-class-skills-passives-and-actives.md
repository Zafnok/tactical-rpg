---
id: "0311"
title: "Class skills: passive effects, active skills paid with durability, timed effects"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: todo
blocked_by: ["0005", "0014", "0302", "0304", "0305", "0306", "0309"]
nick_input: none
completed:
---

# 0311 — Class skills: passives, actives, timed effects

## Context

Ticket 0005 (`docs/design/progression.md`, *Skills*) decided that unlocking
a class gives its **active** skill, and mastering it teaches its
**passive** skills. Passives are kept for good. An active is usable only while
in its class, until that class is mastered; after that it is permanent. A
higher rank of a skill family replaces the lower one. 0302 stores skill ids on classes, and 0601 records
`SkillLearned`. This ticket gives the skills their effects in `core`,
through `Command` → `Event`s
([ADR-0004](../../docs/adr/0004-crate-architecture.md)). The combat
formulas it hooks into are in `stats-and-combat.md`,
`weapons-and-items.md` and `magic.md`, and in 0304's forecast/resolve.
Ticket 0014 (`docs/design/combat-arts.md`) changed what actives cost: **weapon
durability** (or 1 extra spell use for spell actives) instead of uses per
battle; the costs are in `progression.md`'s skill table.

## Nick input

None. All numbers are in `progression.md`'s tier 1–2 skill table.

## Scope

**In:**
- `assets/data/skills.ron` holding every tier 1–2 skill in
  `progression.md`.
- `core::skill`.
- A unit's learned skills (`learned_skills: BTreeSet<SkillId>`) and family
  superseding.
- Passive modifiers applied in the forecast.
- Active skills paid with durability (`combat-arts.md`, *Class actives now
  cost durability*): combat actives as an option on `Attack` (attacking
  weapon), non-combat actives as a `UnitAction` (equipped weapon), spell
  actives costing 1 extra spell use.
- Timed effects "until the start of the unit's next phase".
- Skirmish/Swoop post-action 1-tile move, via 0305's post-action move hook
  (`turn-structure.md`).
- Shove, using the push rule in `magic.md`.
- The lord's skills (ticket 0016): **ally auras** (Leadership 1/2: a passive
  that gives *other* allies within 2 tiles of the lord a bonus) and
  area buffs (Inspire, Rally: allies within 2 tiles, timed until the start
  of the lord's next phase).

**Out (do not do):**
- UI (0412).
- AI use of actives (only bosses use them, ticket 0503; 0501 may use
  passives automatically, since they are always on).
- Tier-3 skills (1001).
- Combat Arts themselves (0312), though the cost-paying helper written here is shared with them.

## Implementation steps

1. `core::skill`:
   - `SkillId`
   - `SkillDef { id, name, family, rank, kind: Passive(PassiveEffect) | Active { cost: SkillCost, effect: ActiveEffect, combat: bool } }`
   - `SkillCost` = `Durability(n)` | `ExtraSpellUse` (`combat-arts.md`).
   - `PassiveEffect` is a small enum covering the table, e.g.
     `StatWhile { stat, amount, condition }`,
     `CombatMod { hit, crit, might, avoid, attack_speed, condition }`,
     `HealBonus(n)`, `SpellMight(n)`, `PostActionMove(n)`,
     `AllyAura { radius, mods }` (the lord's Leadership: applies to other
     allied units within `radius` tiles of the skill's owner).
   - Conditions: `WeaponKindEquipped(kind)`, `NotOwnPhase`,
     `HpAtMostHalf`, `MovedAtLeast(n)`, `AgainstWeaponKind(kind)`,
     `Always`.
2. `skills.ron` and its validation in `content`:
   - ids are unique;
   - a family's ranks are unique;
   - every class `passives`/`active` id exists;
   - durability costs are `≥ 1`; `ExtraSpellUse` only on spell actives.
3. `Unit::usable_skills(class_table)` = learned passives + the current
   class's active + the actives of every **mastered** class (from
   `class_records`), keeping the highest rank per family. An unmastered
   class's active is not usable after a reclass away from it (test). `learn_skill()` follows the superseding rules,
   including "learning a lower rank does nothing".
4. Forecast integration: gather the passive and chosen-active modifiers of
   both sides into 0304's `CombatantInput` before its formulas run. Keep the
   formulas themselves unchanged. "+1 strike" is added after the
   attack-speed strike count and clamped to 4. Combat actives are chosen
   only when attacking. A combat active may carry a **stance rider**: a
   timed effect applied when it is used, which lasts until the unit's next
   phase, so it also applies to counters in the enemy phase
   (`progression.md`).
5. Paying (`combat-arts.md`, *Using an art* rules 3–5, which apply to
   actives too): a durability cost needs an unbroken weapon with
   `durability_left ≥ cost` (the attacking weapon for combat actives, the
   equipped one for non-combat actives) and is paid once when the action is
   committed, via 0306's `spend_durability`; if that reaches 0 the weapon
   breaks **after** the action. `ExtraSpellUse` needs the spell's
   `uses_left ≥ 2` and spends 2 uses. Put the check-and-pay in one helper
   (`core::skill::pay_cost`) so 0312 reuses it for arts.
   `UnitAction::Attack { …, active: Option<SkillId> }` and
   `UnitAction::UseSkill { skill, target }` (Brace, Fortify, War Cry,
   Sanctuary 1/2, Shove). Events: `SkillUsed`, `DurabilitySpent`,
   `EffectApplied`, `EffectExpired`, `Healed`, `Pushed`.
6. Timed effects: `effects: Vec<TimedEffect { source, stat mods, owner_side }>`
   on the battle unit. They expire at the start of the owner side's next
   phase, before anyone acts. Using the same effect again refreshes it
   instead of stacking.
7. Non-combat actives give EXP/CP through 0601's `exp_for_active_skill`
   (if 0601 has landed; otherwise emit the event and note the follow-up).

## Acceptance criteria

- [ ] `skills.ron` matches `progression.md` (a test checks at least 5 skills, including one per condition type).
- [ ] Each tier 1–2 skill has a unit test proving its effect in a forecast or on the state.
- [ ] Superseding: White Magic 2 replaces White Magic 1, and learning 1 after 2 changes nothing (test).
- [ ] Active availability: current class's active usable at class level 1; after reclassing away from an unmastered class it is gone; after mastery it is usable in any class (tests).
- [ ] Using an active spends its durability cost from the right weapon (attacking vs equipped); with a broken weapon or `durability_left < cost` it can't be chosen (state unchanged); a weapon brought to exactly 0 breaks after the action (tests).
- [ ] Overcast/Siphon spend 2 spell uses and are refused at 1 use left (test).
- [ ] Timed effects expire at the right phase boundary, and refresh instead of stacking.
- [ ] "+1 strike" never gives more than 4 strikes (property test).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each skill; superseding; uses; timed effects; Shove blocked and into
  `burning`.
- Property: extend 0305's random-command test with `UseSkill` and attack
  actives. HP stays in `0..=max`, and durability and spell uses never
  underflow.

## Completion notes
