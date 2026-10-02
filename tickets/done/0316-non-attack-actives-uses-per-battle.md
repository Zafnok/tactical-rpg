---
id: "0316"
title: "Non-attack actives cost uses per battle, not durability; Benediction replaces Sanctuary 2"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: done
blocked_by: ["0311", "0412", "0503"]
nick_input: sign-off
completed: 2026-10-01
---

# 0316 — Non-attack actives cost uses per battle

## Context

Reviewing ticket 0503 (PR #131, 2026-10-01) Nick changed what the
**non-attack actives** cost: "Flavor-wise I don't see why we give any of
these dur costs. They should all be per-battle uses... this should count for
both the player and the boss." Actives that swing a weapon keep costing
durability. He also made Sanctuary 2 a skill of its own, so a promoted
Cleric keeps Sanctuary.

The decision, with Nick's words, the rules and the table of uses, is in
`docs/design/combat-arts.md` → *Non-attack actives: uses per battle*
(and `progression.md` → *Superseding*). Read it first: this ticket
implements exactly that.

Today (ticket 0311) a non-attack active pays `SkillCost::Durability(n)` from
the **equipped** weapon (`BattleState::plan_skill`,
`crates/core/src/battle/skills.rs`), and the boss AI (0503,
`Planner::stand` in `crates/core/src/ai.rs`) refuses to spend the weapon's
last durability on one.

## Nick input

**Sign-off** (after merge, on the Pages build): use Brace, Shove and
Sanctuary in a battle and check the uses count down and come back next
battle; say if any number of uses feels wrong (they are *tunable*).

## Scope

**In:**
- A third skill cost, uses per battle, for the seven non-attack actives and
  the new Benediction; their uses tracked per unit and refilled each battle.
- Data: `assets/data/skills.ron` and `classes.ron` (Priest).
- The skill menu shows uses left instead of a durability cost.
- The boss AI's rule: "has a use left" replaces "keeps its weapon's last
  durability".
- Design docs: drop the "until ticket 0316" notes.

**Out (do not do):**
- Combat actives (Keen Edge, Long Shot…) and Combat Arts: they keep costing
  durability. Spell actives keep costing a spell use.
- New skills other than Benediction, or changes to what any active does.
- Uses that carry over between battles, or any way to restore them
  mid-battle.
- Renaming Benediction: it is a working name; Nick may rename it later
  (names stay data, `skills.ron`).

## Implementation steps

1. **Cost** (`crates/core/src/skill.rs`): add `SkillCost::Uses(u8)` (the
   uses per battle). `check_cost`/`pay_cost` get a third `CostSource`
   (e.g. `CostSource::Own(SkillId)`): it needs `uses_left ≥ 1` and spends
   one; a new `CostError` says "no uses left". Document it in the module
   docs.
2. **State** (`crates/core/src/unit.rs`): a per-unit map of skill uses left
   this battle, next to `spells: SpellState`, with `#[serde(default)]` so
   old saves load. Fill it for every unit when the battle starts
   (`BattleState::new`, and for reinforcements when they arrive), the way
   spell uses are filled: every usable active with a `Uses(n)` cost starts
   at `n`. A skill missing from the map has 0 uses.
3. **Battle** (`crates/core/src/battle/skills.rs`, `plan_skill`): a
   `Uses` cost is paid from the skill's own uses; **no equipped weapon is
   needed**. A `Durability` cost on a non-attack active keeps working as
   today (data may still use it). A combat active with a `Uses` cost is a
   content error (step 5). Emit an event with the uses left (like
   `Event::SpellUsesChanged`) so the UI and the history/rewind code
   (`crates/core/src/history.rs`) see it; rewinding restores the uses.
4. **Data** (`assets/data/skills.ron`): Shove `Uses(8)`, Brace `Uses(3)`,
   Sanctuary `Uses(3)`, Fortify `Uses(2)`, War Cry `Uses(2)`, Inspire
   `Uses(2)`, Rally `Uses(1)`. Replace `sanctuary_2` with `benediction`
   ("Benediction", its own family, `Uses(1)`, `Heal(radius: 2, power: 5)`).
   `classes.ron`: the Priest's active becomes `benediction`. A Priest
   promoted from a mastered Cleric then has both (existing rule,
   `progression.md` → *Actives, by class*); check it in a test.
5. **Content validation** (`crates/content/src/skill/`): `Uses(0)` and a
   combat or spell active with `Uses` are errors with their position.
   Also enforce Nick's rank rule (`progression.md` → *Superseding*: a
   higher rank is the same skill with bigger numbers, nothing else). For
   every family with more than one rank, comparing each rank with the one
   below it, it is an error (naming both skills) unless:
   - both are passives with the same number of effects, or both actives;
   - each pair of effects is the same variant with the same non-number
     fields: `when`, `stat`, `with`, `area` and every `radius` (a reach is
     a fundamental, not a number to raise), the same flags (`single_strike`,
     `double_crit`, `ignore_terrain`, `drain`, `this_combat`) and the same
     cost kind;
   - every number (stat amounts, `CombatMods` numbers, `power`, `collision`,
     `tiles`, `range`, `post_move`, uses) is at least the lower rank's, and
     at least one is higher. A cost in durability may be anything. A
     `CombatMods` number going from 0 to more is allowed: Nick kept Bow
     Focus 2 (adds crit +5) and Leadership 2 (adds avoid +10) as
     replacements; both must pass.
   With `sanctuary_2` gone (step 4) the shipped data must pass.
