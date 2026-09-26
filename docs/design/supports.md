# Supports and relationships

Decided: 2026-09-25
Source: ticket 0010

## Nick's words

> **Q1. How are personal stories told?** "I do like the hub activities but I
> agree it can lead to bloat + dating sim vibes. So I think a mix of A and C
> can work. Fire Emblem newer entries also raise support between healer /
> buffer and others when the healer / buffer uses an ability on the other. We
> can take that idea too. Hub activities we can add much later when we've
> refined the core loop. It should be a ticket in the far future or maybe
> never implemented due to scope explosion."
>
> **How A (earned supports) and C (camp events) split the work:** "Supports
> everywhere, camp for extras"
>
> **Q2. Support limits:** "Unlimited"
>
> **Battle effect of a support:** "for now small flat bonus is OK but pair
> abilities might be added far in the future"

Options he was shown: A = FE GBA / Path of Radiance supports, B = Three Houses
hub activities, C = Triangle Strategy / Unicorn Overlord camp events, D = main
script only.

## Summary

- **Earned supports (FE GBA style) carry the personal stories**, including
  the lead's bonds with companions. Pairs build points by fighting together,
  and by one healing or buffing the other (Nick, from newer FE games).
- **Camp events (Triangle Strategy style) are extras**: short, optional,
  scripted scenes between chapters. They don't need grinding and have no
  mechanical effect.
- **Unlimited A-ranks** per unit.
- **Small flat Hit/Avoid bonus** when supported partners are close.
- **Not now:** hub activities (far future, maybe never) and pair abilities
  (far future). Each is a parked ticket (see below).
- **No romance** (from `setting-and-tone.md`, 0007). Supports are friendship,
  rivalry, mentorship and family. There are no S ranks, no marriage and no
  shipping. A couple that already exists before the story starts may appear,
  but it is never developed into a romance on screen.

## Rules: earned supports

### Pairs

- A pair can build support only if it is **listed in the content data** with
  its conversations. Not every pair of units has a support (as in FE GBA).
  Which pairs exist, and what their conversations say, is decided by the
  story pipeline (`story-writing` skill, character sheets in 0701 and later
  content tickets), not here.
- **The lead has supports too**: lead ↔ companion pairs are normal pairs. In
  them the lead follows the Persona-style lead rules in `setting-and-tone.md`:
  few lines, and at most one reply choice per conversation (*tunable*). The
  reply changes a line or two of reaction, never the rank or the points.
- The same pair can't have two rank tracks. Support is symmetric: A↔B is one
  value.

### Ranks and thresholds

Ranks are **C → B → A**. Points are per pair and cumulative (*tunable*):

| Rank | Points needed |
| ---- | ------------- |
| C | 20 |
| B | 50 |
| A | 90 |

- Reaching a threshold **unlocks** that rank's conversation. The rank is
  **gained only when the conversation is viewed** (FE GBA).
- While a conversation is unlocked but not viewed, the pair's points stop at
  that threshold (extra points are lost). This stops a pair jumping two ranks.
- **At most one rank gained per pair per chapter** (*tunable*, from Path of
  Radiance). If a pair reaches the next threshold in the same chapter, that
  conversation unlocks at the start of the next chapter.
- **Unlimited A-ranks** (Nick): a unit can reach A with every partner that it
  has a support with.
- A pair may give a thresholds override in data (e.g. a lifelong-friends pair
  that starts at C, or a slow-burn rivalry needing more points). Default is
  the table above.

### Gaining points (all *tunable*)

A pair gains points only when **both units are deployed and on the map**
(alive and not retreated). The events below are counted per pair.

| Event | Points |
| ----- | ------ |
| At the end of the player phase, the two units are **adjacent** (4 directions, like attack range 1) | +1 |
| A unit **fights** (attacks or is attacked, any range) while its partner is adjacent to it | +3 |
| A unit **heals** its partner with a spell or skill (e.g. Heal, Sanctuary) | +3 |
| A unit **buffs** its partner with a skill (e.g. War Cry) | +3 |
| A unit **uses an item on** its adjacent partner (e.g. a potion, `weapons-and-items.md`) | +2 |

