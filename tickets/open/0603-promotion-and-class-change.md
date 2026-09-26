---
id: "0603"
title: Promotion / class change (logic and choice screen)
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0005", "0306", "0601", "0602"]
nick_input: answer-first
completed:
---

# 0603 — Promotion and class change

## Context

Nick wants class changes "like Fire Emblem". Rules in
`docs/design/progression.md` (0005). **Not on the Chapter 1 critical path**
(Chapter 1 units won't reach promotion level), but needed soon after.

## Nick input

**Answer first:** 0005 (done). **Sign-off** on the choice screen after merge.

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
   unchanged; a new class unlocks at class level 1. Stats never change, and
   stats above the new caps are kept.
   Event `Reclassed { unit, from, to }`.
3. Triggers: `UnitAction::UseItem` on a tier seal in battle (ends the
   action), plus the same actions from the between-battle unit menu /
   Preparations (0408) if it exists; otherwise note it as a follow-up ticket.
4. **Choice screen:** two (or N) columns, each: class name, map glyph, move,
   weapons, the active gained, and every stat `current → promoted` with gains
   highlighted; `h/l` switch column, `f` choose, confirm dialog, `d` cancel.
   Reclass uses the same screen, listing every class the seal can reach
   (unlocked classes show their saved class level).
5. After choice: stat-gain overlay (reuse 0602 widget) → back.

## Acceptance criteria

- [ ] All validation errors tested (not mastered, not in `promotes_to`, no seal, a seal for the wrong tier, enemy-only class, reclass into a tier-2 class whose prerequisite isn't mastered) — state unchanged.
- [ ] Promotion keeps the character level and EXP; shared promotion (Iron Rider from Guard vs from Rider) gives different bonuses, matching a hand-worked example.
- [ ] Reclass never changes stats and always consumes a seal; learned skills, spells and saved class records are kept (leave a class at class level 6, come back, and it is still 6).
- [ ] Choice screen shows correct previews (test against `promote` on a cloned unit).
- [ ] Harness: promote via item → class changed, stats as expected.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit + property: stats after promotion ≤ hard ceilings and never lower than before; level/EXP unchanged by promotion or reclass.
- Harness + snapshot of the choice screen.

## Completion notes

