# World structure

Decided: 2026-09-26
Source: ticket 0008

## Nick's words

> **Q1. How does the player move through the game?** "A for ch 1-2 and maybe
> 3 -- but for playtest we're just doing ch1 for now. And probably B for rest
> of game unless we get a lot of pre-orders then we can scope creep to C. But
> let's keep it simple and not branch unless we get a lot of hype"
>
> **Q2. How do the other continents fit in?** "to keep it tight for balancing
> and a good flow I think A would actually work best. This is an SRPG/TRPG not
> a traditional open world RPG so we need to keep the pace in mind."
>
> **Q3. What's on the world map besides story battles?** "A + B + D. I don't
> think we need C to be open world but they might be inside towns? and E could
> be a very late addition like right before an act battle or the game finale.
> Or even post-game / NG+. It's something for later for sure."
>
> **Q4. How do skirmishes work?** "D but make sure the ones that are random vs
> the fixed ones are clearly delineated so that players know which ones are
> more important and which ones are just for grinding. Some symbol above them
> or a level marker or both."
>
> **Q5. When can the player save, and where is camp?** "A"

Options he was shown:

- **Q1:** A = linear chapters (FE Blazing Blade, Path of Radiance), B = world
  map with optional battles (FE Sacred Stones, Echoes), C = branching routes
  (Tactics Ogre, Triangle Strategy, Three Houses), D = sandbox (Battle
  Brothers, Mount & Blade).
- **Q2:** A = each continent is a new act with a fresh world map, B = one
  world map that grows, C = a late optional trip.
- **Q3:** A = towns with shops, B = optional skirmishes, C = recruitable
  wanderers, D = side quests with a character story (paralogues), E = a
  grinding tower (Tower of Valni).
- **Q4:** A = random spawns on cleared nodes (Sacred Stones), B = fixed
  optional battles that clear once (Echoes), C = random encounters while
  travelling (FFT), D = a mix of A and B with the enemy level capped per act.
- **Q5:** A = save anywhere on the world map and open camp from its menu
  (Sacred Stones, Echoes), B = save after every battle, C = save only at
  town or camp nodes.

## Summary

- **Chapters 1–2 (maybe 3) are linear:** story → one battle → story → next
  chapter. **The Chapter 1 playtest (0804) covers only Chapter 1.**
- **From then on, a world map with optional battles** (FE Sacred Stones
  style): story battles on nodes, towns with shops, fixed and random
  skirmishes, and character side quests (paralogues).
- **No branching routes.** C is only reconsidered if pre-orders and hype
  justify the extra scope, and that would need a new decision ticket.
- **Each continent is its own act with its own world map.** The pace stays
  tight: "This is an SRPG/TRPG not a traditional open world RPG."
- **Skirmishes: fixed ones plus random ones**, with the enemy level capped per
  act. The two kinds look clearly different on the map (symbol **and** level
  marker).
- **Save anywhere on the world map**, and open **camp** from the world map
  menu.
- **Later / maybe:** recruitable wanderers (maybe inside towns), and a
  grinding tower (very late: before an act's final battle, before the finale,
  or post-game / NG+).

## Rules: the two phases

### Linear opening (Chapters 1–2, maybe 3)

- A chapter = its story scenes + **one battle** (`chapter-1.md`).
- Saving works as in `death-and-difficulty.md`: "Save your progress?" after
  the chapter victory, plus the one-time suspend in battle.
- Between chapters the game flow (0801) offers what the design already calls
  for: camp (`supports.md`) and, from Chapter 2, Preparations with its shop
  (`chapter-1.md`, `weapons-and-items.md`).
- Whether Chapter 3 is linear or the first world-map chapter is decided with
  the story outline (0701 gate 2). Default: **the world map unlocks after
  Chapter 2**.

### World map (the rest of each act)

A chapter is still a **story beat** (`chapter-1.md`). On the world map, a
chapter's beat plays out over several map nodes and ends with the chapter's
**story battle** (usually the climax). Until that battle is won, the player
can freely use everything already open on the map.

**Node kinds**, each drawn with its own symbol:

| Node | What it is | Required? | Repeatable? |
| ---- | ---------- | --------- | ----------- |
| **Story battle** | The chapter's climax (or a mid-chapter story fight). Winning it advances the story and opens new paths. | Yes | No |
| **Fixed skirmish** | A hand-placed small battle. Some guard a path and must be won once to pass; others are optional. | Some are (guards) | No, cleared once |
| **Random skirmish** | A group that appears on an already-cleared node for training and loot ("just for grinding"). | No, the player can walk past | Yes, new ones keep appearing |
| **Side quest (paralogue)** | An optional, handcrafted battle tied to one character's story, with story scenes and a unique reward. Unlocked by story progress and conditions (e.g. that character is alive and recruited). | No | No |
| **Town** | Shop menu (Armory, Vendor, Blacksmith, as in `weapons-and-items.md`). May later hold recruitable wanderers. | No | Yes |
| **Empty / cleared** | A node you can pass through. | — | — |

