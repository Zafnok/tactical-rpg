# Battle scenes, talking and recruitment

Decided: 2026-09-29
Source: ticket 0705 (review of its starting rules)

## Nick's words

> 1. unit recruitment should not happen during the same battle but be
> recruited for use for after the battle, revise any tickets or notes that
> contradict this
>
> 2. talking should trigger a pre-written script, it does not matter who
> initiates but will trigger the script and whoever is supposed to start
> talking will begin as the first speaker according to script participants
>
> 3. only certain ppl should be recruitable so this reads to me like you
> think everyone is recruitable... lets say 5 brigands are trash and one is a
> leader and somehow you recruit them I guess this can hold... but tbh I
> think most recruitment triggers should be via defeating them or completing
> a quest which happens outside of battle
>
> 4. sure, most SRPG games just have a set number of lines for a boss, like
> first attack/on attacked, halfway mark, and defeated
>
> 7. sure
>
> 8. sure

Then, on the follow-up questions:

> 1A
>
> 2A - but these should not play on every battle or it will be
> repetitive... again they should be triggered once per moment like first
> battle, first time at half hp, or defeat
>
> 3A

(1A: a unit talked into joining leaves the battlefield at once. 2A: a
boss's special line for a character replaces its general line. 3A: an area
scene plays only where a unit ends its move.)

## Rules

### Recruitment

- **Only characters a map marks as recruitable** can be recruited. Every
  other unit is just an enemy.
- **Nobody changes sides during a battle.** A recruited character leaves the
  battlefield and **joins the army after the battle is won** (0801 adds them
  to the roster when the battle's result is applied). Losing or restarting
  the battle undoes it, like anything else in that battle.
- Ways to recruit:
  1. **Defeat them** (Nick expects this to be the usual way): a map marks an
     enemy as "joins you if defeated". When it falls it says its line,
     leaves the map, and joins after the battle.
  2. **Talk to them**: a map marks a pair of characters who can talk. The
     talk plays its scene; if it recruits, the recruit **leaves the
     battlefield at once** (1A). It no longer counts as an enemy, so it
     counts toward "defeat all enemies".
  3. **Quests outside battle**: later, with the world map (see
     `world-structure.md`); ticket 1009.
- *Claude's starting rule:* if the map's goal is to defeat one enemy and you
  recruit that enemy instead, the goal is met.

### Talking

- A map lists which two characters can talk. **Either one may start it**
  (the player picks `Talk` with whichever of the two is their unit, next to
  the other). It always plays the same pre-written scene; who speaks first
  is up to the script.
- Talking uses up the talker's action and gives no EXP (*Claude's starting
  rule*). A talk that recruits happens once.

### Boss lines

- A boss has a set of lines for **moments**, each played **once per
  battle**: the first fight (attacking or attacked), the **first time its HP
  drops to half or less** (the line plays right after that fight; not if the
  fight defeats it), and when it is defeated (before it leaves the map).
- A boss may have **special lines for certain characters** (e.g. Harl and
  the lead). When that character fights it, the special line plays
  **instead of** the general one, and the general line never plays against
  that character (2A). Each line still plays only once.
- When both fighters have a line for the fight, the attacker's plays first
  (*Claude's starting rule*).

### Other scene moments

- **Turn start:** a scene can play when a given turn's phase starts, after
  its banner.
- **Map areas:** a scene can play when a unit (a certain character, or any
  unit of a side) **ends its move** in an area; walking through, being
  pushed in or arriving there as a reinforcement doesn't count (3A).
- **Falling:** a unit's fall line plays before it leaves the map; a player
  unit can have a Classic death line and a Casual retreat line
  (`death-and-difficulty.md`).
- **Skipping:** skipping a fight's animation still stops for its lines; the
  scene itself can be skipped with its own skip key.

## Open sub-questions

- Quest recruitment (outside battle): ticket 1009, with the world map.
