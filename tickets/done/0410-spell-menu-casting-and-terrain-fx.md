---
id: "0410"
title: "Spell menu, heal and tile targeting, affinity markers, terrain-change display"
type: feature
milestone: M3 Battle UI
model: sonnet-5
effort: medium
status: done
blocked_by: ["0309", "0310", "0404", "0407"]
nick_input: sign-off
completed: 2026-10-01
---

# 0410 — Spell menu and casting UI

## Context

This ticket shows the magic from `docs/design/magic.md` (0004) on the battle
screen. It sends the `Cast`/`Equip` commands from 0309 and 0310 and reacts to
`SpellCast`, `Healed`, `TerrainChanged` and `SpellUsesChanged` events
([ADR-0004](../../docs/adr/0004-crate-architecture.md)). It follows the menu
patterns of 0404 (attack, forecast) and 0407 (item, equip).

## Nick input

**Sign-off:** in Quick Battle, cast Fire on a forest and Frost on water, heal
an ally, and hit an elemental. Comment on readability. Also listen to
the spell sounds from 0424 (cast sound as the caster's name flashes, again
on a follow-up or a counter): Nick wanted to judge them in play.

## Scope

**In:**
- `Magic` entry in the action menu, the spell list with `uses_left/uses`.
- Attack-spell targeting via 0404's forecast.
- Heal targeting with an HP preview.
- Tile targeting for Fire/Frost.
- Affinity markers in the forecast and the info screen.
- Drawing burning/burnt/ice tiles and a short change animation.
- Help-bar text.
- A caster in the Quick Battle (Nick, 0424 sign-off: "add a spell caster
  to our quick battle with both fire and ice spells plus heal"), so the
  sign-off below can be played. See step 7.

**Out (do not do):**
- Battle notes (0411).
- AI playback (0502).
- New spells or rules.

## Implementation steps

1. Action menu: `Magic` is enabled when the unit knows a spell with a legal
   target from `dest`. The spell list shows `Fire  6/10  Mt5 Hit90 Rng1-2`,
   with spells at 0 uses dimmed.
2. Attack spell → 0404's targeting and forecast. The forecast shows `!` for
   Weak, `(resist)` for Resist, and `heals N` for Absorb. The equipped marker
   in 0407's `Equip` menu also lists attack spells.
3. Heal → target mode over valid allies with a preview (`HP 10 → 25`), and
   the 0407 `+N` popup on `Healed`.
4. Tile cast → the cursor snaps between valid tiles (0310's rules) and a
   preview names the change (`Forest → Burning (1 round)`, `Sea → Ice`).
   Confirm sends `Cast { Tile }`.
5. On `TerrainChanged`: redraw the tile with a 0.4 s flash (timing tunable).
   The info screen (0405) lists the unit's spells with uses, and its
   affinities.
6. Help bar: `f cast · d back` in the spell modes.
7. Quick Battle caster. Add `test_mage` to `assets/data/characters.ron`
   (class `mage`, level 1, map label `Ma`, talent `Mag`, `weapon_ranks: []`,
   `loadout: (armour: Some("leather_vest"))`) with
   `personal_spells: [(1, "frost"), (1, "heal")]`. The class only teaches
   Fire at level 1 (Frost at 5), and a character's starting level does
   not teach later class spells, so Frost must be personal. Push it in
   `quick_battle()` (`crates/ui/src/screens/battle/mod.rs`) **after** the
   generic enemies (unit 7), at `(3, 6)`, so every other unit keeps its
   id. Expect this to change ~30 existing UI tests and ~15 snapshots
   (ready-unit counts, cycling order, end-turn prompts, auto-end, the map
   drawn): update each to the new battle, reading every new snapshot.
   `character::tests::embedded_characters_load_and_make_units` asserts
   each character is created with something equipped; a spell-only
   caster only gets one at battle start (`Unit::prepare_for_battle`), so
   accept a learned spell there too. The sign-off also says "hit an
   elemental": the Quick Battle has none. Don't add one without asking
   Nick (`ask-nick`: add a Frost Elemental enemy to the Quick Battle?).

## Acceptance criteria

- [x] Harness: Magic → Fire → forest tile → the tile shows burning; after End Turn and the enemy phase, it shows burnt.
- [x] Harness: Magic → Heal → adjacent ally → HP rises by the design amount; the uses counter drops.
- [x] Snapshots: the spell list, a forecast with each affinity marker, the tile preview, burning/burnt/ice tiles.
- [x] `quick_battle_state` test: unit 7 is the Test Mage, knowing Fire,
      Frost and Heal, Fire equipped. **Met as unit 8:** Nick added a Frost
      Elemental, which is unit 7 (see the Completion notes).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Harness and snapshot as above; cancelling at each level leaves the state unchanged.

## Completion notes

**Done.**

Nick's two answers (asked before building, recorded in
`docs/design/magic.md`, *Casting on the battle screen*):

- A spell that can hit an enemy or change a tile from one spot: "one cursor
  but make a glyph or highlight depict when it's damaging a unit versus
  changing terrain".
- An elemental in the Quick Battle: "One Frost Elemental".

What was built:

- **`Magic`** in the action menu (`MenuEntry::Magic`), the spell list
  (`Mode::SpellMenu`) and one target mode for everything a spell can be cast
  on (`Mode::CastTarget`), in the new `battle/magic.rs`. Every target is one
  `BattleState::check` accepts.
- **Enemies:** 0404's forecast and list, now for a weapon *or* a spell
  (`Targeting.with: Equipped` replaces `slot`; `Technique::action` and
  `art_choices` take it). The Mage's Overcast shows in the list. The
  forecast names the spell and its uses, and marks affinities: `!` (Weak),
  `(resist)`, `heals N`.
- **Allies:** `Heal on Test Knight: HP 2 → 18` over the help bar (the number
  is the core's: new `BattleState::preview_heal`), and 0407's `+N` popup.
- **Tiles:** `Forest → Burning (1 round)` / `Sea → Ice`; every tile the
  spell can change is drawn as what it would become while picking.
- **Terrain change:** the tile flashes for 0.4 s (`TerrainFlash`,
  `TERRAIN_FLASH_S`), also when a fire burns out.
- **Equip** lists attack spells after the weapons. **Info screen:** `E` on
  an equipped spell, and the class's affinities under the weapon ranks.
- **Quick Battle:** `test_mage` (Fire from the class, Frost and Heal
  personal) at (3, 6), and a `test_frost_elemental` at (8, 7) that holds
  its tile (`ai: Stationary`).

Deviations from the plan:

- **The mage is unit 8, not 7.** The Frost Elemental Nick asked for is the
  fourth enemy, so it took 7. Every other unit keeps its number; the rogue
  reinforcement is now 9.
- **Step 7 said to push the mage in `quick_battle()`**, but since 0801 the
  Quick Battle is a data file. To add a player unit without renumbering the
  enemies, a battle file's player slot can now say `after_enemies: true`
  (`assets/battles/README.md`); `PlayerSlot` carries its unit id and
  `Campaign::battle_setup` puts the units in id order.
- `trigger_tests.rs` uses the Quick Battle's first six units only: its
  rogue fixture stands where the new units are.
- About 40 existing tests and 27 snapshots changed, all for the two new
  units; every new snapshot was read.

Rules and looks I had to choose (*Claude's starting rules*; Nick may veto,
they are also in `magic.md`):

1. **The menu opens on `Magic` only when a spell can hit an enemy.**
   Example: the mage beside a forest with no enemy near opens on `Wait`,
   as before; with a brigand two tiles away it opens on `Magic`.
2. **The cursor steps through targets in reading order** (row by row, left
   to right), enemies and tiles together. On an enemy, up and down move
   through the list of spell skills (Overcast) instead, as in the weapon
   forecast.
3. **What tells units from terrain:** enemies a spell can hit get the red
   attack tint; tiles it can change show the fire (`**`) or ice (`▒▒`)
   they would become.
4. **The spell list shows a heal's own power** (`HP+10`); the preview line
   shows what it really restores with the caster's Mag (`HP 2 → 18`).
5. **A spell with no uses left is dimmed in `Equip`.**
6. **`(resist)` and `heals N` sit under the caster's crit** in the forecast.
7. **The info screen writes affinities as `Fire  Weak`, `Ice  Absorb`.**
8. **The Frost Elemental never moves**; it only casts at units within two
   tiles, so it doesn't join the first turn's fight.

Follow-up tickets:

- **0430** — point at an enemy with a caster selected to cast at it (0428
  only looks at weapons).
- **0431** — show the 5 burn damage when a fire burns out under a unit
  (today the HP just drops).

For Nick's sign-off (after the merge, on Pages): Quick Battle → move the
cursor down to the mage → `Magic`. Fire on the forest left of it burns at
once and is burnt on turn 2. Frost needs a walk up to the water. The
elemental is at the bottom, left of the fort's wall: Fire shows `!`, Frost
shows `heals N`. Heal needs someone hurt beside the mage.