**Telling skirmishes apart (Nick):** random skirmishes and fixed ones must be
clearly different at a glance, so players know which matter and which are
"just for grinding". Every battle node shows **both**:

- a **symbol** above the node that differs by kind (story battle, fixed
  skirmish, random skirmish, side quest), and
- a **level marker** (e.g. `Lv 5`): the enemy level, or the average enemy
  level for a group.

The exact glyphs and colours are a look-and-feel choice made in the UI ticket
(1007) and shown to Nick as a mockup (`ascii-art` skill).

**Random skirmishes** (*Claude's starting values, tunable*):

- After each story battle is won, **0–2** random groups may appear on
  cleared nodes of the current act's map. At most **3** random groups exist
  on the map at a time.
- Their enemy level is **capped per act** (Nick picked the capped option, so
  grinding can't run away). Starting cap: the level of the latest story
  battle won in this act, **−1**.
- They're small: about **4–6 enemies**, about **3–6 turns**. Map tier **Easy**
  (2 rewind charges, `death-and-difficulty.md`).
- Rewards: EXP from fighting, plus sometimes a little gold or a common item
  (*tunable*). Never unique items. Those come from story battles, fixed
  skirmishes and side quests.

**Fixed skirmishes** (*starting values, tunable*): each act's map has a few
(about **1–3 per chapter**). They're authored like story battles (map,
enemies, possibly a short scene) but smaller. Their tier is set in the
chapter data, normally Easy or Normal.

**Towns:** the shop inventory grows with the story. **The world map has no
shop screen outside towns.** The Preparations shop before a story battle
still follows `weapons-and-items.md` / 0408.

### Continents and acts

- **Each continent is a separate act with its own world map.** The first act
  is the European-style home continent (`setting-and-tone.md`).
- At the end of an act, the story moves the army to the next continent, and a
  new world map replaces the old one. **The player can't go back** to an
  earlier act's map. (Whether the finale revisits the home continent is a
  story choice for the outline, 0701.)
- An act can span several chapters on the same map. Not every act has to
  change continent. Acts and continents are laid out in the story outline
  (0701).
- *Claude's default, Nick may veto:* the whole army, stock, gold and support
  progress carry over to the next act, like one continuous campaign.
- Each act's map has its own enemy-level range, which is what the
  random-skirmish cap is measured against.

## Saving and camp

- **World map: save anywhere** (Nick, Q5 A). The world map menu has `Save`
  whenever no battle is running. It uses the same 30 slots and slot picker as
  `death-and-difficulty.md`.
- **Battles are unchanged:** no save mid-battle except the one-time
  `Suspend`, which is deleted on load.
- **Linear chapters are unchanged:** "Save your progress?" after the chapter
  victory.
- `Load Game` restores the campaign where it was saved: on the world map, at
  the saved node, or at the start of the next linear chapter.
- **Camp:** in the world-map phase, the world map menu has `Camp` (Supports
  list + camp events, `supports.md`), available whenever no battle is running.
  In the linear opening, camp is offered between chapters. Camp is also
  offered before each story battle's Preparations (1003).

## Implications for the story outline (0701)

- Act 1 = the home continent. Chapters 1–2 (maybe 3) are linear
  single-battle chapters. Then the Act 1 world map opens.
- For each world-map chapter, the outline names its **story battle**, the
  **fixed skirmishes** (and which ones guard paths), the **towns** that open,
  and any **side quests** (which character's story, unlock condition).
- Side quests are the natural home for companions' personal backstories, next
  to supports.
- Every act ends with a big story battle. A move to a new continent happens
  at an act boundary.
- No branching routes: one main story, one ending path.

## Not in this system (deferred)

- **Branching routes (C):** only if pre-orders and hype justify it, via a new
  decision ticket.
- **Recruitable wanderers:** not as open-world nodes. They might appear
  **inside towns** later. Decide with the story (0701 or later chapters).
- **Grinding tower / dungeon (E):** "something for later for sure". It could
  go right before an act's final battle, before the finale, or post-game /
  NG+. It needs a decision ticket when it's wanted.
- **Materials and forging** (`weapons-and-items.md`): still later. Towns,
  skirmishes or side quests are the likely sources.

## Open sub-questions (deferred)

- Is Chapter 3 linear or the first world-map chapter? Decided at 0701 gate 2.
- How many continents and acts, and in what order: 0701 outline.
- The random-skirmish numbers above (spawn count, level cap, size, rewards)
  are starting values. Tune them after the first world-map playtest. When
  they're known, retune the support thresholds (`supports.md`: "Retune B and
  A once it's known").

## Implementation tickets

- **1007** World map: nodes, travel, towns, save and camp from the map menu.
- **1008** World map skirmishes: fixed and random, with markers and level cap.