- A heal or buff that covers several allies (Sanctuary, War Cry) gives the
  points to each pair (caster, ally) it affects.
- Healing, buffing or using an item counts whenever the game allows the
  action. Nothing counts for an ally that isn't in a support pair with the
  caster.
- Rough pace: a pair that fights side by side and ends most turns adjacent
  gets about 8–12 points per chapter, so C after ~2 chapters and A after
  ~8–10. A healer who heals the same ally often gets there faster.
  `stats-and-combat.md`'s tuning work (0013/playtest) can change these.
- Rewind (`death-and-difficulty.md`) rolls support points back like every
  other battle state change (they are ordinary events).

### Battle bonus

When a unit fights, it gets a bonus for **each supported partner within 3
tiles** (Manhattan distance, *tunable*) that is on the map:

| Partner's rank with this unit | Hit | Avoid |
| ----------------------------- | --- | ----- |
| C | +5 | +5 |
| B | +10 | +10 |
| A | +15 | +15 |

- Bonuses from several partners **add up**, with a total cap of **+20 Hit
  and +20 Avoid** (*tunable*; the cap keeps unlimited A-ranks from breaking
  the hit maths in `stats-and-combat.md`).
- The bonus applies to attacking and to defending (counters).
- The forecast shows the bonus as part of the numbers, and the unit info
  screen lists the unit's supports and ranks. How it looks is up to the UI
  tickets.
- No other stat is affected. **Pair abilities** (Awakening-style dual attacks
  or dual guards) are a possible far-future addition, not part of this system.

### Death and retreat

- **Classic:** when a unit dies, its supports stop. Unlocked but unviewed
  conversations with it are removed. Viewed conversations stay in the
  "seen" list.
- **Casual:** a retreated unit comes back next chapter, and its supports carry
  on unchanged (it gains no points for the rest of that battle).

### Viewing support conversations

*Claude's choice (presentation):* support conversations are viewed **between
chapters**, from a **Supports** list reached from the between-chapters flow
(beside Preparations), not during a battle. The list shows each unlocked
conversation (pair and rank) and marks new ones. They use the normal two-portrait dialogue player (0702/0704).

## Rules: camp events

- Short, **scripted, optional** conversations that appear in a **Camp** list
  between chapters once the story reaches a set point (e.g. "after Chapter 3,
  if Mira is alive").
- They need no points and give no points, stats or items. They're **extras**:
  flavour, banter, small world details, group scenes of 3+ characters that
  don't fit a pair.
- They may include the lead, with the same reply-choice rules as supports.
- Which camp events exist and when they appear is decided by the story
  pipeline, per chapter.
- Personal arcs **must not** depend on camp events alone. The arc lives in the
  main script and the character's supports (Nick: "Supports everywhere, camp
  for extras").

## Not in this system

- **Hub activities** (Three Houses: walk a base, meals, gifts). Nick: far
  future, "or maybe never". Parked in ticket 1004.
- **Pair abilities** (dual attack/guard). Parked in ticket 1005.
- **Romance, S ranks, marriage, children.** Ruled out by 0007.

## Open sub-questions (deferred)

- **Endings:** does each unit get a solo epilogue line, and do A-rank pairs
  get a paired ending (FE GBA)? With unlimited A-ranks, paired endings need a
  rule (e.g. "the last A-rank viewed"). Ask Nick when the ending is written.
- **Support log / replay:** whether seen conversations can be re-read from a
  menu. Default: yes, in the Supports list (Claude's choice; not a design
  question unless Nick objects).
- The point values, thresholds and bonus numbers above are starting values,
  to be tuned after playtests.

## Implementation tickets

- **1002** Support points, ranks and battle bonus (core rules + data).
- **1003** Supports and camp lists between chapters (UI + dialogue).
- **1004** Hub activities (parked, far future / maybe never).
- **1005** Pair abilities (parked, far future).
