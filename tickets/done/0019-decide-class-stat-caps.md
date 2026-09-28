---
id: "0019"
title: "Decide: should classes cap stats, and how does that fit a very high level cap?"
type: design-decision
milestone: Design decisions
model: opus-5.5
effort: medium
status: done
blocked_by: []
nick_input: decision
completed: 2026-09-28
---

# 0019 — Decide: class stat caps

## Context

`docs/design/progression.md` ("Stat caps") and `stats-and-combat.md` give
every class a per-stat cap (e.g. Exile HP 42, Str 22; Blade Heir HP 50): a
level-up roll can't raise a stat past its current class's cap. That rule was
**Claude's tunable default**, never Nick's decision. It came up on
2026-09-28 when Nick questioned the `19/42` on the info screen (removed in
0423): "We have 1) a level cap 2) random growth per level so how can a MAX be
constrained by class...?"

It may also clash with Nick's level-pace answer: the level cap "can be quite
high TBD … maybe your level would be in the millions". With per-class caps,
units would stop growing long before a huge level cap.

Run with the `ask-nick` skill. Explain plainly: caps are a hard ceiling, not
a prediction; rolls can leave a unit below them.

## Nick input

**Decision.**

## Questions to ask

1. **Should stats have per-class caps at all?** Options with game refs:
   per-class caps lifted by promotion (FE GBA / Three Houses); one global
   ceiling per stat, no class caps (FE Engage-like hard limits only); no caps,
   growth only limited by the level cap (Disgaea); per-character caps.
2. If caps stay: **where, if anywhere, are they shown** (class-change /
   promotion screen, a MAX marker when reached, nowhere)?
3. How caps interact with the (undecided) level cap and number scale (0013).

## What to record

Update `docs/design/progression.md` (Stat caps) and `stats-and-combat.md`.
Create implementation tickets for any change to `core` level-up rules and
`classes.ron`. Update `docs/design/README.md`.

## Acceptance criteria

- [x] Nick answered 1–3.
- [x] Design docs updated; implementation tickets created.
- [x] Ticket archived.

## Completion notes

Nick's answers (2026-09-28): **1C** (no per-class caps; the max comes only
from the true caps, meaning the hard ceilings and the level cap, and classes
shape growth only through growth rates; staying in a class long is never
penalised), **2A but not in battle** (mark a stat at its ceiling `MAX` on
out-of-battle screens only; in battle "MAX" means max HP), **3C** (the
ceiling numbers, fixed or rising with tier, are decided in 0013).

- `progression.md`: Nick's words; *Stat caps* rewritten; level-up
  eligibility and clamping use the hard ceilings; the Caps column removed
  from the class stat table; reclass, generic-unit and named-character rules
  updated. `stats-and-combat.md`, `weapons-and-items.md` and the design
  README updated to match.
- Tickets changed: 0013 gains Q3 (ceiling numbers, fixed or per tier, and
  the level cap); 0603's choice screen shows `MAX` and no longer keeps
  "stats above the new caps"; 0602's level-up overlay shows no `MAX`.
- Follow-up created: **0604** (remove `caps` from `classes.ron`, content and
  `core`; clamp to the hard ceilings).
- *Claude's starting rule:* the promotion/class-change choice screen shows
  `MAX` even if it opens during a battle (a seal used from the battle menu),
  because it is a class-choice screen, not a battle screen. Nick may veto.
- Until 0604 lands, the game still applies the old class caps.