6. **UI** (`crates/ui/src/screens/battle/skills.rs`, `art_list.rs`): where a
   non-attack active shows `3 dur  Wpn 20/20` it shows its uses left and
   maximum the way a spell's uses are shown; with none left it is dimmed
   with the reason. Follow the `keyboard-input` skill (no key names
   change). Update snapshots; read each new snapshot.
7. **AI** (`crates/core/src/ai.rs`, `Planner::stand`): remove the
   equipped-weapon and "cost < durability left" checks; the battle's own
   check (`check_action`) refuses an active with no use left. Update the
   module docs and the tests in `crates/core/src/ai/tests.rs` (Brace with
   uses left → braces; none left → waits).
8. **Docs:** in `combat-arts.md` and `progression.md` remove the "until
   ticket 0316" notes and the durability rows of the non-attack actives;
   update the skills table in `progression.md` (`3 uses, action` instead of
   `3 dur, action`). If ticket 0429 is still open, fix its example line
   (`Brace  3 dur  Wpn 20/20`).

## Acceptance criteria

- [x] A Grappler starts a battle with 8 Shoves; each Shove spends one, the 9th is refused, and the next battle starts with 8 again (tests).
- [x] A unit with no weapon equipped, or a broken one, can use Brace; using it changes no weapon's durability (tests).
- [x] A Priest promoted from a mastered Cleric has Sanctuary (3 uses, adjacent) and Benediction (1 use, 2 tiles), and White Magic adds to both (test).
- [x] Keen Edge and Guard Break still cost durability; Overcast still costs a spell use (existing tests pass unchanged).
- [x] Rewinding past a use gives it back (test).
- [x] The skill menu shows the uses left; an active with none left is dimmed with the reason (snapshots).
- [x] A boss holding its tile braces while it has a use left and waits once it has none (tests); ordinary enemies still never do.
- [x] Old saves load (the new field defaults).
- [x] Content validation refuses a rank 2 that differs from its rank 1 in anything but bigger numbers: one test per case (a wider radius, another condition, another effect kind, a lower number, no number higher); `assets/data/skills.ron` passes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `core::skill` cost rules; battle tests for each acceptance case;
  AI scenario tests; content validation errors.
- Property: the existing random-battle tests (`crates/core/src/battle/tests.rs`,
  `crates/core/src/ai/tests.rs`) keep passing: uses never go below 0 and
  every listed legal command is accepted.
