# Battle maps (`.map`)

Each `*.map` file here is one battle map, loaded by `trpg_content::map` and
validated by the all-assets test (ADR-0005). The file stem is the map's id
(`test_small.map` → `"test_small"`).

## Format

```
(
    name: "Test Field",
    legend: { '.': "plain", 'T': "forest", '^': "mountain", '~': "water", '#': "wall", 'F': "fort", '=': "bridge" },
)
---
....TT..~~..
..^^TT..==..
```

1. **Header**: a RON struct with
   - `name`: the map's display name;
   - `legend`: tile character → terrain id from `assets/data/terrain.ron`;
   - `features` (optional): tile `(x, y)` → a shop or a chest (below).
   RON `//` comments are allowed.
2. **Separator**: a line that is exactly `---`.
3. **Tiles**: one character per tile, one line per row, top row first. Every
   row must be as wide as the first. Blank lines at the end are ignored.

### Tile features

```
features: {
    (5, 3): Shop(kind: Armoury, stock: ["iron_sword", "leather_vest"]),
    (6, 3): Shop(kind: Vendor, stock: ["potion"]),
    (7, 3): Shop(kind: Blacksmith, stock: []),
    (8, 3): Chest(Gold(500)),
    (9, 3): Chest(Item("elixir")),
},
```

- **Shops** (rules in `trpg_core::shop`): an `Armoury` sells the weapons,
  armour and accessories on its `stock` list; a `Vendor` the consumables; a
  `Blacksmith` repairs and sells nothing (empty list). Armouries and
  vendors buy anything at half price.
- **Chests** hold `Gold(n)` or one `Item(id)`, opened once by a player unit.
- **Villages** are not supported yet: Nick deferred every building tile
  except `fort` (`docs/design/terrain.md`).

One character per tile is the *file* format only: on screen each tile is two
glyphs wide, taken from the terrain's `glyphs` in `terrain.ron` (ADR-0012).

## Rules checked by the loader

Each is reported with `file:line:column`, and all are reported at once:

- missing `---` line, or a RON syntax error in the header;
- a legend entry naming a terrain id that `terrain.ron` doesn't define;
- a tile character that isn't in the legend;
- a row whose width differs from the first row (ragged rows);
- no tiles at all (empty map) or an empty first row;
- a map wider or taller than 64 tiles;
- a feature outside the map, on a tile no movement type can enter, a shop
  with an empty list (or a blacksmith with a list), a chest with 0 gold;
- an unknown item id, or a shop listing an item its kind doesn't sell.

Unused legend entries are allowed.
