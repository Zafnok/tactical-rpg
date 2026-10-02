# Battle files (`.ron`)

Each `*.ron` file here is one battle: everything needed to play it. They
are loaded by `trpg_content::battle` and validated by the all-assets test
(ADR-0005, ADR-0035), so adding a battle needs **only data**: a map, a
battle file and its scenes. The file stem is the battle's id
(`ch01.ron` → `"ch01"`). A chapter (`assets/chapters/`) names the battle it
plays; later the world map's battles and skirmishes (1007, 1008) will too.

## A full example

```ron
(
    id: "ch01",
    map: "ch01",                        // maps/ch01.map
    player_slots: [                     // roster members, placed here
        (character: "lead", pos: (3, 12)),
        (character: "bors", pos: (4, 13)),
    ],
    enemies: [
        (template: "brigand", level: Some(2), pos: (14, 4)),
        (template: "archer", pos: (15, 6), ai: Guard),
        (
            character: "garth", pos: (20, 2), ai: Stationary, boss: true,
            loadout: Some((weapons: ["steel_axe"], armour: None, accessory: None)),
        ),
    ],
    reinforcements: [
        (turn: 3, unit: (template: "brigand", pos: (23, 8))),
    ],
    preparations: false,                // true: the Preparations screen first
    pack_cap: 3,
    default_pack: ["potion", "potion", "potion"],
    clear_gold: 1000,
    objective: Rout(),
    triggers: [
        (when: CombatStart(unit: "garth"), scene: "ch01_garth", once: true),
    ],
    difficulty: Normal,
    music: Cue("battle_bright"),        // or Pool("skirmish")
    seed: 12345,
)
```

## Fields

| Field | Meaning |
| ----- | ------- |
| `id` | The battle's id: must match the file name. |
| `map` | A map id (`assets/maps/<id>.map`). |
| `player_slots` | Where each roster member goes: `(character, pos)`. A slot whose character isn't in the army (e.g. dead in Classic) stays empty; roster members without a slot sit the battle out. The first slot's unit is unit 1, the next unit 2, … A slot with `after_enemies: true` is numbered after the enemies instead (and before the reinforcements): for adding a unit to a battle without renumbering the others. |
| `enemies` | The other units on the map at the start (numbered after the slots). Each is a generic `template` (from `characters.ron`'s `generics`) **or** a named `character`, written bare (`template: "brigand"`). Optional: `level: Some(n)` (templates only; a character's level is its own), `ai` (`Aggressive` default, `Guard`, `Stationary`, `Healer`), `loadout: Some((weapons: [...], armour: Some(..), accessory: Some(..)))` instead of the template's or character's, `boss: true`, `name: Some("Garth")` (a display name; the map label becomes its first two letters). Enemies carry no consumables (`weapons-and-items.md`). |
| `reinforcements` | Units that arrive later: `(turn, unit: <an enemy entry>)`, numbered after the enemies. One on an occupied tile waits (`turn-structure.md`). |
| `preparations` | Whether the Preparations screen comes first (0408): the player changes the loadouts of the deployed units and fills the pack **from their own stock**, so such a battle has no `default_pack`. Default `false`. |
| `pack_cap` | How many consumables the pack may hold, written bare (`pack_cap: 3`). Default: `default_pack_cap` in `items.ron` (6). |
| `default_pack` | The consumables brought in without Preparations (given for free). Must be empty with `preparations: true`. |
| `solo_bench` | Characters in the army but not in the battle when it is played on its own (the Quick Battle): Preparations lists them so their gear can be traded. Ignored in the story. Default: empty. |
| `solo_stock` | The stock when the battle is played on its own (the debug Quick Battle): item ids, one entry per item. Ignored in the story, where the stock is the campaign's. Default: empty. |
| `clear_gold` | Gold for winning. Default 0. |
| `objective` | `Rout()`, `DefeatUnit(unit: "garth")` (a character among the enemies or reinforcements), `Seize(pos: (x, y), by_lord: true)`, `Survive(turns: 8)`. The first three take an optional `turn_limit: Some(n)`. |
| `triggers` | The battle's story moments (0705, ADR-0030): `(when: …, scene: "id", once: true)`, with `when` one of `TurnStart(turn, phase)`, `UnitEntersArea(who: Character("id") \| Faction(Player), area: (x, y, w, h))`, `CombatStart(unit, against: Some("id"))`, `HalfHp(unit)`, `UnitFell(unit, mode: Some(Classic), recruit: true)`, `Talk(a, b)`. |
| `difficulty` | The map's tier: `Easy`, `Normal`, `Hard`, `Finale` → 2 / 3 / 5 / 8 rewind charges (`death-and-difficulty.md`). |
| `music` | **Required.** What plays from the battle's start to its end, through both phases (`docs/design/audio.md`): `Cue("battle_bright")`, one music cue of `assets/audio/audio.ron` (a story battle's chosen theme), or `Pool("skirmish")`, one of its pools, from which a track is picked at random each time the battle starts (skirmishes). Restarting or retrying the battle picks again; rewinding never changes the track. |
| `seed` | Seed of the battle's random numbers. |

## Rules checked by the loader

All reported at once, naming the entry (`player slot 2 ("bors")`,
`enemy 3`, `reinforcement 1`):

- the id doesn't match the file name; an unknown map;
- `preparations: true` with a `default_pack`; an unknown `solo_stock` item; a `solo_bench` character that is unknown, in the battle or listed twice;
- an unknown character or template; an enemy with both or neither;
- a level outside `1..=level_cap`, or a level given for a character;
- a loadout the unit can't carry;
- a unit outside the map, or on a tile its class can't stand on;
- two units on one tile at the start (a reinforcement may wait on one);
- a character placed twice;
- a reinforcement on turn 0;
- a `DefeatUnit` character that isn't an enemy here, a seize tile outside
  the map, `Survive(turns: 0)`, a turn limit of 0;
- a default pack over the cap, with an unknown item or a non-consumable;
- no `music`, or one that isn't a music cue (`Cue`) or a pool (`Pool`) of
  the audio manifest;
- two named units of one side with the same map label (ADR-0018; fix with a
  `map_label` in `characters.ron`); generic units may share one;
- every trigger problem `trpg_content::check_triggers` finds (unknown
  scene, a character not in the battle, an area off the map, …).

The test files `test.ron` (the test chapter's battle) and `quick.ron` (the
debug Quick Battle) are placeholders until Chapter 1 (0803).
