---
id: "0436"
title: "Units on the battle map drawn as the bought map sprites"
type: feature
milestone: M3 Battle UI
model: opus-5.5
effort: high
status: todo
blocked_by: ["0433", "0110", "0116"]
nick_input: sign-off
completed:
---

# 0436 — Units on the battle map as the bought map sprites

## Context

Nick decided on 2026-10-02 (ticket 0038, `docs/design/look-and-feel.md`,
*Battle map: bought tiles and unit sprites*) that the battle map is drawn
with the bought Tiny Tales art. This ticket does the units: each unit is
its **map sprite** instead of two letters. Terrain is 0437; until that
lands, sprites stand on the glyph terrain (render B of the spike, which
Nick also liked).

ADR-0038: units are painted by a map skin from the `MapScene` (0432), and
pictures on the map come from a tileset file through the sprite skin
(0433). 0433's skin reads every unit picture from one grid in one image
and uses a generated test tileset. The bought sprites are different:

- **One file per character or class**, a 48×80 sheet: 3 columns (walking
  frames) × 4 rows (facing down, left, right, up) of **16×20** frames. The
  standing, front-facing frame is column 1 (the middle), row 0.
- Hundreds of them: the 16 heroes, about 210 battler classes, and map
  sprite packs with no battle picture (townsfolk, knights, nobles, 1100
  generated people). Only the ones a chapter uses are imported.
- They are bought, so they live in `assets-private/` (ADR-0040), and a clone
  without them must still build, run and pass every gate.

The bought files are sorted in the private assets repository, in
`assets-private/library/tiny-tales/characters/` (`heroes/<Name>/
map_sprite.png`, `battler-classes/<pack>/<Name>/map_sprite.png`,
`map-sprites-only/<pack>/<Name>.png`); `INDEX.md` one folder up lists
them. ADR-0040 says how they reach the game: fetch them with `cargo xtask
private-assets --library`, write what the game reads into
`assets-private/game/`, commit and push there, then `cargo xtask
private-assets --pin`. That needs the private repository uploaded (0116,
in `blocked_by`).

## Nick input

**Decided 2026-10-02** (`look-and-feel.md`, *Battle map: bought tiles and
unit sprites*): a sprite unit shows its side by a **1-pixel outline in the
side's colour**, and an acted unit is drawn **grey and darker**. Nick: "A,
and potentially A with C", where C is a small corner mark in the side's
colour on the unit's tile.

