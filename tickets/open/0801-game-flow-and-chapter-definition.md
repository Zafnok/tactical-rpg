---
id: "0801"
title: "Game flow: chapter definition file, campaign state, title → story → battle → result"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: todo
blocked_by: ["0405", "0408", "0502", "0705"]
nick_input: none
completed:
---

# 0801 — Game flow and chapter definitions

## Context

Connects everything into a game: a chapter file declares its map, units,
objective, scenes and triggers; a `Campaign` carries the roster between
chapters; screens flow from the title through story and battle to the result.

## Nick input

None.

## Scope

**In:** `assets/chapters/*.ron` format + loader/validator, `core::campaign::Campaign`,
`ui::flow` (chapter sequencing), Classic/Casual mode select, map-menu `Restart battle`, Game Over screen, "To be continued" screen,
replacing the title's placeholder and debug Quick Battle wiring.

**Out:** saving (0802), Chapter 1 content itself (0803), world map (future).

## Implementation steps

1. **Chapter file** (document in `assets/chapters/README.md`):
   ```ron
   (
     id: "ch01", title: "Chapter 1: …",
     map: "maps/ch01.map",
     intro_scenes: ["ch01_intro", "ch01_prebattle"],
     player_slots: [ (character: "ana", pos: (3, 10)), … ],   // roster members placed here
     enemies: [ (template: "brigand", level: 2, pos: (14, 4), ai: Aggressive, loadout: (weapons: ["iron_axe"], armour: None, accessory: None), consumable: None, boss: false, name: None), … ],
     preparations: true,                                       // show the 0408 Preparations screen
     pack_cap: 6, default_pack: ["potion", "potion"],          // shared battle pack (weapons-and-items.md)
     clear_gold: 500,
     objective: DefeatUnit("boss_id") | Rout | Seize((x, y)) | Survive(8),
     triggers: [ … ],                                          // 0705 trigger list
     victory_scenes: ["ch01_victory", "ch01_tbc"],
     next: Some("ch02") | None,
     rewind_charges: 3,                                        // per map, by intended difficulty (0006: easy 2 / normal 3 / hard 5)
     seed: 12345,
   )
   ```
2. Validator: positions in bounds, on terrain passable for that unit's movement
   type, no overlaps; all ids exist (characters, templates, items, scenes);
   objective target exists.
3. `core::campaign::Campaign { mode: GameMode /* Classic | Casual */, chapter: String, roster: Vec<Unit>, stock: Stock, gold: u32, flags: BTreeMap<String, bool>, playtime_s: u64 }`
   (serde). `Campaign::new_game()` with the starting roster;
   `Campaign::battle_setup(&ChapterDef) -> BattleSetup`;
   `Campaign::apply_result(&BattleState)` updates roster (levels, loadouts,
   weapon ranks, durability), handles fallen player units per
   `death-and-difficulty.md` (Classic: removed from the roster, equipped items
   to the stock; Casual: kept), returns unused pack items to the stock and adds gold.
4. `ui::flow`: `New Game` → `ModeSelectScreen` (Classic / Casual, one line
   explaining each) → intro scenes → (`PreparationsScreen` from 0408 if
   `preparations: true`, else the default pack) → `BattleScreen` → on `BattleEnded`
   victory → victory scenes → (0802 save prompt hook) → next chapter or
   `ToBeContinuedScreen` → title. Defeat → `GameOverScreen` (`Retry chapter` / `Title`). Add
   `Restart battle` (with confirm) to the map menu. Both restarts rebuild the
   battle from its setup, which refunds all rewind charges.
5. Title menu: `New Game`, `Quit` (+ debug-only `Quick Battle`, F12 tools).
   Remove `PlaceholderScreen`.
6. A tiny test chapter `assets/chapters/test.ron` (on `test_small.map`) used by tests.

## Acceptance criteria

- [ ] Harness: New Game on the test chapter → skip scenes → win via scripted commands → victory scene → "To be continued" → title.
- [ ] Defeat path → Game Over → Retry restarts the chapter with identical state and full rewind charges.
- [ ] Map-menu `Restart battle` → confirm → same result as Retry.
- [ ] `apply_result`: a fallen unit is removed in Classic and kept in Casual (tests).
- [ ] Validator errors tested.
- [ ] Campaign round-trips through serde.

## Tests required

- Unit: validator, `battle_setup`, `apply_result`.
- Harness: the two flows above.

## Completion notes

