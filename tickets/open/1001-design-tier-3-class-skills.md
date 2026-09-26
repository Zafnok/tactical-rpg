---
id: "1001"
title: Propose tier-3 class skills (and elemental enemy skills)
type: content
milestone: Post–Chapter 1
model: opus-5.5
effort: medium
status: todo
blocked_by: ["0005", "0311"]
nick_input: sign-off
completed:
---

# 1001 — Tier-3 class skills

## Context

`docs/design/progression.md` (ticket 0005) sets the skill system and
fills in placeholder skills for tiers 1–2: an active when a class is
unlocked, passives when it is mastered, and families with ranks that supersede. Tier-3 classes
(Blade Dancer … Oracle) and the elemental enemies have no skills yet,
because Chapter 1 can't reach tier 3. Nick's own example belongs here: an
Oracle learning **White Magic 2/3** ("+3 or +4") that replaces the lower
rank.

## Nick input

**Sign-off:** Nick reads the proposed list and approves it or vetoes
entries. Design questions go through the `ask-nick` skill.

## Scope

**In:**
- 1–2 passives and 1 active for each of the 16 tier-3 classes in
  `progression.md` that don't have skills yet (the tier-3 Flier already has
  Swoop and Sky Dodge 1 from ticket 0017). This includes the lord's
  Sovereign and Grand Marshal (ticket 0016): their skills are the lord's
  signature abilities, so they should lean towards leading the army.
- Optional passives for Fire/Frost Elementals.
- The numbers in `skills.ron`.

**Out (do not do):**
- Tiers 4 and up (not designed).
- New effect kinds that 0311's engine doesn't support. Write a ticket
  instead.

## Implementation steps

1. Draft the list, using higher ranks of tier 1–2 families where they fit
   (Sword Focus 3, Light Feet 3, White Magic 3 …). Keep "+1 strike" skills
   rare, since 3x/4x must stay rare (`stats-and-combat.md`).
2. Ask Nick for sign-off (with the `ask-nick` skill if a real design choice
   comes up). Record the list in `progression.md`.
3. Add the skills to `skills.ron` and the class data, with tests like
   0311's.

## Acceptance criteria

- [ ] Every tier-3 class has passives and an active in `progression.md` and the data.
- [ ] Nick signed off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: one effect test per new skill, as in 0311.

## Completion notes