**Decided 2026-10-02 in ticket 0039** (same section of
`look-and-feel.md`; Nick judged mockups J4, J5, K2–K4, L, L2 and M–M3 in
the bought-art folder's `spike-renders/`):

- **The HP bar never overlaps the sprite's feet.** The sprite is drawn
  higher on its tile: its feet stand directly on top of the 2-pixel HP bar
  (Nick saw it 1 and 2 pixels higher and chose this: "sprite 0 mark 1
  looks best to me"). HP bars
  are drawn over every sprite. A sprite unit's bar is **14 pixels wide**,
  one pixel in from each side of the tile, always.
- **A sprite never covers the unit above it.** A sprite and its outline
  aren't drawn inside a tile another unit stands on: the head is shaved
  off at that tile's edge. With nobody above, the whole head shows.
- **A unit under an effect** shows a small arrow in the **top-right corner
  of its tile**: an **up arrow for a bonus**, a **down arrow for a
  penalty**, 5 pixels wide with a dark edge, reaching 3 pixels above the tile's
  top edge (mockup L2, left panel). It replaces 0433's
  3×3 placeholder square. The up arrow **bounces** 1 pixel up and the
  down arrow 1 pixel down; a unit with both shows one at a time, **the
  two taking turns**. When another unit stands in the tile above, the
  mark sits 4 pixels lower, inside its own tile (*Claude's starting
  rule*; say so in the PR).
- **Sprites step on the spot and walk along their path.** That is ticket
  0440, after this one. Here sprites **stand still**, front-facing.

**Sign-off:** two rendered Quick Battle frames with the bought sprites,
sent to Nick (never committed): **the outline alone, and the outline with
a corner mark** (top-left corner, so it doesn't meet the effect mark). He
picks one; record it in `look-and-feel.md` and leave the other out of the
code. With them, the list of which sprite each Chapter 1 class and
character uses.

Nick wants this in before the Chapter 1 playtest (2026-10-02), so 0804
waits for this ticket.

## Scope

**In:**
- Unit pictures in a tileset file may come from **separate image files**,
  one per character or class, as well as from 0433's grid.
- A skin that paints **terrain as glyphs and units as sprites** (so this
  ticket doesn't wait for 0437).
- The side outline, the acted look and the effect mark (see *Nick
  input*), the HP bar and the fading-out of a falling unit, on sprites.
- A private tileset file naming the bought map sprite of every class and
  named character in the Quick Battle and Chapter 1, and an importer that
  copies those files into `assets-private/game/`.
- A public placeholder so the public build still runs: without
  `assets-private/` the game keeps the glyph skin.
- The game uses the sprite-unit skin by default **when the private tileset
  is present**.

**Out (do not do):**
- Terrain tiles (0437), lighting (0438), zoom (0439).
- Stepping on the spot and walking along a path (0440).
- An Options entry for the look (0824).
- Drawing or editing sprites. Recolours are allowed (`look-and-feel.md`)
  but not needed here.
- The combat scene (0413) and portraits (0711).
- Committing any bought file, or a picture made from one.

## Implementation steps

1. **ADR** (`write-adr`), extending ADR-0038 section 3 and 0433's format:
   how a tileset file names per-unit image files, how the game picks its
   map skin (private tileset present → that skin; otherwise the glyph
   skin), and that a skin may mix glyph terrain with sprite units.
2. **Format** (`crates/content/src/tileset.rs`, `assets/tilesets/README.md`):
   in `units`, an entry is either 0433's `(column, row)` in the tileset's
   own image or `(image: "units/fighter_male.png", frame: (1, 0))` with
   `unit_px: (16, 20)` giving the frame size. Validate: the image is in
   `Content::images`, the frame lies inside it, every named class and
   character exists. A tileset may have `terrain: None` (meaning "paint
   terrain with the glyph skin"); 0433's rule "every terrain id has a
   tile" then doesn't apply.
3. **Skin** (`crates/ui/src/map_view/`): where a tileset has no terrain,
   paint terrain, ranges, cursor and path with `GlyphSkin` and units with
   `SpriteSkin`'s unit code. Share, don't copy. A unit's tile keeps its
   terrain background and loses the terrain's glyphs under the sprite (as
   the glyph skin does under initials). Sprites are drawn top row first so
   a unit's head overlaps the tile above it and not the reverse.
4. **Where the sprite stands, and the effect arrows** (see *Nick
   input*). The sprite's `dest` is centred on the tile and its bottom edge
   is 2 pixels above the tile's bottom edge (the HP bar is the bottom
   2). Paint in two passes: every unit's outline and sprite, top row
   first, then every unit's HP bar and marks, so a bar is never under a
   neighbour's head. **Effect arrows:** `MapScene`'s `UnitView` says
   `has_effect: bool` (0432); change it to say which kinds the unit has,
   `bonus` and `penalty` (a stance on the user is a bonus, a debuff on a
   target is a penalty: `docs/design/combat-arts.md` rule 7 and the stance
   riders in `progression.md`; see how `crates/core/src/skill.rs` stores
   them). The glyph skin tints the background when either is set, so its
   snapshots don't change. The arrows are pictures, so they are sprite
   items from an image file (ADR-0038), never rectangles: a small public
   image of our own (an up arrow and a down arrow, 5×5 pixels plus a
   1-pixel dark edge, as in mockup K2), made by `cargo xtask` like the
   test tileset and listed in `THIRD_PARTY_ASSETS.md` the same way. The
   arrow's dark-edged box is 7×7; its top-right corner is at the tile's
   top-right corner, moved 1 pixel right and 3 pixels up (the box starts
   at tile x + 10, tile y − 3). Colours: new palette names
   `effect_bonus` (`#70d8e8`) and `effect_penalty` (`#c878e8`), *tunable*
   (the mockup's stand-ins; Nick didn't comment on them); draw the image
   in white and tint it, or bake the colours in, whichever the sprite
   item supports. On an acted unit the arrow dims with the sprite.
   **The marks move**, from the screen's animation clock (the one the
   cursor pulse uses; put the scene's clock value in the `MapScene` if
   the skin can't see it): the up arrow is drawn 1 pixel higher and the
   down arrow 1 pixel lower for 375 ms out of every 750 ms; a unit with
   both shows the up arrow for 750 ms, then the down arrow for 750 ms
   (all *tunable*, in one place). A mark whose unit has another unit in
   the tile directly above is drawn 4 pixels lower. **HP bar:** under a
   sprite unit it covers tile columns 1 to 14 (14 pixels) and fills by
   HP over those 14. The glyph look gets the same width in ticket
   0441: **either order works.** If 0441 is done, share its bar
   rectangle and fill maths; if not, leave the glyph skin's bar and its
   snapshots alone here. **Shaving:** when another unit's tile is directly above, the
   unit's sprite and outline are clipped at its own tile's top edge (the
   sprite item's `clip`, 0231); otherwise they aren't clipped. The
   walking case is 0440's.
5. **Side and acted marks** (see *Nick input*). An outline needs the sprite's silhouette: draw the
   same frame four times, offset by one pixel up, down, left and right,
   in the side's colour, under the sprite. That needs a **solid-colour
   draw of a sprite** (every opaque pixel in one colour): add it to
   `Sprite` (the item, `app`'s renderer, the snapshot line, 0232's PNG
   renderer if done) and note it in the ADR. 0413 needs the same thing for
   its hit flash: whichever ticket lands first adds it, the other reuses
   it. Acted: grey and darker (the spike moved each pixel 75% of the way
   to its own grey and multiplied it by 0.6, *tunable*), which needs a
   grey draw on the sprite item too; the outline dims with it. Never
   per-pixel rectangles (ADR-0038).
6. **Unit lookup**: character id, then class id, then `fallback` (0433).
   Write the Chapter 1 and Quick Battle table in the private tileset file
   from `assets/data/characters.ron` and `classes.ron`. Starting picks,
   for Nick's sign-off (from the spike):
   - lead: Heroes 1 Male Fighter / Female Fighter (by the lead's sex);
   - Mage (Rue): Heroes 1 Witch; Archer: Heroes 1 Archer;
   - Guard: *Faith and Evil* Church Knight; Cleric: Church Cleric;
   - Rider: nothing mounted exists in the bundle; a Human Knights sprite
     on foot until 0040 finds mounted art;
   - Brigand, Raider, enemy Archer: Human NPC Advanced `Warrior_M*`,
     `Fighter_M*`, `Rogue_M*` (human bandits, where the combat picture is
     still the orc stand-in);
   - Fire and Frost Elemental: *Elemental Forces*.
   A unit's map sprite and its combat picture (0413) should be the same
   character where both exist.
7. **Importer** (`cargo xtask`, next to 0711's `portrait-import`): copy the
   named map sprite sheets into `assets-private/game/units/` under stable
   names. Clear error when the source folder is missing.
8. **Default skin**: at start-up `Ctx.map_skin` is the private tileset's
   skin when `Content::tilesets` has it, else `GlyphSkin`. The debug-menu
   switch from 0433 keeps working and lists it.
9. Render the Quick Battle with `cargo xtask frame-png` (0232 if done;
   otherwise the web build's debug menu) with the bought sprites, **look
   at it**, and send it to Nick. Don't commit it.
10. Update `crates/ui/README.md`, `assets/tilesets/README.md` and
   `look-and-feel.md` (the class → sprite table, once Nick signs off).

## Acceptance criteria

- [ ] Content validation: each new error has a test with its message (a
      unit image that doesn't exist, a frame outside its image, an unknown
      class or character).
- [ ] A clone **without** `assets-private/` builds, runs with the glyph
      skin and passes every gate; every existing glyph snapshot is
      unchanged.
- [ ] With the private tileset present, the Quick Battle shows every unit
      as a sprite on glyph terrain, with the side mark, the acted look and
      the 14-pixel HP bar, the feet above the bar, an up or down arrow on
      a unit under an effect, and a unit standing below another clipped
      at its tile's top edge (snapshot on a **public fixture** tileset with per-file
      unit images, made by `cargo xtask test-tileset`; never a bought
      file).
- [ ] 0433's `the_skin_never_changes_the_game` and
      `every_scene_feature_is_painted` also run under the mixed skin.
- [ ] `git status` shows no bought file; the PR has no picture made from
      one.
- [ ] Nick was sent the rendered frame and the class → sprite list.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the tileset format's new entries and errors; unit lookup order;
  the frame rectangle for `(1, 0)` in a 48×80 sheet; draw order (top row
  first, bars and marks after every sprite); the outline's four offsets;
  the sprite's height above the HP bar; the bar's 14-pixel width and
  fill; which arrows a unit with a bonus, a penalty or both gets (read
  from the `MapScene`); the bounce and the taking of turns against the
  clock; the clip and the lowered mark with and without a unit above.
- Property: for any unit frame size up to the tile size plus 16 pixels,
  the sprite's `dest` is centred on its tile and bottom-aligned.
- Snapshot / integration: Quick Battle under the mixed skin on the public
  fixture; behaviour tests read the `MapScene` (ADR-0038), not sprites.

## Completion notes

*(Filled in by the session that completes the ticket: what was done, deviations,
follow-up tickets created, notes for Nick.)*
