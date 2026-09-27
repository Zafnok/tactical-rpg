---
id: "0311"
title: "Class skills: passive effects, active skills paid with durability, timed effects"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: done
blocked_by: ["0005", "0014", "0302", "0304", "0305", "0306", "0309"]
nick_input: none
completed: 2026-09-27
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

- [x] `skills.ron` matches `progression.md` (a test checks at least 5 skills, including one per condition type).
- [x] Each tier 1–2 skill has a unit test proving its effect in a forecast or on the state.
- [x] Superseding: White Magic 2 replaces White Magic 1, and learning 1 after 2 changes nothing (test).
- [x] Active availability: current class's active usable at class level 1; after reclassing away from an unmastered class it is gone; after mastery it is usable in any class (tests).
- [x] Using an active spends its durability cost from the right weapon (attacking vs equipped); with a broken weapon or `durability_left < cost` it can't be chosen (state unchanged); a weapon brought to exactly 0 breaks after the action (tests).
- [x] Overcast/Siphon spend 2 spell uses and are refused at 1 use left (test).
- [x] Timed effects expire at the right phase boundary, and refresh instead of stacking.
- [x] "+1 strike" never gives more than 4 strikes (property test).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each skill; superseding; uses; timed effects; Shove blocked and into
  `burning`.
- Property: extend 0305's random-command test with `UseSkill` and attack
  actives. HP stays in `0..=max`, and durability and spell uses never
  underflow.

## Completion notes

- **Done.** `assets/data/skills.ron` holds every tier 1–2 skill of
  `progression.md` plus the Flier's Swoop and Sky Dodge 1 (the class file
  already names them), 48 in all, with `combat-arts.md`'s durability costs.
  `trpg_content::skill` loads and validates it (unique ids, one skill per
  family rank, durability ≥ 1, `ExtraSpellUse` exactly on spell actives,
  radii ≥ 1, every class active/passive exists and is the right kind).
  `core::skill` has the data types, `Unit::learn_skill` (superseding),
  `Unit::usable_skills`/`usable_active`, the bonus gathering, the timed
  effects (`Unit::effects`) and `check_cost`/`pay_cost` for 0312.
  `battle/skills.rs` wires it into commands: `UnitAction::Attack { active,
  then_move }`, `UnitAction::Cast { active }` (spell actives),
  `UnitAction::UseSkill { skill, target }`; events `SkillUsed`,
  `DurabilitySpent`, `EffectApplied`, `EffectExpired`, `Pushed` (plus
  `Healed`, `UnitMoved`, `ItemBroke`). `BattleSetup`/`restore_tables` take
  the skill table. ADR-0021 records the approach.
- **Deviations:**
  - `CombatantInput` gained a `mods: CombatMods` field and `combat.rs`
    applies it (might, hit, crit, avoid, attack speed, extra strikes, one
    strike only, doubled crit, ignore terrain, pierce). With default mods
    every formula is unchanged (the 0304 worked examples still pass), but
    Heavy Blow, Deadly Blow, Trample and Piercing Lance can't be expressed
    through stats alone, so the formulas needed these hooks.
  - `resolve` now alternates strikes 2..N when **both** sides have some
    (only possible with "+1 strike" skills); before, only one side could.
  - `SkillDef.kind` is `Passive(Vec<PassiveEffect>)` (Resolve and Bow Focus
    have two effects) and has no stored `combat: bool`: a combat active is
    the `Strike` effect (`SkillDef::is_combat`). Spell actives use
    `UnitAction::Cast`'s new `active`, since attack spells are cast, not
    `Attack`ed.
  - The post-action move (Vault, Swoop) is its own command, chosen after
    the combat (Nick; see the reviews below), not a field of the attack as
    0305's hook intended.
  - Shove's landing after a burning tile is the first free neighbour of the
    fire in `Dir` order: the target's own tile is always one, so "nearest"
    never goes further than 1.
- **Step 7 (EXP/CP):** 0601 hasn't landed; `Event::SkillUsed` is the hook
  for `exp_for_active_skill` (0601 already lists it). Who calls
  `learn_skill` on mastery is 0601 (`SkillLearned`).
- **Tests:** `core::skill` unit tests (conditions, bonuses, costs,
  superseding, usable skills after reclass and mastery, timed effects);
  `combat` tests for every mod plus a property test (+1 strike never above
  4); `battle/tests/skill.rs` (every active and passive kind, refusals leave
  the state unchanged, weapon broken after the action, phase boundaries,
  Shove blocked and into fire, auras); the random-play property test now
  makes skill commands (attack actives, spell actives, moves after,
  `UseSkill`) and checks durability and spell uses never underflow or move
  without their events; the replay/save-load test uses an active with a
  stance. `trpg-content`'s `tests/skills.rs` has one test per real skill,
  in a battle built from the real data.
