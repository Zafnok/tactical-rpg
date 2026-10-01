---
id: "0316"
title: "Non-attack actives cost uses per battle, not durability; Benediction replaces Sanctuary 2"
type: feature
milestone: M2 Core rules
model: opus-5.5
effort: high
status: todo
blocked_by: ["0311", "0412", "0503"]
nick_input: sign-off
completed:
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
     at least one is higher. A cost in durability may be anything.
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

- [ ] A Grappler starts a battle with 8 Shoves; each Shove spends one, the 9th is refused, and the next battle starts with 8 again (tests).
- [ ] A unit with no weapon equipped, or a broken one, can use Brace; using it changes no weapon's durability (tests).
- [ ] A Priest promoted from a mastered Cleric has Sanctuary (3 uses, adjacent) and Benediction (1 use, 2 tiles), and White Magic adds to both (test).
- [ ] Keen Edge and Guard Break still cost durability; Overcast still costs a spell use (existing tests pass unchanged).
- [ ] Rewinding past a use gives it back (test).
- [ ] The skill menu shows the uses left; an active with none left is dimmed with the reason (snapshots).
- [ ] A boss holding its tile braces while it has a use left and waits once it has none (tests); ordinary enemies still never do.
- [ ] Old saves load (the new field defaults).
- [ ] Content validation refuses a rank 2 that differs from its rank 1 in anything but bigger numbers: one test per case (a wider radius, another condition, another effect kind, a lower number, no number higher); `assets/data/skills.ron` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `core::skill` cost rules; battle tests for each acceptance case;
  AI scenario tests; content validation errors.
- Property: the existing random-battle tests (`crates/core/src/battle/tests.rs`,
  `crates/core/src/ai/tests.rs`) keep passing: uses never go below 0 and
  every listed legal command is accepted.
- Snapshot / integration: the skill menu with uses left and with none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
