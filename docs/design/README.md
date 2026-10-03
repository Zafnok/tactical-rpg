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
| Weapons, gear, items, shops | [`weapons-and-items.md`](weapons-and-items.md) | 0003, 0408 (Preparations) | ✅ decided 2026-09-25 (Preparations: bring only your own items, retry returns there, 2026-10-01) |
| The Preparations shop: basic items and tiers | `weapons-and-items.md` | 0041 | ⏳ after playtest 0804 |
| Combat Arts (and active-skill costs) | [`combat-arts.md`](combat-arts.md) | 0014 | ✅ decided 2026-09-25 (non-attack actives cost uses per battle, 2026-10-01) |
| Combat Arts for ranks C–S, special weapons | `combat-arts.md` | 0018 | ⏳ after playtest 0804 |
| Magic & healing | [`magic.md`](magic.md) | 0004 | ✅ decided 2026-09-25 (spells after a class change and terrain magic details revised 2026-09-27; casting on the battle screen added 2026-10-01) |
| Level ups, classes, class tree | [`progression.md`](progression.md) | 0005, 0017, 0019 | ✅ decided 2026-09-25 (fliers moved to tier 3+ by 0017; no per-class stat caps, 0019, 2026-09-28; seals only between battles, 0603, 2026-09-30) |
| Death, rewind, difficulty, saving | [`death-and-difficulty.md`](death-and-difficulty.md) | 0006 | ✅ decided 2026-09-25 |
| Setting, tone & the lead | [`setting-and-tone.md`](setting-and-tone.md) (+ [`docs/story/beats.md`](../story/beats.md)) | 0007 | ✅ decided 2026-09-25 |
| World structure, saving on the world map, camp | [`world-structure.md`](world-structure.md) | 0008 | ✅ decided 2026-09-26 |
| Chapter 1 scope | [`chapter-1.md`](chapter-1.md) | 0009 | ✅ decided 2026-09-25 |
| The lord's unique class line | [`progression.md`](progression.md) (*The lord's line*) | 0016 | ✅ decided 2026-09-25 |
| Supports & relationships | [`supports.md`](supports.md) | 0010 | ✅ decided 2026-09-25 |
| Look & feel | [`look-and-feel.md`](look-and-feel.md) | 0011, 0404 (forecast), 0021 (bought art), 0038 (bought tiles and sprites on the battle map), 0039 (sprite units and dialogue busts) | ✅ decided 2026-09-25 (forecast 2026-09-27; bought portraits and battle art 2026-09-30; art bought, and the battle map drawn with it, per-map lighting and a zoom key 2026-10-02; walking sprites, effect arrows, the glyph look as an option and busts at 4× in dialogue 2026-10-02) |
| Sprite units: the outline alone or with a corner mark; more packs; the zoom key | `look-and-feel.md`, `controls.md` | 0436, 0040, 0035, 0439 | ⏳ open |
| Music & sound effects | [`audio.md`](audio.md) | 0020 | ✅ decided 2026-09-28 (banter track, plain-spell crit, fliers and later places still open) |
| Title | [`title.md`](title.md) | 0012 | ✅ decided 2026-09-29 (*Visions of Shuyi*) |
| Title screen: "Press any key or button" | [`title-screen.md`](title-screen.md) | 0034, 0032 | ✅ decided 2026-09-30 (every build, keys and buttons, since 0032) |
| Terrain: movement costs & bonuses | [`terrain.md`](terrain.md) | 0301 | ✅ decided 2026-09-26 (capturing & healing tiles deferred) |
| Controls, key layouts & controller | [`controls.md`](controls.md) | 0015, 0030, 0032 | ✅ decided 2026-09-25 (rebinding keys 0030, 2026-09-29; controller 0032, 2026-09-30) |
| Battle scenes, talking & recruitment | [`battle-scenes-and-recruitment.md`](battle-scenes-and-recruitment.md) | 0705 | ✅ decided 2026-09-29 (quest recruitment later) |
| Playtest bots: player types, targets, autobalancing | [`playtest-bots.md`](playtest-bots.md) | 0033 | ✅ decided 2026-09-30 |
| AI voices, a Japanese option, a hired writer later | [`voices-languages-and-script.md`](voices-languages-and-script.md) | 0042 (Japanese), 0043 (voices) | ⏳ direction set 2026-10-03; details open |
| Number scale & strike thresholds | [`stats-and-combat.md`](stats-and-combat.md) | 0013 | ⏳ after playtest 0804 |

Each file starts with `Decided: YYYY-MM-DD`, `Source: ticket NNNN`, and Nick's
words verbatim, followed by the derived rules (numbers marked *tunable* where
Claude chose a starting value).