- **No follow-up tickets.**
- **Nick's review (2026-09-27)** changed two starting rules, now recorded in
  `progression.md` (and `magic.md` for the push):
  - "I think the might + N / damage + N should be multiplied / halved": a
    skill's might / damage bonus is added to the weapon's might, so
    effectiveness (and Weak) multiplies it and a broken weapon halves it.
  - "Shove should not be refused but be used to do collision damage": a
    Shove into a blocked tile (off the map, a unit, impassable terrain)
    goes ahead; the enemy stays put and takes the skill's collision damage
    (5, data: `Push(collision: 5)`), the same as a push into fire.
    `CommandError::PushBlocked` is gone.
  - Second review: "I think Shove can kill, and I think it can cause a
    collateral damage between 2 units": collisions can take units to 0 HP,
    and a unit the target is pushed into takes the same damage
    (`Event::CollisionDamage`). Both can fall, the pushed unit first.
  - Third review: Swoop's (and Skirmish's, now Vault's) step is **chosen
    after the fight plays out**: the attack ends with `Event::MoveAfterOffered`, and a new
    `Command::MoveAfter { unit, to }` makes (or skips) the step; the old
    `then_move` field is gone (ADR-0021 updated). Stance riders say whether
    they also count in their own combat (`Stance { mods, this_combat }`).
    White Magic's bonus also adds to Sanctuary. Charge, Deadly Blow (doubled
    last) and skill stat bonuses were confirmed as they were.
  - Fourth review: "i think skirmish should be renamed vault and it should
    be an active skill", then "1B, 2B - give the players a bit of fun
    here..": **Vault** (1 dur, bows only, a 1-tile step after the fight) is
    the Archer's active; Long Shot moved to the Marksman with range +2 (Long
    Shot 2 is gone); the Archer's passive is Bow Focus 1 (hit +5), which the
    Outrider learns where it learned Skirmish; the Marksman's passive is
    Bow Focus 2. "3A with exception that deadly blow goes to sword only,
    heavy blow goes to gauntlet only, and trample goes to axe only": every
    combat active has a weapon kind (Keen Edge Sw, Flurry Gt, Lance Rush Sp,
    Deadly Blow Sw, Rampage Ax, Trample Ax, Swoop Sp). Heavy Blow became Ax,
    not Gt, because the Raider wields only axes ("fine A is ok.. I'm just
    worried gauntlets won't have enough skills and axe too many").
- **Claude's starting rules** (gameplay, where the design docs were silent):
  1. ~~Skill might added after effectiveness~~: **changed by Nick** (see
     below).
  2. When both sides get extra strikes (only via "+1 strike" skills), the
     strikes after the first two **alternate, attacker first** (confirmed
     by Nick, recorded in `stats-and-combat.md`).
  3. Area actives (War Cry, Inspire, Rally, Sanctuary) affect **other**
     allies only, never the user, and green (Ally) units count as allies.
     They **can't be used if nobody is in reach** (Sanctuary: nobody
     wounded), so durability is never wasted.
  4. Leadership auras from two leaders don't stack (each aura skill counts
     once); the lord never gets its own aura.
  5. ~~Shove collisions never kill~~: **changed by Nick** (see below).
     Pushed into fire, the target lands on the first free tile next to it.
  6. **Vault/Swoop**: one step to an empty tile the unit can enter,
     whatever its move cost (chosen after the fight: Nick).
  7. ~~A stance rider always counts in its own combat~~: **per skill**
     (Nick).
  8. ~~White Magic is for heal spells only~~: **it counts for Sanctuary**
     (Nick).
  9. "Moved ≥ 4 tiles" (Charge) counts tiles walked this action, not move
     cost; counters never charge.
  10. Fury/Resolve's "HP ≤ 50%" means HP × 2 ≤ max HP at the start of the
      combat.
  11. Deadly Blow doubles the crit chance **after** the other crit bonuses,
      then clamps to 100.
  12. Piercing Lance lowers the target's Def by 5 (not below 0); terrain Def
      still counts. Trample removes the target's terrain Def **and** avoid.
  13. Siphon heals half the HP its strikes actually removed (a target with
      2 HP left gives 1).
  14. Skill stat bonuses (Brace, War Cry, Resolve…) count in combat only
      and may go past class caps.
  15. ~~Actives without a weapon kind work with any weapon~~: **every
      combat active has a kind** (Nick).
  16. Bow Focus 1 is hit +5 (half of Bow Focus 2's hit +10), and the
      Marksman's Long Shot keeps Long Shot 2's range +2.
