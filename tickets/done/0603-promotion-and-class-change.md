---
id: "0603"
title: Promotion / class change (logic and choice screen)
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: done
blocked_by: ["0005", "0306", "0601", "0602"]
nick_input: answer-first
completed: 2026-09-30
---

# 0603 — Promotion and class change

## Context

Nick wants class changes "like Fire Emblem". Rules in
`docs/design/progression.md` (0005). **Not on the Chapter 1 critical path**
(Chapter 1 units won't reach promotion level), but needed soon after.

## Nick input

**Answer first:** 0005 (done). **Sign-off** on the choice screen after merge.

**Decided while working this ticket (2026-09-30):** Nick: **no seal works
in a battle**; promotion and reclass happen between battles only
(`progression.md`, *Seals in battle*). That replaces step 3 below (no
`UseItem` on a seal) and moves the choice screen out of the battle: it is
its own screen, opened from the debug menu until ticket 0606 puts it in the
between-battle menu.

## Scope

**In:** core promotion and reclass rules + events; the per-tier **promotion seals**
(Tier 2 Seal, Tier 3 Seal …) and the **Reclass Seal** items (placeholder names); reclass (always costs a
seal, class progress saved); class-choice screen with side-by-side stat
previews; promotion stat-gain overlay (reuse 0602's).

**Out:** tiers above 3 (not designed yet); flavour names; seal
prices/drops (chapter and shop data).

## Implementation steps

1. `core::progression::promote(unit, target_class, classes) -> Result<Vec<Event>, PromoteError>`
   per `progression.md`: current class **mastered** (class level 10),
   `target_class ∈ promotes_to`, the seal for the target class's tier is consumed. The
   **character level and EXP do not reset**. Bonus per stat =
   `max(0, new.base − old.base)` (clamped to the hard ceiling), current HP
   rises by the HP bonus, new class record at class level 1 → learn its
   active is usable at once, plus class-level-1 spells (`SpellLearned`),
   weapon ranks raised to the new class's start ranks, extra weapons to
   stock if the slots shrink (0309's helper). Event `Promoted { unit, from, to, gains }`.
2. `reclass(unit, target_class, classes)` per `progression.md`: always
   consumes a Reclass Seal. The target must be a tier-1 class, or a class
   whose prerequisite the unit has mastered, or a class already in
   `class_records`, and never `enemy_only`. A `lord_only` class (the lord's
   line, ticket 0016) is a valid target only for the `is_lord` unit; the
   lord reclasses out of and back into its line like anyone else. Saved class records come back
   unchanged; a new class unlocks at class level 1. Stats never change
   (classes have no stat caps, ticket 0019).
   Event `Reclassed { unit, from, to }`.
3. Triggers: `UnitAction::UseItem` on a tier seal in battle (ends the
   action), plus the same actions from the between-battle unit menu /
   Preparations (0408) if it exists; otherwise note it as a follow-up ticket.
4. **Choice screen:** two (or N) columns, each: class name, map glyph, move,
   weapons, the active gained, and every stat `current → promoted` with gains
   highlighted, and `MAX` after a stat at its hard ceiling (Nick, ticket
   0019: shown out of battle only; `progression.md` *Showing a maxed stat*); `h/l` switch column, `f` choose, confirm dialog, `d` cancel.
   Reclass uses the same screen, listing every class the seal can reach
   (unlocked classes show their saved class level).
5. After choice: stat-gain overlay (reuse 0602 widget) → back.

## Acceptance criteria

- [x] All validation errors tested (not mastered, not in `promotes_to`, no seal, a seal for the wrong tier, enemy-only class, reclass into a tier-2 class whose prerequisite isn't mastered) — state unchanged.
- [x] Promotion keeps the character level and EXP; shared promotion (Iron Rider from Guard vs from Rider) gives different bonuses, matching a hand-worked example.
- [x] Reclass never changes stats and always consumes a seal; passives and saved class records are kept (leave a class at class level 6, come back, and it is still 6); the spells and active of a class left unmastered are gone after the reclass (`Unit::refresh_spells` reports them as lost) and come back on returning to it.
- [x] Choice screen shows correct previews (test against `promote` on a cloned unit), with `MAX` on stats at their hard ceiling. (The screen can't open during a battle any more: Nick, "no seals in battle".)
- [x] Harness: promote → class changed, stats as expected. **Changed:** not "via item" in a battle (Nick ruled that out); the Harness test promotes through the choice screen, opened from the debug menu.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit + property: stats after promotion ≤ hard ceilings and never lower than before; level/EXP unchanged by promotion or reclass.
- Harness + snapshot of the choice screen.

## Completion notes

**Nick's decision (asked first, as the work-ticket skill says):** no seal
works in a battle. Promotion and reclass are between-battle actions; seals
sit in the party's stock and never take a battle-pack slot. Recorded in
`docs/design/progression.md` (*Seals in battle*, *Promotion*, *Reclass*) and
`weapons-and-items.md` (*Battle pack*).

**What was built**

- **Rules** (`crates/core/src/progression/class_change.rs`): `promote` and
  `reclass` work on a unit and the party's `Stock` (not `Command`s: no
  battle is running), with one error type, `ClassChangeError`
  (`NotMastered`, `NotAPromotion`, `NoSeal`, `EnemyOnly`, `LordOnly`,
  `SameClass`, `NotReachable`, `UnknownClass`). Everything is checked before
  anything changes. `promotion_targets` / `reclass_targets` list what each
  offers; `promotion_gains` is the bonus. Events: `Promoted`, `Reclassed`,
  then `SpellLearned` per spell and `ItemStowed` per item sent to the stock.
- **Seals** (`ItemDef::Seal`, `SealKind::Tier(n)` / `Reclass`; `items.ron`
  `seals:`): Tier 2 Seal, Tier 3 Seal, Reclass Seal (placeholder names).
  Content validation: one seal per kind, a tier seal's tier is at least 2,
  and every tier the class tree promotes into has its seal.
  In a battle a seal is an item that can't be used (`NotConsumable`); one
  found in a chest goes to the stock. No shop sells or buys seals yet
  (`ShopError::NotBought`).
- **Choice screen** (`crates/ui/src/screens/class_change.rs`): one column
  per class (name, tier, saved class level or `new` for a reclass, Mov and
  movement type, tags, weapon ranks before → after, the active, every stat
  `current → new` with the gain highlighted and `MAX` at the hard ceiling).
  Left/right pick a column (they scroll past four), Confirm asks
  `Promote … ?`, Confirm again makes the change, Cancel backs out. A class
  that would be refused is dimmed with the reason.
- **Overlay** (`screens/battle/progress.rs`): the 0602 level-up panel shows
  a promotion (`PROMOTED!`, `Guard → Iron Rider`, the bonus one stat at a
  time); the class box shows a reclass (`Guard → Mage`), `Learned: …` and
  `To stock: …` lines.
- **Debug menu**: `Class change: promote` and `Class change: reclass` open
  the screen on a test knight (a mastered Guard, Def 49 so a `MAX` shows, a
  Rider record at class level 6) with one of every seal.

**Deviations from the steps**

- Step 3 (`UnitAction::UseItem` on a seal) wasn't built: Nick's decision.
  The between-battle menu doesn't exist yet, so the entry point is ticket
  **0606** (follow-up, created).
- Step 4's "map glyph" was left out: classes have no glyph of their own
  (units are drawn as two letters of their name, ADR-0018). The column
  shows the movement type and tags instead.
- The functions take the stock and the item and spell tables too
  (`promote(unit, target, stock, tables)`): the seal comes from the stock
  and the stowed weapons go to it.
- Tickets 0408 (Preparations) and 0409 (shop screen) got a line each: seals
  aren't listed in the pack or the sell list.

**Rules Claude decided where the design was silent** (*Claude's starting
rules*; Nick may veto):

1. **Armour the new class can't wear goes to the stock**, like weapons
   beyond its weapon slots. Example: a Guard in chain mail reclasses to
   Mage (light armour only): the chain mail goes back to the stock.
2. **A Reclass Seal can't be spent on the class the unit is already in**
   (it would change nothing).
3. **No shop sells or buys a seal** until the shop and chapter data say
   where seals come from. A seal has no price.
4. **The debug test knight's numbers** (level 12, Def 49) are made up for
   the demo only.

**For Nick when playing:** open the debug menu and pick `Class change:
promote` or `Class change: reclass`. Nothing done there is kept (there is
no campaign yet); it is only for looking at the screen.

