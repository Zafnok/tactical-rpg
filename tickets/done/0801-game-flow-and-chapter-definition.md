---
id: "0801"
title: "Game flow: chapter definition file, campaign state, title → story → battle → result"
type: feature
milestone: M7 Chapter 1 & game flow
model: opus-5.5
effort: high
status: done
blocked_by: ["0405", "0502", "0705", "0307", "0708"]
nick_input: none
completed: 2026-09-30
---

# 0801 — Game flow and chapter definitions

## Context

Connects everything into a game: a chapter file declares its map, units,
objective, scenes and triggers; a `Campaign` carries the roster between
chapters; screens flow from the title through story and battle to the result.

## Nick input

None.

## Scope

**In:** `assets/battles/*.ron` and `assets/chapters/*.ron` formats + loaders/validators, `core::campaign::Campaign`,
`ui::flow` (chapter sequencing), Classic/Casual mode select, lead gender select, map-menu `Restart battle`, Game Over screen, "To be continued" screen,
replacing the title's placeholder and debug Quick Battle wiring.

**Not blocked by 0408** (changed 2026-09-29): Chapter 1 has no Preparations
screen (`chapter-1.md`), so the flow only needs the `preparations` field and
the default pack. If 0408 isn't done when this lands, the validator rejects
`preparations: true` with a clear message ("Preparations screen not built
yet, ticket 0408"), and the flow always uses the default pack. 0408 then
pushes its screen from the flow and removes that check.

**Out:** saving (0802), Chapter 1 content itself (0803), world map (future),
chapters with several battles (world map and skirmishes: 1007, 1008 per
`world-structure.md`). Chapter 1 is one battle, so one chapter file = one battle here.

## Implementation steps

1. **Battle file and chapter file**, kept separate so a battle can be reused
   outside a linear chapter. World-map story battles, fixed and random
   skirmishes and side quests (1007/1008, `world-structure.md`) are all battle
   files that aren't one-per-chapter. Document both formats in
   `assets/battles/README.md` and `assets/chapters/README.md`.
   **Battle file** (`assets/battles/ch01.ron`), everything needed to play one
   battle:
   ```ron
   (
     id: "ch01",
     map: "maps/ch01.map",
     player_slots: [ (character: "ana", pos: (3, 10)), … ],   // roster members placed here
     enemies: [ (template: "brigand", level: 2, pos: (14, 4), ai: Aggressive, loadout: (weapons: ["iron_axe"], armour: None, accessory: None), consumable: None, boss: false, name: None), … ],
     preparations: true,                                       // show the 0408 Preparations screen
     pack_cap: 6, default_pack: ["potion", "potion"],          // shared battle pack (weapons-and-items.md)
     clear_gold: 500,
     objective: DefeatUnit("boss_id") | Rout | Seize((x, y)) | Survive(8),
     triggers: [ … ],                                          // 0705 trigger list
     difficulty: Normal,                                       // Easy | Normal | Hard | Finale → rewind charges 2 / 3 / 5 / 8 (0006)
     seed: 12345,
   )
   ```
   **Chapter file** (`assets/chapters/ch01.ron`), the story beat around it:
   ```ron
   (
     id: "ch01", title: "Chapter 1: …",
     intro_scenes: ["ch01_intro", "ch01_prebattle"],
     battle: "ch01",                                           // id in assets/battles/
     victory_scenes: ["ch01_victory", "ch01_tbc"],
     next: Some("ch02") | None,
   )
   ```
   Adding a battle must need **only data files** (map + battle file +
   scenes), with no code change. The test in step 2 runs over every battle
   file.
2. Validator (every battle and chapter file): positions in bounds, on terrain passable for that unit's movement
   type, no overlaps; all ids exist (characters, templates, items, scenes);
   objective target exists. Map labels: build the battle's units and run
   `trpg_content::check_map_labels` (added in 0401, not yet called on real
   chapter data). Two named units of one faction on a map must not share a
   two-letter label (ADR-0018; fix with a `map_label` override in
   `characters.ron`). Generic units may share labels.
3. `core::campaign::Campaign { mode: GameMode /* Classic | Casual, `trpg_core::GameMode` from 0705 */, lead: LeadProfile /* 0708 */, chapter: String, roster: Vec<Unit>, stock: Stock, gold: u32, flags: BTreeMap<String, bool>, playtime_s: u64 }`
   (serde). `Campaign::new_game()` with the starting roster;
   `Campaign::battle_setup(&BattleDef) -> BattleSetup`;
   `Campaign::apply_result(&BattleState)` updates roster (levels, loadouts,
   weapon ranks, durability), handles fallen player units per
   `death-and-difficulty.md` (Classic: removed from the roster, equipped items
   to the stock; Casual: kept, full HP for the next battle), gives every deployed
   player unit (standing or retreated) 7% of one level's EXP per unused
   rewind charge (per the design; express it as a percentage of the
   EXP-per-level constant, not a fixed number), returns
   unused pack items to the stock and adds gold. On a **victory** it also
   adds every unit in `BattleState::recruited()` (0705: "joins you if
   defeated") to the roster at full HP
   (`docs/design/battle-scenes-and-recruitment.md`: recruits join only after
   the battle). The battle file's `triggers` go into `BattleSetup` and the
   loader runs `trpg_content::check_triggers` on them.
   `Campaign::downgrade_mode()` allows Classic → Casual only.
4. `ui::flow`: `New Game` → `ModeSelectScreen` (Classic / Casual, one line
   explaining each) → `LeadSelectScreen` (pick the lead's gender, showing the
   `lead_m`/`lead_f` portraits, plus a first-name entry: Nick wants renaming
   (0701 gate 1). Default first name **Ellery** (from the names table, see
   `docs/story/names.md`); the family name "Veyne" isn't editable;
   `setting-and-tone.md`) → intro scenes, with dialogue rendered using
   `campaign.lead` → (`PreparationsScreen` from 0408 if
   `preparations: true`, else the default pack) → `BattleScreen` → on `BattleEnded`
   victory → victory scenes → (0802 save prompt hook) → next chapter or
   `ToBeContinuedScreen` → title. Defeat → `GameOverScreen` (`Retry chapter` / `Title`). Add
   `Restart battle` (with confirm) to the map menu. Both restarts rebuild the
   battle from its setup, which refunds all rewind charges.
5. Title menu: `New Game`, `Quit` (+ debug-only `Quick Battle`, F2 tools).
   Remove `PlaceholderScreen`. Today's wiring (0401):
   `TitleScreen::with_quick_battle()` is chosen by `Game::start` when
   `Ctx::debug_tools` is on, and it pushes a `BattleScreen` built by
   `ui::screens::battle::quick_battle`. Keep that item working, but route it
   through the new flow (e.g. via `assets/chapters/test.ron`).
6. A tiny test chapter `assets/chapters/test.ron` + battle `assets/battles/test.ron` (on `test_small.map`) used by tests.

## Acceptance criteria

- [x] Harness: New Game on the test chapter → skip scenes → win via scripted commands → victory scene → "To be continued" → title.
- [x] Defeat path → Game Over → Retry restarts the chapter with identical state and full rewind charges.
- [x] Map-menu `Restart battle` → confirm → same result as Retry.
- [x] `apply_result`: a fallen unit is removed (gear to stock) in Classic and kept in Casual (tests).
- [x] `apply_result`: unused-charge EXP goes to every deployed unit, including Casual retreats, and not to undeployed units (tests).
- [x] `apply_result`: recruits join the roster after a victory, not after a defeat (tests).
- [x] Chapter `difficulty` maps to 2 / 3 / 5 / 8 charges (test).
- [x] Validator errors tested.
- [x] Harness: New Game → pick the female lead → the test chapter's intro renders her name and pronouns (0708 tokens).
- [x] Campaign (including `lead`) round-trips through serde.

## Tests required

- Unit: validator, `battle_setup`, `apply_result`.
- Harness: the two flows above.

## Completion notes

**Done.** New Game now runs a real game flow; ADR-0033 records the design.

- **Files** (data only, no code per battle): `assets/battles/*.ron` and
  `assets/chapters/*.ron`, plus `assets/data/new_game.ron` (first chapter,
  starting roster, gold, stock). Formats in `assets/battles/README.md` and
  `assets/chapters/README.md`. Loaded and validated by
  `trpg_content::battle` / `trpg_content::chapter` with every other asset
  (every error at once, each with a tested message; map labels via
  `check_map_labels`, triggers via `check_triggers`).
- **`core::campaign`**: `Campaign` (serde), `BattleDef`, `Difficulty`
  (2/3/5/8 charges), `GameTables`, `battle_setup`, `apply_result` (returns
  `BattleRewards` for 0810), `downgrade_mode`.
- **`ui::flow::FlowScreen`** hosts mode select → lead select (gender with
  the `lead_m`/`lead_f` portraits, first name on a letter grid, "Veyne"
  fixed) → intro scenes → battle → victory scenes → next chapter or "To be
  continued" → title; defeat → Game Over (`Retry` / `Title`). Map menu has
  `Restart Battle` with a confirm. The placeholder screen is gone; the
  debug Quick Battle is now `assets/battles/quick.ron` played as chapter
  `quick` through the same flow.
- Placeholder content: a test chapter (`test`): an intro scene using the
  lead's name and pronouns, a tiny battle (the lead seizes the fort next to
  them within 3 turns, one brigand), a victory scene. **New Game plays it
  until Chapter 1 exists (0803)**, then 0803 points `new_game.ron` at `ch01`.

**Deviations from the ticket**

- Blocked by 0502 (enemy phase playback, PR #122 green but not merged):
  built on `main`; nothing here depends on it (tests win and lose with
  scripted commands). Until it lands, enemies still do nothing in their
  phase.
- `map:` names a map id (`"ch01"`), like the rest of the content, not a path.
- No `consumable:` on enemies: `weapons-and-items.md` says enemies carry
  none (Nick, 0501).
- Objectives are written `Rout()`, `DefeatUnit(unit: "garth")`,
  `Seize(pos: (x, y), by_lord: true)`, `Survive(turns: 8)`, each with an
  optional `turn_limit`, since core's objectives have turn limits.
  `DefeatUnit` names a character.
- Added `reinforcements:` to the battle format (the Quick Battle has one).
- `apply_result(def, state, unused_charges)`: the charges left live in the
  battle's rewind history, and the clear gold in the battle file.
- Game Over says `Retry` (the design doc's word) rather than `Retry chapter`;
  it restarts the battle, as the design says.
- The name grid's space cell reads `Blank`: the key checker refuses the word
  "Space" in UI text.
- Playtime is counted (`Ctx::clock_s`), ready for 0802's slots.

**Claude's starting rules (Nick can veto)**

1. After a won battle, every unit is back at full HP for the next one, not
   only Casual retreats (as in Fire Emblem).
2. The unused-rewind bonus is one EXP award per deployed unit (7 per
   charge, at most one level), so it can level a unit up.
3. A unit that dies in Classic sends its whole loadout to the stock:
   weapons (keeping their wear), armour and accessory.
4. A battle without Preparations gives its default pack for free (Chapter 1:
   3 Potions with an empty stock); unused items go to the stock after a
   win. Gold picked up in battle is kept on a win, lost on a defeat.
5. The lead's name: up to 12 characters picked on a letter grid (A-Z, a-z,
   `-`, `'`, a space, Delete, Done); Cancel deletes a letter. The lead's
   two map letters are the first two of the chosen name.
6. Mode screen lines: "Classic: a unit that falls in battle dies and is
   gone for good." / "Casual: a unit that falls retreats, and is back for
   the next battle."
7. `Restart Battle` sits after `Suspend` in the map menu and asks "Restart
   the battle from turn 1?"; Retry and Restart go straight back to the
   battle (not the intro scenes).
8. The last chapter ends on "To be continued..."; Confirm returns to the
   title.

**For Nick to try** (Pages build): New Game → pick a mode → pick the female
lead and rename her → the test chapter's intro should use her name and
"she" → seize the fort → victory scene → "To be continued". Also try the
map menu's `Restart Battle`, and waiting out the 3 turns for Game Over →
Retry. Music: the title music keeps playing through the test chapter
(battle music per battle is 0807).

Follow-up tickets: none new. 0802 (save prompt hook is `FlowScreen::
next_chapter`), 0408 (push Preparations from the flow, drop the validator's
refusal), 0810 (show `BattleRewards`), 0807 (battle music) build on this.

