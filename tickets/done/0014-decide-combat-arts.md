---
id: "0014"
title: "Decide: Combat Arts (weapon skills that spend durability)"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: ["0003"]
nick_input: decision
completed: 2026-09-25
---

# 0014 — Decide: Combat Arts

## Context

Ticket 0003 (`docs/design/weapons-and-items.md`) made durability drop **only**
when a unit uses a Combat Art, Fortune's Weave-style: "the durability only
goes down for skills, and each skill takes a different amount of durability
balanced by how powerful its perceived effect is" (Nick). Nick chose "own
ticket, in Chapter 1": this ticket decides the arts so Chapter 1 has some.
Related: class skills in 0005 (`progression.md`); post-action movement skills
in `turn-structure.md` (e.g. FE's bow skill that steps 1 tile away after
attacking) may be arts or class skills. Run with the `ask-nick` skill.

## Nick input

**Decision.**

## Questions to ask

### Q1. Who gets which arts?

**A. Per weapon, like Fortune's Weave** — some weapons carry their own art
(Killing Edge → "Deadly Blade" crit strike); iron weapons have none.
**B. Per weapon rank, like Three Houses** — reaching rank D/C/B in a weapon
kind teaches that kind's arts (Swords: Wrath Strike at D, Windsweep at C…);
any weapon of that kind can use them.
**C. Per class** — each class brings 1–2 arts.
**D. Mix** — rank arts (B) plus a few special weapons with their own art (A).
**Recommendation:** D — rank arts reward "investment in the weapon skill"
(Nick's words in 0001); special weapons make loot exciting.

### Q2. The starter list

Propose ~2 arts per weapon kind (sword, spear, axe, bow, gauntlet) for
Chapter 1 with exact effects and durability costs, drawn from Three Houses
combat arts (e.g. Wrath Strike: +5 Mt, cost 3; Grounder: +20 hit, effective
vs flying, cost 3; Curved Shot: +1 range, +30 hit, cost 3; Smash: +20 hit,
cost 2; Helm Splitter: +3 Mt, effective vs armored, cost 4). Show the
forecast difference with an ASCII mockup. Nick vetoes/edits.

### Q3. Do arts have other limits?

**A. Durability only** (Fortune's Weave). **B. Also no counter-attack when
using an art** (Three Houses: arts can only be used when attacking, not on
counters, and never double). **C. Also once per turn / cooldown.**
**Recommendation:** A+B's "attacker only, one strike, no follow-ups"
(Three Houses) so arts are a choice, not a free upgrade.

## What to record

A `## Combat Arts` section in `docs/design/weapons-and-items.md` (or a new
`docs/design/combat-arts.md`): Nick's words; how arts are learned; the full
Chapter 1 list with effect, durability cost, range and weapon-EXP gain; rules
for strikes/counters/follow-ups with arts; forecast display. Create the
implementation ticket(s) (core `03xx` + UI `04xx`) with the `write-ticket`
skill. Update `docs/design/README.md`.

## Acceptance criteria

- [x] Nick answered Q1–Q3.
- [x] Design doc section written with exact numbers.
- [x] Implementation tickets created; design README updated.
- [x] Ticket archived.

## Completion notes

Nick answered every question on 2026-09-25. The result is in
`docs/design/combat-arts.md` (a new file rather than a section of
`weapons-and-items.md`, which now links to it).

- **Q1:** "Mix". Weapon ranks teach each kind's arts, and special weapons
  carry their own.
- **Q2 (limits):** "Durability only". An art boosts every strike of the
  combat. Arts are picked only on your own turn, but an art's effect may
  last through the enemy's turn (stance/debuff).
- **Q3 (vs actives):** class actives **also cost durability**, which changes
  `progression.md`. Actives no longer have uses per battle. Spell actives
  cost 1 extra spell use, and non-attack actives cost the equipped weapon's
  durability. There's no fixed dividing line between arts and actives:
  both should be flavourful, and overlaps rare and sensible.
- **Starter list:** approved as proposed. 10 arts, E and D per kind, with
  our own names (not Three Houses').
- **Enemies:** only bosses use arts and actives.

Claude's starting rules (Nick may veto them): one art or active per attack;
cost paid even on a miss; a weapon brought to 0 breaks after the combat;
active costs of 3 (was 2/battle) and 5 (was 1/battle).

Nick's follow-up on the PR: weapon EXP is **base + damage** (base 2, or 4
with an art, plus `dealt / 5`, *tunable*). This replaces the old flat +2/+1
rule in `weapons-and-items.md` and the ticket 0306 step. He also confirmed
spells are the only per-battle resource: they never use durability.

Downstream edits: `progression.md` skill table and actives rules,
`weapons-and-items.md` durability/weapon EXP, tickets 0311 (costs instead of
uses, shared `pay_cost`), 0412 (cost display), 0501 (boss arts out of
scope → 0503), `ROADMAP.md`.

New tickets: **0312** Combat Arts rules (core), **0414** arts menu and
forecast (UI), **0503** boss AI uses arts/actives, **0018** decide
higher-rank arts and special weapons (after the playtest).

