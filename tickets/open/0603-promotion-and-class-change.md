---
id: "0603"
title: Promotion / class change (logic and choice screen)
type: feature
milestone: M5 Progression
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0005", "0601", "0602"]
nick_input: answer-first
completed:
---

# 0603 — Promotion and class change

## Context

Nick wants class changes "like Fire Emblem". Rules in
`docs/design/progression.md` (0005). **Not on the Chapter 1 critical path**
(Chapter 1 units won't reach promotion level), but needed soon after.

## Nick input

**Answer first:** 0005. **Sign-off** on the choice screen after merge.

## Scope

**In:** core promotion rules + event, the promotion item or trigger the design
specifies, class-choice screen with side-by-side stat previews, promotion
stat-gain overlay (reuse 0602's).

**Out:** reclass items or job systems unless the design includes them in its
first version.

## Implementation steps

1. `core::progression::promote(unit, target_class, classes) -> Result<Vec<Event>, PromoteError>`:
   validate (level requirement, `target_class ∈ promotes_to`, item present if
   required), apply stat bonuses per design, reset level/EXP per design, change
   class; `Promoted { unit, from, to, gains }` event.
2. Trigger per design (e.g. `UnitAction::UseItem` on a promotion item in
   battle, and/or a between-chapter menu). Implement the in-battle item path;
   note any between-chapter path as a follow-up ticket if the flow screen
   doesn't exist yet.
3. **Choice screen:** two (or N) columns, each: class name, map glyph, move,
   weapons, and every stat `current → promoted` with gains highlighted; `h/l`
   switch column, `f` choose, confirm dialog, `d` cancel.
4. After choice: stat-gain overlay (reuse 0602 widget) → back.

## Acceptance criteria

- [ ] All validation errors tested (state unchanged).
- [ ] Choice screen shows correct previews (test against `promote` on a cloned unit).
- [ ] Harness: promote via item → class changed, stats as expected.

## Tests required

- Unit + property: stats after promotion ≤ new class caps; level/EXP reset per design.
- Harness + snapshot of the choice screen.

## Completion notes

