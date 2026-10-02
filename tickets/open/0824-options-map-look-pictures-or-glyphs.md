---
id: "0824"
title: "Options: Map look (pictures or glyphs)"
type: feature
milestone: M7 Chapter 1 & game flow
model: sonnet-5
effort: medium
status: todo
blocked_by: ["0436", "0805"]
nick_input: none
completed:
---

# 0824 — Options: Map look (pictures or glyphs)

## Context

The battle map has two looks (ADR-0038): the bought tiles and sprites
(0436, 0437) and the glyph look (letters for units, symbols for terrain),
which stays in the game because it is what the public repository shows.
Nick decided on 2026-10-02 (ticket 0039, `docs/design/look-and-feel.md`,
*Battle map: bought tiles and unit sprites*) that players may pick: "if
both exist I guess we can add the option and we already have the
decoupled art/engine logic afaik". Compare Dwarf Fortress on Steam, which
switches between its new art and classic ASCII.

0436 makes the game pick its map skin at start-up (the private tileset's
skin when it is present, else the glyph skin) and keeps 0433's debug-menu
switch. 0805 builds the Options screen and the saved `Settings`.

## Nick input

None. The starting rules below are Claude's; list them in the PR so Nick
can veto:

- The entry is **"Map look"** with the values **Pictures** and **Glyphs**;
  Pictures is the default.
- It switches terrain and units together.
- It changes the map at once and is remembered.
- A build without the bought files has only the glyph look and doesn't
  show the entry.

## Scope

**In:**
- `Settings::map_look: MapLook` (`Pictures` (default) | `Glyphs`), saved
  with the other settings (0805).
- A "Map look" row on the Options screen, shown only when a picture skin
  exists (`Content::tilesets` has the private tileset, as in 0436).
- `Ctx.map_skin` follows the setting at start-up and when it changes.

**Out (do not do):**
- A key that switches the look during a battle.
- Looks for other screens (portraits, the combat scene, menus).
- Colour themes (0806).
- Removing the debug-menu skin switch (0433): it stays, and lists the
  test tileset too.
- Terrain pictures (0437): **either order works.** The function in step 2
  returns whichever picture skin the game has.

## Implementation steps

1. `MapLook` and the field in `Settings` (`crates/ui`, where 0805 put
   `Settings`); bump nothing if 0805's loader already defaults missing
   fields, otherwise follow its versioning.
2. One function that picks the skin from `(settings.map_look, content)`:
   `Pictures` and a private tileset present → the picture skin 0436
   chooses at start-up; otherwise `GlyphSkin`. Use it at start-up
   (replacing 0436's step) and when the row changes.
3. The row: Left/Right changes the value like the other rows; its help
   line follows the `keyboard-input` skill. Hidden when there is no
   picture skin.
4. "Restore defaults" sets it back to Pictures.

## Acceptance criteria

- [ ] Harness test on a public fixture tileset: changing the row swaps
      `ctx.map_skin` at once, and the choice is still there after a
      restart.
- [ ] Harness test: without a picture skin the row isn't on the screen
      and the game uses the glyph skin whatever the saved value is.
- [ ] 0433's `the_skin_never_changes_the_game` still passes.
- [ ] Snapshot of the Options screen with the row.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the skin-picking function for each combination.
- Snapshot / integration: as listed.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
