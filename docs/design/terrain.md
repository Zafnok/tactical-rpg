# Terrain

Decided: 2026-09-26 (healing tiles and capturing deferred)
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
>
> **Q4. How good terrain helps in a fight** (avoid and a bit of defence like
> FE GBA / defence only like Advance Wars / minimal like FFT): "1A"
>
> **Q5. Healing tiles** (forts, gates, thrones heal like FE GBA / only your own
> buildings like Advance Wars / none): "2 I think we can defer healing for
> now, since the idea of capturing sounds interesting, but I also don't think
> I ever explicitly signed off on a fort, gate, or throne tile type either."
>
> **Q6. Do fliers get terrain bonuses** (no like FE GBA / yes): "3A"
>
> **Q7. Building tiles** (the FE set: fort, gate, throne, village / buildings
> only later as a capturable layer like Advance Wars / just one "fort" tile
> for now): "we can go w C for now. make a note that we might allow capturing
> later and the other building type tiles might come around then or even
> without capturing. but let's keep it simple for the first playtest."

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
| plain, road, bridge, floor | 1 | 1 | 1 | 1 |
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

Fire Emblem GBA style (Nick, Q4): good terrain makes a unit harder to hit
(avoid) and adds a little defence. The bonus applies to the unit standing on
the tile, whether it attacks or defends; `defense` adds to both Def and Res.
Numbers are FE GBA values, *tunable*:

| Terrain | Def | Avoid |
| ------- | --- | ----- |
| plain, road, bridge, floor | 0 | 0 |
| forest | +1 | +20 |
| mountain | +2 | +30 |
| peak | +2 | +40 |
| water, sea | 0 | +10 |
| fort | +2 | +20 |
| thicket, wall, door | — | — |

"—" = impassable, so it never matters in combat.

- **Fliers get no terrain Def/Avoid bonus** (Nick, Q6): they fly above the
  trees. This balances their mobility.

### Building tiles

**One building tile for now: `fort`** (Nick, Q7), a defensive building for
bosses and chokepoints: costs above, +2 Def / +20 Avoid, no healing. Kept
simple for the first playtest. **No village, gate or throne tiles yet.**

Later (Nick): capturing may be added, and more building tiles (village,
gate, throne, …) may come with it, or even without capturing.

### Healing tiles

**Deferred** (Nick, Q5): no tile heals for now. Nick finds the Advance Wars
idea of *capturing* buildings that then heal your side interesting, so
healing tiles are to be designed together with capturing later. The data
keeps a `heal_percent` field, set to 0 on every terrain.

## Open sub-questions

- **Fliers as late-game classes** (Nick: "tier 3+"): moving the Flier line
  out of tier 1 changes the class tree. Ticket 0017 asks this.
- **Capturing, healing tiles and more building tiles** (village, gate,
  throne, …) (Nick, Q5 and Q7), after the first playtest.
- Weather (Nick: "no weather for now").
