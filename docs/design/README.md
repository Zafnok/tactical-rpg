# Game design decisions

Nick decides how the game plays. Each `00xx` ticket asks him (using the
`ask-nick` skill: options from real games + "describe your own"), and records
the answer here as a concrete, implementable rule set. Implementation tickets
treat these files as the source of truth, and never invent answers that aren't
here.

| Topic | File | Ticket | Status |
| ----- | ---- | ------ | ------ |
| Stats & combat maths | [`stats-and-combat.md`](stats-and-combat.md) | 0001 | ✅ decided 2026-09-25 |
| Turn structure | [`turn-structure.md`](turn-structure.md) | 0002 | ✅ decided 2026-09-25 |
| Weapons, gear, items, shops | [`weapons-and-items.md`](weapons-and-items.md) | 0003 | ✅ decided 2026-09-25 |
| Combat Arts (and active-skill costs) | [`combat-arts.md`](combat-arts.md) | 0014 | ✅ decided 2026-09-25 |
| Combat Arts for ranks C–S, special weapons | `combat-arts.md` | 0018 | ⏳ after playtest 0804 |
| Magic & healing | [`magic.md`](magic.md) | 0004 | ✅ decided 2026-09-25 |
| Level ups, classes, class tree | [`progression.md`](progression.md) | 0005, 0017 | ✅ decided 2026-09-25 (fliers moved to tier 3+ by 0017) |
| Death, rewind, difficulty, saving | [`death-and-difficulty.md`](death-and-difficulty.md) | 0006 | ✅ decided 2026-09-25 |
| Setting, tone & the lead | [`setting-and-tone.md`](setting-and-tone.md) (+ [`docs/story/beats.md`](../story/beats.md)) | 0007 | ✅ decided 2026-09-25 |
| World structure | `world-structure.md` | 0008 | ⏳ awaiting Nick |
| Chapter 1 scope | [`chapter-1.md`](chapter-1.md) | 0009 | ✅ decided 2026-09-25 |
| The lord's unique class line | `progression.md` (lord section) | 0016 | ⏳ awaiting Nick |
| Supports & relationships | [`supports.md`](supports.md) | 0010 | ✅ decided 2026-09-25 |
| Look & feel | [`look-and-feel.md`](look-and-feel.md) | 0011 | ✅ decided 2026-09-25 |
| Title | `title.md` | 0012 | ⏳ after story bible 0701 |
| Terrain: movement costs & bonuses | [`terrain.md`](terrain.md) | 0301 | ✅ decided 2026-09-26 (capturing & healing tiles deferred) |
| Controls & key layouts | [`controls.md`](controls.md) | 0015 | ✅ decided 2026-09-25 |
| Number scale & strike thresholds | [`stats-and-combat.md`](stats-and-combat.md) | 0013 | ⏳ after playtest 0804 |

Each file starts with `Decided: YYYY-MM-DD`, `Source: ticket NNNN`, and Nick's
words verbatim, followed by the derived rules (numbers marked *tunable* where
Claude chose a starting value).
