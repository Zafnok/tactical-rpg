# Game design decisions

Nick decides how the game plays. Each `00xx` ticket asks him (using the
`ask-nick` skill: options from real games + "describe your own"), and records
the answer here as a concrete, implementable rule set. Implementation tickets
treat these files as the source of truth, and never invent answers that aren't
here.

| Topic | File | Ticket | Status |
| ----- | ---- | ------ | ------ |
| Stats & combat maths | [`stats-and-combat.md`](stats-and-combat.md) | 0001 | ✅ decided 2026-09-25 |
| Turn structure | `turn-structure.md` | 0002 | ⏳ awaiting Nick |
| Weapons, items, triangle | `weapons-and-items.md` | 0003 | ⏳ awaiting Nick |
| Magic & healing | `magic.md` | 0004 | ⏳ awaiting Nick |
| Level ups, classes, class tree | `progression.md` | 0005 | ⏳ awaiting Nick |
| Death, rewind, difficulty, saving | `death-and-difficulty.md` | 0006 | ⏳ awaiting Nick |
| Setting & tone | `setting-and-tone.md` (+ `docs/story/beats.md`) | 0007 | ⏳ awaiting Nick |
| World structure | `world-structure.md` | 0008 | ⏳ awaiting Nick |
| Chapter 1 scope | `chapter-1.md` | 0009 | ⏳ awaiting Nick |
| Supports & relationships | `supports.md` | 0010 | ⏳ awaiting Nick |
| Look & feel | `look-and-feel.md` | 0011 | ⏳ after font ticket 0203 |
| Title | `title.md` | 0012 | ⏳ after story bible 0701 |
| Number scale & strike thresholds | [`stats-and-combat.md`](stats-and-combat.md) | 0013 | ⏳ after playtest 0804 |

Each file starts with `Decided: YYYY-MM-DD`, `Source: ticket NNNN`, and Nick's
words verbatim, followed by the derived rules (numbers marked *tunable* where
Claude chose a starting value).