- Snapshot / integration: the skill menu with uses left and with none.

## Completion notes

**What was done**

- **Cost** (`core::skill`): `SkillCost::Uses(n)`, paid from
  `CostSource::Own(skill)`; `CostError::NoUsesLeft`. A unit's uses left are
  `Unit::skill_uses` (`SkillUses`, `#[serde(default)]`), filled by
  `Unit::prepare_for_battle` for every unit and reinforcement from its usable
  actives. Spending one emits `Event::SkillUsesChanged`.
- **Battle** (`plan_skill`): a `Uses` cost needs no weapon and spends no
  durability; a `Durability` cost on a non-attack active still works (paid
  from the equipped weapon). Rewind replays the battle, so a rewound use
  comes back.
- **Data**: Shove 8, Brace 3, Sanctuary 3, Fortify 2, War Cry 2, Inspire 2,
  Rally 1. `sanctuary_2` is gone; the Priest's active is `benediction`
  (1 use, heals allies within 2 tiles), a skill of its own.
- **Content validation**: `Uses(0)` and `Uses` on a combat or spell active
  are errors. The rank rule is in `crates/content/src/skill/ranks.rs`: each
  rank is compared with the rank below it in its family; the shipped data
  passes (Bow Focus 2 and Leadership 2 included).
- **UI**: the Skill menu and the info screen show `Brace  2/3`. A skill
  with no uses left is dimmed with `no uses left`. Picking who to Shove
  shows `Shove on Brigand (8 → 7 uses)`.
- **AI**: `Planner::stand` no longer looks at the weapon; the battle refuses
  an active with no use left, so a boss braces while it has a use and then
  waits.
- **Docs**: `combat-arts.md`, `progression.md`, `docs/design/README.md` and
  ticket 0429's example line.

**Deviations**

- `plan_skill` now checks the cost **before** the skill's targets, so a
  skill with no uses left says so whoever stands near (the menu's reason
  comes from the core this way). Only the error a doubly-wrong command
  gets changed.
- The validation module is `crates/content/src/skill.rs` plus the new
  `skill/ranks.rs` (the ticket named a `skill/` directory).
- `ROADMAP.md` still lists 0316 on row 1: that table is redone at each
  dependency check, not per ticket.

**Rules Claude had to decide** (*Claude's starting rules*; Nick may veto)

1. **A skill learned in the middle of a battle has no uses until the next
   battle** (like a spell learned in battle). Nothing in the game teaches an
   active mid-battle yet, so today this never happens.
2. **What the screens say**: `Brace  2/3` (uses left / uses per battle),
   `no uses left` after a dimmed skill, `(8 → 7 uses)` when picking who to
   Shove.
3. **A unit whose only non-attack skill is used up** sees `Skill` greyed out
   in its action menu (as it already was for a skill it can't use now); the
   `no uses left` reason shows in the skill list when the unit has another
   skill it can still use. The info screen always shows `Brace  0/3`.
4. **The rank check, where Nick's rule left room**: a rank 2 may add a stat
   to a buff the way Bow Focus 2 adds crit (a missing number counts as 0);
   a stance that only one rank has is a difference; "after moving 4 tiles"
   vs "after moving 3" counts as another condition, not a number; a
   durability cost is neither a "higher number" nor a lower one.
5. **A boss doesn't save its uses**: it braces on every turn it is
   threatened while it has one (3 turns for Brace).

**Notes for Nick (sign-off, on the Pages build after merge)**

- Guard: Brace 3 times in one battle, then `Skill` is greyed out; next
  battle it has 3 again. Grappler: 8 Shoves. Cleric: 3 Sanctuaries.
- None of them touch the weapon's durability any more, and they work with
  a broken weapon or none.
- The numbers of uses are *tunable* in `assets/data/skills.ron`. Benediction
  is still a working name.

**Follow-up tickets:** none.
