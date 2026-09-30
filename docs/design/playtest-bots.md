# Playtest bots: player types, targets, autobalancing

Decided: 2026-09-30
Source: ticket 0033

## Nick's words

Original idea (before the ticket):

> "the casual is able to clear (assuming they play on casual mode) with
> probably some fallen units (which will come back since they play on casual
> mode). The normal player probably clears with none but takes a while and
> uses many consumables and the hardcore can clear quickly without many
> consumables used"

Also decided before the ticket: **bots never use rewind.**

Answers:

> **Q1–Q3** "1A 2A 3A but give statistics on how many units died/retreated
> for each epoch and attempt..."
>
> **Q4** "I'm thinking maybe 60% winrate is good enough... no consequences
> for death means it should still be a bit tight for the player... the # of
> deaths does not influence bot at all"
>
> **Q5** "nobody dying in 5/10 with max 3 items used"
>
> **Q6** "nobody dying in 3/10 with max 1 item used"
>
> "also -- we should ensure that we can backwards infer the level of
> difficulty onto a skirmish from these results if at all possible. Meaning, I
> tell you I want a skirmish on grid 10,10 of open world map that's lv35
> scaled difficulty, made up of a mix of units, you can generate one meeting
> that. Or a skirmish on grid 13,13 lv40 scaled difficulty showing up in ch2
> only made up wholly of mages, you can generate one for that."
>
> **Floor or band** "band"
>
> **Hardcore speed** "the point of automated balancing is I don't want to
> manually do this stuff.... so if there is a way to make it so hardcore
> player is able to do it much better then that's ok..."
>
> "also I realized.. I guess hardcore should play better than normal, and we
> don't have a fixed hard mode yet. So I think maybe hardcore should have a
> 50% WR too but it's more for seeing where veterans would end up"
>
> **Q7** "B to start, maybe C later" (targets per map tier; per-battle
> overrides maybe later)
>
> **Q8** "A" (no bot knows about surprise reinforcements)
>
> **Q9** "C - but autobalancing should mean this never happens. I will tell
> you I think the map is empty and I want a skirmish on this part of the grid
> and you will put one that makes sense for the lv I think of."
>
> **Q10** (which army the bots bring to a generated skirmish) "if I tell you
> put a lv35 battle here that means a lv35 battle...?"
>
> **Q11** (what autobalancing may change) "C for random skirmishes, A for
> fixed skirmishes (ie side quests)"

## The three player types

A **try** is one play of a battle from its first turn with fresh luck. A bot
plays each try once, never rewinds and never restarts (Nick, Q3: first try
only).

| Bot | Plays in | What it cares about while playing | A try is a **success** when… |
| --- | -------- | --------------------------------- | ---------------------------- |
| **Casual** | Casual mode | Winning and keeping the lord alive. **Other units falling doesn't matter to it at all** (Nick: "the # of deaths does not influence bot at all"). | it wins |
| **Normal** | Classic mode | Nobody dying. Spends items when that helps; doesn't hurry. | it wins, **nobody dies**, and it used **at most 3 items** |
| **Hardcore** | Classic mode | Nobody dying, few turns, few items. **Plays better than Normal** (a bigger thinking budget). It stands in for veterans, since there is no Hard mode yet. | it wins, **nobody dies**, and it used **at most 1 item** |

"Items" means consumables used by player units (Vulneraries, antidotes, …),
counted per try.

## Targets: a band per map tier

Each bot's **success rate** over many tries (100 by default) must land in a
band. Below the band = too hard (✗); above it = too easy (⚠). Nick picked
bands over floors because a battle "should still be a bit tight" even in
Casual mode.

Bands depend on the map's tier (the same tier that sets rewind charges,
`death-and-difficulty.md`). Nick fixed the **Normal tier** row; the other rows
are *Claude's starting values, tunable*, and per-battle overrides may come
later (Nick: "maybe C later").

| Map tier | Casual | Normal | Hardcore |
| -------- | ------ | ------ | -------- |
| Easy (e.g. random skirmishes) | 75–100% | 65–90% | 65–90% |
| **Normal** | **60**–85% | **50**–75% | **50**–75% |
| Hard | 55–80% | 40–65% | 40–65% |
| Finale | 50–75% | 35–60% | 35–60% |

