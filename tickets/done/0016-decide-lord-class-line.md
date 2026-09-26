---
id: "0016"
title: "Decide: the lord's unique class line"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: ["0005", "0009"]
nick_input: decision
completed: 2026-09-25
---

# 0016 — Decide: the lord's unique class line

## Context

In 0009 (`docs/design/chapter-1.md`) Nick said the lord "has a unique
starting class/tree" instead of one of the shared tier-1 classes in
`docs/design/progression.md`. Chapter 1 content (0803) needs that class's
data. Run with the `ask-nick` skill. Keep it consistent with
`progression.md` (class levels, mastery, tiers as a number, 3 weapon slots),
`weapons-and-items.md` (weapon kinds and ranks) and `setting-and-tone.md`
(disgraced/exiled noble, the player picks the gender).

## Nick input

**Decision.**

## Questions to ask

Use real-game examples (e.g. FE Eliwood/Hector/Lyn Lord → Knight Lord /
Great Lord / Blade Lord, FE Ike Ranger → Lord, Three Houses Byleth Commoner →
Enlightened One, Engage Alear Dragon Child → Divine Dragon, Tactics Ogre
Denam's unique class, Unicorn Overlord Alain Prince → Lord).

1. **Weapons and role.** Which weapon kinds (and start/max ranks) and what
   role: a sword infantry all-rounder, mounted, a hybrid with a spell, ...?
2. **Shape of the line.** A single unique path through tiers 1–3 (FE Lord →
   Great Lord), or a unique tier 1 that branches into unique or shared tier-2
   classes?
3. **Reclassing.** Can the lord reclass into other lines (and back), can
   other units ever enter the lord's line, and does mastery work as normal?
4. **Signature.** Any unique skill or personal spell (see `progression.md`
   for actives/passives and `magic.md` for personal spells)?
5. **Stats.** Show a sample tier-1 base/growth line next to Swordsman and
   Rider so Nick can judge it (numbers stay *tunable*).

## What to record

Add the lord's line to `docs/design/progression.md` (class tree diagram,
class table rows for each tier, promotion rules) with Nick's words quoted and
*tunable* marks. Update `chapter-1.md`'s roster row. If 0302 is already done,
create a ticket to add the lord classes to `assets/data/classes.ron`;
otherwise note in 0302 that the tree includes them.

## Acceptance criteria

- [x] Nick answered Q1–Q5.
- [x] `progression.md` and `chapter-1.md` updated.
- [x] Design README updated; downstream tickets adjusted; ticket archived.

## Completion notes

Nick answered in two rounds (2026-09-25):

- **Q1 "1A":** a sword foot soldier who is good at everything (FE Eliwood/Lyn/Marth).
- **Q2 "2B":** a unique tier 1 (**Exile**) branching into two lord-only tier-2
  classes (**Blade Heir**, **Commander**), each with its own lord-only tier 3
  (**Sovereign**, **Grand Marshal**).
- **Q3 "3B":** the lord reclasses out and back with seals like anyone else;
  nobody else can enter the lord's line; mastery works as normal.
- **Q4:** the signature abilities *are* the lord classes' own skills,
  "nothing else on top" (no extra skill, personal spell or weapon).
- **Q5:** names and numbers OK, except "Warlord", which Nick reserved for a
  shared line. Claude renamed the lord's tier 3 **Grand Marshal**
  (placeholder; Nick may rename it).

Recorded in `progression.md` (*The lord's line*, tree, class tables,
stats, tier-1–2 skills) and `chapter-1.md`'s roster row. The lord classes'
skills (Inspire / Leadership, Crest Strike / Resolve, Rally / Leadership 2)
and all the numbers are Claude's placeholders, as for every other class.

0302 isn't done, so no new ticket was needed. Downstream tickets adjusted:
0302 (the tree includes the `lord_only` line, plus validation), 0311 (ally
aura passive and area buffs), 0603 (reclass targets for `lord_only`), 1001
(Sovereign and Grand Marshal skills).
