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

**In:** core promotion and reclass rules + events; the **Promotion Seal**
and **Reclass Seal** items (placeholder names); free switching between
already-unlocked classes; class-choice screen with side-by-side stat
previews; promotion stat-gain overlay (reuse 0602's).

**Out:** tiers above 3 (not designed yet); flavour names; seal
prices/drops (chapter and shop data).

## Implementation steps

1. `core::progression::promote(unit, target_class, classes) -> Result<Vec<Event>, PromoteError>`
   per `progression.md`: current class **mastered** (class level 10),
   `target_class ∈ promotes_to`, a Promotion Seal is consumed. The
   **character level and EXP do not reset**. Bonus per stat =
   `max(0, new.base − old.base)` (clamped to the hard ceiling), current HP
   rises by the HP bonus, new class record at class level 1 → learn its
   passives and class-level-1 spells (`SkillLearned` / `SpellLearned`),
   weapon ranks raised to the new class's start ranks, extra weapons to
   stock if the slots shrink (0309's helper). Event `Promoted { unit, from, to, gains }`.
2. `reclass(unit, target_class, via: {Unlocked, ReclassSeal}, classes)`:
   `Unlocked` = switching to a class already in `class_records`, free,
   between battles / Preparations only; `ReclassSeal` = entering a not yet
   unlocked **tier-1** class of another line (not `enemy_only`), consuming
   the seal. Stats never change; stats above the new caps are kept.
   Event `Reclassed { unit, from, to }`.
3. Triggers: `UnitAction::UseItem` on a Promotion Seal in battle (ends the
   action), plus the same actions from the between-battle unit menu /
   Preparations (0408) if it exists; otherwise note it as a follow-up ticket.
4. **Choice screen:** two (or N) columns, each: class name, map glyph, move,
   weapons, passives gained, and every stat `current → promoted` with gains
   highlighted; `h/l` switch column, `f` choose, confirm dialog, `d` cancel.
   Reclass uses the same screen listing unlocked classes (and, with a seal,
   eligible tier-1 classes).
5. After choice: stat-gain overlay (reuse 0602 widget) → back.

## Acceptance criteria

- [ ] All validation errors tested (not mastered, not in `promotes_to`, no seal, enemy-only class, reclass seal into a tier-2 class) — state unchanged.
- [ ] Promotion keeps the character level and EXP; shared promotion (Iron Rider from Guard vs from Rider) gives different bonuses, matching a hand-worked example.
- [ ] Reclass never changes stats; learned skills and spells are kept.
- [ ] Choice screen shows correct previews (test against `promote` on a cloned unit).
- [ ] Harness: promote via item → class changed, stats as expected.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit + property: stats after promotion ≤ hard ceilings and never lower than before; level/EXP unchanged by promotion or reclass.
- Harness + snapshot of the choice screen.

## Completion notes