The lower edges on the Normal row are Nick's (60%, 5/10, and Hardcore
raised from 3/10 to 50%). The upper edges (+25 points) are *Claude's starting
values, tunable*.

**Hardcore is faster than Normal (no par numbers).** Nick doesn't want to set
a turn target per map by hand. The report compares the two bots
automatically: Hardcore's median turns (over its successful tries) must be
**lower than Normal's**. This checks that the veteran really plays better; it
is not a per-map par. If Hardcore is not faster, that's flagged as a problem
with the bots, not the map.

## Surprise reinforcements

**No bot knows about them** (Q8 A). Every bot plays like someone seeing the
map for the first time: while planning, it can't see reinforcements that
haven't arrived yet. An unfair ambush therefore shows up in the numbers.

## What the report shows

Per bot, per battle:

```
ch01 · Normal tier · 100 tries (seeds 1–100)

Casual   · Casual  · won 64/100                            band 60–85%  ✓
           retreats per try  0: 30  1: 21  2: 25  3+: 24   most: Kael 41×
Normal   · Classic · won, nobody died, ≤3 items: 47/100    band 50–75%  ✗ too hard
           deaths per try    0: 55  1: 30  2: 12  3+: 3    most: Mira 22×
           items per try     median 2  max 7
           turns             median 14
Hardcore · Classic · won, nobody died, ≤1 item: 52/100     band 50–75%  ✓
           turns             median 10 (Normal 14)          faster ✓

try 17 · Normal · lost (lord fell T9) · died: Mira T6, Kael T8 · items 2
…
```

- **Every try** is listed: result, who fell (died or retreated) and on which
  turn, items used, turns (Nick: "statistics on how many units
  died/retreated for each … attempt").
- **History across runs** ("each epoch", read as each batch of runs): every
  run of the report is kept, so a change shows as "Normal success 47% → 58%
  after the change". *Claude's reading of "epoch"; Nick may correct it.*

## What a miss does

- **Report only** for ✗ and ⚠ (Q9 C).
- **Blocks the change** only when a battle is basically unwinnable: the
  **Casual bot succeeds in under 20% of tries** (*starting value, tunable*).
- Nick expects this never to happen, because battles are **autobalanced**
  (below).

## Autobalancing

Nick doesn't want to tune battles by hand. A dev tool changes a battle until
all three bots land in their bands for its tier. The bots' own settings are
never changed to make a battle pass; only the battle changes.

What it may change (Q11):

| Battle kind | What autobalancing may change |
| ----------- | ----------------------------- |
| **Random skirmish** | Anything: enemy levels, count, classes, gear, positions, reinforcements, and the terrain. |
| **Fixed skirmish, side quest** | Only the enemies' numbers: level, count, gear, and which classes appear. The map, named units (bosses, characters) and objectives stay as authored. |
| **Story battle** | Same as fixed skirmishes. *Claude's reading: Nick grouped fixed skirmishes with side quests; story battles need their script intact, so they get the same limits. Nick may veto autobalancing story battles at all.* |

## Generating a skirmish to order

Nick tells Claude where and what, in words, for example:

> "put a lv 35 skirmish on grid 10,10 of the world map, mix of units"
>
> "a lv 40 skirmish on grid 13,13, only in Chapter 2, all mages"

Claude generates it with a dev tool and adds it to the game data:

- **"Lv 35" means a lv 35 battle** (Nick): the enemies are around lv 35 (the
  node's level marker shows 35, `world-structure.md`), and the bots play it
  with **a lv 35 army**: the units the player would have recruited by that
  point in the story, at lv 35, with ordinary gear for that point.
- Unit mix, place on the grid, and which chapters it appears in are taken
  from the request.
- The tool autobalances it (random-skirmish rules above) until the three bots
  land in the bands for its tier (skirmishes are Easy or Normal,
  `world-structure.md`).

## Open sub-questions (deferred)

- Per-battle overrides of the bands (Nick: "maybe C later").
- A "first-timer" bot (Q1 B), if ever wanted.
- Whether the bands feel right: compared against Nick's own play in 0508.
