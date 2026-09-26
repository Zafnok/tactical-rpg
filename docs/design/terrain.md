# Terrain

Decided: 2026-09-26 (movement costs); combat bonuses still under review
Source: ticket 0301 (Nick revisited the player-facing choices of that ticket)

## Nick's words

> "Idk I didn't even mention terrain defense etc so that's not really already
> mine. The four movement types I'm ok with though."
>
> **Q1. How much rough terrain slows units** (options: heavy like FE GBA /
> light like Three Houses / type-driven like Advance Wars): "1A with tuning. I
> think horses shouldn't be better in every scenario compared to foot units.
> Flyers I guess they'll be late game? Like tier 3+. We can adjust the class
> progression to make that work. But for terrain cost maybe mountains are only
> 3 on foot and maybe horses can climb mountains but it costs like 5. Rest of
> numbers seem ok."
>
> **Q2. Can foot units wade rivers** (wadeable but slow like FE7 / impassable
> / shallow ford like FFT): "2A"
>
> **Q3. What fliers can pass over** (everything but walls like FE GBA /
> slowed by some terrain like Three Houses / everything like Advance Wars):
> "3B but no weather for now just peaks"

## Rules

### Movement types

`foot`, `mounted`, `armored`, `flying` (Nick; the class list in
`progression.md` uses these).

### Movement costs

Move points to enter one tile; `–` = can't enter. Heavy, Fire Emblem GBA
style: terrain shapes routes (Nick, Q1). Numbers Nick set are **bold**; the
rest are FE GBA values Nick accepted ("rest of numbers seem ok"). All
*tunable*.

| Terrain | Foot | Mounted | Armored | Flying |
| ------- | ---- | ------- | ------- | ------ |
| plain, road, bridge, floor, village, gate, throne | 1 | 1 | 1 | 1 |
| forest | 2 | 3 | 2 | 1 |
| mountain | **3** | **5** | – | 1 |
| peak | – | – | – | **3** |
| fort | 2 | 2 | 2 | 1 |
| water (river, shallow) | 5 | – | – | 1 |
| sea | – | – | – | 1 |
| thicket, wall, door | – | – | – | – |

- **Horses are not better everywhere** (Nick): mounted units are fastest on
  open ground but pay more than foot units in forest (3 vs 2) and mountains
  (5 vs 3), and can't enter rivers.
- **Rivers** (Nick, Q2): foot units can wade at 5, which usually uses a whole
  turn; mounted and armored units can't enter. Bridges are the normal crossing.
- **Fliers** (Nick, Q3): cost 1 almost everywhere but are slowed by **peaks**
  (3, *Claude's starting value* inside Nick's "slowed by peaks"). No weather
  for now. Walls, doors and thickets still block them (*Claude's starting
  rule*, unchanged by Nick's answers).

### Terrain combat bonuses

The Def / Avoid / Heal % table in `stats-and-combat.md` ("Terrain combat
effects") is **Claude's starting values, not Nick's decision**. Nick is
reviewing it next; this section will record his answer.

## Open sub-questions

- **Fliers as late-game classes** (Nick: "tier 3+"): moving the Flier line
  out of tier 1 changes the class tree. Ticket 0017 asks this.
- Terrain combat bonuses (above).
- Weather (Nick: "no weather for now").
